//! Data Transfer Objects for application layer

use crate::generation::Language;
use crate::protocols::Protocol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Request to generate a server implementation
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateServerRequest {
    pub protocol: Protocol,
    pub language: Language,
    pub project_name: String,
    pub schema_path: Option<String>,
    pub output_dir: PathBuf,
    pub options: HashMap<String, serde_json::Value>,
}

impl GenerateServerRequest {
    pub fn validate(&self) -> Result<(), crate::application::ValidationError> {
        if self.project_name.is_empty() {
            return Err(crate::application::ValidationError::EmptyProjectName);
        }

        // Validate protocol supports server role
        self.protocol
            .validate_role(&crate::protocols::Role::Server)
            .map_err(|_| crate::application::ValidationError::UnsupportedRole {
                protocol: self.protocol,
                role: crate::protocols::Role::Server,
            })?;

        // Check if OpenAPI is required but not provided
        let capabilities = self.protocol.capabilities();
        if capabilities.requires_openapi && self.schema_path.is_none() {
            return Err(crate::application::ValidationError::MissingField(
                "MCP server requires OpenAPI schema path".to_string(),
            ));
        }

        // Validate project name
        crate::generation::rules::validate_project_name(&self.project_name)
            .map_err(|e| crate::application::ValidationError::InvalidProjectName(e.to_string()))?;

        Ok(())
    }
}

/// Response from server generation
#[derive(Debug, Clone, Serialize)]
pub struct GenerateServerResponse {
    pub artifacts_count: usize,
    pub output_path: PathBuf,
    pub metadata: crate::generation::GenerationMetadata,
}

/// Request to generate a client implementation
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateClientRequest {
    pub protocol: Protocol,
    pub language: Language,
    pub project_name: String,
    pub output_dir: PathBuf,
    pub options: HashMap<String, serde_json::Value>,
}

impl GenerateClientRequest {
    pub fn validate(&self) -> Result<(), crate::application::ValidationError> {
        if self.project_name.is_empty() {
            return Err(crate::application::ValidationError::EmptyProjectName);
        }

        // Validate protocol supports client role
        self.protocol
            .validate_role(&crate::protocols::Role::Client)
            .map_err(|_| crate::application::ValidationError::UnsupportedRole {
                protocol: self.protocol,
                role: crate::protocols::Role::Client,
            })?;

        // Validate project name
        crate::generation::rules::validate_project_name(&self.project_name)
            .map_err(|e| crate::application::ValidationError::InvalidProjectName(e.to_string()))?;

        Ok(())
    }
}

/// Response from client generation
#[derive(Debug, Clone, Serialize)]
pub struct GenerateClientResponse {
    pub artifacts_count: usize,
    pub output_path: PathBuf,
    pub metadata: crate::generation::GenerationMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_server_request_validation() {
        let valid = GenerateServerRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "test-server".to_string(),
            schema_path: Some("/path/to/openapi.yaml".to_string()),
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        assert!(valid.validate().is_ok());

        // Test empty project name
        let mut invalid = valid.clone();
        invalid.project_name = "".to_string();
        assert!(matches!(
            invalid.validate().unwrap_err(),
            crate::application::ValidationError::EmptyProjectName
        ));

        // Test invalid project name
        let mut invalid = valid.clone();
        invalid.project_name = "invalid name!".to_string();
        assert!(matches!(
            invalid.validate().unwrap_err(),
            crate::application::ValidationError::InvalidProjectName(_)
        ));
    }

    #[test]
    fn test_generate_client_request_validation() {
        let valid = GenerateClientRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "test-client".to_string(),
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        assert!(valid.validate().is_ok());

        // Test empty project name
        let mut invalid = valid.clone();
        invalid.project_name = "".to_string();
        assert!(matches!(
            invalid.validate().unwrap_err(),
            crate::application::ValidationError::EmptyProjectName
        ));
    }
}
