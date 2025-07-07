//! Port interfaces for the generation domain

use crate::generation::{Artifact, GenerationContext, GenerationError, RenderContext};
use crate::infrastructure::Template;
use async_trait::async_trait;

/// Builds render context from generation context
#[async_trait]
pub trait ContextBuilder: Send + Sync {
    /// Build a render context for template rendering
    async fn build(
        &self,
        context: &GenerationContext,
        template: &Template,
    ) -> Result<RenderContext, GenerationError>;
}

/// Template rendering strategy - protocol-specific rendering logic
#[async_trait]
pub trait TemplateRenderingStrategy: Send + Sync {
    /// Render templates with context using protocol-specific logic
    async fn render(
        &self,
        template: &Template,
        context: &RenderContext,
        generation_context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError>;
}

/// Post-processes generated artifacts and executes post-generation commands
#[async_trait]
pub trait PostProcessor: Send + Sync {
    /// Process artifacts after generation and execute post-generation commands
    async fn process(
        &self,
        artifacts: Vec<Artifact>,
        context: &GenerationContext,
        post_generation_commands: &[String],
    ) -> Result<Vec<Artifact>, GenerationError>;
}

/// Loads OpenAPI specifications
#[async_trait]
pub trait OpenApiLoader: Send + Sync {
    /// Load an OpenAPI spec from a source
    async fn load(
        &self,
        source: &str,
    ) -> Result<crate::generation::OpenApiContext, GenerationError>;
}
