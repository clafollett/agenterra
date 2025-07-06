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

use serde_json::{Value as JsonValue, json};

use crate::generation::{
    ApiInfo, Components, GenerationError, OpenApiSpec, Operation, Parameter, ParameterLocation,
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

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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
    pub async fn parse(&self) -> Result<OpenApiSpec, GenerationError> {
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

        Ok(OpenApiSpec {
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
                            .get(method.as_str())
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
        Ok(Response {
            status_code: status_code.to_string(),
            description: response
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("No description")
                .to_string(),
            content: response.get("content").cloned(),
        })
    }

    /// Parse a request body
    fn parse_request_body(&self, body: &JsonValue) -> Result<RequestBody, GenerationError> {
        Ok(RequestBody {
            required: body
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            content: body.get("content").cloned().unwrap_or_default(),
            description: body
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    /// Parse a schema object
    fn parse_schema(&self, schema: &JsonValue) -> Result<Schema, GenerationError> {
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

        let properties = schema.get("properties").cloned();
        let required = schema
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            });

        Ok(Schema {
            schema_type,
            format,
            items,
            properties,
            required,
        })
    }

    /// Extract properties from a schema, resolving $ref if necessary
    /// This is a complete port from core::openapi::OpenApiContext::extract_schema_properties
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
            return Err(GenerationError::ValidationError(format!(
                "Unexpected schema ref '{}'",
                ref_str
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
                GenerationError::ValidationError("No components.schemas section".to_string())
            })?;

        let def = schemas.get(schema_name).ok_or_else(|| {
            GenerationError::ValidationError(format!("Schema '{}' not found", schema_name))
        })?;

        let props = def.get("properties").cloned().unwrap_or(JsonValue::Null);
        Ok((props, Some(schema_name.to_string())))
    }

    /// Extract row properties from properties JSON
    /// This is a complete port from core::openapi::OpenApiContext::extract_row_properties
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
        assert_eq!(get_pet_by_id.parameters[0].required, true);

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
        assert_eq!(request_body.required, true);
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
