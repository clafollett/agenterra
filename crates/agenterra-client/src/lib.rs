//! Agenterra MCP Client
//!
//! Ergonomic wrapper around the official `rmcp` SDK for Model Context Protocol interactions.
//! Provides high-level APIs for tool discovery, invocation, resource management, and real-time communication.

pub mod client;
pub mod error;
pub mod registry;
pub mod result;
pub mod transport;

// Re-exports
pub use client::AgenterraClient;
pub use error::{ClientError, Result};
pub use registry::{ToolInfo, ToolRegistry};
pub use result::{ContentType, ToolResult};
pub use transport::Transport;
