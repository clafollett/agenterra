//! Use case for generating client implementations

use crate::application::{
    ApplicationError, GenerateClientRequest, GenerateClientResponse, OutputService,
};
use crate::generation::GenerationOrchestrator;
use crate::protocols::{ProtocolConfig, ProtocolError, ProtocolInput, ProtocolRegistry, Role};
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
        let handler =
            self.protocol_registry
                .get(request.protocol)
                .ok_or(ApplicationError::ProtocolError(
                    ProtocolError::NotImplemented(request.protocol),
                ))?;

        // 3. Prepare protocol input (no OpenAPI needed for clients)
        let input = ProtocolInput {
            role: Role::Client,
            language: request.language,
            config: ProtocolConfig {
                project_name: request.project_name.clone(),
                version: None,
                options: request.options.clone(),
            },
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
    use crate::generation::{self, Language};
    use crate::infrastructure;
    use crate::protocols::{self, Protocol};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_execute_success() {
        let protocol_registry = Arc::new(create_mock_registry());
        let template_discovery = Arc::new(MockTemplateDiscovery::new());
        let output_service = Arc::new(MockOutputService::new());
        let generation_orchestrator =
            Arc::new(create_mock_orchestrator(template_discovery.clone()));

        let use_case = GenerateClientUseCase::new(
            protocol_registry,
            generation_orchestrator,
            output_service.clone(),
        );

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

        // Verify template discovery was called with correct descriptor
        let discovered = template_discovery.get_discovered_templates();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].0, Protocol::Mcp);
        assert_eq!(discovered[0].1, Role::Client);
        assert_eq!(discovered[0].2, Language::Rust);

        // Verify output service was called
        let ensured_dirs = output_service.get_ensured_directories();
        assert_eq!(ensured_dirs.len(), 1);
        assert_eq!(ensured_dirs[0], PathBuf::from("/output"));

        let written = output_service.get_written_artifacts();
        assert_eq!(written.len(), 3);
        // Verify paths were prepended with output directory
        for artifact in &written {
            assert!(artifact.path.starts_with("/output"));
        }
    }

    #[tokio::test]
    async fn test_execute_invalid_protocol() {
        let protocol_registry = Arc::new(ProtocolRegistry::new()); // Empty registry
        let template_discovery = Arc::new(MockTemplateDiscovery::new());
        let output_service = Arc::new(MockOutputService::new());
        let generation_orchestrator = Arc::new(create_mock_orchestrator(template_discovery));

        let use_case =
            GenerateClientUseCase::new(protocol_registry, generation_orchestrator, output_service);

        let request = GenerateClientRequest {
            protocol: Protocol::A2a, // Not registered
            language: Language::Rust,
            project_name: "test-client".to_string(),
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        let result = use_case.execute(request).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        // A2a protocol exists but doesn't have a handler registered, so we get ValidationError
        match err {
            ApplicationError::ValidationError(_) => {
                // Expected: A2a doesn't support Client role
            }
            _ => panic!("Expected ValidationError, got: {err:?}"),
        }
    }

    #[tokio::test]
    async fn test_execute_invalid_request() {
        let protocol_registry = Arc::new(create_mock_registry());
        let template_discovery = Arc::new(MockTemplateDiscovery::new());
        let output_service = Arc::new(MockOutputService::new());
        let generation_orchestrator = Arc::new(create_mock_orchestrator(template_discovery));

        let use_case =
            GenerateClientUseCase::new(protocol_registry, generation_orchestrator, output_service);

        let request = GenerateClientRequest {
            protocol: Protocol::Mcp,
            language: Language::Rust,
            project_name: "".to_string(), // Invalid: empty project name
            output_dir: PathBuf::from("/output"),
            options: HashMap::new(),
        };

        let result = use_case.execute(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ApplicationError::ValidationError(_) => {}
            _ => panic!("Expected ValidationError"),
        }
    }

    // Helper functions
    fn create_mock_registry() -> ProtocolRegistry {
        let registry = ProtocolRegistry::new();
        let _ = registry.register(
            Protocol::Mcp,
            Arc::new(protocols::handlers::mcp::McpProtocolHandler::new()),
        );
        registry
    }

    fn create_mock_orchestrator(
        template_discovery: Arc<MockTemplateDiscovery>,
    ) -> GenerationOrchestrator {
        GenerationOrchestrator::new(
            template_discovery,
            Arc::new(MockContextBuilder),
            Arc::new(MockTemplateRenderer),
            Arc::new(MockPostProcessor),
        )
    }

    // Mock implementations
    struct MockOutputService {
        written_artifacts: std::sync::Mutex<Vec<generation::Artifact>>,
        ensured_directories: std::sync::Mutex<Vec<PathBuf>>,
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

        fn get_ensured_directories(&self) -> Vec<PathBuf> {
            self.ensured_directories.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl OutputService for MockOutputService {
        async fn write_artifacts(
            &self,
            artifacts: &[generation::Artifact],
        ) -> Result<(), ApplicationError> {
            self.written_artifacts
                .lock()
                .unwrap()
                .extend_from_slice(artifacts);
            Ok(())
        }

        async fn ensure_directory(&self, path: &std::path::Path) -> Result<(), ApplicationError> {
            self.ensured_directories
                .lock()
                .unwrap()
                .push(path.to_path_buf());
            Ok(())
        }
    }

    struct MockTemplateDiscovery {
        discovered_templates:
            std::sync::Mutex<Vec<(protocols::Protocol, protocols::Role, generation::Language)>>,
    }

    impl MockTemplateDiscovery {
        fn new() -> Self {
            Self {
                discovered_templates: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn get_discovered_templates(
            &self,
        ) -> Vec<(protocols::Protocol, protocols::Role, generation::Language)> {
            self.discovered_templates.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl generation::TemplateDiscovery for MockTemplateDiscovery {
        async fn discover(
            &self,
            protocol: protocols::Protocol,
            role: protocols::Role,
            language: generation::Language,
        ) -> Result<infrastructure::Template, generation::GenerationError> {
            self.discovered_templates
                .lock()
                .unwrap()
                .push((protocol, role.clone(), language));

            // Return a template that matches the requested parameters
            Ok(infrastructure::Template {
                manifest: infrastructure::TemplateManifest {
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
                source: infrastructure::TemplateSource::Embedded,
            })
        }
    }

    struct MockContextBuilder;

    #[async_trait::async_trait]
    impl generation::ContextBuilder for MockContextBuilder {
        async fn build(
            &self,
            _context: &generation::GenerationContext,
            _template: &infrastructure::Template,
        ) -> Result<generation::RenderContext, generation::GenerationError> {
            Ok(generation::RenderContext::default())
        }
    }

    struct MockTemplateRenderer;

    #[async_trait::async_trait]
    impl generation::TemplateRenderingStrategy for MockTemplateRenderer {
        async fn render(
            &self,
            _template: &infrastructure::Template,
            _context: &generation::RenderContext,
            _generation_context: &generation::GenerationContext,
        ) -> Result<Vec<generation::Artifact>, generation::GenerationError> {
            Ok(vec![
                generation::Artifact {
                    path: PathBuf::from("src/main.rs"),
                    content: "fn main() {}".to_string(),
                    permissions: None,
                };
                3  // Client has fewer artifacts than server
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
        ) -> Result<Vec<generation::Artifact>, generation::GenerationError> {
            Ok(artifacts)
        }
    }
}
