//! Code generation functionality for Agenterra

use std::{path::PathBuf, str::FromStr};

use crate::{
    config::Config,
    error::Result,
    openapi::OpenApiContext,
    templates::{TemplateKind, TemplateManager, TemplateOptions},
};

/// Generates MCP server code from an OpenAPI specification.
///
/// This is the main entry point for Agenterra's code generation functionality.
/// It loads the OpenAPI schema, initializes the appropriate template system,
/// and generates the complete MCP server code structure.
///
/// # Arguments
/// * `config` - Configuration containing schema path, output directory, and template settings
/// * `template_opts` - Optional template-specific options for customizing generation
///
/// # Returns
/// `Result<()>` indicating success or failure of the generation process
///
/// # Errors
/// This function will return an error if:
/// - The OpenAPI schema file cannot be loaded or parsed
/// - The specified template kind is invalid or unavailable
/// - The output directory cannot be created or written to
/// - Template rendering fails due to invalid schema or template errors
///
/// # Examples
/// ```no_run
/// use agenterra_core::{Config, generate::generate};
///
/// # async fn example() -> agenterra_core::Result<()> {
/// let config = Config {
///     project_name: "my_server".to_string(),
///     openapi_schema_path: "./petstore.json".to_string(),
///     output_dir: "./generated".to_string(),
///     template_kind: "rust_axum".to_string(),
///     template_dir: None,
///     include_all: true,
///     include_operations: Vec::new(),
///     exclude_operations: Vec::new(),
///     base_url: None,
/// };
///
/// generate(&config, None).await?;
/// # Ok(())
/// # }
/// ```
pub async fn generate(config: &Config, template_opts: Option<TemplateOptions>) -> Result<()> {
    // 1. Load OpenAPI schema
    let schema = OpenApiContext::from_file(&config.openapi_schema_path).await?;

    // 2. Initialize template manager with template_dir from config if available
    let template_kind = TemplateKind::from_str(&config.template_kind).unwrap_or_default();
    let template_dir = config.template_dir.as_ref().map(PathBuf::from);
    let template_manager = TemplateManager::new(template_kind, template_dir).await?;

    // 3. Delegate to TemplateManager.generate
    template_manager
        .generate(&schema, config, template_opts)
        .await?;

    Ok(())
}
