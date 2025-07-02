//! Embedded template management for binary distribution
//!
//! This module provides access to templates embedded in the binary at compile time,
//! allowing Agenterra to work immediately after `cargo install` without requiring
//! separate template files.

use rust_embed::RustEmbed;
use std::io;
use std::path::Path;
use tracing::{debug, info};

/// Domain service for accessing embedded templates
#[allow(dead_code)]
pub trait EmbeddedTemplateRepository {
    /// List all available embedded templates
    fn list_templates(&self) -> Vec<EmbeddedTemplate>;

    /// Get a specific template by path
    fn get_template(&self, path: &str) -> Option<EmbeddedTemplate>;

    /// Check if a template exists
    fn has_template(&self, path: &str) -> bool;

    /// Get all files for a template directory
    fn get_template_files(&self, template_path: &str) -> Vec<EmbeddedTemplateFile>;
}

/// Value object representing an embedded template
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedTemplate {
    /// Path relative to templates/ directory
    pub path: String,
    /// Template type (server/client)
    pub template_type: TemplateType,
    /// Template kind (e.g., rust_axum, rust_reqwest)
    pub kind: String,
    /// Protocol (e.g., mcp)
    pub protocol: String,
    /// Template description from manifest
    pub description: Option<String>,
}

/// Template type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateType {
    Server,
    Client,
}

/// Represents a single file within an embedded template
#[derive(Debug, Clone)]
pub struct EmbeddedTemplateFile {
    /// Relative path within the template
    pub relative_path: String,
    /// File contents
    pub contents: Vec<u8>,
}

/// Domain service for exporting templates to filesystem
#[allow(dead_code)]
pub trait TemplateExporter {
    /// Export a single template to the specified directory
    fn export_template(&self, template: &EmbeddedTemplate, output_dir: &Path) -> io::Result<()>;

    /// Export all templates to the specified directory
    fn export_all_templates(&self, output_dir: &Path) -> io::Result<usize>;
}

/// Embedded templates from the templates/ directory
#[derive(RustEmbed)]
#[folder = "templates/"]
#[include = "**/*.tera"]
#[include = "**/manifest.yml"]
#[include = "**/manifest.toml"]
#[include = "**/Cargo.toml.tera"]
#[include = "**/*.rs.tera"]
#[include = "**/*.md.tera"]
#[include = "**/*.gitignore.tera"]
#[include = "**/*.json.tera"]
#[include = "**/*.yaml.tera"]
#[include = "**/*.yml.tera"]
pub struct Templates;

/// Production implementation of embedded template repository
pub struct RustEmbedTemplateRepository;

impl RustEmbedTemplateRepository {
    /// Create a new embedded template repository
    pub fn new() -> Self {
        Self
    }

    /// Parse template path to extract metadata
    fn parse_template_path(path: &str) -> Option<(String, TemplateType, String)> {
        let parts: Vec<&str> = path.split('/').collect();

        // Expected format: protocol/role/kind/...
        if parts.len() < 3 {
            return None;
        }

        let protocol = parts[0].to_string();
        let role = parts[1];
        let kind = parts[2].to_string();

        let template_type = match role {
            "server" => TemplateType::Server,
            "client" => TemplateType::Client,
            _ => return None,
        };

        Some((protocol, template_type, kind))
    }
}

impl Default for RustEmbedTemplateRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedTemplateRepository for RustEmbedTemplateRepository {
    fn list_templates(&self) -> Vec<EmbeddedTemplate> {
        let mut templates = Vec::new();
        let mut seen_templates = std::collections::HashSet::new();

        // Iterate through all embedded files
        for file_path in Templates::iter() {
            let path_str = file_path.as_ref();

            // Extract template directory (protocol/role/kind)
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() >= 3 {
                let template_path = format!("{}/{}/{}", parts[0], parts[1], parts[2]);

                // Skip if we've already processed this template
                if seen_templates.contains(&template_path) {
                    continue;
                }

                if let Some((protocol, template_type, kind)) =
                    Self::parse_template_path(&template_path)
                {
                    // Try to load manifest for description
                    let manifest_path = format!("{template_path}/manifest.yml");
                    let description = Templates::get(&manifest_path).and_then(|data| {
                        let content = std::str::from_utf8(data.data.as_ref()).ok()?;
                        let manifest: serde_yaml::Value = serde_yaml::from_str(content).ok()?;
                        manifest
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(|s| s.to_string())
                    });

                    templates.push(EmbeddedTemplate {
                        path: template_path.clone(),
                        template_type,
                        kind,
                        protocol,
                        description,
                    });

                    seen_templates.insert(template_path);
                }
            }
        }

        // Sort by path for consistent ordering
        templates.sort_by(|a, b| a.path.cmp(&b.path));
        templates
    }

    fn get_template(&self, path: &str) -> Option<EmbeddedTemplate> {
        // Verify the template exists by checking for a manifest
        let manifest_paths = [
            format!("{path}/manifest.yml"),
            format!("{path}/manifest.toml"),
        ];

        let manifest_exists = manifest_paths.iter().any(|p| Templates::get(p).is_some());

        if !manifest_exists {
            return None;
        }

        let (protocol, template_type, kind) = Self::parse_template_path(path)?;

        // Load description from manifest
        let description = manifest_paths
            .iter()
            .find_map(|manifest_path| {
                Templates::get(manifest_path).map(|data| (manifest_path, data))
            })
            .and_then(|(manifest_path, data)| {
                let content = std::str::from_utf8(data.data.as_ref()).ok()?;
                if manifest_path.ends_with(".yml") {
                    let manifest: serde_yaml::Value = serde_yaml::from_str(content).ok()?;
                    manifest
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string())
                } else {
                    let manifest: toml::Value = toml::from_str(content).ok()?;
                    manifest
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string())
                }
            });

        Some(EmbeddedTemplate {
            path: path.to_string(),
            template_type,
            kind,
            protocol,
            description,
        })
    }

    fn has_template(&self, path: &str) -> bool {
        self.get_template(path).is_some()
    }

    fn get_template_files(&self, template_path: &str) -> Vec<EmbeddedTemplateFile> {
        let mut files = Vec::new();
        let prefix = format!("{template_path}/");

        for file_path in Templates::iter() {
            let path_str = file_path.as_ref();

            if path_str.starts_with(&prefix) {
                if let Some(embedded_file) = Templates::get(path_str) {
                    // Get relative path within the template
                    let relative_path = path_str[prefix.len()..].to_string();

                    files.push(EmbeddedTemplateFile {
                        relative_path,
                        contents: embedded_file.data.to_vec(),
                    });
                }
            }
        }

        files
    }
}

/// Production implementation of template exporter
pub struct RustEmbedTemplateExporter {
    repository: RustEmbedTemplateRepository,
}

impl RustEmbedTemplateExporter {
    /// Create a new template exporter
    pub fn new() -> Self {
        Self {
            repository: RustEmbedTemplateRepository::new(),
        }
    }
}

impl Default for RustEmbedTemplateExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateExporter for RustEmbedTemplateExporter {
    fn export_template(&self, template: &EmbeddedTemplate, output_dir: &Path) -> io::Result<()> {
        let template_output_dir = output_dir.join(&template.path);

        info!(
            template = %template.path,
            output_dir = %template_output_dir.display(),
            "Exporting template"
        );

        // Create the base directory
        std::fs::create_dir_all(&template_output_dir)?;

        // Get all files for this template
        let files = self.repository.get_template_files(&template.path);
        let file_count = files.len();

        for file in files {
            let file_path = template_output_dir.join(&file.relative_path);

            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Write the file
            std::fs::write(&file_path, &file.contents)?;

            debug!(
                file = %file_path.display(),
                "Exported template file"
            );
        }

        info!(
            template = %template.path,
            file_count = file_count,
            "Template export completed"
        );

        Ok(())
    }

    fn export_all_templates(&self, output_dir: &Path) -> io::Result<usize> {
        let templates = self.repository.list_templates();
        let count = templates.len();

        info!(
            output_dir = %output_dir.display(),
            template_count = count,
            "Exporting all templates"
        );

        for template in &templates {
            self.export_template(template, output_dir)?;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Mock implementation for testing
    struct MockEmbeddedTemplateRepository {
        templates: Vec<EmbeddedTemplate>,
    }

    impl MockEmbeddedTemplateRepository {
        fn new() -> Self {
            Self {
                templates: vec![
                    EmbeddedTemplate {
                        path: "mcp/server/rust_axum".to_string(),
                        template_type: TemplateType::Server,
                        kind: "rust_axum".to_string(),
                        protocol: "mcp".to_string(),
                        description: Some("Rust MCP server using Axum framework".to_string()),
                    },
                    EmbeddedTemplate {
                        path: "mcp/client/rust_reqwest".to_string(),
                        template_type: TemplateType::Client,
                        kind: "rust_reqwest".to_string(),
                        protocol: "mcp".to_string(),
                        description: Some("Rust MCP client with REPL interface".to_string()),
                    },
                ],
            }
        }
    }

    impl EmbeddedTemplateRepository for MockEmbeddedTemplateRepository {
        fn list_templates(&self) -> Vec<EmbeddedTemplate> {
            self.templates.clone()
        }

        fn get_template(&self, path: &str) -> Option<EmbeddedTemplate> {
            self.templates.iter().find(|t| t.path == path).cloned()
        }

        fn has_template(&self, path: &str) -> bool {
            self.templates.iter().any(|t| t.path == path)
        }

        fn get_template_files(&self, _template_path: &str) -> Vec<EmbeddedTemplateFile> {
            vec![]
        }
    }

    #[test]
    fn test_list_embedded_templates() {
        let repo = MockEmbeddedTemplateRepository::new();
        let templates = repo.list_templates();

        assert_eq!(templates.len(), 2);
        assert!(templates.iter().any(|t| t.kind == "rust_axum"));
        assert!(templates.iter().any(|t| t.kind == "rust_reqwest"));
    }

    #[test]
    fn test_get_template_by_path() {
        let repo = MockEmbeddedTemplateRepository::new();

        let template = repo.get_template("mcp/server/rust_axum");
        assert!(template.is_some());

        let template = template.unwrap();
        assert_eq!(template.kind, "rust_axum");
        assert_eq!(template.template_type, TemplateType::Server);
    }

    #[test]
    fn test_has_template() {
        let repo = MockEmbeddedTemplateRepository::new();

        assert!(repo.has_template("mcp/server/rust_axum"));
        assert!(repo.has_template("mcp/client/rust_reqwest"));
        assert!(!repo.has_template("nonexistent/template"));
    }

    #[test]
    fn test_template_not_found() {
        let repo = MockEmbeddedTemplateRepository::new();

        let template = repo.get_template("invalid/path");
        assert!(template.is_none());
    }

    #[test]
    fn test_real_embedded_templates_available() {
        let repo = RustEmbedTemplateRepository::new();
        let templates = repo.list_templates();

        // We should have at least the Rust templates
        assert!(!templates.is_empty());
        assert!(templates.iter().any(|t| t.kind == "rust_axum"));
        assert!(templates.iter().any(|t| t.kind == "rust_reqwest"));
    }

    #[test]
    fn test_get_specific_template() {
        let repo = RustEmbedTemplateRepository::new();

        let template = repo.get_template("mcp/server/rust_axum");
        assert!(template.is_some());

        let template = template.unwrap();
        assert_eq!(template.protocol, "mcp");
        assert_eq!(template.template_type, TemplateType::Server);
        assert_eq!(template.kind, "rust_axum");
    }

    #[test]
    fn test_get_template_files() {
        let repo = RustEmbedTemplateRepository::new();

        let files = repo.get_template_files("mcp/server/rust_axum");
        assert!(!files.is_empty());

        // Should have at least manifest and Cargo.toml
        assert!(files.iter().any(|f| f.relative_path == "manifest.yml"));
        assert!(files.iter().any(|f| f.relative_path == "Cargo.toml.tera"));
    }

    #[test]
    fn test_export_template() {
        let temp_dir = TempDir::new().unwrap();
        let exporter = RustEmbedTemplateExporter::new();
        let repo = RustEmbedTemplateRepository::new();

        let template = repo.get_template("mcp/server/rust_axum").unwrap();
        let result = exporter.export_template(&template, temp_dir.path());
        assert!(result.is_ok());

        // Verify files were exported
        let exported_dir = temp_dir.path().join("mcp/server/rust_axum");
        assert!(exported_dir.exists());
        assert!(exported_dir.join("manifest.yml").exists());
        assert!(exported_dir.join("Cargo.toml.tera").exists());
    }

    #[test]
    fn test_export_all_templates() {
        let temp_dir = TempDir::new().unwrap();
        let exporter = RustEmbedTemplateExporter::new();

        let result = exporter.export_all_templates(temp_dir.path());
        assert!(result.is_ok());

        let count = result.unwrap();
        assert!(count >= 2); // At least server and client

        // Verify both templates were exported
        assert!(temp_dir.path().join("mcp/server/rust_axum").exists());
        assert!(temp_dir.path().join("mcp/client/rust_reqwest").exists());
    }

    // Tests for template export functionality
    mod export_tests {
        use super::*;
        use std::fs;

        struct MockTemplateExporter;

        impl TemplateExporter for MockTemplateExporter {
            fn export_template(
                &self,
                template: &EmbeddedTemplate,
                output_dir: &Path,
            ) -> io::Result<()> {
                let template_dir = output_dir.join(&template.path);
                fs::create_dir_all(&template_dir)?;

                // Create a dummy file to verify export worked
                let dummy_file = template_dir.join("exported.txt");
                fs::write(dummy_file, b"exported template")?;

                Ok(())
            }

            fn export_all_templates(&self, output_dir: &Path) -> io::Result<usize> {
                let repo = MockEmbeddedTemplateRepository::new();
                let templates = repo.list_templates();

                for template in &templates {
                    self.export_template(template, output_dir)?;
                }

                Ok(templates.len())
            }
        }

        #[test]
        fn test_export_single_template() {
            let temp_dir = TempDir::new().unwrap();
            let exporter = MockTemplateExporter;

            let template = EmbeddedTemplate {
                path: "mcp/server/rust_axum".to_string(),
                template_type: TemplateType::Server,
                kind: "rust_axum".to_string(),
                protocol: "mcp".to_string(),
                description: Some("Test template".to_string()),
            };

            let result = exporter.export_template(&template, temp_dir.path());
            assert!(result.is_ok());

            // Verify the template was exported
            let exported_path = temp_dir.path().join("mcp/server/rust_axum/exported.txt");
            assert!(exported_path.exists());
        }

        #[test]
        fn test_export_all_templates() {
            let temp_dir = TempDir::new().unwrap();
            let exporter = MockTemplateExporter;

            let result = exporter.export_all_templates(temp_dir.path());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);

            // Verify both templates were exported
            assert!(
                temp_dir
                    .path()
                    .join("mcp/server/rust_axum/exported.txt")
                    .exists()
            );
            assert!(
                temp_dir
                    .path()
                    .join("mcp/client/rust_reqwest/exported.txt")
                    .exists()
            );
        }
    }
}