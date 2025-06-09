//! Context builder traits and adapters for language-specific codegen.
pub mod rust;

use crate::openapi::OpenApiOperation;
use crate::templates::TemplateKind;
use serde_json::Value as JsonValue;

/// Trait for converting an OpenApiOperation into a language-specific context.
pub trait EndpointContextBuilder {
    fn build(&self, op: &OpenApiOperation) -> crate::Result<JsonValue>;
}

pub struct EndpointContext;

impl EndpointContext {
    /// Transform a list of OpenAPI operations into language-specific endpoint contexts
    /// The returned contexts are sorted alphabetically by endpoint name for consistent output
    pub fn transform_endpoints(
        template: TemplateKind,
        operations: Vec<OpenApiOperation>,
    ) -> crate::Result<Vec<JsonValue>> {
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

    pub fn get_builder(template: TemplateKind) -> Box<dyn EndpointContextBuilder> {
        match template {
            TemplateKind::RustAxum => Box::new(rust::RustEndpointContextBuilder),
            _ => unimplemented!("Builder not implemented for template: {:?}", template),
        }
    }
}
