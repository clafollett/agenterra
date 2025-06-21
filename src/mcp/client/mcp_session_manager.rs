//! MCP Session Manager - Multi-session architecture with clean single-session API
//!
//! Provides session lifecycle management for MCP clients with future support for
//! multiple named sessions (different LLMs, environments, etc.)

use crate::mcp::client::error::{ClientError, Result};
use crate::mcp::client::mcp_client::{ConnectionConfig, ConnectionState, McpClient};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// MCP Session Manager - handles session lifecycle with singleton pattern
///
/// Current API provides clean single-session interface, but internal architecture
/// is designed for future multi-session expansion (multiple LLMs, dev environments, etc.)
pub struct McpSessionManager;

impl McpSessionManager {
    /// Default session name for single-session API
    const DEFAULT_SESSION: &'static str = "default";

    /// Internal multi-session storage (future-ready architecture)
    ///
    /// Uses HashMap to support future named sessions:
    /// - "default" -> primary session (current API)
    /// - "openai-gpt4" -> future multi-session support
    /// - "anthropic-claude" -> future multi-session support
    /// - etc.
    fn sessions() -> &'static Lazy<Arc<Mutex<HashMap<String, Arc<Mutex<McpClient>>>>>> {
        static SESSIONS: Lazy<Arc<Mutex<HashMap<String, Arc<Mutex<McpClient>>>>>> =
            Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
        &SESSIONS
    }

    // ========================================
    // Public API - Clean Single-Session Interface
    // ========================================

    /// Connect to an MCP server using the default session
    ///
    /// This is the primary API for single-session use cases.
    /// Internally delegates to the session-aware implementation.
    pub async fn connect(config: ConnectionConfig) -> Result<()> {
        Self::connect_session(Self::DEFAULT_SESSION, config).await
    }

    /// List available tools from the connected MCP server
    ///
    /// Returns an error if no session is connected.
    pub async fn list_tools() -> Result<Vec<String>> {
        let client = Self::get_session_client(Self::DEFAULT_SESSION).await?;
        let mut client_guard = client.lock().await;
        client_guard.list_tools().await
    }

    /// Call a tool on the connected MCP server
    ///
    /// # Arguments
    /// * `name` - The name of the tool to call
    /// * `args` - JSON arguments for the tool
    ///
    /// Returns the tool result as JSON or an error if the tool call fails.
    pub async fn call_tool(name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let client = Self::get_session_client(Self::DEFAULT_SESSION).await?;
        let mut client_guard = client.lock().await;
        client_guard.call_tool_stateful(name, args).await
    }

    /// Disconnect the current session
    ///
    /// Removes the session from storage and cleans up resources.
    pub async fn disconnect() -> Result<()> {
        Self::disconnect_session(Self::DEFAULT_SESSION).await
    }

    /// Get the connection status of the current session
    ///
    /// Returns the current connection state or Disconnected if no session exists.
    pub async fn status() -> Result<ConnectionState> {
        match Self::try_get_session_client(Self::DEFAULT_SESSION).await {
            Some(client) => {
                let client_guard = client.lock().await;
                Ok(client_guard.connection_state().clone())
            }
            None => Ok(ConnectionState::Disconnected),
        }
    }

    // ========================================
    // Internal Session-Aware Methods (Future Building Blocks)
    // ========================================

    /// Connect to an MCP server with a specific session name
    ///
    /// This is the core implementation that supports future multi-session functionality.
    /// Currently used internally, but designed to be exposed in future versions.
    async fn connect_session(session_name: &str, config: ConnectionConfig) -> Result<()> {
        log::info!("Connecting MCP session '{}' to server", session_name);

        // Create new stateful client
        let mut client = McpClient::new_stateful();

        // Connect using domain entity
        client.connect(config).await?;

        // Wrap in Arc<Mutex> and store in session map
        let client_arc = Arc::new(Mutex::new(client));
        let sessions = Self::sessions();
        let mut sessions_guard = sessions.lock().await;
        sessions_guard.insert(session_name.to_string(), client_arc);

        log::info!("Successfully established MCP session '{}'", session_name);
        Ok(())
    }

    /// Get a session client by name, returning an error if not found
    ///
    /// This enforces the business rule that operations require an active connection.
    async fn get_session_client(session_name: &str) -> Result<Arc<Mutex<McpClient>>> {
        Self::try_get_session_client(session_name)
            .await
            .ok_or_else(|| {
                ClientError::Client(format!(
                    "No active MCP session '{}'. Run 'connect' command first.",
                    session_name
                ))
            })
    }

    /// Try to get a session client by name (internal helper)
    ///
    /// Returns None if session doesn't exist, used for status checking.
    async fn try_get_session_client(session_name: &str) -> Option<Arc<Mutex<McpClient>>> {
        let sessions = Self::sessions();
        let sessions_guard = sessions.lock().await;
        sessions_guard.get(session_name).cloned()
    }

    /// Disconnect a specific session
    ///
    /// Future API for managing multiple sessions independently.
    async fn disconnect_session(session_name: &str) -> Result<()> {
        let sessions = Self::sessions();
        let mut sessions_guard = sessions.lock().await;

        match sessions_guard.remove(session_name) {
            Some(_client) => {
                log::info!("Disconnected MCP session '{}'", session_name);
                Ok(())
            }
            None => Err(ClientError::Client(format!(
                "No active session '{}' to disconnect",
                session_name
            ))),
        }
    }

    // ========================================
    // Future API Extensions (Multi-Session Support)
    // ========================================

    // TODO: Future public methods for multi-session support:
    // pub async fn connect_session(name: &str, config: ConnectionConfig) -> Result<()>
    // pub async fn list_sessions() -> Vec<String>
    // pub async fn set_active_session(name: &str) -> Result<()>
    // pub async fn get_active_session() -> Option<String>
    // pub async fn list_tools_for_session(session_name: &str) -> Result<Vec<String>>
    // pub async fn call_tool_for_session(session_name: &str, tool_name: &str, args: serde_json::Value) -> Result<serde_json::Value>
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_session_manager_lifecycle() {
        // Test basic connection lifecycle
        let config = ConnectionConfig::builder()
            .command("echo")
            .args(vec!["mock".to_string()])
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        // Initially disconnected
        let status = McpSessionManager::status().await.unwrap();
        assert_eq!(status, ConnectionState::Disconnected);

        // Connect should work
        let result = McpSessionManager::connect(config).await;
        assert!(result.is_ok());

        // Should now be connected
        let status = McpSessionManager::status().await.unwrap();
        assert_eq!(status, ConnectionState::Connected);

        // Disconnect should work
        let result = McpSessionManager::disconnect().await;
        assert!(result.is_ok());

        // Should be disconnected again
        let status = McpSessionManager::status().await.unwrap();
        assert_eq!(status, ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_operations_require_connection() {
        // Ensure we start disconnected
        let _ = McpSessionManager::disconnect().await;

        // Operations should fail when not connected
        let result = McpSessionManager::list_tools().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No active MCP session")
        );

        let result = McpSessionManager::call_tool("test_tool", serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No active MCP session")
        );
    }
}
