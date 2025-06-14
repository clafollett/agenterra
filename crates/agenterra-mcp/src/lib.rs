//! Agenterra MCP Server and Client Generation
//!
//! This library provides functionality for generating MCP (Model Context Protocol)
//! servers and clients from OpenAPI specifications.

pub mod builders;
pub mod generate;
pub mod manifest;
pub mod templates;

// Re-exports
pub use generate::{generate, generate_client, ClientConfig};
pub use templates::{ServerTemplateKind, ClientTemplateKind, TemplateManager, TemplateOptions};
