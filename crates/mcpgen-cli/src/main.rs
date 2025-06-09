//! mcpgen CLI entrypoint
//! Parses command-line arguments and dispatches to the core generator.

// Internal imports (std, crate)
use reqwest::Url;
use std::path::{Path, PathBuf};

// External imports (alphabetized)
use anyhow::Context;
use clap::Parser;
use mcpgen_core::{
    TemplateOptions, template_kind::TemplateKind, template_manager::TemplateManager,
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
        /// Project name
        #[arg(long, default_value = "mcpgen_mcp_server")]
        project_name: String,
        /// Path or URL to OpenAPI spec (YAML or JSON)
        ///
        /// Can be a local file path or an HTTP/HTTPS URL
        /// Example: --spec path/to/spec.yaml
        /// Example: --spec https://example.com/openapi.json
        #[arg(long)]
        spec: String,
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
            spec,
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
            let output_path = output_dir.clone().unwrap_or_else(|| {
                PathBuf::from(project_name)
            });

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

            // Load the OpenAPI spec from either a file or URL
            println!("Loading OpenAPI spec from: {}", spec);

            // Check if the spec is a URL or a file path
            let spec_obj = if spec.starts_with("http://") || spec.starts_with("https://") {
                // It's a URL, use from_url
                let response = reqwest::get(spec.as_str()).await.map_err(|e| {
                    anyhow::anyhow!("Failed to fetch OpenAPI spec from {}: {}", spec, e)
                })?;

                if !response.status().is_success() {
                    return Err(anyhow::anyhow!(
                        "Failed to fetch OpenAPI spec from {}: HTTP {}",
                        spec,
                        response.status()
                    ));
                }

                let content = response
                    .text()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read response from {}: {}", spec, e))?;

                // Parse the content as OpenAPI spec
                // We need to save it to a temporary file since OpenApiContext::from_file expects a file path
                let temp_dir = tempfile::tempdir()?;
                let temp_file = temp_dir.path().join("openapi_spec.json");
                tokio::fs::write(&temp_file, &content).await?;

                mcpgen_core::openapi::OpenApiContext::from_file(&temp_file)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to parse OpenAPI spec from {}: {}", spec, e)
                    })?
            } else {
                // It's a file path
                mcpgen_core::openapi::OpenApiContext::from_file(&spec)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to load OpenAPI spec: {}", e))?
            };

            // Create config with template
            let config = mcpgen_core::Config {
                project_name: project_name.clone(),
                openapi_spec: spec.to_string(),
                output_dir: output_path.to_string_lossy().to_string(),
                template_kind: template_kind.to_string(),
                template_dir: template_dir.as_ref().map(|p| p.to_string_lossy().to_string()),
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
                .generate(&spec_obj, &config, Some(template_opts))
                .await?;

            println!("✅ Successfully generated server in: {}", output_path.display());
        }
    }
    Ok(())
}

/// Resolves the template directory path based on the provided template_dir and template_kind.
///
/// # Arguments
/// * `template_dir` - Optional user-provided template directory (takes ownership)
/// * `template_kind` - The type of template being used (built-in or custom)
///
/// # Returns
/// Returns the resolved PathBuf to the template directory or an error if validation fails
async fn resolve_template_dir(
    template_dir: &Option<PathBuf>,
    template_kind: &TemplateKind,
) -> anyhow::Result<PathBuf> {
    // If a template directory was provided, validate it exists
    if let Some(template_dir) = template_dir {
        println!(
            "Using custom template directory: {}",
            template_dir.display()
        );
        if !template_dir.exists() {
            return Err(anyhow::anyhow!(
                "Template directory not found: {}",
                template_dir.display()
            ));
        }
        return Ok(template_dir.clone());
    }

    // For custom templates without a specified directory, use default "./templates"
    if *template_kind == TemplateKind::Custom {
        return Ok(PathBuf::from("./templates"));
    }

    // For built-in templates, use workspace templates/<template>
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(manifest_dir)
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to determine workspace root from CARGO_MANIFEST_DIR")
        })?;

    let templates_dir = workspace_root.join("templates");
    let built_in_dir = templates_dir.join(template_kind.as_str());

    println!(
        "DEBUG - Full template directory: {}",
        built_in_dir.display()
    );

    Ok(built_in_dir)
}
