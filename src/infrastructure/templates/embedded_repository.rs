//! Embedded template repository implementation

use crate::generation::Language;
use crate::protocols::types::{Protocol, Role};

use super::{
    RawTemplateFile, Template, TemplateDiscovery, TemplateError, TemplateExporter, TemplateFile,
    TemplateFileType, TemplateManifest, TemplateRepository, TemplateSource,
};
use async_trait::async_trait;
use rust_embed::RustEmbed;
use std::io;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

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
}

impl Default for EmbeddedTemplateRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRepository for EmbeddedTemplateRepository {
    fn list_manifests(&self) -> Vec<TemplateManifest> {
        let mut manifests = Vec::new();

        // Iterate through all embedded files looking for manifest files
        for manifest_path in EmbeddedTemplates::iter().filter(|p| p.ends_with("/manifest.yml")) {
            if let Ok(Some(manifest)) = self.get_manifest(manifest_path.as_ref()) {
                manifests.push(manifest);
            }
        }

        // Sort by path for consistent ordering
        manifests.sort_by(|a, b| a.path.cmp(&b.path));

        manifests
    }

    fn get_manifest(&self, manifest_path: &str) -> Result<Option<TemplateManifest>, TemplateError> {
        let path = if manifest_path.ends_with("/manifest.yml") {
            manifest_path.to_string()
        } else {
            format!("{manifest_path}/manifest.yml")
        };

        // Load manifest content
        let file = match EmbeddedTemplates::get(&path) {
            Some(file) => file,
            None => return Ok(None),
        };

        let content = match std::str::from_utf8(file.data.as_ref()) {
            Ok(content) => content,
            Err(e) => return Err(TemplateError::manifest_parse_error(manifest_path, e)),
        };

        let manifest = TemplateManifest::from_yaml(content, manifest_path)?;

        Ok(Some(manifest))
    }

    fn has_template(&self, path: &str) -> bool {
        let manifest_path = if path.ends_with("/manifest.yml") {
            path.to_string()
        } else {
            format!("{path}/manifest.yml")
        };

        EmbeddedTemplates::get(&manifest_path).is_some()
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
    async fn discover(
        &self,
        protocol: Protocol,
        role: Role,
        language: Language,
    ) -> Result<Template, TemplateError> {
        // Build the template path from attributes
        let template_path = format!("{protocol}/{role}/{language}");

        // Check if template exists
        if !self.has_template(&template_path) {
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
                TemplateManifest::from_yaml(&content, &template_path)?
            }
            None => {
                return Err(TemplateError::InvalidManifest(format!(
                    "No manifest.yml or manifest.yaml found for template '{template_path}'"
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

        let template = Template {
            manifest,
            files,
            source: TemplateSource::Embedded,
        };

        debug!(
            template_path = %template_path,
            source = %template.source,
            file_count = template.files.len(),
            "Template discovered"
        );

        Ok(template)
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
    fn export_template(&self, template: &TemplateManifest, output_dir: &Path) -> io::Result<()> {
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
        let templates = self.repository.list_manifests();
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

    #[test]
    fn test_list_manifests() {
        let repo = EmbeddedTemplateRepository;
        let manifests = repo.list_manifests();

        // We only support MCP Client and Server at this time.
        assert!(manifests.len() == 2);
    }

    #[test]
    fn test_has_template() {
        let repo = EmbeddedTemplateRepository;

        // Test with a template that should exist
        assert!(repo.has_template("mcp/client/rust"));

        // Test with a template that should exist
        assert!(repo.has_template("mcp/server/rust"));

        // Test with a template that doesn't exist
        assert!(!repo.has_template("nonexistent/template/path"));
    }
}
