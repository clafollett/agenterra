//! Port interfaces for the application layer

use async_trait::async_trait;
use std::path::Path;

/// Service for writing generated artifacts to the output destination
#[async_trait]
pub trait OutputService: Send + Sync {
    /// Write all artifacts to the output destination
    async fn write_artifacts(
        &self,
        artifacts: &[crate::generation::Artifact],
    ) -> Result<(), crate::application::ApplicationError>;

    /// Ensure a directory exists
    async fn ensure_directory(
        &self,
        path: &Path,
    ) -> Result<(), crate::application::ApplicationError>;
}
