//! End-to-end integration test for MCP server and client generation and communication

use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Command as AsyncCommand;
use tokio::time::timeout;

#[tokio::test]
async fn test_mcp_server_client_generation() -> Result<()> {
    // Discover project root first
    let current_dir = std::env::current_dir()?;
    let project_dir = current_dir
        .parent()
        .unwrap() // Go up from crates/agenterra-cli
        .parent()
        .unwrap(); // Go up to project root

    // Resolve path to agenterra binary (prefer Cargo-built path)
    let agenterra = project_dir
        .join("target/debug/agenterra")
        .to_string_lossy()
        .into_owned();

    // Pass project root as template dir - the code will append "templates" internally
    let template_dir = project_dir;

    // Use workspace .agenterra directory for generated artifacts
    // Clean any previous run directories to avoid duplicate headers or build conflicts
    let scaffold_path = project_dir.join(".agenterra");
    // Clean any previous run directories to avoid conflicts
    for sub in ["test_server", "test_client", "debug_client"] {
        let dir = scaffold_path.join(sub);
        let _ = fs::remove_dir_all(&dir);
    }
    std::fs::create_dir_all(&scaffold_path)?;

    println!("=== Testing MCP Server Generation ===");
    println!("Project dir: {}", project_dir.display());
    println!("Template dir: {}", template_dir.display());
    println!("Expected template path: {}/templates/mcp/server/rust_axum", template_dir.display());

    // Test 1: Generate MCP server
    let server_name = "test_server";
    let server_output = scaffold_path.join(server_name);
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
            server_output.to_str().unwrap(),
            "--schema-path",
            schema_path.to_str().unwrap(),
            "--template-dir",
            template_dir.to_str().unwrap(),
            "--template",
            "rust_axum",
            "--base-url",
            "https://petstore3.swagger.io",
        ])
        .output()?;

    println!("Server generation stdout: {}", String::from_utf8_lossy(&server_result.stdout));
    if !server_result.stderr.is_empty() {
        println!("Server generation stderr: {}", String::from_utf8_lossy(&server_result.stderr));
    }

    if !server_result.status.success() {
        panic!("Server generation failed with exit code: {:?}", server_result.status.code());
    }

    // Verify server files exist
    assert!(server_output.join("Cargo.toml").exists());
    assert!(server_output.join("src/main.rs").exists());
    assert!(server_output.join("src/handlers/mod.rs").exists());

    println!("✅ Server generation successful");

    println!("\n=== Testing MCP Client Generation ===");

    // Test 2: Generate MCP client
    let client_name = "test_client";
    let client_output = scaffold_path.join(client_name);
    let client_result = Command::new(&agenterra)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--project-name",
            client_name,
            "--output-dir",
            client_output.to_str().unwrap(),
            "--template-dir",
            template_dir.to_str().unwrap(),
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

    println!("✅ Client generation successful");

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
    println!("\n=== Building Generated Server ===");

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

    println!("✅ Server builds successfully");

    println!("\n=== Building Generated Client ===");

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

    println!("✅ Client builds successfully");

    // Test 4: End-to-end MCP communication using generated client
    println!("\n=== Testing MCP Server ↔ Client Communication ===");

    // The generated binary name matches the project name we passed ("test_server")
    let server_binary = server_output.join("target/debug/test_server");
    if !server_binary.exists() {
        anyhow::bail!(
            "Expected server binary not found at {}",
            server_binary.display()
        );
    }

    println!("✅ Server binary found at: {}", server_binary.display());

    // Generate a debug client with the fixed template
    let debug_client_path = scaffold_path.join("debug_client");
    println!("Generating debug client...");
    let debug_client_result = Command::new(&agenterra)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--project-name",
            client_name,
            "--output-dir",
            debug_client_path.to_str().unwrap(),
            "--template-dir",
            template_dir.to_str().unwrap(),
            "--template",
            "rust_reqwest",
        ])
        .output()?;

    if !debug_client_result.status.success() {
        eprintln!("Debug client generation failed:");
        eprintln!(
            "stdout: {}",
            String::from_utf8_lossy(&debug_client_result.stdout)
        );
        eprintln!(
            "stderr: {}",
            String::from_utf8_lossy(&debug_client_result.stderr)
        );
        panic!("Debug client generation failed");
    }

    // Ensure standalone debug client crate by adding workspace header once
    let cargo_toml = debug_client_path.join("Cargo.toml");
    if let Ok(contents) = fs::read_to_string(&cargo_toml) {
        if !contents.contains("[workspace]") {
            if let Ok(mut f) = OpenOptions::new().append(true).open(&cargo_toml) {
                writeln!(f, "\n[workspace]\n").ok();
            }
        }
    }

    // Build the debug client
    println!("Building debug client...");
    let client_build = Command::new("cargo")
        .args([
            "build",
            "--manifest-path",
            &debug_client_path.join("Cargo.toml").to_string_lossy(),
        ])
        .output()?;

    if !client_build.status.success() {
        eprintln!("Debug client build failed:");
        eprintln!("stderr: {}", String::from_utf8_lossy(&client_build.stderr));
        panic!("Debug client build failed");
    }

    // Use the generated client to test MCP communication
    let test_result = timeout(Duration::from_secs(60), async {
        test_mcp_with_interactive_client(&server_binary, &debug_client_path).await
    })
    .await;

    match test_result {
        Ok(Ok(())) => {
            println!("✅ MCP communication test successful");
        }
        Ok(Err(e)) => {
            panic!("MCP communication test failed: {}", e);
        }
        Err(_) => {
            panic!("MCP communication test timed out");
        }
    }

    println!("\n🎉 Complete end-to-end MCP test passed!");

    Ok(())
}

/// Test MCP communication using the generated client's interactive REPL
async fn test_mcp_with_interactive_client(
    server_binary: &std::path::Path,
    client_output: &std::path::Path,
) -> Result<()> {
    println!("Starting interactive MCP client test...");

    // Find the client binary
    let client_binary = client_output.join("target/debug/test_client");
    if !client_binary.exists() {
        return Err(anyhow::anyhow!(
            "Client binary not found at: {}",
            client_binary.display()
        ));
    }

    // Start the client with the server binary path
    println!("Starting MCP client: {}", client_binary.display());
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
    let mut output_lines = Vec::new();

    // Read multiple lines of initial output with timeout
    for _ in 0..10 {
        line.clear();
        match timeout(Duration::from_millis(500), reader.read_line(&mut line)).await {
            Ok(Ok(0)) => break, // EOF
            Ok(Ok(_)) => {
                output_lines.push(line.clone());
                println!("Client output: {}", line.trim());
            }
            Ok(Err(_)) | Err(_) => break, // Error or timeout
        }
    }

    // Send 'tools' command to list available tools
    println!("Sending 'tools' command...");
    writer
        .write_all(b"tools\n")
        .await
        .context("Failed to write tools command")?;
    writer.flush().await.context("Failed to flush writer")?;

    // Read tools response
    let mut tools_found = false;
    let mut tools_list = Vec::new();

    for _ in 0..10 {
        line.clear();
        match timeout(Duration::from_secs(2), reader.read_line(&mut line)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(_)) => {
                println!("Tools response: {}", line.trim());
                if line.contains("Available tools:") || line.contains("Tools:") {
                    tools_found = true;
                }
                if line.trim().starts_with("-")
                    || line.trim().starts_with("*")
                    || line.contains(":")
                {
                    tools_list.push(line.trim().to_string());
                }
                if line.contains("mcp>") {
                    break; // Back to prompt
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }

    if !tools_found && tools_list.is_empty() {
        return Err(anyhow::anyhow!("No tools found or tools command failed"));
    }

    println!(
        "✅ Tools discovery successful. Found {} tools",
        tools_list.len()
    );
    for tool in &tools_list {
        println!("  - {}", tool);
    }

    // Send 'call ping' command to test tool invocation
    println!("Sending 'call ping' command...");
    writer
        .write_all(b"call ping\n")
        .await
        .context("Failed to write call command")?;
    writer.flush().await.context("Failed to flush writer")?;

    // Read call response - look specifically for Tool result
    let mut call_successful = false;
    let mut found_tool_result = false;

    for _ in 0..20 {
        line.clear();
        match timeout(Duration::from_secs(5), reader.read_line(&mut line)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(_)) => {
                println!("Call response: {}", line.trim());
                
                // Check if we're in the tool result section
                if line.contains("Tool result:") {
                    found_tool_result = true;
                }
                
                // If we found tool result marker, check for success indicators
                if found_tool_result && (
                    line.contains("success")
                    || line.contains("alive")
                    || line.contains("pong")
                    || line.contains("result")
                    || line.contains("status")
                    || line.contains("ok")
                ) {
                    call_successful = true;
                }
                
                if line.contains("mcp>") {
                    break; // Back to prompt
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }

    // Send 'quit' to exit cleanly
    writer.write_all(b"quit\n").await.ok();
    writer.flush().await.ok();

    // Clean up client process
    if let Err(e) = client_process.kill().await {
        eprintln!("Warning: Failed to kill client process: {}", e);
    }

    if !call_successful {
        return Err(anyhow::anyhow!("Tool call did not appear to succeed"));
    }

    println!("✅ Tool call successful");
    Ok(())
}

#[test]
fn test_cli_help_output() {
    let agenterra = env!("CARGO_BIN_EXE_agenterra");

    // Test main help
    let result = Command::new(&agenterra)
        .arg("--help")
        .output()
        .expect("Failed to run agenterra");

    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("scaffold"));
    assert!(output.contains("Scaffold MCP servers and clients"));

    // Test scaffold mcp help
    let result = Command::new(&agenterra)
        .args(["scaffold", "mcp", "--help"])
        .output()
        .expect("Failed to run agenterra");

    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("server"));
    assert!(output.contains("client"));
    assert!(output.contains("Generate MCP server from OpenAPI specification"));
    assert!(output.contains("Generate MCP client"));
}

#[test]
fn test_new_cli_structure() {
    let agenterra = env!("CARGO_BIN_EXE_agenterra");

    // Test server help shows correct options
    let result = Command::new(&agenterra)
        .args(["scaffold", "mcp", "server", "--help"])
        .output()
        .expect("Failed to run agenterra");

    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("--schema-path"));
    assert!(output.contains("--template"));
    assert!(output.contains("--output-dir"));

    // Test client help shows correct options
    let result = Command::new(&agenterra)
        .args(["scaffold", "mcp", "client", "--help"])
        .output()
        .expect("Failed to run agenterra");

    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("--template"));
    assert!(output.contains("--output-dir"));
    // Client should NOT have schema-path
    assert!(!output.contains("--schema-path"));
}
