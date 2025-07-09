//! Adapter that bridges infrastructure TemplateDiscovery to generation domain

use async_trait::async_trait;
use std::sync::Arc;

use crate::generation::{GenerationError, Language};
use crate::infrastructure::{
    Template, TemplateDiscovery as InfrastructureTemplateDiscovery, TemplateError,
};
use crate::protocols::{Protocol, Role};

/// Generation-specific trait for template discovery
/// This trait uses GenerationError which is appropriate for the generation domain
#[async_trait]
pub trait TemplateDiscovery: Send + Sync {
    /// Find a template by its attributes
    async fn discover(
        &self,
        protocol: Protocol,
        role: Role,
        language: Language,
    ) -> Result<Template, GenerationError>;
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
    async fn discover(
        &self,
        protocol: Protocol,
        role: Role,
        language: Language,
    ) -> Result<Template, GenerationError> {
        self.inner
            .discover(protocol, role, language)
            .await
            .map_err(|e| match e {
                TemplateError::TemplateNotFound(path) => {
                    GenerationError::DiscoveryError(format!("Template not found: {path}"))
                }
                TemplateError::InvalidManifest(msg) => {
                    GenerationError::LoadError(format!("Invalid manifest: {msg}"))
                }
                TemplateError::IoError(e) => {
                    GenerationError::LoadError(format!("Template IO error: {e}"))
                }
                TemplateError::YamlError(e) => {
                    GenerationError::LoadError(format!("YAML error: {e}"))
                }
            })
    }
}
