//! Agenterra MCP Server and Client Generation
//!
//! This library provides functionality for generating MCP (Model Context Protocol)
//! servers and clients from OpenAPI specifications.
#![allow(dead_code)]

pub mod builders;
#[cfg(feature = "mcp_client")]
pub mod client;

pub mod manifest;
pub mod templates;

// Re-exports removed: obsolete generate module
pub use templates::{ClientTemplateKind, ServerTemplateKind, TemplateManager, TemplateOptions};
