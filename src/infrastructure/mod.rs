//! Infrastructure layer - concrete implementations of domain ports

pub mod generation;
pub mod openapi;
pub mod output;
pub mod shell;
pub mod templates;

pub use shell::*;
pub use templates::*;
