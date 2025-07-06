//! Output service implementations

pub mod filesystem_output;

pub use filesystem_output::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::OutputService;
    use crate::generation::Artifact;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_filesystem_output_write_artifacts() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let output_service = FileSystemOutputService::new();

        let artifacts = vec![
            Artifact {
                path: temp_dir.path().join("src/main.rs"),
                content: "fn main() { println!(\"Hello\"); }".to_string(),
                permissions: None,
                post_commands: vec![],
            },
            Artifact {
                path: temp_dir.path().join("Cargo.toml"),
                content: "[package]\nname = \"test\"\nversion = \"0.1.0\"".to_string(),
                permissions: None,
                post_commands: vec![],
            },
        ];

        // Write artifacts
        let result = output_service.write_artifacts(&artifacts).await;
        assert!(result.is_ok());

        // Verify files were created
        assert!(temp_dir.path().join("src/main.rs").exists());
        assert!(temp_dir.path().join("Cargo.toml").exists());

        // Verify content
        let main_content = std::fs::read_to_string(temp_dir.path().join("src/main.rs"))
            .expect("Failed to read main.rs");
        assert_eq!(main_content, "fn main() { println!(\"Hello\"); }");
    }

    #[tokio::test]
    async fn test_filesystem_output_ensure_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let output_service = FileSystemOutputService::new();

        let nested_path = temp_dir.path().join("deeply/nested/directory");

        // Ensure directory
        let result = output_service.ensure_directory(&nested_path).await;
        assert!(result.is_ok());

        // Verify directory was created
        assert!(nested_path.exists());
        assert!(nested_path.is_dir());
    }

    #[tokio::test]
    async fn test_filesystem_output_with_permissions() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let output_service = FileSystemOutputService::new();

        let artifact = Artifact {
            path: temp_dir.path().join("script.sh"),
            content: "#!/bin/bash\necho 'Hello'".to_string(),
            permissions: Some(0o755),
            post_commands: vec![],
        };

        // Write artifact
        let result = output_service.write_artifacts(&[artifact]).await;
        assert!(result.is_ok());

        // Verify file was created with correct permissions (on Unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(temp_dir.path().join("script.sh"))
                .expect("Failed to get metadata");
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
    }
}
