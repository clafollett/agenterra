//! Default template renderer for generic template rendering

use async_trait::async_trait;
use std::path::PathBuf;
use tera::{Context, Tera};

use crate::generation::{
    Artifact, GenerationContext, GenerationError, RenderContext, TemplateRenderingStrategy,
};
use crate::infrastructure::{Template, TemplateFileType};

/// Default template renderer for clients and non-OpenAPI protocols
pub struct DefaultTemplateRenderer;

impl DefaultTemplateRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultTemplateRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TemplateRenderingStrategy for DefaultTemplateRenderer {
    async fn render(
        &self,
        template: &Template,
        context: &RenderContext,
        _generation_context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        let mut artifacts = Vec::new();

        // Create a new Tera instance for this template
        let mut tera = Tera::default();

        // Add all template files to Tera
        for file in &template.files {
            let template_name = file.path.to_string_lossy().to_string();
            tera.add_raw_template(&template_name, &file.content)
                .map_err(|e| {
                    GenerationError::RenderError(format!("Failed to add template: {e}"))
                })?;
        }

        // Process each template file
        for file in &template.files {
            let template_name = file.path.to_string_lossy().to_string();

            match &file.file_type {
                TemplateFileType::Template { for_each } => {
                    if for_each.is_some() {
                        // Default renderer doesn't support for_each
                        return Err(GenerationError::InvalidConfiguration(
                            "Default renderer does not support for_each templates. Use a protocol-specific renderer.".to_string()
                        ));
                    }

                    // Single file template
                    let mut tera_context = Context::new();
                    for (key, value) in context.variables.iter() {
                        tera_context.insert(key, value);
                    }

                    let rendered = tera.render(&template_name, &tera_context).map_err(|e| {
                        GenerationError::RenderError(format!(
                            "Failed to render template '{template_name}': {e}"
                        ))
                    })?;

                    // Get destination from manifest
                    let destination = template
                        .manifest
                        .files
                        .iter()
                        .find(|f| f.source == template_name)
                        .map(|f| PathBuf::from(&f.target))
                        .unwrap_or_else(|| file.path.clone());

                    artifacts.push(Artifact {
                        path: destination,
                        content: rendered,
                        permissions: None,
                    });
                }
                TemplateFileType::Static => {
                    // Copy static files as-is
                    let destination = template
                        .manifest
                        .files
                        .iter()
                        .find(|f| f.source == template_name)
                        .map(|f| PathBuf::from(&f.target))
                        .unwrap_or_else(|| file.path.clone());

                    artifacts.push(Artifact {
                        path: destination,
                        content: file.content.clone(),
                        permissions: None,
                    });
                }
                _ => {
                    // Skip other file types
                }
            }
        }

        Ok(artifacts)
    }
}
