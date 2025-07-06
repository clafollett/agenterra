//! MCP Server-specific template renderer

use async_trait::async_trait;
use std::path::PathBuf;
use tera::{Context as TeraContext, Tera};

use crate::generation::{
    Artifact, GenerationContext, GenerationError, RenderContext, TemplateRenderingStrategy,
    utils::to_snake_case,
};
use crate::infrastructure::templates::{Template, TemplateFileType};
use crate::protocols::{Protocol, Role};

/// MCP Server-specific template renderer
/// Handles OpenAPI operation iteration and endpoint path substitution
pub struct McpServerTemplateRenderer;

impl McpServerTemplateRenderer {
    pub fn new() -> Self {
        Self
    }
    
    /// Generate schema JSON files for each endpoint
    fn generate_schema_artifacts(
        &self,
        context: &RenderContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        let mut artifacts = Vec::new();
        
        // Get endpoints from context
        let endpoints = context
            .variables
            .get("endpoints")
            .or_else(|| context.variables.get("endpoint"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                GenerationError::RenderError(
                    "No endpoints found in context for schema generation".to_string(),
                )
            })?;
        
        // Generate one schema file per endpoint
        for endpoint in endpoints {
            let endpoint_name = endpoint
                .get("endpoint")
                .or_else(|| endpoint.get("endpoint_fs"))
                .or_else(|| endpoint.get("fn_name"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    GenerationError::RenderError(
                        "Endpoint object missing 'endpoint' field".to_string(),
                    )
                })?;
            
            // Use snake_case for the filename to match MCP conventions
            let schema_filename = to_snake_case(endpoint_name);
            let schema_path = PathBuf::from(format!("schemas/{}.json", schema_filename));
            
            // Serialize the endpoint object as the schema
            let schema_json = serde_json::to_string_pretty(endpoint)
                .map_err(|e| GenerationError::RenderError(format!(
                    "Failed to serialize schema for endpoint '{}': {}",
                    endpoint_name, e
                )))?;
            
            artifacts.push(Artifact {
                path: schema_path,
                content: schema_json,
                permissions: None,
                post_commands: vec![],
            });
        }
        
        Ok(artifacts)
    }

    /// Process a template file for each operation
    async fn process_operation_file(
        &self,
        tera: &Tera,
        template_name: &str,
        file_destination: &str,
        context: &RenderContext,
        _generation_context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        let mut artifacts = Vec::new();

        // Get endpoints from context
        let endpoints = context
            .variables
            .get("endpoints")
            .or_else(|| context.variables.get("endpoint"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                GenerationError::RenderError(
                    "No endpoints found in context for operation template".to_string(),
                )
            })?;

        // Debug: log template name
        tracing::debug!("Processing operation template: {}", template_name);
        
        // Generate one file per endpoint
        for endpoint in endpoints {
            let mut tera_context = TeraContext::new();

            // Add base context variables
            for (key, value) in &context.variables {
                tera_context.insert(key, value);
            }

            // Extract endpoint name for path substitution
            let endpoint_name = endpoint
                .get("endpoint")
                .or_else(|| endpoint.get("endpoint_fs"))
                .or_else(|| endpoint.get("fn_name"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    GenerationError::RenderError(
                        "Endpoint object missing 'endpoint' field".to_string(),
                    )
                })?;

            // Add endpoint fields to context at top level for template access
            if let Some(obj) = endpoint.as_object() {
                for (key, value) in obj {
                    tera_context.insert(key, value);
                }
                // Debug logging
                tracing::debug!(
                    "Endpoint context for '{}': properties count = {}, parameters count = {}",
                    endpoint_name,
                    obj.get("properties")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0),
                    obj.get("parameters")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0)
                );
                
                // Additional debug: check specific fields that template expects
                tracing::debug!(
                    "Endpoint '{}' has response_type: {}, response_is_array: {}",
                    endpoint_name,
                    obj.get("response_type").and_then(|v| v.as_str()).unwrap_or("missing"),
                    obj.get("response_is_array").and_then(|v| v.as_bool()).unwrap_or(false)
                );
            }

            // Replace {endpoint} placeholder in destination path
            let output_path = file_destination
                .replace("{endpoint}", endpoint_name)
                .replace("{operation_id}", endpoint_name);

            // Render the template
            let rendered = tera.render(template_name, &tera_context)
                .map_err(|e| {
                    // Extract the actual error message and source
                    let error_msg = format!("{:?}", e);
                    tracing::error!(
                        "Template render error for '{template_name}' endpoint '{endpoint_name}': Full Tera error: {error_msg}", 
                    );
                    GenerationError::RenderError(format!(
                        "Failed to render template '{template_name}' for endpoint '{endpoint_name}': {}",
                        e
                    ))
                })?;

            artifacts.push(Artifact {
                path: PathBuf::from(output_path),
                content: rendered,
                permissions: None,
                post_commands: vec![],
            });
        }

        Ok(artifacts)
    }
}

#[async_trait]
impl TemplateRenderingStrategy for McpServerTemplateRenderer {
    async fn render(
        &self,
        template: &Template,
        context: &RenderContext,
        generation_context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        // Verify this is for MCP server
        if generation_context.protocol != Protocol::Mcp || generation_context.role != Role::Server {
            return Err(GenerationError::InvalidConfiguration(
                "McpServerTemplateRenderer can only be used for MCP servers".to_string(),
            ));
        }

        let mut artifacts = Vec::new();
        let mut tera = Tera::default();

        // Add template files to Tera, indexed by their manifest source names
        for manifest_file in &template.manifest.files {
            if let Some(template_file) = template
                .files
                .iter()
                .find(|f| f.path.to_string_lossy() == manifest_file.source)
            {
                if matches!(manifest_file.file_type, TemplateFileType::Template { .. }) {
                    tera.add_raw_template(&manifest_file.source, &template_file.content)
                        .map_err(|e| {
                            GenerationError::RenderError(format!(
                                "Failed to add template '{}': {}",
                                manifest_file.source, e
                            ))
                        })?;
                }
            }
        }

        // Process each manifest file entry
        for manifest_file in &template.manifest.files {
            let template_file = template
                .files
                .iter()
                .find(|f| f.path.to_string_lossy() == manifest_file.source)
                .ok_or_else(|| {
                    GenerationError::RenderError(format!(
                        "Template file for manifest entry '{}' not found",
                        manifest_file.source
                    ))
                })?;

            match &manifest_file.file_type {
                TemplateFileType::Template { for_each } => {
                    if let Some(collection_key) = for_each {
                        if collection_key == "endpoint" || collection_key == "operation" {
                            // Generate one file per endpoint
                            artifacts.extend(
                                self.process_operation_file(
                                    &tera,
                                    &manifest_file.source,
                                    &manifest_file.target,
                                    context,
                                    generation_context,
                                )
                                .await?,
                            );
                        } else {
                            return Err(GenerationError::InvalidConfiguration(format!(
                                "Unsupported for_each value: {}",
                                collection_key
                            )));
                        }
                    } else {
                        // Regular template - render once
                        let mut tera_context = TeraContext::new();
                        for (key, value) in &context.variables {
                            tera_context.insert(key, value);
                        }

                        let rendered =
                            tera.render(&manifest_file.source, &tera_context)
                                .map_err(|e| {
                                    GenerationError::RenderError(format!(
                                        "Failed to render template '{}': {}",
                                        manifest_file.source, e
                                    ))
                                })?;

                        artifacts.push(Artifact {
                            path: PathBuf::from(&manifest_file.target),
                            content: rendered,
                            permissions: None,
                            post_commands: vec![],
                        });
                    }
                }
                TemplateFileType::Static => {
                    // Copy static files as-is
                    artifacts.push(Artifact {
                        path: PathBuf::from(&manifest_file.target),
                        content: template_file.content.clone(),
                        permissions: None,
                        post_commands: vec![],
                    });
                }
                _ => {
                    // Skip other file types
                }
            }
        }
        
        // Generate schema files for MCP servers
        artifacts.extend(self.generate_schema_artifacts(context)?);

        Ok(artifacts)
    }
}
