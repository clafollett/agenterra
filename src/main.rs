//! agenterra CLI entrypoint
//! Parses command-line arguments and dispatches to the core generator.
#![deny(unsafe_code)]
mod core;
mod mcp;

// Internal imports (std, crate)
use core::{
    openapi::OpenApiContext,
    protocol::Protocol,
    templates::{
        ClientTemplateKind, EmbeddedTemplateExporter, EmbeddedTemplates, ServerTemplateKind,
        TemplateExporter, TemplateManager, TemplateOptions, TemplateRepository,
        dir::resolve_output_dir,
    },
};
use std::path::{Path, PathBuf};

// External imports (alphabetized)
use anyhow::Context;
use clap::Parser;
use reqwest::Url;
use tracing::{Level, error, info};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "agenterra")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Scaffold servers and clients for various targets
    Scaffold {
        #[command(subcommand)]
        target: TargetCommands,
    },
    /// Manage embedded templates
    Templates {
        #[command(subcommand)]
        action: TemplateCommands,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum TargetCommands {
    /// Model Context Protocol (MCP) servers and clients
    Mcp {
        #[command(subcommand)]
        role: McpCommands,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum McpCommands {
    /// Generate MCP server from OpenAPI specification that exposes API endpoints as MCP tools
    Server {
        /// Project name for the generated MCP server
        #[arg(long, default_value = "agenterra_mcp_server")]
        project_name: String,
        /// Path or URL to OpenAPI schema (YAML or JSON)
        #[arg(long)]
        schema_path: String,
        /// Template to use for code generation
        #[arg(long, default_value = "rust")]
        template: String,
        /// Custom template directory
        #[arg(long)]
        template_dir: Option<PathBuf>,
        /// Output directory for generated code
        #[arg(long)]
        output_dir: Option<PathBuf>,
        /// Log file name without extension
        #[arg(long)]
        log_file: Option<String>,
        /// Server port
        #[arg(long)]
        port: Option<u16>,
        /// Base URL of the OpenAPI specification
        #[arg(long)]
        base_url: Option<Url>,
    },
    /// Generate MCP client that can connect to MCP servers (no OpenAPI spec required)
    Client {
        /// Project name for the generated MCP client
        #[arg(long, default_value = "agenterra_mcp_client")]
        project_name: String,
        /// Template to use for client generation
        #[arg(long, default_value = "rust")]
        template: String,
        /// Custom template directory
        #[arg(long)]
        template_dir: Option<PathBuf>,
        /// Output directory for generated code
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum TemplateCommands {
    /// List all available embedded templates
    List,
    /// Export templates to a directory
    Export {
        /// Directory to export templates to
        path: PathBuf,
        /// Optional: Export only a specific template (e.g., "mcp/server/rust")
        #[arg(long)]
        template: Option<String>,
    },
    /// Show information about a specific template
    Info {
        /// Template path (e.g., "mcp/server/rust")
        template: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging with default level INFO
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    info!("Starting Agenterra CLI");
    let cli = Cli::parse();
    match &cli.command {
        Commands::Scaffold { target } => match target {
            TargetCommands::Mcp { role } => match role {
                McpCommands::Server {
                    project_name,
                    schema_path,
                    template,
                    template_dir,
                    output_dir,
                    log_file,
                    port,
                    base_url,
                } => {
                    generate_mcp_server(ServerGenParams {
                        project_name,
                        schema_path,
                        template,
                        template_dir,
                        output_dir,
                        log_file,
                        port,
                        base_url,
                    })
                    .await?
                }
                McpCommands::Client {
                    project_name,
                    template,
                    template_dir,
                    output_dir,
                } => generate_mcp_client(project_name, template, template_dir, output_dir).await?,
            },
        },
        Commands::Templates { action } => match action {
            TemplateCommands::List => {
                list_templates().await?;
            }
            TemplateCommands::Export { path, template } => {
                export_templates(path.as_path(), template).await?;
            }
            TemplateCommands::Info { template } => {
                show_template_info(template).await?;
            }
        },
    }
    Ok(())
}

/// Parameters for MCP server generation
struct ServerGenParams<'a> {
    project_name: &'a str,
    schema_path: &'a str,
    template: &'a str,
    template_dir: &'a Option<PathBuf>,
    output_dir: &'a Option<PathBuf>,
    log_file: &'a Option<String>,
    port: &'a Option<u16>,
    base_url: &'a Option<Url>,
}

/// Generate MCP server from OpenAPI specification
async fn generate_mcp_server(params: ServerGenParams<'_>) -> anyhow::Result<()> {
    info!(
        template = %params.template,
        "Generating MCP server"
    );

    // Parse template
    let template_kind_enum: ServerTemplateKind = params
        .template
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid server template '{}': {}", params.template, e))?;

    // Resolve output directory with workspace-aware defaults
    let output_path = resolve_output_dir(params.project_name, params.output_dir.as_deref())
        .context("Failed to resolve output directory")?;

    // Initialize the template manager with MCP protocol
    let template_manager = TemplateManager::new_server(
        Protocol::Mcp,
        template_kind_enum,
        params.template_dir.clone(),
    )
    .await
    .context("Failed to initialize server template manager")?;

    // Load and validate OpenAPI schema BEFORE creating directories
    let schema_obj = OpenApiContext::from_file_or_url(params.schema_path)
        .await
        .context("Failed to load OpenAPI schema")?;

    // Create output directory only after all validations pass
    if !output_path.exists() {
        info!(path = %output_path.display(), "Creating output directory");
        tokio::fs::create_dir_all(&output_path).await.map_err(|e| {
            error!(path = %output_path.display(), error = %e, "Failed to create output directory");
            anyhow::anyhow!("Failed to create output directory: {}", e)
        })?
    }

    // Create config
    let config = crate::core::config::Config {
        project_name: params.project_name.to_string(),
        openapi_schema_path: params.schema_path.to_string(),
        output_dir: output_path.to_string_lossy().to_string(),
        template_kind: params.template.to_string(),
        template_dir: params
            .template_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        include_all: true,
        include_operations: Vec::new(),
        exclude_operations: Vec::new(),
        base_url: params.base_url.clone(),
    };

    // Create template options
    let template_opts = TemplateOptions {
        server_port: *params.port,
        log_file: params.log_file.clone(),
        ..Default::default()
    };

    // Generate the server code
    info!("Generating MCP server code...");
    template_manager
        .generate(&schema_obj, &config, Some(template_opts))
        .await
        .map_err(|e| {
            error!("Failed to generate server code: {}", e);
            anyhow::anyhow!("Failed to generate server code: {}", e)
        })?;

    info!(
        output_path = %output_path.display(),
        "Successfully generated MCP server"
    );
    Ok(())
}

/// Generate MCP client
async fn generate_mcp_client(
    project_name: &str,
    template: &str,
    template_dir: &Option<PathBuf>,
    output_dir: &Option<PathBuf>,
) -> anyhow::Result<()> {
    info!(
        template = %template,
        "Generating MCP client"
    );

    // Parse and validate template
    let template_kind_enum: ClientTemplateKind = template
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid client template '{}': {}", template, e))?;

    // Resolve output directory with workspace-aware defaults
    let output_path = resolve_output_dir(project_name, output_dir.as_deref())
        .context("Failed to resolve output directory")?;

    // Initialize template manager for the chosen client template with MCP protocol
    let template_manager =
        TemplateManager::new_client(Protocol::Mcp, template_kind_enum, template_dir.clone())
            .await?;

    // Build a core config (no OpenAPI schema needed for clients)
    let core_config = crate::core::config::Config {
        project_name: project_name.to_string(),
        openapi_schema_path: String::new(),
        output_dir: output_path.to_string_lossy().to_string(),
        template_kind: template_kind_enum.as_str().to_string(),
        template_dir: template_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        include_all: true,
        include_operations: Vec::new(),
        exclude_operations: Vec::new(),
        base_url: None,
    };

    // Generate the client directly via TemplateManager
    info!("Generating MCP client code...");
    template_manager.generate_client(&core_config, None).await?;

    info!(
        output_path = %output_path.display(),
        "Successfully generated MCP client"
    );
    Ok(())
}

/// List all available embedded templates
async fn list_templates() -> anyhow::Result<()> {
    let repo = EmbeddedTemplates::new();
    let templates = repo.list_templates();

    if templates.is_empty() {
        info!("No embedded templates found.");
        return Ok(());
    }

    // For user-facing output in a CLI tool, we'll use println! for the actual listing
    // but use tracing for operational messages
    println!("Available templates:\n");

    // Group by template type
    let mut server_templates = Vec::new();
    let mut client_templates = Vec::new();

    for template in templates {
        match template.template_type {
            core::templates::TemplateType::Server => server_templates.push(template),
            core::templates::TemplateType::Client => client_templates.push(template),
        }
    }

    // Print server templates
    if !server_templates.is_empty() {
        println!("Server Templates:");
        for template in server_templates {
            println!(
                "  {} - {}",
                template.path,
                template.description.as_deref().unwrap_or("No description")
            );
        }
        println!();
    }

    // Print client templates
    if !client_templates.is_empty() {
        println!("Client Templates:");
        for template in client_templates {
            println!(
                "  {} - {}",
                template.path,
                template.description.as_deref().unwrap_or("No description")
            );
        }
    }

    Ok(())
}

/// Export templates to a directory (all or a specific one)
async fn export_templates(path: &Path, template: &Option<String>) -> anyhow::Result<()> {
    info!("Exporting templates to: {}", path.display());

    let exporter = EmbeddedTemplateExporter::new();
    let repository = EmbeddedTemplates::new();

    match template {
        Some(template_path) => {
            // Export single template
            info!("Exporting single template: {}", template_path);

            // Get the template metadata
            let template_meta = repository
                .get_template(template_path)
                .with_context(|| format!("Template not found: {template_path}"))?;

            // Export the template
            exporter
                .export_template(&template_meta, path)
                .context("Failed to export template")?;

            info!(
                "Successfully exported template {} to {}",
                template_path,
                path.display()
            );
            // User-facing output
            println!("Exported template {} to {}", template_path, path.display());
        }
        None => {
            // Export all templates
            let count = exporter
                .export_all_templates(path)
                .context("Failed to export templates")?;

            info!(
                "Successfully exported {} templates to {}",
                count,
                path.display()
            );
            // User-facing output
            println!("Exported {} templates to {}", count, path.display());
        }
    }

    Ok(())
}

/// Show detailed information about a specific template
async fn show_template_info(template_path: &str) -> anyhow::Result<()> {
    let repository = EmbeddedTemplates::new();

    match repository.get_template(template_path) {
        Some(template) => {
            println!("Template: {}", template.path);
            println!("Type: {:?}", template.template_type);
            println!("Kind: {}", template.kind);
            println!("Protocol: {}", template.protocol);
            println!(
                "Description: {}",
                template.description.as_deref().unwrap_or("No description")
            );

            // Show file count
            let files = repository.get_template_files(template_path);
            println!("Files: {} files", files.len());

            // List some key files
            println!("\nKey files:");
            for file in &files {
                if file.relative_path == "manifest.yml"
                    || file.relative_path == "Cargo.toml.tera"
                    || file.relative_path == "README.md.tera"
                    || file.relative_path.ends_with("main.rs.tera")
                {
                    println!("  - {}", file.relative_path);
                }
            }
        }
        None => {
            eprintln!("Template not found: {template_path}");
            eprintln!("\nRun 'agenterra templates list' to see available templates.");
            std::process::exit(1);
        }
    }

    Ok(())
}
