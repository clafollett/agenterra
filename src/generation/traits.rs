//! Port interfaces for the generation domain

use crate::generation::{Artifact, GenerationContext, GenerationError, RenderContext};
use crate::infrastructure::templates::Template;
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

/// Renders templates to artifacts
#[async_trait]
pub trait TemplateRenderer: Send + Sync {
    /// Render a template with the given context
    async fn render(
        &self,
        template: &Template,
        context: &RenderContext,
    ) -> Result<Vec<Artifact>, GenerationError>;
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

/// Post-processes generated artifacts
#[async_trait]
pub trait PostProcessor: Send + Sync {
    /// Process artifacts after generation
    async fn process(
        &self,
        artifacts: Vec<Artifact>,
        context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError>;
}

/// Loads OpenAPI specifications
#[async_trait]
pub trait OpenApiLoader: Send + Sync {
    /// Load an OpenAPI spec from a source
    async fn load(&self, source: &str) -> Result<crate::generation::OpenApiSpec, GenerationError>;
}

/// Validates OpenAPI specifications
pub trait OpenApiValidator: Send + Sync {
    /// Validate an OpenAPI spec
    fn validate(&self, spec: &crate::generation::OpenApiSpec) -> Result<(), GenerationError>;
}

/// Transforms OpenAPI operations for specific languages
pub trait OperationTransformer: Send + Sync {
    /// Transform an operation for a specific language
    fn transform(
        &self,
        operation: &crate::generation::Operation,
        language: crate::generation::Language,
    ) -> Result<TransformedOperation, GenerationError>;
}

/// A transformed operation ready for template rendering
#[derive(Debug, Clone)]
pub struct TransformedOperation {
    pub name: String,
    pub method: String,
    pub path: String,
    pub parameters: Vec<TransformedParameter>,
    pub return_type: String,
    pub throws: Vec<String>,
}

/// A transformed parameter
#[derive(Debug, Clone)]
pub struct TransformedParameter {
    pub name: String,
    pub type_name: String,
    pub is_required: bool,
    pub default_value: Option<String>,
}
