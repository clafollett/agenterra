//! Resource data structures for MCP client
//!
//! Provides data structures for MCP resources. Resource functionality is available
//! through the main client's `list_resources()` and `get_resource()` methods.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about an MCP resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Resource URI identifier
    pub uri: String,
    /// Human-readable name
    pub name: Option<String>,
    /// Resource description
    pub description: Option<String>,
    /// MIME type of the resource
    pub mime_type: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Content of a resource retrieved from MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    /// Resource metadata
    pub info: ResourceInfo,
    /// Raw content data
    pub data: Vec<u8>,
    /// Content encoding (if any)
    pub encoding: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_info_creation() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "size".to_string(),
            serde_json::Value::Number(serde_json::Number::from(1024)),
        );

        let resource = ResourceInfo {
            uri: "file:///test.txt".to_string(),
            name: Some("test.txt".to_string()),
            description: Some("A test file".to_string()),
            mime_type: Some("text/plain".to_string()),
            metadata,
        };

        assert_eq!(resource.uri, "file:///test.txt");
        assert_eq!(resource.name, Some("test.txt".to_string()));
        assert_eq!(resource.mime_type, Some("text/plain".to_string()));
        assert!(resource.metadata.contains_key("size"));
    }

    #[test]
    fn test_resource_content_creation() {
        let resource_info = ResourceInfo {
            uri: "file:///test.txt".to_string(),
            name: Some("test.txt".to_string()),
            description: Some("A test file".to_string()),
            mime_type: Some("text/plain".to_string()),
            metadata: HashMap::new(),
        };

        let content = ResourceContent {
            info: resource_info.clone(),
            data: b"Hello, World!".to_vec(),
            encoding: Some("utf-8".to_string()),
        };

        assert_eq!(content.info.uri, "file:///test.txt");
        assert_eq!(content.data, b"Hello, World!");
        assert_eq!(content.encoding, Some("utf-8".to_string()));
    }
}
