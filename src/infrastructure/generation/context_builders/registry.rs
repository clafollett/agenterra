//! Registry for language-specific context builders

use std::collections::HashMap;
use std::sync::Arc;

use crate::generation::{ContextBuilder, GenerationError, Language};

/// Registry that manages language-specific context builders
pub struct ContextBuilderRegistry {
    builders: HashMap<Language, Arc<dyn ContextBuilder>>,
}

impl ContextBuilderRegistry {
    /// Create a new registry with default builders
    pub fn new() -> Self {
        let mut builders = HashMap::new();

        // Register default builders
        builders.insert(
            Language::Rust,
            Arc::new(super::RustContextBuilder::new()) as Arc<dyn ContextBuilder>,
        );
        builders.insert(
            Language::Python,
            Arc::new(super::PythonContextBuilder::new()) as Arc<dyn ContextBuilder>,
        );
        builders.insert(
            Language::TypeScript,
            Arc::new(super::TypeScriptContextBuilder::new()) as Arc<dyn ContextBuilder>,
        );

        Self { builders }
    }

    /// Get a builder for a specific language
    pub fn get(&self, language: Language) -> Result<Arc<dyn ContextBuilder>, GenerationError> {
        self.builders.get(&language).cloned().ok_or_else(|| {
            let available = Language::all()
                .iter()
                .map(|l| l.display_name())
                .collect::<Vec<_>>()
                .join(", ");
            GenerationError::UnsupportedLanguage(format!(
                "{} (available: {})",
                language.display_name(),
                available
            ))
        })
    }
}

impl Default for ContextBuilderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Composite context builder that delegates to language-specific builders
pub struct CompositeContextBuilder {
    registry: Arc<ContextBuilderRegistry>,
}

impl CompositeContextBuilder {
    pub fn new(registry: Arc<ContextBuilderRegistry>) -> Self {
        Self { registry }
    }
}

impl Default for CompositeContextBuilder {
    fn default() -> Self {
        Self::new(Arc::new(ContextBuilderRegistry::default()))
    }
}

#[async_trait::async_trait]
impl ContextBuilder for CompositeContextBuilder {
    async fn build(
        &self,
        context: &crate::generation::GenerationContext,
        template: &crate::infrastructure::Template,
    ) -> Result<crate::generation::RenderContext, GenerationError> {
        tracing::debug!(
            "CompositeContextBuilder selecting builder for language: {:?}",
            context.language
        );
        let builder = self.registry.get(context.language)?;
        builder.build(context, template).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::GenerationContext;
    use crate::infrastructure::{Template, TemplateManifest, TemplateSource};
    use crate::protocols::{Protocol, Role};
    use std::collections::HashMap;

    #[test]
    fn test_registry_default_builders() {
        let registry = ContextBuilderRegistry::new();

        // Test that default builders are registered
        assert!(registry.get(Language::Rust).is_ok());
        assert!(registry.get(Language::Python).is_ok());
        assert!(registry.get(Language::TypeScript).is_ok());
    }

    #[tokio::test]
    async fn test_composite_builder() {
        let registry = Arc::new(ContextBuilderRegistry::new());
        let composite = CompositeContextBuilder::new(registry);

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "test".to_string();

        let manifest = TemplateManifest {
            name: "test-template".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            path: "mcp/server/rust".to_string(),
            protocol: Protocol::Mcp,
            role: Role::Server,
            language: Language::Rust,
            files: vec![],
            variables: HashMap::new(),
            post_generate_hooks: vec![],
        };

        let template = Template {
            manifest,
            files: vec![],
            source: TemplateSource::Embedded,
        };

        let result = composite.build(&context, &template).await;
        assert!(result.is_ok());
    }
}
