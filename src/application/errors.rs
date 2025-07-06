//! Application layer error types

use thiserror::Error;

/// Application layer errors
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Protocol {0} is not implemented")]
    ProtocolNotImplemented(crate::protocols::Protocol),

    #[error("Protocol error: {0}")]
    ProtocolError(#[from] crate::protocols::ProtocolError),

    #[error("Generation error: {0}")]
    GenerationError(#[from] crate::generation::GenerationError),

    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationError),

    #[error("Output error: {0}")]
    OutputError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Invalid template: {0}")]
    InvalidTemplate(String),
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

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}
