//! TypeScript-specific context builder for code generation

use async_trait::async_trait;
use serde_json::{Value as JsonValue, json};

use crate::generation::{
    ContextBuilder, GenerationContext, GenerationError, Language, RenderContext,
    utils::{to_camel_case, to_proper_case, to_snake_case},
};
use crate::infrastructure::templates::Template;

/// TypeScript-specific context builder
pub struct TypeScriptContextBuilder;

impl TypeScriptContextBuilder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ContextBuilder for TypeScriptContextBuilder {
    async fn build(
        &self,
        context: &GenerationContext,
        template: &Template,
    ) -> Result<RenderContext, GenerationError> {
        if context.language != Language::TypeScript {
            return Err(GenerationError::InvalidConfiguration(format!(
                "TypeScriptContextBuilder can only build contexts for TypeScript, got {:?}",
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
        render_context.add_variable("language", json!("typescript"));

        // TypeScript-specific naming
        let package_name = to_snake_case(&context.metadata.project_name).replace('_', "-"); // npm packages use kebab-case
        let class_name = to_proper_case(&context.metadata.project_name);
        let variable_name = to_camel_case(&context.metadata.project_name);

        render_context.add_variable("package_name", json!(package_name));
        render_context.add_variable("class_name", json!(class_name));
        render_context.add_variable("variable_name", json!(variable_name));
        render_context.add_variable("cli_command", json!(package_name));

        // Process operations
        let mut endpoints = Vec::new();
        for operation in &context.operations {
            let endpoint_context = build_typescript_endpoint_context(operation)?;
            endpoints.push(endpoint_context);
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

fn build_typescript_endpoint_context(
    op: &crate::generation::Operation,
) -> Result<JsonValue, GenerationError> {
    let method_name = to_camel_case(&op.id);

    Ok(json!({
        "method_name": method_name,
        "interface_name": to_proper_case(&format!("{}_params", op.id)),
        "response_interface": to_proper_case(&format!("{}_response", op.id)),
        "path": op.path,
        "http_method": op.method.to_lowercase(),
        "summary": op.summary.clone().unwrap_or_default(),
        "description": op.description.clone().unwrap_or_default(),
        "parameters": build_typescript_parameters(op),
        "response_type": map_response_to_typescript_type(op),
        "tags": op.tags.clone().unwrap_or_default(),
    }))
}

fn build_typescript_parameters(op: &crate::generation::Operation) -> Vec<JsonValue> {
    op.parameters
        .iter()
        .map(|p| {
            json!({
                "name": to_camel_case(&p.name),
                "original_name": p.name.clone(),
                "type": "any", // Simplified for now
                "in": format!("{:?}", p.location).to_lowercase(),
                "required": p.required,
                "description": p.description,
                "example": serde_json::Value::Null
            })
        })
        .collect()
}

fn map_response_to_typescript_type(op: &crate::generation::Operation) -> String {
    for response in &op.responses {
        if response.status_code.starts_with('2') {
            if let Some(content) = response.content.as_ref() {
                if let Some(json_content) = content.get("application/json") {
                    if let Some(schema) = json_content.get("schema") {
                        return map_json_to_typescript_type(schema);
                    }
                }
            }
        }
    }
    "Record<string, any>".to_string()
}

fn map_json_to_typescript_type(schema: &JsonValue) -> String {
    if let Some(typ) = schema.get("type").and_then(|v| v.as_str()) {
        match typ {
            "string" => "string".to_string(),
            "integer" | "number" => "number".to_string(),
            "boolean" => "boolean".to_string(),
            "array" => {
                if let Some(items) = schema.get("items") {
                    format!("{}[]", map_json_to_typescript_type(items))
                } else {
                    "any[]".to_string()
                }
            }
            "object" => "Record<string, any>".to_string(),
            _ => "any".to_string(),
        }
    } else {
        "any".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::templates::{
        Template, TemplateDescriptor, TemplateManifest, TemplateSource,
    };
    use crate::protocols::{Protocol, Role};

    #[tokio::test]
    async fn test_typescript_context_builder() {
        let builder = TypeScriptContextBuilder::new();

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::TypeScript);
        context.metadata.project_name = "test_server".to_string();

        let template = Template {
            descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::TypeScript),
            manifest: TemplateManifest::default(),
            files: vec![],
            source: TemplateSource::Embedded,
        };

        let result = builder.build(&context, &template).await;
        assert!(result.is_ok());

        let render_context = result.unwrap();
        assert_eq!(
            render_context.get_variable("package_name").unwrap(),
            "test-server"
        );
        assert_eq!(
            render_context.get_variable("class_name").unwrap(),
            "TestServer"
        );
        assert_eq!(
            render_context.get_variable("variable_name").unwrap(),
            "testServer"
        );
    }

    #[tokio::test]
    async fn test_template_manifest_fields_in_context() {
        let builder = TypeScriptContextBuilder::new();

        let mut context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::TypeScript);
        context.metadata.project_name = "test_project".to_string();

        let mut manifest = TemplateManifest::default();
        manifest.name = "typescript-test-template".to_string();
        manifest.version = "4.0.0".to_string();
        manifest.description = Some("TypeScript test template description".to_string());

        let template = Template {
            descriptor: TemplateDescriptor::new(Protocol::Mcp, Role::Server, Language::TypeScript),
            manifest,
            files: vec![],
            source: TemplateSource::Embedded,
        };

        let result = builder.build(&context, &template).await;
        assert!(result.is_ok());

        let render_context = result.unwrap();
        assert_eq!(
            render_context.get_variable("template_name").unwrap(),
            "typescript-test-template"
        );
        assert_eq!(
            render_context.get_variable("template_version").unwrap(),
            "4.0.0"
        );
        assert_eq!(
            render_context.get_variable("template_description").unwrap(),
            "TypeScript test template description"
        );
    }
}
