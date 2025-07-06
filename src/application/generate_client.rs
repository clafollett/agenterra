//! Use case for generating client implementations

use crate::application::{
    ApplicationError, GenerateClientRequest, GenerateClientResponse, OutputService,
};
use crate::generation::GenerationOrchestrator;
use crate::protocols::{ProtocolConfig, ProtocolInput, ProtocolRegistry, Role};
use std::sync::Arc;

/// Use case for generating client implementations
pub struct GenerateClientUseCase {
    protocol_registry: Arc<ProtocolRegistry>,
    generation_orchestrator: Arc<GenerationOrchestrator>,
    output_service: Arc<dyn OutputService>,
}

impl GenerateClientUseCase {
    pub fn new(
        protocol_registry: Arc<ProtocolRegistry>,
        generation_orchestrator: Arc<GenerationOrchestrator>,
        output_service: Arc<dyn OutputService>,
    ) -> Self {
        Self {
            protocol_registry,
            generation_orchestrator,
            output_service,
        }
    }

    pub async fn execute(
        &self,
        request: GenerateClientRequest,
    ) -> Result<GenerateClientResponse, ApplicationError> {
        // 1. Validate request
        request.validate()?;

        // 2. Get protocol handler
        let handler = self
            .protocol_registry
            .get(request.protocol)
            .ok_or(ApplicationError::ProtocolNotImplemented(request.protocol))?;

        // 3. Prepare protocol input (no OpenAPI needed for clients)
        let input = ProtocolInput {
            role: Role::Client,
            language: request.language,
            config: ProtocolConfig {
                project_name: request.project_name.clone(),
                output_dir: request.output_dir.clone(),
                version: None,
                options: request.options.clone(),
            },
            openapi_path: None,
            openapi_spec: None,
        };

        // 4. Build generation context
        let context = handler.prepare_context(input).await?;

        // 5. Generate code
        let result = self.generation_orchestrator.generate(context).await?;

        // 6. Ensure output directory exists
        self.output_service
            .ensure_directory(&request.output_dir)
            .await?;

        // 7. Prepend output directory to artifact paths and write
        let mut output_artifacts = result.artifacts;
        for artifact in &mut output_artifacts {
            artifact.path = request.output_dir.join(&artifact.path);
        }

        let artifacts_count = output_artifacts.len();

        self.output_service
            .write_artifacts(&output_artifacts)
            .await?;

        Ok(GenerateClientResponse {
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
        let generation_orchestrator = Arc::new(create_mock_orchestrator());
        let output_service = Arc::new(MockOutputService);

        let use_case =
            GenerateClientUseCase::new(protocol_registry, generation_orchestrator, output_service);

        let request = GenerateClientRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "test-client".to_string(),
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        let response = use_case.execute(request).await.unwrap();
        assert_eq!(response.artifacts_count, 3);
        assert_eq!(response.output_path, PathBuf::from("/output"));
    }

    // Helper functions
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
        ) -> Result<crate::infrastructure::templates::Template, crate::generation::GenerationError>
        {
            Ok(crate::infrastructure::templates::Template {
                descriptor: crate::infrastructure::templates::TemplateDescriptor::new(
                    Protocol::Mcp,
                    crate::protocols::Role::Client,
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
            // TODO: Need to implement usage of these parameters
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
                3  // Client has fewer artifacts than server
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
