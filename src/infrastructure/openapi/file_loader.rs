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
        // Read file content
        let content = fs::read_to_string(source)
            .await
            .map_err(GenerationError::IoError)?;

        // Parse content as JSON or YAML
        let spec_value = if source.ends_with(".json") {
            serde_json::from_str(&content).map_err(GenerationError::SerializationError)?
        } else if source.ends_with(".yaml") || source.ends_with(".yml") {
            serde_yaml::from_str(&content)
                .map_err(|e| GenerationError::LoadError(format!("Failed to parse YAML: {e}")))?
        } else {
            // Try JSON first, then YAML
            serde_json::from_str(&content)
                .or_else(|_| serde_yaml::from_str(&content))
                .map_err(|e| {
                    GenerationError::LoadError(format!("Failed to parse OpenAPI spec: {e}"))
                })?
        };

        // Use the dedicated parser to parse the complete specification
        let parser = OpenApiParser::new(spec_value);
        parser.parse().await
    }
}

impl Default for FileOpenApiLoader {
    fn default() -> Self {
        Self::new()
    }
}
