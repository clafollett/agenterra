//! Agenterra MCP Server and Client Generation
//!
//! This library provides functionality for generating MCP (Model Context Protocol)
//! servers and clients from OpenAPI specifications.

pub mod builders;
#[cfg(feature = "mcp_client")]
pub mod client;
