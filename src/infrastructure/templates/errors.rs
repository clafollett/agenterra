//! Error types for the template infrastructure layer

use thiserror::Error;

/// Errors that can occur in template operations
#[derive(Error, Debug)]
pub enum TemplateError {
    /// Template not found at the specified path
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// Invalid manifest file or format
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    /// IO error during template operations
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

impl TemplateError {
    /// Create a new template not found error with path
    pub fn not_found<S: Into<String>>(path: S) -> Self {
        Self::TemplateNotFound(path.into())
    }

    /// Helper to create consistent manifest parsing errors
    pub fn manifest_parse_error(path: &str, reason: impl std::fmt::Display) -> Self {
        Self::InvalidManifest(format!(
            "Failed to parse manifest.yml for template '{path}': {reason}"
        ))
    }
}
