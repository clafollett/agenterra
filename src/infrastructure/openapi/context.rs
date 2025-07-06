//! OpenAPI specification parsing and utilities.
//!
//! This module provides functionality for loading and querying OpenAPI specifications.
//! It supports loading from files and provides convenient accessors for common fields.
//!
//! # Examples
//!
//! ```no_run
//! use agenterra_core::openapi::OpenApiContext;
//! use agenterra_core::error::Result;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! // Load an OpenAPI spec from a file
//! let spec = OpenApiContext::from_file("openapi.json").await?;
//!
//! // Access common fields
//! if let Some(title) = spec.title() {
//!     println!("API Title: {}", title);
//! }
//! if let Some(version) = spec.version() {
//!     println!("API Version: {}", version);
//! }
//! # Ok(())
//! # }
//! ```

// Internal imports (std, crate)
use std::path::Path;

use crate::generation::GenerationError;

// External imports (alphabetized)
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::fmt;
use std::str::FromStr;
use tokio::fs;

/// HTTP methods supported by OpenAPI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    /// Get all HTTP methods as an array
    pub fn all() -> &'static [HttpMethod] {
        &[
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Delete,
            HttpMethod::Patch,
            HttpMethod::Head,
            HttpMethod::Options,
        ]
    }

    /// Get the lowercase string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "get",
            HttpMethod::Post => "post",
            HttpMethod::Put => "put",
            HttpMethod::Delete => "delete",
            HttpMethod::Patch => "patch",
            HttpMethod::Head => "head",
            HttpMethod::Options => "options",
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for HttpMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "get" => Ok(HttpMethod::Get),
            "post" => Ok(HttpMethod::Post),
            "put" => Ok(HttpMethod::Put),
            "delete" => Ok(HttpMethod::Delete),
            "patch" => Ok(HttpMethod::Patch),
            "head" => Ok(HttpMethod::Head),
            "options" => Ok(HttpMethod::Options),
            _ => Err(format!("Invalid HTTP method: {s}")),
        }
    }
}

/// Represents an OpenAPI specification
#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
pub struct OpenApiContext {
    /// The raw JSON value of the OpenAPI spec
    pub json: JsonValue,
}

impl OpenApiContext {
    /// Create a new OpenAPISpec from a file or URL (supports both YAML and JSON)
    pub async fn from_file_or_url<P: AsRef<str>>(location: P) -> Result<Self, GenerationError> {
        let location = location.as_ref();

        // Check if the input looks like a URL
        if location.starts_with("http://") || location.starts_with("https://") {
            return Self::from_url(location).await;
        }

        // Otherwise treat as a file path
        Self::from_file(location).await
    }

    /// Create a new OpenAPISpec from a file (supports both YAML and JSON)
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, GenerationError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| GenerationError::IoError(e))?;
        Self::parse_content(&content).map_err(|e| {
            GenerationError::LoadError(format!(
                "Failed to parse OpenAPI spec at {}: {}",
                path.display(),
                e
            ))
        })
    }

    /// Create a new OpenAPISpec from a URL (supports both YAML and JSON)
    async fn from_url(url: &str) -> Result<Self, GenerationError> {
        let response = reqwest::get(url).await.map_err(|e| {
            GenerationError::LoadError(format!("Failed to fetch OpenAPI spec from {url}: {e}"))
        })?;

        if !response.status().is_success() {
            return Err(GenerationError::LoadError(format!(
                "Failed to fetch OpenAPI spec from {}: HTTP {}",
                url,
                response.status()
            )));
        }

        let content = response.text().await.map_err(|e| {
            GenerationError::LoadError(format!("Failed to read response from {url}: {e}"))
        })?;

        Self::parse_content(&content).map_err(|e| {
            GenerationError::LoadError(format!("Failed to parse OpenAPI spec from {url}: {e}"))
        })
    }

    /// Parse content as either JSON or YAML
    fn parse_content(content: &str) -> Result<Self, String> {
        // Try to parse as JSON first
        if let Ok(json) = serde_json::from_str(content) {
            return Ok(Self { json });
        }

        // If JSON parsing fails, try YAML
        if let Ok(json) = serde_yaml::from_str(content) {
            return Ok(Self { json });
        }

        // If both parsers fail, return an error
        Err("content is neither valid JSON nor YAML".to_string())
    }
    /// Get the title of the API
    pub fn title(&self) -> Option<&str> {
        self.json.get("info")?.get("title")?.as_str()
    }

    /// Get the version of the API
    pub fn version(&self) -> Option<&str> {
        self.json.get("info")?.get("version")?.as_str()
    }

    /// Get the base path of the API
    pub fn base_path(&self) -> Option<String> {
        // Try OpenAPI 3.0+ servers format first
        if let Some(servers) = self.json.get("servers").and_then(|s| s.as_array()) {
            if let Some(server) = servers.first() {
                if let Some(url) = server.get("url").and_then(|u| u.as_str()) {
                    return Some(url.to_string());
                }
            }
        }

        // Fall back to Swagger 2.0 host + basePath format
        if let (Some(host), base_path) = (
            self.json.get("host").and_then(|h| h.as_str()),
            self.json
                .get("basePath")
                .and_then(|bp| bp.as_str())
                .unwrap_or(""),
        ) {
            // Determine protocol - Swagger 2.0 specs can specify schemes
            let scheme = if let Some(schemes) = self.json.get("schemes").and_then(|s| s.as_array())
            {
                // Use the first scheme, prefer https if available
                if schemes.iter().any(|s| s.as_str() == Some("https")) {
                    "https"
                } else {
                    schemes.first().and_then(|s| s.as_str()).unwrap_or("https")
                }
            } else {
                "https" // Default to https if no schemes specified
            };

            return Some(format!("{scheme}://{host}{base_path}"));
        }

        None
    }

    /// Parse all endpoints into structured contexts for template rendering
    pub async fn parse_operations(&self) -> Result<Vec<OpenApiOperation>, GenerationError> {
        // Get paths object
        let paths = self
            .json
            .get("paths")
            .and_then(JsonValue::as_object)
            .ok_or_else(|| GenerationError::LoadError("Missing 'paths' object".to_string()))?;

        // Use iterator combinators to flatten and map operations
        let operations = paths
            .iter()
            .flat_map(|(path, path_item)| {
                HttpMethod::all()
                    .iter()
                    .filter_map(|method| {
                        path_item
                            .get(method.as_str())
                            .and_then(JsonValue::as_object)
                            .map(|method_item| (path, method, path_item, method_item))
                    })
                    .collect::<Vec<_>>()
            })
            .map(|(path, method, path_item, method_item)| {
                self.build_operation(path, method, path_item, method_item)
            })
            .collect();

        Ok(operations)
    }

    /// Build an OpenApiOperation from path, method, and method item
    fn build_operation(
        &self,
        path: &str,
        method: &HttpMethod,
        path_item: &JsonValue,
        method_item: &serde_json::Map<String, JsonValue>,
    ) -> OpenApiOperation {
        let operation_id = method_item
            .get("operationId")
            .and_then(JsonValue::as_str)
            .map(String::from)
            .unwrap_or_else(|| {
                format!(
                    "{}_{}",
                    method,
                    path.trim_start_matches('/').replace('/', "_")
                )
            });

        let summary = method_item
            .get("summary")
            .and_then(JsonValue::as_str)
            .map(String::from);
        let description = method_item
            .get("description")
            .and_then(JsonValue::as_str)
            .map(String::from);
        let external_docs = method_item.get("externalDocs").cloned();
        let parameters = self.extract_parameters(path_item);
        let request_body = method_item.get("requestBody").cloned();
        let responses = self.extract_responses(method_item);
        let callbacks = method_item.get("callbacks").cloned();
        let deprecated = method_item.get("deprecated").and_then(JsonValue::as_bool);
        let security = method_item
            .get("security")
            .and_then(JsonValue::as_array)
            .cloned();
        let servers = method_item
            .get("servers")
            .and_then(JsonValue::as_array)
            .cloned();
        let tags = method_item
            .get("tags")
            .and_then(JsonValue::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(JsonValue::as_str)
                    .map(String::from)
                    .collect()
            });
        let vendor_extensions = self.extract_vendor_extensions(method_item);

        OpenApiOperation {
            id: operation_id,
            path: path.to_string(),
            method: method.clone(),
            summary,
            description,
            external_docs,
            parameters,
            request_body,
            responses,
            callbacks,
            deprecated,
            security,
            servers,
            tags,
            vendor_extensions,
        }
    }

    /// Extracts parameters from an OpenAPI path item, resolving any $ref references
    fn extract_parameters(&self, path_item: &JsonValue) -> Option<Vec<OpenApiParameter>> {
        path_item
            .get("parameters")
            .and_then(JsonValue::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|param| {
                        if let Some(ref_str) = param.get("$ref").and_then(JsonValue::as_str) {
                            self.json
                                .pointer(&ref_str[1..])
                                .and_then(|p| serde_json::from_value(p.clone()).ok())
                        } else {
                            serde_json::from_value(param.clone()).ok()
                        }
                    })
                    .collect::<Vec<OpenApiParameter>>()
            })
    }

    /// Extracts response definitions from an OpenAPI operation
    fn extract_responses(
        &self,
        get_item: &serde_json::Map<String, JsonValue>,
    ) -> std::collections::HashMap<String, OpenApiResponse> {
        get_item
            .get("responses")
            .and_then(JsonValue::as_object)
            .map(|map| {
                map.iter()
                    .filter_map(|(k, v)| {
                        serde_json::from_value(v.clone())
                            .ok()
                            .map(|resp| (k.clone(), resp))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extracts vendor extensions (x-* prefixed properties) from an OpenAPI operation
    fn extract_vendor_extensions(
        &self,
        get_item: &serde_json::Map<String, JsonValue>,
    ) -> std::collections::HashMap<String, JsonValue> {
        get_item
            .iter()
            .filter(|(k, _)| k.starts_with("x-"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Extract row properties from properties JSON
    pub fn extract_row_properties(properties_json: &JsonValue) -> Vec<JsonValue> {
        if let Some(data) = properties_json.get("data").and_then(JsonValue::as_object) {
            if let Some(props) = data.get("properties").and_then(JsonValue::as_object) {
                return props
                    .iter()
                    .map(|(k, v)| json!({"name": k, "schema": v}))
                    .collect();
            }
        }
        if let Some(props) = properties_json.as_object() {
            return props
                .iter()
                .map(|(k, v)| json!({"name": k, "schema": v}))
                .collect();
        }
        Vec::new()
    }

    /// Extract typed property info from properties JSON
    pub fn extract_property_info(properties_json: &JsonValue) -> Vec<OpenApiPropertyInfo> {
        OpenApiContext::extract_row_properties(properties_json)
            .into_iter()
            .map(|prop| {
                let name = prop
                    .get("name")
                    .and_then(JsonValue::as_str)
                    .unwrap_or_default()
                    .to_string();
                let schema = prop.get("schema");
                let title = schema
                    .and_then(|s| s.get("title"))
                    .and_then(JsonValue::as_str)
                    .map(String::from);
                let description = schema
                    .and_then(|s| s.get("description"))
                    .and_then(JsonValue::as_str)
                    .map(String::from);
                let example = schema.and_then(|s| s.get("example")).cloned();
                OpenApiPropertyInfo {
                    name: name.clone(),
                    title,
                    description,
                    example,
                }
            })
            .collect()
    }

    /// Sanitize a string to be safe for use as a filename across all operating systems
    #[allow(dead_code)]
    fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Sanitize endpoint names to be valid Rust module identifiers
    #[allow(dead_code)]
    fn sanitize_endpoint_name(endpoint: &str) -> String {
        // Replace any characters that aren't valid in Rust identifiers
        let mut result = String::new();

        // First, replace common problematic patterns
        let s = endpoint.replace("{", "").replace("}", "");
        let s = s.replace("/", "_").replace("-", "_");

        // Then ensure we only have valid Rust identifier characters
        for c in s.chars() {
            if c.is_alphanumeric() || c == '_' {
                result.push(c);
            } else {
                result.push('_');
            }
        }

        // Ensure it starts with a letter or underscore (valid Rust identifier)
        if !result.is_empty()
            && !result.chars().next().unwrap().is_alphabetic()
            && !result.starts_with('_')
        {
            result = format!("m_{result}");
        }

        // Handle empty string case
        if result.is_empty() {
            result = "root".to_string();
        }

        result
    }

    /// Sanitizes Markdown for Rust doc comments and Swagger UI.
    #[allow(dead_code)]
    pub fn sanitize_markdown(input: &str) -> String {
        use regex::Regex;
        // Regex for problematic Unicode (e.g., smart quotes, em-dash)
        let unicode_re = Regex::new(r"[\u2018\u2019\u201C\u201D\u2014]").unwrap();
        // Regex to collapse any whitespace sequence into a single space
        let ws_re = Regex::new(r"\s+").unwrap();
        input
            .lines()
            .map(|line| {
                let mut line = line.replace('\t', " ");
                // Remove problematic Unicode
                line = unicode_re
                    .replace_all(&line, |caps: &regex::Captures| match &caps[0] {
                        "\u{2018}" | "\u{2019}" => "'",
                        "\u{201C}" | "\u{201D}" => "\"",
                        "\u{2014}" => "-",
                        _ => "",
                    })
                    .to_string();
                // Trim edges and collapse inner whitespace
                let mut trimmed = ws_re.replace_all(line.trim(), " ").to_string();
                // Remove spaces around hyphens
                trimmed = trimmed
                    .replace(" - ", "-")
                    .replace("- ", "-")
                    .replace(" -", "-");
                // Escape backslashes and quotes
                let mut safe = trimmed.replace('\\', "\\\\").replace('"', "\\\"");
                // Escape braces and brackets
                safe = safe
                    .replace("{", "&#123;")
                    .replace("}", "&#125;")
                    .replace("[", "&#91;")
                    .replace("]", "&#93;");
                safe
            })
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Extract properties from a schema, resolving $ref if necessary
    fn extract_schema_properties(
        &self,
        schema: &JsonValue,
    ) -> Result<(JsonValue, Option<String>), GenerationError> {
        // Handle null or non-object schemas
        let schema_obj = match schema.as_object() {
            Some(obj) => obj,
            None => return Ok((JsonValue::Null, None)),
        };

        // Direct inline object schema with properties
        if schema_obj.get("properties").is_some()
            || schema_obj.get("additionalProperties").is_some()
        {
            let props = schema_obj
                .get("properties")
                .cloned()
                .unwrap_or(JsonValue::Null);
            return Ok((props, None));
        }

        // Primitive types: return no properties
        if let Some(typ) = schema_obj.get("type").and_then(JsonValue::as_str) {
            if typ != "object" && typ != "array" {
                return Ok((JsonValue::Null, None));
            }
        }

        // Handle $ref
        let ref_str = match schema_obj.get("$ref").and_then(JsonValue::as_str) {
            Some(r) => r,
            None => {
                // Check for array items ref
                if let Some(items) = schema_obj.get("items").and_then(JsonValue::as_object) {
                    if let Some(r) = items.get("$ref").and_then(JsonValue::as_str) {
                        r
                    } else {
                        return Ok((JsonValue::Null, None));
                    }
                } else {
                    return Ok((JsonValue::Null, None));
                }
            }
        };

        // Resolve the reference
        let key = "#/components/schemas/";
        if !ref_str.starts_with(key) {
            return Err(GenerationError::LoadError(format!(
                "Unexpected schema ref '{ref_str}'"
            )));
        }

        let schema_name = &ref_str[key.len()..];
        let schemas = self
            .json
            .get("components")
            .and_then(JsonValue::as_object)
            .and_then(|m| m.get("schemas"))
            .and_then(JsonValue::as_object)
            .ok_or_else(|| {
                GenerationError::LoadError("No components.schemas section".to_string())
            })?;

        let def = schemas.get(schema_name).ok_or_else(|| {
            GenerationError::LoadError(format!("Schema '{schema_name}' not found"))
        })?;

        let props = def.get("properties").cloned().unwrap_or(JsonValue::Null);
        Ok((props, Some(schema_name.to_string())))
    }

    /// Extract request body properties from an operation
    ///
    /// Returns a tuple of (properties_json, schema_name) where:
    /// - properties_json: The schema properties as a JSON object
    /// - schema_name: The name of the schema if it could be determined
    pub fn extract_request_body_properties(
        &self,
        operation: &OpenApiOperation,
    ) -> Result<(JsonValue, Option<String>), GenerationError> {
        // If there's no request body, return empty properties
        let Some(request_body) = &operation.request_body else {
            return Ok((serde_json::json!({}), None));
        };

        // Extract content from request body
        let content = request_body
            .get("content")
            .and_then(JsonValue::as_object)
            .ok_or_else(|| GenerationError::LoadError("Request body has no content".to_string()))?;

        // Look for application/json content
        let json_content = content
            .get("application/json")
            .and_then(JsonValue::as_object)
            .ok_or_else(|| {
                GenerationError::LoadError(
                    "Request body has no application/json content".to_string(),
                )
            })?;

        // Extract schema
        let schema = json_content.get("schema").ok_or_else(|| {
            GenerationError::LoadError("Request body content has no schema".to_string())
        })?;

        // Use the generic schema extraction method
        self.extract_schema_properties(schema)
    }
}

/// Parsed OpenAPI operation for template rendering
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenApiOperation {
    /// Unique string used to identify the operation. The id MUST be unique among all operations described in the API.
    #[serde(rename = "operationId")]
    pub id: String,
    /// The path where this operation is defined (e.g., "/pet/findByStatus")
    pub path: String,
    /// The HTTP method for this operation
    pub method: HttpMethod,
    /// A list of tags for API documentation control. Tags can be used for logical grouping of operations.
    #[serde(rename = "tags")]
    pub tags: Option<Vec<String>>,
    /// A short summary of what the operation does.
    pub summary: Option<String>,
    /// A verbose explanation of the operation behavior. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// Additional external documentation for this operation.
    #[serde(rename = "externalDocs")]
    pub external_docs: Option<serde_json::Value>,
    /// A list of parameters that are applicable for this operation. If a parameter is already defined at the Path Item, the new definition will override it but can never remove it.
    pub parameters: Option<Vec<OpenApiParameter>>,
    /// The request body applicable for this operation.
    #[serde(rename = "requestBody")]
    pub request_body: Option<serde_json::Value>,
    /// The list of possible responses as they are returned from executing this operation.
    pub responses: std::collections::HashMap<String, OpenApiResponse>,
    /// A map of possible out-of band callbacks related to the parent operation.
    pub callbacks: Option<serde_json::Value>,
    /// Declares this operation to be deprecated. Consumers SHOULD refrain from usage of the declared operation.
    pub deprecated: Option<bool>,
    /// A declaration of which security mechanisms can be used for this operation.
    pub security: Option<Vec<serde_json::Value>>,
    /// An alternative server array to service this operation.
    pub servers: Option<Vec<serde_json::Value>>,
    /// Specification extensions (fields starting with `x-`).
    #[serde(flatten)]
    pub vendor_extensions: std::collections::HashMap<String, serde_json::Value>,
}

/// Info about a single OpenAPI parameter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenApiParameterInfo {
    /// Name of the parameter as defined in the OpenAPI spec
    pub name: String,
    /// Optional description of the parameter
    pub description: Option<String>,
    /// Optional example value for the parameter
    pub example: Option<JsonValue>,
    // Note: Language-specific type info (e.g., rust_type) is injected by the builder, not stored here.
}

/// Info about a single response property
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenApiPropertyInfo {
    /// Name of the property as defined in the OpenAPI schema
    pub name: String,
    /// Optional title metadata for the property
    pub title: Option<String>,
    /// Optional description of the property
    pub description: Option<String>,
    /// Optional example value for the property
    pub example: Option<JsonValue>,
}

/// Information about a single parameter in an OpenAPI operation.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenApiParameter {
    /// The name of the parameter. Parameter names are case sensitive.
    pub name: String,
    /// The location of the parameter. Possible values: "query", "header", "path", or "cookie".
    #[serde(rename = "in")]
    pub in_: String,
    /// A brief description of the parameter. This could contain examples of use. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// Determines whether this parameter is mandatory. If the parameter location is "path", this property is REQUIRED and its value MUST be true. Otherwise, the property MAY be included and its default value is false.
    pub required: Option<bool>,
    /// Specifies that a parameter is deprecated and SHOULD be transitioned out of usage.
    pub deprecated: Option<bool>,
    /// Sets the ability to pass empty-valued parameters. This is valid only for query parameters and allows sending a parameter with an empty value. Default value is false.
    #[serde(rename = "allowEmptyValue")]
    pub allow_empty_value: Option<bool>,
    /// Describes how the parameter value will be serialized depending on the type of the parameter value. Default values (based on value of in): for query - form; for path - simple; for header - simple; for cookie - form.
    pub style: Option<String>,
    /// When this is true, parameter values of type array or object generate separate parameters for each value of the array or key-value pair of the map. Default value is false.
    pub explode: Option<bool>,
    /// Determines whether the parameter value SHOULD allow reserved characters, as defined by RFC3986, to appear unescaped in the parameter value. Default value is false.
    #[serde(rename = "allowReserved")]
    pub allow_reserved: Option<bool>,
    /// The schema defining the type used for the parameter.
    pub schema: Option<serde_json::Value>,
    /// Example of the parameter's potential value. The example SHOULD match the specified schema and encoding properties if present.
    pub example: Option<serde_json::Value>,
    /// Examples of the parameter's potential value. Each example SHOULD contain a value in the correct format as specified in the parameter encoding.
    pub examples: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// A map containing the representations for the parameter. The key is the media type and the value describes it.
    pub content: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Specification extensions (fields starting with `x-`).
    #[serde(flatten)]
    pub vendor_extensions: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenApiResponse {
    /// A short description of the response. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// Maps a header name to its definition. The key is the name of the header, and the value describes it.
    pub headers: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// A map containing descriptions of potential response payloads. The key is a media type, and the value describes it.
    pub content: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// A map of operations links that can be followed from the response. The key is the link name, the value describes the link.
    pub links: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Specification extensions (fields starting with `x-`).
    #[serde(flatten)]
    pub vendor_extensions: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    impl OpenApiContext {
        /// Extract operation metadata for testing
        fn extract_operation_metadata(path_item: &JsonValue) -> (String, String, Vec<String>) {
            let get_item = path_item.get("get").and_then(JsonValue::as_object);
            let summary = get_item
                .and_then(|g| g.get("summary"))
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .trim()
                .replace(['\n', '\t'], " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

            let description = get_item
                .and_then(|g| g.get("description"))
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .trim()
                .replace(['\n', '\t'], " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

            let tags = get_item
                .and_then(|g| g.get("tags"))
                .and_then(JsonValue::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(JsonValue::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();

            (summary, description, tags)
        }

        /// Extract parameters for handler testing
        fn extract_parameters_for_handler(&self, path_item: &JsonValue) -> Vec<JsonValue> {
            let get_item = path_item.get("get").and_then(JsonValue::as_object);
            let mut params = get_item
                .and_then(|g| g.get("parameters"))
                .and_then(JsonValue::as_array)
                .cloned()
                .unwrap_or_default();

            // Sort parameters: path params first, then others
            params.sort_by_key(|p| match p.get("in").and_then(JsonValue::as_str) {
                Some("path") => 0,
                _ => 1,
            });

            params
        }

        /// Build response schema for testing
        fn build_response_schema(schema_name: &str) -> JsonValue {
            json!({
                "$ref": format!("#/components/schemas/{}", schema_name)
            })
        }
    }

    #[tokio::test]
    async fn test_from_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("openapi_async.json");
        let json_content = r#"
        {
            "openapi": "3.0.0",
            "info": {
                "title": "Test API Async",
                "version": "2.0.0"
            },
            "servers": [
                {
                    "url": "https://api.example.com/v2"
                }
            ]
        }
        "#;
        tokio::fs::write(&file_path, json_content).await?;

        let spec = OpenApiContext::from_file(&file_path).await?;
        assert_eq!(spec.title(), Some("Test API Async"));
        assert_eq!(spec.version(), Some("2.0.0"));
        assert_eq!(
            spec.base_path(),
            Some("https://api.example.com/v2".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_extract_operation_metadata() {
        let path_item =
            json!({"get": {"summary": "sum", "description": "desc", "tags": ["a","b"]}});
        let (sum, desc, tags) = OpenApiContext::extract_operation_metadata(&path_item);
        assert_eq!(sum, "sum");
        assert_eq!(desc, "desc");
        assert_eq!(tags, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_extract_parameters_for_handler() {
        let spec = OpenApiContext { json: json!({}) };
        let path_item = json!({"get": {"parameters": [{"name": "p", "in": "query"}]}});
        let params = spec.extract_parameters_for_handler(&path_item);
        assert_eq!(params, vec![json!({"name": "p", "in": "query"})]);
    }

    #[test]
    fn test_build_response_schema() {
        let schema = OpenApiContext::build_response_schema("X");
        assert_eq!(schema, json!({"$ref": "#/components/schemas/X"}));
    }

    #[test]
    fn test_extract_row_properties() {
        let props = json!({"data": {"properties": {"k": 1, "m": 2}}});
        let rows = OpenApiContext::extract_row_properties(&props);
        let names: Vec<_> = rows
            .iter()
            .filter_map(|r| r.get("name").and_then(JsonValue::as_str))
            .collect();
        assert_eq!(names, vec!["k", "m"]);
    }

    #[test]
    fn test_http_method_from_str() {
        // Test case-insensitive parsing
        assert_eq!(HttpMethod::from_str("GET").unwrap(), HttpMethod::Get);
        assert_eq!(HttpMethod::from_str("get").unwrap(), HttpMethod::Get);
        assert_eq!(HttpMethod::from_str("Post").unwrap(), HttpMethod::Post);
        assert_eq!(HttpMethod::from_str("DELETE").unwrap(), HttpMethod::Delete);

        // Test invalid method
        assert!(HttpMethod::from_str("INVALID").is_err());
    }

    #[test]
    fn test_extract_row_properties_direct() {
        let props = json!({"x": {"type": "string"}, "y": {"type": "integer"}});
        let rows = OpenApiContext::extract_row_properties(&props);
        let mut names: Vec<_> = rows
            .iter()
            .filter_map(|r| r.get("name").and_then(JsonValue::as_str))
            .collect();
        names.sort();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn test_sanitize_markdown_basic() {
        let raw = "Line one\n\nLine two";
        assert_eq!(OpenApiContext::sanitize_markdown(raw), "Line one Line two");
    }

    #[test]
    fn test_sanitize_markdown_escape_and_unicode() {
        let raw = "\"hi\" {x} “quote” — dash";
        let out = OpenApiContext::sanitize_markdown(raw);
        // Check escapes
        assert!(out.contains("\\\"hi\\\""));
        assert!(out.contains("&#123;x&#125;"));
        // Unicode replaced
        assert!(!out.contains("“"));
        assert!(!out.contains("—"));
    }

    #[test]
    fn test_extract_operation_metadata_trims_and_sanitizes() {
        let path_item = json!({"get": {"summary": " sum \n next", "description": " desc-\tline", "tags": ["t"]}});
        let (s, d, tags) = OpenApiContext::extract_operation_metadata(&path_item);
        assert_eq!(s, "sum next");
        assert_eq!(d, "desc- line");
        assert_eq!(tags, vec!["t".to_string()]);
    }

    #[test]
    fn test_extract_parameters_ordering() {
        let spec = OpenApiContext { json: json!({}) };
        let path_item = json!({"get": {"parameters": [
            {"name": "q", "in": "query"},
            {"name": "p", "in": "path"}
        ]}});
        let names: Vec<String> = spec
            .extract_parameters_for_handler(&path_item)
            .into_iter()
            .filter_map(|p| p.get("name").and_then(JsonValue::as_str).map(String::from))
            .collect();
        assert_eq!(names, vec!["p".to_string(), "q".to_string()]);
    }
}
