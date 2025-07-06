//! Use case for generating server implementations

use crate::application::{
    ApplicationError, GenerateServerRequest, GenerateServerResponse, OutputService,
};
use crate::generation::{GenerationOrchestrator, OpenApiLoader};
use crate::protocols::{ProtocolConfig, ProtocolInput, ProtocolRegistry, Role};
use std::sync::Arc;

/// Use case for generating server implementations
pub struct GenerateServerUseCase {
    protocol_registry: Arc<ProtocolRegistry>,
    openapi_loader: Arc<dyn OpenApiLoader>,
    generation_orchestrator: Arc<GenerationOrchestrator>,
    output_service: Arc<dyn OutputService>,
}

impl GenerateServerUseCase {
    pub fn new(
        protocol_registry: Arc<ProtocolRegistry>,
        openapi_loader: Arc<dyn OpenApiLoader>,
        generation_orchestrator: Arc<GenerationOrchestrator>,
        output_service: Arc<dyn OutputService>,
    ) -> Self {
        Self {
            protocol_registry,
            openapi_loader,
            generation_orchestrator,
            output_service,
        }
    }

    pub async fn execute(
        &self,
        request: GenerateServerRequest,
    ) -> Result<GenerateServerResponse, ApplicationError> {
        // 1. Validate request
        request.validate()?;

        // 2. Get protocol handler
        let handler = self
            .protocol_registry
            .get(request.protocol)
            .ok_or(ApplicationError::ProtocolNotImplemented(request.protocol))?;

        // 3. Load OpenAPI if needed
        let capabilities = handler.protocol().capabilities();
        let openapi_spec = if capabilities.requires_openapi {
            match &request.schema_path {
                Some(path) => Some(self.openapi_loader.load(path).await?),
                None => {
                    return Err(ApplicationError::ValidationError(
                        crate::application::ValidationError::MissingField(
                            "MCP server requires OpenAPI schema path".to_string(),
                        ),
                    ));
                }
            }
        } else {
            None
        };

        // 4. Prepare protocol input
        let input = ProtocolInput {
            role: Role::Server,
            language: request.language,
            config: ProtocolConfig {
                project_name: request.project_name.clone(),
                output_dir: request.output_dir.clone(),
                version: None,
                options: request.options.clone(),
            },
            openapi_path: request
                .schema_path
                .as_ref()
                .map(|p| std::path::PathBuf::from(p)),
            openapi_spec,
        };

        // 5. Build generation context
        let context = handler.prepare_context(input).await?;

        // 6. Generate code
        let result = self.generation_orchestrator.generate(context).await?;

        // 7. Ensure output directory exists
        self.output_service
            .ensure_directory(&request.output_dir)
            .await?;

        // 8. Prepend output directory to artifact paths and write
        let mut output_artifacts = result.artifacts;
        for artifact in &mut output_artifacts {
            artifact.path = request.output_dir.join(&artifact.path);
        }
        
        let artifacts_count = output_artifacts.len();
        
        self.output_service
            .write_artifacts(&output_artifacts)
            .await?;

        Ok(GenerateServerResponse {
            artifacts_count,
            output_path: request.output_dir,
            metadata: result.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::Language;
    use crate::protocols::Protocol;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_execute_success() {
        let protocol_registry = Arc::new(create_mock_registry());
        let openapi_loader = Arc::new(MockOpenApiLoader);
        let generation_orchestrator = Arc::new(create_mock_orchestrator());
        let output_service = Arc::new(MockOutputService);

        let use_case = GenerateServerUseCase::new(
            protocol_registry,
            openapi_loader,
            generation_orchestrator,
            output_service,
        );

        let request = GenerateServerRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "test-server".to_string(),
            schema_path: Some("/path/to/openapi.yaml".to_string()),
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        let response = use_case.execute(request).await.unwrap();
        assert_eq!(response.artifacts_count, 5);
        assert_eq!(response.output_path, PathBuf::from("/output"));
    }

    // Helper functions to create mocks
    fn create_mock_registry() -> ProtocolRegistry {
        let mut registry = ProtocolRegistry::new();
        let _ = registry.register(
            Protocol::Mcp,
            Arc::new(crate::protocols::handlers::mcp::McpProtocolHandler::new()),
        );
        registry
    }

    fn create_mock_orchestrator() -> GenerationOrchestrator {
        GenerationOrchestrator::new(
            Arc::new(MockTemplateDiscovery),
            Arc::new(MockContextBuilder),
            Arc::new(MockTemplateRenderer),
            Arc::new(MockPostProcessor),
        )
    }

    // Mock implementations
    struct MockOpenApiLoader;

    #[async_trait::async_trait]
    impl crate::generation::OpenApiLoader for MockOpenApiLoader {
        async fn load(
            &self,
            _source: &str,
        ) -> Result<crate::generation::OpenApiSpec, crate::generation::GenerationError> {
            Ok(crate::generation::OpenApiSpec {
                version: "3.0.0".to_string(),
                info: crate::generation::ApiInfo {
                    title: "Test API".to_string(),
                    version: "1.0.0".to_string(),
                    description: None,
                },
                servers: vec![],
                operations: vec![],
                components: None,
            })
        }
    }

    struct MockOutputService;

    #[async_trait::async_trait]
    impl OutputService for MockOutputService {
        async fn write_artifacts(
            &self,
            _artifacts: &[crate::generation::Artifact],
        ) -> Result<(), ApplicationError> {
            Ok(())
        }

        async fn ensure_directory(&self, _path: &std::path::Path) -> Result<(), ApplicationError> {
            Ok(())
        }
    }

    struct MockTemplateDiscovery;

    #[async_trait::async_trait]
    impl crate::generation::TemplateDiscovery for MockTemplateDiscovery {
        async fn discover(
            &self,
            _descriptor: &crate::infrastructure::templates::TemplateDescriptor,
        ) -> Result<crate::infrastructure::templates::Template, crate::generation::GenerationError> {
            Ok(crate::infrastructure::templates::Template {
                descriptor: crate::infrastructure::templates::TemplateDescriptor::new(
                    Protocol::Mcp,
                    crate::protocols::Role::Server,
                    Language::Rust,
                ),
                manifest: crate::infrastructure::templates::TemplateManifest::default(),
                files: vec![],
                source: crate::infrastructure::templates::TemplateSource::Embedded,
            })
        }
    }

    struct MockContextBuilder;

    #[async_trait::async_trait]
    impl crate::generation::ContextBuilder for MockContextBuilder {
        async fn build(
            &self,
            _context: &crate::generation::GenerationContext,
            _template: &crate::infrastructure::templates::Template,
        ) -> Result<crate::generation::RenderContext, crate::generation::GenerationError> {
            Ok(crate::generation::RenderContext::default())
        }
    }

    struct MockTemplateRenderer;

    #[async_trait::async_trait]
    impl crate::generation::TemplateRenderingStrategy for MockTemplateRenderer {
        async fn render(
            &self,
            _template: &crate::infrastructure::templates::Template,
            _context: &crate::generation::RenderContext,
            _generation_context: &crate::generation::GenerationContext,
        ) -> Result<Vec<crate::generation::Artifact>, crate::generation::GenerationError> {
            Ok(vec![
                crate::generation::Artifact {
                    path: PathBuf::from("src/main.rs"),
                    content: "fn main() {}".to_string(),
                    permissions: None,
                    post_commands: vec![],
                };
                5
            ])
        }
    }

    struct MockPostProcessor;

    #[async_trait::async_trait]
    impl crate::generation::PostProcessor for MockPostProcessor {
        async fn process(
            &self,
            artifacts: Vec<crate::generation::Artifact>,
            _context: &crate::generation::GenerationContext,
        ) -> Result<Vec<crate::generation::Artifact>, crate::generation::GenerationError> {
            Ok(artifacts)
        }
    }
}
