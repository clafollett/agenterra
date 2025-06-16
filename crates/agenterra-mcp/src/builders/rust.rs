//! Rust-specific endpoint context builder for Agenterra codegen.
//!
//! This module provides the Rust language implementation of the `EndpointContextBuilder` trait,
//! converting OpenAPI operations into Rust-specific contexts suitable for generating idiomatic
//! Rust code using frameworks like Axum.
//!
//! The builder handles:
//! - Converting OpenAPI identifiers to Rust naming conventions (snake_case, PascalCase)
//! - Mapping OpenAPI types to Rust types (string -> String, integer -> i32, etc.)
//! - Organizing parameters and responses into Rust-appropriate structures
//! - Generating type names for structs, enums, and functions

use super::EndpointContextBuilder;
use crate::templates::{ParameterKind, TemplateParameterInfo};
use agenterra_core::openapi::OpenApiOperation;
use agenterra_core::utils::{to_snake_case, to_upper_camel_case};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};

/// Type alias for Rust-specific parameter info.
///
/// This reuses the generic `TemplateParameterInfo` structure but provides a more
/// specific name in the Rust context for clarity.
pub type RustParameterInfo = TemplateParameterInfo;

/// Rust-specific property information with type mapping.
///
/// Extends the basic OpenAPI property information with Rust-specific type information,
/// allowing templates to generate properly typed Rust code.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RustPropertyInfo {
    /// The property name in snake_case format
    pub name: String,
    /// The corresponding Rust type (e.g., "String", "i32", "bool")
    pub rust_type: String,
    /// Optional title from the OpenAPI schema
    pub title: Option<String>,
    /// Optional description from the OpenAPI schema
    pub description: Option<String>,
    /// Optional example value from the OpenAPI schema
    pub example: Option<JsonValue>,
}

/// Complete Rust-specific context for code generation.
///
/// This struct contains all the information needed to generate idiomatic Rust code
/// for a single OpenAPI endpoint, including proper naming conventions, type mappings,
/// and structured data for template rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustEndpointContext {
    /// Identifier for the endpoint (path with slashes replaced by '_')
    pub endpoint: String,
    /// Uppercase form of the endpoint for type names
    pub endpoint_cap: String,
    /// Sanitized endpoint name for file system use
    pub endpoint_fs: String,
    /// Raw path as defined in the OpenAPI spec (e.g., "/pet/{petId}")
    pub path: String,
    /// Name of the generated function for the endpoint
    pub fn_name: String,
    /// Name of the generated parameters struct (e.g., 'users_params')
    pub parameters_type: String,
    /// Name of the generated properties struct
    pub properties_type: String,
    /// Name of the generated response struct
    pub response_type: String,
    /// Raw JSON object representing the response schema properties
    pub envelope_properties: JsonValue,
    /// Typed response property information
    pub properties: Vec<RustPropertyInfo>,
    /// Names of properties to pass into handler functions
    pub properties_for_handler: Vec<String>,
    /// Typed list of parameters for the endpoint
    pub parameters: Vec<TemplateParameterInfo>,
    /// Summary of the endpoint
    pub summary: String,
    /// Description of the endpoint
    pub description: String,
    /// Tags associated with the endpoint
    pub tags: Vec<String>,
    /// Schema reference for the properties
    pub properties_schema: JsonMap<String, JsonValue>,
    /// Schema reference for the response
    pub response_schema: JsonValue,
    /// Name of the spec file (if loaded from a file)
    pub spec_file_name: Option<String>,
    /// Valid fields for the endpoint
    pub valid_fields: Vec<String>,
}

/// Builder for creating Rust-specific endpoint contexts.
///
/// This builder implements the `EndpointContextBuilder` trait to convert OpenAPI operations
/// into contexts suitable for generating Rust code with appropriate naming conventions
/// and type mappings.
#[derive(Debug, Clone)]
pub struct RustEndpointContextBuilder;

impl EndpointContextBuilder for RustEndpointContextBuilder {
    fn build(&self, op: &OpenApiOperation) -> agenterra_core::Result<JsonValue> {
        let context = RustEndpointContext {
            fn_name: to_snake_case(&op.id),
            parameters_type: to_upper_camel_case(&format!("{}_params", op.id)),
            endpoint: to_snake_case(&op.id),
            endpoint_cap: to_upper_camel_case(&op.id),
            endpoint_fs: to_snake_case(&op.id),
            path: op.path.clone(),
            properties_type: to_upper_camel_case(&format!("{}_properties", op.id)),
            response_type: to_upper_camel_case(&format!("{}_response", op.id)),
            envelope_properties: extract_envelope_properties(op),
            properties: extract_response_properties(op),
            properties_for_handler: extract_handler_properties(op),
            parameters: op
                .parameters
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|p| TemplateParameterInfo {
                    name: p.name,
                    target_type: map_openapi_schema_to_rust_type(p.schema.as_ref()),
                    description: p.description,
                    example: p.example,
                    kind: match p.in_.as_str() {
                        "path" => ParameterKind::Path,
                        "query" => ParameterKind::Query,
                        "header" => ParameterKind::Header,
                        "cookie" => ParameterKind::Cookie,
                        _ => ParameterKind::Query, // Safe default
                    },
                })
                .collect(),
            summary: op.summary.clone().unwrap_or_default(),
            description: op.description.clone().unwrap_or_default(),
            tags: op.tags.clone().unwrap_or_default(),
            properties_schema: extract_properties_schema(op),
            response_schema: extract_response_schema(op),
            spec_file_name: extract_spec_file_name(op),
            valid_fields: extract_valid_fields(op),
        };

        // Convert to JSON
        Ok(serde_json::to_value(&context)?)
    }
}

/// Maps OpenAPI schema types to appropriate Rust types.
///
/// This function converts OpenAPI type definitions into their Rust equivalents,
/// providing sensible defaults for cases where type information is missing or ambiguous.
///
/// # Arguments
/// * `schema` - Optional reference to the OpenAPI schema JSON value
///
/// # Returns
/// A String representing the appropriate Rust type
///
/// # Type Mappings
/// - `string` → `String`
/// - `integer` → `i32`
/// - `boolean` → `bool`
/// - `number` → `f64`
/// - Unknown/missing types → `String` (safe default)
///
fn map_openapi_schema_to_rust_type(schema: Option<&JsonValue>) -> String {
    if let Some(sch) = schema {
        if let Some(typ) = sch.get("type").and_then(|v| v.as_str()) {
            match typ {
                "string" => "String".to_string(),
                "integer" => "i32".to_string(),
                "boolean" => "bool".to_string(),
                "number" => "f64".to_string(),
                other => other.to_string(),
            }
        } else {
            "String".to_string()
        }
    } else {
        "String".to_string()
    }
}

/// Extracts envelope properties from OpenAPI operation responses
fn extract_envelope_properties(op: &OpenApiOperation) -> JsonValue {
    // Look for successful response (200, 201, etc.)
    for (status_code, response) in &op.responses {
        if status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema) = json_content.get("schema") {
                        return extract_schema_envelope_properties(schema);
                    }
                }
            }
        }
    }
    serde_json::json!({})
}

/// Extracts properties from schema and maps them to RustPropertyInfo
fn extract_response_properties(op: &OpenApiOperation) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    // Look for successful response (200, 201, etc.)
    for (status_code, response) in &op.responses {
        if status_code.starts_with('2') {
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

/// Extracts property names for handler functions
fn extract_handler_properties(op: &OpenApiOperation) -> Vec<String> {
    extract_response_properties(op)
        .into_iter()
        .map(|prop| prop.name)
        .collect()
}

/// Helper to extract envelope properties from a schema
fn extract_schema_envelope_properties(schema: &JsonValue) -> JsonValue {
    // Handle $ref references
    if let Some(_ref_str) = schema.get("$ref").and_then(JsonValue::as_str) {
        // For now, return empty object for $ref schemas
        // In a full implementation, we'd resolve the reference
        return serde_json::json!({});
    }

    // Handle direct properties
    if let Some(properties) = schema.get("properties") {
        return properties.clone();
    }

    // Handle array responses
    if schema.get("type").and_then(JsonValue::as_str) == Some("array") {
        if let Some(items) = schema.get("items") {
            return extract_schema_envelope_properties(items);
        }
    }

    serde_json::json!({})
}

/// Helper to extract schema properties and convert to RustPropertyInfo
fn extract_schema_properties_as_rust(schema: &JsonValue) -> Vec<RustPropertyInfo> {
    let mut rust_properties = Vec::new();

    // Handle $ref references
    if let Some(_ref_str) = schema.get("$ref").and_then(JsonValue::as_str) {
        // For now, return empty for $ref schemas
        // In a full implementation, we'd resolve the reference
        return rust_properties;
    }

    // Handle direct properties
    if let Some(properties) = schema.get("properties").and_then(JsonValue::as_object) {
        for (prop_name, prop_schema) in properties {
            let rust_type = map_openapi_schema_to_rust_type(Some(prop_schema));
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

    // Handle array responses - extract properties from items
    if schema.get("type").and_then(JsonValue::as_str) == Some("array") {
        if let Some(items) = schema.get("items") {
            rust_properties.extend(extract_schema_properties_as_rust(items));
        }
    }

    rust_properties
}

/// Extracts properties schema as a JSON Map from OpenAPI operation responses
fn extract_properties_schema(op: &OpenApiOperation) -> JsonMap<String, JsonValue> {
    // Look for successful response (200, 201, etc.)
    for (status_code, response) in &op.responses {
        if status_code.starts_with('2') {
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

/// Extracts response schema from OpenAPI operation responses  
fn extract_response_schema(op: &OpenApiOperation) -> JsonValue {
    // Look for successful response (200, 201, etc.)
    for (status_code, response) in &op.responses {
        if status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema) = json_content.get("schema") {
                        return schema.clone();
                    }
                }
            }
        }
    }
    serde_json::json!({})
}

/// Extracts spec file name from operation (currently not available in operation context)
fn extract_spec_file_name(_op: &OpenApiOperation) -> Option<String> {
    // The OpenApiOperation doesn't contain file name information
    // This would need to be passed in from the calling context
    None
}

/// Extracts valid field names from operation responses
fn extract_valid_fields(op: &OpenApiOperation) -> Vec<String> {
    extract_response_properties(op)
        .into_iter()
        .map(|prop| prop.name)
        .collect()
}

/// Helper to extract properties as a JSON Map from a schema
fn extract_schema_properties_map(schema: &JsonValue) -> Option<JsonMap<String, JsonValue>> {
    // Handle $ref references
    if let Some(_ref_str) = schema.get("$ref").and_then(JsonValue::as_str) {
        // For now, return None for $ref schemas
        // In a full implementation, we'd resolve the reference
        return None;
    }

    // Handle direct properties
    if let Some(properties) = schema.get("properties").and_then(JsonValue::as_object) {
        return Some(properties.clone());
    }

    // Handle array responses - extract properties from items
    if schema.get("type").and_then(JsonValue::as_str) == Some("array") {
        if let Some(items) = schema.get("items") {
            return extract_schema_properties_map(items);
        }
    }

    None
}
