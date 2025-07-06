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
        assert!(context.operations.is_empty());
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
    fn test_template_descriptor_creation() {
        use crate::infrastructure::templates::TemplateDescriptor;

        let descriptor = TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::TypeScript);

        assert_eq!(descriptor.protocol, Protocol::Mcp);
        assert_eq!(descriptor.role, Role::Server);
        assert_eq!(descriptor.language, Language::TypeScript);
        assert_eq!(descriptor.path(), "mcp/server/typescript");
    }

    #[test]
    fn test_language_properties() {
        assert_eq!(Language::Rust.as_str(), "rust");
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
            post_commands: vec!["cargo fmt".to_string()],
        };

        assert_eq!(artifact.path.to_str().unwrap(), "src/main.rs");
        assert_eq!(artifact.content, "fn main() {}");
        assert_eq!(artifact.permissions, Some(0o755));
        assert_eq!(artifact.post_commands.len(), 1);
    }

    #[test]
    fn test_generation_result() {
        let artifacts = vec![Artifact {
            path: std::path::PathBuf::from("src/lib.rs"),
            content: "// lib".to_string(),
            permissions: None,
            post_commands: vec![],
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

    // Mock implementations for testing
    struct MockTemplateDiscovery;
    struct MockContextBuilder;
    struct MockTemplateRenderer;
    struct MockPostProcessor;

    #[async_trait::async_trait]
    impl TemplateDiscovery for MockTemplateDiscovery {
        async fn discover(
            &self,
            _descriptor: &crate::infrastructure::templates::TemplateDescriptor,
        ) -> Result<crate::infrastructure::templates::Template, GenerationError> {
            use crate::infrastructure::templates::{
                Template, TemplateDescriptor, TemplateManifest, TemplateSource,
            };

            Ok(Template {
                descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::Rust),
                manifest: TemplateManifest::default(),
                files: vec![],
                source: TemplateSource::Embedded,
            })
        }
    }

    #[async_trait::async_trait]
    impl ContextBuilder for MockContextBuilder {
        async fn build(
            &self,
            _context: &GenerationContext,
            _template: &crate::infrastructure::templates::Template,
        ) -> Result<RenderContext, GenerationError> {
            Ok(RenderContext::default())
        }
    }

    #[async_trait::async_trait]
    impl TemplateRenderingStrategy for MockTemplateRenderer {
        async fn render(
            &self,
            // TODO: Need to implement usage of these parameters
            _template: &crate::infrastructure::templates::Template,
            _context: &RenderContext,
            _generation_context: &GenerationContext,
        ) -> Result<Vec<Artifact>, GenerationError> {
            Ok(vec![])
        }
    }

    #[async_trait::async_trait]
    impl PostProcessor for MockPostProcessor {
        async fn process(
            &self,
            artifacts: Vec<Artifact>,
            _context: &GenerationContext,
        ) -> Result<Vec<Artifact>, GenerationError> {
            Ok(artifacts)
        }
    }

    #[tokio::test]
    async fn test_generation_orchestrator_workflow() {
        use std::sync::Arc;

        let orchestrator = GenerationOrchestrator::new(
            Arc::new(MockTemplateDiscovery),
            Arc::new(MockContextBuilder),
            Arc::new(MockTemplateRenderer),
            Arc::new(MockPostProcessor),
        );

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "test".to_string();

        let result = orchestrator.generate(context).await;
        assert!(result.is_ok());
    }
}
