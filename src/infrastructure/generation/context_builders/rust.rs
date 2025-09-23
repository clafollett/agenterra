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

/// Source of a unified parameter (query param vs request body property)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParameterSource {
    Query,
    Body,
}

/// Unified parameter combining query parameters and request body properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedParameter {
    pub name: String,
    pub original_name: String,
    pub source: ParameterSource,
    pub rust_type: String,
    pub description: Option<String>,
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
    // NEW: Unified parameter support for Issue #106
    pub unified_parameters: Vec<UnifiedParameter>,
    pub has_body_properties: bool,
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

    // Extract parameters and properties for unified handling
    let query_params = &op.parameters;
    let body_properties = extract_request_body_properties(op);
    let unified_parameters = build_unified_parameters(query_params, &body_properties);
    let has_body_properties = !body_properties.is_empty();

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
        properties: body_properties,
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
        // NEW: Unified parameter support for Issue #106
        unified_parameters,
        has_body_properties,
    })
}

fn extract_envelope_properties(op: &Operation) -> JsonValue {
    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(content) = response.content.as_ref()
            && let Some(json_content) = content.get("application/json")
            && let Some(schema_json) = json_content.get("schema")
            && let Ok(schema) =
                serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
        {
            return extract_typed_envelope_properties(&schema);
        }
    }
    json!({})
}

fn extract_response_properties(op: &Operation) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(content) = response.content.as_ref()
            && let Some(json_content) = content.get("application/json")
            && let Some(schema_json) = json_content.get("schema")
            && let Ok(schema) =
                serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
        {
            properties.extend(extract_typed_schema_properties(&schema));
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

    if schema.schema_type.as_deref() == Some("array")
        && let Some(items) = &schema.items
    {
        return extract_typed_envelope_properties(items);
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

    if schema.schema_type.as_deref() == Some("array")
        && let Some(items) = &schema.items
    {
        rust_properties.extend(extract_typed_schema_properties(items));
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
    if let Some(request_body) = &op.request_body
        && let Some(content) = request_body.content.as_object()
        && let Some(json_content) = content.get("application/json")
        && let Some(schema_json) = json_content.get("schema")
        && let Ok(schema) = serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
        && let Some(properties) = extract_typed_properties_map(&schema)
    {
        return properties;
    }
    JsonMap::new()
}

fn extract_response_schema(op: &Operation) -> JsonValue {
    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(content) = response.content.as_ref()
            && let Some(json_content) = content.get("application/json")
            && let Some(schema) = json_content.get("schema")
        {
            return schema.clone();
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

    if schema.schema_type.as_deref() == Some("array")
        && let Some(items) = &schema.items
    {
        return extract_typed_properties_map(items);
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
    if is_array_response(op)
        && let Some(schema) = get_typed_response_schema(op)
        && let Some(items) = &schema.items
    {
        return map_schema_to_rust_type(items);
    }
    "serde_json::Value".to_string()
}

fn get_primitive_type(op: &Operation) -> String {
    if is_primitive_response(op)
        && let Some(schema) = get_typed_response_schema(op)
    {
        return map_schema_to_rust_type(&schema);
    }
    "serde_json::Value".to_string()
}
fn extract_request_body_properties(op: &Operation) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    if let Some(request_body) = &op.request_body
        && let Some(content) = request_body.content.as_object()
        && let Some(json_content) = content.get("application/json")
        && let Some(schema_json) = json_content.get("schema")
        && let Ok(schema) = serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
    {
        properties.extend(extract_typed_schema_properties(&schema));
    }

    properties
}

fn get_typed_response_schema(op: &Operation) -> Option<crate::generation::Schema> {
    // Look for successful response
    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(content) = response.content.as_ref()
            && let Some(json_content) = content.get("application/json")
            && let Some(schema_json) = json_content.get("schema")
            && let Ok(schema) =
                serde_json::from_value::<crate::generation::Schema>(schema_json.clone())
        {
            return Some(schema);
        }
    }
    None
}

// Removed map_openapi_type_to_rust - now using map_schema_to_rust_type for typed schemas

/// Build unified parameters from query params and request body properties
/// Handles collision detection by adding _q/_b suffixes when names collide
fn build_unified_parameters(
    query_params: &[crate::generation::Parameter],
    body_properties: &[RustPropertyInfo],
) -> Vec<UnifiedParameter> {
    use std::collections::HashMap;

    let mut unified = Vec::new();
    let mut name_counts = HashMap::new();

    // Count all names to detect collisions
    for param in query_params {
        *name_counts.entry(&param.name).or_insert(0) += 1;
    }
    for prop in body_properties {
        *name_counts.entry(&prop.name).or_insert(0) += 1;
    }

    // Generate parameters with suffixes only when needed
    for param in query_params {
        let final_name = if name_counts[&param.name] > 1 {
            format!("{}_q", param.name) // Collision: add suffix
        } else {
            param.name.clone() // No collision: keep original
        };

        unified.push(UnifiedParameter {
            name: to_snake_case(&final_name),
            original_name: param.name.clone(),
            source: ParameterSource::Query,
            rust_type: map_schema_to_rust_type(&param.schema),
            description: param.description.clone(),
        });
    }

    for prop in body_properties {
        let final_name = if name_counts[&prop.name] > 1 {
            format!("{}_b", prop.name) // Collision: add suffix
        } else {
            prop.name.clone() // No collision: keep original
        };

        unified.push(UnifiedParameter {
            name: to_snake_case(&final_name),
            original_name: prop.name.clone(),
            source: ParameterSource::Body,
            rust_type: prop.rust_type.clone(),
            description: prop.description.clone(),
        });
    }

    unified
}

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

    // RED PHASE: Tests for unified parameter collision detection (Issue #106)

    #[test]
    fn test_build_unified_parameters_no_collision() {
        // GIVEN: Query param "limit" and body prop "query" (no collision)
        let query_params = vec![create_test_parameter("limit", "integer")];
        let body_properties = vec![create_test_property("query", "String")];

        // WHEN: build_unified_parameters() called
        let result = build_unified_parameters(&query_params, &body_properties);

        // THEN: Original names preserved, no suffixes
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "limit");
        assert_eq!(result[0].original_name, "limit");
        assert!(matches!(result[0].source, ParameterSource::Query));
        assert_eq!(result[1].name, "query");
        assert_eq!(result[1].original_name, "query");
        assert!(matches!(result[1].source, ParameterSource::Body));
    }

    #[test]
    fn test_build_unified_parameters_with_collision() {
        // GIVEN: Query param "limit" and body prop "limit" (collision!)
        let query_params = vec![create_test_parameter("limit", "integer")];
        let body_properties = vec![create_test_property("limit", "i32")];

        // WHEN: build_unified_parameters() called
        let result = build_unified_parameters(&query_params, &body_properties);

        // THEN: Query becomes "limit_q", body becomes "limit_b"
        assert_eq!(result.len(), 2);

        let query_param = result
            .iter()
            .find(|p| matches!(p.source, ParameterSource::Query))
            .unwrap();
        assert_eq!(query_param.name, "limit_q");
        assert_eq!(query_param.original_name, "limit");

        let body_param = result
            .iter()
            .find(|p| matches!(p.source, ParameterSource::Body))
            .unwrap();
        assert_eq!(body_param.name, "limit_b");
        assert_eq!(body_param.original_name, "limit");
    }

    #[test]
    fn test_build_unified_parameters_multiple_collisions() {
        // GIVEN: Multiple colliding names
        let query_params = vec![
            create_test_parameter("limit", "integer"),
            create_test_parameter("format", "string"),
        ];
        let body_properties = vec![
            create_test_property("limit", "i32"),
            create_test_property("query", "String"),
            create_test_property("format", "String"),
        ];

        // WHEN: build_unified_parameters() called
        let result = build_unified_parameters(&query_params, &body_properties);

        // THEN: All collisions properly suffixed
        assert_eq!(result.len(), 5);

        // Check "limit" collision
        let limit_q = result.iter().find(|p| p.name == "limit_q").unwrap();
        assert!(matches!(limit_q.source, ParameterSource::Query));
        let limit_b = result.iter().find(|p| p.name == "limit_b").unwrap();
        assert!(matches!(limit_b.source, ParameterSource::Body));

        // Check "format" collision
        let format_q = result.iter().find(|p| p.name == "format_q").unwrap();
        assert!(matches!(format_q.source, ParameterSource::Query));
        let format_b = result.iter().find(|p| p.name == "format_b").unwrap();
        assert!(matches!(format_b.source, ParameterSource::Body));

        // Check no collision
        let query_param = result.iter().find(|p| p.name == "query").unwrap();
        assert!(matches!(query_param.source, ParameterSource::Body));
    }

    #[test]
    fn test_rust_endpoint_context_unified_parameters() {
        // GIVEN: Operation with both query params and request body
        let operation = create_test_operation_with_both_params_and_body();

        // WHEN: build_rust_endpoint_context() called
        let result = build_rust_endpoint_context(&operation);

        // THEN: unified_parameters populated, has_body_properties = true
        assert!(result.is_ok());
        let context = result.unwrap();
        assert!(!context.unified_parameters.is_empty());
        assert!(context.has_body_properties);
    }

    #[test]
    fn test_rust_endpoint_context_query_only() {
        // GIVEN: GET operation with only query params
        let operation = create_test_operation_query_only();

        // WHEN: build_rust_endpoint_context() called
        let result = build_rust_endpoint_context(&operation);

        // THEN: unified_parameters = query params, has_body_properties = false
        assert!(result.is_ok());
        let context = result.unwrap();
        assert!(!context.unified_parameters.is_empty());
        assert!(!context.has_body_properties);
    }

    #[test]
    fn test_rust_endpoint_context_body_only() {
        // GIVEN: POST operation with only request body
        let operation = create_test_operation_body_only();

        // WHEN: build_rust_endpoint_context() called
        let result = build_rust_endpoint_context(&operation);

        // THEN: unified_parameters = body props, has_body_properties = true
        assert!(result.is_ok());
        let context = result.unwrap();
        assert!(!context.unified_parameters.is_empty());
        assert!(context.has_body_properties);
    }

    // Helper functions for tests
    fn create_test_parameter(name: &str, schema_type: &str) -> crate::generation::Parameter {
        use crate::generation::{Parameter, ParameterLocation};
        use crate::infrastructure::openapi::types::Schema;
        Parameter {
            name: name.to_string(),
            location: ParameterLocation::Query,
            required: false,
            schema: Schema {
                schema_type: Some(schema_type.to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                description: None,
                title: None,
                default: None,
                example: None,
                enum_values: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                min_items: None,
                max_items: None,
                unique_items: None,
                additional_properties: None,
                all_of: None,
                one_of: None,
                any_of: None,
                not: None,
                discriminator: None,
                read_only: None,
                write_only: None,
                xml: None,
                external_docs: None,
                deprecated: None,
                nullable: None,
            },
            description: None,
        }
    }

    fn create_test_property(name: &str, rust_type: &str) -> RustPropertyInfo {
        RustPropertyInfo {
            name: name.to_string(),
            rust_type: rust_type.to_string(),
            title: None,
            description: None,
            example: None,
        }
    }

    fn create_test_operation_with_both_params_and_body() -> crate::generation::Operation {
        use crate::generation::{Operation, RequestBody};
        use serde_json::json;

        Operation {
            id: "testOp".to_string(),
            path: "/test".to_string(),
            method: "POST".to_string(),
            summary: Some("Test operation".to_string()),
            description: None,
            external_docs: None,
            tags: None,
            parameters: vec![create_test_parameter("limit", "integer")],
            request_body: Some(RequestBody {
                description: None,
                content: json!({
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "query": {"type": "string"}
                            }
                        }
                    }
                }),
                required: true,
            }),
            responses: vec![],
            callbacks: None,
            deprecated: None,
            security: None,
            servers: None,
            vendor_extensions: Default::default(),
        }
    }

    fn create_test_operation_query_only() -> crate::generation::Operation {
        use crate::generation::Operation;

        Operation {
            id: "getOp".to_string(),
            path: "/get".to_string(),
            method: "GET".to_string(),
            summary: Some("Get operation".to_string()),
            description: None,
            external_docs: None,
            tags: None,
            parameters: vec![create_test_parameter("limit", "integer")],
            request_body: None,
            responses: vec![],
            callbacks: None,
            deprecated: None,
            security: None,
            servers: None,
            vendor_extensions: Default::default(),
        }
    }

    fn create_test_operation_body_only() -> crate::generation::Operation {
        use crate::generation::{Operation, RequestBody};
        use serde_json::json;

        Operation {
            id: "postOp".to_string(),
            path: "/post".to_string(),
            method: "POST".to_string(),
            summary: Some("Post operation".to_string()),
            description: None,
            external_docs: None,
            tags: None,
            parameters: vec![],
            request_body: Some(RequestBody {
                description: None,
                content: json!({
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "query": {"type": "string"}
                            }
                        }
                    }
                }),
                required: true,
            }),
            responses: vec![],
            callbacks: None,
            deprecated: None,
            security: None,
            servers: None,
            vendor_extensions: Default::default(),
        }
    }
}
