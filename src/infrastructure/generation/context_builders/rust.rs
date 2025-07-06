//! Rust-specific context builder for code generation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue, json};

use crate::generation::{
    ContextBuilder, GenerationContext, GenerationError, Language, Operation, RenderContext,
    utils::{to_proper_case, to_snake_case},
};
use crate::infrastructure::templates::Template;

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

        // Process operations into Rust endpoint contexts
        let mut endpoints = Vec::new();
        tracing::debug!(
            "Rust context builder processing {} operations",
            context.operations.len()
        );
        for operation in &context.operations {
            let endpoint_context = build_rust_endpoint_context(operation)?;
            endpoints.push(serde_json::to_value(endpoint_context)?);
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
            tracing::debug!("First endpoint structure: {}", serde_json::to_string_pretty(first_endpoint).unwrap_or_default());
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
        properties: extract_response_properties(op),
        properties_for_handler: extract_handler_properties(op),
        parameters: extract_parameters(op),
        summary: op.summary.clone().unwrap_or_default(),
        description: op.description.clone().unwrap_or_default(),
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
                    if let Some(schema) = json_content.get("schema") {
                        return extract_schema_envelope_properties(schema);
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
                    if let Some(schema) = json_content.get("schema") {
                        properties.extend(extract_schema_properties_as_rust(schema));
                    }
                }
            }
        }
    }

    properties
}

fn extract_handler_properties(op: &Operation) -> Vec<String> {
    extract_response_properties(op)
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
                "description": p.description,
                "example": serde_json::Value::Null
            })
        })
        .collect()
}

fn extract_schema_envelope_properties(schema: &JsonValue) -> JsonValue {
    if let Some(_ref_str) = schema.get("$ref").and_then(JsonValue::as_str) {
        return json!({});
    }

    if let Some(properties) = schema.get("properties") {
        return properties.clone();
    }

    if schema.get("type").and_then(JsonValue::as_str) == Some("array") {
        if let Some(items) = schema.get("items") {
            return extract_schema_envelope_properties(items);
        }
    }

    json!({})
}

fn extract_schema_properties_as_rust(schema: &JsonValue) -> Vec<RustPropertyInfo> {
    let mut rust_properties = Vec::new();

    if let Some(_ref_str) = schema.get("$ref").and_then(JsonValue::as_str) {
        return rust_properties;
    }

    if let Some(properties) = schema.get("properties").and_then(JsonValue::as_object) {
        for (prop_name, prop_schema) in properties {
            let rust_type = map_json_schema_to_rust_type(prop_schema);
            let title = prop_schema
                .get("title")
                .and_then(JsonValue::as_str)
                .map(String::from);
            let description = prop_schema
                .get("description")
                .and_then(JsonValue::as_str)
                .map(String::from);
            let example = prop_schema.get("example").cloned();

            rust_properties.push(RustPropertyInfo {
                name: to_snake_case(prop_name),
                rust_type,
                title,
                description,
                example,
            });
        }
    }

    if schema.get("type").and_then(JsonValue::as_str) == Some("array") {
        if let Some(items) = schema.get("items") {
            rust_properties.extend(extract_schema_properties_as_rust(items));
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
                    "Vec<JsonValue>".to_string()
                }
            }
            "object" => "JsonValue".to_string(),
            _ => "String".to_string(),
        }
    } else {
        "String".to_string()
    }
}

fn map_json_schema_to_rust_type(schema: &JsonValue) -> String {
    if let Some(typ) = schema.get("type").and_then(|v| v.as_str()) {
        match typ {
            "string" => "String".to_string(),
            "integer" => "i32".to_string(),
            "boolean" => "bool".to_string(),
            "number" => "f64".to_string(),
            "array" => {
                if let Some(items) = schema.get("items") {
                    format!("Vec<{}>", map_json_schema_to_rust_type(items))
                } else {
                    "Vec<JsonValue>".to_string()
                }
            }
            "object" => "JsonValue".to_string(),
            other => other.to_string(),
        }
    } else {
        "String".to_string()
    }
}

fn extract_properties_schema(op: &Operation) -> JsonMap<String, JsonValue> {
    for response in &op.responses {
        if response.status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema) = json_content.get("schema") {
                        if let Some(properties) = extract_schema_properties_map(schema) {
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

fn extract_schema_properties_map(schema: &JsonValue) -> Option<JsonMap<String, JsonValue>> {
    if let Some(_ref_str) = schema.get("$ref").and_then(JsonValue::as_str) {
        return None;
    }

    if let Some(properties) = schema.get("properties").and_then(JsonValue::as_object) {
        return Some(properties.clone());
    }

    if schema.get("type").and_then(JsonValue::as_str) == Some("array") {
        if let Some(items) = schema.get("items") {
            return extract_schema_properties_map(items);
        }
    }

    None
}

fn is_array_response(op: &Operation) -> bool {
    get_response_schema(op)
        .get("type")
        .and_then(|v| v.as_str()) == Some("array")
}

fn is_object_response(op: &Operation) -> bool {
    let schema = get_response_schema(op);
    schema.get("type").and_then(|v| v.as_str()) == Some("object") ||
    schema.get("properties").is_some()
}

fn is_primitive_response(op: &Operation) -> bool {
    let schema = get_response_schema(op);
    match schema.get("type").and_then(|v| v.as_str()) {
        Some("string") | Some("integer") | Some("number") | Some("boolean") => true,
        _ => false,
    }
}

fn get_array_item_type(op: &Operation) -> String {
    if is_array_response(op) {
        get_response_schema(op)
            .get("items")
            .and_then(|items| items.get("type"))
            .and_then(|v| v.as_str())
            .map(|t| map_openapi_type_to_rust(t))
            .unwrap_or_else(|| "serde_json::Value".to_string())
    } else {
        "serde_json::Value".to_string()
    }
}

fn get_primitive_type(op: &Operation) -> String {
    if is_primitive_response(op) {
        get_response_schema(op)
            .get("type")
            .and_then(|v| v.as_str())
            .map(|t| map_openapi_type_to_rust(t))
            .unwrap_or_else(|| "serde_json::Value".to_string())
    } else {
        "serde_json::Value".to_string()
    }
}

fn extract_properties(op: &Operation) -> Vec<RustPropertyInfo> {
    extract_response_properties(op)
}

fn get_response_schema(op: &Operation) -> &JsonValue {
    // Look for successful response
    for response in &op.responses {
        if response.status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema) = json_content.get("schema") {
                        return schema;
                    }
                }
            }
        }
    }
    &JsonValue::Null
}

fn map_openapi_type_to_rust(openapi_type: &str) -> String {
    match openapi_type {
        "string" => "String",
        "integer" => "i32", 
        "number" => "f64",
        "boolean" => "bool",
        _ => "serde_json::Value",
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::templates::{
        Template, TemplateDescriptor, TemplateManifest, TemplateSource,
    };
    use crate::protocols::{Protocol, Role};

    #[tokio::test]
    async fn test_rust_context_builder() {
        let builder = RustContextBuilder::new();

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);
        context.metadata.project_name = "test_project".to_string();
        context.metadata.version = "1.0.0".to_string();

        let template = Template {
            descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::Rust),
            manifest: TemplateManifest::default(),
            files: vec![],
            source: TemplateSource::Embedded,
        };

        let result = builder.build(&context, &template).await;
        assert!(result.is_ok());

        let render_context = result.unwrap();
        assert_eq!(
            render_context.get_variable("project_name").unwrap(),
            "test_project"
        );
        assert_eq!(
            render_context.get_variable("crate_name").unwrap(),
            "test_project"
        );
        assert_eq!(
            render_context.get_variable("struct_name").unwrap(),
            "TestProject"
        );
    }

    #[tokio::test]
    async fn test_rust_context_builder_wrong_language() {
        let builder = RustContextBuilder::new();

        let context = GenerationContext::new(
            Protocol::Mcp,
            Role::Server,
            Language::Python, // Wrong language
        );

        let template = Template {
            descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::Python),
            manifest: TemplateManifest::default(),
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

        let mut manifest = TemplateManifest::default();
        manifest.name = "test-template".to_string();
        manifest.version = "2.0.0".to_string();
        manifest.description = Some("Test template description".to_string());

        let template = Template {
            descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::Rust),
            manifest,
            files: vec![],
            source: TemplateSource::Embedded,
        };

        let result = builder.build(&context, &template).await;
        assert!(result.is_ok());

        let render_context = result.unwrap();
        assert_eq!(
            render_context.get_variable("template_name").unwrap(),
            "test-template"
        );
        assert_eq!(
            render_context.get_variable("template_version").unwrap(),
            "2.0.0"
        );
        assert_eq!(
            render_context.get_variable("template_description").unwrap(),
            "Test template description"
        );
    }
}
