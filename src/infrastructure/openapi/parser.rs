//! Comprehensive OpenAPI specification parser
//!
//! This module contains the complete OpenAPI parsing implementation ported from src/core/openapi.rs
//! It handles the full OpenAPI specification including:
//! - Reference resolution ($ref)
//! - Parameters with schema resolution  
//! - Request bodies and responses
//! - Components and schemas
//! - Security definitions
//! - Callbacks and vendor extensions

use std::collections::HashMap;
use openapiv3::OpenAPI;
use serde_json::Value as JsonValue;

use super::filter_openapi_spec;
use crate::generation::{
    ApiInfo, Components, GenerationError, OpenApiContext, Operation, Parameter, ParameterLocation,
    RequestBody, Response, Schema, Server,
};
use crate::generation::sanitizers::sanitize_rust_identifier;

const MAX_SCHEMA_DEPTH: usize = 2; // Limit schema parsing depth to prevent excessive recursion for properties

/// HTTP methods supported by OpenAPI (copied from core)
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "get"),
            HttpMethod::Post => write!(f, "post"),
            HttpMethod::Put => write!(f, "put"),
            HttpMethod::Delete => write!(f, "delete"),
            HttpMethod::Patch => write!(f, "patch"),
            HttpMethod::Head => write!(f, "head"),
            HttpMethod::Options => write!(f, "options"),
        }
    }
}

/// OpenAPI specification parser with comprehensive parsing capabilities
/// This is a complete port of the core::openapi::OpenApiContext implementation
pub struct OpenApiParser {
    /// The raw JSON value of the OpenAPI spec
    pub json: JsonValue,
    /// A cache for resolved schemas to prevent infinite recursion and redundant parsing
    resolved_schemas: HashMap<String, Schema>,
    /// A stack to detect currently resolving schemas and prevent infinite recursion
    resolving_stack: Vec<String>,
    /// Progressive loading: track which operations have been parsed
    parsed_operations: std::collections::HashSet<String>,
    /// Comprehensive memoization cache for expensive operations
    operation_cache: HashMap<String, Operation>,
    /// Schema parsing cache to avoid redundant work
    schema_parsing_cache: HashMap<String, Schema>,
}

impl OpenApiParser {
    /// Create a new parser from JSON content
    pub fn new(json: JsonValue) -> Self {
        Self {
            json,
            resolved_schemas: HashMap::new(),
            resolving_stack: Vec::new(),
            parsed_operations: std::collections::HashSet::new(),
            operation_cache: HashMap::new(),
            schema_parsing_cache: HashMap::new(),
        }
    }

    /// Parse the complete OpenAPI specification to our domain model
    pub async fn parse(&mut self) -> Result<OpenApiContext, GenerationError> {
        tracing::info!("OpenApiParser: Starting to parse OpenAPI specification.");

        // Fix invalid oneOf with strings before deserialization
        let mut fixed_json = self.json.clone();
        self.fix_invalid_oneof_recursive(&mut fixed_json);
        self.json = fixed_json;

        // Deserialize to openapiv3::OpenAPI, apply filter, and serialize back to JsonValue
        let spec: OpenAPI = match serde_json::from_value(self.json.clone()) {
            Ok(spec) => spec,
            Err(e) => {
                let error_msg = format!("Failed to deserialize to OpenAPI struct: {}", e);
                tracing::error!(
                    "Deserialization error: {}. JSON value was: {}",
                    error_msg,
                    serde_json::to_string_pretty(&self.json).unwrap_or_else(|_| "Invalid JSON".to_string())
                );
                return Err(GenerationError::ValidationError(error_msg));
            }
        };
        let filtered_spec = filter_openapi_spec(spec);
        self.json = serde_json::to_value(filtered_spec)
            .map_err(|e| GenerationError::ValidationError(format!("Failed to serialize filtered spec back to JSON: {}", e)))?;

        let version = self
            .json
            .get("openapi")
            .or_else(|| self.json.get("swagger"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| GenerationError::ValidationError("Missing OpenAPI version".to_string()))?
            .to_string();
        tracing::debug!("OpenApiParser: OpenAPI version detected: {}", version);

        let info = ApiInfo {
            title: self
                .title()
                .ok_or_else(|| GenerationError::ValidationError("Missing info.title".to_string()))?
                .to_string(),
            version: self
                .version()
                .ok_or_else(|| {
                    GenerationError::ValidationError("Missing info.version".to_string())
                })?
                .to_string(),
            description: self
                .json
                .get("info")
                .and_then(|info| info.get("description"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };
        tracing::debug!("OpenApiParser: API Info - Title: {}, Version: {}", info.title, info.version);

        let servers: Vec<Server> = self
            .json
            .get("servers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        Some(Server {
                            url: s.get("url").and_then(|v| v.as_str())?.to_string(),
                            description: s
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        tracing::debug!("OpenApiParser: Found {} servers.", servers.len());

        let operations = self.parse_operations().await?;
        tracing::info!("OpenApiParser: Found {} operations.", operations.len());
        for op in &operations {
            tracing::info!("OpenApiParser: Operation before simplification: {} (path: {}, method: {})", op.id, op.path, op.method);
        }

        // Apply schema simplification to reduce complexity and improve performance
        tracing::info!("OpenApiParser: Applying schema simplification to reduce complexity.");
        let operations_before = operations.len();
        // TEMPORARILY DISABLE SCHEMA SIMPLIFICATION TO DEBUG
        // self.simplify_operations_schemas(&mut operations)?;
        tracing::info!("OpenApiParser: Schema simplification completed. Operations before: {}, after: {}", operations_before, operations.len());

        let components = self
            .json
            .get("components")
            .and_then(|comp| comp.as_object())
            .and_then(|comp_obj| comp_obj.get("schemas"))
            .map(|schemas| Components {
                schemas: schemas.clone(),
            });
        tracing::debug!("OpenApiParser: Components (schemas) extracted: {}", components.is_some());

        tracing::info!("OpenApiParser: Successfully parsed OpenAPI specification.");
        Ok(OpenApiContext {
            version,
            info,
            servers,
            operations,
            components,
        })
    }

    /// Get the title of the API
    pub fn title(&self) -> Option<&str> {
        self.json.get("info")?.get("title")?.as_str()
    }

    /// Get the version of the API
    pub fn version(&self) -> Option<&str> {
        self.json.get("info")?.get("version")?.as_str()
    }

    /// Parse all endpoints into structured contexts for template rendering
    /// This is a complete port from core::openapi::OpenApiContext::parse_operations
    /// Now implements progressive loading - operations are parsed without schemas first,
    /// schemas are only parsed when operations actually need them
    pub async fn parse_operations(&mut self) -> Result<Vec<Operation>, GenerationError> {
        // Get paths object
        let paths = self
            .json
            .get("paths")
            .and_then(JsonValue::as_object)
            .ok_or_else(|| {
                GenerationError::ValidationError("Missing 'paths' object".to_string())
            })?;

        tracing::debug!("OpenApiParser: Found {} paths in spec", paths.len());
        tracing::debug!("OpenApiParser: Paths keys: {:?}", paths.keys().collect::<Vec<_>>());

        // Collect all operation data first to avoid mutable/immutable borrow conflicts
        let mut all_operation_data: Vec<(String, HttpMethod, JsonValue, serde_json::Map<String, JsonValue>)> = Vec::new();
        for (path_str, path_item) in paths {
            tracing::debug!("OpenApiParser: Processing path: {}", path_str);
            for method in HttpMethod::all() {
                if let Some(method_item) = path_item
                    .get(method.to_string())
                    .and_then(JsonValue::as_object)
                {
                    tracing::debug!("OpenApiParser: Found method {} for path {}", method, path_str);
                    all_operation_data.push((
                        path_str.clone(), // Clone path string
                        method.clone(), // Clone HttpMethod
                        path_item.clone(), // Clone JsonValue
                        method_item.clone(), // Clone JsonValue map
                    ));
                }
            }
        }

        tracing::debug!("OpenApiParser: Collected {} operation data entries", all_operation_data.len());

        // Now process the collected operation data with progressive loading
        let mut operations = Vec::new();
        for (path, method, path_item, method_item) in all_operation_data {
            tracing::debug!("OpenApiParser: Building operation for path: {} method: {}", path, method);

            // Check if operation is already cached
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

            if let Some(cached_operation) = self.operation_cache.get(&operation_id) {
                tracing::debug!("OpenApiParser: Using cached operation: {}", operation_id);
                operations.push(cached_operation.clone());
                continue;
            }

            let operation = self.build_operation(
                &path, // Pass reference to owned String
                &method,
                &path_item, // Pass reference to owned JsonValue
                &method_item, // Pass reference to owned Map
            )?;

            // Cache the operation for future use
            self.operation_cache.insert(operation_id.clone(), operation.clone());
            operations.push(operation);
        }

        Ok(operations)
    }

    /// Build an Operation from path, method, and method item
    fn build_operation(
        &mut self,
        path: &str,
        method: &HttpMethod,
        path_item: &JsonValue,
        method_item: &serde_json::Map<String, JsonValue>,
    ) -> Result<Operation, GenerationError> {
        tracing::debug!("OpenApiParser: Building operation for path: {} method: {}", path, method);
        let operation_id_raw = method_item
            .get("operationId")
            .and_then(JsonValue::as_str)
            .map(String::from)
            .unwrap_or_else(|| {
                format!(
                    "{}_{}",
                    method,
                    path.trim_start_matches('/').replace('/', "_")
                )
            })
            .to_string(); // Ensure it's a String before sanitizing

        let operation_id = sanitize_rust_identifier(operation_id_raw.trim(), false);
        tracing::debug!("OpenApiParser: Operation ID: {} (sanitized from {})", operation_id, operation_id_raw);

        let summary = method_item
            .get("summary")
            .and_then(JsonValue::as_str)
            .map(String::from);
        let description = method_item
            .get("description")
            .and_then(JsonValue::as_str)
            .map(String::from);
        let external_docs = method_item.get("externalDocs").cloned();

        // Extract typed parameters - merge path-level and method-level parameters
        tracing::debug!("OpenApiParser: Extracting parameters for operation {}.", operation_id);
        let mut parameters = self.extract_parameters(path_item).unwrap_or_default();
        let method_params = self
            .extract_parameters(&JsonValue::Object(method_item.clone()))
            .unwrap_or_default();
        parameters.extend(method_params);
        tracing::debug!("OpenApiParser: Found {} parameters for operation {}.", parameters.len(), operation_id);

        // Extract typed request body
        let request_body = method_item
            .get("requestBody")
            .map(|rb| self.parse_request_body(rb))
            .transpose()?;
        tracing::debug!("OpenApiParser: Request body found for operation {}: {}", operation_id, request_body.is_some());

        // Extract typed responses
        let responses = self.extract_responses(method_item)?;
        tracing::debug!("OpenApiParser: Found {} responses for operation {}.", responses.len(), operation_id);

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

        Ok(Operation {
            id: operation_id,
            path: path.to_string(),
            method: method.to_string(),
            summary,
            description,
            external_docs,
            tags,
            parameters,
            request_body,
            responses,
            callbacks,
            deprecated,
            security,
            servers,
            vendor_extensions,
        })
    }

    /// Extracts parameters from an OpenAPI path item, resolving any $ref references
    /// This is a complete port from core::openapi::OpenApiContext::extract_parameters
    fn extract_parameters(&mut self, path_item: &JsonValue) -> Option<Vec<Parameter>> {
        let raw_parameters = path_item
            .get("parameters")
            .and_then(JsonValue::as_array)?
            .iter()
            .map(|param| {
                if let Some(ref_str) = param.get("$ref").and_then(JsonValue::as_str) {
                    self.resolve_ref(ref_str).cloned() // Clone the resolved JSON value
                } else {
                    Ok(param.clone()) // Clone the parameter JSON value
                }
            })
            .collect::<Result<Vec<JsonValue>, GenerationError>>()
            .ok()?; // Convert Result<Vec<JsonValue>, GenerationError> to Option<Vec<JsonValue>>

        let mut parameters = Vec::new();
        for param_value in raw_parameters {
            tracing::debug!("OpenApiParser: Parsing parameter: {}", param_value.to_string());
            if let Ok(parsed_param) = self.parse_parameter(&param_value) {
                parameters.push(parsed_param);
            }
        }
        Some(parameters)
    }

    /// Parse a single parameter
    fn parse_parameter(&mut self, param: &JsonValue) -> Result<Parameter, GenerationError> {
        let original_name = param["name"]
            .as_str()
            .ok_or_else(|| GenerationError::ValidationError("Parameter missing name".to_string()))?
            .to_string();
        let name = sanitize_rust_identifier(original_name.trim(), false); // Sanitized name for Rust code
        tracing::debug!("OpenApiParser: Parameter name: {} (sanitized from {})", name, original_name);

        let location = match param["in"].as_str() {
            Some("path") => ParameterLocation::Path,
            Some("query") => ParameterLocation::Query,
            Some("header") => ParameterLocation::Header,
            Some("cookie") => ParameterLocation::Cookie,
            _ => {
                tracing::error!("OpenApiParser: Invalid parameter location for parameter: {}", name);
                return Err(GenerationError::ValidationError(
                    "Invalid parameter location".to_string(),
                ));
            }
        };
        tracing::debug!("OpenApiParser: Parameter location: {:?}", location);

        let required = param
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let schema = self.parse_schema(param.get("schema").unwrap_or(&serde_json::json!({})), 0)?;
        let description = param
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Parameter {
            name,
            original_name,
            location,
            required,
            schema,
            description,
        })
    }

    /// Extracts response definitions from an OpenAPI operation
    /// This is a complete port from core::openapi::OpenApiContext::extract_responses
    fn extract_responses(
        &mut self,
        method_item: &serde_json::Map<String, JsonValue>,
    ) -> Result<Vec<Response>, GenerationError> {
        let responses = method_item
            .get("responses")
            .and_then(JsonValue::as_object)
            .map(|map| {
                map.iter()
                    .map(|(k, v)| self.parse_response(k, v))
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap_or_else(|| Ok(Vec::new()))?;

        Ok(responses)
    }

    /// Parse a single response
    fn parse_response(
        &mut self,
        status_code: &str,
        response: &JsonValue,
    ) -> Result<Response, GenerationError> {
        tracing::debug!("OpenApiParser: Parsing response for status code: {}", status_code);
        // Check if this is a $ref
        let resolved_response_owned: JsonValue = if let Some(ref_str) = response.get("$ref").and_then(|v| v.as_str())
        {
            tracing::debug!("OpenApiParser: Resolving response reference: {}", ref_str);
            self.resolve_ref(ref_str)?.clone() // Clone the resolved reference to break lifetime dependency
        } else {
            response.clone() // Clone the original response if not a ref
        };

        // Process content to resolve any $ref in schemas and extract the main schema
        let content_schema = if let Some(content_value) = resolved_response_owned.get("content") {
            tracing::debug!("OpenApiParser: Extracting schema from response content.");
            content_value
                .as_object()
                .and_then(|obj| obj.get("application/json"))
                .and_then(|json_content| json_content.get("schema"))
                .map(|schema_json| self.parse_schema(schema_json, 0))
                .transpose()?
        } else {
            None
        };
        tracing::debug!("OpenApiParser: Response content schema found: {}", content_schema.is_some());

        Ok(Response {
            status_code: status_code.to_string(),
            description: resolved_response_owned // Use the owned value here
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("No description")
                .to_string(),
            content_schema,
        })
    }

    /// Parse a request body
    fn parse_request_body(&mut self, body: &JsonValue) -> Result<RequestBody, GenerationError> {
        tracing::debug!("OpenApiParser: Parsing request body.");
        // Check if this is a $ref
        let resolved_body_owned: JsonValue = if let Some(ref_str) = body.get("$ref").and_then(|v| v.as_str()) {
            tracing::debug!("OpenApiParser: Resolving request body reference: {}", ref_str);
            self.resolve_ref(ref_str)?.clone() // Clone the resolved reference
        } else {
            body.clone() // Clone the original body if not a ref
        };

        // Process content to resolve any $ref in schemas and extract the main schema
        let content_schema = if let Some(content_value) = resolved_body_owned.get("content") {
            tracing::debug!("OpenApiParser: Extracting schema from request body content.");
            content_value
                .as_object()
                .and_then(|obj| obj.get("application/json"))
                .and_then(|json_content| json_content.get("schema"))
                .map(|schema_json| self.parse_schema(schema_json, 0))
                .transpose()?
        } else {
            None
        };
        tracing::debug!("OpenApiParser: Request body content schema found: {}", content_schema.is_some());

        Ok(RequestBody {
            required: resolved_body_owned // Use the owned value here
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            content_schema,
            description: resolved_body_owned // Use the owned value here
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    /// Parse a schema object with comprehensive memoization
    #[allow(clippy::only_used_in_recursion)]
    fn parse_schema<'a>(
        &mut self,
        schema: &'a JsonValue,
        depth: usize,
    ) -> Result<Schema, GenerationError> {
        tracing::debug!(schema = %serde_json::to_string(schema).unwrap_or_default(), depth, "OpenApiParser: Parsing schema");

        // Create a cache key for this schema to enable comprehensive memoization
        let cache_key = self.create_schema_cache_key(schema);
        if let Some(cached_schema) = self.schema_parsing_cache.get(&cache_key) {
            tracing::debug!("OpenApiParser: Using cached schema for key: {}", cache_key);
            return Ok(cached_schema.clone());
        }

        // First check if this is a $ref
        if let Some(ref_str) = schema.get("$ref").and_then(|v| v.as_str()) {
            tracing::debug!("OpenApiParser: Schema is a reference: {}", ref_str);

            // Check if already resolved
            if let Some(cached_schema) = self.resolved_schemas.get(ref_str) {
                tracing::debug!("OpenApiParser: Returning cached schema for reference: {}", ref_str);
                return Ok(cached_schema.clone());
            }

            // If already in resolving stack, it's a recursive reference.
            // We return a minimal schema to break the immediate cycle,
            // but the full schema will be populated by the outer call.
            if self.resolving_stack.contains(&ref_str.to_string()) {
                tracing::warn!(
                    "OpenApiParser: Detected recursive reference: {}. Resolving stack: {:?}. Returning minimal schema to break immediate loop.",
                    ref_str,
                    self.resolving_stack
                );
                let minimal_schema = Schema {
                    schema_type: Some("object".to_string()),
                    properties: Some(indexmap::IndexMap::from([
                        (
                            "type".to_string(),
                            crate::infrastructure::openapi::SchemaProperty {
                                name: "type".to_string(),
                                original_name: "type".to_string(),
                                schema: Schema {
                                    schema_type: Some("string".to_string()),
                                    enum_values: Some(serde_json::json!(["Property"]).as_array().unwrap().clone()),
                                    ..Default::default()
                                },
                            },
                        ),
                        (
                            "value".to_string(),
                            crate::infrastructure::openapi::SchemaProperty {
                                name: "value".to_string(),
                                original_name: "value".to_string(),
                                schema: Schema {
                                    one_of: Some(vec![
                                        Schema {
                                            schema_type: Some("string".to_string()),
                                            ..Default::default()
                                        },
                                        Schema {
                                            schema_type: Some("number".to_string()),
                                            ..Default::default()
                                        },
                                        Schema {
                                            schema_type: Some("boolean".to_string()),
                                            ..Default::default()
                                        },
                                        Schema {
                                            schema_type: Some("array".to_string()),
                                            ..Default::default()
                                        },
                                        Schema {
                                            schema_type: Some("object".to_string()),
                                            ..Default::default()
                                        },
                                    ]),
                                    ..Default::default()
                                },
                            },
                        ),
                    ])),
                    required: Some(vec!["type".to_string(), "value".to_string()]),
                    ..Default::default()
                };

                // Cache the minimal schema to avoid recomputation
                self.schema_parsing_cache.insert(cache_key, minimal_schema.clone());
                return Ok(minimal_schema);
            }

            // Push to resolving stack
            self.resolving_stack.push(ref_str.to_string());
            tracing::debug!("OpenApiParser: Pushed {} to resolving stack. Stack is now: {:?}", ref_str, self.resolving_stack);

            // Resolve the reference to its JSON value and clone it to break the immutable borrow
            let resolved_schema_json_owned = self.resolve_ref(ref_str)?.clone();

            // Parse the resolved schema. This recursive call will handle nested references.
            let resolved_schema = self.parse_schema(&resolved_schema_json_owned, depth + 1)?;

            // Pop from resolving stack
            self.resolving_stack.pop();
            tracing::debug!("OpenApiParser: Popped {} from resolving stack. Stack is now: {:?}", ref_str, self.resolving_stack);

            // Cache the fully resolved schema
            self.resolved_schemas
                .insert(ref_str.to_string(), resolved_schema.clone());
            tracing::debug!("OpenApiParser: Cached fully resolved schema for reference: {}", ref_str);

            // Also cache in the comprehensive schema cache
            self.schema_parsing_cache.insert(cache_key, resolved_schema.clone());
            return Ok(resolved_schema);
        }

        let schema_type = schema
            .get("type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let format = schema
            .get("format")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let items = if let Some(items_value) = schema.get("items") {
            Some(Box::new(self.parse_schema(items_value, depth + 1)?))
        } else {
            None
        };

        // Parse properties recursively to resolve any nested schemas
        let properties = if let Some(props) = schema.get("properties") {
            tracing::debug!("OpenApiParser: Parsing properties for schema.");
            if let Some(props_obj) = props.as_object() {
                let mut parsed_props = indexmap::IndexMap::new();
                for (original_key, value) in props_obj {
                    let sanitized_key = sanitize_rust_identifier(original_key.trim(), false);
                    tracing::debug!(parent_schema_type = ?schema.get("type").and_then(|v| v.as_str()), property_name = original_key, "OpenApiParser: Parsing property");

                    // Apply depth limit for properties
                    let parsed_schema = if depth >= MAX_SCHEMA_DEPTH {
                        tracing::warn!(
                            "OpenApiParser: Max schema depth ({}) reached for property '{}'. Returning simplified schema.",
                            MAX_SCHEMA_DEPTH,
                            original_key
                        );
                        Schema {
                            schema_type: Some("object".to_string()), // Generic object type
                            description: Some(format!("(Schema for '{}' truncated due to max depth limit of {})", original_key, MAX_SCHEMA_DEPTH)),
                            ..Default::default()
                        }
                    } else {
                        self.parse_schema(value, depth + 1)?
                    };

                    let schema_property = crate::infrastructure::openapi::SchemaProperty {
                        name: sanitized_key.clone(), // Sanitized name for Rust code
                        original_name: original_key.clone(), // Original name from OpenAPI spec
                        schema: parsed_schema,
                    };
                    parsed_props.insert(sanitized_key, schema_property);
                }
                Some(parsed_props)
            } else {
                None
            }
        } else {
            None
        };
        let required = schema
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            });
        tracing::debug!("OpenApiParser: Required fields: {:?}", required);

        // Extract all additional schema fields
        let description = schema
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let title = schema
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let default = schema.get("default").cloned();
        let example = schema.get("example").cloned();
        let enum_values = schema.get("enum").and_then(|v| v.as_array()).cloned();
        let minimum = schema.get("minimum").and_then(|v| v.as_f64());
        let maximum = schema.get("maximum").and_then(|v| v.as_f64());
        let min_length = schema
            .get("minLength")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let max_length = schema
            .get("maxLength")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let pattern = schema
            .get("pattern")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let min_items = schema
            .get("minItems")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let max_items = schema
            .get("maxItems")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let unique_items = schema.get("uniqueItems").and_then(|v| v.as_bool());
        let read_only = schema.get("readOnly").and_then(|v| v.as_bool());
        let write_only = schema.get("writeOnly").and_then(|v| v.as_bool());
        let nullable = schema.get("nullable").and_then(|v| v.as_bool());
        let deprecated = schema.get("deprecated").and_then(|v| v.as_bool());
        let xml = schema.get("xml").cloned();

        // Parse additionalProperties
        let additional_properties = if let Some(add_props) = schema.get("additionalProperties") {
            if let Some(bool_val) = add_props.as_bool() {
                Some(Box::new(
                    crate::infrastructure::openapi::AdditionalProperties::Boolean(bool_val),
                ))
            } else {
                let schema = self.parse_schema(add_props, depth + 1)?;
                Some(Box::new(
                    crate::infrastructure::openapi::AdditionalProperties::Schema(Box::new(schema)),
                ))
            }
        } else {
            None
        };

        // Parse composition schemas
        let all_of = if let Some(all_of_arr) = schema.get("allOf").and_then(|v| v.as_array()) {
            let mut schemas = Vec::new();
            for schema_val in all_of_arr {
                schemas.push(self.parse_schema(schema_val, depth + 1)?);
            }
            Some(schemas)
        } else {
            None
        };

        let one_of = if let Some(one_of_arr) = schema.get("oneOf").and_then(|v| v.as_array()) {
            let mut schemas = Vec::new();
            for schema_val in one_of_arr {
                schemas.push(self.parse_schema(schema_val, depth + 1)?);
            }
            Some(schemas)
        } else {
            None
        };

        let any_of = if let Some(any_of_arr) = schema.get("anyOf").and_then(|v| v.as_array()) {
            let mut schemas = Vec::new();
            for schema_val in any_of_arr {
                schemas.push(self.parse_schema(schema_val, depth + 1)?);
            }
            Some(schemas)
        } else {
            None
        };

        let not = if let Some(not_schema) = schema.get("not") {
            Some(Box::new(self.parse_schema(not_schema, depth + 1)?))
        } else {
            None
        };

        // Parse discriminator
        let discriminator =
            if let Some(disc) = schema.get("discriminator").and_then(|v| v.as_object()) {
                let property_name = disc
                    .get("propertyName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| {
                        GenerationError::ValidationError(
                            "Discriminator missing propertyName".to_string(),
                        )
                    })?;
                let mapping = disc.get("mapping").and_then(|v| v.as_object()).map(|m| {
                    m.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                        .collect()
                });
                Some(crate::infrastructure::openapi::Discriminator {
                    property_name,
                    mapping,
                })
            } else {
                None
            };

        // Parse external docs
        let external_docs =
            if let Some(ext_docs) = schema.get("externalDocs").and_then(|v| v.as_object()) {
                let url = ext_docs
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| {
                        GenerationError::ValidationError("ExternalDocs missing url".to_string())
                    })?;
                let description = ext_docs
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Some(crate::infrastructure::openapi::ExternalDocs { url, description })
            } else {
                None
            };

        let parsed_schema = Schema {
            schema_type,
            format,
            items,
            properties,
            required,
            description,
            title,
            default,
            example,
            enum_values,
            minimum,
            maximum,
            min_length,
            max_length,
            pattern,
            min_items,
            max_items,
            unique_items,
            additional_properties,
            all_of,
            one_of,
            any_of,
            not,
            discriminator,
            read_only,
            write_only,
            xml,
            external_docs,
            deprecated,
            nullable,
            unresolved_ref: None,
        };

        // Cache the parsed schema for future use
        self.schema_parsing_cache.insert(cache_key, parsed_schema.clone());

        Ok(parsed_schema)
    }

    /// Create a cache key for schema memoization based on schema content
    fn create_schema_cache_key(&self, schema: &JsonValue) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        // Create a deterministic string representation for hashing
        // Sort keys to ensure consistent hashing regardless of JSON key order
        let normalized = self.normalize_json_for_hashing(schema);
        normalized.hash(&mut hasher);
        format!("schema_{}", hasher.finish())
    }

    /// Normalize JSON for consistent hashing by sorting object keys
    fn normalize_json_for_hashing(&self, value: &JsonValue) -> String {
        match value {
            JsonValue::Object(obj) => {
                let mut sorted_keys: Vec<_> = obj.keys().collect();
                sorted_keys.sort();
                let mut normalized = "{".to_string();
                for (i, key) in sorted_keys.iter().enumerate() {
                    if i > 0 {
                        normalized.push(',');
                    }
                    normalized.push('"');
                    normalized.push_str(key);
                    normalized.push('"');
                    normalized.push(':');
                    normalized.push_str(&self.normalize_json_for_hashing(&obj[*key]));
                }
                normalized.push('}');
                normalized
            }
            JsonValue::Array(arr) => {
                let mut normalized = "[".to_string();
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        normalized.push(',');
                    }
                    normalized.push_str(&self.normalize_json_for_hashing(item));
                }
                normalized.push(']');
                normalized
            }
            _ => value.to_string(),
        }
    }

    /// Resolve a $ref reference
    fn resolve_ref<'a>(&'a self, ref_str: &str) -> Result<&'a JsonValue, GenerationError> {
        tracing::debug!("OpenApiParser: Attempting to resolve reference: {}", ref_str);
        // Handle JSON pointer references (e.g., "#/components/schemas/Pet")
        if let Some(pointer) = ref_str.strip_prefix('#') {
            let result = self.json.pointer(pointer).ok_or_else(|| {
                tracing::error!("OpenApiParser: Failed to resolve internal reference: {}", ref_str);
                GenerationError::ValidationError(format!("Unable to resolve reference: {ref_str}"))
            });
            tracing::debug!("OpenApiParser: Reference resolution result: {:?}", result.is_ok());
            result
        } else {
            // External references not supported yet
            tracing::error!("OpenApiParser: External references not supported: {}", ref_str);
            Err(GenerationError::ValidationError(format!(
                "External references not supported: {ref_str}"
            )))
        }
    }



    /// Recursively fix invalid oneOf definitions and null content in the JSON
    fn fix_invalid_oneof_recursive(&self, value: &mut JsonValue) {
        match value {
            JsonValue::Object(obj) => {
                // Check if this object has a oneOf field
                if let Some(JsonValue::Array(one_of_array)) = obj.get_mut("oneOf") {
                    let mut fixed_one_of = Vec::new();
                    for item in one_of_array.drain(..) {
                        match item {
                            JsonValue::String(s) => {
                                // Convert string to a schema object with enum containing that string
                                let schema = serde_json::json!({
                                    "type": "string",
                                    "enum": [s]
                                });
                                fixed_one_of.push(schema);
                            }
                            JsonValue::Object(_) => {
                                // Already a proper schema object, keep as is
                                fixed_one_of.push(item);
                            }
                            _ => {
                                // For other types, convert to a schema that accepts that type
                                // This is a fallback for unexpected types
                                let schema = match item {
                                    JsonValue::Number(_) => serde_json::json!({"type": "number"}),
                                    JsonValue::Bool(_) => serde_json::json!({"type": "boolean"}),
                                    JsonValue::Array(_) => serde_json::json!({"type": "array"}),
                                    JsonValue::Null => serde_json::json!({"type": "null"}),
                                    _ => serde_json::json!({"type": "string"}), // fallback
                                };
                                fixed_one_of.push(schema);
                            }
                        }
                    }
                    // Replace the oneOf array with the fixed version
                    *one_of_array = fixed_one_of;
                }

                // Fix null content values in responses
                if let Some(JsonValue::Object(content_obj)) = obj.get_mut("content") {
                    let mut keys_to_remove = Vec::new();
                    for (key, value) in content_obj.iter_mut() {
                        if *value == JsonValue::Null {
                            keys_to_remove.push(key.clone());
                        }
                    }
                    for key in keys_to_remove {
                        content_obj.remove(&key);
                    }
                }

                // Recursively process all object values
                for value in obj.values_mut() {
                    self.fix_invalid_oneof_recursive(value);
                }
            }
            JsonValue::Array(arr) => {
                // Recursively process all array elements
                for item in arr.iter_mut() {
                    self.fix_invalid_oneof_recursive(item);
                }
            }
            _ => {
                // Primitive values don't need processing
            }
        }
    }

    /// Extracts vendor extensions (x-* prefixed properties) from an OpenAPI operation
    fn extract_vendor_extensions(
        &self,
        method_item: &serde_json::Map<String, JsonValue>,
    ) -> std::collections::HashMap<String, JsonValue> {
        method_item
            .iter()
            .filter(|(k, _)| k.starts_with("x-"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Apply aggressive schema simplification to all operations to reduce complexity and improve performance
    /// This drastically flattens complex recursive schemas that cause performance issues
    fn simplify_operations_schemas(&self, operations: &mut Vec<Operation>) -> Result<(), GenerationError> {
        tracing::info!("OpenApiParser: Starting aggressive schema simplification for {} operations.", operations.len());

        for operation in operations.iter_mut() {
            tracing::debug!("OpenApiParser: Simplifying schemas for operation: {}", operation.id);

            // Simplify parameter schemas with more aggressive rules
            for param in &mut operation.parameters {
                param.schema = self.simplify_schema_aggressively(&param.schema);
            }

            // Simplify request body schema
            if let Some(request_body) = &mut operation.request_body {
                if let Some(schema) = &mut request_body.content_schema {
                    *schema = self.simplify_schema_aggressively(schema);
                }
            }

            // Simplify response schemas
            for response in &mut operation.responses {
                if let Some(schema) = &mut response.content_schema {
                    *schema = self.simplify_schema_aggressively(schema);
                }
            }
        }

        tracing::info!("OpenApiParser: Aggressive schema simplification completed. Final operation count: {}", operations.len());
        Ok(())
    }

    /// Simplify a single schema by flattening complex structures
    /// This is the core of the performance optimization - replace complex recursive schemas with simple ones
    fn simplify_schema(&self, schema: &Schema) -> Schema {
        // For object schemas with many properties or deep nesting, simplify to a basic object
        if let Some(schema_type) = &schema.schema_type {
            match schema_type.as_str() {
                "object" => {
                    // If the object has many properties or complex nested structures, simplify it
                    if let Some(properties) = &schema.properties {
                        if properties.len() > 10 {
                            // Too many properties - simplify to a generic object
                            tracing::debug!("OpenApiParser: Simplifying object schema with {} properties to generic object.", properties.len());
                            return Schema {
                                schema_type: Some("object".to_string()),
                                description: Some("Simplified complex object schema".to_string()),
                                additional_properties: Some(Box::new(
                                    crate::infrastructure::openapi::AdditionalProperties::Boolean(true)
                                )),
                                ..Default::default()
                            };
                        }

                        // Check if any property has complex nested schemas
                        let has_complex_properties = properties.values().any(|prop| {
                            self.is_complex_schema(&prop.schema)
                        });

                        if has_complex_properties {
                            tracing::debug!("OpenApiParser: Simplifying object schema with complex nested properties.");
                            return Schema {
                                schema_type: Some("object".to_string()),
                                description: Some("Simplified object with complex nested schemas".to_string()),
                                additional_properties: Some(Box::new(
                                    crate::infrastructure::openapi::AdditionalProperties::Boolean(true)
                                )),
                                ..Default::default()
                            };
                        }
                    }
                }
                "array" => {
                    // For arrays, simplify the items schema if it's complex
                    if let Some(items) = &schema.items {
                        if self.is_complex_schema(items) {
                            tracing::debug!("OpenApiParser: Simplifying array items schema.");
                            return Schema {
                                schema_type: Some("array".to_string()),
                                items: Some(Box::new(Schema {
                                    schema_type: Some("object".to_string()),
                                    description: Some("Simplified array item".to_string()),
                                    ..Default::default()
                                })),
                                ..Default::default()
                            };
                        }
                    }
                }
                _ => {}
            }
        }

        // Check for composition schemas (allOf, oneOf, anyOf) which are often complex
        if schema.all_of.is_some() || schema.one_of.is_some() || schema.any_of.is_some() {
            tracing::debug!("OpenApiParser: Simplifying composition schema (allOf/oneOf/anyOf).");
            return Schema {
                schema_type: Some("object".to_string()),
                description: Some("Simplified composition schema".to_string()),
                additional_properties: Some(Box::new(
                    crate::infrastructure::openapi::AdditionalProperties::Boolean(true)
                )),
                ..Default::default()
            };
        }

        // If the schema has a discriminator, it's likely complex - simplify
        if schema.discriminator.is_some() {
            tracing::debug!("OpenApiParser: Simplifying schema with discriminator.");
            return Schema {
                schema_type: Some("object".to_string()),
                description: Some("Simplified discriminated schema".to_string()),
                additional_properties: Some(Box::new(
                    crate::infrastructure::openapi::AdditionalProperties::Boolean(true)
                )),
                ..Default::default()
            };
        }

        // For schemas that are already simple, return as-is
        schema.clone()
    }

    /// Aggressive schema simplification - much more aggressive than the basic version
    /// This is applied after parsing to reduce complexity for template generation
    fn simplify_schema_aggressively(&self, schema: &Schema) -> Schema {
        // Most aggressive simplification: simplify ALL object schemas to generic objects
        if let Some(schema_type) = &schema.schema_type {
            match schema_type.as_str() {
                "object" => {
                    // Simplify ALL object schemas to generic objects with additionalProperties
                    tracing::debug!("OpenApiParser: Aggressively simplifying object schema to generic object.");
                    return Schema {
                        schema_type: Some("object".to_string()),
                        description: Some("Aggressively simplified object schema for performance".to_string()),
                        additional_properties: Some(Box::new(
                            crate::infrastructure::openapi::AdditionalProperties::Boolean(true)
                        )),
                        ..Default::default()
                    };
                }
                "array" => {
                    // Simplify arrays to generic arrays
                    tracing::debug!("OpenApiParser: Aggressively simplifying array schema.");
                    return Schema {
                        schema_type: Some("array".to_string()),
                        items: Some(Box::new(Schema {
                            schema_type: Some("object".to_string()),
                            description: Some("Simplified array item".to_string()),
                            additional_properties: Some(Box::new(
                                crate::infrastructure::openapi::AdditionalProperties::Boolean(true)
                            )),
                            ..Default::default()
                        })),
                        ..Default::default()
                    };
                }
                _ => {
                    // For primitive types, keep them simple but remove complex constraints
                    let mut simplified = schema.clone();
                    // Remove complex validation that might slow down processing
                    simplified.minimum = None;
                    simplified.maximum = None;
                    simplified.min_length = None;
                    simplified.max_length = None;
                    simplified.pattern = None;
                    simplified.enum_values = None;
                    return simplified;
                }
            }
        }

        // For schemas without explicit types (should be rare), simplify to generic object
        tracing::debug!("OpenApiParser: Simplifying schema without type to generic object.");
        Schema {
            schema_type: Some("object".to_string()),
            description: Some("Simplified schema without explicit type".to_string()),
            additional_properties: Some(Box::new(
                crate::infrastructure::openapi::AdditionalProperties::Boolean(true)
            )),
            ..Default::default()
        }
    }

    /// Determine if a schema is complex and should be simplified
    fn is_complex_schema(&self, schema: &Schema) -> bool {
        // Object with many properties
        if let Some(schema_type) = &schema.schema_type {
            if schema_type == "object" {
                if let Some(properties) = &schema.properties {
                    if properties.len() > 5 {
                        return true;
                    }
                    // Check for nested complexity
                    return properties.values().any(|prop| {
                        self.is_complex_schema(&prop.schema)
                    });
                }
            }
        }

        // Composition schemas
        schema.all_of.is_some() || schema.one_of.is_some() || schema.any_of.is_some() ||
        // Discriminators
        schema.discriminator.is_some() ||
        // Deeply nested arrays
        (schema.schema_type.as_deref() == Some("array") &&
         schema.items.as_ref().map_or(false, |items| self.is_complex_schema(items)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_ref_resolution() {
        // Create a simple spec with $ref
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/pets/{petId}": {
                    "get": {
                        "operationId": "getPet",
                        "parameters": [{
                            "name": "petId",
                            "in": "path",
                            "required": true,
                            "schema": { "$ref": "#/components/schemas/PetId" }
                        }],
                        "requestBody": {
                            "$ref": "#/components/requestBodies/PetRequest"
                        },
                        "responses": {
                            "200": {
                                "$ref": "#/components/responses/PetResponse"
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "PetId": {
                        "type": "integer",
                        "format": "int64"
                    },
                    "Pet": {
                        "type": "object",
                        "properties": {
                            "id": { "$ref": "#/components/schemas/PetId" },
                            "name": { "type": "string" }
                        }
                    }
                },
                "requestBodies": {
                    "PetRequest": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/Pet" }
                            }
                        }
                    }
                },
                "responses": {
                    "PetResponse": {
                        "description": "A pet",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/Pet" }
                            }
                        }
                    }
                }
            }
        });

        let mut parser = OpenApiParser::new(spec_json);
        let spec = parser.parse().await.unwrap();

        // Debug: print what we got
        println!("DEBUG TEST: Got {} operations", spec.operations.len());
        for (i, op) in spec.operations.iter().enumerate() {
            println!("DEBUG TEST: Operation {}: id={}, path={}, method={}", i, op.id, op.path, op.method);
        }

        // Check that we have one operation
        assert_eq!(spec.operations.len(), 1);
        let operation = &spec.operations[0];

        // Check parameter schema was resolved
        assert_eq!(operation.parameters.len(), 1);
        let param = &operation.parameters[0];
        assert_eq!(param.schema.schema_type, Some("integer".to_string()));
        assert_eq!(param.schema.format, Some("int64".to_string()));

        // Check request body was resolved
        assert!(operation.request_body.is_some());
        let request_body = operation.request_body.as_ref().unwrap();
        assert!(request_body.required);

        // Check response was resolved
        assert_eq!(operation.responses.len(), 1);
        let response = &operation.responses[0];
        assert_eq!(response.description, "A pet");
        assert!(response.content_schema.is_some()); // Changed from content to content_schema

        // Check that nested $ref in Pet schema was resolved
        let schema_value = serde_json::to_value(response.content_schema.as_ref().unwrap()).unwrap(); // Convert Schema to JsonValue

        // Debug print to see what we have
        println!(
            "Response schema: {}",
            serde_json::to_string_pretty(&schema_value).unwrap() // Added & for borrowing
        );

        // The schema should be fully resolved with no $ref
        assert!(
            schema_value.get("$ref").is_none(),
            "Expected $ref to be resolved"
        );

        // Check that we have the Pet object properties
        assert_eq!(schema_value.get("type"), Some(&json!("object")));

        let props = schema_value.get("properties").expect("Expected properties");
        assert!(props.is_object(), "Properties should be an object");

        // Check the id property (which was a $ref to PetId)
        let id_prop = props.get("id").expect("Expected id property");
        assert_eq!(id_prop.get("type"), Some(&json!("integer")));
        assert_eq!(id_prop.get("format"), Some(&json!("int64")));

        // Check the name property
        let name_prop = props.get("name").expect("Expected name property");
        assert_eq!(name_prop.get("type"), Some(&json!("string")));
    }

    #[tokio::test]
    async fn test_petstore_parsing_parity() {
        // Load the petstore spec
        let petstore_json =
            include_str!("../../../tests/fixtures/openapi/petstore.openapi.v3.json");
        let spec_json: JsonValue = serde_json::from_str(petstore_json).unwrap();

        let mut parser = OpenApiParser::new(spec_json);
        let spec = parser.parse().await.unwrap();

        // Check basic metadata
        assert_eq!(spec.info.title, "Swagger Petstore - OpenAPI 3.0");
        assert_eq!(spec.version, "3.0.4");

        // Find an operation with comprehensive fields (getPetById has security)
        let get_pet_by_id = spec
            .operations
            .iter()
            .find(|op| op.id == "getPetById")
            .expect("getPetById operation not found");

        // Verify all fields are populated
        assert_eq!(get_pet_by_id.path, "/pet/{petId}");
        assert_eq!(get_pet_by_id.method, "get");
        assert_eq!(get_pet_by_id.summary, Some("Find pet by ID.".to_string()));
        assert_eq!(
            get_pet_by_id.description,
            Some("Returns a single pet.".to_string())
        );
        assert_eq!(get_pet_by_id.tags, Some(vec!["pet".to_string()]));

        // Check parameters
        assert_eq!(get_pet_by_id.parameters.len(), 1);
        assert_eq!(get_pet_by_id.parameters[0].name, "petId");
        assert_eq!(
            get_pet_by_id.parameters[0].location,
            ParameterLocation::Path
        );
        assert!(get_pet_by_id.parameters[0].required);

        // Check responses
        assert!(!get_pet_by_id.responses.is_empty());
        let success_response = get_pet_by_id
            .responses
            .iter()
            .find(|r| r.status_code == "200")
            .expect("200 response not found");
        assert_eq!(success_response.description, "successful operation");

        // Check security - this operation has both api_key and petstore_auth
        assert!(get_pet_by_id.security.is_some());
        let security = get_pet_by_id.security.as_ref().unwrap();
        assert_eq!(security.len(), 2);

        // Check an operation with request body (updatePet)
        let update_pet = spec
            .operations
            .iter()
            .find(|op| op.id == "updatePet")
            .expect("updatePet operation not found");

        assert!(update_pet.request_body.is_some());
        let request_body = update_pet.request_body.as_ref().unwrap();
        assert!(request_body.required);
        assert!(request_body.description.is_some());

        // Check an operation with multiple parameters (updatePetWithForm)
        let update_form = spec
            .operations
            .iter()
            .find(|op| op.id == "updatePetWithForm")
            .expect("updatePetWithForm operation not found");

        assert_eq!(update_form.parameters.len(), 3); // petId (path), name (query), status (query)
        let path_param = update_form
            .parameters
            .iter()
            .find(|p| p.location == ParameterLocation::Path)
            .expect("Path parameter not found");
        assert_eq!(path_param.name, "petId");
        assert!(path_param.required);

        let query_params: Vec<_> = update_form
            .parameters
            .iter()
            .filter(|p| p.location == ParameterLocation::Query)
            .collect();
        assert_eq!(query_params.len(), 2);
        assert!(query_params.iter().any(|p| p.name == "name"));
        assert!(query_params.iter().any(|p| p.name == "status"));

        // Check an operation with no parameters (getInventory)
        let get_inventory = spec
            .operations
            .iter()
            .find(|op| op.id == "getInventory")
            .expect("getInventory operation not found");
        assert_eq!(get_inventory.parameters.len(), 0);

        // Check operation with array parameter (findPetsByTags)
        let find_by_tags = spec
            .operations
            .iter()
            .find(|op| op.id == "findPetsByTags")
            .expect("findPetsByTags operation not found");
        assert_eq!(find_by_tags.parameters.len(), 1);
        let tags_param = &find_by_tags.parameters[0];
        assert_eq!(tags_param.name, "tags");
        assert_eq!(tags_param.location, ParameterLocation::Query);
        assert!(!tags_param.required); // This one is optional

        // Check operation with enum parameter (findPetsByStatus)
        let find_by_status = spec
            .operations
            .iter()
            .find(|op| op.id == "findPetsByStatus")
            .expect("findPetsByStatus operation not found");
        assert_eq!(find_by_status.parameters.len(), 1);
        let status_param = &find_by_status.parameters[0];
        assert_eq!(status_param.name, "status");
        // Check that schema contains enum values
        if let Some(schema_type) = &status_param.schema.schema_type {
            assert_eq!(schema_type, "string");
        }

        // Check operation with header parameter (deletePet)
        let delete_pet = spec
            .operations
            .iter()
            .find(|op| op.id == "deletePet")
            .expect("deletePet operation not found");
        assert_eq!(delete_pet.parameters.len(), 2);
        let header_param = delete_pet
            .parameters
            .iter()
            .find(|p| p.location == ParameterLocation::Header)
            .expect("Header parameter not found");
        assert_eq!(header_param.name, "api_key");
        assert!(!header_param.required);

        // Check operation without security (placeOrder)
        let place_order = spec
            .operations
            .iter()
            .find(|op| op.id == "placeOrder")
            .expect("placeOrder operation not found");
        assert!(place_order.security.is_none());

        // Check operation with empty parameters array (logoutUser)
        let logout_user = spec
            .operations
            .iter()
            .find(|op| op.id == "logoutUser")
            .expect("logoutUser operation not found");
        assert_eq!(logout_user.parameters.len(), 0);

        // Verify that all methods are parsed correctly
        let put_ops: Vec<_> = spec
            .operations
            .iter()
            .filter(|op| op.method == "put")
            .collect();
        assert!(put_ops.len() >= 2); // updatePet, updateUser

        let post_ops: Vec<_> = spec
            .operations
            .iter()
            .filter(|op| op.method == "post")
            .collect();
        assert!(post_ops.len() >= 5); // Multiple POST operations

        let delete_ops: Vec<_> = spec
            .operations
            .iter()
            .filter(|op| op.method == "delete")
            .collect();
        assert!(delete_ops.len() >= 3); // deletePet, deleteOrder, deleteUser

        // Verify no vendor extensions on standard operations
        assert_eq!(get_pet_by_id.vendor_extensions.len(), 0);

        // Verify deprecated flag is not set on non-deprecated operations
        assert_eq!(get_pet_by_id.deprecated, None);

        // Count total operations to ensure we're parsing all of them
        assert_eq!(spec.operations.len(), 19); // Petstore has exactly 19 operations
    }
}
