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
        let handler =
            self.protocol_registry
                .get(request.protocol)
                .ok_or(ApplicationError::ProtocolError(
                    crate::protocols::ProtocolError::NotImplemented(request.protocol),
                ))?;

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
                version: None,
                options: request.options.clone(),
            },
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
    use crate::generation::{self, GenerationError, Language};
    use crate::infrastructure::{Template, TemplateManifest, TemplateSource};
    use crate::protocols::{self, Protocol, Role};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_execute_success() {
        let protocol_registry = Arc::new(create_mock_registry());
        let openapi_loader = Arc::new(MockOpenApiLoader);
        let generation_orchestrator = Arc::new(create_mock_orchestrator());
        let output_service = Arc::new(MockOutputService::new());

        let use_case = GenerateServerUseCase::new(
            protocol_registry,
            openapi_loader,
            generation_orchestrator,
            output_service.clone(),
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

        // Verify the output service was called with artifacts
        let written = output_service.get_written_artifacts();
        assert_eq!(written.len(), 5);

        // Verify directory was ensured
        let dirs = output_service.get_ensured_directories();
        assert!(!dirs.is_empty());
        assert!(dirs.contains(&PathBuf::from("/output")));
    }

    // Helper functions to create mocks
    fn create_mock_registry() -> ProtocolRegistry {
        let registry = ProtocolRegistry::new();
        let _ = registry.register(
            Protocol::Mcp,
            Arc::new(protocols::handlers::mcp::McpProtocolHandler::new()),
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
    impl generation::OpenApiLoader for MockOpenApiLoader {
        async fn load(&self, _source: &str) -> Result<generation::OpenApiContext, GenerationError> {
            Ok(generation::OpenApiContext {
                version: "3.0.0".to_string(),
                info: generation::ApiInfo {
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

    struct MockOutputService {
        written_artifacts: std::sync::Mutex<Vec<generation::Artifact>>,
        ensured_directories: std::sync::Mutex<Vec<std::path::PathBuf>>,
    }

    impl MockOutputService {
        fn new() -> Self {
            Self {
                written_artifacts: std::sync::Mutex::new(Vec::new()),
                ensured_directories: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn get_written_artifacts(&self) -> Vec<generation::Artifact> {
            self.written_artifacts.lock().unwrap().clone()
        }

        fn get_ensured_directories(&self) -> Vec<std::path::PathBuf> {
            self.ensured_directories.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl OutputService for MockOutputService {
        async fn write_artifacts(
            &self,
            artifacts: &[generation::Artifact],
        ) -> Result<(), ApplicationError> {
            // Track artifacts being written
            let mut written = self.written_artifacts.lock().unwrap();
            written.extend(artifacts.iter().cloned());

            // Validate artifacts have content
            for artifact in artifacts {
                if artifact.path.to_string_lossy().is_empty() {
                    return Err(ApplicationError::GenerationError(
                        GenerationError::InvalidConfiguration(
                            "Artifact path cannot be empty".to_string(),
                        ),
                    ));
                }
                if artifact.content.is_empty() {
                    return Err(ApplicationError::GenerationError(
                        GenerationError::InvalidConfiguration(
                            "Artifact content cannot be empty".to_string(),
                        ),
                    ));
                }
            }

            Ok(())
        }

        async fn ensure_directory(&self, path: &std::path::Path) -> Result<(), ApplicationError> {
            // Track directories being ensured
            self.ensured_directories
                .lock()
                .unwrap()
                .push(path.to_path_buf());

            // Validate path
            if path.to_string_lossy().is_empty() {
                return Err(ApplicationError::GenerationError(
                    GenerationError::InvalidConfiguration(
                        "Directory path cannot be empty".to_string(),
                    ),
                ));
            }

            Ok(())
        }
    }

    struct MockTemplateDiscovery;

    #[async_trait::async_trait]
    impl generation::TemplateDiscovery for MockTemplateDiscovery {
        async fn discover(
            &self,
            protocol: Protocol,
            role: Role,
            language: Language,
        ) -> Result<Template, GenerationError> {
            Ok(Template {
                manifest: TemplateManifest {
                    name: "test-template".to_string(),
                    version: "1.0.0".to_string(),
                    description: Some("Test template".to_string()),
                    path: "test-template".to_string(),
                    protocol,
                    role,
                    language,
                    files: vec![],
                    variables: HashMap::new(),
                    post_generate_hooks: vec![],
                },
                files: vec![],
                source: TemplateSource::Embedded,
            })
        }
    }

    struct MockContextBuilder;

    #[async_trait::async_trait]
    impl generation::ContextBuilder for MockContextBuilder {
        async fn build(
            &self,
            _context: &generation::GenerationContext,
            _template: &Template,
        ) -> Result<generation::RenderContext, GenerationError> {
            Ok(generation::RenderContext::default())
        }
    }

    struct MockTemplateRenderer;

    #[async_trait::async_trait]
    impl generation::TemplateRenderingStrategy for MockTemplateRenderer {
        async fn render(
            &self,
            _template: &Template,
            _context: &generation::RenderContext,
            _generation_context: &generation::GenerationContext,
        ) -> Result<Vec<generation::Artifact>, GenerationError> {
            Ok(vec![
                crate::generation::Artifact {
                    path: PathBuf::from("src/main.rs"),
                    content: "fn main() {}".to_string(),
                    permissions: None,
                };
                5
            ])
        }
    }

    struct MockPostProcessor;

    #[async_trait::async_trait]
    impl generation::PostProcessor for MockPostProcessor {
        async fn process(
            &self,
            artifacts: Vec<generation::Artifact>,
            _context: &generation::GenerationContext,
            _post_generation_commands: &[String],
        ) -> Result<Vec<generation::Artifact>, GenerationError> {
            Ok(artifacts)
        }
    }
}
