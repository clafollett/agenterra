//! MCP Server-specific template renderer

use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;
use tera::{Context as TeraContext, Tera, Result as TeraResult, Value as TeraValue, from_value};

use crate::generation::{
    Artifact, GenerationContext, GenerationError, RenderContext, TemplateRenderingStrategy,
    utils::to_snake_case,
};
use crate::generation::sanitizers::sanitize_rust_identifier; // Import the sanitizer
use crate::infrastructure::{Template, TemplateFileType};
use crate::protocols::{Protocol, Role};

/// MCP Server-specific template renderer
/// Handles OpenAPI operation iteration and endpoint path substitution
pub struct McpServerTemplateRenderer;

impl McpServerTemplateRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Generate schema JSON files for each endpoint (optimized version)
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

        // OPTIMIZATION: Pre-allocate vector capacity
        artifacts.reserve(endpoints.len());

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
            let schema_path = PathBuf::from(format!("schemas/{schema_filename}.json"));

            // OPTIMIZATION: Create minimal schema object - skip complex cleaning for simplified schemas
            let mut clean = serde_json::Map::new();

            // Add only essential metadata
            clean.insert("operationId".to_string(), json!(endpoint_name));

            // Add summary and description if present (single operations, no loops)
            if let Some(summary) = endpoint.get("summary").and_then(|v| v.as_str()) {
                if !summary.is_empty() {
                    clean.insert("summary".to_string(), json!(summary));
                }
            }

            if let Some(description) = endpoint.get("description").and_then(|v| v.as_str()) {
                if !description.is_empty() {
                    clean.insert("description".to_string(), json!(description));
                }
            }

            if let Some(path) = endpoint.get("path").and_then(|v| v.as_str()) {
                clean.insert("path".to_string(), json!(path));
            }

            // Add tags if present
            if let Some(tags) = endpoint.get("tags").and_then(|v| v.as_array()) {
                if !tags.is_empty() {
                    clean.insert("tags".to_string(), json!(tags));
                }
            }

            // OPTIMIZATION: Skip parameter processing for simplified schemas
            // The schemas are already simplified, so we don't need complex parameter extraction

            // OPTIMIZATION: Use minimal empty schemas instead of complex ones for faster generation
            // Add minimal request body schema
            let mut request_body = serde_json::Map::new();
            request_body.insert("schema".to_string(), json!({}));
            clean.insert("requestBody".to_string(), serde_json::Value::Object(request_body));

            // Add minimal response schema
            let mut response = serde_json::Map::new();
            response.insert("schema".to_string(), json!({}));
            clean.insert("response".to_string(), serde_json::Value::Object(response));

            let clean_schema = serde_json::Value::Object(clean);

            let schema_json = serde_json::to_string_pretty(&clean_schema).map_err(|e| {
                GenerationError::RenderError(format!(
                    "Failed to serialize schema for endpoint '{endpoint_name}': {e}"
                ))
            })?;

            artifacts.push(Artifact {
                path: schema_path,
                content: schema_json,
                permissions: None,
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
        tracing::info!("McpServerTemplateRenderer: Starting to process operation template '{}' for destination '{}'", template_name, file_destination);

        let mut artifacts = Vec::new();

        // Get endpoints from context
        let endpoints = context
            .variables
            .get("endpoints")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                GenerationError::RenderError(
                    "No 'endpoints' array found in context for operation template".to_string(),
                )
            })?;

        tracing::info!("McpServerTemplateRenderer: Found {} endpoints to process for template '{}'", endpoints.len(), template_name);

        // OPTIMIZATION: Pre-allocate artifacts vector
        artifacts.reserve(endpoints.len());

        // OPTIMIZATION: Pre-build base context once, avoiding repeated insertions
        let mut base_tera_context = TeraContext::new();
        for (key, value) in &context.variables {
            // Skip heavy variables that aren't needed for individual endpoint rendering
            if key != "endpoints" && key != "endpoint" {
                base_tera_context.insert(key, value);
            }
        }

        // Generate one file per endpoint
        for endpoint in endpoints {
            // OPTIMIZATION: Clone base context instead of rebuilding from scratch
            let mut tera_context = base_tera_context.clone();

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
            // OPTIMIZATION: Replace expensive deep cleaning with targeted unresolved_ref replacement
            if let Some(obj) = endpoint.as_object() {
                // OPTIMIZATION: Fast unresolved_ref detection without full traversal
                let has_unresolved_ref = Self::has_unresolved_ref(obj);

                if has_unresolved_ref {
                    // OPTIMIZATION: Targeted cleaning instead of deep traversal
                    let cleaned_obj = Self::clean_unresolved_refs(obj);
                    // OPTIMIZATION: Batch insert all values at once
                    for (key, value) in cleaned_obj {
                        tera_context.insert(key, &value);
                    }
                } else {
                    // Fast path: insert directly without cleaning
                    // OPTIMIZATION: Batch insert all values at once
                    for (key, value) in obj {
                        tera_context.insert(key, &value);
                    }
                }

                // Minimal debug logging (only in debug mode)
                tracing::debug!(
                    "McpServerTemplateRenderer: Endpoint context for '{}' processed",
                    endpoint_name
                );
            }

            // Replace {endpoint} placeholder in destination path
            let output_path = file_destination
                .replace("{endpoint}", endpoint_name)
                .replace("{operation_id}", endpoint_name);

            // Render the template
            let rendered = tera.render(template_name, &tera_context)
                .map_err(|e| {
                    GenerationError::RenderError(format!(
                        "Failed to render template '{template_name}' for endpoint '{endpoint_name}': {e}"
                    ))
                })?;

            artifacts.push(Artifact {
                path: PathBuf::from(output_path),
                content: rendered,
                permissions: None,
            });
        }

        Ok(artifacts)
    }

    /// OPTIMIZATION: Fast unresolved_ref detection without full object traversal
    fn has_unresolved_ref(obj: &serde_json::Map<String, serde_json::Value>) -> bool {
        // OPTIMIZATION: Early return on first unresolved_ref found, avoiding full scan
        for value in obj.values() {
            if let Some(inner_obj) = value.as_object() {
                if inner_obj.contains_key("unresolved_ref") {
                    return true;
                }
            }
        }
        false
    }

    /// OPTIMIZATION: Targeted cleaning that only processes objects with unresolved_ref
    fn clean_unresolved_refs(obj: &serde_json::Map<String, serde_json::Value>) -> serde_json::Map<String, serde_json::Value> {
        let mut cleaned = serde_json::Map::new();

        for (key, value) in obj {
            match value {
                serde_json::Value::Object(inner_obj) => {
                    if inner_obj.contains_key("unresolved_ref") {
                        // OPTIMIZATION: Replace entire unresolved_ref object with placeholder
                        // No need for deep traversal - just replace the problematic object
                        if let Some(ref_value) = inner_obj.get("unresolved_ref").and_then(|v| v.as_str()) {
                            tracing::debug!("Targeted clean: Detected unresolved_ref for key '{}'. Replacing with placeholder.", key);
                            cleaned.insert(key.clone(), json!({
                                "type": "object",
                                "description": format!("Recursive reference to {}", ref_value)
                            }));
                        } else {
                            // Fallback: insert as-is if unresolved_ref format is unexpected
                            cleaned.insert(key.clone(), value.clone());
                        }
                    } else {
                        // OPTIMIZATION: No unresolved_ref in this object, insert directly
                        cleaned.insert(key.clone(), value.clone());
                    }
                }
                // OPTIMIZATION: Non-object values inserted directly without processing
                _ => {
                    cleaned.insert(key.clone(), value.clone());
                }
            }
        }

        cleaned
    }

    // Helper function to deep clean JsonValue for Tera context
    fn deep_clean_json_value(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut cleaned_map = serde_json::Map::new();
                for (k, v) in map {
                    if k == "unresolved_ref" && v.is_string() {
                        // If it's an unresolved_ref, replace the entire object with a placeholder
                        tracing::debug!("Deep clean: Detected unresolved_ref for key '{}'. Replacing with placeholder.", k);
                        return json!({
                            "type": "object",
                            "description": format!("Recursive reference to {}", v.as_str().unwrap_or("unknown"))
                        });
                    }
                    if !v.is_null() {
                        let cleaned_value = Self::deep_clean_json_value(v);
                        // Only include non-empty objects and arrays, and non-null values
                        // Do not filter out empty objects or arrays, as templates might check their length
                        // Filter out only null values
                        if !cleaned_value.is_null() {
                            cleaned_map.insert(k.clone(), cleaned_value);
                        }
                    }
                }
                serde_json::Value::Object(cleaned_map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::deep_clean_json_value).collect())
            }
            _ => value.clone(),
        }
    }
}

/// Tera filter to escape Rust keywords with `r#` prefix.
///
/// Usage in templates: `{{ my_variable | rust_escape_keyword }}`
fn rust_escape_keyword_filter(value: &TeraValue, _args: &std::collections::HashMap<String, TeraValue>) -> TeraResult<TeraValue> {
    let s = from_value::<String>(value.clone())?;
    Ok(TeraValue::String(sanitize_rust_identifier(&s, false)))
}

/// Tera filter to preserve Rust keywords (no `r#` prefix).
///
/// Usage in templates: `{{ my_variable | rust_preserve_keyword }}`
fn rust_preserve_keyword_filter(value: &TeraValue, _args: &std::collections::HashMap<String, TeraValue>) -> TeraResult<TeraValue> {
    let s = from_value::<String>(value.clone())?;
    Ok(TeraValue::String(sanitize_rust_identifier(&s, true)))
}

#[async_trait]
impl TemplateRenderingStrategy for McpServerTemplateRenderer {
    async fn render(
        &self,
        template: &Template,
        context: &RenderContext,
        generation_context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        tracing::info!("McpServerTemplateRenderer: Starting template rendering for protocol {:?}, role {:?}", generation_context.protocol, generation_context.role);

        // Verify this is for MCP server
        if generation_context.protocol != Protocol::Mcp || generation_context.role != Role::Server {
            tracing::error!("McpServerTemplateRenderer: Invalid protocol/role combination: {:?}/{:?}", generation_context.protocol, generation_context.role);
            return Err(GenerationError::InvalidConfiguration(
                "McpServerTemplateRenderer can only be used for MCP servers".to_string(),
            ));
        }

        let mut artifacts = Vec::new();
        let mut tera = Tera::default();

        tracing::debug!("McpServerTemplateRenderer: Initializing Tera template engine");

        // Register custom filters for Rust keyword handling
        tera.register_filter("rust_escape_keyword", rust_escape_keyword_filter);
        tera.register_filter("rust_preserve_keyword", rust_preserve_keyword_filter);

        // Add template files to Tera, indexed by their manifest source names
        for manifest_file in &template.manifest.files {
            if let Some(template_file) = template
                .files
                .iter()
                .find(|f| f.path.to_string_lossy() == manifest_file.source)
                && matches!(manifest_file.file_type, TemplateFileType::Template { .. })
            {
                tera.add_raw_template(&manifest_file.source, &template_file.content)
                    .map_err(|e| {
                        GenerationError::RenderError(format!(
                            "Failed to add template '{}': {}",
                            manifest_file.source, e
                        ))
                    })?;
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
                                "Unsupported for_each value: {collection_key}"
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
                        });
                    }
                }
                TemplateFileType::Static => {
                    // Copy static files as-is
                    artifacts.push(Artifact {
                        path: PathBuf::from(&manifest_file.target),
                        content: template_file.content.clone(),
                        permissions: None,
                    });
                }
                _ => {
                    // Skip other file types
                }
            }
        }

        // Generate schema files for MCP servers
        tracing::info!("McpServerTemplateRenderer: Generating schema artifacts.");
        artifacts.extend(self.generate_schema_artifacts(context)?);
        tracing::info!("McpServerTemplateRenderer: Finished generating schema artifacts.");

        Ok(artifacts)
    }
}
