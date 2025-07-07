//! Python-specific context builder for code generation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::generation::{
    ContextBuilder, GenerationContext, GenerationError, Language, Operation, RenderContext,
    sanitizers::sanitize_markdown,
    utils::{to_proper_case, to_snake_case},
};
use crate::infrastructure::Template;

/// Python-specific property information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PythonPropertyInfo {
    pub name: String,
    pub python_type: String,
    pub type_hint: String,
    pub description: Option<String>,
    pub example: Option<JsonValue>,
}

/// Python-specific context builder
pub struct PythonContextBuilder;

impl PythonContextBuilder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ContextBuilder for PythonContextBuilder {
    async fn build(
        &self,
        context: &GenerationContext,
        template: &Template,
    ) -> Result<RenderContext, GenerationError> {
        if context.language != Language::Python {
            return Err(GenerationError::InvalidConfiguration(format!(
                "PythonContextBuilder can only build contexts for Python, got {:?}",
                context.language
            )));
        }

        let mut render_context = RenderContext::new();

        // Base context
        render_context.add_variable("project_name", json!(context.metadata.project_name));
        render_context.add_variable("version", json!(context.metadata.version));
        render_context.add_variable("description", json!(context.metadata.description));
        render_context.add_variable("protocol", json!(context.protocol.to_string()));
        render_context.add_variable("role", json!(context.role.to_string()));
        render_context.add_variable("language", json!("python"));

        // Python-specific naming
        let package_name = to_snake_case(&context.metadata.project_name);
        let module_name = to_snake_case(&context.metadata.project_name);
        let class_name = to_proper_case(&context.metadata.project_name);

        render_context.add_variable("package_name", json!(package_name));
        render_context.add_variable("module_name", json!(module_name));
        render_context.add_variable("class_name", json!(class_name));
        render_context.add_variable("cli_script_name", json!(package_name));

        // Handle protocol-specific context
        let mut endpoints = Vec::new();
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

                    // Process operations into Python endpoint contexts
                    for operation in operations {
                        let endpoint_context = build_python_endpoint_context(operation)?;
                        endpoints.push(endpoint_context);
                    }
                }
            }
        }
        render_context.add_variable("endpoints", json!(endpoints));

        // Add custom variables
        for (key, value) in &context.variables {
            render_context.add_variable(key, value.clone());
        }

        // Add template variables
        for (key, value) in &template.manifest.variables {
            if !render_context.has_variable(key) {
                render_context.add_variable(key, value.clone());
            }
        }

        // Add template manifest metadata
        render_context.add_variable("template_name", json!(template.manifest.name));
        render_context.add_variable("template_version", json!(template.manifest.version));
        if let Some(description) = &template.manifest.description {
            render_context.add_variable("template_description", json!(description));
        }

        Ok(render_context)
    }
}

fn build_python_endpoint_context(op: &Operation) -> Result<JsonValue, GenerationError> {
    let method_name = to_snake_case(&op.id);

    Ok(json!({
        "method_name": method_name,
        "class_name": to_proper_case(&format!("{}_handler", op.id)),
        "path": op.path,
        "http_method": op.method.to_lowercase(),
        "summary": op.summary.as_ref().map(|s| sanitize_markdown(s)).unwrap_or_default(),
        "description": op.description.as_ref().map(|s| sanitize_markdown(s)).unwrap_or_default(),
        "parameters": build_python_parameters(op),
        "response_type": map_response_to_python_type(op),
        "tags": op.tags.clone().unwrap_or_default(),
    }))
}

fn build_python_parameters(op: &Operation) -> Vec<JsonValue> {
    op.parameters
        .iter()
        .map(|p| {
            let python_type = map_schema_to_python_type(&p.schema);
            json!({
                "name": to_snake_case(&p.name),
                "python_name": to_snake_case(&p.name),
                "type": python_type.clone(),
                "type_hint": python_type,
                "in": format!("{:?}", p.location).to_lowercase(),
                "required": p.required,
                "description": p.description.as_ref().map(|d| sanitize_markdown(d)),
                "example": serde_json::Value::Null
            })
        })
        .collect()
}

fn map_schema_to_python_type(schema: &crate::generation::Schema) -> String {
    if let Some(typ) = &schema.schema_type {
        match typ.as_str() {
            "string" => "str".to_string(),
            "integer" => "int".to_string(),
            "boolean" => "bool".to_string(),
            "number" => "float".to_string(),
            "array" => {
                if let Some(items) = &schema.items {
                    format!("List[{}]", map_schema_to_python_type(items))
                } else {
                    "List[Any]".to_string()
                }
            }
            "object" => "Dict[str, Any]".to_string(),
            _ => "Any".to_string(),
        }
    } else {
        "Any".to_string()
    }
}

fn map_response_to_python_type(op: &Operation) -> String {
    for response in &op.responses {
        if response.status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema) = json_content.get("schema") {
                        return map_json_to_python_type(schema);
                    }
                }
            }
        }
    }
    "Dict[str, Any]".to_string()
}

fn map_json_to_python_type(schema: &JsonValue) -> String {
    if let Some(typ) = schema.get("type").and_then(|v| v.as_str()) {
        match typ {
            "string" => "str".to_string(),
            "integer" => "int".to_string(),
            "boolean" => "bool".to_string(),
            "number" => "float".to_string(),
            "array" => {
                if let Some(items) = schema.get("items") {
                    format!("List[{}]", map_json_to_python_type(items))
                } else {
                    "List[Any]".to_string()
                }
            }
            "object" => "Dict[str, Any]".to_string(),
            _ => "Any".to_string(),
        }
    } else {
        "Any".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{Template, TemplateManifest, TemplateSource};
    use crate::protocols::{Protocol, Role};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_python_context_builder() {
        let builder = PythonContextBuilder::new();

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Client, Language::Python);
        context.metadata.project_name = "TestClient".to_string();

        let manifest = TemplateManifest {
            name: "test-template".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            path: "mcp/client/python".to_string(),
            protocol: Protocol::Mcp,
            role: Role::Client,
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
        assert!(result.is_ok());

        // Test passes if build succeeds
    }

    #[tokio::test]
    async fn test_template_manifest_fields_in_context() {
        let builder = PythonContextBuilder::new();

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Client, Language::Python);
        context.metadata.project_name = "test_project".to_string();

        let manifest = TemplateManifest {
            name: "python-test-template".to_string(),
            version: "3.0.0".to_string(),
            description: Some("Python test template description".to_string()),
            path: "mcp/client/python".to_string(),
            protocol: Protocol::Mcp,
            role: Role::Client,
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
        assert!(result.is_ok());

        // Test passes if build succeeds with manifest fields
    }
}
