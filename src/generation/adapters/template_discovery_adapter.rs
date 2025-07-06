//! Adapter that bridges infrastructure TemplateDiscovery to generation domain

use async_trait::async_trait;
use std::sync::Arc;

use crate::generation::GenerationError;
use crate::infrastructure::templates::{
    Template, TemplateDescriptor, TemplateDiscovery as InfrastructureTemplateDiscovery,
    TemplateError,
};

/// Generation-specific trait for template discovery
/// This trait uses GenerationError which is appropriate for the generation domain
#[async_trait]
pub trait TemplateDiscovery: Send + Sync {
    /// Find a template by its descriptor
    async fn discover(&self, descriptor: &TemplateDescriptor) -> Result<Template, GenerationError>;
}

/// Adapter that converts infrastructure TemplateError to GenerationError
pub struct TemplateDiscoveryAdapter<T: InfrastructureTemplateDiscovery> {
    inner: Arc<T>,
}

impl<T: InfrastructureTemplateDiscovery> TemplateDiscoveryAdapter<T> {
    /// Create a new adapter wrapping an infrastructure template discovery
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<T: InfrastructureTemplateDiscovery> TemplateDiscovery for TemplateDiscoveryAdapter<T> {
    async fn discover(&self, descriptor: &TemplateDescriptor) -> Result<Template, GenerationError> {
        self.inner
            .discover(descriptor)
            .await
            .map_err(|e| match e {
                TemplateError::TemplateNotFound(path) => {
                    GenerationError::DiscoveryError(format!("Template not found: {}", path))
                }
                TemplateError::InvalidManifest(msg) => {
                    GenerationError::LoadError(format!("Invalid manifest: {}", msg))
                }
                TemplateError::IoError(e) => {
                    GenerationError::LoadError(format!("IO error: {}", e))
                }
                TemplateError::YamlError(e) => {
                    GenerationError::LoadError(format!("YAML error: {}", e))
                }
                TemplateError::Other(msg) => GenerationError::LoadError(msg),
            })
    }
}