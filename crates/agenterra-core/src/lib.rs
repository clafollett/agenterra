//! Agenterra Core Library
//!
//! This library provides the core functionality for generating AI agent
//! server code from OpenAPI specifications.

pub mod builders;
pub mod config;
pub mod error;
pub mod generate;
pub mod manifest;
pub mod openapi;
pub mod templates;
pub mod utils;

pub use crate::{
    config::Config,
    error::{Error, Result},
    generate::generate,
    openapi::OpenApiContext,
    templates::{TemplateDir, TemplateKind, TemplateManager, TemplateOptions},
};

/// Result type for Agenterra generation operations
pub type AgenterraResult<T> = std::result::Result<T, Error>;
