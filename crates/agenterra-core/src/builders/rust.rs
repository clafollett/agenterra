//! Rust-specific endpoint context builder for Agenterra codegen.

use super::EndpointContextBuilder;
use crate::openapi::OpenApiOperation;
use crate::templates::{ParameterKind, TemplateParameterInfo};
use crate::utils::{to_snake_case, to_upper_camel_case};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};

// Type alias for Rust-specific parameter info
pub type RustParameterInfo = TemplateParameterInfo;

/// Rust-specific property info (adds rust_type to OpenAPI property)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RustPropertyInfo {
    pub name: String,
    pub rust_type: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub example: Option<JsonValue>,
}

// Rust-specific context for codegen
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

#[derive(Debug, Clone)]
pub struct RustEndpointContextBuilder;

impl EndpointContextBuilder for RustEndpointContextBuilder {
    fn build(&self, op: &OpenApiOperation) -> crate::Result<JsonValue> {
        let context = RustEndpointContext {
            fn_name: to_snake_case(&op.id),
            parameters_type: to_upper_camel_case(&format!("{}_params", op.id)),
            endpoint: to_snake_case(&op.id),
            endpoint_cap: to_upper_camel_case(&op.id),
            endpoint_fs: to_snake_case(&op.id),
            path: op.path.clone(),
            properties_type: to_upper_camel_case(&format!("{}_properties", op.id)),
            response_type: to_upper_camel_case(&format!("{}_response", op.id)),
            envelope_properties: serde_json::json!({}), // TODO: Extract from op.responses if present
            properties: vec![], // TODO: Extract properties from op.responses and map to RustPropertyInfo
            properties_for_handler: vec![],
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
            properties_schema: serde_json::Map::new(), // TODO: Extract from op.responses
            response_schema: serde_json::json!({}),    // TODO: Extract from op.responses
            spec_file_name: None,                      // TODO: Set if available
            valid_fields: vec![],                      // TODO: Populate with valid fields
        };

        // Convert to JSON
        Ok(serde_json::to_value(&context)?)
    }
}

// Helper to map OpenAPI schema to Rust type
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
