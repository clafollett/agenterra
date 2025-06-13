//! agenterra CLI entrypoint
//! Parses command-line arguments and dispatches to the core generator.

// Internal imports (std, crate)
use reqwest::Url;
use std::path::{Path, PathBuf};

// External imports (alphabetized)
use agenterra_core::{TemplateKind, TemplateManager, TemplateOptions};
use anyhow::Context;
use clap::Parser;
use tokio::fs;

#[derive(Parser)]
#[command(name = "agenterra")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    // TODO: Add future subcommands here (e.g., Validate, ListTemplates, etc.)
    /// Scaffold a new MCP server from an OpenAPI spec
    Scaffold {
        /// Project name
        #[arg(long, default_value = "agenterra_mcp_server")]
        project_name: String,
        /// Path or URL to OpenAPI schema (YAML or JSON)
        ///
        /// Can be a local file path or an HTTP/HTTPS URL
        /// Example: --schema-path path/to/schema.yaml
        /// Example: --schema-path <https://petstore3.swagger.io/api/v3/openapi.json>
        #[arg(long)]
        schema_path: String,
        /// Template to use for code generation (e.g., rust_axum, python_fastapi)
        #[arg(long, default_value = "rust_axum")]
        template_kind: String,
        /// Custom template directory (only used with --template-kind=custom)
        #[arg(long)]
        template_dir: Option<PathBuf>,
        /// Output directory for generated code
        #[arg(long)]
        output_dir: Option<PathBuf>,
        /// Log file name without extension (default: mcp-server)
        #[arg(long)]
        log_file: Option<String>,
        /// Server port (default: 3000)
        #[arg(long)]
        port: Option<u16>,
        /// Base URL of the OpenAPI specification (Optional)
        #[arg(long)]
        base_url: Option<Url>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    match &cli.command {
        Commands::Scaffold {
            project_name,
            schema_path,
            template_kind,
            template_dir,
            output_dir,
            log_file,
            port,
            base_url,
        } => {
            // Parse template
            let template_kind_enum: TemplateKind = template_kind
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid template '{template_kind}': {e}"))?;

            // Resolve output directory - use project_name if not specified
            let output_path = output_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from(project_name));

            // Debug log template and paths
            println!(
                "Scaffolding with template: {}, template_dir: {:?}, output_dir: {:?}",
                template_kind_enum.as_str(),
                template_dir,
                output_path
            );

            // Initialize the template manager using the resolved template directory
            let template_manager = TemplateManager::new(template_kind_enum, template_dir.clone())
                .await
                .context("Failed to initialize template manager")?;

            // Create output directory if it doesn't exist
            if !output_path.exists() {
                println!("Creating output directory: {}", output_path.display());
                fs::create_dir_all(&output_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create output directory: {}", e))?;
            }

            // List available templates for debugging
            println!("Available templates:");
            for template in template_manager.list_templates() {
                println!("Source: {} -> Destination: {}", template.0, template.1);
            }

            println!(
                "Using templates from: {}",
                template_manager.template_dir().display()
            );

            // Create directories for all template file destinations
            for file in &template_manager.manifest().files {
                if let Some(parent) = Path::new(&file.destination).parent() {
                    let dir = output_path.join(parent);
                    if !dir.exists() {
                        println!("Creating directory: {}", dir.display());
                        fs::create_dir_all(&dir).await.map_err(|e| {
                            anyhow::anyhow!("Failed to create directory {}: {}", dir.display(), e)
                        })?;
                    }
                }
            }

            // Load the OpenAPI schema from either a file or URL
            println!("Loading OpenAPI schema from: {}", schema_path);

            // Check if the schema_path is a URL or a file path
            let schema_obj = if schema_path.starts_with("http://")
                || schema_path.starts_with("https://")
            {
                // It's a URL, use from_url
                let response = reqwest::get(schema_path.as_str()).await.map_err(|e| {
                    anyhow::anyhow!("Failed to fetch OpenAPI schema from {}: {}", schema_path, e)
                })?;

                if !response.status().is_success() {
                    return Err(anyhow::anyhow!(
                        "Failed to fetch OpenAPI schema from {}: HTTP {}",
                        schema_path,
                        response.status()
                    ));
                }

                let content = response.text().await.map_err(|e| {
                    anyhow::anyhow!("Failed to read response from {}: {}", schema_path, e)
                })?;

                // Parse the content as OpenAPI schema
                // We need to save it to a temporary file since OpenApiContext::from_file expects a file path
                let temp_dir = tempfile::tempdir()?;
                let temp_file = temp_dir.path().join("openapi_schema.json");
                tokio::fs::write(&temp_file, &content).await?;

                agenterra_core::openapi::OpenApiContext::from_file(&temp_file)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to parse OpenAPI schema from {}: {}",
                            schema_path,
                            e
                        )
                    })?
            } else {
                // It's a file path
                agenterra_core::openapi::OpenApiContext::from_file(&schema_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to load OpenAPI schema: {}", e))?
            };

            // Create config with template
            let config = agenterra_core::Config {
                project_name: project_name.clone(),
                openapi_schema_path: schema_path.to_string(),
                output_dir: output_path.to_string_lossy().to_string(),
                template_kind: template_kind.to_string(),
                template_dir: template_dir
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string()),
                include_all: true,              // Include all operations by default
                include_operations: Vec::new(), // No specific operations to include
                exclude_operations: Vec::new(), // No operations to exclude
                base_url: base_url.clone(),
            };

            // Create template options
            let template_opts = TemplateOptions {
                server_port: *port,
                log_file: log_file.clone(),
                ..Default::default()
            };

            // Generate the server using the template manager we already created
            template_manager
                .generate(&schema_obj, &config, Some(template_opts))
                .await?;

            println!(
                "✅ Successfully generated server in: {}",
                output_path.display()
            );
        }
    }
    Ok(())
}
