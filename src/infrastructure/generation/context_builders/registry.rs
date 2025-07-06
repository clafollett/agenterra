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

    /// Register a custom builder for a language
    pub fn register(&mut self, language: Language, builder: Arc<dyn ContextBuilder>) {
        self.builders.insert(language, builder);
    }

    /// Get a builder for a specific language
    pub fn get(&self, language: Language) -> Result<Arc<dyn ContextBuilder>, GenerationError> {
        self.builders
            .get(&language)
            .cloned()
            .ok_or_else(|| GenerationError::UnsupportedLanguage(language.to_string()))
    }

    /// Check if a language has a registered builder
    pub fn has_builder(&self, language: Language) -> bool {
        self.builders.contains_key(&language)
    }

    /// Get all supported languages
    pub fn supported_languages(&self) -> Vec<Language> {
        self.builders.keys().copied().collect()
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
        template: &crate::infrastructure::templates::Template,
    ) -> Result<crate::generation::RenderContext, GenerationError> {
        tracing::debug!(
            "CompositeContextBuilder selecting builder for language: {:?}, operations: {}",
            context.language,
            context.operations.len()
        );
        let builder = self.registry.get(context.language)?;
        builder.build(context, template).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::GenerationContext;
    use crate::infrastructure::templates::{
        Template, TemplateDescriptor, TemplateManifest, TemplateSource,
    };
    use crate::protocols::{Protocol, Role};

    #[test]
    fn test_registry_default_builders() {
        let registry = ContextBuilderRegistry::new();

        assert!(registry.has_builder(Language::Rust));
        assert!(registry.has_builder(Language::Python));
        assert!(registry.has_builder(Language::TypeScript));

        let languages = registry.supported_languages();
        assert!(languages.contains(&Language::Rust));
        assert!(languages.contains(&Language::Python));
        assert!(languages.contains(&Language::TypeScript));
    }

    #[tokio::test]
    async fn test_composite_builder() {
        let registry = Arc::new(ContextBuilderRegistry::new());
        let composite = CompositeContextBuilder::new(registry);

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "test".to_string();

        let template = Template {
            descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::Rust),
            manifest: TemplateManifest::default(),
            files: vec![],
            source: TemplateSource::Embedded,
        };

        let result = composite.build(&context, &template).await;
        assert!(result.is_ok());
    }
}
