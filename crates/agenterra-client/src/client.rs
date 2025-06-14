//! Main client implementation for Agenterra MCP Client

use crate::error::{ClientError, Result};
use crate::transport::Transport;
use std::time::Duration;

// Import rmcp types for real MCP protocol integration
use rmcp::{
    RoleClient,
    service::{RunningService, ServiceExt},
    transport::TokioChildProcess,
};

/// High-level MCP client with ergonomic APIs
pub struct AgenterraClient {
    // We'll store the rmcp service for actual MCP communication
    service: Option<RunningService<RoleClient, ()>>,
    timeout: Duration,
}

impl AgenterraClient {
    /// Create a new client - for now still accepting Transport but will transition to rmcp
    pub fn new(_transport: Box<dyn Transport>) -> Self {
        Self {
            service: None,                        // Will be connected later via connect()
            timeout: Duration::from_millis(5000), // 5 second default timeout
        }
    }

    /// Set the timeout duration for operations
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Connect to an MCP server using child process transport
    /// This is a temporary simplified API - we'll make it more flexible later
    pub async fn connect_to_child_process(
        &mut self,
        command: tokio::process::Command,
    ) -> Result<()> {
        let transport = TokioChildProcess::new(command).map_err(|e| {
            ClientError::Transport(format!("Failed to create child process: {}", e))
        })?;

        let service = ().serve(transport).await.map_err(|e| {
            ClientError::Protocol(format!("Failed to connect to MCP server: {}", e))
        })?;

        self.service = Some(service);
        Ok(())
    }

    /// Ping the MCP server to test connectivity
    pub async fn ping(&mut self) -> Result<()> {
        match &self.service {
            Some(service) => {
                // rmcp doesn't have a direct ping - let's use peer_info as connectivity test
                let _info = service.peer_info();
                Ok(())
            }
            None => Err(ClientError::Client(
                "Not connected to MCP server. Call connect_to_child_process() first.".to_string(),
            )),
        }
    }

    /// List available tools from the server
    pub async fn list_tools(&mut self) -> Result<Vec<String>> {
        match &self.service {
            Some(service) => {
                let tools_response = service
                    .list_tools(Default::default())
                    .await
                    .map_err(|e| ClientError::Protocol(format!("Failed to list tools: {}", e)))?;

                let tool_names = tools_response
                    .tools
                    .into_iter()
                    .map(|tool| tool.name.to_string()) // Convert Cow<str> to String
                    .collect();

                Ok(tool_names)
            }
            None => Err(ClientError::Client(
                "Not connected to MCP server. Call connect_to_child_process() first.".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use serde_json::json;

    // Integration test with a real MCP server process
    #[tokio::test]
    #[ignore] // Ignore by default since it requires an MCP server binary
    async fn test_connect_to_mcp_server() {
        use tokio::process::Command;

        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Try to connect to a mock MCP server (this will fail but shows the API)
        let mut command = Command::new("echo");
        command.arg("Mock MCP server that doesn't exist");

        let result = client.connect_to_child_process(command).await;

        // We expect this to fail since echo is not an MCP server
        assert!(result.is_err());
        if let Err(ClientError::Protocol(msg)) = result {
            assert!(msg.contains("Failed to connect to MCP server"));
        } else if let Err(ClientError::Transport(_)) = result {
            // Also acceptable - transport layer might reject the connection
        } else {
            panic!("Expected connection to fail with Protocol or Transport error");
        }
    }

    #[tokio::test]
    async fn test_client_creation() {
        let mock_transport = MockTransport::new(vec![]);
        let client = AgenterraClient::new(Box::new(mock_transport));

        // Should be able to create client successfully
        assert_eq!(client.timeout, Duration::from_millis(5000));
    }

    #[tokio::test]
    async fn test_client_with_custom_timeout() {
        let mock_transport = MockTransport::new(vec![]);
        let timeout = Duration::from_millis(1000);
        let client = AgenterraClient::new(Box::new(mock_transport)).with_timeout(timeout);

        assert_eq!(client.timeout, timeout);
    }

    #[tokio::test]
    async fn test_ping_not_connected() {
        let mock_response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {}
        });

        let mock_transport = MockTransport::new(vec![mock_response]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Without connecting to a server, ping should fail
        let result = client.ping().await;

        // Should fail with "not connected" error
        assert!(result.is_err());
        if let Err(ClientError::Client(msg)) = result {
            assert!(msg.contains("Not connected to MCP server"));
        } else {
            panic!("Expected ClientError::Client");
        }
    }

    #[tokio::test]
    async fn test_list_tools_not_connected() {
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Without connecting to a server, list_tools should fail
        let result = client.list_tools().await;

        // Should fail with "not connected" error
        assert!(result.is_err());
        if let Err(ClientError::Client(msg)) = result {
            assert!(msg.contains("Not connected to MCP server"));
        } else {
            panic!("Expected ClientError::Client");
        }
    }
}
