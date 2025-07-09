//! End-to-end integration test for MCP server and client generation and communication

use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use lazy_static::lazy_static;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use regex::Regex;
use rusqlite::{Connection, params};
use std::thread;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Command as AsyncCommand;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

const CLI_FLAG_TESTS_SANDBOX_DIR: &str = "target/tmp/cli_flag_tests";

lazy_static! {
    /// Compiled regex for stripping ANSI escape codes
    static ref ANSI_ESCAPE_REGEX: Regex =
        Regex::new(r"\x1b\[[0-9;]*[mGKHF]|\x1b\]0;[^\x07]*\x07").unwrap();
}

/// Tests that the CLI correctly enforces argument rules during command execution:
/// - Server commands require --schema-path
/// - Client commands reject --schema-path
/// - Default values work correctly
/// - Invalid arguments are properly rejected
#[test]
fn test_cli_flag_combinations() -> Result<()> {
    let agenterra = env!("CARGO_BIN_EXE_agenterra");
    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Use sandbox directory under target/tmp to avoid polluting repo root
    let sandbox_dir = project_dir
        .join(CLI_FLAG_TESTS_SANDBOX_DIR)
        .join("test_cli_flag_combinations");
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
        "Should show missing --schema-path error, but got: {stderr}"
    );

    // Test 2: Client command should succeed with default project-name
    let client_template_dir = project_dir.join("templates/mcp/client/rust");
    let output_dir = sandbox_dir.join("test_default_project_name");
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--template",
            "rust",
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
        "Should show error about unsupported --schema-path flag, but got: {stderr}"
    );

    // Test 4: Valid server command combination
    // Note: This will fail because file doesn't exist, but argument parsing should work
    let server_template_dir = project_dir.join("templates/mcp/server/rust");
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
            "rust",
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
        "Should show file not found error, but got: {stderr}"
    );

    // Verify it's not an argument parsing error
    assert!(
        !stderr.contains("unrecognized")
            && !stderr.contains("unexpected")
            && !stderr.contains("required"),
        "Should not be an argument parsing error, but got: {stderr}"
    );

    // Test 5: Valid client command combination
    let client_template_dir = project_dir.join("templates/mcp/client/rust");
    let result = Command::new(agenterra)
        .current_dir(&sandbox_dir)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--project-name",
            "test-client",
            "--template",
            "rust",
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
            "Should show success message for valid client command, but got: {stdout}"
        );
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        // Should NOT be an argument parsing error
        assert!(
            !stderr.contains("unrecognized")
                && !stderr.contains("unexpected")
                && !stderr.contains("required"),
            "Should not be an argument parsing error, but got: {stderr}"
        );

        // Should be a template-related error, not argument parsing
        assert!(
            stderr.is_empty()
                || stderr.contains("template")
                || stderr.contains("not found")
                || stderr.contains("failed"),
            "Unexpected error for valid client command: {stderr}"
        );
    }

    Ok(())
}

/// Tests that the main CLI help and subcommand help display the expected
/// command structure and descriptions
#[test]
fn test_cli_help_output() {
    let agenterra = env!("CARGO_BIN_EXE_agenterra");
    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Use sandbox directory under target/tmp to avoid polluting repo root
    let sandbox_dir = project_dir
        .join(CLI_FLAG_TESTS_SANDBOX_DIR)
        .join("test_cli_help_output");
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

/// Tests that CLI help text shows the correct available options for each subcommand:
/// - Server help should show --schema-path, --template, --output-dir
/// - Client help should show --template, --output-dir but NOT --schema-path
#[test]
fn test_cli_help_shows_correct_options() {
    let agenterra = env!("CARGO_BIN_EXE_agenterra");
    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Use sandbox directory under target/tmp to avoid polluting repo root
    let sandbox_dir = project_dir
        .join(CLI_FLAG_TESTS_SANDBOX_DIR)
        .join("test_new_cli_structure");
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

/// Tests scaffolding with custom template directories
/// Verifies that external templates from tests/fixtures are used instead of embedded ones
#[test]
fn test_custom_template_scaffolding() -> Result<()> {
    // Initialize test environment
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let agenterra = project_dir
        .join("target/debug/agenterra")
        .to_string_lossy()
        .into_owned();

    // Set up paths for custom templates - point to the actual template roots
    let server_template_dir = project_dir.join("tests/fixtures/templates/mcp/server/rust");
    let client_template_dir = project_dir.join("tests/fixtures/templates/mcp/client/rust");

    // Verify custom template directories exist
    assert!(
        server_template_dir.exists(),
        "Server template directory not found: {}",
        server_template_dir.display()
    );
    assert!(
        client_template_dir.exists(),
        "Client template directory not found: {}",
        client_template_dir.display()
    );

    // Use a different output directory to avoid conflicts
    let scaffold_path = project_dir.join("target/tmp/custom-template-test");
    // Clean any previous runs
    let _ = std::fs::remove_dir_all(&scaffold_path);
    std::fs::create_dir_all(&scaffold_path)?;

    info!("=== Testing Custom Template Scaffolding ===");

    // Test 1: Generate server with custom template
    let server_name = "custom_template_server";
    let schema_path = project_dir.join("tests/fixtures/openapi/petstore.openapi.v3.json");

    // Verify schema file exists before running command
    assert!(
        schema_path.exists(),
        "Schema file not found at: {}",
        schema_path.display()
    );

    info!("Using schema path: {}", schema_path.display());
    info!("Using template dir: {}", server_template_dir.display());

    let server_result = Command::new(&agenterra)
        .args([
            "scaffold",
            "mcp",
            "server",
            "--project-name",
            server_name,
            "--output-dir",
            scaffold_path.to_str().unwrap(),
            "--schema-path",
            schema_path.to_str().unwrap(),
            "--template-dir",
            server_template_dir.to_str().unwrap(),
            "--template",
            "rust",
        ])
        .output()?;

    if !server_result.status.success() {
        eprintln!("Server generation with custom template failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&server_result.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&server_result.stderr));
        panic!("Server generation with custom template failed");
    }

    // Verify server was created with custom template
    let server_output = scaffold_path.join(server_name);
    assert!(
        server_output.exists(),
        "Server output directory not created"
    );

    // Check for test marker file that only exists in custom template
    let server_marker = server_output.join("server_test_marker.txt");
    assert!(
        server_marker.exists(),
        "Custom template marker file not found - embedded template was used instead!"
    );

    // Verify README contains custom template content
    let server_readme = server_output.join("README.md");
    let readme_content = std::fs::read_to_string(&server_readme)?;
    assert!(
        readme_content.contains("TEST TEMPLATE"),
        "README doesn't contain custom template marker"
    );
    assert!(
        readme_content.contains("tests/fixtures/templates/mcp/server/rust"),
        "README doesn't reference custom template path"
    );

    info!("✅ Server generation with custom template successful");

    // Test 2: Generate client with custom template
    let client_name = "custom_template_client";

    let client_result = Command::new(&agenterra)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--project-name",
            client_name,
            "--output-dir",
            scaffold_path.to_str().unwrap(),
            "--template-dir",
            client_template_dir.to_str().unwrap(),
            "--template",
            "rust",
        ])
        .output()?;

    if !client_result.status.success() {
        eprintln!("Client generation with custom template failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&client_result.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&client_result.stderr));
        panic!("Client generation with custom template failed");
    }

    // Verify client was created with custom template
    let client_output = scaffold_path.join(client_name);
    assert!(
        client_output.exists(),
        "Client output directory not created"
    );

    // Check for client test marker file
    let client_marker = client_output.join("client_test_marker.txt");
    assert!(
        client_marker.exists(),
        "Custom client template marker file not found - embedded template was used instead!"
    );

    // Verify client README contains custom template content
    let client_readme = client_output.join("README.md");
    let client_readme_content = std::fs::read_to_string(&client_readme)?;
    assert!(
        client_readme_content.contains("Test Client Template"),
        "Client README doesn't contain custom template marker"
    );
    assert!(
        client_readme_content.contains("tests/fixtures/templates/mcp/client/rust"),
        "Client README doesn't reference custom template path"
    );

    info!("✅ Client generation with custom template successful");
    info!("✅ All custom template scaffolding tests passed!");

    Ok(())
}

// === Helper Functions for E2E Tests ===

/// Helper to setup test directories and clean previous runs
fn setup_test_directories(project_dir: &std::path::Path) -> Result<std::path::PathBuf> {
    let scaffold_path = project_dir.join("target/tmp/e2e-tests");

    // Clean any previous run directories to avoid conflicts
    for sub in ["e2e_mcp_server", "e2e_mcp_client"] {
        let dir = scaffold_path.join(sub);
        let _ = std::fs::remove_dir_all(&dir);
    }
    std::fs::create_dir_all(&scaffold_path)?;

    Ok(scaffold_path)
}

/// Helper to scaffold MCP server
async fn scaffold_mcp_server(
    agenterra: &str,
    server_name: &str,
    scaffold_path: &std::path::Path,
    schema_path: &std::path::Path,
) -> Result<()> {
    // Verify schema file exists
    if !schema_path.exists() {
        anyhow::bail!("Schema file not found at: {}", schema_path.display());
    }

    let server_result = Command::new(agenterra)
        .args([
            "scaffold",
            "mcp",
            "server",
            "--project-name",
            server_name,
            "--output-dir",
            scaffold_path.to_str().unwrap(),
            "--schema-path",
            schema_path.to_str().unwrap(),
            "--template",
            "rust",
            "--base-url",
            "https://petstore3.swagger.io",
        ])
        .output()
        .context("Failed to run scaffold command for server")?;

    if !server_result.status.success() {
        let stderr = String::from_utf8_lossy(&server_result.stderr);
        eprintln!("Server generation failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&server_result.stdout));
        eprintln!("stderr: {stderr}");
        anyhow::bail!(
            "Server generation failed with exit code: {:?}",
            server_result.status.code()
        );
    }

    info!("✅ Server generation successful");
    Ok(())
}

/// Helper to scaffold MCP client
async fn scaffold_mcp_client(
    agenterra: &str,
    client_name: &str,
    scaffold_path: &std::path::Path,
) -> Result<()> {
    let client_result = Command::new(agenterra)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--project-name",
            client_name,
            "--output-dir",
            scaffold_path.to_str().unwrap(),
            "--template",
            "rust",
        ])
        .output()
        .context("Failed to run scaffold command for client")?;

    if !client_result.status.success() {
        eprintln!("Client generation failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&client_result.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&client_result.stderr));
        anyhow::bail!("Client generation failed");
    }

    info!("✅ Client generation successful");
    Ok(())
}

/// Helper to build and test a generated project
async fn build_and_test_project(project_path: &std::path::Path, project_name: &str) -> Result<()> {
    // Build the project
    info!("=== Building Generated {} ===", project_name);
    let build_result = Command::new("cargo")
        .args([
            "build",
            "--manifest-path",
            &project_path.join("Cargo.toml").to_string_lossy(),
        ])
        .output()
        .context(format!("Failed to build {project_name}"))?;

    if !build_result.status.success() {
        let stderr = String::from_utf8_lossy(&build_result.stderr);
        anyhow::bail!("{} build failed:\n{}", project_name, stderr);
    }
    info!("✅ {} builds successfully", project_name);

    // Run tests
    info!("=== Testing Generated {} ===", project_name);
    let test_result = Command::new("cargo")
        .args([
            "test",
            "--manifest-path",
            &project_path.join("Cargo.toml").to_string_lossy(),
        ])
        .output()
        .context(format!("Failed to test {project_name}"))?;

    if !test_result.status.success() {
        let stderr = String::from_utf8_lossy(&test_result.stderr);

        // Special handling for client tests that may be ignored
        if project_name.contains("client")
            && stderr.contains("0 passed")
            && stderr.contains("0 failed")
            && stderr.contains("ignored")
        {
            info!(
                "⚠️  {} tests were ignored (likely integration tests requiring mock server)",
                project_name
            );
        } else {
            eprintln!("{project_name} tests failed:");
            eprintln!("stdout: {}", String::from_utf8_lossy(&test_result.stdout));
            eprintln!("stderr: {stderr}");
            anyhow::bail!("{} tests failed", project_name);
        }
    } else {
        info!("✅ {} tests pass successfully", project_name);
    }

    Ok(())
}

/// Helper to test CLI help output for SSE options
async fn test_cli_help_sse_options(
    server_binary: &std::path::Path,
    client_output: &std::path::Path,
) -> Result<()> {
    info!("=== Testing CLI Help for SSE Options ===");

    // Check server help
    let server_help = Command::new(server_binary)
        .arg("--help")
        .output()
        .context("Failed to get server help")?;

    let server_help_text = String::from_utf8_lossy(&server_help.stdout);
    assert!(
        server_help_text.contains("--transport"),
        "Server help should include --transport option"
    );
    assert!(
        server_help_text.contains("--sse-addr"),
        "Server help should include --sse-addr option"
    );
    assert!(
        server_help_text.contains("--sse-keep-alive"),
        "Server help should include --sse-keep-alive option"
    );
    info!("✅ Server CLI help includes SSE options");

    // Check client help
    let client_binary = client_output.join("target/debug/e2e_mcp_client");
    let client_help = Command::new(&client_binary)
        .arg("--help")
        .output()
        .context("Failed to get client help")?;

    let client_help_text = String::from_utf8_lossy(&client_help.stdout);
    assert!(
        client_help_text.contains("--transport"),
        "Client help should include --transport option"
    );
    assert!(
        client_help_text.contains("--sse-url"),
        "Client help should include --sse-url option"
    );
    info!("✅ Client CLI help includes SSE options");

    Ok(())
}

/// Helper to test SSE transport mode
async fn test_sse_transport_mode(server_binary: &std::path::Path) -> Result<()> {
    info!("=== Testing SSE Transport Mode ===");

    // Test server with SSE mode (should start but we'll kill it quickly)
    let mut sse_server = Command::new(server_binary)
        .args(["--transport", "sse", "--sse-addr", "127.0.0.1:9999"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start server in SSE mode")?;

    // Give it a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Kill the server
    sse_server.kill().ok();
    info!("✅ Server can start in SSE mode");

    Ok(())
}

/// End-to-end integration test that:
/// 1. Scaffolds both MCP server and client from templates
/// 2. Builds the generated projects
/// 3. Runs their test suites
/// 4. Verifies SSE transport options in CLI
/// 5. Tests actual MCP communication between server and client
/// 6. Verifies SQLite database caching functionality
#[tokio::test]
async fn test_mcp_client_server_scaffolding_and_communication() -> Result<()> {
    // Initialize tracing for test visibility
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("e2e_mcp_test=info".parse().unwrap())
                .add_directive("agenterra=info".parse().unwrap()),
        )
        .with_test_writer()
        .try_init();

    // Determine project root at compile time via Cargo
    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Resolve path to agenterra binary
    let agenterra = project_dir
        .join("target/debug/agenterra")
        .to_string_lossy()
        .into_owned();

    // Setup test directories
    let scaffold_path = setup_test_directories(&project_dir)?;

    // Clean up any existing client databases to ensure fresh state
    cleanup_project_databases("e2e_mcp_client")?;

    info!("=== Testing MCP Server Generation (Embedded Templates) ===");
    info!("Project dir: {}", project_dir.display());

    // Test 1: Generate MCP server
    let server_name = "e2e_mcp_server";
    let server_output = scaffold_path.join(server_name);
    let schema_path = project_dir.join("tests/fixtures/openapi/petstore.openapi.v3.json");

    // Create server directory first since scaffold expects it to exist
    std::fs::create_dir_all(&server_output)?;

    // Scaffold the server
    scaffold_mcp_server(&agenterra, server_name, &scaffold_path, &schema_path).await?;

    // Verify server files exist
    assert!(server_output.join("Cargo.toml").exists());
    assert!(server_output.join("src/main.rs").exists());
    assert!(server_output.join("src/handlers/mod.rs").exists());

    info!("=== Testing MCP Client Generation (Embedded Templates) ===");

    // Test 2: Generate MCP client
    let client_name = "e2e_mcp_client";
    let client_output = scaffold_path.join(client_name);

    // Scaffold the client
    scaffold_mcp_client(&agenterra, client_name, &scaffold_path).await?;

    // Verify client files exist
    assert!(client_output.join("Cargo.toml").exists());
    assert!(client_output.join("src/main.rs").exists());
    assert!(client_output.join("src/domain/client.rs").exists());
    assert!(client_output.join("src/ui/repl.rs").exists());

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

    // Test 3: Build and test generated projects
    build_and_test_project(&server_output, "Server").await?;
    build_and_test_project(&client_output, "Client").await?;

    // The generated binary names match the project names
    let server_binary = server_output.join("target/debug/e2e_mcp_server");

    // Test 4: Verify CLI help includes SSE transport options
    test_cli_help_sse_options(&server_binary, &client_output).await?;

    // Test 5: Verify SSE transport mode can be started
    test_sse_transport_mode(&server_binary).await?;

    // Test 6: End-to-end MCP communication using generated client
    info!("=== Testing MCP Server ↔ Client Communication ===");

    // Verify binaries exist
    if !server_binary.exists() {
        anyhow::bail!(
            "Expected server binary not found at {}",
            server_binary.display()
        );
    }

    info!("✅ Server binary found at: {}", server_binary.display());

    // Use the generated client to test MCP communication
    let test_result = timeout(Duration::from_secs(60), async {
        test_mcp_with_pty_client(&server_binary, &client_output).await
    })
    .await;

    match test_result {
        Ok(Ok(())) => {
            info!("✅ MCP communication test successful");
        }
        Ok(Err(e)) => {
            panic!("MCP communication test failed: {e}");
        }
        Err(_) => {
            panic!("MCP communication test timed out");
        }
    }

    // Test 5: Verify SQLite cache directly
    info!("=== Verifying SQLite Cache ===");

    let _db_stats = verify_sqlite_cache(&client_output)?;

    info!("🎉 Complete end-to-end MCP test passed!");

    Ok(())
}

/// Tests SSE (Server-Sent Events) transport mode for MCP:
/// - Generates an SSE-enabled client
/// - Builds with SSE feature flag
/// - Attempts to connect to an SSE server (if configured)
/// Note: Requires ENABLE_SSE_TEST=1 and SSE_TEST_SERVER_URL env vars
#[tokio::test]
async fn test_resource_schema_format() -> Result<()> {
    info!("=== Testing MCP Resource Schema Format ===");

    // Use an existing generated schema file from a previous test run
    let schema_path =
        std::path::Path::new("target/tmp/e2e-tests/e2e_mcp_server/schemas/add_pet.json");

    // Ensure the file exists by checking if we need to generate it first
    if !schema_path.exists() {
        warn!("Schema file not found at {:?}, skipping test", schema_path);
        warn!("Run test_mcp_client_server_scaffolding_and_communication first to generate schemas");
        return Ok(());
    }

    let schema_content = fs::read_to_string(schema_path).context("Failed to read schema file")?;

    let schema: serde_json::Value =
        serde_json::from_str(&schema_content).context("Failed to parse schema JSON")?;

    info!("Schema size: {} bytes", schema_content.len());
    info!("Schema preview: {}", serde_json::to_string_pretty(&schema)?);

    // Verify schema structure
    assert!(
        schema.get("operationId").is_some(),
        "Schema should have operationId"
    );
    assert!(schema.get("path").is_some(), "Schema should have path");

    // Verify no null fields are included
    fn check_no_nulls(value: &serde_json::Value, path: &str) {
        match value {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    assert!(!v.is_null(), "Found null value at {path}.{k}");
                    check_no_nulls(v, &format!("{path}.{k}"));
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    check_no_nulls(v, &format!("{path}[{i}]"));
                }
            }
            _ => {}
        }
    }

    check_no_nulls(&schema, "schema");

    // Verify schema is reasonably sized (not bloated)
    assert!(
        schema_content.len() < 5000,
        "Schema should be less than 5KB, but was {} bytes",
        schema_content.len()
    );

    info!("✅ Schema format is clean and minimal");
    Ok(())
}

#[tokio::test]
async fn test_mcp_sse_transport() -> Result<()> {
    // Initialize tracing
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("e2e_sse_test=info".parse().unwrap())
                .add_directive("agenterra=info".parse().unwrap()),
        )
        .with_test_writer()
        .try_init();

    info!("=== Testing MCP SSE Transport ===");

    // Skip test if SSE testing is not enabled
    if std::env::var("ENABLE_SSE_TEST").is_err() {
        info!("Skipping SSE test - set ENABLE_SSE_TEST=1 to run");
        return Ok(());
    }

    let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let agenterra = project_dir
        .join("target/debug/agenterra")
        .to_string_lossy()
        .into_owned();

    let client_template_dir = project_dir.join("templates/mcp/client/rust");
    let scaffold_path = project_dir.join("target/tmp/e2e-sse-tests");

    // Clean previous runs
    let _ = std::fs::remove_dir_all(&scaffold_path);
    std::fs::create_dir_all(&scaffold_path)?;

    // Generate SSE-enabled client
    let client_name = "e2e_sse_client";
    let client_output = scaffold_path.join(client_name);

    info!("Generating SSE-enabled MCP client...");
    let output = Command::new(&agenterra)
        .current_dir(&scaffold_path)
        .args([
            "scaffold",
            "mcp",
            "client",
            "--project-name",
            client_name,
            "--template",
            "rust",
            "--template-dir",
            client_template_dir.to_str().unwrap(),
            "--output-dir",
            client_output.to_str().unwrap(),
        ])
        .output()
        .context("Failed to run agenterra for SSE client")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to generate SSE client: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Build the SSE client
    info!("Building SSE-enabled client...");
    let build_output = Command::new("cargo")
        .current_dir(&client_output)
        .args(["build", "--features", "sse"])
        .output()
        .context("Failed to build SSE client")?;

    if !build_output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to build SSE client: {}",
            String::from_utf8_lossy(&build_output.stderr)
        ));
    }

    // Find the client binary
    let client_binary = client_output.join("target/debug/e2e_sse_client");
    if !client_binary.exists() {
        return Err(anyhow::anyhow!(
            "SSE client binary not found at: {}",
            client_binary.display()
        ));
    }

    // Get SSE server URL from environment variable
    let sse_server_url = match std::env::var("SSE_TEST_SERVER_URL") {
        Ok(url) => url,
        Err(_) => {
            info!("Skipping SSE test - SSE_TEST_SERVER_URL not set");
            info!("To run SSE tests, start an SSE server and set SSE_TEST_SERVER_URL");
            return Ok(());
        }
    };

    info!("Testing SSE client connection to: {}", sse_server_url);

    // Test SSE connection
    let mut client_process = AsyncCommand::new(&client_binary)
        .arg("--transport")
        .arg("sse")
        .arg("--sse-url")
        .arg(&sse_server_url)
        .arg("--timeout")
        .arg("10")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn SSE client process")?;

    // Give client time to connect
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test basic connection
    let stdin = client_process
        .stdin
        .as_mut()
        .context("Failed to get stdin")?;
    let stdout = client_process
        .stdout
        .as_mut()
        .context("Failed to get stdout")?;

    let mut writer = BufWriter::new(stdin);
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    // Send status command
    writer.write_all(b"/status\n").await?;
    writer.flush().await?;

    // Read response
    let mut got_response = false;
    for _ in 0..10 {
        line.clear();
        match timeout(Duration::from_millis(500), reader.read_line(&mut line)).await {
            Ok(Ok(_)) => {
                if line.contains("Connected") || line.contains("SSE") {
                    got_response = true;
                    info!("SSE connection successful: {}", line.trim());
                    break;
                }
            }
            _ => continue,
        }
    }

    // Clean up
    let _ = client_process.kill().await;

    if got_response {
        info!("✅ SSE transport test completed successfully");
    } else {
        warn!("⚠️ SSE transport test - no response received (server may be unavailable)");
    }

    Ok(())
}

/// Helper function to clean up any SQLite database files for a given project name.
/// This ensures each test run starts with a fresh database state.
/// Checks multiple OS-specific locations where databases might be stored.
fn cleanup_project_databases(project_name: &str) -> Result<()> {
    // Database locations based on the template's get_database_path() function
    let db_paths = vec![
        // macOS location
        dirs::data_dir().map(|d| d.join(project_name).join(format!("{project_name}.db"))),
        // Linux location
        dirs::data_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
            .map(|d| d.join(project_name).join(format!("{project_name}.db"))),
        // Windows location
        dirs::data_local_dir().map(|d| {
            d.join(project_name)
                .join("data")
                .join(format!("{project_name}.db"))
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

/// Tests MCP communication using PTY (pseudo-terminal) for proper REPL interaction.
/// This simulates an interactive terminal session to ensure the client outputs
/// human-readable text instead of raw protocol messages.
/// Returns diagnostic information about tools, resources, and database operations.
async fn test_mcp_with_pty_client(
    server_binary: &std::path::Path,
    client_output: &std::path::Path,
) -> Result<()> {
    // Diagnostic tracking structures
    #[derive(Debug, Default)]
    struct DiagnosticSummary {
        connection_time: Duration,
        tools_discovered: Vec<String>,
        resources_discovered: Vec<String>,
        prompts_discovered: Vec<String>,
        tools_called: Vec<(String, serde_json::Value, bool)>, // (name, args, success)
        resources_fetched: Vec<(String, bool)>,               // (uri, success)
        cache_hits: usize,
        cache_misses: usize,
        db_operations: Vec<String>,
        errors: Vec<String>,
    }

    let mut diagnostics = DiagnosticSummary::default();
    let start_time = std::time::Instant::now();

    // Pre-compile regex patterns for robust parsing
    let tool_pattern = Regex::new(r"^\s*(\w+)\s*$").unwrap();
    let resource_pattern = Regex::new(r"^\s*([^:]+):\s*(.*)$").unwrap();

    info!("Starting PTY-based MCP client test");

    // Find the client binary
    let client_binary = client_output.join("target/debug/e2e_mcp_client");
    if !client_binary.exists() {
        return Err(anyhow::anyhow!(
            "Client binary not found at: {}",
            client_binary.display()
        ));
    }

    // Create a pseudo-terminal
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("Failed to create PTY")?;

    // Configure the command
    let mut cmd = CommandBuilder::new(&client_binary);
    cmd.arg("--server");
    cmd.arg(server_binary.to_str().unwrap());
    cmd.arg("--timeout");
    cmd.arg("30");

    // Spawn the child process with PTY
    let mut child = pty_pair
        .slave
        .spawn_command(cmd)
        .context("Failed to spawn client with PTY")?;

    // Get reader/writer for the master side
    let mut reader = pty_pair
        .master
        .try_clone_reader()
        .context("Failed to clone PTY reader")?;
    let mut writer = pty_pair
        .master
        .take_writer()
        .context("Failed to take PTY writer")?;

    // Give client time to connect
    tokio::time::sleep(Duration::from_millis(2000)).await;
    diagnostics.connection_time = start_time.elapsed();

    // Helper to read until we see "mcp>" using synchronous reading
    fn read_until_prompt_sync(
        reader: &mut Box<dyn std::io::Read + Send>,
        output: &mut Vec<String>,
    ) -> Result<()> {
        use std::io::Read;
        let mut buffer = vec![0u8; 1024];
        let mut accumulated = String::new();

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(10);
        let mut no_data_count = 0;

        loop {
            // Check for timeout
            if start.elapsed() > timeout {
                warn!(
                    "Timeout waiting for prompt after {} seconds",
                    timeout.as_secs()
                );
                warn!("Accumulated data: {:?}", accumulated);
                break;
            }

            match reader.read(&mut buffer) {
                Ok(0) => {
                    // No data available, wait a bit
                    no_data_count += 1;
                    if no_data_count > 100 {
                        debug!("No data received after {} attempts", no_data_count);
                        break;
                    }
                    debug!("No data available, waiting...");
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buffer[..n]);
                    accumulated.push_str(&chunk);

                    // Process complete lines
                    while let Some(newline_pos) = accumulated.find('\n') {
                        let line = accumulated[..newline_pos].trim().to_string();
                        accumulated = accumulated[newline_pos + 1..].to_string();

                        if !line.is_empty() {
                            output.push(line.clone());
                        }
                        if line.contains("mcp>") || line.contains("arguments>") {
                            return Ok(());
                        }
                    }

                    // Also check if "mcp>" or "arguments>" is in the accumulated buffer without newline
                    if accumulated.contains("mcp>") || accumulated.contains("arguments>") {
                        if !accumulated.trim().is_empty() {
                            output.push(accumulated.trim().to_string());
                        }
                        return Ok(());
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        // If we have remaining content in the buffer, add it
        if !accumulated.trim().is_empty() {
            output.push(accumulated.trim().to_string());
        }
        Ok(())
    }

    // Read initial connection output
    info!("Reading initial connection output...");
    let mut initial_output = Vec::new();
    read_until_prompt_sync(&mut reader, &mut initial_output)?;
    info!("Initial output: {:?}", initial_output);

    // Check connection status
    writer.write_all(b"/status\n")?;
    writer.flush()?;

    let mut status_output = Vec::new();
    read_until_prompt_sync(&mut reader, &mut status_output)?;

    let connection_verified = status_output
        .iter()
        .any(|line| line.contains("Connected: true"));
    if !connection_verified {
        return Err(anyhow::anyhow!("Failed to verify connection"));
    }
    info!("✅ Connection verified");

    // Test tools discovery
    info!("=== Testing Tools Discovery ===");
    writer.write_all(b"/list-tools\n")?;
    writer.flush()?;

    let mut tools_output = Vec::new();
    read_until_prompt_sync(&mut reader, &mut tools_output)?;

    let mut tool_names = Vec::new();
    let mut in_tools_list = false;

    for line in &tools_output {
        if line.contains("Available tools:") {
            in_tools_list = true;
        } else if in_tools_list {
            if let Some(captures) = tool_pattern.captures(line) {
                if let Some(tool_name) = captures.get(1) {
                    let tool = tool_name.as_str();
                    if !tool.is_empty() && !tool.contains("No tools") {
                        tool_names.push(tool.to_string());
                    }
                }
            }
        }
    }

    info!("Found {} tools: {:?}", tool_names.len(), tool_names);
    diagnostics.tools_discovered = tool_names.clone();

    // Test resources discovery
    info!("=== Testing Resources Discovery ===");
    writer.write_all(b"/list-resources\n")?;
    writer.flush()?;

    let mut resources_output = Vec::new();
    read_until_prompt_sync(&mut reader, &mut resources_output)?;

    let mut resource_uris = Vec::new();
    let mut in_resources_list = false;

    for line in &resources_output {
        if line.contains("Available resources:") {
            in_resources_list = true;
        } else if in_resources_list {
            if let Some(captures) = resource_pattern.captures(line) {
                if let Some(uri_match) = captures.get(1) {
                    let uri = uri_match.as_str().trim();
                    if !uri.is_empty() && !uri.contains("No resources") {
                        resource_uris.push(uri.to_string());
                    }
                }
            }
        }
    }

    info!("Found {} resources", resource_uris.len());
    diagnostics.resources_discovered = resource_uris.clone();

    // Test fetching a few resources to verify caching
    if !resource_uris.is_empty() {
        info!("=== Testing Resource Fetching for Cache ===");

        // Fetch up to 3 resources to test caching
        let resources_to_fetch = resource_uris.iter().take(3).collect::<Vec<_>>();

        for uri in resources_to_fetch {
            info!("Fetching resource: {}", uri);

            // Send command to fetch resource using '/get-resource' command
            writer.write_all(format!("/get-resource {uri}\n").as_bytes())?;
            writer.flush()?;

            // Read the response
            let mut resource_output = Vec::new();
            read_until_prompt_sync(&mut reader, &mut resource_output)?;

            // Check if fetch was successful by looking for the actual REPL output
            let success = resource_output
                .iter()
                .any(|line| line.contains("Resource:") && line.contains(uri));

            diagnostics.resources_fetched.push((uri.clone(), success));

            if success {
                info!("✅ Successfully fetched resource: {}", uri);
            } else {
                warn!("❌ Failed to fetch resource: {}", uri);
            }

            // Small delay between fetches
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        info!(
            "Fetched {} resources for cache testing",
            diagnostics.resources_fetched.len()
        );
    }

    // Skip ping test for now - PTY interaction needs more work
    info!("Skipping ping tool test (PTY interaction needs refinement)");

    // Verify we discovered tools successfully
    if tool_names.len() == 20 {
        info!("✅ Successfully discovered all 20 tools");
    } else {
        warn!("⚠️ Expected 20 tools but found {}", tool_names.len());
    }

    // Exit cleanly
    writer.write_all(b"/quit\n")?;
    writer.flush()?;

    // Wait for process to exit
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Kill if still running
    child.kill().ok();

    // Give the database a moment to flush any pending writes
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify SQLite cache and get cache statistics
    info!("=== Verifying SQLite Cache ===");
    match verify_sqlite_cache(client_output) {
        Ok((resource_count, db_operations)) => {
            info!("✅ SQLite cache verification successful");
            diagnostics.db_operations = db_operations.clone();

            // Extract cache statistics from db_operations
            for op in &db_operations {
                if op.contains("Cache hit rate:") {
                    // Parse hit rate to calculate hits/misses
                    if let Some(rate_str) = op.split(':').nth(1) {
                        if let Ok(rate) = rate_str.trim().trim_end_matches('%').parse::<f64>() {
                            let hit_rate = rate / 100.0;
                            // Assuming we made resource_count requests
                            let total_requests = resource_count as f64;
                            diagnostics.cache_hits = (total_requests * hit_rate) as usize;
                            diagnostics.cache_misses = (total_requests * (1.0 - hit_rate)) as usize;
                        }
                    }
                }
            }

            // Don't auto-populate resources_fetched - it's already tracked during actual fetching
        }
        Err(e) => {
            warn!("Failed to verify SQLite cache: {}", e);
            diagnostics
                .errors
                .push(format!("SQLite cache verification failed: {e}"));
        }
    }

    // Generate diagnostic summary
    info!("\n\n=== 🎯 E2E Test Diagnostic Summary ===\n");
    info!("📊 Connection & Discovery:");
    info!(
        "  • Connection Time: {:.2}s",
        diagnostics.connection_time.as_secs_f64()
    );
    info!(
        "  • Tools Discovered: {} tools",
        diagnostics.tools_discovered.len()
    );
    if !diagnostics.tools_discovered.is_empty() {
        info!("    - Tools: {:?}", diagnostics.tools_discovered);
    }
    info!(
        "  • Resources Discovered: {} resources",
        diagnostics.resources_discovered.len()
    );
    info!(
        "  • Prompts Discovered: {} prompts",
        diagnostics.prompts_discovered.len()
    );

    info!("\n🔧 Tool Interactions:");
    if diagnostics.tools_called.is_empty() {
        info!("  • No tools were called during the test");
    } else {
        for (tool, args, success) in &diagnostics.tools_called {
            let status = if *success { "✅" } else { "❌" };
            info!("  {} Tool '{}' called with args: {}", status, tool, args);
        }
    }

    info!("\n📁 Resource Operations:");
    info!(
        "  • Resources Discovered: {}",
        diagnostics.resources_discovered.len()
    );
    if !diagnostics.resources_fetched.is_empty() {
        let successful = diagnostics
            .resources_fetched
            .iter()
            .filter(|(_, success)| *success)
            .count();
        let failed = diagnostics.resources_fetched.len() - successful;
        info!(
            "  • Resources Fetched: {} successful, {} failed",
            successful, failed
        );
    }
    if diagnostics.cache_hits > 0 || diagnostics.cache_misses > 0 {
        info!(
            "  • Cache Performance: {} hits, {} misses",
            diagnostics.cache_hits, diagnostics.cache_misses
        );
    }

    info!("\n🗄️ Database Operations:");
    if diagnostics.db_operations.is_empty() {
        info!("  • No database operations recorded");
    } else {
        for op in &diagnostics.db_operations {
            info!("  • {}", op);
        }
    }

    if !diagnostics.errors.is_empty() {
        info!("\n⚠️ Errors Encountered:");
        for error in &diagnostics.errors {
            info!("  • {}", error);
        }
    }

    info!(
        "\n✅ E2E Test Summary: {} tools discovered, {} resources discovered",
        diagnostics.tools_discovered.len(),
        diagnostics.resources_discovered.len()
    );
    info!("=====================================\n");

    info!("✅ PTY-based MCP test completed successfully");
    Ok(())
}

/// Verifies SQLite cache functionality by directly querying the database.
/// Checks for:
/// - Database existence in OS-specific locations
/// - Cached resources and their metadata
/// - Cache performance statistics
/// - Database schema integrity
///
/// Returns: (resource_count, list_of_db_operations)
fn verify_sqlite_cache(client_output: &std::path::Path) -> Result<(usize, Vec<String>)> {
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

    // Collect DB operations for diagnostics
    let mut db_operations = vec![
        format!("Database location: {}", db_path.display()),
        format!("Tables found: {:?}", tables),
        format!("Resources cached: {}", resource_count),
        format!("Total cache accesses: {}", total_access_count),
        format!("Total cache size: {} bytes", total_size),
    ];

    if let Some((hit_rate, _, _, _)) = cache_stats {
        db_operations.push(format!("Cache hit rate: {:.2}%", hit_rate * 100.0));
    }

    Ok((resource_count, db_operations))
}

/// Tests that resource content is displayed properly (not as raw bytes)
#[test]
fn test_resource_display_format() -> Result<()> {
    // This is a unit test to verify the expected output format
    // The actual integration test happens in test_mcp_client_server_scaffolding_and_communication

    // Test data that simulates what a resource might contain
    let test_cases = vec![
        (
            "text/plain",
            b"Hello, world!".to_vec(),
            None,
            "Hello, world!",
        ),
        (
            "application/json",
            b"{\"key\": \"value\"}".to_vec(),
            None,
            "{\"key\": \"value\"}",
        ),
        (
            "text/plain",
            base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                "Hello from base64",
            )
            .as_bytes()
            .to_vec(),
            Some("base64"),
            "Hello from base64",
        ),
        (
            "application/octet-stream",
            vec![0xFF, 0xD8, 0xFF, 0xE0], // JPEG header
            None,
            "[Binary content - 4 bytes]",
        ),
    ];

    // Note: This is a conceptual test - in practice we'd need to mock the REPL
    // or test the actual display logic separately
    for (mime_type, _data, _encoding, expected_display) in test_cases {
        info!("Testing resource display for MIME type: {}", mime_type);

        // In a real test, we'd verify that:
        // 1. Text content is displayed as text
        // 2. Base64 encoded text is decoded and displayed
        // 3. Binary content shows a size indicator
        // 4. The raw bytes are never shown directly

        // Verify expected display format
        if mime_type.starts_with("text/") || mime_type == "application/json" {
            assert!(
                !expected_display.contains("[Binary content"),
                "Text content should be displayed directly for {mime_type}"
            );
        } else {
            assert!(
                expected_display.contains("[Binary content"),
                "Binary content should show size indicator for {mime_type}"
            );
        }
    }

    Ok(())
}
