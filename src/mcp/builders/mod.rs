//! Context builder traits and adapters for language-specific codegen.
//!
//! This module provides the infrastructure for converting OpenAPI operations into
//! language-specific contexts that can be used in template rendering. Each supported
//! target language has its own builder implementation that handles the specifics
//! of that language's naming conventions, type mappings, and code generation patterns.

pub mod rust;

use crate::core::openapi::OpenApiOperation;
use crate::core::templates::{ClientTemplateKind, ServerTemplateKind};
use serde_json::Value as JsonValue;

/// Trait for building language-specific contexts for code generation.
///
/// Implementations of this trait are responsible for transforming generic
/// context data into structured contexts that templates can use to generate
/// idiomatic code for specific programming languages.
///
/// The builder should handle:
/// - Language-specific naming conventions (e.g., snake_case for Rust, camelCase for JavaScript)
/// - Type mappings and default values
/// - Parameter and response structure organization
/// - Any language-specific metadata needed for code generation
pub trait LanguageContextBuilder {
    /// Build a language-specific context from generic input.
    ///
    /// # Arguments
    /// * `context` - The generic context to transform
    ///
    /// # Returns
    /// A JSON value containing the language-specific context data for template rendering
    ///
    /// # Errors
    /// Returns an error if the context cannot be transformed
    fn build(&self, context: &JsonValue) -> crate::core::error::Result<JsonValue>;
}

/// Legacy trait for OpenAPI endpoint context building.
/// This is used for server generation from OpenAPI specs.
pub trait EndpointContextBuilder {
    /// Build a language-specific context from an OpenAPI operation.
    fn build(&self, op: &OpenApiOperation) -> crate::core::error::Result<JsonValue>;
}

/// Factory for creating and managing endpoint context builders.
///
/// This struct provides the main interface for transforming OpenAPI operations
/// into language-specific contexts suitable for code generation.
pub struct EndpointContext;

impl EndpointContext {
    /// Transform a list of OpenAPI operations into language-specific endpoint contexts.
    ///
    /// This method converts all operations using the appropriate language-specific builder
    /// and returns them sorted alphabetically by endpoint name for consistent output.
    ///
    /// # Arguments
    /// * `template` - The target template kind that determines which builder to use
    /// * `operations` - The list of OpenAPI operations to transform
    ///
    /// # Returns
    /// A vector of JSON values representing the language-specific endpoint contexts,
    /// sorted alphabetically by endpoint name
    ///
    /// # Errors
    /// Returns an error if any operation cannot be converted to the target language context
    ///
    /// # Examples
    /// ```no_run
    /// use agenterra_mcp::builders::EndpointContext;
    /// use agenterra_mcp::templates::ServerTemplateKind;
    /// # use agenterra_core::openapi::OpenApiOperation;
    ///
    /// # fn example(operations: Vec<OpenApiOperation>) -> agenterra_core::Result<()> {
    /// let contexts = EndpointContext::transform_endpoints(
    ///     ServerTemplateKind::RustAxum,
    ///     operations
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn transform_endpoints(
        template: ServerTemplateKind,
        operations: Vec<OpenApiOperation>,
    ) -> crate::core::error::Result<Vec<JsonValue>> {
        let builder = Self::get_builder(template);
        let mut contexts = Vec::new();
        for op in operations {
            contexts.push(builder.build(&op)?);
        }

        // Sort endpoints alphabetically by endpoint name for consistent output
        contexts.sort_by(|a, b| {
            let name_a = a.get("endpoint").and_then(|v| v.as_str()).unwrap_or("");
            let name_b = b.get("endpoint").and_then(|v| v.as_str()).unwrap_or("");
            name_a.cmp(name_b)
        });

        Ok(contexts)
    }

    /// Get the appropriate context builder for a given server template kind.
    ///
    /// # Arguments
    /// * `template` - The server template kind to get a builder for
    ///
    /// # Returns
    /// A boxed trait object implementing `EndpointContextBuilder` for the specified server template
    ///
    /// # Panics
    /// Panics if the template kind is not supported (via `unimplemented!` macro)
    pub fn get_builder(template: ServerTemplateKind) -> Box<dyn EndpointContextBuilder> {
        match template {
            ServerTemplateKind::RustAxum => Box::new(rust::RustEndpointContextBuilder),
            _ => unimplemented!("Builder not implemented for template: {:?}", template),
        }
    }

    /// Get the appropriate context builder for a given client template kind.
    ///
    /// # Arguments
    /// * `template` - The client template kind to get a builder for
    ///
    /// # Returns
    /// A boxed trait object implementing `LanguageContextBuilder` for the specified client template
    ///
    /// # Panics
    /// Panics if the template kind is not supported (via `unimplemented!` macro)
    pub fn get_client_builder(template: ClientTemplateKind) -> Box<dyn LanguageContextBuilder> {
        match template {
            ClientTemplateKind::RustReqwest => Box::new(rust::RustMcpClientContextBuilder),
            _ => unimplemented!(
                "Client builder not implemented for template: {:?}",
                template
            ),
        }
    }
}
