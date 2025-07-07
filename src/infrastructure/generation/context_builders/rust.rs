//! Rust-specific context builder for code generation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue, json};

use crate::generation::{
    ContextBuilder, GenerationContext, GenerationError, Language, Operation, RenderContext,
    sanitizers::sanitize_markdown,
    utils::{sanitize_rust_field_name, to_proper_case, to_snake_case},
};
use crate::infrastructure::Template;

/// Rust-specific property information with type mapping
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RustPropertyInfo {
    pub name: String,
    pub rust_type: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub example: Option<JsonValue>,
}

/// Complete Rust-specific context for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustEndpointContext {
    pub endpoint: String,
    pub endpoint_cap: String,
    pub endpoint_fs: String,
    pub path: String,
    pub fn_name: String,
    pub parameters_type: String,
    pub properties_type: String,
    pub response_type: String,
    pub envelope_properties: JsonValue,
    pub properties: Vec<RustPropertyInfo>,
    pub properties_for_handler: Vec<String>,
    pub parameters: Vec<JsonValue>,
    pub summary: String,
    pub description: String,
    pub tags: Vec<String>,
    pub properties_schema: JsonMap<String, JsonValue>,
    pub response_schema: JsonValue,
    pub spec_file_name: Option<String>,
    pub valid_fields: Vec<String>,
    // Response type analysis for template compatibility
    pub response_is_array: bool,
    pub response_is_object: bool,
    pub response_is_primitive: bool,
    pub response_item_type: String,
    pub response_primitive_type: String,
    pub response_properties: Vec<RustPropertyInfo>,
}

/// Rust-specific context builder
pub struct RustContextBuilder;

impl RustContextBuilder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ContextBuilder for RustContextBuilder {
    async fn build(
        &self,
        context: &GenerationContext,
        template: &Template,
    ) -> Result<RenderContext, GenerationError> {
        // Ensure this is for Rust
        if context.language != Language::Rust {
            return Err(GenerationError::InvalidConfiguration(format!(
                "RustContextBuilder can only build contexts for Rust, got {:?}",
                context.language
            )));
        }

        let mut render_context = RenderContext::new();

        // Add base context variables
        render_context.add_variable("project_name", json!(context.metadata.project_name));
        render_context.add_variable("version", json!(context.metadata.version));
        render_context.add_variable("description", json!(context.metadata.description));
        render_context.add_variable("protocol", json!(context.protocol.to_string()));
        render_context.add_variable("role", json!(context.role.to_string()));
        render_context.add_variable("language", json!("rust"));

        // Add Rust-specific naming conventions
        let crate_name = to_snake_case(&context.metadata.project_name);
        let module_name = to_snake_case(&context.metadata.project_name);
        let struct_name = to_proper_case(&context.metadata.project_name);

        render_context.add_variable("crate_name", json!(crate_name));
        render_context.add_variable("module_name", json!(module_name));
        render_context.add_variable("struct_name", json!(struct_name));
        render_context.add_variable("cli_binary_name", json!(crate_name));
        render_context.add_variable("license", json!("MIT License"));

        // Handle protocol-specific context
        let mut endpoints = Vec::new();
        if let Some(protocol_context) = &context.protocol_context {
            match protocol_context {
                crate::generation::ProtocolContext::McpServer {
                    openapi_spec,
                    endpoints: operations,
                } => {
                    // Add OpenAPI spec information
                    render_context.add_variable("api_version", json!(openapi_spec.version));
                    render_context.add_variable("api_title", json!(openapi_spec.info.title));
                    render_context
                        .add_variable("api_info_version", json!(openapi_spec.info.version));
                    if let Some(desc) = &openapi_spec.info.description {
                        render_context.add_variable("api_description", json!(desc));
                    }

                    // Add servers information
                    if !openapi_spec.servers.is_empty() {
                        render_context.add_variable("api_servers", json!(openapi_spec.servers));
                        render_context
                            .add_variable("api_base_url", json!(openapi_spec.servers[0].url));
                    }

                    // Add components for potential $ref resolution
                    if let Some(components) = &openapi_spec.components {
                        render_context.add_variable("api_components", json!(components.schemas));
                    }

                    // Process operations into Rust endpoint contexts
                    tracing::debug!(
                        "Rust context builder processing {} MCP endpoints from OpenAPI operations",
                        operations.len()
                    );
                    for operation in operations {
                        let endpoint_context = build_rust_endpoint_context(operation)?;
                        endpoints.push(serde_json::to_value(endpoint_context)?);
                    }
                }
            }
        }
        tracing::debug!(
            "Rust context builder created {} endpoint contexts",
            endpoints.len()
        );
        // Add both "endpoints" and "endpoint" for compatibility
        render_context.add_variable("endpoints", json!(endpoints.clone()));
        render_context.add_variable("endpoint", json!(endpoints));

        // Debug: Print first endpoint to see parameter structure
        if let Some(first_endpoint) = endpoints.first() {
            tracing::debug!(
                "First endpoint structure: {}",
                serde_json::to_string_pretty(first_endpoint).unwrap_or_default()
            );
        }

        // Add all custom variables from context
        for (key, value) in &context.variables {
            render_context.add_variable(key, value.clone());
        }

        // Add template manifest variables if any
        for (key, value) in &template.manifest.variables {
            if !render_context.has_variable(key) {
                render_context.add_variable(key, value.clone());
            }
        }

        // Add template manifest metadata
        render_context.add_variable("template_name", json!(template.manifest.name));
        render_context.add_variable("template_version", json!(template.manifest.version));
        if let Some(description) = &template.manifest.description {
            render_context.add_variable("template_description", json!(description));
        }

        Ok(render_context)
    }
}

fn build_rust_endpoint_context(op: &Operation) -> Result<RustEndpointContext, GenerationError> {
    let endpoint_id = to_snake_case(&op.id);

    Ok(RustEndpointContext {
        fn_name: endpoint_id.clone(),
        parameters_type: to_proper_case(&format!("{}_params", op.id)),
        endpoint: endpoint_id.clone(),
        endpoint_cap: to_proper_case(&op.id),
        endpoint_fs: endpoint_id,
        path: op.path.clone(),
        properties_type: to_proper_case(&format!("{}_properties", op.id)),
        response_type: to_proper_case(&format!("{}_response", op.id)),
        envelope_properties: extract_envelope_properties(op),
        properties: extract_request_body_properties(op),
        properties_for_handler: extract_handler_properties(op),
        parameters: extract_parameters(op),
        summary: op
            .summary
            .as_ref()
            .map(|s| sanitize_markdown(s))
            .unwrap_or_default(),
        description: op
            .description
            .as_ref()
            .map(|s| sanitize_markdown(s))
            .unwrap_or_default(),
        tags: op.tags.clone().unwrap_or_default(),
        properties_schema: extract_properties_schema(op),
        response_schema: extract_response_schema(op),
        spec_file_name: None, // Would need to be passed from context
        valid_fields: extract_valid_fields(op),
        // Simple response type analysis
        response_is_array: is_array_response(op),
        response_is_object: is_object_response(op),
        response_is_primitive: is_primitive_response(op),
        response_item_type: get_array_item_type(op),
        response_primitive_type: get_primitive_type(op),
        response_properties: extract_response_properties(op),
    })
}

fn extract_envelope_properties(op: &Operation) -> JsonValue {
    for response in &op.responses {
        if response.status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema_json) = json_content.get("schema") {
                        if let Ok(schema) =
                            serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
                        {
                            return extract_typed_envelope_properties(&schema);
                        }
                    }
                }
            }
        }
    }
    json!({})
}

fn extract_response_properties(op: &Operation) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    for response in &op.responses {
        if response.status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema_json) = json_content.get("schema") {
                        if let Ok(schema) =
                            serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
                        {
                            properties.extend(extract_typed_schema_properties(&schema));
                        }
                    }
                }
            }
        }
    }

    properties
}

fn extract_handler_properties(op: &Operation) -> Vec<String> {
    extract_request_body_properties(op)
        .into_iter()
        .map(|prop| prop.name)
        .collect()
}

fn extract_parameters(op: &Operation) -> Vec<JsonValue> {
    op.parameters
        .iter()
        .map(|p| {
            json!({
                "name": to_snake_case(&p.name),
                "rust_name": to_snake_case(&p.name),
                "target_type": map_schema_to_rust_type(&p.schema),
                "rust_type": map_schema_to_rust_type(&p.schema),  // Template expects rust_type
                "in": format!("{:?}", p.location).to_lowercase(),
                "required": p.required,
                "description": p.description.as_ref().map(|d| sanitize_markdown(d)),
                "example": serde_json::Value::Null
            })
        })
        .collect()
}

fn extract_typed_envelope_properties(schema: &crate::generation::Schema) -> JsonValue {
    if let Some(properties) = &schema.properties {
        // Convert HashMap<String, Schema> back to JsonValue for compatibility
        let mut json_props = serde_json::Map::new();
        for (key, value) in properties {
            if let Ok(json_val) = serde_json::to_value(value) {
                json_props.insert(key.clone(), json_val);
            }
        }
        return JsonValue::Object(json_props);
    }

    if schema.schema_type.as_deref() == Some("array") {
        if let Some(items) = &schema.items {
            return extract_typed_envelope_properties(items);
        }
    }

    json!({})
}

fn extract_typed_schema_properties(schema: &crate::generation::Schema) -> Vec<RustPropertyInfo> {
    let mut rust_properties = Vec::new();

    if let Some(properties) = &schema.properties {
        for (prop_name, prop_schema) in properties {
            let rust_type = map_schema_to_rust_type(prop_schema);
            let title = prop_schema.title.clone();
            let description = prop_schema
                .description
                .as_ref()
                .map(|d| sanitize_markdown(d));
            let example = prop_schema.example.clone();

            rust_properties.push(RustPropertyInfo {
                name: sanitize_rust_field_name(prop_name),
                rust_type,
                title,
                description,
                example,
            });
        }
    }

    if schema.schema_type.as_deref() == Some("array") {
        if let Some(items) = &schema.items {
            rust_properties.extend(extract_typed_schema_properties(items));
        }
    }

    rust_properties
}

fn map_schema_to_rust_type(schema: &crate::generation::Schema) -> String {
    if let Some(typ) = &schema.schema_type {
        match typ.as_str() {
            "string" => "String".to_string(),
            "integer" => "i32".to_string(),
            "boolean" => "bool".to_string(),
            "number" => "f64".to_string(),
            "array" => {
                if let Some(items) = &schema.items {
                    format!("Vec<{}>", map_schema_to_rust_type(items))
                } else {
                    "Vec<serde_json::Value>".to_string()
                }
            }
            "object" => "serde_json::Value".to_string(),
            _ => "String".to_string(),
        }
    } else {
        "String".to_string()
    }
}

// Removed map_json_schema_to_rust_type - now using map_schema_to_rust_type for typed schemas

fn extract_properties_schema(op: &Operation) -> JsonMap<String, JsonValue> {
    if let Some(request_body) = &op.request_body {
        if let Some(content) = request_body.content.as_object() {
            if let Some(json_content) = content.get("application/json") {
                if let Some(schema_json) = json_content.get("schema") {
                    if let Ok(schema) =
                        serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
                    {
                        if let Some(properties) = extract_typed_properties_map(&schema) {
                            return properties;
                        }
                    }
                }
            }
        }
    }
    JsonMap::new()
}

fn extract_response_schema(op: &Operation) -> JsonValue {
    for response in &op.responses {
        if response.status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema) = json_content.get("schema") {
                        return schema.clone();
                    }
                }
            }
        }
    }
    json!({})
}

fn extract_valid_fields(op: &Operation) -> Vec<String> {
    extract_response_properties(op)
        .into_iter()
        .map(|prop| prop.name)
        .collect()
}

fn extract_typed_properties_map(
    schema: &crate::generation::Schema,
) -> Option<JsonMap<String, JsonValue>> {
    if let Some(properties) = &schema.properties {
        // Convert HashMap<String, Schema> back to JsonMap<String, JsonValue> for compatibility
        let mut json_map = JsonMap::new();
        for (key, value) in properties {
            if let Ok(json_val) = serde_json::to_value(value) {
                json_map.insert(key.clone(), json_val);
            }
        }
        return Some(json_map);
    }

    if schema.schema_type.as_deref() == Some("array") {
        if let Some(items) = &schema.items {
            return extract_typed_properties_map(items);
        }
    }

    None
}

fn is_array_response(op: &Operation) -> bool {
    if let Some(schema) = get_typed_response_schema(op) {
        schema.schema_type.as_deref() == Some("array")
    } else {
        false
    }
}

fn is_object_response(op: &Operation) -> bool {
    if let Some(schema) = get_typed_response_schema(op) {
        schema.schema_type.as_deref() == Some("object") || schema.properties.is_some()
    } else {
        false
    }
}

fn is_primitive_response(op: &Operation) -> bool {
    if let Some(schema) = get_typed_response_schema(op) {
        matches!(
            schema.schema_type.as_deref(),
            Some("string") | Some("integer") | Some("number") | Some("boolean")
        )
    } else {
        false
    }
}

fn get_array_item_type(op: &Operation) -> String {
    if is_array_response(op) {
        if let Some(schema) = get_typed_response_schema(op) {
            if let Some(items) = &schema.items {
                return map_schema_to_rust_type(items);
            }
        }
    }
    "serde_json::Value".to_string()
}

fn get_primitive_type(op: &Operation) -> String {
    if is_primitive_response(op) {
        if let Some(schema) = get_typed_response_schema(op) {
            return map_schema_to_rust_type(&schema);
        }
    }
    "serde_json::Value".to_string()
}
fn extract_request_body_properties(op: &Operation) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    if let Some(request_body) = &op.request_body {
        if let Some(content) = request_body.content.as_object() {
            if let Some(json_content) = content.get("application/json") {
                if let Some(schema_json) = json_content.get("schema") {
                    if let Ok(schema) =
                        serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
                    {
                        properties.extend(extract_typed_schema_properties(&schema));
                    }
                }
            }
        }
    }

    properties
}

fn get_typed_response_schema(op: &Operation) -> Option<crate::generation::Schema> {
    // Look for successful response
    for response in &op.responses {
        if response.status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema_json) = json_content.get("schema") {
                        if let Ok(schema) =
                            serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
                        {
                            return Some(schema);
                        }
                    }
                }
            }
        }
    }
    None
}

// Removed map_openapi_type_to_rust - now using map_schema_to_rust_type for typed schemas

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{Template, TemplateManifest, TemplateSource};
    use crate::protocols::{Protocol, Role};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_rust_context_builder() {
        let builder = RustContextBuilder::new();

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "test_project".to_string();
        context.metadata.version = "1.0.0".to_string();

        let manifest = TemplateManifest {
            name: "test-template".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            path: "mcp/server/rust".to_string(),
            protocol: Protocol::Mcp,
            role: Role::Server,
            language: Language::Rust,
            files: vec![],
            variables: HashMap::new(),
            post_generate_hooks: vec![],
        };

        let template = Template {
            manifest,
            files: vec![],
            source: TemplateSource::Embedded,
        };

        let result = builder.build(&context, &template).await;
        assert!(result.is_ok());

        // Test passes if build succeeds - the actual rendering will verify the variables
    }

    #[tokio::test]
    async fn test_context_builder_wrong_language() {
        let builder = RustContextBuilder::new();

        let context = GenerationContext::new(
            Protocol::Mcp,
            Role::Server,
            Language::Python, // Wrong language
        );

        let manifest = TemplateManifest {
            name: "test-template".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            path: "mcp/server/python".to_string(),
            protocol: Protocol::Mcp,
            role: Role::Server,
            language: Language::Python,
            files: vec![],
            variables: HashMap::new(),
            post_generate_hooks: vec![],
        };

        let template = Template {
            manifest,
            files: vec![],
            source: TemplateSource::Embedded,
        };

        let result = builder.build(&context, &template).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_template_manifest_fields_in_context() {
        let builder = RustContextBuilder::new();

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "test_project".to_string();

        let manifest = TemplateManifest {
            name: "test-template".to_string(),
            version: "2.0.0".to_string(),
            description: Some("Test template description".to_string()),
            path: "mcp/server/rust".to_string(),
            protocol: Protocol::Mcp,
            role: Role::Server,
            language: Language::Rust,
            files: vec![],
            variables: HashMap::new(),
            post_generate_hooks: vec![],
        };

        let template = Template {
            manifest,
            files: vec![],
            source: TemplateSource::Embedded,
        };

        let result = builder.build(&context, &template).await;
        assert!(result.is_ok());

        // Test passes if build succeeds with manifest fields
    }
}
