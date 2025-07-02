//! Template system for code generation.
//!
//! This module provides the template system used by Agenterra to generate code from OpenAPI
//! specifications. It includes template discovery, loading, rendering, and management
//! functionality for multiple target languages and frameworks.
//!
//! The template system supports:
//! - Multiple template kinds (Rust Axum, Python FastAPI, etc.)
//! - Template directory discovery and resolution
//! - Template rendering with language-specific contexts
//! - Customizable generation options and parameters
//! - Embedded templates for binary distribution

pub mod dir;
pub mod embedded;
pub mod kind;
pub mod manager;
pub mod manifest;
pub mod options;
pub mod repository;
pub mod source;
pub mod types;

pub use dir::*;
pub use embedded::*;
pub use kind::*;
pub use manager::*;
pub use manifest::*;
pub use options::*;
pub use repository::*;
pub use types::*;
