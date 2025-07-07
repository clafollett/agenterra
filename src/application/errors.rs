//! Application layer error types

use thiserror::Error;

/// Application layer errors
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Export error: {0}")]
    ExportError(String),

    #[error("Generation error: {0}")]
    GenerationError(#[from] crate::generation::GenerationError),

    #[error("Invalid template: {0}")]
    InvalidTemplate(String),

    #[error("Output error: {0}")]
    OutputError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(#[from] crate::protocols::ProtocolError),

    #[error("Template error: {0}")]
    TemplateError(#[from] crate::infrastructure::TemplateError),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationError),
}

/// Validation errors for requests
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Project name cannot be empty")]
    EmptyProjectName,

    #[error("Invalid project name: {0}")]
    InvalidProjectName(String),

    #[error("Protocol {protocol} does not support role {role}")]
    UnsupportedRole {
        protocol: crate::protocols::Protocol,
        role: crate::protocols::Role,
    },

    #[error("Missing required field: {0}")]
    MissingField(String),
}
