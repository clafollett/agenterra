//! Generation context - the core aggregate for the generation domain

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::generation::{Language, Operation};
use crate::infrastructure::templates::TemplateDescriptor;
use crate::protocols::{Protocol, Role};

/// The main generation context that flows through the generation workflow
#[derive(Debug, Clone)]
pub struct GenerationContext {
    pub protocol: Protocol,
    pub role: Role,
    pub language: Language,
    pub template_descriptor: TemplateDescriptor,
    pub variables: HashMap<String, JsonValue>,
    pub operations: Vec<Operation>,
    pub metadata: GenerationMetadata,
    pub openapi_spec: Option<crate::generation::OpenApiSpec>,
}

/// Metadata about the generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    pub project_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
}

impl GenerationContext {
    /// Create a new generation context
    pub fn new(protocol: Protocol, role: Role, language: Language) -> Self {
        let template_descriptor = TemplateDescriptor::new(protocol, role.clone(), language);

        Self {
            protocol,
            role,
            language,
            template_descriptor,
            variables: HashMap::new(),
            operations: Vec::new(),
            metadata: GenerationMetadata::default(),
            openapi_spec: None,
        }
    }

    /// Add a variable to the context
    pub fn add_variable(&mut self, key: String, value: JsonValue) {
        self.variables.insert(key, value);
    }

    /// Add multiple variables at once
    pub fn add_variables(&mut self, vars: HashMap<String, JsonValue>) {
        self.variables.extend(vars);
    }

    /// Set the operations from OpenAPI
    pub fn set_operations(&mut self, operations: Vec<Operation>) {
        self.operations = operations;
    }

    /// Validate the context has all required data
    pub fn validate(&self) -> Result<(), crate::generation::GenerationError> {
        // Validate metadata
        if self.metadata.project_name.is_empty() {
            return Err(crate::generation::GenerationError::ValidationError(
                "Project name is required".to_string(),
            ));
        }

        // Validate role is supported by protocol
        self.protocol.validate_role(&self.role).map_err(|e| {
            crate::generation::GenerationError::ValidationError(format!(
                "Invalid role for protocol: {}",
                e
            ))
        })?;

        // Additional validation can be added here

        Ok(())
    }
}

impl Default for GenerationMetadata {
    fn default() -> Self {
        Self {
            project_name: String::new(),
            version: "0.1.0".to_string(),
            description: None,
            author: None,
            license: None,
            repository: None,
        }
    }
}

/// Render context used for template rendering
#[derive(Debug, Clone)]
pub struct RenderContext {
    pub data: JsonValue,
    pub variables: HashMap<String, JsonValue>,
}

impl RenderContext {
    /// Create a new render context
    pub fn new() -> Self {
        Self {
            data: JsonValue::Object(serde_json::Map::new()),
            variables: HashMap::new(),
        }
    }

    /// Add a variable to the render context
    pub fn add_variable(&mut self, key: &str, value: JsonValue) {
        self.variables.insert(key.to_string(), value.clone());

        // Also add to data for backward compatibility
        if let Some(obj) = self.data.as_object_mut() {
            obj.insert(key.to_string(), value);
        }
    }

    /// Check if a variable exists
    pub fn has_variable(&self, key: &str) -> bool {
        self.variables.contains_key(key)
    }

    /// Get a variable value
    pub fn get_variable(&self, key: &str) -> Option<&JsonValue> {
        self.variables.get(key)
    }

    /// Convert to Tera context
    pub fn to_tera_context(&self) -> tera::Context {
        let mut context = tera::Context::new();

        // Add all variables to the Tera context
        for (key, value) in &self.variables {
            context.insert(key, value);
        }

        context
    }
}

impl Default for RenderContext {
    fn default() -> Self {
        Self::new()
    }
}
