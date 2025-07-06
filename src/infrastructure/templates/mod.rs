//! Template repository implementations

pub mod embedded_repository;
pub mod errors;
pub mod filesystem_loader;
pub mod loader_adapter;
pub mod manifest;
pub mod traits;
pub mod types;

pub use embedded_repository::*;
pub use errors::*;
pub use filesystem_loader::*;
pub use loader_adapter::*;
pub use traits::*;
pub use types::*;
