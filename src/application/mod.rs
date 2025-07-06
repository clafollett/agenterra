//! Application layer - orchestrates use cases and coordinates between domains

pub mod dto;
pub mod errors;
pub mod generate_client;
pub mod generate_server;
pub mod template_management;
pub mod traits;

pub use dto::*;
pub use errors::*;
pub use template_management::*;
pub use traits::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::Language;
    use crate::protocols::Protocol;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_generate_server_request_validation() {
        // Valid request
        let request = GenerateServerRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "test-server".to_string(),
            schema_path: Some("/path/to/openapi.yaml".to_string()),
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        assert!(request.validate().is_ok());

        // Empty project name
        let mut invalid_request = request.clone();
        invalid_request.project_name = "".to_string();
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_generate_client_request_validation() {
        // Valid request
        let request = GenerateClientRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "test-client".to_string(),
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        assert!(request.validate().is_ok());

        // Invalid protocol for client
        let mut invalid_request = request.clone();
        invalid_request.protocol = Protocol::Anp; // Assuming ANP doesn't support client
        assert!(invalid_request.validate().is_err());
    }

    #[tokio::test]
    async fn test_generate_server_use_case_success() {
        // We'll test this with infrastructure implementations
        // For now, just ensure the types compile correctly
        let request = GenerateServerRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "test-server".to_string(),
            schema_path: Some("/path/to/openapi.yaml".to_string()),
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        // Ensure request validates correctly
        assert!(request.validate().is_ok());

        // Test ACP server (doesn't require OpenAPI)
        let acp_request = GenerateServerRequest {
            protocol: Protocol::Acp,
            language: Language::Rust,
            project_name: "test-acp-server".to_string(),
            schema_path: None,
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        // Should validate successfully without schema_path
        assert!(acp_request.validate().is_ok());

        // Test MCP server without OpenAPI should fail validation
        let invalid_mcp = GenerateServerRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "test-mcp-server".to_string(),
            schema_path: None,
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        let result = invalid_mcp.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::MissingField(msg) => {
                assert!(msg.contains("MCP server requires OpenAPI"));
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_application_error_types() {
        // Test that error types are correctly defined
        let error = ApplicationError::ProtocolNotImplemented(Protocol::Mcp);
        match error {
            ApplicationError::ProtocolNotImplemented(p) => assert_eq!(p, Protocol::Mcp),
            _ => panic!("Expected ProtocolNotImplemented"),
        }

        let validation_error = ValidationError::EmptyProjectName;
        match validation_error {
            ValidationError::EmptyProjectName => {}
            _ => panic!("Expected EmptyProjectName"),
        }
    }

    #[test]
    fn test_generate_client_use_case_types() {
        // We'll test this with infrastructure implementations
        // For now, just ensure the types compile correctly
        let request = GenerateClientRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "test-client".to_string(),
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        // Ensure request validates correctly
        assert!(request.validate().is_ok());
    }
}
