//! OpenAPI/Swagger specification types
//! 
//! This module contains types for representing OpenAPI 2.0 (Swagger) and OpenAPI 3.0+ specifications.
//! These types are designed to handle both versions with proper schema resolution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenAPI specification version
#[derive(Debug, Clone, PartialEq)]
pub enum OpenApiVersion {
    V2_0,
    V3_0,
    V3_1,
}

impl Default for OpenApiVersion {
    fn default() -> Self {
        OpenApiVersion::V3_0
    }
}

impl OpenApiVersion {
    pub fn from_str(version: &str) -> Option<Self> {
        match version {
            "2.0" => Some(OpenApiVersion::V2_0),
            s if s.starts_with("3.0") => Some(OpenApiVersion::V3_0),
            s if s.starts_with("3.1") => Some(OpenApiVersion::V3_1),
            _ => None,
        }
    }
}

/// OpenAPI specification (works for both 2.0 and 3.0+)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    /// Version of the OpenAPI spec (2.0 for Swagger, 3.0.x or 3.1.x for OpenAPI)
    #[serde(skip)]
    pub version: OpenApiVersion,
    
    /// The raw version string from the spec
    pub version_string: String,
    
    /// API metadata
    pub info: ApiInfo,
    
    /// Available servers (OpenAPI 3.0+) or host/basePath/schemes (Swagger 2.0)
    pub servers: Vec<Server>,
    
    /// API operations
    pub operations: Vec<Operation>,
    
    /// Components/definitions
    pub components: Option<Components>,
}

/// API information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
    pub terms_of_service: Option<String>,
    pub contact: Option<Contact>,
    pub license: Option<License>,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: Option<String>,
    pub url: Option<String>,
    pub email: Option<String>,
}

/// License information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub name: String,
    pub url: Option<String>,
}

/// Server definition (OpenAPI 3.0+) or derived from host/basePath (Swagger 2.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
    pub variables: Option<HashMap<String, ServerVariable>>,
}

/// Server variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerVariable {
    pub default: String,
    pub description: Option<String>,
    pub enum_values: Option<Vec<String>>,
}

/// API Operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique operation ID
    #[serde(rename = "operationId")]
    pub id: String,
    
    /// Path template
    pub path: String,
    
    /// HTTP method
    pub method: String,
    
    /// Tags for grouping
    pub tags: Option<Vec<String>>,
    
    /// Short summary
    pub summary: Option<String>,
    
    /// Detailed description
    pub description: Option<String>,
    
    /// External documentation
    #[serde(rename = "externalDocs")]
    pub external_docs: Option<ExternalDocs>,
    
    /// Operation parameters
    pub parameters: Vec<Parameter>,
    
    /// Request body (OpenAPI 3.0+) or body parameter (Swagger 2.0)
    pub request_body: Option<RequestBody>,
    
    /// Possible responses
    pub responses: HashMap<String, Response>,
    
    /// Callbacks (OpenAPI 3.0+ only)
    pub callbacks: Option<HashMap<String, serde_json::Value>>,
    
    /// Deprecation flag
    pub deprecated: Option<bool>,
    
    /// Security requirements
    pub security: Option<Vec<HashMap<String, Vec<String>>>>,
    
    /// Servers (OpenAPI 3.0+ only)
    pub servers: Option<Vec<Server>>,
}

/// External documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocs {
    pub url: String,
    pub description: Option<String>,
}

/// Parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: ParameterLocation,
    pub description: Option<String>,
    pub required: bool,
    pub deprecated: Option<bool>,
    pub allow_empty_value: Option<bool>,
    
    // Schema can be inline or a reference
    #[serde(flatten)]
    pub schema: ParameterSchema,
}

/// Parameter location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Query,
    Header,
    Path,
    Cookie,
    // For Swagger 2.0
    #[serde(rename = "formData")]
    FormData,
    Body,
}

/// Parameter schema (handles both inline and $ref)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterSchema {
    /// Direct schema
    Schema(Schema),
    /// Reference to a schema
    Ref {
        #[serde(rename = "$ref")]
        reference: String,
    },
}

/// Request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub description: Option<String>,
    pub required: bool,
    pub content: HashMap<String, MediaType>,
}

/// Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub description: String,
    pub headers: Option<HashMap<String, Header>>,
    pub content: Option<HashMap<String, MediaType>>,
    pub links: Option<HashMap<String, serde_json::Value>>,
}

/// Header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub description: Option<String>,
    pub required: Option<bool>,
    pub deprecated: Option<bool>,
    pub schema: Schema,
}

/// Media type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    pub schema: SchemaOrRef,
    pub example: Option<serde_json::Value>,
    pub examples: Option<HashMap<String, Example>>,
    pub encoding: Option<HashMap<String, Encoding>>,
}

/// Example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub value: Option<serde_json::Value>,
    #[serde(rename = "externalValue")]
    pub external_value: Option<String>,
}

/// Encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encoding {
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    pub headers: Option<HashMap<String, Header>>,
    pub style: Option<String>,
    pub explode: Option<bool>,
    #[serde(rename = "allowReserved")]
    pub allow_reserved: Option<bool>,
}

/// Schema or reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaOrRef {
    /// Direct schema
    Schema(Schema),
    /// Reference to a schema
    Ref {
        #[serde(rename = "$ref")]
        reference: String,
    },
}

/// Schema object
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schema {
    // Basic type info
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub format: Option<String>,
    
    // Common properties
    pub title: Option<String>,
    pub description: Option<String>,
    pub default: Option<serde_json::Value>,
    pub example: Option<serde_json::Value>,
    pub deprecated: Option<bool>,
    #[serde(rename = "readOnly")]
    pub read_only: Option<bool>,
    #[serde(rename = "writeOnly")]
    pub write_only: Option<bool>,
    
    // Numeric constraints
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    #[serde(rename = "exclusiveMinimum")]
    pub exclusive_minimum: Option<f64>,
    #[serde(rename = "exclusiveMaximum")]
    pub exclusive_maximum: Option<f64>,
    #[serde(rename = "multipleOf")]
    pub multiple_of: Option<f64>,
    
    // String constraints
    #[serde(rename = "minLength")]
    pub min_length: Option<usize>,
    #[serde(rename = "maxLength")]
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    
    // Array constraints
    #[serde(rename = "minItems")]
    pub min_items: Option<usize>,
    #[serde(rename = "maxItems")]
    pub max_items: Option<usize>,
    #[serde(rename = "uniqueItems")]
    pub unique_items: Option<bool>,
    pub items: Option<Box<SchemaOrRef>>,
    
    // Object constraints
    pub required: Option<Vec<String>>,
    pub properties: Option<HashMap<String, SchemaOrRef>>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<Box<SchemaOrRef>>,
    #[serde(rename = "minProperties")]
    pub min_properties: Option<usize>,
    #[serde(rename = "maxProperties")]
    pub max_properties: Option<usize>,
    
    // Enumeration
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    
    // Composition
    #[serde(rename = "allOf")]
    pub all_of: Option<Vec<SchemaOrRef>>,
    #[serde(rename = "anyOf")]
    pub any_of: Option<Vec<SchemaOrRef>>,
    #[serde(rename = "oneOf")]
    pub one_of: Option<Vec<SchemaOrRef>>,
    pub not: Option<Box<SchemaOrRef>>,
    
    // OpenAPI specific
    pub discriminator: Option<Discriminator>,
    pub xml: Option<serde_json::Value>,
    #[serde(rename = "externalDocs")]
    pub external_docs: Option<ExternalDocs>,
    
    // Extensions
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Discriminator for polymorphism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discriminator {
    #[serde(rename = "propertyName")]
    pub property_name: String,
    pub mapping: Option<HashMap<String, String>>,
}

/// Components (OpenAPI 3.0+) or Definitions (Swagger 2.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub schemas: Option<HashMap<String, SchemaOrRef>>,
    pub responses: Option<HashMap<String, Response>>,
    pub parameters: Option<HashMap<String, Parameter>>,
    pub examples: Option<HashMap<String, Example>>,
    #[serde(rename = "requestBodies")]
    pub request_bodies: Option<HashMap<String, RequestBody>>,
    pub headers: Option<HashMap<String, Header>>,
    #[serde(rename = "securitySchemes")]
    pub security_schemes: Option<HashMap<String, serde_json::Value>>,
    pub links: Option<HashMap<String, serde_json::Value>>,
    pub callbacks: Option<HashMap<String, serde_json::Value>>,
}

impl Schema {
    /// Check if this schema represents a primitive type
    pub fn is_primitive(&self) -> bool {
        matches!(
            self.schema_type.as_deref(),
            Some("string") | Some("number") | Some("integer") | Some("boolean")
        )
    }
    
    /// Check if this schema represents an array
    pub fn is_array(&self) -> bool {
        self.schema_type.as_deref() == Some("array")
    }
    
    /// Check if this schema represents an object
    pub fn is_object(&self) -> bool {
        self.schema_type.as_deref() == Some("object") || self.properties.is_some()
    }
}