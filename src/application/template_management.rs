//! Template management use cases

use crate::application::ApplicationError;
use crate::generation::GenerationError;
use crate::infrastructure::{TemplateDiscovery, TemplateExporter, TemplateRepository};
use crate::protocols::Role;
use std::path::Path;

/// Use case for listing all available templates
pub struct ListTemplatesUseCase<R: TemplateRepository> {
    repository: R,
}

impl<R: TemplateRepository> ListTemplatesUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(&self) -> String {
        let manifests = self.repository.list_manifests();

        // Group templates by role
        let mut agent_manifests = Vec::new();
        let mut broker_manifests = Vec::new();
        let mut client_manifests = Vec::new();
        let mut server_manifests = Vec::new();
        let mut custom_manifests = Vec::new();

        for manifest in manifests {
            match &manifest.role {
                Role::Agent => agent_manifests.push(manifest),
                Role::Broker => broker_manifests.push(manifest),
                Role::Client => client_manifests.push(manifest),
                Role::Server => server_manifests.push(manifest),
                Role::Custom(_) => custom_manifests.push(manifest),
            }
        }

        let mut output = String::from("Available embedded templates:\n");

        if !agent_manifests.is_empty() {
            output.push_str("\nAgent Templates:\n");
            for manifest in agent_manifests {
                output.push_str(&format!("  {}\n", manifest.path));
                if let Some(desc) = &manifest.description {
                    output.push_str(&format!("    {desc}\n"));
                }
            }
        }

        if !broker_manifests.is_empty() {
            output.push_str("\nBroker Templates:\n");
            for manifest in broker_manifests {
                output.push_str(&format!("  {}\n", manifest.path));
                if let Some(desc) = &manifest.description {
                    output.push_str(&format!("    {desc}\n"));
                }
            }
        }

        if !server_manifests.is_empty() {
            output.push_str("\nServer Templates:\n");
            for manifest in server_manifests {
                output.push_str(&format!("  {}\n", manifest.path));
                if let Some(desc) = &manifest.description {
                    output.push_str(&format!("    {desc}\n"));
                }
            }
        }

        if !client_manifests.is_empty() {
            output.push_str("\nClient Templates:\n");
            for manifest in client_manifests {
                output.push_str(&format!("  {}\n", manifest.path));
                if let Some(desc) = &manifest.description {
                    output.push_str(&format!("    {desc}\n"));
                }
            }
        }

        if !custom_manifests.is_empty() {
            output.push_str("\nCustom Templates:\n");
            for manifest in custom_manifests {
                output.push_str(&format!("  {}\n", manifest.path));
                if let Some(desc) = &manifest.description {
                    output.push_str(&format!("    {desc}\n"));
                }
            }
        }

        output.push_str("\nUse 'agenterra templates info <template>' for more details");
        output
    }
}

/// Use case for exporting templates
pub struct ExportTemplatesUseCase<E: TemplateExporter, R: TemplateRepository> {
    exporter: E,
    repository: R,
}

impl<E: TemplateExporter, R: TemplateRepository> ExportTemplatesUseCase<E, R> {
    pub fn new(exporter: E, repository: R) -> Self {
        Self {
            exporter,
            repository,
        }
    }

    pub fn execute_all(&self, output_dir: &Path) -> Result<usize, ApplicationError> {
        self.exporter
            .export_all_templates(output_dir)
            .map_err(|e| ApplicationError::GenerationError(GenerationError::IoError(e)))
    }

    pub fn execute_single(
        &self,
        template_path: &str,
        output_dir: &Path,
    ) -> Result<(), ApplicationError> {
        // Check if template exists first
        if !self.repository.has_template(template_path) {
            return Err(ApplicationError::TemplateNotFound(
                template_path.to_string(),
            ));
        }

        let manifest = match self.repository.get_manifest(template_path) {
            Ok(manifest) => manifest,
            Err(e) => return Err(ApplicationError::TemplateError(e)),
        };

        let manifest =
            manifest.ok_or(ApplicationError::InvalidTemplate(template_path.to_string()))?;

        self.exporter
            .export_template(&manifest, output_dir)
            .map_err(|e| ApplicationError::ExportError(e.to_string()))
    }
}

/// Use case for showing template information
pub struct TemplateInfoUseCase<R: TemplateRepository, D: TemplateDiscovery> {
    repository: R,
    discovery: D,
}

impl<R: TemplateRepository, D: TemplateDiscovery> TemplateInfoUseCase<R, D> {
    pub fn new(repository: R, discovery: D) -> Self {
        Self {
            repository,
            discovery,
        }
    }

    pub async fn execute(&self, template_path: &str) -> Result<String, ApplicationError> {
        // Check if template exists first
        if !self.repository.has_template(template_path) {
            return Err(ApplicationError::TemplateNotFound(
                template_path.to_string(),
            ));
        }

        // Get template manifest
        let manifest = match self.repository.get_manifest(template_path) {
            Ok(manifest) => manifest,
            Err(e) => return Err(ApplicationError::TemplateError(e)),
        };

        let manifest =
            manifest.ok_or(ApplicationError::InvalidTemplate(template_path.to_string()))?;

        let mut output = format!("Template: {}\n", manifest.path);
        output.push_str(&format!("Protocol: {}\n", manifest.protocol));
        output.push_str(&format!("Role: {:?}\n", manifest.role));
        output.push_str(&format!("Language: {}\n", manifest.language));

        // Try to load the full template to get more manifest details
        if let Ok(full_template) = self
            .discovery
            .discover(manifest.protocol, manifest.role.clone(), manifest.language)
            .await
        {
            output.push_str("\nManifest Information:\n");
            output.push_str(&format!("  Name: {}\n", full_template.manifest.name));
            output.push_str(&format!("  Version: {}\n", full_template.manifest.version));
            if let Some(description) = &full_template.manifest.description {
                output.push_str(&format!("  Description: {description}\n"));
            }

            // Display template source
            output.push_str(&format!("  Source: {}\n", full_template.source));
        } else {
            // Fallback to manifest description
            if let Some(desc) = &manifest.description {
                output.push_str(&format!("Description: {desc}\n"));
            }
        }

        // Display template files from manifest (lightweight - no content loading)
        output.push_str(&format!("\nFiles: {} total\n", manifest.files.len()));

        output.push_str("\nTemplate files:\n");
        let mut sorted_files = manifest.files.clone();
        sorted_files.sort_by(|a, b| a.source.cmp(&b.source));

        for file in &sorted_files {
            output.push_str(&format!("  - {} -> {}\n", file.source, file.target));
        }

        Ok(output)
    }
}
