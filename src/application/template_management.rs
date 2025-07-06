//! Template management use cases

use crate::infrastructure::templates::{
    TemplateRepository, TemplateExporter, TemplateDiscovery, TemplateDescriptor
};
use crate::application::errors::ApplicationError;
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
        let templates = self.repository.list_templates();
        
        // Group templates by role
        let mut server_templates = Vec::new();
        let mut client_templates = Vec::new();

        for template in templates {
            match template.template_type {
                crate::infrastructure::templates::TemplateType::Server => server_templates.push(template),
                crate::infrastructure::templates::TemplateType::Client => client_templates.push(template),
            }
        }

        let mut output = String::from("Available embedded templates:\n");

        if !server_templates.is_empty() {
            output.push_str("\nServer Templates:\n");
            for template in server_templates {
                output.push_str(&format!("  {}\n", template.path));
                if let Some(desc) = &template.description {
                    output.push_str(&format!("    {}\n", desc));
                }
            }
        }

        if !client_templates.is_empty() {
            output.push_str("\nClient Templates:\n");
            for template in client_templates {
                output.push_str(&format!("  {}\n", template.path));
                if let Some(desc) = &template.description {
                    output.push_str(&format!("    {}\n", desc));
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
        Self { exporter, repository }
    }

    pub fn execute_all(&self, output_dir: &Path) -> Result<usize, ApplicationError> {
        self.exporter
            .export_all_templates(output_dir)
            .map_err(ApplicationError::IoError)
    }

    pub fn execute_single(&self, template_path: &str, output_dir: &Path) -> Result<(), ApplicationError> {
        let template_metadata = self.repository
            .get_template(template_path)
            .ok_or_else(|| ApplicationError::TemplateNotFound(template_path.to_string()))?;
        
        self.exporter
            .export_template(&template_metadata, output_dir)
            .map_err(ApplicationError::IoError)
    }
}

/// Use case for showing template information
pub struct TemplateInfoUseCase<R: TemplateRepository, D: TemplateDiscovery> {
    repository: R,
    discovery: D,
}

impl<R: TemplateRepository, D: TemplateDiscovery> TemplateInfoUseCase<R, D> {
    pub fn new(repository: R, discovery: D) -> Self {
        Self { repository, discovery }
    }

    pub async fn execute(&self, template_path: &str) -> Result<String, ApplicationError> {
        // Get basic template metadata
        let template_info = self.repository.get_template(template_path)
            .ok_or_else(|| ApplicationError::TemplateNotFound(template_path.to_string()))?;

        let mut output = format!("Template: {}\n", template_info.path);
        output.push_str(&format!("Type: {:?}\n", template_info.template_type));
        output.push_str(&format!("Kind: {}\n", template_info.kind));
        output.push_str(&format!("Protocol: {}\n", template_info.protocol));

        // Try to load the full template to get manifest details
        if let Ok(descriptor) = TemplateDescriptor::from_path(&template_info.path)
            .ok_or_else(|| ApplicationError::InvalidTemplate("Invalid template path".to_string())) 
        {
            if let Ok(full_template) = self.discovery.discover(&descriptor).await {
                output.push_str("\nManifest Information:\n");
                output.push_str(&format!("  Name: {}\n", full_template.manifest.name));
                output.push_str(&format!("  Version: {}\n", full_template.manifest.version));
                if let Some(description) = &full_template.manifest.description {
                    output.push_str(&format!("  Description: {}\n", description));
                }
            } else {
                // Fallback to repository description
                if let Some(desc) = &template_info.description {
                    output.push_str(&format!("Description: {}\n", desc));
                }
            }
        }

        // Get template files
        let files = self.repository.get_template_files(&template_info.path);
        output.push_str(&format!("\nFiles: {} total\n", files.len()));

        output.push_str("\nAll files in template:\n");
        let mut sorted_files = files.clone();
        sorted_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        
        for file in &sorted_files {
            output.push_str(&format!("  - {}\n", file.relative_path));
        }

        Ok(output)
    }
}