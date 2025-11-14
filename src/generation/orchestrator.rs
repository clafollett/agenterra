//! Generation orchestration - coordinates the generation workflow

use crate::generation::{
    ContextBuilder, GenerationContext, GenerationError, GenerationResult, PostProcessor,
    TemplateDiscovery, TemplateRenderingStrategy,
};
use std::sync::Arc;

/// Orchestrates the code generation workflow
pub struct GenerationOrchestrator {
    template_discovery: Arc<dyn TemplateDiscovery>,
    context_builder: Arc<dyn ContextBuilder>,
    template_renderer: Arc<dyn TemplateRenderingStrategy>,
    post_processor: Arc<dyn PostProcessor>,
}

impl GenerationOrchestrator {
    /// Create a new generation orchestrator
    pub fn new(
        template_discovery: Arc<dyn TemplateDiscovery>,
        context_builder: Arc<dyn ContextBuilder>,
        template_renderer: Arc<dyn TemplateRenderingStrategy>,
        post_processor: Arc<dyn PostProcessor>,
    ) -> Self {
        Self {
            template_discovery,
            context_builder,
            template_renderer,
            post_processor,
        }
    }

    /// Execute the generation workflow
    pub async fn generate(
        &self,
        context: GenerationContext,
    ) -> Result<GenerationResult, GenerationError> {
        tracing::info!("GenerationOrchestrator: Starting generation workflow.");

        // 1. Validate context
        context.validate()?;
        tracing::info!("GenerationOrchestrator: Context validated successfully.");

        tracing::info!(
            "Orchestrator starting generation for {:?}/{:?}",
            context.protocol,
            context.role
        );

        // 2. Discover template based on context attributes
        let template = self
            .template_discovery
            .discover(context.protocol, context.role.clone(), context.language)
            .await?;
        tracing::info!("GenerationOrchestrator: Template discovered: {}", template.source);

        tracing::info!(
            protocol = %context.protocol,
            role = %context.role,
            language = %context.language,
            source = %template.source,
            "Using template for generation"
        );

        // 3. Build render context from generation context
        let render_context = self.context_builder.build(&context, &template).await?;
        tracing::info!("GenerationOrchestrator: Render context built.");

        // 4. Render templates to artifacts using strategy pattern
        let artifacts = self
            .template_renderer
            .render(&template, &render_context, &context)
            .await?;
        tracing::info!("GenerationOrchestrator: Templates rendered to {} artifacts.", artifacts.len());

        // 5. Post-process artifacts (permissions only, not commands)
        let processed_artifacts = self
            .post_processor
            .process(artifacts, &context, &[]) // Empty commands - we'll run them later
            .await?;
        tracing::info!("GenerationOrchestrator: Artifacts post-processed.");

        // 6. Return result with post-generation commands
        tracing::info!("GenerationOrchestrator: Generation workflow completed successfully.");
        Ok(GenerationResult {
            artifacts: processed_artifacts,
            metadata: context.metadata,
            post_generation_commands: template.manifest.post_generate_hooks,
        })
    }
}
