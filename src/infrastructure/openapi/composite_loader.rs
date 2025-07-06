//! Composite OpenAPI loader that tries multiple loading strategies

use crate::generation::{GenerationError, OpenApiLoader, OpenApiSpec};
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

    /// Add a custom loader
    pub fn add_loader(mut self, loader: Box<dyn OpenApiLoader>) -> Self {
        self.loaders.push(loader);
        self
    }
}

impl Default for CompositeOpenApiLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpenApiLoader for CompositeOpenApiLoader {
    async fn load(&self, source: &str) -> Result<OpenApiSpec, GenerationError> {
        let mut last_error = None;

        for loader in &self.loaders {
            match loader.load(source).await {
                Ok(spec) => return Ok(spec),
                Err(e) => {
                    last_error = Some(e);
                    // Try next loader
                }
            }
        }

        // If all loaders failed, return the last error
        Err(last_error.unwrap_or_else(|| {
            GenerationError::LoadError(format!("No loader could handle source: {}", source))
        }))
    }
}
