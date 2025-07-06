//! Filesystem-based output service implementation

use async_trait::async_trait;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::application::{ApplicationError, OutputService};
use crate::generation::Artifact;

/// Output service that writes artifacts to the filesystem
pub struct FileSystemOutputService;

impl FileSystemOutputService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OutputService for FileSystemOutputService {
    async fn write_artifacts(&self, artifacts: &[Artifact]) -> Result<(), ApplicationError> {
        for artifact in artifacts {
            // Create parent directory if needed
            if let Some(parent) = artifact.path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    ApplicationError::OutputError(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }

            // Write file content
            let mut file = fs::File::create(&artifact.path).await.map_err(|e| {
                ApplicationError::OutputError(format!(
                    "Failed to create file {}: {}",
                    artifact.path.display(),
                    e
                ))
            })?;

            file.write_all(artifact.content.as_bytes())
                .await
                .map_err(|e| {
                    ApplicationError::OutputError(format!(
                        "Failed to write file {}: {}",
                        artifact.path.display(),
                        e
                    ))
                })?;

            file.flush().await.map_err(|e| {
                ApplicationError::OutputError(format!(
                    "Failed to flush file {}: {}",
                    artifact.path.display(),
                    e
                ))
            })?;

            // Set permissions if specified (Unix only)
            #[cfg(unix)]
            if let Some(mode) = artifact.permissions {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(mode);
                fs::set_permissions(&artifact.path, permissions)
                    .await
                    .map_err(|e| {
                        ApplicationError::OutputError(format!(
                            "Failed to set permissions on {}: {}",
                            artifact.path.display(),
                            e
                        ))
                    })?;
            }
        }

        Ok(())
    }

    async fn ensure_directory(&self, path: &Path) -> Result<(), ApplicationError> {
        fs::create_dir_all(path).await.map_err(|e| {
            ApplicationError::OutputError(format!(
                "Failed to create directory {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(())
    }
}

impl Default for FileSystemOutputService {
    fn default() -> Self {
        Self::new()
    }
}
