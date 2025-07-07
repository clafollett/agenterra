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

use serde_json::Value as JsonValue;

use crate::generation::{
    ApiInfo, Components, GenerationError, OpenApiContext, Operation, Parameter, ParameterLocation,
    RequestBody, Response, Schema, Server,
};

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
}

impl OpenApiParser {
    /// Create a new parser from JSON content
    pub fn new(json: JsonValue) -> Self {
        Self { json }
    }

    /// Parse the complete OpenAPI specification to our domain model
    pub async fn parse(&self) -> Result<OpenApiContext, GenerationError> {
        // Extract version
        let version = self
            .json
            .get("openapi")
            .or_else(|| self.json.get("swagger"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| GenerationError::ValidationError("Missing OpenAPI version".to_string()))?
            .to_string();

        // Extract info
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

        // Extract servers
        let servers = self
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

        // Parse operations using the comprehensive implementation
        let operations = self.parse_operations().await?;
        tracing::debug!("OpenAPI parser found {} operations", operations.len());

        // Extract components if present
        let components = self
            .json
            .get("components")
            .and_then(|comp| comp.as_object())
            .and_then(|comp_obj| comp_obj.get("schemas"))
            .map(|schemas| Components {
                schemas: schemas.clone(),
            });

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
    pub async fn parse_operations(&self) -> Result<Vec<Operation>, GenerationError> {
        // Get paths object
        let paths = self
            .json
            .get("paths")
            .and_then(JsonValue::as_object)
            .ok_or_else(|| {
                GenerationError::ValidationError("Missing 'paths' object".to_string())
            })?;

        // Use iterator combinators to flatten and map operations
        let operations = paths
            .iter()
            .flat_map(|(path, path_item)| {
                HttpMethod::all()
                    .iter()
                    .filter_map(|method| {
                        path_item
                            .get(method.to_string())
                            .and_then(JsonValue::as_object)
                            .map(|method_item| (path, method, path_item, method_item))
                    })
                    .collect::<Vec<_>>()
            })
            .map(|(path, method, path_item, method_item)| {
                self.build_operation(path, method, path_item, method_item)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(operations)
    }

    /// Build an Operation from path, method, and method item
    fn build_operation(
        &self,
        path: &str,
        method: &HttpMethod,
        path_item: &JsonValue,
        method_item: &serde_json::Map<String, JsonValue>,
    ) -> Result<Operation, GenerationError> {
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

        // Extract typed parameters - merge path-level and method-level parameters
        let mut parameters = self.extract_parameters(path_item).unwrap_or_default();
        let method_params = self
            .extract_parameters(&JsonValue::Object(method_item.clone()))
            .unwrap_or_default();
        parameters.extend(method_params);

        // Extract typed request body
        let request_body = method_item
            .get("requestBody")
            .map(|rb| self.parse_request_body(rb))
            .transpose()?;

        // Extract typed responses
        let responses = self.extract_responses(method_item)?;

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
    fn extract_parameters(&self, path_item: &JsonValue) -> Option<Vec<Parameter>> {
        path_item
            .get("parameters")
            .and_then(JsonValue::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|param| {
                        if let Some(ref_str) = param.get("$ref").and_then(JsonValue::as_str) {
                            self.json
                                .pointer(&ref_str[1..])
                                .and_then(|p| self.parse_parameter(p).ok())
                        } else {
                            self.parse_parameter(param).ok()
                        }
                    })
                    .collect::<Vec<Parameter>>()
            })
    }

    /// Parse a single parameter
    fn parse_parameter(&self, param: &JsonValue) -> Result<Parameter, GenerationError> {
        let name = param["name"]
            .as_str()
            .ok_or_else(|| GenerationError::ValidationError("Parameter missing name".to_string()))?
            .to_string();

        let location = match param["in"].as_str() {
            Some("path") => ParameterLocation::Path,
            Some("query") => ParameterLocation::Query,
            Some("header") => ParameterLocation::Header,
            Some("cookie") => ParameterLocation::Cookie,
            _ => {
                return Err(GenerationError::ValidationError(
                    "Invalid parameter location".to_string(),
                ));
            }
        };

        let required = param
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let schema = self.parse_schema(param.get("schema").unwrap_or(&serde_json::json!({})))?;
        let description = param
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Parameter {
            name,
            location,
            required,
            schema,
            description,
        })
    }

    /// Extracts response definitions from an OpenAPI operation
    /// This is a complete port from core::openapi::OpenApiContext::extract_responses
    fn extract_responses(
        &self,
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
        &self,
        status_code: &str,
        response: &JsonValue,
    ) -> Result<Response, GenerationError> {
        // Check if this is a $ref
        let resolved_response = if let Some(ref_str) = response.get("$ref").and_then(|v| v.as_str())
        {
            self.resolve_ref(ref_str)?
        } else {
            response.clone()
        };

        // Process content to resolve any $ref in schemas
        let content = if let Some(content_value) = resolved_response.get("content") {
            if let Some(content_obj) = content_value.as_object() {
                let mut resolved_content = serde_json::Map::new();
                for (media_type, media_value) in content_obj {
                    if let Some(media_obj) = media_value.as_object() {
                        let mut resolved_media = media_obj.clone();
                        // Check if there's a schema to resolve
                        if let Some(schema) = media_obj.get("schema") {
                            let resolved_schema = self.resolve_schema_refs(schema)?;
                            resolved_media.insert("schema".to_string(), resolved_schema);
                        }
                        resolved_content
                            .insert(media_type.clone(), JsonValue::Object(resolved_media));
                    } else {
                        resolved_content.insert(media_type.clone(), media_value.clone());
                    }
                }
                Some(JsonValue::Object(resolved_content))
            } else {
                Some(content_value.clone())
            }
        } else {
            None
        };

        Ok(Response {
            status_code: status_code.to_string(),
            description: resolved_response
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("No description")
                .to_string(),
            content,
        })
    }

    /// Parse a request body
    fn parse_request_body(&self, body: &JsonValue) -> Result<RequestBody, GenerationError> {
        // Check if this is a $ref
        let resolved_body = if let Some(ref_str) = body.get("$ref").and_then(|v| v.as_str()) {
            self.resolve_ref(ref_str)?
        } else {
            body.clone()
        };

        // Process content to resolve any $ref in schemas
        let content = if let Some(content_value) = resolved_body.get("content") {
            self.resolve_schema_refs(content_value)?
        } else {
            JsonValue::Null
        };

        Ok(RequestBody {
            required: resolved_body
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            content,
            description: resolved_body
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    /// Parse a schema object
    #[allow(clippy::only_used_in_recursion)]
    fn parse_schema(&self, schema: &JsonValue) -> Result<Schema, GenerationError> {
        // First check if this is a $ref
        if let Some(ref_str) = schema.get("$ref").and_then(|v| v.as_str()) {
            // Resolve the reference
            let resolved_schema = self.resolve_ref(ref_str)?;
            // Parse the resolved schema
            return self.parse_schema(&resolved_schema);
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
            Some(Box::new(self.parse_schema(items_value)?))
        } else {
            None
        };

        // Parse properties recursively to resolve any nested schemas
        let properties = if let Some(props) = schema.get("properties") {
            if let Some(props_obj) = props.as_object() {
                let mut parsed_props = std::collections::HashMap::new();
                for (key, value) in props_obj {
                    let parsed_schema = self.parse_schema(value)?;
                    parsed_props.insert(key.clone(), parsed_schema);
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
                let schema = self.parse_schema(add_props)?;
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
                schemas.push(self.parse_schema(schema_val)?);
            }
            Some(schemas)
        } else {
            None
        };

        let one_of = if let Some(one_of_arr) = schema.get("oneOf").and_then(|v| v.as_array()) {
            let mut schemas = Vec::new();
            for schema_val in one_of_arr {
                schemas.push(self.parse_schema(schema_val)?);
            }
            Some(schemas)
        } else {
            None
        };

        let any_of = if let Some(any_of_arr) = schema.get("anyOf").and_then(|v| v.as_array()) {
            let mut schemas = Vec::new();
            for schema_val in any_of_arr {
                schemas.push(self.parse_schema(schema_val)?);
            }
            Some(schemas)
        } else {
            None
        };

        let not = if let Some(not_schema) = schema.get("not") {
            Some(Box::new(self.parse_schema(not_schema)?))
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

        Ok(Schema {
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
        })
    }

    /// Recursively resolve all $ref in a JSON value
    fn resolve_schema_refs(&self, value: &JsonValue) -> Result<JsonValue, GenerationError> {
        match value {
            JsonValue::Object(obj) => {
                // Check if this object has a $ref
                if let Some(ref_str) = obj.get("$ref").and_then(|v| v.as_str()) {
                    // Resolve the reference and recursively resolve any nested refs
                    let resolved = self.resolve_ref(ref_str)?;
                    return self.resolve_schema_refs(&resolved);
                }

                // Otherwise, recursively process all fields
                let mut resolved_obj = serde_json::Map::new();
                for (key, val) in obj {
                    resolved_obj.insert(key.clone(), self.resolve_schema_refs(val)?);
                }
                Ok(JsonValue::Object(resolved_obj))
            }
            JsonValue::Array(arr) => {
                // Recursively process array elements
                let resolved_arr: Result<Vec<_>, _> = arr
                    .iter()
                    .map(|elem| self.resolve_schema_refs(elem))
                    .collect();
                Ok(JsonValue::Array(resolved_arr?))
            }
            // Primitive values are returned as-is
            _ => Ok(value.clone()),
        }
    }

    /// Resolve a $ref reference
    fn resolve_ref(&self, ref_str: &str) -> Result<JsonValue, GenerationError> {
        // Handle JSON pointer references (e.g., "#/components/schemas/Pet")
        if let Some(pointer) = ref_str.strip_prefix('#') {
            self.json.pointer(pointer).cloned().ok_or_else(|| {
                GenerationError::ValidationError(format!("Unable to resolve reference: {ref_str}"))
            })
        } else {
            // External references not supported yet
            Err(GenerationError::ValidationError(format!(
                "External references not supported: {ref_str}"
            )))
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

        let parser = OpenApiParser::new(spec_json);
        let spec = parser.parse().await.unwrap();

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
        assert!(response.content.is_some());

        // Check that nested $ref in Pet schema was resolved
        let response_content = response.content.as_ref().unwrap();
        let json_content = response_content.get("application/json").unwrap();
        let schema_value = json_content.get("schema").unwrap();

        // Debug print to see what we have
        println!(
            "Response schema: {}",
            serde_json::to_string_pretty(schema_value).unwrap()
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

        let parser = OpenApiParser::new(spec_json);
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
