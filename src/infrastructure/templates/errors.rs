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
    
    /// Generic template error
    #[error("Template error: {0}")]
    Other(String),
}

impl TemplateError {
    /// Create a new template not found error with path
    pub fn not_found<S: Into<String>>(path: S) -> Self {
        Self::TemplateNotFound(path.into())
    }
    
    /// Create a new invalid manifest error
    pub fn invalid_manifest<S: Into<String>>(message: S) -> Self {
        Self::InvalidManifest(message.into())
    }
}