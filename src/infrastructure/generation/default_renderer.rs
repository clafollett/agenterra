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
        tracing::info!("DefaultTemplateRenderer: Starting template rendering for template: {}", template.source);
        let mut artifacts = Vec::new();

        let mut tera = Tera::default();
        tracing::debug!("DefaultTemplateRenderer: Initialized Tera instance.");

        // First, add all template files to the Tera instance
        for file in &template.files {
            if let TemplateFileType::Template { .. } = &file.file_type {
                let template_name = file.path.to_string_lossy().to_string();
                tera.add_raw_template(&template_name, &file.content)
                    .map_err(|e| {
                        tracing::error!("DefaultTemplateRenderer: Failed to add template '{}': {}", template_name, e);
                        GenerationError::RenderError(format!("Failed to add template: {e}"))
                    })?;
                tracing::debug!("DefaultTemplateRenderer: Added template file to Tera: {}", template_name);
            }
        }

        // Now, iterate through the files to render or copy them
        for file in &template.files {
            let template_name = file.path.to_string_lossy().to_string();
            tracing::debug!("DefaultTemplateRenderer: Processing template file: {}", template_name);

            match &file.file_type {
                TemplateFileType::Template { for_each } => {
                    if for_each.is_some() {
                        tracing::warn!("DefaultTemplateRenderer: Detected 'for_each' in template {}. This renderer does not support it.", template_name);
                        return Err(GenerationError::InvalidConfiguration(
                            "Default renderer does not support for_each templates. Use a protocol-specific renderer.".to_string()
                        ));
                    }

                    let mut tera_context = Context::new();
                    for (key, value) in context.variables.iter() {
                        tera_context.insert(key, value);
                    }
                    tracing::debug!("DefaultTemplateRenderer: Created Tera context for {}.", template_name);

                    let rendered = tera.render(&template_name, &tera_context).map_err(|e| {
                        tracing::error!("DefaultTemplateRenderer: Failed to render template '{}': {}", template_name, e);
                        GenerationError::RenderError(format!(
                            "Failed to render template '{template_name}': {e}"
                        ))
                    })?;
                    tracing::debug!("DefaultTemplateRenderer: Successfully rendered template: {}", template_name);

                    let destination = template
                        .manifest
                        .files
                        .iter()
                        .find(|f| f.source == template_name)
                        .map(|f| PathBuf::from(&f.target))
                        .unwrap_or_else(|| file.path.clone());
                    tracing::debug!("DefaultTemplateRenderer: Destination for {}: {:?}", template_name, destination);

                    artifacts.push(Artifact {
                        path: destination.clone(),
                        content: rendered,
                        permissions: None,
                    });
                    tracing::info!("DefaultTemplateRenderer: Added rendered artifact: {:?}", destination);
                }
                TemplateFileType::Static => {
                    let destination = template
                        .manifest
                        .files
                        .iter()
                        .find(|f| f.source == template_name)
                        .map(|f| PathBuf::from(&f.target))
                        .unwrap_or_else(|| file.path.clone());
                    tracing::debug!("DefaultTemplateRenderer: Destination for static file {}: {:?}", template_name, destination);

                    artifacts.push(Artifact {
                        path: destination.clone(),
                        content: file.content.clone(),
                        permissions: None,
                    });
                    tracing::info!("DefaultTemplateRenderer: Added static artifact: {:?}", destination);
                }
                _ => {
                    tracing::debug!("DefaultTemplateRenderer: Skipping unsupported file type for: {}", template_name);
                    // Skip other file types
                }
            }
        }
        tracing::info!("DefaultTemplateRenderer: Finished rendering. Total artifacts: {}", artifacts.len());
        Ok(artifacts)
    }
}
