//! Agenterra MCP Client
//!
//! Ergonomic wrapper around the official `rmcp` SDK for Model Context Protocol interactions.
//! Provides high-level APIs for tool discovery, invocation, resource management, and real-time communication.

#![allow(dead_code, unused_imports)]

pub mod auth;
pub mod cache;
pub mod client;
pub mod error;
pub mod registry;
pub mod resource;
pub mod result;
pub mod transport;

pub use auth::{AuthConfig, AuthMethod, CredentialType, SecureCredential};
pub use cache::{CacheAnalytics, CacheConfig, CachedResource, ResourceCache};
pub use client::McpClient;
pub use error::{ClientError, Result};
pub use registry::{ToolInfo, ToolRegistry};
pub use resource::{ResourceContent, ResourceInfo};
pub use result::{ContentType, ToolResult};
pub use transport::Transport;
