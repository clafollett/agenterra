//! OpenAPI types matching the existing generation types
//! This is a temporary compatibility layer during migration

use serde::{Deserialize, Serialize};

/// OpenAPI operation representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique string used to identify the operation
    #[serde(rename = "operationId")]
    pub id: String,
    /// The path where this operation is defined (e.g., "/pet/findByStatus")
    pub path: String,
    /// The HTTP method for this operation
    pub method: String,
    /// A list of tags for API documentation control
    pub tags: Option<Vec<String>>,
    /// A short summary of what the operation does
    pub summary: Option<String>,
    /// A verbose explanation of the operation behavior
    pub description: Option<String>,
    /// Additional external documentation for this operation
    #[serde(rename = "externalDocs")]
    pub external_docs: Option<serde_json::Value>,
    /// A list of parameters that are applicable for this operation
    pub parameters: Vec<Parameter>,
    /// The request body applicable for this operation
    pub request_body: Option<RequestBody>,
    /// The list of possible responses
    pub responses: Vec<Response>,
    /// A map of possible out-of band callbacks related to the parent operation
    pub callbacks: Option<serde_json::Value>,
    /// Declares this operation to be deprecated
    pub deprecated: Option<bool>,
    /// A declaration of which security mechanisms can be used for this operation
    pub security: Option<Vec<serde_json::Value>>,
    /// An alternative server array to service this operation
    pub servers: Option<Vec<serde_json::Value>>,
    /// Specification extensions (fields starting with `x-`)
    #[serde(flatten)]
    pub vendor_extensions: std::collections::HashMap<String, serde_json::Value>,
}

/// Operation parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub schema: Schema,
    pub description: Option<String>,
}

/// Parameter location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

/// Request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub required: bool,
    pub content: serde_json::Value,
    pub description: Option<String>,
}

/// Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status_code: String,
    pub description: String,
    pub content: Option<serde_json::Value>,
}

/// Schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub format: Option<String>,
    pub items: Option<Box<Schema>>,
    pub properties: Option<std::collections::HashMap<String, Schema>>,
    pub required: Option<Vec<String>>,
    // Additional OpenAPI schema fields
    pub description: Option<String>,
    pub title: Option<String>,
    pub default: Option<serde_json::Value>,
    pub example: Option<serde_json::Value>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub unique_items: Option<bool>,
    pub additional_properties: Option<Box<AdditionalProperties>>,
    pub all_of: Option<Vec<Schema>>,
    pub one_of: Option<Vec<Schema>>,
    pub any_of: Option<Vec<Schema>>,
    pub not: Option<Box<Schema>>,
    pub discriminator: Option<Discriminator>,
    pub read_only: Option<bool>,
    pub write_only: Option<bool>,
    pub xml: Option<serde_json::Value>,
    pub external_docs: Option<ExternalDocs>,
    pub deprecated: Option<bool>,
    pub nullable: Option<bool>,
}

/// Additional properties specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Boolean(bool),
    Schema(Box<Schema>),
}

/// Discriminator for polymorphism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discriminator {
    pub property_name: String,
    pub mapping: Option<std::collections::HashMap<String, String>>,
}

/// External documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocs {
    pub url: String,
    pub description: Option<String>,
}

/// OpenAPI specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiContext {
    pub version: String,
    pub info: ApiInfo,
    pub servers: Vec<Server>,
    pub operations: Vec<Operation>,
    pub components: Option<Components>,
}

/// API information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

/// Server definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
}

/// Components section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub schemas: serde_json::Value,
}
