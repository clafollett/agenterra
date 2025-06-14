//! Tool registry for managing MCP tool metadata and validation

use crate::error::{ClientError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about an MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name (snake_case format)
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// Input schema for parameters (JSON Schema)
    pub input_schema: Option<serde_json::Value>,
}

/// Registry for managing tool metadata and validation
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    /// Map of tool name to tool information
    tools: HashMap<String, ToolInfo>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Add a tool to the registry
    pub fn add_tool(&mut self, tool: ToolInfo) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Get tool information by name
    pub fn get_tool(&self, name: &str) -> Option<&ToolInfo> {
        self.tools.get(name)
    }

    /// Check if a tool exists in the registry
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Validate tool parameters against the schema (basic validation for now)
    pub fn validate_parameters(&self, tool_name: &str, params: &serde_json::Value) -> Result<()> {
        if !self.has_tool(tool_name) {
            return Err(ClientError::Client(format!(
                "Unknown tool: '{}'. Available tools: {:?}",
                tool_name,
                self.tool_names()
            )));
        }

        // For now, just check that parameters is an object if the tool expects parameters
        if let Some(tool_info) = self.get_tool(tool_name) {
            if tool_info.input_schema.is_some() && !params.is_object() && !params.is_null() {
                return Err(ClientError::Client(format!(
                    "Tool '{}' expects object parameters, got: {}",
                    tool_name, params
                )));
            }
        }

        Ok(())
    }

    /// Update the registry with tools from list_tools response
    pub fn update_from_rmcp_tools(&mut self, rmcp_tools: Vec<rmcp::model::Tool>) {
        self.tools.clear();

        for rmcp_tool in rmcp_tools {
            let tool_info = ToolInfo {
                name: rmcp_tool.name.to_string(),
                description: rmcp_tool.description.map(|d| d.to_string()),
                input_schema: Some(serde_json::Value::Object((*rmcp_tool.input_schema).clone())),
            };
            self.add_tool(tool_info);
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_registry_basic() {
        let mut registry = ToolRegistry::new();

        let tool = ToolInfo {
            name: "get_pet_by_id".to_string(),
            description: Some("Get a pet by ID".to_string()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "id": {"type": "integer"}
                },
                "required": ["id"]
            })),
        };

        registry.add_tool(tool);

        assert!(registry.has_tool("get_pet_by_id"));
        assert!(!registry.has_tool("nonexistent_tool"));
        assert_eq!(registry.tool_names(), vec!["get_pet_by_id"]);
    }

    #[test]
    fn test_parameter_validation() {
        let mut registry = ToolRegistry::new();

        let tool = ToolInfo {
            name: "get_pet_by_id".to_string(),
            description: Some("Get a pet by ID".to_string()),
            input_schema: Some(json!({"type": "object"})),
        };

        registry.add_tool(tool);

        // Valid object parameters
        assert!(
            registry
                .validate_parameters("get_pet_by_id", &json!({"id": 123}))
                .is_ok()
        );

        // Valid null parameters
        assert!(
            registry
                .validate_parameters("get_pet_by_id", &json!(null))
                .is_ok()
        );

        // Invalid array parameters when object expected
        assert!(
            registry
                .validate_parameters("get_pet_by_id", &json!([1, 2, 3]))
                .is_err()
        );

        // Unknown tool
        assert!(
            registry
                .validate_parameters("unknown_tool", &json!({}))
                .is_err()
        );
    }
}
