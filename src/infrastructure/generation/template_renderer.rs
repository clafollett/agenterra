//! Tera-based template renderer implementation

use async_trait::async_trait;
use tera::Tera;

use crate::generation::{Artifact, GenerationError, RenderContext, TemplateRenderer};
use crate::infrastructure::templates::Template;

/// Tera-based template renderer
pub struct TeraTemplateRenderer;

impl TeraTemplateRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TeraTemplateRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TemplateRenderer for TeraTemplateRenderer {
    async fn render(
        &self,
        template: &Template,
        context: &RenderContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        let mut artifacts = Vec::new();

        // Create a new Tera instance for this template
        let mut tera = Tera::default();

        // Add all template files to Tera
        for file in &template.files {
            let template_name = file.path.to_string_lossy().to_string();
            tera.add_raw_template(&template_name, &file.content)
                .map_err(|e| {
                    GenerationError::RenderError(format!("Failed to add template: {}", e))
                })?;
        }

        // Render each template file
        for file in &template.files {
            let template_name = file.path.to_string_lossy().to_string();

            match &file.file_type {
                crate::infrastructure::templates::TemplateFileType::Template { for_each } => {
                    if let Some(collection_key) = for_each {
                        // Render once per item in collection
                        artifacts.extend(
                            self.render_for_each(
                                &tera,
                                &template_name,
                                &file.path,
                                context,
                                collection_key,
                            )
                            .await?,
                        );
                    } else {
                        // Render once
                        artifacts.push(
                            self.render_single(&tera, &template_name, &file.path, context)
                                .await?,
                        );
                    }
                }
                crate::infrastructure::templates::TemplateFileType::Static => {
                    // Copy as-is
                    artifacts.push(Artifact {
                        path: file.path.clone(),
                        content: file.content.clone(),
                        permissions: None,
                        post_commands: vec![],
                    });
                }
                crate::infrastructure::templates::TemplateFileType::Configuration => {
                    // Configuration files are copied as-is for now
                    artifacts.push(Artifact {
                        path: file.path.clone(),
                        content: file.content.clone(),
                        permissions: None,
                        post_commands: vec![],
                    });
                }
            }
        }

        Ok(artifacts)
    }
}

impl TeraTemplateRenderer {
    async fn render_single(
        &self,
        tera: &Tera,
        template_name: &str,
        original_path: &std::path::Path,
        context: &RenderContext,
    ) -> Result<Artifact, GenerationError> {
        let tera_context = context.to_tera_context();

        let rendered = tera.render(template_name, &tera_context).map_err(|e| {
            GenerationError::RenderError(format!("Failed to render template: {}", e))
        })?;

        // Render the path as well (it might contain variables)
        let path_str = original_path.to_string_lossy();
        let rendered_path = if path_str.contains("{{") || path_str.contains("{") {
            // Handle both Tera-style {{var}} and single-brace {var} replacements
            let mut processed_path = path_str.to_string();
            
            // First, handle single-brace replacements like {endpoint}
            // This is a simple implementation for common cases
            if processed_path.contains("{endpoint}") {
                if let Some(endpoint_value) = tera_context.get("endpoint") {
                    if let Some(str_value) = endpoint_value.as_str() {
                        processed_path = processed_path.replace("{endpoint}", str_value);
                    }
                }
            }
            
            // Handle other common single-brace replacements
            if processed_path.contains("{operation_id}") {
                if let Some(op_value) = tera_context.get("operation_id") {
                    if let Some(str_value) = op_value.as_str() {
                        processed_path = processed_path.replace("{operation_id}", str_value);
                    }
                }
            }
            
            // Then handle Tera-style double-brace replacements if any remain
            if processed_path.contains("{{") {
                let mut path_tera = Tera::default();
                path_tera.add_raw_template("path", &processed_path).map_err(|e| {
                    GenerationError::RenderError(format!("Failed to add path template: {}", e))
                })?;
                path_tera.render("path", &tera_context).map_err(|e| {
                    GenerationError::RenderError(format!("Failed to render path: {}", e))
                })?
            } else {
                processed_path
            }
        } else {
            path_str.to_string()
        };

        Ok(Artifact {
            path: std::path::PathBuf::from(rendered_path),
            content: rendered,
            permissions: None,
            post_commands: vec![],
        })
    }

    async fn render_for_each(
        &self,
        tera: &Tera,
        template_name: &str,
        original_path: &std::path::Path,
        context: &RenderContext,
        collection_key: &str,
    ) -> Result<Vec<Artifact>, GenerationError> {
        let mut artifacts = Vec::new();

        // Get the collection from context
        let collection = context.variables.get(collection_key).ok_or_else(|| {
            GenerationError::RenderError(format!(
                "Collection '{}' not found in context",
                collection_key
            ))
        })?;

        // Ensure it's an array
        let items = collection.as_array().ok_or_else(|| {
            GenerationError::RenderError(format!("Collection '{}' is not an array", collection_key))
        })?;

        // Handle empty collections gracefully
        if items.is_empty() {
            tracing::debug!("Collection '{}' is empty, no files will be generated", collection_key);
            return Ok(artifacts);
        }

        // Render once per item
        for (index, item) in items.iter().enumerate() {
            let mut item_context = context.clone();
            item_context.add_variable("item", item.clone());
            item_context.add_variable("index", serde_json::json!(index));

            // For endpoint collections, extract the endpoint name for path substitution
            if collection_key == "endpoint" || collection_key == "endpoints" {
                if let Some(endpoint_obj) = item.as_object() {
                    // Try multiple possible field names for the endpoint identifier
                    let endpoint_name = endpoint_obj.get("endpoint")
                        .or_else(|| endpoint_obj.get("endpoint_fs"))
                        .or_else(|| endpoint_obj.get("fn_name"))
                        .or_else(|| endpoint_obj.get("operation_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    
                    item_context.add_variable("endpoint", serde_json::json!(endpoint_name));
                }
            }

            let artifact = self
                .render_single(tera, template_name, original_path, &item_context)
                .await?;
            artifacts.push(artifact);
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::Language;
    use crate::infrastructure::templates::{
        TemplateDescriptor, TemplateFile, TemplateFileType, TemplateManifest, TemplateSource,
    };
    use crate::protocols::{Protocol, Role};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_tera_renderer_static_file() {
        let renderer = TeraTemplateRenderer::new();

        let template = Template {
            descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::Rust),
            manifest: TemplateManifest::default(),
            files: vec![TemplateFile {
                path: PathBuf::from("README.md"),
                content: "# Static Content".to_string(),
                file_type: TemplateFileType::Static,
            }],
            source: TemplateSource::Embedded,
        };

        let context = RenderContext::new();
        let artifacts = renderer.render(&template, &context).await.unwrap();

        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].content, "# Static Content");
    }

    #[tokio::test]
    async fn test_tera_renderer_template_file() {
        let renderer = TeraTemplateRenderer::new();

        let template = Template {
            descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::Rust),
            manifest: TemplateManifest::default(),
            files: vec![TemplateFile {
                path: PathBuf::from("Cargo.toml"),
                content: "[package]\nname = \"{{ project_name }}\"".to_string(),
                file_type: TemplateFileType::Template { for_each: None },
            }],
            source: TemplateSource::Embedded,
        };

        let mut context = RenderContext::new();
        context.add_variable("project_name", serde_json::json!("test-project"));

        let artifacts = renderer.render(&template, &context).await.unwrap();

        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].content, "[package]\nname = \"test-project\"");
    }
}
