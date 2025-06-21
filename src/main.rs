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
        ClientTemplateKind, ServerTemplateKind, TemplateDir, TemplateManager, TemplateOptions,
    },
};
use std::path::PathBuf;

// External imports (alphabetized)
use anyhow::Context;
use clap::Parser;
use reqwest::Url;
use tracing::{Level, error, info, warn};
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
    /// Serve as various runtime components
    Serve {
        #[command(subcommand)]
        target: ServeCommands,
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
pub enum ServeCommands {
    /// Run as MCP client
    Client {
        #[command(subcommand)]
        role: McpClientCommands,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum McpClientCommands {
    /// Model Context Protocol client operations
    Mcp {
        #[command(subcommand)]
        action: McpClientActions,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum McpClientActions {
    /// Connect to an MCP server process
    Connect {
        /// Command to run MCP server
        #[arg(long)]
        command: String,
        /// Additional arguments for the server command
        #[arg(long)]
        args: Vec<String>,
    },
    /// List available tools from connected server
    #[command(name = "list-tools")]
    ListTools,
    /// Call a tool with JSON arguments
    Call {
        /// Name of the tool to call
        tool_name: String,
        /// JSON arguments for the tool
        #[arg(long)]
        args: Option<String>,
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
        #[arg(long, default_value = "rust_axum")]
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
        #[arg(long, default_value = "rust_reqwest")]
        template: String,
        /// Custom template directory
        #[arg(long)]
        template_dir: Option<PathBuf>,
        /// Output directory for generated code
        #[arg(long)]
        output_dir: Option<PathBuf>,
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
        Commands::Serve { target } => match target {
            ServeCommands::Client { role } => match role {
                McpClientCommands::Mcp { action } => match action {
                    McpClientActions::Connect { command, args } => {
                        run_mcp_client_connect(command, args).await?
                    }
                    McpClientActions::ListTools => run_mcp_client_list_tools().await?,
                    McpClientActions::Call { tool_name, args } => {
                        run_mcp_client_call(tool_name, args.as_deref()).await?
                    }
                },
            },
        },
    }
    Ok(())
}

/// Runtime handler for MCP client connect command using session manager
async fn run_mcp_client_connect(command: &str, args: &[String]) -> anyhow::Result<()> {
    use crate::mcp::client::{ConnectionConfig, McpSessionManager};
    use std::time::Duration;

    info!("Connecting to MCP server: {} {:?}", command, args);

    // Build connection configuration
    let config = ConnectionConfig::builder()
        .command(command)
        .args(args.to_vec())
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to build connection configuration")?;

    // Connect using session manager
    McpSessionManager::connect(config)
        .await
        .context("Failed to connect to MCP server")?;

    info!("✅ Successfully connected to MCP server: {}", command);
    Ok(())
}

/// Runtime handler for MCP client list tools command using session manager
async fn run_mcp_client_list_tools() -> anyhow::Result<()> {
    use crate::mcp::client::McpSessionManager;

    info!("Listing tools from MCP server");

    let tools = McpSessionManager::list_tools()
        .await
        .context("Failed to list tools from MCP server")?;

    if tools.is_empty() {
        println!("No tools available from the MCP server.");
    } else {
        println!("Available tools:");
        for tool in &tools {
            println!("  • {}", tool);
        }
        println!("\n✅ Found {} tool(s)", tools.len());
    }

    Ok(())
}

/// Runtime handler for MCP client call tool command using session manager
async fn run_mcp_client_call(tool_name: &str, args: Option<&str>) -> anyhow::Result<()> {
    use crate::mcp::client::McpSessionManager;

    // Parse arguments JSON or use empty object
    let parsed_args = match args {
        Some(json_str) => {
            serde_json::from_str(json_str).context("Failed to parse JSON arguments")?
        }
        None => serde_json::Value::Object(serde_json::Map::new()),
    };

    info!("Calling tool '{}' with args: {}", tool_name, parsed_args);
    println!("Calling tool: {}", tool_name);
    if args.is_some() {
        println!("Arguments: {}", serde_json::to_string_pretty(&parsed_args)?);
    }

    // Call the tool using session manager
    let result = McpSessionManager::call_tool(tool_name, parsed_args)
        .await
        .context("Failed to call tool")?;

    // Pretty print the result
    println!("\n✅ Tool result:");
    println!("{}", serde_json::to_string_pretty(&result)?);

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
    let output_path =
        TemplateDir::resolve_output_dir(params.project_name, params.output_dir.as_deref())
            .context("Failed to resolve output directory")?;

    // Initialize the template manager with MCP protocol
    let template_manager = TemplateManager::new_with_protocol(
        Protocol::Mcp,
        template_kind_enum,
        params.template_dir.clone(),
    )
    .await
    .context("Failed to initialize server template manager")?;

    // Create output directory if it doesn't exist
    if !output_path.exists() {
        info!(path = %output_path.display(), "Creating output directory");
        tokio::fs::create_dir_all(&output_path).await.map_err(|e| {
            error!(path = %output_path.display(), error = %e, "Failed to create output directory");
            anyhow::anyhow!("Failed to create output directory: {}", e)
        })?
    }

    // Load OpenAPI schema
    let schema_obj = OpenApiContext::from_file_or_url(params.schema_path)
        .await
        .context("Failed to load OpenAPI schema")?;

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
    let output_path = TemplateDir::resolve_output_dir(project_name, output_dir.as_deref())
        .context("Failed to resolve output directory")?;

    // Initialize template manager for the chosen client template with MCP protocol
    let template_manager = TemplateManager::new_client_with_protocol(
        Protocol::Mcp,
        template_kind_enum,
        template_dir.clone(),
    )
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
