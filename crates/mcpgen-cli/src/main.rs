//! mcpgen CLI entrypoint
//! Parses command-line arguments and dispatches to the core generator.

// Internal imports (std, crate)
use std::path::{Path, PathBuf};

// External imports (alphabetized)
use anyhow::Context;
use clap::Parser;
use mcpgen_core::{
    TemplateOptions, template::Template, template_manager::TemplateManager,
};
use reqwest;
use tempfile;
use tokio::fs;

#[derive(Parser)]
#[command(name = "mcpgen")]
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
        /// Path or URL to OpenAPI spec (YAML or JSON)
        ///
        /// Can be a local file path or an HTTP/HTTPS URL
        /// Example: --spec path/to/spec.yaml
        /// Example: --spec https://example.com/openapi.json
        #[arg(long)]
        spec: String,
        /// Output directory for generated code
        #[arg(long)]
        output: PathBuf,
        /// Template to use for code generation (e.g., rust-axum, python-fastapi)
        #[arg(short, long, default_value = "rust-axum")]
        template: String,
        /// Custom template directory (only used with --template=custom)
        #[arg(long)]
        template_dir: Option<PathBuf>,
        /// Comma-separated list of policy plugins
        #[arg(long)]
        policy_plugins: Option<String>,
        /// Server port (default: 3000)
        #[arg(long)]
        port: Option<u16>,
        /// Log file name without extension (default: mcp-server)
        #[arg(long)]
        log_file: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    match &cli.command {
        Commands::Scaffold {
            spec,
            output,
            template,
            policy_plugins: _,
            port,
            log_file,
            template_dir,
        } => {
            // Parse template
            let template_kind: Template = template
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid template '{}': {}", template, e))?;

            // Debug log template and paths
            tracing::debug!(
                "Scaffolding with template: {}, output: {}",
                template_kind.as_str(),
                output.display()
            );

            if let Some(template_dir) = template_dir.as_ref() {
                tracing::debug!(
                    "Using custom template directory: {}",
                    template_dir.display()
                );
                if !template_dir.exists() {
                    return Err(anyhow::anyhow!(
                        "Template directory not found: {}",
                        template_dir.display()
                    ));
                }
            }

            println!("Generating server with template: {}", template_kind);

            // Log the template being used for code generation
            println!(
                "Generating server from OpenAPI spec using template: {}",
                template_kind
            );

            // Determine the template directory, honoring --template-dir if provided
            let template_dir_path = if let Some(dir) = template_dir.clone() {
                dir
            } else if template_kind == Template::Custom {
                PathBuf::from("./templates")
            } else {
                // For built-in templates, use workspace templates/<template>
                let manifest_dir = env!("CARGO_MANIFEST_DIR");
                let workspace_root = Path::new(manifest_dir)
                    .parent()
                    .and_then(Path::parent)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Failed to determine workspace root from CARGO_MANIFEST_DIR"
                        )
                    })?;
                let templates_dir = workspace_root.join("templates");
                let built_in_dir = templates_dir.join(template_kind.as_str());
                println!(
                    "DEBUG - Full template directory: {}",
                    built_in_dir.display()
                );
                built_in_dir
            };

            println!("Using template directory: {}", template_dir_path.display());

            // For custom templates, ensure the directory exists
            if template_kind == Template::Custom && !template_dir_path.exists() {
                fs::create_dir_all(&template_dir_path)
                    .await
                    .context("Failed to create template directory")?;
                println!(
                    "Created template directory at: {}",
                    template_dir_path.display()
                );
            }

            // Initialize the template manager using the resolved template directory
            let template_manager =
                TemplateManager::new(template_kind, Some(template_dir_path.clone()))
                    .await
                    .context("Failed to initialize template manager")?;

            tracing::debug!("Creating output directory: {}", output.display());

            // Create output directory if it doesn't exist
            if let Some(parent) = output.parent() {
                if !parent.exists() {
                    tracing::debug!("Creating parent directory: {}", parent.display());
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to create parent directory: {}", e))?;
                }
            }

            // List available templates for debugging
            println!("Available templates:");
            for template in template_manager.list_templates() {
                println!("  - {}", template);
            }

            println!(
                "Using templates from: {}",
                template_manager.template_dir().display()
            );

            // Ensure output directory and all required subdirectories exist
            println!("Creating output directory: {}", output.display());
            fs::create_dir_all(&output)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create output directory: {}", e))?;

            // Create directories for all template file destinations
            for file in &template_manager.manifest().files {
                if let Some(parent) = Path::new(&file.destination).parent() {
                    let dir = output.join(parent);
                    if !dir.exists() {
                        println!("Creating directory: {}", dir.display());
                        fs::create_dir_all(&dir).await.map_err(|e| {
                            anyhow::anyhow!("Failed to create directory {}: {}", dir.display(), e)
                        })?;
                    }
                }
            }

            // Load the OpenAPI spec from either a file or URL
            tracing::debug!("Loading OpenAPI spec from: {}", spec);
            
            // Check if the spec is a URL or a file path
            let spec_obj = if spec.starts_with("http://") || spec.starts_with("https://") {
                // It's a URL, use from_url
                let response = reqwest::get(spec.as_str()).await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch OpenAPI spec from {}: {}", spec, e))?;
                
                if !response.status().is_success() {
                    return Err(anyhow::anyhow!(
                        "Failed to fetch OpenAPI spec from {}: HTTP {}",
                        spec,
                        response.status()
                    ));
                }
                
                let content = response.text().await
                    .map_err(|e| anyhow::anyhow!("Failed to read response from {}: {}", spec, e))?;
                
                // Parse the content as OpenAPI spec
                // We need to save it to a temporary file since OpenApiContext::from_file expects a file path
                let temp_dir = tempfile::tempdir()?;
                let temp_file = temp_dir.path().join("openapi_spec.json");
                tokio::fs::write(&temp_file, &content).await?;
                
                mcpgen_core::openapi::OpenApiContext::from_file(&temp_file)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to parse OpenAPI spec from {}: {}", spec, e))?
            } else {
                // It's a file path
                mcpgen_core::openapi::OpenApiContext::from_file(&spec)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to load OpenAPI spec: {}", e))?
            };

            // Create config with template
            let config = mcpgen_core::Config {
                openapi_spec: spec.to_string(),
                output_dir: output.to_string_lossy().to_string(),
                template: template.to_string(),
                include_all: true,              // Include all operations by default
                include_operations: Vec::new(), // No specific operations to include
                exclude_operations: Vec::new(), // No operations to exclude
            };

            // Create template options
            let template_opts = TemplateOptions {
                server_port: *port,
                log_file: log_file.clone(),
                ..Default::default()
            };

            // Generate the server using the template manager we already created
            template_manager.generate(&spec_obj, &config, Some(template_opts)).await?;
            
            println!("✅ Successfully generated server in: {}", output.display());
        }
    }
    Ok(())
}
