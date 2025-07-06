//! Adapter to convert a TemplateLoader into a TemplateDiscovery
//!
//! This adapter is used when the user provides a --template-dir flag,
//! allowing us to load a specific template bundle while still conforming
//! to the TemplateDiscovery interface expected by the generation domain.

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use crate::infrastructure::templates::{
    Template, TemplateDescriptor, TemplateDiscovery, TemplateError, TemplateLoader,
};

/// Adapter that wraps a TemplateLoader to provide TemplateDiscovery
pub struct TemplateLoaderDiscoveryAdapter {
    loader: Arc<dyn TemplateLoader>,
    template_path: PathBuf,
}

impl TemplateLoaderDiscoveryAdapter {
    /// Create a new adapter with the given loader and template path
    pub fn new(loader: Arc<dyn TemplateLoader>, template_path: PathBuf) -> Self {
        Self {
            loader,
            template_path,
        }
    }
}

#[async_trait]
impl TemplateDiscovery for TemplateLoaderDiscoveryAdapter {
    async fn discover(&self, _descriptor: &TemplateDescriptor) -> Result<Template, TemplateError> {
        // When using --template-dir, we ignore the descriptor and load the specific template
        // The descriptor will be derived from the template's manifest
        self.loader.load_template(&self.template_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::Language;
    use crate::protocols::{Protocol, Role};

    struct MockTemplateLoader {
        template: Template,
    }

    #[async_trait]
    impl TemplateLoader for MockTemplateLoader {
        async fn load_template(&self, _path: &std::path::Path) -> Result<Template, TemplateError> {
            Ok(self.template.clone())
        }
    }

    #[tokio::test]
    async fn test_adapter_ignores_descriptor() {
        let template = Template {
            descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Client, Language::Go),
            manifest: Default::default(),
            files: vec![],
            source: crate::infrastructure::templates::TemplateSource::Embedded,
        };

        let loader = Arc::new(MockTemplateLoader {
            template: template.clone(),
        });

        let adapter = TemplateLoaderDiscoveryAdapter::new(
            loader,
            PathBuf::from("/some/path"),
        );

        // Pass a different descriptor - it should be ignored
        let different_descriptor = TemplateDescriptor::new(
            Protocol::A2a,
            Role::Server,
            Language::Rust,
        );

        let result = adapter.discover(&different_descriptor).await.unwrap();
        
        // Should get the template from the loader, not based on the descriptor
        assert_eq!(result.descriptor.protocol, Protocol::Mcp);
        assert_eq!(result.descriptor.role, Role::Client);
        assert_eq!(result.descriptor.language, Language::Go);
    }
}