//! Error types for the Agenterra MCP Client

use thiserror::Error;

/// Result type alias for client operations
pub type Result<T> = std::result::Result<T, ClientError>;

/// Errors that can occur during MCP client operations
#[derive(Error, Debug)]
pub enum ClientError {
    /// Transport-level errors (connection, I/O, etc.)
    #[error("Transport error: {0}")]
    Transport(String),

    /// Protocol-level errors (invalid messages, unknown methods, etc.)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Timeout errors for operations that exceed their deadline
    #[error("Operation timed out after {timeout_ms}ms: {operation}")]
    Timeout { operation: String, timeout_ms: u64 },

    /// Server returned an error response
    #[error("Server error: {message}")]
    Server { message: String },

    /// Client configuration or usage errors
    #[error("Client error: {0}")]
    Client(String),
}
