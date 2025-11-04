//! File-based OpenAPI spec loader
//!
//! This loader handles only file I/O. The actual parsing is done by the OpenApiParser.

use async_trait::async_trait;
use tokio::fs;

use super::parser::OpenApiParser;
use crate::generation::{GenerationError, OpenApiContext, OpenApiLoader};

/// Loads OpenAPI specifications from local files
pub struct FileOpenApiLoader;

impl FileOpenApiLoader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OpenApiLoader for FileOpenApiLoader {
    async fn load(&self, source: &str) -> Result<OpenApiContext, GenerationError> {
        tracing::info!("FileOpenApiLoader: Starting to load OpenAPI spec from path: {source}");

        let content = fs::read_to_string(source).await.map_err(|e| {
            tracing::error!("FileOpenApiLoader: Failed to read file '{source}': {e}");
            GenerationError::IoError(e)
        })?;
        tracing::debug!("FileOpenApiLoader: Successfully read content from {source}");


        // Parse content as JSON or YAML
        let spec_value = if source.ends_with(".json") {
            tracing::debug!("FileOpenApiLoader: Attempting to parse as JSON.");
            serde_json::from_str(&content).map_err(GenerationError::SerializationError)?
        } else if source.ends_with(".yaml") || source.ends_with(".yml") {
            tracing::debug!("FileOpenApiLoader: Attempting to parse as YAML.");
            serde_yaml::from_str(&content)
                .map_err(|e| GenerationError::LoadError(format!("Failed to parse YAML: {e}")))?
        } else {
            // Try JSON first, then YAML
            tracing::debug!("FileOpenApiLoader: File extension unknown, trying JSON then YAML.");
            serde_json::from_str(&content)
                .or_else(|_| serde_yaml::from_str(&content))
                .map_err(|e| {
                    GenerationError::LoadError(format!("Failed to parse OpenAPI spec: {e}"))
                })?
        };
        tracing::info!("FileOpenApiLoader: Successfully parsed OpenAPI spec from {source}");

        let mut parser = OpenApiParser::new(spec_value);
        tracing::debug!("FileOpenApiLoader: Calling OpenApiParser to parse the spec.");
        let context = parser.parse().await?;
        tracing::info!("FileOpenApiLoader: Successfully loaded and parsed OpenAPI spec.");
        Ok(context)
    }
}

impl Default for FileOpenApiLoader {
    fn default() -> Self {
        Self::new()
    }
}
