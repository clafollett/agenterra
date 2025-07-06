//! Integration layer for connecting CLI commands to DDD architecture
//!
//! This module provides adapters that bridge the existing CLI interface
//! with the new domain-driven architecture.

use crate::application::{
    GenerateClientRequest, GenerateServerRequest, generate_client::GenerateClientUseCase,
    generate_server::GenerateServerUseCase,
};
use crate::generation::Language;
use crate::protocols::Protocol;
use serde_json;
use std::path::PathBuf;

/// Server generation parameters from CLI
pub struct ServerParams {
    pub project_name: String,
    pub schema_path: String,
    pub template: String,
    pub template_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub port: Option<u16>,
    pub log_file: Option<String>,
    pub base_url: Option<reqwest::Url>,
}

/// Client generation parameters from CLI
pub struct ClientParams {
    pub project_name: String,
    pub template: String,
    pub template_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
}

/// Integration service for MCP server generation
pub struct McpServerIntegration;

impl McpServerIntegration {
    pub async fn generate(params: ServerParams) -> anyhow::Result<()> {
        // Parse language
        let language = params
            .template
            .parse::<Language>()
            .map_err(|_| anyhow::anyhow!("Invalid language: {}", params.template))?;

        // Resolve output directory
        let output_dir = if let Some(dir) = params.output_dir {
            dir.join(&params.project_name)
        } else {
            std::env::current_dir()?.join(&params.project_name)
        };

        // Create request with options
        let mut options = std::collections::HashMap::new();

        // Add base_url as base_api_url for template compatibility
        if let Some(ref base_url) = params.base_url {
            options.insert(
                "base_api_url".to_string(),
                serde_json::json!(base_url.to_string()),
            );
        }

        // Add port if provided
        if let Some(port) = params.port {
            options.insert("server_port".to_string(), serde_json::json!(port));
        }

        // Add log file if provided
        if let Some(ref log_file) = params.log_file {
            options.insert("log_file".to_string(), serde_json::json!(log_file));
        }

        let request = GenerateServerRequest {
            protocol: Protocol::Mcp,
            language,
            project_name: params.project_name,
            schema_path: Some(params.schema_path),
            output_dir,
            options,
        };

        // Create dependencies
        let protocol_registry = std::sync::Arc::new(
            crate::protocols::ProtocolRegistry::with_defaults()
                .map_err(|e| anyhow::anyhow!("Failed to create protocol registry: {}", e))?,
        );

        let openapi_loader =
            std::sync::Arc::new(crate::infrastructure::openapi::CompositeOpenApiLoader::new())
                as std::sync::Arc<dyn crate::generation::OpenApiLoader>;

        // Create template discovery based on whether template_dir is provided
        let template_discovery: std::sync::Arc<dyn crate::generation::TemplateDiscovery> =
            if let Some(ref template_dir) = params.template_dir {
                // Use the new TemplateLoader approach for filesystem templates
                let loader = std::sync::Arc::new(
                    crate::infrastructure::templates::FileSystemTemplateLoader::new()
                );
                let infrastructure_discovery = std::sync::Arc::new(
                    crate::infrastructure::templates::TemplateLoaderDiscoveryAdapter::new(
                        loader,
                        template_dir.clone(),
                    )
                );
                // Wrap with the generation adapter
                std::sync::Arc::new(crate::generation::TemplateDiscoveryAdapter::new(
                    infrastructure_discovery,
                ))
            } else {
                // Use embedded templates with adapter
                std::sync::Arc::new(crate::generation::TemplateDiscoveryAdapter::new(
                    std::sync::Arc::new(
                        crate::infrastructure::templates::EmbeddedTemplateRepository::new(),
                    ),
                ))
            };

        // Select appropriate renderer based on protocol and role
        let template_renderer = crate::infrastructure::generation::select_renderer(
            Protocol::Mcp,
            crate::protocols::Role::Server,
        );

        let generation_orchestrator = std::sync::Arc::new(
            crate::generation::GenerationOrchestrator::new(
                template_discovery,
                std::sync::Arc::new(crate::infrastructure::generation::context_builders::registry::CompositeContextBuilder::default()),
                template_renderer,
                std::sync::Arc::new(crate::infrastructure::generation::CompositePostProcessor::new()),
            )
        );

        let output_service =
            std::sync::Arc::new(crate::infrastructure::output::FileSystemOutputService::new())
                as std::sync::Arc<dyn crate::application::OutputService>;

        // Execute use case
        let use_case = GenerateServerUseCase::new(
            protocol_registry,
            openapi_loader,
            generation_orchestrator,
            output_service,
        );

        use_case
            .execute(request)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate server: {}", e))?;

        Ok(())
    }
}

/// Integration service for MCP client generation
pub struct McpClientIntegration;

impl McpClientIntegration {
    pub async fn generate(params: ClientParams) -> anyhow::Result<()> {
        // Parse language
        let language = params
            .template
            .parse::<Language>()
            .map_err(|_| anyhow::anyhow!("Invalid language: {}", params.template))?;

        // Resolve output directory
        let output_dir = if let Some(dir) = params.output_dir {
            dir.join(&params.project_name)
        } else {
            std::env::current_dir()?.join(&params.project_name)
        };

        // Create request
        let request = GenerateClientRequest {
            protocol: Protocol::Mcp,
            language,
            project_name: params.project_name,
            output_dir,
            options: std::collections::HashMap::new(),
        };

        // Create dependencies
        let protocol_registry = std::sync::Arc::new(
            crate::protocols::ProtocolRegistry::with_defaults()
                .map_err(|e| anyhow::anyhow!("Failed to create protocol registry: {}", e))?,
        );

        // Create template discovery based on whether template_dir is provided
        let template_discovery: std::sync::Arc<dyn crate::generation::TemplateDiscovery> =
            if let Some(ref template_dir) = params.template_dir {
                // Use the new TemplateLoader approach for filesystem templates
                let loader = std::sync::Arc::new(
                    crate::infrastructure::templates::FileSystemTemplateLoader::new()
                );
                let infrastructure_discovery = std::sync::Arc::new(
                    crate::infrastructure::templates::TemplateLoaderDiscoveryAdapter::new(
                        loader,
                        template_dir.clone(),
                    )
                );
                // Wrap with the generation adapter
                std::sync::Arc::new(crate::generation::TemplateDiscoveryAdapter::new(
                    infrastructure_discovery,
                ))
            } else {
                // Use embedded templates with adapter
                std::sync::Arc::new(crate::generation::TemplateDiscoveryAdapter::new(
                    std::sync::Arc::new(
                        crate::infrastructure::templates::EmbeddedTemplateRepository::new(),
                    ),
                ))
            };

        // Select appropriate renderer for client (uses default renderer)
        let template_renderer = crate::infrastructure::generation::select_renderer(
            Protocol::Mcp,
            crate::protocols::Role::Client,
        );

        let generation_orchestrator = std::sync::Arc::new(
            crate::generation::GenerationOrchestrator::new(
                template_discovery,
                std::sync::Arc::new(crate::infrastructure::generation::context_builders::registry::CompositeContextBuilder::default()),
                template_renderer,
                std::sync::Arc::new(crate::infrastructure::generation::CompositePostProcessor::new()),
            )
        );

        let output_service =
            std::sync::Arc::new(crate::infrastructure::output::FileSystemOutputService::new())
                as std::sync::Arc<dyn crate::application::OutputService>;

        // Execute use case
        let use_case =
            GenerateClientUseCase::new(protocol_registry, generation_orchestrator, output_service);

        use_case
            .execute(request)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate client: {}", e))?;

        Ok(())
    }
}

// CustomDirTemplateDiscovery has been removed in favor of TemplateLoaderDiscoveryAdapter
