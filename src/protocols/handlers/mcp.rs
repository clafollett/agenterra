//! MCP (Model Context Protocol) handler implementation

use async_trait::async_trait;
use serde_json::json;

use crate::protocols::{
    Protocol, ProtocolConfig, ProtocolError, ProtocolHandler, ProtocolInput, Role,
};

/// Handler for the Model Context Protocol (MCP)
#[derive(Debug, Clone)]
pub struct McpProtocolHandler;

impl McpProtocolHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProtocolHandler for McpProtocolHandler {
    fn protocol(&self) -> Protocol {
        Protocol::Mcp
    }

    async fn prepare_context(
        &self,
        input: ProtocolInput,
    ) -> Result<crate::generation::GenerationContext, ProtocolError> {
        // Validate role is supported
        self.protocol().validate_role(&input.role)?;

        // Validate configuration
        self.validate_configuration(&input.config)?;

        // If Server role, OpenAPI is required
        if input.role == Role::Server && input.openapi_spec.is_none() {
            return Err(ProtocolError::InvalidConfiguration(
                "MCP Server role requires OpenAPI specification".to_string(),
            ));
        }

        // Validate language support for this protocol/role combination
        crate::generation::rules::validate_language_support(
            Protocol::Mcp,
            &input.role,
            input.language,
        )
        .map_err(|e| match e {
            crate::generation::GenerationError::UnsupportedLanguageForProtocol {
                language,
                protocol,
            } => ProtocolError::InvalidConfiguration(format!(
                "Language {:?} is not supported for {:?}/{:?}",
                language, protocol, input.role
            )),
            _ => ProtocolError::InternalError(e.to_string()),
        })?;

        // Create generation context
        let mut context = crate::generation::GenerationContext::new(
            Protocol::Mcp,
            input.role.clone(),
            input.language,
        );

        // Set metadata
        context.metadata.project_name = input.config.project_name.clone();
        context.metadata.version = input.config.version.unwrap_or_else(|| "0.1.0".to_string());

        // Add variables
        context.add_variable("project_name".to_string(), json!(input.config.project_name));
        context.add_variable("version".to_string(), json!(context.metadata.version));

        // Add role-specific variables
        match input.role {
            Role::Server => {
                context.add_variable("requires_openapi".to_string(), json!(true));
                context.add_variable("transport".to_string(), json!("stdio"));
                context.add_variable(
                    "features".to_string(),
                    json!({
                        "tools": true,
                        "resources": true,
                        "prompts": true,
                        "sampling": false,
                    }),
                );
            }
            Role::Client => {
                context.add_variable("requires_openapi".to_string(), json!(false));
                context.add_variable("transport".to_string(), json!("stdio"));
                context.add_variable("connection_type".to_string(), json!("direct"));
            }
            _ => {
                return Err(ProtocolError::UnsupportedRole {
                    protocol: self.protocol(),
                    role: input.role.clone(),
                });
            }
        }

        // Add OpenAPI spec if present (for MCP Server)
        if let Some(spec) = input.openapi_spec {
            // Add OpenAPI metadata to context variables
            context.add_variable("api_title".to_string(), json!(spec.info.title));
            context.add_variable("api_version".to_string(), json!(spec.info.version));
            if let Some(description) = &spec.info.description {
                context.add_variable("api_description".to_string(), json!(description));
            }

            // Add server information if available
            if !spec.servers.is_empty() {
                context.add_variable("base_api_url".to_string(), json!(spec.servers[0].url));
            }

            // Extract operations that become MCP endpoints
            let endpoints = spec.operations.clone();
            tracing::debug!(
                "MCP handler extracted {} operations from OpenAPI spec to create MCP endpoints",
                endpoints.len()
            );

            // Store in protocol-specific context
            context.protocol_context = Some(crate::generation::ProtocolContext::McpServer {
                openapi_spec: spec,
                endpoints,
            });
        }

        // Add custom options as variables
        for (key, value) in &input.config.options {
            context.add_variable(key.clone(), value.clone());
        }

        Ok(context)
    }

    fn validate_configuration(&self, config: &ProtocolConfig) -> Result<(), ProtocolError> {
        // Validate required fields
        if config.project_name.is_empty() {
            return Err(ProtocolError::InvalidConfiguration(
                "Project name is required".to_string(),
            ));
        }

        // Validate project name format (alphanumeric with dashes/underscores)
        if !config
            .project_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ProtocolError::InvalidConfiguration(
                "Project name must be alphanumeric with optional dashes or underscores".to_string(),
            ));
        }

        // Validate MCP-specific options if present
        if let Some(transport) = config.options.get("transport") {
            if let Some(transport_str) = transport.as_str() {
                match transport_str {
                    "stdio" | "http" | "websocket" => {}
                    _ => {
                        return Err(ProtocolError::InvalidConfiguration(format!(
                            "Invalid transport type: {transport_str}. Must be one of: stdio, http, websocket"
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_handler_protocol() {
        let handler = McpProtocolHandler::new();
        assert_eq!(handler.protocol(), Protocol::Mcp);
    }

    #[tokio::test]
    async fn test_mcp_server_requires_openapi() {
        let handler = McpProtocolHandler::new();
        let input = ProtocolInput {
            openapi_spec: None,
            config: ProtocolConfig {
                project_name: "test-project".to_string(),
                version: None,
                options: std::collections::HashMap::new(),
            },
            role: Role::Server,
            language: crate::generation::Language::Rust,
        };

        let result = handler.prepare_context(input).await;
        assert!(result.is_err());

        if let Err(ProtocolError::InvalidConfiguration(msg)) = result {
            assert!(msg.contains("requires OpenAPI"));
        } else {
            panic!("Expected InvalidConfiguration error");
        }
    }

    #[tokio::test]
    async fn test_mcp_client_no_openapi_required() {
        let handler = McpProtocolHandler::new();
        let input = ProtocolInput {
            openapi_spec: None,
            config: ProtocolConfig {
                project_name: "test-client".to_string(),
                version: Some("1.0.0".to_string()),
                options: std::collections::HashMap::new(),
            },
            role: Role::Client,
            language: crate::generation::Language::Rust,
        };

        let result = handler.prepare_context(input).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.protocol, Protocol::Mcp);
        assert_eq!(context.role, Role::Client);
        assert_eq!(context.metadata.project_name, "test-client");
        assert_eq!(context.metadata.version, "1.0.0");
        assert_eq!(context.variables["requires_openapi"], false);
    }

    #[tokio::test]
    async fn test_mcp_server_with_openapi() {
        let handler = McpProtocolHandler::new();
        let mut options = std::collections::HashMap::new();
        options.insert("transport".to_string(), json!("http"));
        options.insert("port".to_string(), json!(8080));

        let input = ProtocolInput {
            openapi_spec: Some(crate::generation::OpenApiContext {
                version: "3.0.0".to_string(),
                info: crate::generation::ApiInfo {
                    title: "Test API".to_string(),
                    version: "1.0.0".to_string(),
                    description: None,
                },
                servers: vec![],
                operations: vec![],
                components: None,
            }),
            config: ProtocolConfig {
                project_name: "test-server".to_string(),
                version: None,
                options,
            },
            role: Role::Server,
            language: crate::generation::Language::Rust,
        };

        let result = handler.prepare_context(input).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.protocol, Protocol::Mcp);
        assert_eq!(context.role, Role::Server);
        assert_eq!(context.variables["requires_openapi"], true);
        assert_eq!(context.variables["transport"], "http");
        assert_eq!(context.variables["port"], 8080);
    }

    #[tokio::test]
    async fn test_mcp_validate_configuration_success() {
        let handler = McpProtocolHandler::new();
        let mut options = std::collections::HashMap::new();
        options.insert("transport".to_string(), json!("stdio"));

        let config = ProtocolConfig {
            project_name: "valid-project-name".to_string(),
            version: Some("1.0.0".to_string()),
            options,
        };

        assert!(handler.validate_configuration(&config).is_ok());
    }

    #[tokio::test]
    async fn test_mcp_validate_configuration_empty_name() {
        let handler = McpProtocolHandler::new();
        let config = ProtocolConfig {
            project_name: "".to_string(),
            version: None,
            options: std::collections::HashMap::new(),
        };

        let result = handler.validate_configuration(&config);
        assert!(result.is_err());

        if let Err(ProtocolError::InvalidConfiguration(msg)) = result {
            assert!(msg.contains("name is required"));
        } else {
            panic!("Expected InvalidConfiguration error");
        }
    }

    #[tokio::test]
    async fn test_mcp_validate_configuration_invalid_name() {
        let handler = McpProtocolHandler::new();
        let config = ProtocolConfig {
            project_name: "invalid name!".to_string(),
            version: None,
            options: std::collections::HashMap::new(),
        };

        let result = handler.validate_configuration(&config);
        assert!(result.is_err());

        if let Err(ProtocolError::InvalidConfiguration(msg)) = result {
            assert!(msg.contains("alphanumeric"));
        } else {
            panic!("Expected InvalidConfiguration error");
        }
    }

    #[tokio::test]
    async fn test_mcp_validate_configuration_invalid_transport() {
        let handler = McpProtocolHandler::new();
        let mut options = std::collections::HashMap::new();
        options.insert("transport".to_string(), json!("invalid"));

        let config = ProtocolConfig {
            project_name: "test-project".to_string(),
            version: None,
            options,
        };

        let result = handler.validate_configuration(&config);
        assert!(result.is_err());

        if let Err(ProtocolError::InvalidConfiguration(msg)) = result {
            assert!(msg.contains("Invalid transport"));
        } else {
            panic!("Expected InvalidConfiguration error");
        }
    }

    #[tokio::test]
    async fn test_mcp_unsupported_role() {
        let handler = McpProtocolHandler::new();
        let input = ProtocolInput {
            openapi_spec: None,
            config: ProtocolConfig {
                project_name: "test-project".to_string(),
                version: None,
                options: std::collections::HashMap::new(),
            },
            role: Role::Agent,
            language: crate::generation::Language::Rust,
        };

        let result = handler.prepare_context(input).await;
        assert!(result.is_err());

        if let Err(ProtocolError::UnsupportedRole { protocol, role }) = result {
            assert_eq!(protocol, Protocol::Mcp);
            assert_eq!(role, Role::Agent);
        } else {
            panic!("Expected UnsupportedRole error");
        }
    }
}
