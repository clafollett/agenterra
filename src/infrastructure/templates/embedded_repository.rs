//! Embedded template repository implementation

use crate::infrastructure::templates::{
    RawTemplateFile, Template, TemplateDescriptor, TemplateDiscovery, TemplateError,
    TemplateExporter, TemplateFile, TemplateFileType, TemplateMetadata,
    TemplateRepository, TemplateSource, TemplateType,
};
use async_trait::async_trait;
use rust_embed::RustEmbed;
use std::io;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

// Use the common manifest parsing
use super::manifest::parse_manifest_yaml;

/// Container for all templates embedded at compile time
#[derive(RustEmbed)]
#[folder = "templates/"]
struct EmbeddedTemplates;

/// Template repository backed by embedded templates
pub struct EmbeddedTemplateRepository;

impl EmbeddedTemplateRepository {
    pub fn new() -> Self {
        Self
    }

    /// Parse a template path to extract its protocol, type, and kind
    fn parse_template_path(path: &str) -> Option<(String, TemplateType, String)> {
        let parts: Vec<&str> = path.split('/').collect();

        // Expected format: protocol/role/kind/...
        if parts.len() < 3 {
            return None;
        }

        let protocol = parts[0].to_string();
        let role = parts[1];
        let kind = parts[2].to_string();

        // TODO: Use a TemplateType from_str method implemented using the FromStr trait
        let template_type = match role {
            "server" => TemplateType::Server,
            "client" => TemplateType::Client,
            _ => return None,
        };

        Some((protocol, template_type, kind))
    }
}

impl Default for EmbeddedTemplateRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRepository for EmbeddedTemplateRepository {
    fn list_templates(&self) -> Vec<TemplateMetadata> {
        let mut templates = Vec::new();
        let mut seen_templates = std::collections::HashSet::new();

        // Iterate through all embedded files
        for file_path in EmbeddedTemplates::iter() {
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
                    let description = EmbeddedTemplates::get(&manifest_path).and_then(|data| {
                        let content = std::str::from_utf8(data.data.as_ref()).ok()?;
                        let manifest: serde_yaml::Value = serde_yaml::from_str(content).ok()?;
                        manifest
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(|s| s.to_string())
                    });

                    templates.push(TemplateMetadata {
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

    fn get_template(&self, path: &str) -> Option<TemplateMetadata> {
        // Verify the template exists by checking for a manifest
        let manifest_paths = [
            format!("{path}/manifest.yml"),
            format!("{path}/manifest.toml"),
        ];

        let manifest_exists = manifest_paths
            .iter()
            .any(|p| EmbeddedTemplates::get(p).is_some());

        if !manifest_exists {
            return None;
        }

        let (protocol, template_type, kind) = Self::parse_template_path(path)?;

        // Load description from manifest
        let description = manifest_paths
            .iter()
            .find_map(|manifest_path| {
                EmbeddedTemplates::get(manifest_path).map(|data| (manifest_path, data))
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

        Some(TemplateMetadata {
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

    fn get_template_files(&self, template_path: &str) -> Vec<RawTemplateFile> {
        let mut files = Vec::new();
        let prefix = format!("{template_path}/");

        for file_path in EmbeddedTemplates::iter() {
            let path_str = file_path.as_ref();

            if path_str.starts_with(&prefix) {
                if let Some(embedded_file) = EmbeddedTemplates::get(path_str) {
                    // Get relative path within the template
                    let relative_path = path_str[prefix.len()..].to_string();

                    files.push(RawTemplateFile {
                        relative_path,
                        contents: embedded_file.data.to_vec(),
                    });
                }
            }
        }

        files
    }
}

#[async_trait]
impl TemplateDiscovery for EmbeddedTemplateRepository {
    async fn discover(&self, descriptor: &TemplateDescriptor) -> Result<Template, TemplateError> {
        // Build the template path
        let template_path = descriptor.path();

        // Check if template exists
        if self.get_template(&template_path).is_none() {
            return Err(TemplateError::not_found(&template_path));
        }

        // Get template files
        let template_files = self.get_template_files(&template_path);

        // Find and parse the manifest file
        let manifest_file = template_files
            .iter()
            .find(|f| f.relative_path == "manifest.yml" || f.relative_path == "manifest.yaml");

        let manifest = match manifest_file {
            Some(file) => {
                // Parse the actual manifest YAML
                let content = String::from_utf8_lossy(&file.contents);
                parse_manifest_yaml(&content)?
            }
            None => {
                return Err(TemplateError::InvalidManifest(format!(
                    "No manifest.yml or manifest.yaml found for template '{}'",
                    template_path
                )));
            }
        };

        // Convert template files to our domain model, excluding the manifest
        let files: Vec<TemplateFile> = template_files
            .into_iter()
            .filter(|file| {
                file.relative_path != "manifest.yml" && file.relative_path != "manifest.yaml"
            })
            .map(|file| {
                // Find the corresponding manifest entry to determine file type
                let manifest_entry = manifest
                    .files
                    .iter()
                    .find(|mf| mf.source == file.relative_path);

                let file_type = match manifest_entry {
                    Some(mf) => mf.file_type.clone(),
                    None => {
                        // Fallback logic for files not in manifest
                        if file.relative_path.ends_with(".tera") {
                            TemplateFileType::Template { for_each: None }
                        } else {
                            TemplateFileType::Static
                        }
                    }
                };

                TemplateFile {
                    path: PathBuf::from(&file.relative_path),
                    content: String::from_utf8_lossy(&file.contents).to_string(),
                    file_type,
                }
            })
            .collect();

        Ok(Template {
            descriptor: descriptor.clone(),
            manifest,
            files,
            source: TemplateSource::Embedded,
        })
    }
}

/// Template exporter for embedded templates
pub struct EmbeddedTemplateExporter {
    repository: EmbeddedTemplateRepository,
}

impl EmbeddedTemplateExporter {
    pub fn new() -> Self {
        Self {
            repository: EmbeddedTemplateRepository::new(),
        }
    }
}

impl Default for EmbeddedTemplateExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateExporter for EmbeddedTemplateExporter {
    fn export_template(&self, template: &TemplateMetadata, output_dir: &Path) -> io::Result<()> {
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
