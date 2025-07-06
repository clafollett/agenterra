//! Filesystem-based template loader
//!
//! This implementation loads a single template bundle from a directory on the filesystem,
//! typically specified via the --template-dir CLI flag.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::infrastructure::templates::{
    Template, TemplateDescriptor, TemplateError, TemplateFile, TemplateLoader, TemplateManifest,
    TemplateSource,
};

// Use the common manifest parsing
use super::manifest::parse_manifest_yaml;

/// Template loader that loads a single template bundle from filesystem
pub struct FileSystemTemplateLoader;

impl FileSystemTemplateLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileSystemTemplateLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TemplateLoader for FileSystemTemplateLoader {
    async fn load_template(&self, path: &Path) -> Result<Template, TemplateError> {
        // Verify the directory exists
        if !path.exists() {
            return Err(TemplateError::not_found(path.to_string_lossy().as_ref()));
        }

        // Load the manifest
        let manifest = load_manifest_from_dir(path).await?;

        // Create descriptor from manifest metadata
        let descriptor = TemplateDescriptor::from_manifest(&manifest)?;

        // Load all template files
        let files = load_template_files(path, &manifest).await?;

        Ok(Template {
            descriptor,
            manifest,
            files,
            source: TemplateSource::FileSystem(path.to_path_buf()),
        })
    }
}

/// Load a template manifest from a directory
async fn load_manifest_from_dir(dir: &Path) -> Result<TemplateManifest, TemplateError> {
    let manifest_path = dir.join("manifest.yml");
    let alt_path = dir.join("manifest.yaml");

    let path = if manifest_path.exists() {
        manifest_path
    } else if alt_path.exists() {
        alt_path
    } else {
        return Err(TemplateError::InvalidManifest(format!(
            "No manifest.yml or manifest.yaml found in {}",
            dir.display()
        )));
    };

    let content = fs::read_to_string(&path)
        .await
        .map_err(|e| TemplateError::IoError(e))?;

    // Use the common manifest parser
    parse_manifest_yaml(&content)
}

/// Load all template files referenced in the manifest
async fn load_template_files(
    dir: &Path,
    manifest: &TemplateManifest,
) -> Result<Vec<TemplateFile>, TemplateError> {
    let mut files = Vec::new();

    for manifest_file in &manifest.files {
        let file_path = dir.join(&manifest_file.source);

        let content = fs::read_to_string(&file_path)
            .await
            .map_err(|e| TemplateError::IoError(e))?;

        // Keep the source filename for path to be consistent with embedded templates
        let relative_path = manifest_file.source.clone();

        files.push(TemplateFile {
            path: PathBuf::from(relative_path),
            content,
            file_type: manifest_file.file_type.clone(),
        });
    }

    Ok(files)
}

// Manifest structures have been moved to the common manifest module

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::templates::TemplateFileType;
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_test_template(dir: &Path) {
        // Create manifest
        let manifest_content = r#"
name: "test-template"
version: "1.0.0"
description: "A test template"
protocol: "mcp"
role: "server"
language: "rust"

files:
  - source: "main.rs.tera"
    destination: "src/main.rs"
  - source: "Cargo.toml.tera"
    destination: "Cargo.toml"
  - source: "lib.rs.tera"
    destination: "src/lib.rs"
    for_each: "operation"
    context:
      custom_key: "custom_value"

variables:
  default_port: 3000

hooks:
  pre_generate: "echo 'Starting generation'"
  post_generate:
    - "cargo fmt"
    - "cargo check"
"#;

        fs::write(dir.join("manifest.yml"), manifest_content)
            .await
            .unwrap();

        // Create template files
        fs::write(
            dir.join("main.rs.tera"),
            "fn main() {\n    println!(\"Hello, {{ project_name }}!\");\n}",
        )
        .await
        .unwrap();

        fs::write(
            dir.join("Cargo.toml.tera"),
            "[package]\nname = \"{{ project_name }}\"\nversion = \"0.1.0\"",
        )
        .await
        .unwrap();

        fs::write(
            dir.join("lib.rs.tera"),
            "// {{ operation.name }}\npub fn {{ operation.name }}() {}",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_discover_at_filesystem_template() {
        let temp_dir = TempDir::new().unwrap();
        create_test_template(temp_dir.path()).await;

        let loader = FileSystemTemplateLoader::new();

        let result = loader.load_template(temp_dir.path()).await;
        assert!(result.is_ok());

        let template = result.unwrap();
        assert_eq!(template.manifest.name, "test-template");
        assert_eq!(template.manifest.version, "1.0.0");
        assert_eq!(template.files.len(), 3);

        // Check file types
        let main_file = &template.files[0];
        assert!(matches!(
            main_file.file_type,
            TemplateFileType::Template { for_each: None }
        ));

        let lib_file = &template.files[2];
        assert!(matches!(
            lib_file.file_type,
            TemplateFileType::Template { for_each: Some(ref s) } if s == "operation"
        ));
    }

    #[tokio::test]
    async fn test_discover_at_missing_directory() {
        let loader = FileSystemTemplateLoader::new();

        let result = loader.load_template(Path::new("/nonexistent")).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            TemplateError::TemplateNotFound(_) => {}
            other => panic!("Expected TemplateNotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_discover_at_missing_manifest() {
        let temp_dir = TempDir::new().unwrap();
        // Create directory but no manifest

        let loader = FileSystemTemplateLoader::new();

        let result = loader.load_template(temp_dir.path()).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            TemplateError::InvalidManifest(msg) => {
                assert!(msg.contains("No manifest"));
            }
            other => panic!("Expected LoadError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_manifest_with_variables() {
        let temp_dir = TempDir::new().unwrap();
        create_test_template(temp_dir.path()).await;

        let manifest = load_manifest_from_dir(temp_dir.path()).await.unwrap();

        assert!(manifest.variables.contains_key("default_port"));
        assert_eq!(manifest.variables["default_port"], 3000);
    }
}
