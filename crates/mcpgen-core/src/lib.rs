//! MCPGen Core Library
//!
//! This library provides the core functionality for generating MCP (Model-Controller-Presenter)
//! server code from OpenAPI specifications.

pub mod builders;
pub mod config;
pub mod error;
pub mod generate;
pub mod manifest;
pub mod openapi;
pub mod template_dir;
pub mod template_kind;
pub mod template_manager;
pub mod template_options;
pub mod utils;

pub use crate::{
    config::Config,
    error::{Error, Result},
    generate::generate,
    openapi::OpenApiContext,
    template_dir::TemplateDir,
    template_kind::TemplateKind,
    template_manager::TemplateManager,
    template_options::TemplateOptions,
};

/// Result type for MCP generation operations
pub type MCPResult<T> = std::result::Result<T, Error>;
