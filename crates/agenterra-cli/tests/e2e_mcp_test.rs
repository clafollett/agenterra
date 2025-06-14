//! End-to-end integration test for MCP server and client generation and communication

use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command as AsyncCommand};
use tokio::time::timeout;

#[tokio::test]
async fn test_mcp_server_client_generation() -> Result<()> {
    // Create temporary directory for test outputs
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();
    
    // Get path to agenterra binary and project root
    let agenterra = env!("CARGO_BIN_EXE_agenterra");
    let current_dir = std::env::current_dir()?;
    let project_root = current_dir
        .parent()
        .unwrap() // Go up from crates/agenterra-cli
        .parent()
        .unwrap(); // Go up to project root
    
    println!("=== Testing MCP Server Generation ===");
    
    // Test 1: Generate MCP server
    let server_output = base_path.join("test_server");
    let server_result = Command::new(agenterra)
        .args(&[
            "scaffold",
            "mcp", 
            "server",
            "--project-name", "test_server",
            "--output-dir", server_output.to_str().unwrap(),
            "--schema-path", "tests/fixtures/openapi/petstore.openapi.v3.json",
            "--template", "rust_axum",
            "--base-url", "https://petstore3.swagger.io",
        ])
        .current_dir(project_root)
        .output()?;
    
    if !server_result.status.success() {
        eprintln!("Server generation failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&server_result.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&server_result.stderr));
        panic!("Server generation failed");
    }
    
    // Verify server files exist
    assert!(server_output.join("Cargo.toml").exists());
    assert!(server_output.join("src/main.rs").exists());
    assert!(server_output.join("src/handlers/mod.rs").exists());
    
    println!("✅ Server generation successful");
    
    println!("\n=== Testing MCP Client Generation ===");
    
    // Test 2: Generate MCP client
    let client_output = base_path.join("test_client");
    let client_result = Command::new(agenterra)
        .args(&[
            "scaffold",
            "mcp",
            "client", 
            "--project-name", "test_client",
            "--output-dir", client_output.to_str().unwrap(),
            "--template", "rust_reqwest",
        ])
        .current_dir(project_root)
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
    
    // Test 3: Build generated projects (always test compilation)
    println!("\n=== Building Generated Server ===");
    
    let server_build = Command::new("cargo")
        .args(&["build"])
        .current_dir(&server_output)
        .output()?;
        
    if !server_build.status.success() {
        eprintln!("Server build failed:");
        eprintln!("stderr: {}", String::from_utf8_lossy(&server_build.stderr));
        panic!("Server build failed");
    }
    
    println!("✅ Server builds successfully");
    
    println!("\n=== Building Generated Client ===");
    
    let client_build = Command::new("cargo")
        .args(&["build"])
        .current_dir(&client_output)
        .output()?;
        
    if !client_build.status.success() {
        eprintln!("Client build failed:");
        eprintln!("stderr: {}", String::from_utf8_lossy(&client_build.stderr));
        panic!("Client build failed");
    }
    
    println!("✅ Client builds successfully");
    
    // Test 4: End-to-end MCP communication
    println!("\n=== Testing MCP Server ↔ Client Communication ===");
    
    // Start the MCP server as a subprocess
    println!("Starting MCP server...");
    let mut server_process = AsyncCommand::new("cargo")
        .args(&["run"])
        .current_dir(&server_output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn server process")?;
    
    // Give server a moment to start up
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    // Create a simple test client that connects via stdio
    let test_result = timeout(Duration::from_secs(30), async {
        test_mcp_communication(&mut server_process).await
    }).await;
    
    // Clean up server process
    if let Err(e) = server_process.kill().await {
        eprintln!("Warning: Failed to kill server process: {}", e);
    }
    
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

/// Test basic MCP protocol communication with the server
async fn test_mcp_communication(server_process: &mut Child) -> Result<()> {
    let stdin = server_process.stdin.as_mut()
        .context("Failed to get server stdin")?;
    
    let stdout = server_process.stdout.as_mut()
        .context("Failed to get server stdout")?;
    
    let mut reader = BufReader::new(stdout);
    
    // Send initialize request (basic MCP handshake)
    let initialize_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "agenterra-test",
                "version": "0.1.0"
            }
        }
    });
    
    let request_line = format!("{}\n", initialize_request);
    stdin.write_all(request_line.as_bytes()).await
        .context("Failed to send initialize request")?;
    stdin.flush().await.context("Failed to flush stdin")?;
    
    // Read response
    let mut response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await
        .context("Timeout waiting for server response")?
        .context("Failed to read server response")?;
    
    // Parse and validate response
    let response: serde_json::Value = serde_json::from_str(&response_line)
        .context("Failed to parse JSON response")?;
    
    // Verify it's a valid JSON-RPC response
    if response.get("jsonrpc") != Some(&serde_json::Value::String("2.0".to_string())) {
        return Err(anyhow::anyhow!("Invalid JSON-RPC response: missing or invalid jsonrpc field"));
    }
    
    if response.get("id") != Some(&serde_json::Value::Number(serde_json::Number::from(1))) {
        return Err(anyhow::anyhow!("Invalid JSON-RPC response: missing or invalid id field"));
    }
    
    // Check if response has result (success) or error
    if let Some(error) = response.get("error") {
        return Err(anyhow::anyhow!("Server returned error: {}", error));
    }
    
    if response.get("result").is_none() {
        return Err(anyhow::anyhow!("Server response missing result field"));
    }
    
    println!("✅ MCP initialize handshake successful");
    
    // Send list_tools request to verify the server has tools
    let list_tools_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });
    
    let request_line = format!("{}\n", list_tools_request);
    stdin.write_all(request_line.as_bytes()).await
        .context("Failed to send list_tools request")?;
    stdin.flush().await.context("Failed to flush stdin")?;
    
    // Read tools response
    let mut tools_response_line = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut tools_response_line)).await
        .context("Timeout waiting for tools response")?
        .context("Failed to read tools response")?;
    
    let tools_response: serde_json::Value = serde_json::from_str(&tools_response_line)
        .context("Failed to parse tools JSON response")?;
    
    // Verify tools list response
    if let Some(result) = tools_response.get("result") {
        if let Some(tools) = result.get("tools") {
            if let Some(tools_array) = tools.as_array() {
                if !tools_array.is_empty() {
                    println!("✅ Server has {} tools available", tools_array.len());
                } else {
                    return Err(anyhow::anyhow!("Server returned empty tools list"));
                }
            } else {
                return Err(anyhow::anyhow!("Tools result is not an array"));
            }
        } else {
            return Err(anyhow::anyhow!("Tools response missing tools field"));
        }
    } else if let Some(error) = tools_response.get("error") {
        return Err(anyhow::anyhow!("Server returned error for tools/list: {}", error));
    } else {
        return Err(anyhow::anyhow!("Invalid tools response: missing result or error"));
    }
    
    Ok(())
}

#[test]
fn test_cli_help_output() {
    let agenterra = env!("CARGO_BIN_EXE_agenterra");
    
    // Test main help
    let result = Command::new(agenterra)
        .arg("--help")
        .output()
        .expect("Failed to run agenterra");
        
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("scaffold"));
    assert!(output.contains("Scaffold MCP servers and clients"));
    
    // Test scaffold mcp help
    let result = Command::new(agenterra)
        .args(&["scaffold", "mcp", "--help"])
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
    let result = Command::new(agenterra)
        .args(&["scaffold", "mcp", "server", "--help"])
        .output()
        .expect("Failed to run agenterra");
        
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("--schema-path"));
    assert!(output.contains("--template"));
    assert!(output.contains("--output-dir"));
    
    // Test client help shows correct options
    let result = Command::new(agenterra)
        .args(&["scaffold", "mcp", "client", "--help"])
        .output()
        .expect("Failed to run agenterra");
        
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("--template"));
    assert!(output.contains("--output-dir"));
    // Client should NOT have schema-path
    assert!(!output.contains("--schema-path"));
}