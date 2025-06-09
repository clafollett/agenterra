//! Code generation functionality for MCPGen

use std::{path::PathBuf, str::FromStr};

use crate::{
    config::Config,
    error::Result,
    openapi::OpenApiContext,
    templates::{TemplateKind, TemplateManager, TemplateOptions},
};

/// Main entry point for code generation
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
