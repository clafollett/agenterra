//! Error handling for the Agenterra code generation library.
//!
//! This module defines the main error type `Error` used throughout the library,
//! along with a convenient `Result` type alias. It uses `thiserror` for easy
//! error handling and implements conversions from common error types.
//!
//! # Examples
//!
//! ```
//! use agenterra_core::error::{Error, Result};
//!
//! fn might_fail() -> Result<()> {
//!     // Operations that might fail...
//!     Ok(())
//! }
//! ```

use thiserror::Error;

/// Result type for Agenterra generation operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for Agenterra generation operations
#[derive(Debug, Error)]
pub enum Error {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// OpenAPI error
    #[error("OpenAPI error: {0}")]
    OpenApi(String),

    /// Template error
    #[error("Template error: {0}")]
    Template(String),

    /// Template engine error
    #[error("Template engine error: {0}")]
    Tera(#[from] tera::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

#[allow(dead_code)]
impl Error {
    /// Create a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new OpenAPI error
    pub fn openapi<S: Into<String>>(msg: S) -> Self {
        Self::OpenApi(msg.into())
    }

    /// Create a new template error
    pub fn template<S: Into<String>>(msg: S) -> Self {
        Self::Template(msg.into())
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self::Config(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Config(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_config_creation() {
        let error = Error::config("Invalid configuration");
        assert!(matches!(error, Error::Config(_)));
        assert_eq!(
            error.to_string(),
            "Configuration error: Invalid configuration"
        );
    }

    #[test]
    fn test_error_openapi_creation() {
        let error = Error::openapi("Schema validation failed");
        assert!(matches!(error, Error::OpenApi(_)));
        assert_eq!(error.to_string(), "OpenAPI error: Schema validation failed");
    }

    #[test]
    fn test_error_template_creation() {
        let error = Error::template("Template not found");
        assert!(matches!(error, Error::Template(_)));
        assert_eq!(error.to_string(), "Template error: Template not found");
    }

    #[test]
    fn test_error_from_str() {
        let error: Error = "Test error message".into();
        assert!(matches!(error, Error::Config(_)));
        assert_eq!(error.to_string(), "Configuration error: Test error message");
    }

    #[test]
    fn test_error_from_string() {
        let error: Error = "Test string error".to_string().into();
        assert!(matches!(error, Error::Config(_)));
        assert_eq!(error.to_string(), "Configuration error: Test string error");
    }

    #[test]
    fn test_error_from_io_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error: Error = io_error.into();
        assert!(matches!(error, Error::Io(_)));
        assert!(error.to_string().contains("I/O error"));
        assert!(error.to_string().contains("File not found"));
    }

    #[test]
    fn test_error_from_serde_json_error() {
        let json_result: std::result::Result<serde_json::Value, _> =
            serde_json::from_str("invalid json");
        let json_error = json_result.unwrap_err();
        let error: Error = json_error.into();
        assert!(matches!(error, Error::Json(_)));
        assert!(error.to_string().contains("JSON parsing error"));
    }

    #[test]
    fn test_error_debug_display() {
        let error = Error::config("Debug test");
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("Debug test"));
    }
}
