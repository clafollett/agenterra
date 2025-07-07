//! Generation domain module - orchestrates code generation workflow
//!
//! This module implements the core code generation logic, taking protocol
//! contexts and transforming them into generated code artifacts through
//! template discovery, rendering, and post-processing.

pub mod adapters;
pub mod context;
pub mod errors;
pub mod orchestrator;
pub mod rules;
pub mod sanitizers;
pub mod traits;
pub mod types;
pub mod utils;

pub use adapters::*;
pub use context::*;
pub use errors::*;
pub use orchestrator::*;
pub use traits::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::{Protocol, Role};
    use std::str::FromStr;

    #[test]
    fn test_generation_context_creation() {
        let context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);

        assert_eq!(context.protocol, Protocol::Mcp);
        assert_eq!(context.role, Role::Server);
        assert_eq!(context.language, Language::Rust);
        assert!(context.variables.is_empty());
        assert!(context.protocol_context.is_none());
    }

    #[test]
    fn test_context_validation_empty_project_name() {
        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "".to_string();

        let result = context.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            GenerationError::ValidationError(msg) => {
                assert!(msg.contains("Project name is required"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_context_validation_invalid_role_for_protocol() {
        let mut context = GenerationContext::new(Protocol::A2a, Role::Server, Language::Rust);
        context.metadata.project_name = "test-project".to_string();

        let result = context.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            GenerationError::ValidationError(msg) => {
                assert!(msg.contains("Invalid role for protocol"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_context_validation_valid_protocol_role_combinations() {
        // Test valid MCP combinations
        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "valid-project".to_string();
        assert!(context.validate().is_ok());

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Client, Language::Rust);
        context.metadata.project_name = "valid-project".to_string();
        assert!(context.validate().is_ok());

        // Test valid A2A combination
        let mut context = GenerationContext::new(Protocol::A2a, Role::Agent, Language::Rust);
        context.metadata.project_name = "valid-project".to_string();
        assert!(context.validate().is_ok());
    }

    #[test]
    fn test_generation_context_add_variable() {
        let mut context = GenerationContext::new(Protocol::Mcp, Role::Client, Language::Python);

        context.add_variable("project_name".to_string(), serde_json::json!("test-client"));
        context.add_variable("version".to_string(), serde_json::json!("1.0.0"));

        assert_eq!(context.variables.len(), 2);
        assert_eq!(context.variables["project_name"], "test-client");
        assert_eq!(context.variables["version"], "1.0.0");
    }

    #[test]
    fn test_generation_metadata_default() {
        let metadata = GenerationMetadata::default();
        assert_eq!(metadata.project_name, "");
        assert_eq!(metadata.version, "0.1.0");
        assert!(metadata.description.is_none());
        assert!(metadata.author.is_none());
    }

    #[test]
    fn test_language_properties() {
        assert_eq!(Language::Rust.to_string(), "rust");
        assert_eq!(Language::Python.display_name(), "Python");
        assert_eq!(Language::TypeScript.file_extension(), "ts");

        let all_langs = Language::all();
        assert_eq!(all_langs.len(), 6);
        assert!(all_langs.contains(&Language::Rust));
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("rust").unwrap(), Language::Rust);
        assert_eq!(Language::from_str("PYTHON").unwrap(), Language::Python);
        assert_eq!(Language::from_str("ts").unwrap(), Language::TypeScript);
        assert!(Language::from_str("unknown").is_err());
    }

    #[test]
    fn test_artifact_creation() {
        let artifact = Artifact {
            path: std::path::PathBuf::from("src/main.rs"),
            content: "fn main() {}".to_string(),
            permissions: Some(0o755),
        };

        assert_eq!(artifact.path.to_str().unwrap(), "src/main.rs");
        assert_eq!(artifact.content, "fn main() {}");
        assert_eq!(artifact.permissions, Some(0o755));
    }

    #[test]
    fn test_generation_result() {
        let artifacts = vec![Artifact {
            path: std::path::PathBuf::from("src/lib.rs"),
            content: "// lib".to_string(),
            permissions: None,
        }];

        let metadata = GenerationMetadata {
            project_name: "test-project".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };

        let result = GenerationResult {
            artifacts: artifacts.clone(),
            metadata: metadata.clone(),
        };

        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(result.metadata.project_name, "test-project");
    }

    #[tokio::test]
    async fn test_orchestrator_validates_context_before_processing() {
        use std::sync::Arc;

        // Simple no-op mocks just for this test
        struct NoOpTemplateDiscovery;
        #[async_trait::async_trait]
        impl TemplateDiscovery for NoOpTemplateDiscovery {
            async fn discover(
                &self,
                _protocol: crate::protocols::Protocol,
                _role: crate::protocols::Role,
                _language: crate::generation::Language,
            ) -> Result<crate::infrastructure::Template, GenerationError> {
                panic!("Should not be called when validation fails");
            }
        }

        struct NoOpContextBuilder;
        #[async_trait::async_trait]
        impl ContextBuilder for NoOpContextBuilder {
            async fn build(
                &self,
                _context: &GenerationContext,
                _template: &crate::infrastructure::Template,
            ) -> Result<RenderContext, GenerationError> {
                panic!("Should not be called when validation fails");
            }
        }

        struct NoOpTemplateRenderer;
        #[async_trait::async_trait]
        impl TemplateRenderingStrategy for NoOpTemplateRenderer {
            async fn render(
                &self,
                _template: &crate::infrastructure::Template,
                _context: &RenderContext,
                _generation_context: &GenerationContext,
            ) -> Result<Vec<Artifact>, GenerationError> {
                panic!("Should not be called when validation fails");
            }
        }

        struct NoOpPostProcessor;
        #[async_trait::async_trait]
        impl PostProcessor for NoOpPostProcessor {
            async fn process(
                &self,
                _artifacts: Vec<Artifact>,
                _context: &GenerationContext,
                _post_generation_commands: &[String],
            ) -> Result<Vec<Artifact>, GenerationError> {
                panic!("Should not be called when validation fails");
            }
        }

        let orchestrator = GenerationOrchestrator::new(
            Arc::new(NoOpTemplateDiscovery),
            Arc::new(NoOpContextBuilder),
            Arc::new(NoOpTemplateRenderer),
            Arc::new(NoOpPostProcessor),
        );

        // Test with invalid context (empty project name)
        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "".to_string();

        let result = orchestrator.generate(context).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            GenerationError::ValidationError(msg) => {
                assert!(msg.contains("Project name is required"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_orchestrator_workflow_with_valid_context() {
        use std::sync::Arc;
        use std::sync::Mutex;

        // Create a shared vec to track call order
        let call_order = Arc::new(Mutex::new(Vec::new()));

        // Create tracking mocks
        let discovery_order = call_order.clone();
        let builder_order = call_order.clone();
        let renderer_order = call_order.clone();
        let processor_order = call_order.clone();

        struct TrackingTemplateDiscovery {
            call_order: Arc<Mutex<Vec<&'static str>>>,
        }

        #[async_trait::async_trait]
        impl TemplateDiscovery for TrackingTemplateDiscovery {
            async fn discover(
                &self,
                protocol: crate::protocols::Protocol,
                role: crate::protocols::Role,
                language: crate::generation::Language,
            ) -> Result<crate::infrastructure::Template, GenerationError> {
                self.call_order.lock().unwrap().push("discover");
                use std::collections::HashMap;
                let manifest = crate::infrastructure::TemplateManifest {
                    name: "test-template".to_string(),
                    version: "1.0.0".to_string(),
                    description: None,
                    path: format!("{protocol}/{role}/{language}"),
                    protocol,
                    role,
                    language,
                    files: vec![],
                    variables: HashMap::new(),
                    post_generate_hooks: vec![],
                };
                Ok(crate::infrastructure::Template {
                    manifest,
                    files: vec![],
                    source: crate::infrastructure::TemplateSource::Embedded,
                })
            }
        }

        struct TrackingContextBuilder {
            call_order: Arc<Mutex<Vec<&'static str>>>,
        }

        #[async_trait::async_trait]
        impl ContextBuilder for TrackingContextBuilder {
            async fn build(
                &self,
                _context: &GenerationContext,
                _template: &crate::infrastructure::Template,
            ) -> Result<RenderContext, GenerationError> {
                self.call_order.lock().unwrap().push("build");
                Ok(RenderContext::default())
            }
        }

        struct TrackingTemplateRenderer {
            call_order: Arc<Mutex<Vec<&'static str>>>,
        }

        #[async_trait::async_trait]
        impl TemplateRenderingStrategy for TrackingTemplateRenderer {
            async fn render(
                &self,
                _template: &crate::infrastructure::Template,
                _context: &RenderContext,
                _generation_context: &GenerationContext,
            ) -> Result<Vec<Artifact>, GenerationError> {
                self.call_order.lock().unwrap().push("render");
                Ok(vec![])
            }
        }

        struct TrackingPostProcessor {
            call_order: Arc<Mutex<Vec<&'static str>>>,
        }

        #[async_trait::async_trait]
        impl PostProcessor for TrackingPostProcessor {
            async fn process(
                &self,
                artifacts: Vec<Artifact>,
                _context: &GenerationContext,
                _post_generation_commands: &[String],
            ) -> Result<Vec<Artifact>, GenerationError> {
                self.call_order.lock().unwrap().push("process");
                Ok(artifacts)
            }
        }

        let orchestrator = GenerationOrchestrator::new(
            Arc::new(TrackingTemplateDiscovery {
                call_order: discovery_order,
            }),
            Arc::new(TrackingContextBuilder {
                call_order: builder_order,
            }),
            Arc::new(TrackingTemplateRenderer {
                call_order: renderer_order,
            }),
            Arc::new(TrackingPostProcessor {
                call_order: processor_order,
            }),
        );

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "test-project".to_string();

        let result = orchestrator.generate(context).await;
        assert!(result.is_ok());

        // Verify the correct order of operations
        let order = call_order.lock().unwrap();
        assert_eq!(*order, vec!["discover", "build", "render", "process"]);
    }

    #[test]
    fn test_protocol_context_for_mcp_server() {
        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "test-api".to_string();

        // Create a sample OpenAPI spec
        let openapi_spec = OpenApiContext {
            version: "3.0.0".to_string(),
            info: ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API description".to_string()),
            },
            servers: vec![Server {
                url: "https://api.example.com".to_string(),
                description: Some("Production server".to_string()),
            }],
            operations: vec![],
            components: None,
        };

        // Set protocol context for MCP Server
        context.protocol_context = Some(ProtocolContext::McpServer {
            openapi_spec: openapi_spec.clone(),
            endpoints: vec![],
        });

        // Verify we can extract the data back
        match &context.protocol_context {
            Some(ProtocolContext::McpServer {
                openapi_spec: spec,
                endpoints,
            }) => {
                assert_eq!(spec.version, "3.0.0");
                assert_eq!(spec.info.title, "Test API");
                assert!(endpoints.is_empty());
            }
            _ => panic!("Expected McpServer protocol context"),
        }
    }

    #[test]
    fn test_protocol_context_none_for_mcp_client() {
        let context = GenerationContext::new(Protocol::Mcp, Role::Client, Language::Rust);
        // MCP clients don't need OpenAPI spec
        assert!(context.protocol_context.is_none());
    }
}
