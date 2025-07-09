//! Composite OpenAPI loader that tries multiple loading strategies

use crate::generation::{GenerationError, OpenApiContext, OpenApiLoader};
use async_trait::async_trait;

/// Composite loader that tries multiple loaders in sequence
pub struct CompositeOpenApiLoader {
    loaders: Vec<Box<dyn OpenApiLoader>>,
}

impl CompositeOpenApiLoader {
    pub fn new() -> Self {
        Self {
            loaders: vec![
                Box::new(super::HttpOpenApiLoader::new()),
                Box::new(super::FileOpenApiLoader::new()),
            ],
        }
    }
}

impl Default for CompositeOpenApiLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpenApiLoader for CompositeOpenApiLoader {
    async fn load(&self, source: &str) -> Result<OpenApiContext, GenerationError> {
        tracing::debug!("CompositeOpenApiLoader: Loading from source: {source}");

        // Intelligently detect the source type and use the appropriate loader
        if source.starts_with("http://") || source.starts_with("https://") {
            // Use HTTP loader for URLs
            tracing::debug!("CompositeOpenApiLoader: Using HTTP loader");
            self.loaders[0].load(source).await
        } else {
            // Use file loader for file paths
            tracing::debug!("CompositeOpenApiLoader: Using file loader");
            self.loaders[1].load(source).await
        }
    }
}
