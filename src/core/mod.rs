//! Agenterra Core Library
//!
//! This library provides the core functionality for generating AI agent
//! server code from OpenAPI specifications.

pub mod config;
pub mod error;
pub mod openapi;
pub mod utils;

pub use error::Error;
