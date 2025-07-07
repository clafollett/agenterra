//! Error types for the generation domain

use crate::generation::Language;
use crate::protocols::Protocol;
use thiserror::Error;

/// Errors that can occur during code generation
#[derive(Error, Debug)]
pub enum GenerationError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Invalid language: {0}")]
    InvalidLanguage(String),

    #[error("Template discovery error: {0}")]
    DiscoveryError(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("Post-processing error: {0}")]
    PostProcessingError(String),

    #[error("OpenAPI loading error: {0}")]
    LoadError(String),

    #[error("Unsupported language {language:?} for protocol {protocol:?}")]
    UnsupportedLanguageForProtocol {
        language: Language,
        protocol: Protocol,
    },

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Protocol error: {0}")]
    ProtocolError(#[from] crate::protocols::ProtocolError),
}
