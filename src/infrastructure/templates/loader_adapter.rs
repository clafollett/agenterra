//! Adapter to convert a TemplateLoader into a TemplateDiscovery
//!
//! This adapter is used when the user provides a --template-dir flag,
//! allowing us to load a specific template bundle while still conforming
//! to the TemplateDiscovery interface expected by the generation domain.

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use crate::infrastructure::{Template, TemplateDiscovery, TemplateError, TemplateLoader};

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
    async fn discover(
        &self,
        _protocol: crate::protocols::Protocol,
        _role: crate::protocols::Role,
        _language: crate::generation::Language,
    ) -> Result<Template, TemplateError> {
        // TODO: This is a bit of a hack. Completely ignoring all parameters just seems wrong. This needs to be rethought
        // When using --template-dir, we ignore the requested attributes and load the specific template
        // The protocol/role/language will be derived from the template's manifest
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
    async fn test_adapter_ignores_requested_attributes() {
        use crate::infrastructure::TemplateManifest;
        use std::collections::HashMap;

        let manifest = TemplateManifest {
            name: "test-template".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            path: "mcp/client/go".to_string(),
            protocol: Protocol::Mcp,
            role: Role::Client,
            language: Language::Go,
            files: vec![],
            variables: HashMap::new(),
            post_generate_hooks: vec![],
        };

        let template = Template {
            manifest: manifest.clone(),
            files: vec![],
            source: crate::infrastructure::TemplateSource::Embedded,
        };

        let loader = Arc::new(MockTemplateLoader {
            template: template.clone(),
        });

        let adapter = TemplateLoaderDiscoveryAdapter::new(loader, PathBuf::from("/some/path"));

        // Pass different attributes - they should be ignored
        let result = adapter
            .discover(Protocol::A2a, Role::Server, Language::Rust)
            .await
            .unwrap();

        // Should get the template from the loader, not based on the requested attributes
        assert_eq!(result.manifest.protocol, Protocol::Mcp);
        assert_eq!(result.manifest.role, Role::Client);
        assert_eq!(result.manifest.language, Language::Go);
    }
}
