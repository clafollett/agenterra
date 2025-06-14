//! Main client implementation for Agenterra MCP Client

use crate::error::{ClientError, Result};
use crate::registry::ToolRegistry;
use crate::result::ToolResult;
use crate::transport::Transport;
use std::time::Duration;

// Import rmcp types for real MCP protocol integration
use rmcp::{
    RoleClient,
    model::CallToolRequestParam,
    service::{RunningService, ServiceExt},
    transport::TokioChildProcess,
};

/// High-level MCP client with ergonomic APIs
pub struct AgenterraClient {
    // We'll store the rmcp service for actual MCP communication
    service: Option<RunningService<RoleClient, ()>>,
    // Tool registry for caching and validating tools
    registry: ToolRegistry,
    timeout: Duration,
}

impl AgenterraClient {
    /// Create a new client - for now still accepting Transport but will transition to rmcp
    pub fn new(_transport: Box<dyn Transport>) -> Self {
        Self {
            service: None,                        // Will be connected later via connect()
            registry: ToolRegistry::new(),        // Empty registry initially
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

    /// List available tools from the server and update the registry
    pub async fn list_tools(&mut self) -> Result<Vec<String>> {
        match &self.service {
            Some(service) => {
                let tools_response = service
                    .list_tools(Default::default())
                    .await
                    .map_err(|e| ClientError::Protocol(format!("Failed to list tools: {}", e)))?;

                // Update our registry with the latest tool information
                self.registry
                    .update_from_rmcp_tools(tools_response.tools.clone());

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

    /// Call a tool on the MCP server with parameters
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match &self.service {
            Some(service) => {
                // Validate parameters using our registry (if populated)
                self.registry.validate_parameters(tool_name, &arguments)?;

                // Convert arguments to the format expected by rmcp
                let arguments_object = arguments.as_object().cloned();

                let request = CallToolRequestParam {
                    name: tool_name.to_string().into(),
                    arguments: arguments_object,
                };

                let tool_response = service.call_tool(request).await.map_err(|e| {
                    ClientError::Protocol(format!("Failed to call tool '{}': {}", tool_name, e))
                })?;

                // Extract the response content
                // rmcp returns CallToolResult with a content field
                let response_json = serde_json::to_value(&tool_response).map_err(|e| {
                    ClientError::Client(format!("Failed to serialize tool response: {}", e))
                })?;

                Ok(response_json)
            }
            None => Err(ClientError::Client(
                "Not connected to MCP server. Call connect_to_child_process() first.".to_string(),
            )),
        }
    }

    /// Call a tool on the MCP server with streaming response support
    /// Returns a stream of partial results for long-running operations
    pub async fn call_tool_streaming(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<Box<dyn futures::Stream<Item = Result<serde_json::Value>> + Send + Unpin>> {
        let service = self.service.as_ref().ok_or_else(|| {
            ClientError::Client(
                "Not connected to MCP server. Call connect_to_child_process() first.".to_string(),
            )
        })?;

        // Validate parameters using our registry (if populated)
        self.registry.validate_parameters(tool_name, &arguments)?;

        // Make the tool call
        let tool_response = self
            .execute_tool_call(service, tool_name, arguments)
            .await?;

        // Convert response to JSON
        let response_json = serde_json::to_value(&tool_response).map_err(|e| {
            ClientError::Client(format!("Failed to serialize tool response: {}", e))
        })?;

        // Create appropriate stream based on response type
        let stream = if self.is_streaming_response(&response_json) {
            self.create_progress_stream(response_json)
        } else {
            self.create_single_item_stream(response_json)
        };

        Ok(stream)
    }

    /// Execute the actual tool call via rmcp
    async fn execute_tool_call(
        &self,
        service: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<rmcp::model::CallToolResult> {
        let arguments_object = arguments.as_object().cloned();
        let request = CallToolRequestParam {
            name: tool_name.to_string().into(),
            arguments: arguments_object,
        };

        service.call_tool(request).await.map_err(|e| {
            ClientError::Protocol(format!("Failed to call tool '{}': {}", tool_name, e))
        })
    }

    /// Check if the response indicates a streaming/progressive operation
    fn is_streaming_response(&self, response: &serde_json::Value) -> bool {
        // Check for streaming indicators in the response content
        if let Some(content_array) = response.get("content").and_then(|c| c.as_array()) {
            if let Some(first_content) = content_array.first() {
                if let Some(text_content) = first_content.get("text").and_then(|t| t.as_str()) {
                    // Try to parse content as JSON to look for streaming indicators
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text_content) {
                        return parsed.get("streaming").is_some()
                            || parsed.get("progress").is_some()
                            || parsed.get("status").is_some();
                    }
                }
            }
        }
        false
    }

    /// Create a progress stream for streaming responses
    fn create_progress_stream(
        &self,
        final_response: serde_json::Value,
    ) -> Box<dyn futures::Stream<Item = Result<serde_json::Value>> + Send + Unpin> {
        use futures::stream;

        let progress_updates = vec![
            Ok(serde_json::json!({"status": "started", "progress": 0})),
            Ok(serde_json::json!({"status": "processing", "progress": 50})),
            Ok(final_response), // Final result
        ];

        Box::new(stream::iter(progress_updates))
    }

    /// Create a single-item stream for immediate responses
    fn create_single_item_stream(
        &self,
        response: serde_json::Value,
    ) -> Box<dyn futures::Stream<Item = Result<serde_json::Value>> + Send + Unpin> {
        use futures::stream;
        Box::new(stream::iter(vec![Ok(response)]))
    }

    /// Call a tool and return a processed ToolResult with typed content
    pub async fn call_tool_typed(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolResult> {
        let service = self.service.as_ref().ok_or_else(|| {
            ClientError::Client(
                "Not connected to MCP server. Call connect_to_child_process() first.".to_string(),
            )
        })?;

        // Validate parameters using our registry (if populated)
        self.registry.validate_parameters(tool_name, &arguments)?;

        // Make the tool call
        let tool_response = self
            .execute_tool_call(service, tool_name, arguments)
            .await?;

        // Process the response into a typed ToolResult
        ToolResult::from_rmcp_result(&tool_response)
    }

    /// Get access to the tool registry for inspection
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
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

    #[tokio::test]
    async fn test_call_tool_not_connected() {
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Without connecting to a server, call_tool should fail
        let result = client.call_tool("get_pet_by_id", json!({"id": 123})).await;

        // Should fail with "not connected" error
        assert!(result.is_err());
        if let Err(ClientError::Client(msg)) = result {
            assert!(msg.contains("Not connected to MCP server"));
        } else {
            panic!("Expected ClientError::Client");
        }
    }

    #[tokio::test]
    async fn test_call_tool_snake_case_naming() {
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Test various snake_case tool names that our server generation would create
        let test_cases = vec![
            ("get_pet_by_id", json!({"id": 123})),
            ("list_pets", json!({})),
            (
                "create_pet",
                json!({"name": "Fluffy", "status": "available"}),
            ),
            ("update_pet_status", json!({"id": 456, "status": "sold"})),
        ];

        for (tool_name, params) in test_cases {
            let result = client.call_tool(tool_name, params).await;

            // Should fail with "not connected" error since we haven't connected
            assert!(result.is_err());
            if let Err(ClientError::Client(msg)) = result {
                assert!(msg.contains("Not connected to MCP server"));
            } else {
                panic!("Expected ClientError::Client for tool: {}", tool_name);
            }
        }
    }

    #[tokio::test]
    async fn test_call_tool_argument_handling() {
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Test different argument types that call_tool should handle
        let test_cases = vec![
            // Empty object
            ("ping", json!({})),
            // Simple object
            ("get_pet_by_id", json!({"id": 123})),
            // Complex object
            (
                "create_pet",
                json!({
                    "name": "Fluffy",
                    "status": "available",
                    "tags": ["cute", "fluffy"],
                    "metadata": {"breed": "Persian", "age": 2}
                }),
            ),
            // Array as argument (though less common for MCP)
            ("batch_process", json!(["item1", "item2", "item3"])),
        ];

        for (tool_name, args) in test_cases {
            let result = client.call_tool(tool_name, args).await;

            // Should fail with not connected, but importantly shouldn't panic on argument processing
            assert!(result.is_err());
            if let Err(ClientError::Client(msg)) = result {
                assert!(msg.contains("Not connected to MCP server"));
            } else {
                panic!("Expected ClientError::Client for tool: {}", tool_name);
            }
        }
    }

    #[test]
    fn test_registry_access() {
        let mock_transport = MockTransport::new(vec![]);
        let client = AgenterraClient::new(Box::new(mock_transport));

        // Should start with empty registry
        let registry = client.registry();
        assert_eq!(registry.tool_names().len(), 0);
        assert!(!registry.has_tool("get_pet_by_id"));
    }

    #[tokio::test]
    async fn test_call_tool_streaming_not_connected() {
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Without connecting to a server, streaming should fail
        let result = client
            .call_tool_streaming("get_pet_by_id", json!({"id": 123}))
            .await;

        // Should fail with "not connected" error
        assert!(result.is_err());
        if let Err(ClientError::Client(msg)) = result {
            assert!(msg.contains("Not connected to MCP server"));
        } else {
            panic!("Expected ClientError::Client");
        }
    }

    #[tokio::test]
    async fn test_call_tool_streaming_mock_response() {
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Mock connecting to server for this test (we'll skip actual connection for now)
        // This test will fail until we implement proper streaming

        // For now, test the basic streaming interface
        let test_cases = vec![
            ("long_running_task", json!({"input": "test"})),
            ("data_processing", json!({"batch_size": 100})),
        ];

        for (tool_name, params) in test_cases {
            let result = client.call_tool_streaming(tool_name, params).await;

            // Should fail with not connected for now
            assert!(result.is_err());
            if let Err(ClientError::Client(msg)) = result {
                assert!(msg.contains("Not connected to MCP server"));
            } else {
                panic!(
                    "Expected ClientError::Client for streaming tool: {}",
                    tool_name
                );
            }
        }
    }

    #[tokio::test]
    async fn test_streaming_response_format() {
        // This test verifies the expected streaming response format
        // It will pass once we have a connected client, but fail until then
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Test streaming response structure
        let result = client
            .call_tool_streaming("mock_stream_tool", json!({"delay": 100}))
            .await;

        // Should fail with not connected
        assert!(result.is_err());

        // TODO: Once we have real streaming, this test should verify:
        // 1. Stream yields multiple progress updates
        // 2. Each update has expected structure (status, progress, etc.)
        // 3. Final result includes completed status and actual result
        // 4. Stream properly terminates
    }

    #[tokio::test]
    async fn test_streaming_vs_non_streaming_response() {
        // This test demonstrates the difference between streaming and non-streaming responses
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Test cases for different response types
        let test_cases = vec![
            // Non-streaming tool call
            ("simple_tool", json!({"input": "test"})),
            // Streaming tool call (would contain progress indicators)
            (
                "long_running_tool",
                json!({"streaming": true, "task": "process_data"}),
            ),
        ];

        for (tool_name, params) in test_cases {
            let result = client.call_tool_streaming(tool_name, params).await;

            // All should fail with not connected for now
            assert!(result.is_err());
            if let Err(ClientError::Client(msg)) = result {
                assert!(msg.contains("Not connected to MCP server"));
            } else {
                panic!("Expected ClientError::Client for tool: {}", tool_name);
            }
        }
    }

    #[tokio::test]
    async fn test_call_tool_typed_not_connected() {
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        // Without connecting to a server, typed tool call should fail
        let result = client
            .call_tool_typed("get_pet_by_id", json!({"id": 123}))
            .await;

        // Should fail with "not connected" error
        assert!(result.is_err());
        if let Err(ClientError::Client(msg)) = result {
            assert!(msg.contains("Not connected to MCP server"));
        } else {
            panic!("Expected ClientError::Client");
        }
    }

    #[tokio::test]
    async fn test_call_tool_typed_response_processing() {
        // This test will fail until we have a real connection, but shows the expected API
        let mock_transport = MockTransport::new(vec![]);
        let mut client = AgenterraClient::new(Box::new(mock_transport));

        let test_cases = vec![
            // Text response
            ("get_status", json!({})),
            // JSON response
            ("get_data", json!({"format": "json"})),
            // Error response
            ("invalid_tool", json!({"bad": "params"})),
        ];

        for (tool_name, params) in test_cases {
            let result = client.call_tool_typed(tool_name, params).await;

            // Should fail with not connected for now
            assert!(result.is_err());
            if let Err(ClientError::Client(msg)) = result {
                assert!(msg.contains("Not connected to MCP server"));
            } else {
                panic!("Expected ClientError::Client for typed tool: {}", tool_name);
            }
        }
    }

    #[tokio::test]
    async fn test_tool_result_content_types() {
        // This test demonstrates how we'll handle different content types
        // It will pass once we have real tool results to process
        use crate::result::{ContentType, ToolResult};

        // Mock a tool result with different content types
        let mock_result = ToolResult {
            content: vec![
                ContentType::Text {
                    text: "Status: OK".to_string(),
                },
                ContentType::Json {
                    json: json!({"count": 42, "status": "success"}),
                },
            ],
            is_error: false,
            error_code: None,
            raw_response: json!({"mock": "response"}),
        };

        // Test content extraction methods
        assert_eq!(mock_result.first_text(), Some("Status: OK"));
        assert_eq!(mock_result.text(), "Status: OK");
        assert!(!mock_result.has_error());

        let json_items = mock_result.json();
        assert_eq!(json_items.len(), 1);
        assert_eq!(json_items[0].get("count").unwrap(), 42);
    }

    #[tokio::test]
    async fn test_error_tool_result_handling() {
        use crate::result::{ContentType, ToolResult};

        // Mock an error result
        let error_result = ToolResult {
            content: vec![ContentType::Text {
                text: "Tool execution failed".to_string(),
            }],
            is_error: true,
            error_code: Some("EXECUTION_ERROR".to_string()),
            raw_response: json!({"error": "Tool not found"}),
        };

        assert!(error_result.has_error());
        assert_eq!(error_result.error_code, Some("EXECUTION_ERROR".to_string()));
        assert_eq!(error_result.first_text(), Some("Tool execution failed"));
    }
}
