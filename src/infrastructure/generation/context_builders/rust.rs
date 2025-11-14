//! Rust-specific context builder for code generation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use rayon::prelude::*;

use crate::generation::{
    ContextBuilder, GenerationContext, GenerationError, Language, Operation, RenderContext,
    sanitizers::{sanitize_markdown, sanitize_rust_identifier}, // Add sanitize_rust_identifier
    utils::{to_proper_case, to_snake_case, sanitize_rust_field_name},
};
use crate::infrastructure::Template;

/// Rust-specific property information with type mapping
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RustPropertyInfo {
    pub name: String, // Sanitized Rust identifier
    pub original_name: String, // Original name from OpenAPI spec
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
    Path, // Add Path as a new parameter source
}

/// Unified parameter combining query parameters and request body properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedParameter {
    pub name: String,
    pub original_name: String,
    pub source: ParameterSource,
    pub rust_type: String,
    pub description: Option<String>,
    pub required: bool, // Add this field
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
    pub spec_file_name: Option<String>,
    // NEW: Unified parameter support for Issue #106
    pub unified_parameters: Vec<UnifiedParameter>,
    pub has_body_properties: bool,
    pub http_method: String,
}

/// Rust-specific context builder
pub struct RustContextBuilder;

impl RustContextBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Recursively check if a JSON object contains unresolved_ref anywhere
    fn has_unresolved_ref_recursive(obj: &serde_json::Map<String, JsonValue>) -> bool {
        for value in obj.values() {
            match value {
                JsonValue::Object(inner_obj) => {
                    if inner_obj.contains_key("unresolved_ref") {
                        return true;
                    }
                    if Self::has_unresolved_ref_recursive(inner_obj) {
                        return true;
                    }
                }
                JsonValue::Array(arr) => {
                    for item in arr {
                        if let Some(inner_obj) = item.as_object() {
                            if Self::has_unresolved_ref_recursive(inner_obj) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
}

/// Lazy evaluation wrapper for schema properties
/// Only computes properties when first accessed
#[derive(Debug)]
struct LazySchemaProperties {
    schema: crate::generation::Schema,
    max_depth: usize,
    properties: OnceLock<Vec<RustPropertyInfo>>,
}

impl LazySchemaProperties {
    fn new(schema: crate::generation::Schema, max_depth: usize) -> Self {
        Self {
            schema,
            max_depth,
            properties: OnceLock::new(),
        }
    }

    fn get(&self) -> &Vec<RustPropertyInfo> {
        self.properties.get_or_init(|| {
            extract_typed_schema_properties_iterative(&self.schema, self.max_depth)
        })
    }
}

/// Lazy evaluation wrapper for envelope properties
#[derive(Debug)]
struct LazyEnvelopeProperties {
    schema: crate::generation::Schema,
    max_depth: usize,
    envelope: OnceLock<JsonValue>,
}

impl LazyEnvelopeProperties {
    fn new(schema: crate::generation::Schema, max_depth: usize) -> Self {
        Self {
            schema,
            max_depth,
            envelope: OnceLock::new(),
        }
    }

    fn get(&self) -> &JsonValue {
        self.envelope.get_or_init(|| {
            extract_typed_envelope_properties_iterative(&self.schema, self.max_depth)
        })
    }
}

/// Lazy evaluation wrapper for properties map
#[derive(Debug)]
struct LazyPropertiesMap {
    schema: crate::generation::Schema,
    max_depth: usize,
    map: OnceLock<Option<JsonMap<String, JsonValue>>>,
}

impl LazyPropertiesMap {
    fn new(schema: crate::generation::Schema, max_depth: usize) -> Self {
        Self {
            schema,
            max_depth,
            map: OnceLock::new(),
        }
    }

    fn get(&self) -> &Option<JsonMap<String, JsonValue>> {
        self.map.get_or_init(|| {
            extract_typed_properties_map_iterative(&self.schema, self.max_depth)
        })
    }
}

/// Global schema processing cache shared across all operations
/// Uses Arc<Mutex<>> for thread-safe access during parallel processing
/// Now includes lazy evaluation for better performance
#[derive(Debug, Clone)]
struct GlobalSchemaCache {
    envelope_properties: Arc<Mutex<HashMap<String, LazyEnvelopeProperties>>>,
    schema_properties: Arc<Mutex<HashMap<String, LazySchemaProperties>>>,
    properties_maps: Arc<Mutex<HashMap<String, LazyPropertiesMap>>>,
    processing_stack: Arc<Mutex<Vec<String>>>, // Track recursion to prevent infinite loops
    max_processing_time_ms: u64, // Maximum time to spend processing a single operation
    max_schema_depth: usize, // Maximum depth to process schemas
}

impl GlobalSchemaCache {
    fn new() -> Self {
        Self {
            envelope_properties: Arc::new(Mutex::new(HashMap::new())),
            schema_properties: Arc::new(Mutex::new(HashMap::new())),
            properties_maps: Arc::new(Mutex::new(HashMap::new())),
            processing_stack: Arc::new(Mutex::new(Vec::new())),
            max_processing_time_ms: 5000, // 5 seconds max per operation
            max_schema_depth: 3, // Maximum 3 levels of recursion
        }
    }

    // Convenience methods that return computed values (for backward compatibility)
    fn get_or_compute_envelope(&self, schema: &crate::generation::Schema, cache_key: &str) -> JsonValue {
        let mut cache = self.envelope_properties.lock().unwrap();
        if let Some(lazy) = cache.get(cache_key) {
            return lazy.get().clone();
        }

        let lazy = LazyEnvelopeProperties::new(schema.clone(), self.max_schema_depth);
        let result = lazy.get().clone();
        cache.insert(cache_key.to_string(), lazy);
        result
    }

    fn get_or_compute_schema_properties(&self, schema: &crate::generation::Schema, cache_key: &str) -> Vec<RustPropertyInfo> {
        let mut cache = self.schema_properties.lock().unwrap();
        if let Some(lazy) = cache.get(cache_key) {
            return lazy.get().clone();
        }

        let lazy = LazySchemaProperties::new(schema.clone(), self.max_schema_depth);
        let result = lazy.get().clone();
        cache.insert(cache_key.to_string(), lazy);
        result
    }

    fn get_or_compute_properties_map(&self, schema: &crate::generation::Schema, cache_key: &str) -> Option<JsonMap<String, JsonValue>> {
        let mut cache = self.properties_maps.lock().unwrap();
        if let Some(lazy) = cache.get(cache_key) {
            return lazy.get().clone();
        }

        let lazy = LazyPropertiesMap::new(schema.clone(), self.max_schema_depth);
        let result = lazy.get().clone();
        cache.insert(cache_key.to_string(), lazy);
        result
    }
}

/// Legacy cache for backward compatibility - will be removed once all functions are migrated
#[derive(Debug, Clone)]
struct ProcessedSchemaCache {
    envelope_properties: HashMap<String, JsonValue>,
    schema_properties: HashMap<String, Vec<RustPropertyInfo>>,
    properties_maps: HashMap<String, Option<JsonMap<String, JsonValue>>>,
    processing_stack: Vec<String>, // Track recursion to prevent infinite loops
    max_processing_time_ms: u64, // Maximum time to spend processing a single operation
    max_schema_depth: usize, // Maximum depth to process schemas
}

impl ProcessedSchemaCache {
    fn new() -> Self {
        Self {
            envelope_properties: HashMap::new(),
            schema_properties: HashMap::new(),
            properties_maps: HashMap::new(),
            processing_stack: Vec::new(),
            max_processing_time_ms: 5000, // 5 seconds max per operation
            max_schema_depth: 3, // Maximum 3 levels of recursion
        }
    }

    fn get_or_compute_envelope(&mut self, schema: &crate::generation::Schema, cache_key: &str) -> JsonValue {
        if let Some(cached) = self.envelope_properties.get(cache_key) {
            return cached.clone();
        }
        let result = extract_typed_envelope_properties_iterative(schema, self.max_schema_depth);
        self.envelope_properties.insert(cache_key.to_string(), result.clone());
        result
    }

    fn get_or_compute_schema_properties(&mut self, schema: &crate::generation::Schema, cache_key: &str) -> Vec<RustPropertyInfo> {
        if let Some(cached) = self.schema_properties.get(cache_key) {
            return cached.clone();
        }
        let result = extract_typed_schema_properties_iterative(schema, self.max_schema_depth);
        self.schema_properties.insert(cache_key.to_string(), result.clone());
        result
    }

    fn get_or_compute_properties_map(&mut self, schema: &crate::generation::Schema, cache_key: &str) -> Option<JsonMap<String, JsonValue>> {
        if let Some(cached) = self.properties_maps.get(cache_key) {
            return cached.clone();
        }
        let result = extract_typed_properties_map_iterative(schema, self.max_schema_depth);
        self.properties_maps.insert(cache_key.to_string(), result.clone());
        result
    }
}

#[async_trait]
impl ContextBuilder for RustContextBuilder {
    async fn build(
        &self,
        context: &GenerationContext,
        template: &Template,
    ) -> Result<RenderContext, GenerationError> {
        tracing::info!(
            "RustContextBuilder: Building render context for protocol {:?}, role {:?}, language {:?} with template {}",
            context.protocol,
            context.role,
            context.language,
            template.source
        );
        // Ensure this is for Rust
        if context.language != Language::Rust {
            tracing::error!(
                "RustContextBuilder: Invalid language {:?} for RustContextBuilder. Expected Rust.",
                context.language
            );
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
        let mut endpoints: Vec<serde_json::Value> = Vec::new();
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

                    // Initialize global schema processing cache for parallel processing
                    let global_cache = Arc::new(GlobalSchemaCache::new());

                    // Process operations into Rust endpoint contexts using parallel processing
                    tracing::info!("RustContextBuilder: Starting parallel processing of {} operations.", operations.len());

                    // Use Rayon for parallel processing of operations
                    let endpoint_results: Vec<Result<serde_json::Value, GenerationError>> = operations
                        .par_iter()
                        .map(|operation| {
                            tracing::debug!("RustContextBuilder: Processing operation ID: {} (parallel)", operation.id);
                            let endpoint_context = build_rust_endpoint_context_parallel(operation, &global_cache)?;

                            // Try to serialize to JSON and catch any serialization errors
                            let mut endpoint_json = match serde_json::to_value(&endpoint_context) {
                                Ok(json) => json,
                                Err(e) => {
                                    tracing::error!("RustContextBuilder: Failed to serialize endpoint context for operation {}: {}", operation.id, e);
                                    tracing::error!("RustContextBuilder: Endpoint context debug: {:?}", endpoint_context);
                                    return Err(GenerationError::InvalidConfiguration(format!(
                                        "Failed to serialize endpoint context for operation {}: {}",
                                        operation.id, e
                                    )));
                                }
                            };

                            // Ensure 'unified_parameters' is always present, even if empty
                            if endpoint_json.get("unified_parameters").is_none() {
                                endpoint_json["unified_parameters"] = json!([]);
                                tracing::warn!("RustContextBuilder: Added missing 'unified_parameters' to endpoint ID: {}", operation.id);
                            }

                            Ok(endpoint_json)
                        })
                        .collect();

                    // Collect results and handle any errors
                    for result in endpoint_results {
                        match result {
                            Ok(endpoint_json) => endpoints.push(endpoint_json),
                            Err(e) => return Err(e),
                        }
                    }

                    tracing::info!("RustContextBuilder: Finished parallel processing of operations.");
                }
            }
        }
        tracing::info!("RustContextBuilder: before Add both endpoints & endpoint for compatibility.");
        tracing::info!("RustContextBuilder: endpoints vector has {} items", endpoints.len());

        // Debug: Check if any endpoint has problematic structure
        let mut total_keys = 0;
        let mut max_keys = 0;
        let mut endpoints_with_large_keys = 0;
        let mut endpoints_with_unresolved_refs = 0;

        for (i, endpoint) in endpoints.iter().enumerate() {
            if let Some(obj) = endpoint.as_object() {
                let key_count = obj.len();
                total_keys += key_count;
                max_keys = max_keys.max(key_count);

                if key_count > 50 { // Log if unusually large
                    tracing::warn!("RustContextBuilder: Endpoint {} has {} keys (unusually large)", i, key_count);
                    endpoints_with_large_keys += 1;
                }

                // Check for unresolved_ref in endpoint data (recursive check)
                if Self::has_unresolved_ref_recursive(obj) {
                    endpoints_with_unresolved_refs += 1;
                    tracing::warn!("RustContextBuilder: Endpoint {} contains unresolved_ref", i);
                }
            }
        }

        tracing::info!(
            "RustContextBuilder: Context summary - Endpoints: {}, Total keys: {}, Max keys per endpoint: {}, Large endpoints (>50 keys): {}, Endpoints with unresolved_ref: {}",
            endpoints.len(),
            total_keys,
            max_keys,
            endpoints_with_large_keys,
            endpoints_with_unresolved_refs
        );

        // Add "endpoints" for iteration in the renderer
        render_context.add_variable("endpoints", json!(endpoints));
        // Add all custom variables from context
        tracing::info!("RustContextBuilder: before Add all custom variables from context.");
        for (key, value) in &context.variables {
            render_context.add_variable(key, value.clone());
        }
        tracing::info!("RustContextBuilder: finished Adding all custom variables from context.");

        // Add template manifest variables if any
        for (key, value) in &template.manifest.variables {
            if !render_context.has_variable(key) {
                render_context.add_variable(key, value.clone());
            }
        }
        tracing::info!("RustContextBuilder: finished Adding template manifest variables if any.");

        // Add template manifest metadata
        render_context.add_variable("template_name", json!(template.manifest.name));
        render_context.add_variable("template_version", json!(template.manifest.version));
        if let Some(description) = &template.manifest.description {
            render_context.add_variable("template_description", json!(description));
        }

        tracing::info!("RustContextBuilder: Render context built successfully.");
        Ok(render_context)
    }
}

/// Parallel version of build_rust_endpoint_context that uses global schema cache
fn build_rust_endpoint_context_parallel(op: &Operation, global_cache: &Arc<GlobalSchemaCache>) -> Result<RustEndpointContext, GenerationError> {
    tracing::debug!("build_rust_endpoint_context_parallel: Starting for operation ID: {}", op.id);
    let endpoint_id = to_snake_case(&op.id);

    // Extract parameters and properties for unified handling
    let query_params = &op.parameters;
    let body_properties = extract_request_body_properties_parallel(op, global_cache);
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
        envelope_properties: extract_envelope_properties_parallel(op, global_cache),
        properties: body_properties,
        properties_for_handler: extract_handler_properties_parallel(op, global_cache),
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
        properties_schema: extract_properties_schema_parallel(op, global_cache),
        spec_file_name: None, // Would need to be passed from context
        // NEW: Unified parameter support for Issue #106
        unified_parameters,
        has_body_properties,
        http_method: op.method.to_uppercase(),
    })
}

/// Cached version of build_rust_endpoint_context that uses schema processing cache
fn build_rust_endpoint_context_cached(op: &Operation, cache: &mut ProcessedSchemaCache) -> Result<RustEndpointContext, GenerationError> {
    tracing::info!("build_rust_endpoint_context_cached: Starting for operation ID: {}", op.id);
    let endpoint_id = to_snake_case(&op.id);

    // Extract parameters and properties for unified handling
    let query_params = &op.parameters;
    let body_properties = extract_request_body_properties_cached(op, cache);
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
        envelope_properties: extract_envelope_properties_cached(op, cache),
        properties: body_properties,
        properties_for_handler: extract_handler_properties_cached(op, cache),
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
        properties_schema: extract_properties_schema_cached(op, cache),
        spec_file_name: None, // Would need to be passed from context
        // NEW: Unified parameter support for Issue #106
        unified_parameters,
        has_body_properties,
        http_method: op.method.to_uppercase(),
    })
}

fn build_rust_endpoint_context(op: &Operation) -> Result<RustEndpointContext, GenerationError> {
    tracing::info!("build_rust_endpoint_context: Starting for operation ID: {}", op.id);
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
        spec_file_name: None, // Would need to be passed from context
        // NEW: Unified parameter support for Issue #106
        unified_parameters,
        has_body_properties,
        http_method: op.method.to_uppercase(),
    })
}

fn extract_envelope_properties(op: &Operation) -> JsonValue {
    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(schema) = response.content_schema.as_ref()
        {
            return extract_typed_envelope_properties(schema);
        }
    }
    json!({})
}

fn extract_response_properties(op: &Operation) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(schema) = response.content_schema.as_ref()
        {
            properties.extend(extract_typed_schema_properties(schema));
        }
    }
    properties
}

fn extract_handler_properties(op: &Operation) -> Vec<String> {
    let handler_props: Vec<String> = extract_request_body_properties(op)
        .into_iter()
        .map(|prop| prop.name)
        .collect();
    handler_props
}

fn extract_parameters(op: &Operation) -> Vec<JsonValue> {
    let params: Vec<JsonValue> = op.parameters
        .iter()
        .map(|p| {
            json!({
                "name": p.original_name.clone(), // Use original_name for OpenAPI JSON
                "rust_name": to_snake_case(&p.name), // Keep sanitized name for Rust-specific context
                "target_type": map_schema_to_rust_type(&p.schema),
                "rust_type": map_schema_to_rust_type(&p.schema),  // Template expects rust_type
                "in": format!("{:?}", p.location).to_lowercase(),
                "required": p.required,
                "description": p.description.as_ref().map(|d| sanitize_markdown(d)),
                "example": serde_json::Value::Null
            })
        })
        .collect();
    params
}

fn extract_typed_envelope_properties(schema: &crate::generation::Schema) -> JsonValue {
    let mut json_props = serde_json::Map::new();

    if let Some(properties) = &schema.properties {
        for (key, schema_prop) in properties {
            if schema_prop.schema.unresolved_ref.is_some() {
                tracing::warn!(
                    "RustContextBuilder: Detected recursive reference in property '{}' for schema with title {:?}. Skipping property in envelope.",
                    key, schema.title
                );
                continue;
            }
            if let Ok(json_val) = serde_json::to_value(&schema_prop) {
                json_props.insert(key.clone(), json_val);
            }
        }
    }

    // NEW: Handle additionalProperties with recursive reference detection
    if let Some(additional_properties_wrapper) = &schema.additional_properties {
        // AdditionalProperties can be a boolean or a Schema
        match additional_properties_wrapper.as_ref() {
            crate::infrastructure::openapi::types::AdditionalProperties::Schema(additional_properties_schema) => {
                if additional_properties_schema.unresolved_ref.is_some() {
                    tracing::warn!(
                        "RustContextBuilder: Detected recursive reference in additionalProperties for schema with title {:?}. Replacing with placeholder in envelope.",
                        schema.title
                    );
                    // Replace with a simplified placeholder to prevent infinite recursion in Tera
                    json_props.insert(
                        "additionalProperties".to_string(),
                        json!({
                            "type": "object",
                            "description": format!("Recursive reference to {}", additional_properties_schema.unresolved_ref.as_ref().unwrap_or(&"unknown".to_string()))
                        }),
                    );
                } else {
                    // If not recursive, include additionalProperties in the schema map
                    if let Ok(json_val) = serde_json::to_value(additional_properties_schema) {
                        json_props.insert("additionalProperties".to_string(), json_val);
                    }
                }
            },
            crate::infrastructure::openapi::types::AdditionalProperties::Boolean(true) => {
                // If additionalProperties is true, it means any additional properties are allowed.
                // We can represent this as an empty object or a generic value in the context.
                json_props.insert("additionalProperties".to_string(), json!({ "type": "object" }));
            },
            crate::infrastructure::openapi::types::AdditionalProperties::Boolean(false) => {
                // If additionalProperties is false, no additional properties are allowed.
                // We don't need to add anything to the context for this.
            },
        }
    }

    if schema.schema_type.as_deref() == Some("array")
        && let Some(items) = &schema.items
    {
        // Recursively get properties from array items if it's an array
        let item_envelope_props = extract_typed_envelope_properties(items);
        if let Some(map) = item_envelope_props.as_object() {
            json_props.extend(map.clone());
        }
    }
    JsonValue::Object(json_props)
}

fn extract_typed_schema_properties(schema: &crate::generation::Schema) -> Vec<RustPropertyInfo> {
    let mut rust_properties = Vec::new();

    if let Some(properties) = &schema.properties {
        for (prop_name, schema_prop) in properties {
            let rust_type = map_schema_to_rust_type(&schema_prop.schema);
            let title = schema_prop.schema.title.clone();
            let description = schema_prop
                .schema
                .description
                .as_ref()
                .map(|d| sanitize_markdown(d));
            let example = schema_prop.schema.example.clone();

            rust_properties.push(RustPropertyInfo {
                name: sanitize_rust_field_name(prop_name),
                original_name: prop_name.clone(), // Populate original_name
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
        // NEW: Check for recursive reference in array items before recursing
        if items.unresolved_ref.is_some() {
            tracing::warn!(
                "RustContextBuilder: Detected recursive reference in array items for schema with title {:?}. Skipping recursion.",
                schema.title
            );
            // Return an empty vector or a placeholder to break recursion
            // For properties, an empty vector is appropriate as we don't want to generate properties from a recursive item.
        } else {
            rust_properties.extend(extract_typed_schema_properties(items));
        }
    }
    rust_properties
}

fn map_schema_to_rust_type(schema: &crate::generation::Schema) -> String {
    // If this schema is a placeholder for an unresolved recursive reference,
    // return a generic type to prevent infinite recursion during code generation.
    if schema.unresolved_ref.is_some() {
        tracing::warn!(
            "RustContextBuilder: Encountered unresolved recursive reference: {}. Mapping to serde_json::Value.",
            schema.unresolved_ref.as_ref().unwrap()
        );
        return "serde_json::Value".to_string();
    }

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

/// Iterative version of extract_typed_envelope_properties to avoid deep recursion
fn extract_typed_envelope_properties_iterative(schema: &crate::generation::Schema, max_depth: usize) -> JsonValue {
    let mut json_props = serde_json::Map::new();
    let mut stack = vec![(schema, 0)]; // (schema, depth)
    let mut visited = std::collections::HashSet::new();

    while let Some((current_schema, depth)) = stack.pop() {
        if depth >= max_depth {
            continue; // Skip if too deep
        }

        // Avoid processing the same schema multiple times
        if let Some(title) = &current_schema.title {
            if !visited.insert(title.clone()) {
                continue;
            }
        }

        if let Some(properties) = &current_schema.properties {
            for (key, schema_prop) in properties {
                if schema_prop.schema.unresolved_ref.is_some() {
                    tracing::warn!(
                        "RustContextBuilder: Detected recursive reference in property '{}' for schema with title {:?}. Skipping property in envelope.",
                        key, current_schema.title
                    );
                    continue;
                }
                if let Ok(json_val) = serde_json::to_value(&schema_prop) {
                    json_props.insert(key.clone(), json_val);
                }
            }
        }

        // Handle additionalProperties
        if let Some(additional_properties_wrapper) = &current_schema.additional_properties {
            match additional_properties_wrapper.as_ref() {
                crate::infrastructure::openapi::types::AdditionalProperties::Schema(additional_properties_schema) => {
                    if additional_properties_schema.unresolved_ref.is_some() {
                        tracing::warn!(
                            "RustContextBuilder: Detected recursive reference in additionalProperties for schema with title {:?}. Replacing with placeholder in envelope.",
                            current_schema.title
                        );
                        json_props.insert(
                            "additionalProperties".to_string(),
                            json!({
                                "type": "object",
                                "description": format!("Recursive reference to {}", additional_properties_schema.unresolved_ref.as_ref().unwrap_or(&"unknown".to_string()))
                            }),
                        );
                    } else {
                        if let Ok(json_val) = serde_json::to_value(additional_properties_schema) {
                            json_props.insert("additionalProperties".to_string(), json_val);
                        }
                    }
                },
                crate::infrastructure::openapi::types::AdditionalProperties::Boolean(true) => {
                    json_props.insert("additionalProperties".to_string(), json!({ "type": "object" }));
                },
                crate::infrastructure::openapi::types::AdditionalProperties::Boolean(false) => {
                    // Do nothing
                },
            }
        }

        // Handle arrays - add array items to processing stack
        if current_schema.schema_type.as_deref() == Some("array")
            && let Some(items) = &current_schema.items
        {
            if depth + 1 < max_depth {
                stack.push((items, depth + 1));
            }
        }
    }

    JsonValue::Object(json_props)
}

/// Iterative version of extract_typed_schema_properties to avoid deep recursion
fn extract_typed_schema_properties_iterative(schema: &crate::generation::Schema, max_depth: usize) -> Vec<RustPropertyInfo> {
    let mut rust_properties = Vec::new();
    let mut stack = vec![(schema, 0)]; // (schema, depth)
    let mut visited = std::collections::HashSet::new();

    while let Some((current_schema, depth)) = stack.pop() {
        if depth >= max_depth {
            continue; // Skip if too deep
        }

        // Avoid processing the same schema multiple times
        if let Some(title) = &current_schema.title {
            if !visited.insert(title.clone()) {
                continue;
            }
        }

        if let Some(properties) = &current_schema.properties {
            for (prop_name, schema_prop) in properties {
                let rust_type = map_schema_to_rust_type(&schema_prop.schema);
                let title = schema_prop.schema.title.clone();
                let description = schema_prop
                    .schema
                    .description
                    .as_ref()
                    .map(|d| sanitize_markdown(d));
                let example = schema_prop.schema.example.clone();

                rust_properties.push(RustPropertyInfo {
                    name: sanitize_rust_field_name(prop_name),
                    original_name: prop_name.clone(),
                    rust_type,
                    title,
                    description,
                    example,
                });
            }
        }

        // Handle arrays - add array items to processing stack
        if current_schema.schema_type.as_deref() == Some("array")
            && let Some(items) = &current_schema.items
        {
            if items.unresolved_ref.is_some() {
                tracing::warn!(
                    "RustContextBuilder: Detected recursive reference in array items for schema with title {:?}. Skipping recursion.",
                    current_schema.title
                );
            } else if depth + 1 < max_depth {
                stack.push((items, depth + 1));
            }
        }
    }

    rust_properties
}

/// Iterative version of extract_typed_properties_map to avoid deep recursion
fn extract_typed_properties_map_iterative(schema: &crate::generation::Schema, max_depth: usize) -> Option<JsonMap<String, JsonValue>> {
    let mut json_map = JsonMap::new();
    let mut stack = vec![(schema, 0)]; // (schema, depth)
    let mut visited = std::collections::HashSet::new();

    while let Some((current_schema, depth)) = stack.pop() {
        if depth >= max_depth {
            continue; // Skip if too deep
        }

        // Avoid processing the same schema multiple times
        if let Some(title) = &current_schema.title {
            if !visited.insert(title.clone()) {
                continue;
            }
        }

        if let Some(properties) = &current_schema.properties {
            for (key, schema_prop) in properties {
                if schema_prop.schema.unresolved_ref.is_some() {
                    tracing::warn!(
                        "RustContextBuilder: Detected recursive reference in property '{}' for schema with title {:?}. Skipping property in map.",
                        key, current_schema.title
                    );
                    continue;
                }
                if let Ok(json_val) = serde_json::to_value(&schema_prop) {
                    json_map.insert(key.clone(), json_val);
                }
            }
        }

        // Handle additionalProperties
        if let Some(additional_properties_wrapper) = &current_schema.additional_properties {
            match additional_properties_wrapper.as_ref() {
                crate::infrastructure::openapi::types::AdditionalProperties::Schema(additional_properties_schema) => {
                    if additional_properties_schema.unresolved_ref.is_some() {
                        tracing::warn!(
                            "RustContextBuilder: Detected recursive reference in additionalProperties for schema with title {:?}. Replacing with placeholder in map.",
                            current_schema.title
                        );
                        json_map.insert(
                            "additionalProperties".to_string(),
                            json!({
                                "type": "object",
                                "description": format!("Recursive reference to {}", additional_properties_schema.unresolved_ref.as_ref().unwrap_or(&"unknown".to_string()))
                            }),
                        );
                    } else {
                        if let Ok(json_val) = serde_json::to_value(additional_properties_schema) {
                            json_map.insert("additionalProperties".to_string(), json_val);
                        }
                    }
                },
                crate::infrastructure::openapi::types::AdditionalProperties::Boolean(true) => {
                    json_map.insert("additionalProperties".to_string(), json!({ "type": "object" }));
                },
                crate::infrastructure::openapi::types::AdditionalProperties::Boolean(false) => {
                    // Do nothing
                },
            }
        }

        // Handle arrays - add array items to processing stack
        if current_schema.schema_type.as_deref() == Some("array")
            && let Some(items) = &current_schema.items
        {
            if depth + 1 < max_depth {
                stack.push((items, depth + 1));
            }
        }
    }

    if json_map.is_empty() {
        None
    } else {
        Some(json_map)
    }
}

/// Helper function to convert a Schema to a simplified JsonValue,
/// especially for recursive references to prevent excessive data in templates.
fn schema_to_simplified_json(schema: &crate::generation::Schema) -> JsonValue {
    if schema.unresolved_ref.is_some() {
        tracing::warn!(
            "RustContextBuilder: Simplifying JSON for recursive reference: {}",
            schema.unresolved_ref.as_ref().unwrap_or(&"unknown".to_string())
        );
        json!({
            "type": "object", // Generic type for simplified representation
            "description": format!("Recursive reference to {}", schema.unresolved_ref.as_ref().unwrap_or(&"unknown".to_string()))
        })
    } else {
        // Attempt to serialize the full schema. Handle potential errors.
        serde_json::to_value(schema).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize schema to JSON: {}", e);
            json!({}) // Return empty object on serialization error
        })
    }
}

// Removed map_json_schema_to_rust_type - now using map_schema_to_rust_type for typed schemas

fn extract_properties_schema(op: &Operation) -> JsonMap<String, JsonValue> {
    if let Some(request_body) = &op.request_body
        && let Some(schema) = request_body.content_schema.as_ref()
        // For properties_schema, we still want the full map, but individual properties
        // within that map are already handled for recursive refs in extract_typed_properties_map.
        // So, we don't apply schema_to_simplified_json to the root schema here,
        // but rely on the inner function's logic.
        && let Some(properties) = extract_typed_properties_map(schema)
    {
        return properties;
    }
    JsonMap::new()
}

fn extract_response_schema(op: &Operation) -> JsonValue {
    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(schema) = response.content_schema.as_ref()
        {
            // Use the helper function to simplify the response schema if it's recursive
            return schema_to_simplified_json(schema);
        }
    }
    json!({})
}

fn extract_valid_fields(op: &Operation) -> Vec<String> {
    let valid_fields: Vec<String> = extract_response_properties(op)
        .into_iter()
        .map(|prop| prop.name)
        .collect();
    valid_fields
}

fn extract_typed_properties_map(
    schema: &crate::generation::Schema,
) -> Option<JsonMap<String, JsonValue>> {
    let mut json_map = JsonMap::new();

    if let Some(properties) = &schema.properties {
        for (key, schema_prop) in properties {
            if schema_prop.schema.unresolved_ref.is_some() {
                tracing::warn!(
                    "RustContextBuilder: Detected recursive reference in property '{}' for schema with title {:?}. Skipping property in map.",
                    key, schema.title
                );
                continue;
            }
            if let Ok(json_val) = serde_json::to_value(&schema_prop) {
                json_map.insert(key.clone(), json_val);
            }
        }
    }

    // NEW: Handle additionalProperties with recursive reference detection
    if let Some(additional_properties_wrapper) = &schema.additional_properties {
        // AdditionalProperties can be a boolean or a Schema
        match additional_properties_wrapper.as_ref() {
            crate::infrastructure::openapi::types::AdditionalProperties::Schema(additional_properties_schema) => {
                if additional_properties_schema.unresolved_ref.is_some() {
                    tracing::warn!(
                        "RustContextBuilder: Detected recursive reference in additionalProperties for schema with title {:?}. Replacing with placeholder in map.",
                        schema.title
                    );
                    // Replace with a simplified placeholder to prevent infinite recursion in Tera
                    json_map.insert(
                        "additionalProperties".to_string(),
                        json!({
                            "type": "object",
                            "description": format!("Recursive reference to {}", additional_properties_schema.unresolved_ref.as_ref().unwrap_or(&"unknown".to_string()))
                        }),
                    );
                } else {
                    // If not recursive, include additionalProperties in the schema map
                    if let Ok(json_val) = serde_json::to_value(additional_properties_schema) {
                        json_map.insert("additionalProperties".to_string(), json_val);
                    }
                }
            },
            crate::infrastructure::openapi::types::AdditionalProperties::Boolean(true) => {
                json_map.insert("additionalProperties".to_string(), json!({ "type": "object" }));
            },
            crate::infrastructure::openapi::types::AdditionalProperties::Boolean(false) => {
                // Do nothing
            },
        }
    }

    if schema.schema_type.as_deref() == Some("array")
        && let Some(items) = &schema.items
    {
        // Recursively get properties from array items if it's an array
        if let Some(item_props) = extract_typed_properties_map(items) {
            json_map.extend(item_props);
        }
    }

    if json_map.is_empty() {
        None
    } else {
        Some(json_map)
    }
}

fn is_array_response(op: &Operation) -> bool {
    let is_arr = if let Some(schema) = get_typed_response_schema(op) {
        schema.schema_type.as_deref() == Some("array")
    } else {
        false
    };
    is_arr
}

fn is_object_response(op: &Operation) -> bool {
    let is_obj = if let Some(schema) = get_typed_response_schema(op) {
        schema.schema_type.as_deref() == Some("object") || schema.properties.is_some()
    } else {
        false
    };
    is_obj
}

fn is_primitive_response(op: &Operation) -> bool {
    let is_prim = if let Some(schema) = get_typed_response_schema(op) {
        matches!(
            schema.schema_type.as_deref(),
            Some("string") | Some("integer") | Some("number") | Some("boolean")
        )
    } else {
        false
    };
    is_prim
}

fn get_array_item_type(op: &Operation) -> String {
    let item_type = if is_array_response(op)
        && let Some(schema) = get_typed_response_schema(op)
        && let Some(items) = &schema.items
    {
        map_schema_to_rust_type(items)
    } else {
        "serde_json::Value".to_string()
    };
    item_type
}

fn get_primitive_type(op: &Operation) -> String {
    let prim_type = if is_primitive_response(op)
        && let Some(schema) = get_typed_response_schema(op)
    {
        map_schema_to_rust_type(&schema)
    } else {
        "serde_json::Value".to_string()
    };
    prim_type
}
/// Cached version of extract_request_body_properties
fn extract_request_body_properties_cached(op: &Operation, cache: &mut ProcessedSchemaCache) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    if let Some(request_body) = &op.request_body
        && let Some(schema) = request_body.content_schema.as_ref()
    {
        let cache_key = format!("request_body_{}", op.id);
        properties.extend(cache.get_or_compute_schema_properties(schema, &cache_key));
    }
    properties
}

/// Cached version of extract_envelope_properties
fn extract_envelope_properties_cached(op: &Operation, cache: &mut ProcessedSchemaCache) -> JsonValue {
    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(schema) = response.content_schema.as_ref()
        {
            let cache_key = format!("envelope_{}", op.id);
            return cache.get_or_compute_envelope(schema, &cache_key);
        }
    }
    json!({})
}

/// Cached version of extract_response_properties
fn extract_response_properties_cached(op: &Operation, cache: &mut ProcessedSchemaCache) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(schema) = response.content_schema.as_ref()
        {
            let cache_key = format!("response_props_{}", op.id);
            properties.extend(cache.get_or_compute_schema_properties(schema, &cache_key));
        }
    }
    properties
}

/// Cached version of extract_handler_properties
fn extract_handler_properties_cached(op: &Operation, cache: &mut ProcessedSchemaCache) -> Vec<String> {
    let handler_props: Vec<String> = extract_request_body_properties_cached(op, cache)
        .into_iter()
        .map(|prop| prop.name)
        .collect();
    handler_props
}

/// Cached version of extract_properties_schema
fn extract_properties_schema_cached(op: &Operation, cache: &mut ProcessedSchemaCache) -> JsonMap<String, JsonValue> {
    if let Some(request_body) = &op.request_body
        && let Some(schema) = request_body.content_schema.as_ref()
        && let Some(properties) = {
            let cache_key = format!("properties_map_{}", op.id);
            cache.get_or_compute_properties_map(schema, &cache_key)
        }
    {
        return properties;
    }
    JsonMap::new()
}

/// Cached version of extract_valid_fields
fn extract_valid_fields_cached(op: &Operation, cache: &mut ProcessedSchemaCache) -> Vec<String> {
    let valid_fields: Vec<String> = extract_response_properties_cached(op, cache)
        .into_iter()
        .map(|prop| prop.name)
        .collect();
    valid_fields
}

/// Parallel version of extract_request_body_properties using global cache
fn extract_request_body_properties_parallel(op: &Operation, global_cache: &Arc<GlobalSchemaCache>) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    if let Some(request_body) = &op.request_body
        && let Some(schema) = request_body.content_schema.as_ref()
    {
        let cache_key = format!("request_body_{}", op.id);
        properties.extend(global_cache.get_or_compute_schema_properties(schema, &cache_key));
    }
    properties
}

/// Parallel version of extract_envelope_properties using global cache
fn extract_envelope_properties_parallel(op: &Operation, global_cache: &Arc<GlobalSchemaCache>) -> JsonValue {
    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(schema) = response.content_schema.as_ref()
        {
            let cache_key = format!("envelope_{}", op.id);
            return global_cache.get_or_compute_envelope(schema, &cache_key);
        }
    }
    json!({})
}

/// Parallel version of extract_handler_properties using global cache
fn extract_handler_properties_parallel(op: &Operation, global_cache: &Arc<GlobalSchemaCache>) -> Vec<String> {
    let handler_props: Vec<String> = extract_request_body_properties_parallel(op, global_cache)
        .into_iter()
        .map(|prop| prop.name)
        .collect();
    handler_props
}

/// Parallel version of extract_properties_schema using global cache
fn extract_properties_schema_parallel(op: &Operation, global_cache: &Arc<GlobalSchemaCache>) -> JsonMap<String, JsonValue> {
    if let Some(request_body) = &op.request_body
        && let Some(schema) = request_body.content_schema.as_ref()
        && let Some(properties) = {
            let cache_key = format!("properties_map_{}", op.id);
            global_cache.get_or_compute_properties_map(schema, &cache_key)
        }
    {
        return properties;
    }
    JsonMap::new()
}

fn extract_request_body_properties(op: &Operation) -> Vec<RustPropertyInfo> {
    let mut properties = Vec::new();

    if let Some(request_body) = &op.request_body
        && let Some(schema) = request_body.content_schema.as_ref()
    {
        properties.extend(extract_typed_schema_properties(schema));
    }
    properties
}

fn get_typed_response_schema(op: &Operation) -> Option<crate::generation::Schema> {
    // Look for successful response
    for response in &op.responses {
        if response.status_code.starts_with('2')
            && let Some(schema) = response.content_schema.as_ref()
        {
            return Some(schema.clone());
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

        let mut grouped_by_base_sanitized_name: HashMap<String, Vec<UnifiedParameter>> = HashMap::new();

    for param in query_params {
        let snake_name = to_snake_case(&param.original_name);
        let base_sanitized_name = sanitize_rust_identifier(&snake_name, false);
        let rust_type = if param.required {
            map_schema_to_rust_type(&param.schema)
        } else {
            format!("Option<{}>", map_schema_to_rust_type(&param.schema))
        };
        grouped_by_base_sanitized_name.entry(base_sanitized_name).or_default().push(UnifiedParameter {
            name: String::new(), // Placeholder for now
            original_name: param.original_name.clone(),
            source: ParameterSource::Query,
            rust_type,
            description: param.description.clone(),
            required: param.required,
        });
    }

    for prop in body_properties {
        let snake_name = to_snake_case(&prop.original_name);
        let base_sanitized_name = sanitize_rust_identifier(&snake_name, false);
        // Body properties are always optional in the generated struct
        grouped_by_base_sanitized_name.entry(base_sanitized_name).or_default().push(UnifiedParameter {
            name: String::new(), // Placeholder for now
            original_name: prop.original_name.clone(),
            source: ParameterSource::Body,
            rust_type: format!("Option<{}>", prop.rust_type.clone()), // Always wrap body properties in Option
            description: prop.description.clone(),
            required: false, // Body properties are treated as optional in the unified struct
        });
    }

    let mut final_unified_parameters = Vec::new();
    let mut used_final_names: HashMap<String, usize> = HashMap::new(); // To catch collisions after initial suffixing

    for (base_sanitized_name, params_in_group) in grouped_by_base_sanitized_name {
        if params_in_group.len() == 1 {
            let mut p = params_in_group.into_iter().next().unwrap();
            p.name = base_sanitized_name.clone(); // No collision, use base name
            final_unified_parameters.push(p);
            used_final_names.insert(base_sanitized_name, 1);
        } else {
            // Collision detected for this base_sanitized_name
            for mut p in params_in_group {
                let mut current_suffix = match p.source {
                    ParameterSource::Path => "_p".to_string(),
                    ParameterSource::Query => "_q".to_string(),
                    ParameterSource::Body => "_b".to_string(),
                };
                let mut final_name = format!("{}{}", base_sanitized_name, current_suffix);

                let mut counter = 1;
                while used_final_names.contains_key(&final_name) {
                    current_suffix = format!("{}_{}", current_suffix, counter);
                    final_name = format!("{}{}", base_sanitized_name, current_suffix);
                    counter += 1;
                }
                p.name = final_name.clone();
                final_unified_parameters.push(p);
                used_final_names.insert(final_name, 1);
            }
        }
    }

    final_unified_parameters
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
        let query_params = vec![create_test_parameter("limit", "integer", false)]; // Added 'false' for required
        let body_properties = vec![create_test_property("query", "String")];

        // WHEN: build_unified_parameters() called
        let result = build_unified_parameters(&query_params, &body_properties);

        // THEN: Original names preserved, no suffixes
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "limit");
        assert_eq!(result[0].original_name, "limit");
        assert!(matches!(result[0].source, ParameterSource::Query));
        assert_eq!(result[0].rust_type, "Option<i32>"); // Now optional
        assert_eq!(result[1].name, "query");
        assert_eq!(result[1].original_name, "query");
        assert!(matches!(result[1].source, ParameterSource::Body));
        assert_eq!(result[1].rust_type, "String"); // Body properties are always optional in the generated struct

    }

    #[test]
    fn test_build_unified_parameters_with_collision() {
        // GIVEN: Query param "limit" and body prop "limit" (collision!)
        let query_params = vec![create_test_parameter("limit", "integer", false)]; // Not required
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
        assert_eq!(query_param.rust_type, "Option<i32>"); // Now optional
        assert!(!query_param.required);

        let body_param = result
            .iter()
            .find(|p| matches!(p.source, ParameterSource::Body))
            .unwrap();
        assert_eq!(body_param.name, "limit_b");
        assert_eq!(body_param.original_name, "limit");
        assert_eq!(body_param.rust_type, "Option<i32>"); // Body properties are always optional in the generated struct
        assert!(!body_param.required);
    }

    #[test]
    fn test_build_unified_parameters_multiple_collisions() {
        // GIVEN: Multiple colliding names
        let query_params = vec![
            create_test_parameter("limit", "integer", false), // Not required
            create_test_parameter("format", "string", true), // Required
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
        assert_eq!(limit_q.rust_type, "Option<i32>"); // Now optional
        let limit_b = result.iter().find(|p| p.name == "limit_b").unwrap();
        assert!(matches!(limit_b.source, ParameterSource::Body));
        assert_eq!(limit_b.rust_type, "i32"); // Body properties are always optional in the generated struct

        // Check "format" collision
        let format_q = result.iter().find(|p| p.name == "format_q").unwrap();
        assert!(matches!(format_q.source, ParameterSource::Query));
        assert_eq!(format_q.rust_type, "String"); // Required, so not Option
        assert!(format_q.required);
        let format_b = result.iter().find(|p| p.name == "format_b").unwrap();
        assert!(matches!(format_b.source, ParameterSource::Body));
        assert_eq!(format_b.rust_type, "Option<String>"); // Body properties are always optional in the generated struct
        assert!(!format_b.required);

        // Check no collision
        let query_param = result.iter().find(|p| p.name == "query").unwrap();
        assert!(matches!(query_param.source, ParameterSource::Body));
        assert_eq!(query_param.rust_type, "Option<String>"); // Body properties are always optional in the generated struct
        assert!(!query_param.required);
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

        // Verify the rust_type for the query parameter
        let limit_param = context.unified_parameters.iter().find(|p| p.original_name == "limit").unwrap();
        assert_eq!(limit_param.rust_type, "Option<i32>"); // Not required, so Option
        assert!(!limit_param.required);
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

        // Verify the rust_type for the query parameter
        let limit_param = context.unified_parameters.iter().find(|p| p.original_name == "limit").unwrap();
        assert_eq!(limit_param.rust_type, "Option<i32>"); // Not required, so Option
        assert!(!limit_param.required);
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

        // Verify the rust_type for the body property
        let query_prop = context.unified_parameters.iter().find(|p| p.original_name == "query").unwrap();
        assert_eq!(query_prop.rust_type, "Option<String>"); // Body properties are always optional in the generated struct
        assert!(!query_prop.required);
    }

    // Helper functions for tests
    fn create_test_parameter(name: &str, schema_type: &str, required: bool) -> crate::generation::Parameter {
        use crate::generation::{Parameter, ParameterLocation};
        use crate::infrastructure::openapi::types::Schema;
        Parameter {
            name: name.to_string(),
            original_name: name.to_string(), // Added original_name
            location: ParameterLocation::Query,
            required, // Use the provided required value
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
                unresolved_ref: None,
            },
            description: None,
        }
    }

    fn create_test_property(name: &str, rust_type: &str) -> RustPropertyInfo {
        RustPropertyInfo {
            name: sanitize_rust_field_name(name), // Sanitized name
            original_name: name.to_string(), // Original name
            rust_type: rust_type.to_string(),
            title: None,
            description: None,
            example: None,
        }
    }

    fn create_test_operation_with_both_params_and_body() -> crate::generation::Operation {
        use crate::generation::{Operation, RequestBody};

        Operation {
            id: "testOp".to_string(),
            path: "/test".to_string(),
            method: "POST".to_string(),
            summary: Some("Test operation".to_string()),
            description: None,
            external_docs: None,
            tags: None,
            parameters: vec![create_test_parameter("limit", "integer", false)], // Not required
            request_body: Some(RequestBody {
                description: None,
                content_schema: Some(crate::infrastructure::openapi::Schema { // Use content_schema
                    schema_type: Some("object".to_string()),
                    properties: Some(indexmap::IndexMap::from([
                        ("query".to_string(), crate::infrastructure::openapi::SchemaProperty {
                            name: "query".to_string(),
                            original_name: "query".to_string(),
                            schema: crate::infrastructure::openapi::Schema {
                                schema_type: Some("string".to_string()),
                                ..Default::default()
                            }
                        })
                    ])),
                    ..Default::default()
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
            parameters: vec![create_test_parameter("limit", "integer", false)], // Not required
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
                content_schema: Some(crate::infrastructure::openapi::Schema { // Use content_schema
                    schema_type: Some("object".to_string()),
                    properties: Some(indexmap::IndexMap::from([
                        ("query".to_string(), crate::infrastructure::openapi::SchemaProperty {
                            name: "query".to_string(),
                            original_name: "query".to_string(),
                            schema: crate::infrastructure::openapi::Schema {
                                schema_type: Some("string".to_string()),
                                ..Default::default()
                            }
                        })
                    ])),
                    ..Default::default()
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

    fn create_recursive_property_schema() -> crate::infrastructure::openapi::Schema {
        use crate::infrastructure::openapi::Schema;
        // This simulates a recursive reference to Property itself
        // The actual resolution happens earlier, setting `unresolved_ref`
        Schema {
            schema_type: Some("object".to_string()),
            title: Some("Property".to_string()),
            unresolved_ref: Some("#/components/schemas/Property".to_string()), // This is the key
            ..Default::default()
        }
    }

    #[test]
    fn test_extract_typed_properties_map_with_recursive_additional_properties() {
        // GIVEN: A schema with recursive additionalProperties
        let recursive_additional_props_schema = create_recursive_property_schema();
        let schema_with_recursive_additional_props = crate::infrastructure::openapi::Schema {
            schema_type: Some("object".to_string()),
            properties: Some(indexmap::IndexMap::from([
                ("id".to_string(), crate::infrastructure::openapi::SchemaProperty {
                    name: "id".to_string(),
                    original_name: "id".to_string(),
                    schema: crate::infrastructure::openapi::Schema {
                        schema_type: Some("string".to_string()),
                        ..Default::default()
                    }
                })
            ])),
            additional_properties: Some(Box::new(crate::infrastructure::openapi::types::AdditionalProperties::Schema(Box::new(recursive_additional_props_schema)))),
            title: Some("TestSchemaWithRecursiveAdditionalProps".to_string()),
            ..Default::default()
        };

        // WHEN: extract_typed_properties_map is called
        let result = extract_typed_properties_map(&schema_with_recursive_additional_props);

        // THEN: additionalProperties should be in the resulting map with a placeholder
        assert!(result.is_some());
        let json_map = result.unwrap();
        assert!(json_map.contains_key("id"));
        assert!(json_map.contains_key("additionalProperties")); // Changed assertion
        let additional_props_value = &json_map["additionalProperties"];
        assert_eq!(additional_props_value["type"], "object");
        assert!(additional_props_value["description"].as_str().unwrap().contains("Recursive reference"));
    }

    #[test]
    fn test_extract_typed_envelope_properties_with_recursive_additional_properties() {
        // GIVEN: A schema with recursive additionalProperties
        let recursive_additional_props_schema = create_recursive_property_schema();
        let schema_with_recursive_additional_props = crate::infrastructure::openapi::Schema {
            schema_type: Some("object".to_string()),
            properties: Some(indexmap::IndexMap::from([
                ("id".to_string(), crate::infrastructure::openapi::SchemaProperty {
                    name: "id".to_string(),
                    original_name: "id".to_string(),
                    schema: crate::infrastructure::openapi::Schema {
                        schema_type: Some("string".to_string()),
                        ..Default::default()
                    }
                })
            ])),
            additional_properties: Some(Box::new(crate::infrastructure::openapi::types::AdditionalProperties::Schema(Box::new(recursive_additional_props_schema)))),
            title: Some("TestSchemaWithRecursiveAdditionalProps".to_string()),
            ..Default::default()
        };

        // WHEN: extract_typed_envelope_properties is called
        let result = extract_typed_envelope_properties(&schema_with_recursive_additional_props);

        // THEN: additionalProperties should be in the resulting JsonValue with a placeholder
        assert!(result.is_object());
        let json_obj = result.as_object().unwrap();
        assert!(json_obj.contains_key("id"));
        assert!(json_obj.contains_key("additionalProperties")); // Changed assertion
        let additional_props_value = &json_obj["additionalProperties"];
        assert_eq!(additional_props_value["type"], "object");
        assert!(additional_props_value["description"].as_str().unwrap().contains("Recursive reference"));
    }

    #[test]
    fn test_required_query_parameter() {
        // GIVEN: A query parameter that is required
        let query_params = vec![create_test_parameter("required_param", "string", true)];
        let body_properties = vec![];

        // WHEN: build_unified_parameters() is called
        let result = build_unified_parameters(&query_params, &body_properties);

        // THEN: The rust_type should be String (not Option<String>)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "required_param");
        assert_eq!(result[0].rust_type, "String");
        assert!(result[0].required);
    }

    #[test]
    fn test_optional_query_parameter() {
        // GIVEN: A query parameter that is NOT required
        let query_params = vec![create_test_parameter("optional_param", "string", false)];
        let body_properties = vec![];

        // WHEN: build_unified_parameters() is called
        let result = build_unified_parameters(&query_params, &body_properties);

        // THEN: The rust_type should be Option<String>
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "optional_param");
        assert_eq!(result[0].rust_type, "Option<String>"); // Now it's Option<String> directly
        assert!(!result[0].required);
    }
}
