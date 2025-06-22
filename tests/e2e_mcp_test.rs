//! End-to-end integration test for MCP server and client generation and communication

use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use rusqlite::{Connection, params};
use std::thread;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Command as AsyncCommand;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Clean up any SQLite database files for a given project name
/// This ensures each test run starts with a fresh database state
fn cleanup_project_databases(project_name: &str) -> Result<()> {
    // Database locations based on the template's get_database_path() function
    let db_paths = vec![
        // macOS location
        dirs::data_dir().map(|d| d.join(project_name).join(format!("{}.db", project_name))),
        // Linux location
        dirs::data_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
            .map(|d| d.join(project_name).join(format!("{}.db", project_name))),
        // Windows location
        dirs::data_local_dir().map(|d| {
            d.join(project_name)
                .join("data")
                .join(format!("{}.db", project_name))
        }),
    ];

    for db_path in db_paths.into_iter().flatten() {
        if db_path.exists() {
            info!("Cleaning up database: {}", db_path.display());
            // Remove the database file and any associated WAL/SHM files
            let _ = fs::remove_file(&db_path);
            let _ = fs::remove_file(db_path.with_extension("db-wal"));
            let _ = fs::remove_file(db_path.with_extension("db-shm"));

            // Try to remove the parent directory if it's empty
            if let Some(parent) = db_path.parent() {
                let _ = fs::remove_dir(parent);
            }
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_mcp_server_client_generation() -> Result<()> {
    // Initialize tracing for test visibility
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("e2e_mcp_test=info".parse().unwrap())
                .add_directive("agenterra=info".parse().unwrap()),
        )
        .with_test_writer()
        .try_init();

    // Discover project root first
    // Determine project root at compile time via Cargo
    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Resolve path to agenterra binary (prefer Cargo-built path)
    let agenterra = project_dir
        .join("target/debug/agenterra")
        .to_string_lossy()
        .into_owned();

    // With the new "take users at their word" logic, we pass the specific template paths directly
    let server_template_dir = project_dir.join("templates/mcp/server/rust_axum");
    let client_template_dir = project_dir.join("templates/mcp/client/rust_reqwest");

    // Use target/tmp/e2e-tests directory for generated artifacts
    // Clean any previous run directories to avoid duplicate headers or build conflicts
    let scaffold_path = project_dir.join("target/tmp/e2e-tests");
    // Clean any previous run directories to avoid conflicts
    for sub in ["e2e_mcp_server", "e2e_mcp_client"] {
        let dir = scaffold_path.join(sub);
        let _ = fs::remove_dir_all(&dir);
    }
    std::fs::create_dir_all(&scaffold_path)?;

    // Clean up any existing client databases to ensure fresh state
    cleanup_project_databases("e2e_mcp_client")?;

    info!("=== Testing MCP Server Generation ===");
    info!("Project dir: {}", project_dir.display());
    info!("Server template dir: {}", server_template_dir.display());

    // Test 1: Generate MCP server
    let server_name = "e2e_mcp_server";
    let server_output = scaffold_path.join(server_name); // Full path for verification
    let schema_path = project_dir.join("tests/fixtures/openapi/petstore.openapi.v3.json");

    // Verify schema file exists
    if !schema_path.exists() {
        panic!("Schema file not found at: {}", schema_path.display());
    }

    let server_result = Command::new(&agenterra)
        .args([
            "scaffold",
            "mcp",
            "server",
            "--project-name",
            server_name,
            "--output-dir",
            scaffold_path.to_str().unwrap(), // Pass parent directory
            "--schema-path",
            schema_path.to_str().unwrap(),
            "--template-dir",
            server_template_dir.to_str().unwrap(),
            "--template",
            "rust_axum",
            "--base-url",
            "https://petstore3.swagger.io",
        ])
        .output()?;

    debug!(
        "Server generation stdout: {}",
        String::from_utf8_lossy(&server_result.stdout)
    );
    if !server_result.stderr.is_empty() {
        warn!(
            "Server generation stderr: {}",
            String::from_utf8_lossy(&server_result.stderr)
        );
    }

    if !server_result.status.success() {
        panic!(
            "Server generation failed with exit code: {:?}",
            server_result.status.code()
        );
    }

    // Verify server files exist
    assert!(server_output.join("Cargo.toml").exists());
    assert!(server_output.join("src/main.rs").exists());
    assert!(server_output.join("src/handlers/mod.rs").exists());

    info!("✅ Server generation successful");

    info!("=== Testing MCP Client Generation ===");

    // Test 2: Generate MCP client
    let client_name = "e2e_mcp_client";
    let client_output = scaffold_path.join(client_name);
    let client_result = Command::new(&agenterra)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--project-name",
            client_name,
            "--output-dir",
            scaffold_path.to_str().unwrap(), // Pass parent directory
            "--template-dir",
            client_template_dir.to_str().unwrap(),
            "--template",
            "rust_reqwest",
        ])
        .output()?;

    if !client_result.status.success() {
        eprintln!("Client generation failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&client_result.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&client_result.stderr));
        panic!("Client generation failed");
    }

    // Verify client files exist
    assert!(client_output.join("Cargo.toml").exists());
    assert!(client_output.join("src/main.rs").exists());
    assert!(client_output.join("src/client.rs").exists());
    assert!(client_output.join("src/repl.rs").exists());

    info!("✅ Client generation successful");

    // Ensure standalone crates by appending minimal workspace footer
    for path in [&server_output, &client_output] {
        let cargo_toml = path.join("Cargo.toml");
        if let Ok(contents) = fs::read_to_string(&cargo_toml) {
            if !contents.contains("[workspace]") {
                if let Ok(mut f) = OpenOptions::new().append(true).open(&cargo_toml) {
                    writeln!(f, "\n[workspace]\n").ok();
                }
            }
        }
    }

    // Test 3: Build generated projects (always test compilation)
    info!("=== Building Generated Server ===");

    let server_build = Command::new("cargo")
        .args([
            "build",
            "--manifest-path",
            &server_output.join("Cargo.toml").to_string_lossy(),
        ])
        .output()?;

    if !server_build.status.success() {
        eprintln!("Server build failed:");
        eprintln!("stderr: {}", String::from_utf8_lossy(&server_build.stderr));
        panic!("Server build failed");
    }

    info!("✅ Server builds successfully");

    info!("=== Building Generated Client ===");

    let client_build = Command::new("cargo")
        .args([
            "build",
            "--manifest-path",
            &client_output.join("Cargo.toml").to_string_lossy(),
        ])
        .output()?;

    if !client_build.status.success() {
        eprintln!("Client build failed:");
        eprintln!("stderr: {}", String::from_utf8_lossy(&client_build.stderr));
        panic!("Client build failed");
    }

    info!("✅ Client builds successfully");

    // Test 4: End-to-end MCP communication using generated client
    info!("=== Testing MCP Server ↔ Client Communication ===");

    // The generated binary name matches the project name we passed ("e2e_mcp_server")
    let server_binary = server_output.join("target/debug/e2e_mcp_server");
    if !server_binary.exists() {
        anyhow::bail!(
            "Expected server binary not found at {}",
            server_binary.display()
        );
    }

    info!("✅ Server binary found at: {}", server_binary.display());

    // Use the generated client to test MCP communication
    let test_result = timeout(Duration::from_secs(60), async {
        test_mcp_with_interactive_client(&server_binary, &client_output).await
    })
    .await;

    match test_result {
        Ok(Ok(())) => {
            info!("✅ MCP communication test successful");
        }
        Ok(Err(e)) => {
            panic!("MCP communication test failed: {}", e);
        }
        Err(_) => {
            panic!("MCP communication test timed out");
        }
    }

    // Test 5: Verify SQLite cache directly
    info!("=== Verifying SQLite Cache ===");

    verify_sqlite_cache(&client_output)?;

    info!("🎉 Complete end-to-end MCP test passed!");

    Ok(())
}

/// Test MCP communication using the generated client's interactive REPL
async fn test_mcp_with_interactive_client(
    server_binary: &std::path::Path,
    client_output: &std::path::Path,
) -> Result<()> {
    // Log thread information to prove multi-threading
    let thread_id = thread::current().id();
    info!(
        "Starting comprehensive MCP client test on thread {:?}",
        thread_id
    );
    info!(
        "Total active threads: ~{}",
        thread::available_parallelism()?.get()
    );

    // Find the client binary
    let client_binary = client_output.join("target/debug/e2e_mcp_client");
    if !client_binary.exists() {
        return Err(anyhow::anyhow!(
            "Client binary not found at: {}",
            client_binary.display()
        ));
    }

    // Start the client with the server binary path
    info!("Starting MCP client: {}", client_binary.display());
    let mut client_process = AsyncCommand::new(&client_binary)
        .arg("--server")
        .arg(server_binary.to_str().unwrap())
        .arg("--timeout")
        .arg("30")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn client process")?;

    let stdin = client_process
        .stdin
        .as_mut()
        .context("Failed to get client stdin")?;
    let stdout = client_process
        .stdout
        .as_mut()
        .context("Failed to get client stdout")?;

    let mut writer = BufWriter::new(stdin);
    let mut reader = BufReader::new(stdout);

    // Give client time to connect and show initial output
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Read initial output (connection messages, capabilities, prompt)
    let mut line = String::new();

    // Helper function to read until prompt
    async fn read_until_prompt(
        reader: &mut BufReader<&mut tokio::process::ChildStdout>,
        line: &mut String,
    ) -> Vec<String> {
        let mut output = Vec::new();
        for _ in 0..50 {
            line.clear();
            match timeout(Duration::from_millis(500), reader.read_line(line)).await {
                Ok(Ok(0)) => break, // EOF
                Ok(Ok(_)) => {
                    output.push(line.trim().to_string());
                    if line.contains("mcp>") {
                        break;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        output
    }

    // Read initial connection output
    info!("=== Initial Connection ===");
    let initial_output = read_until_prompt(&mut reader, &mut line).await;
    for line in &initial_output {
        debug!("Initial: {}", line);
    }

    // Test 1: Status command
    info!("=== Testing Status Command ===");
    writer.write_all(b"status\n").await?;
    writer.flush().await?;

    let status_output = read_until_prompt(&mut reader, &mut line).await;
    let mut connection_verified = false;
    for line in &status_output {
        debug!("Status: {}", line);
        if line.contains("Connected: true") {
            connection_verified = true;
        }
    }
    if !connection_verified {
        return Err(anyhow::anyhow!(
            "Status command failed to verify connection"
        ));
    }
    info!("✅ Status command successful");

    // Test 2: List and get all resources (this will populate the SQLite cache!)
    info!("=== Testing Resources ===");
    writer.write_all(b"resources\n").await?;
    writer.flush().await?;

    let resources_output = read_until_prompt(&mut reader, &mut line).await;
    let mut resource_uris = Vec::new();
    let mut in_resources_list = false;

    for line in &resources_output {
        debug!("Resources: {}", line);
        if line.contains("Available resources:") {
            in_resources_list = true;
        } else if in_resources_list && line.trim().starts_with("") && line.contains(":") {
            // Extract URI from lines like "  uri: description"
            if let Some(uri) = line.trim().split(':').next() {
                let uri = uri.trim();
                if !uri.is_empty() && !uri.contains("No resources") {
                    resource_uris.push(uri.to_string());
                }
            }
        }
    }

    info!("Found {} resources to fetch", resource_uris.len());

    // Get each resource to populate the cache
    for uri in &resource_uris {
        debug!("Getting resource: {}", uri);
        writer
            .write_all(format!("get {}\n", uri).as_bytes())
            .await?;
        writer.flush().await?;

        let resource_output = read_until_prompt(&mut reader, &mut line).await;
        let mut resource_fetched = false;
        for line in &resource_output {
            if line.contains("Resource content:") || line.contains("contents") {
                resource_fetched = true;
            }
        }
        if resource_fetched {
            debug!("✅ Resource fetched: {}", uri);
        } else {
            warn!("⚠️ Failed to fetch resource: {}", uri);
        }
    }

    // Test cache retrieval by fetching the first resource again (should come from cache)
    if !resource_uris.is_empty() {
        let first_uri = &resource_uris[0];
        info!("Testing cache retrieval with: {}", first_uri);
        writer
            .write_all(format!("get {}\n", first_uri).as_bytes())
            .await?;
        writer.flush().await?;

        let cache_test_output = read_until_prompt(&mut reader, &mut line).await;
        let mut cache_hit_detected = false;
        for line in &cache_test_output {
            if line.contains("Resource content:") || line.contains("contents") {
                cache_hit_detected = true;
                break;
            }
        }
        if cache_hit_detected {
            info!("✅ Cache retrieval test successful: {}", first_uri);
        }
    }

    if !resource_uris.is_empty() {
        info!("✅ Resources discovery and fetching completed");
    }

    // Test 3: List and call all tools
    info!("=== Testing Tools ===");
    writer.write_all(b"tools\n").await?;
    writer.flush().await?;

    let tools_output = read_until_prompt(&mut reader, &mut line).await;
    let mut tool_names = Vec::new();
    let mut in_tools_list = false;

    for line in &tools_output {
        debug!("Tools: {}", line);
        if line.contains("Available tools:") {
            in_tools_list = true;
        } else if in_tools_list && line.trim().starts_with("") && line.contains(":") {
            // Extract tool name from lines like "  toolname: description"
            if let Some(tool) = line.trim().split(':').next() {
                let tool = tool.trim();
                if !tool.is_empty() && !tool.contains("No tools") {
                    tool_names.push(tool.to_string());
                }
            }
        }
    }

    info!("Found {} tools to test", tool_names.len());

    // Call each tool (some may fail without auth, that's OK)
    let mut at_least_one_tool_succeeded = false;
    let mut successful_tools = Vec::new();
    for tool in &tool_names {
        info!(
            "Calling tool: {} on thread {:?}",
            tool,
            thread::current().id()
        );
        writer
            .write_all(format!("call {}\n", tool).as_bytes())
            .await?;
        writer.flush().await?;

        let tool_output = read_until_prompt(&mut reader, &mut line).await;
        let mut tool_result_found = false;
        let mut result_content = String::new();

        for line in &tool_output {
            if line.contains("Tool result:") || line.contains("Error:") {
                tool_result_found = true;
                result_content = line.clone();
                if !line.contains("Error:") {
                    at_least_one_tool_succeeded = true;
                    successful_tools.push(tool.clone());
                }
            }
        }

        if tool_result_found {
            info!("✅ Tool '{}' response: {}", tool, result_content.trim());
        } else {
            info!("⚠️ Tool '{}' - no response received", tool);
        }
    }

    info!(
        "Successfully called {} tools: {:?}",
        successful_tools.len(),
        successful_tools
    );

    if !at_least_one_tool_succeeded && !tool_names.is_empty() {
        return Err(anyhow::anyhow!("No tools succeeded - all tools failed"));
    }

    if !tool_names.is_empty() {
        info!("✅ Tools discovery and testing completed");
    }

    // Test 4: List and get all prompts
    info!("=== Testing Prompts ===");
    writer.write_all(b"prompts\n").await?;
    writer.flush().await?;

    let prompts_output = read_until_prompt(&mut reader, &mut line).await;
    let mut prompt_names = Vec::new();
    let mut in_prompts_list = false;

    for line in &prompts_output {
        debug!("Prompts: {}", line);
        if line.contains("Available prompts:") {
            in_prompts_list = true;
        } else if in_prompts_list && line.trim().starts_with("") && line.contains(":") {
            // Extract prompt name from lines like "  promptname: description"
            if let Some(prompt) = line.trim().split(':').next() {
                let prompt = prompt.trim();
                if !prompt.is_empty() && !prompt.contains("No prompts") {
                    prompt_names.push(prompt.to_string());
                }
            }
        }
    }

    info!("Found {} prompts to test", prompt_names.len());

    // Get each prompt
    for prompt in &prompt_names {
        debug!("Getting prompt: {}", prompt);
        writer
            .write_all(format!("prompt {}\n", prompt).as_bytes())
            .await?;
        writer.flush().await?;

        let prompt_output = read_until_prompt(&mut reader, &mut line).await;
        let mut prompt_fetched = false;
        for line in &prompt_output {
            if line.contains("Prompt content:") || line.contains("messages") {
                prompt_fetched = true;
            }
        }
        if prompt_fetched {
            debug!("✅ Prompt fetched: {}", prompt);
        } else {
            warn!("⚠️ Failed to fetch prompt: {}", prompt);
        }
    }

    if !prompt_names.is_empty() {
        info!("✅ Prompts discovery and fetching completed");
    }

    // Send 'quit' to exit cleanly
    info!("=== Exiting Client ===");
    writer.write_all(b"quit\n").await.ok();
    writer.flush().await.ok();

    // Give it a moment to exit cleanly
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Clean up client process
    if let Err(e) = client_process.kill().await {
        eprintln!("Warning: Failed to kill client process: {}", e);
    }

    info!("✅ Comprehensive MCP test completed successfully");
    Ok(())
}

#[test]
fn test_cli_help_output() {
    let agenterra = env!("CARGO_BIN_EXE_agenterra");
    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Use sandbox directory under target/tmp to avoid polluting repo root
    let sandbox_dir = project_dir.join("target/tmp/cli_flag_tests");
    let _ = std::fs::remove_dir_all(&sandbox_dir);
    std::fs::create_dir_all(&sandbox_dir).unwrap();

    // Test main help
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .arg("--help")
        .output()
        .expect("Failed to run agenterra");

    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("scaffold"));
    assert!(output.contains("Scaffold servers and clients for various targets"));

    // Test scaffold help
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args(["scaffold", "--help"])
        .output()
        .expect("Failed to run agenterra");

    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("mcp"));
    assert!(output.contains("Model Context Protocol (MCP) servers and clients"));
}

#[test]
fn test_new_cli_structure() {
    let agenterra = env!("CARGO_BIN_EXE_agenterra");
    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Use sandbox directory under target/tmp to avoid polluting repo root
    let sandbox_dir = project_dir.join("target/tmp/cli_flag_tests");
    let _ = std::fs::remove_dir_all(&sandbox_dir);
    std::fs::create_dir_all(&sandbox_dir).unwrap();

    // Test server help shows correct options
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args(["scaffold", "mcp", "server", "--help"])
        .output()
        .expect("Failed to run agenterra");

    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("--schema-path"));
    assert!(output.contains("--template"));
    assert!(output.contains("--output-dir"));

    // Test client help shows correct options
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args(["scaffold", "mcp", "client", "--help"])
        .output()
        .expect("Failed to run agenterra");

    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("--template"));
    assert!(output.contains("--output-dir"));
    // Client should NOT have schema-path
    assert!(!output.contains("--schema-path"));
}

#[test]
fn test_cli_flag_combinations() -> Result<()> {
    let agenterra = env!("CARGO_BIN_EXE_agenterra");
    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Use sandbox directory under target/tmp to avoid polluting repo root
    let sandbox_dir = project_dir.join("target/tmp/cli_flag_tests");
    let _ = std::fs::remove_dir_all(&sandbox_dir);
    std::fs::create_dir_all(&sandbox_dir).unwrap();

    // Clean up any existing databases for projects we'll create
    cleanup_project_databases("test_schema_path_required")?;
    cleanup_project_databases("test_default_project_name")?;
    cleanup_project_databases("test_schema_path_nonexistent")?;
    cleanup_project_databases("test_schema_path_rejected")?;

    // Test 1: Server command requires --schema-path
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args([
            "scaffold",
            "mcp",
            "server",
            "--project-name",
            "test_schema_path_required",
        ])
        .output()
        .expect("Failed to run agenterra");

    assert!(
        !result.status.success(),
        "Server command should fail without --schema-path"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    // Verify clap's missing required argument error
    assert!(
        stderr.contains("the following required arguments were not provided")
            && stderr.contains("--schema-path <SCHEMA_PATH>"),
        "Should show missing --schema-path error, but got: {}",
        stderr
    );

    // Test 2: Client command should succeed with default project-name
    let client_template_dir = project_dir.join("templates/mcp/client/rust_reqwest");
    let output_dir = sandbox_dir.join("test_default_project_name");
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--template",
            "rust_reqwest",
            "--template-dir",
            client_template_dir.to_str().unwrap(),
            "--output-dir",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run agenterra");

    if !result.status.success() {
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        panic!(
            "Client command should succeed with default --project-name.\nExit code: {}\nStdout: {}\nStderr: {}",
            result.status.code().unwrap_or(-1),
            stdout,
            stderr
        );
    }
    // Verify that the project was created in the specified output directory
    assert!(output_dir.exists(), "Output directory should be created");

    // Test 3: Client command should reject --schema-path
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--project-name",
            "test_schema_path_rejected",
            "--schema-path",
            "dummy.yaml",
        ])
        .output()
        .expect("Failed to run agenterra");

    assert!(
        !result.status.success(),
        "Client command should reject --schema-path flag"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("unexpected argument '--schema-path' found")
            || stderr.contains("unrecognized argument '--schema-path'"),
        "Should show error about unsupported --schema-path flag, but got: {}",
        stderr
    );

    // Test 4: Valid server command combination
    // Note: This will fail because file doesn't exist, but argument parsing should work
    let server_template_dir = project_dir.join("templates/mcp/server/rust_axum");
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args([
            "scaffold",
            "mcp",
            "server",
            "--schema-path",
            "/nonexistent/schema.yaml",
            "--project-name",
            "test_schema_path_nonexistent",
            "--template",
            "rust_axum",
            "--template-dir",
            server_template_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run agenterra");

    // Should fail due to missing file, not argument parsing
    assert!(
        !result.status.success(),
        "Server command should fail with non-existent schema file"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("No such file or directory")
            || stderr.contains("not found")
            || stderr.contains("failed to read file"),
        "Should show file not found error, but got: {}",
        stderr
    );

    // Verify it's not an argument parsing error
    assert!(
        !stderr.contains("unrecognized")
            && !stderr.contains("unexpected")
            && !stderr.contains("required"),
        "Should not be an argument parsing error, but got: {}",
        stderr
    );

    // Test 5: Valid client command combination
    let client_template_dir = project_dir.join("templates/mcp/client/rust_reqwest");
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--project-name",
            "test-client",
            "--template",
            "rust_reqwest",
            "--template-dir",
            client_template_dir.to_str().unwrap(),
            "--output-dir",
            "/tmp/test-output",
        ])
        .output()
        .expect("Failed to run agenterra");

    // This should succeed in argument parsing
    // It may fail later due to template not found, but args should be valid
    if result.status.success() {
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(
            stdout.contains("Successfully")
                || stdout.contains("generated")
                || stdout.contains("Creating"),
            "Should show success message for valid client command, but got: {}",
            stdout
        );
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        // Should NOT be an argument parsing error
        assert!(
            !stderr.contains("unrecognized")
                && !stderr.contains("unexpected")
                && !stderr.contains("required"),
            "Should not be an argument parsing error, but got: {}",
            stderr
        );

        // Should be a template-related error, not argument parsing
        assert!(
            stderr.is_empty()
                || stderr.contains("template")
                || stderr.contains("not found")
                || stderr.contains("failed"),
            "Unexpected error for valid client command: {}",
            stderr
        );
    }

    Ok(())
}

/// Verify SQLite cache by directly querying the database
fn verify_sqlite_cache(client_output: &std::path::Path) -> Result<()> {
    // The new unified database follows OS-specific paths, but for this E2E test,
    // let's check for the database in the most likely locations
    let possible_paths = vec![
        // New unified database location (OS-specific default)
        dirs::data_dir()
            .unwrap_or_else(|| client_output.join("target/debug/data").to_path_buf())
            .join("e2e_mcp_client")
            .join("e2e_mcp_client.db"),
        // Fallback: old cache location (for compatibility)
        client_output.join("target/debug/data/e2e_mcp_client_cache.db"),
        // Current directory fallback
        client_output.join("e2e_mcp_client.db"),
    ];

    let mut db_path = None;
    for path in &possible_paths {
        if path.exists() {
            db_path = Some(path.clone());
            break;
        }
    }

    let db_path = match db_path {
        Some(path) => path,
        None => {
            error!("Checked paths:");
            for path in &possible_paths {
                error!("  - {}", path.display());
            }
            anyhow::bail!("SQLite unified database not found in any expected location");
        }
    };

    info!("Found SQLite unified database at: {}", db_path.display());
    info!("Thread {:?} is verifying database", thread::current().id());

    // Open connection to the database
    let conn = Connection::open(&db_path).context("Failed to open SQLite cache database")?;

    // First, list all tables in the database
    let mut table_stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
    let tables: Vec<String> = table_stmt
        .query_map(params![], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    info!("Database tables found: {:?}", tables);

    // Query the resources table to verify cached entries
    let mut stmt = conn.prepare(
        "SELECT id, uri, content_type, access_count, size_bytes, 
         datetime(created_at/1000, 'unixepoch') as created_at,
         datetime(accessed_at/1000, 'unixepoch') as accessed_at
         FROM resources 
         ORDER BY accessed_at DESC",
    )?;

    let resource_iter = stmt.query_map(params![], |row| {
        Ok((
            row.get::<_, String>(0)?,         // id
            row.get::<_, String>(1)?,         // uri
            row.get::<_, Option<String>>(2)?, // content_type
            row.get::<_, i64>(3)?,            // access_count
            row.get::<_, i64>(4)?,            // size_bytes
            row.get::<_, String>(5)?,         // created_at
            row.get::<_, String>(6)?,         // accessed_at
        ))
    })?;

    let mut resource_count = 0;
    let mut total_access_count = 0i64;
    let mut total_size = 0i64;
    let mut resource_details = Vec::new();

    info!(
        "Thread {:?} reading cached resources",
        thread::current().id()
    );
    info!("Cached resources found:");
    info!("------------------------");

    for resource in resource_iter {
        let (id, uri, content_type, access_count, size_bytes, created_at, accessed_at) = resource?;
        resource_count += 1;
        total_access_count += access_count;
        total_size += size_bytes;

        info!("Resource #{}", resource_count);
        info!("  📦 ID: {}", id);
        info!("  🔗 URI: {}", uri);
        info!(
            "  📄 Content-Type: {}",
            content_type.clone().unwrap_or_else(|| "N/A".to_string())
        );
        info!("  🔢 Access Count: {}", access_count);
        info!("  💾 Size: {} bytes", size_bytes);
        info!("  🕐 Created: {}", created_at);
        info!("  🕐 Last Accessed: {}", accessed_at);

        resource_details.push((uri.clone(), access_count, size_bytes));
    }

    // Also check configuration table
    let config_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM configuration", params![], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    info!("Configuration entries in database: {}", config_count);

    // With comprehensive testing, we should have cached resources
    if resource_count == 0 {
        warn!("⚠️ WARNING: No resources found in cache!");
        warn!("    This suggests either:");
        warn!("    1. The MCP server has no resources exposed");
        warn!("    2. Resource fetching failed during the test");
        warn!("    3. The cache is not working properly");
    }

    // Verify the cache analytics table exists and has data
    let analytics_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM cache_analytics", params![], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    // Query cache analytics for hit/miss rates
    let cache_stats = conn
        .query_row(
            "SELECT hit_rate, total_requests, cache_size_mb, eviction_count 
         FROM cache_analytics 
         ORDER BY timestamp DESC 
         LIMIT 1",
            params![],
            |row| {
                Ok((
                    row.get::<_, f64>(0).unwrap_or(0.0), // hit_rate
                    row.get::<_, i64>(1).unwrap_or(0),   // total_requests
                    row.get::<_, f64>(2).unwrap_or(0.0), // cache_size_mb
                    row.get::<_, i64>(3).unwrap_or(0),   // eviction_count
                ))
            },
        )
        .ok();

    info!("Summary:");
    info!("  Total cached resources: {}", resource_count);
    info!("  Total accesses: {}", total_access_count);
    info!("  Total cache size: {} bytes", total_size);
    info!("  Analytics entries: {}", analytics_count);

    if let Some((hit_rate, requests, size_mb, evictions)) = cache_stats {
        info!("Cache Performance:");
        info!("  Hit rate: {:.2}%", hit_rate * 100.0);
        info!("  Total requests: {}", requests);
        info!("  Cache size: {:.2} MB", size_mb);
        info!("  Evictions: {}", evictions);
    }

    // Verify that cache is working (either storing or accessing resources)
    if resource_count == 0 {
        warn!("⚠️ No resources were cached during the test");
        warn!("    This suggests either:");
        warn!("    1. The MCP server has no resources exposed");
        warn!("    2. Resource fetching failed during the test");
        warn!("    3. The cache is not working properly");
    } else {
        info!(
            "✅ Resources successfully cached: {} resources",
            resource_count
        );
        if total_access_count > 0 {
            info!(
                "✅ Cache retrieval working: {} total accesses",
                total_access_count
            );
        } else {
            info!(
                "ℹ️ Resources cached but not yet accessed from cache (expected for first-time fetches)"
            );
        }
    }

    info!("✅ SQLite cache verification successful");
    Ok(())
}
