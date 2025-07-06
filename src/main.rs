//! agenterra CLI entrypoint - simplified to use integration layer
//! Parses command-line arguments and dispatches to the integration layer.
#![deny(unsafe_code)]

mod application;
mod generation;
mod infrastructure;
mod integration;
mod protocols;

use anyhow::Context;
use clap::Parser;
use infrastructure::templates::{EmbeddedTemplateExporter, EmbeddedTemplateRepository};
use integration::{ClientParams, McpClientIntegration, McpServerIntegration, ServerParams};
use reqwest::Url;
use std::path::PathBuf;
use tracing::{Level, info};
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
    /// Agent to Agent Protocol (A2A) - by Google
    A2a {
        #[command(subcommand)]
        role: A2aCommands,
    },
    /// Agent Communication Protocol (ACP) - by IBM
    Acp {
        #[command(subcommand)]
        role: AcpCommands,
    },
    /// Agent Network Protocol (ANP) - by Cisco
    Anp {
        #[command(subcommand)]
        role: AnpCommands,
    },
    /// Model Context Protocol (MCP) servers and clients
    Mcp {
        #[command(subcommand)]
        role: McpCommands,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum McpCommands {
    /// Generate MCP server from OpenAPI specification
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
    /// Generate MCP client
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

// Placeholder enums for unimplemented protocols
#[derive(clap::Subcommand, Debug)]
pub enum A2aCommands {
    /// Generate A2A agent
    Agent {
        /// Project name
        #[arg(long, default_value = "agenterra_a2a_agent")]
        project_name: String,
        /// Template to use
        #[arg(long, default_value = "rust")]
        template: String,
        /// Custom template directory
        #[arg(long)]
        template_dir: Option<PathBuf>,
        /// Output directory
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum AcpCommands {
    /// Generate ACP server
    Server {
        /// Project name
        #[arg(long, default_value = "agenterra_acp_server")]
        project_name: String,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum AnpCommands {
    /// Generate ANP broker
    Broker {
        /// Project name
        #[arg(long, default_value = "agenterra_anp_broker")]
        project_name: String,
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
        /// Optional: Export only a specific template
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
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    info!("Starting Agenterra CLI");
    let cli = Cli::parse();

    match &cli.command {
        Commands::Scaffold { target } => match target {
            TargetCommands::Mcp { role } => handle_mcp_command(role).await?,
            TargetCommands::A2a { .. } => {
                anyhow::bail!(
                    "A2A protocol is not yet implemented. Currently only MCP is supported."
                );
            }
            TargetCommands::Acp { .. } => {
                anyhow::bail!(
                    "ACP protocol is not yet implemented. Currently only MCP is supported."
                );
            }
            TargetCommands::Anp { .. } => {
                anyhow::bail!(
                    "ANP protocol is not yet implemented. Currently only MCP is supported."
                );
            }
        },
        Commands::Templates { action } => handle_template_command(action).await?,
    }

    Ok(())
}

async fn handle_mcp_command(role: &McpCommands) -> anyhow::Result<()> {

    match role {
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
            let params = ServerParams {
                project_name: project_name.clone(),
                schema_path: schema_path.clone(),
                template: template.clone(),
                template_dir: template_dir.clone(),
                output_dir: output_dir.clone(),
                port: *port,
                log_file: log_file.clone(),
                base_url: base_url.clone(),
            };

            McpServerIntegration::generate(params)
                .await
                .context("Failed to generate MCP server")?;

            info!("Successfully generated MCP server");
        }
        McpCommands::Client {
            project_name,
            template,
            template_dir,
            output_dir,
        } => {
            let params = ClientParams {
                project_name: project_name.clone(),
                template: template.clone(),
                template_dir: template_dir.clone(),
                output_dir: output_dir.clone(),
            };

            McpClientIntegration::generate(params)
                .await
                .context("Failed to generate MCP client")?;

            info!("Successfully generated MCP client");
        }
    }

    Ok(())
}

async fn handle_template_command(action: &TemplateCommands) -> anyhow::Result<()> {
    match action {
        TemplateCommands::List => {
            let repository = EmbeddedTemplateRepository::new();
            let use_case = application::ListTemplatesUseCase::new(repository);
            println!("{}", use_case.execute());
        }
        TemplateCommands::Export { path, template } => {
            let exporter = EmbeddedTemplateExporter::new();
            let repository = EmbeddedTemplateRepository::new();
            let use_case = application::ExportTemplatesUseCase::new(exporter, repository);

            match template {
                Some(template_path) => {
                    match use_case.execute_single(template_path, path) {
                        Ok(()) => println!("Exported template {} to {}", template_path, path.display()),
                        Err(application::ApplicationError::TemplateNotFound(_)) => {
                            eprintln!("Template not found: {template_path}");
                            std::process::exit(1);
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                None => {
                    let count = use_case.execute_all(path)?;
                    println!("Exported {} templates to {}", count, path.display());
                }
            }
        }
        TemplateCommands::Info { template } => {
            let repository = EmbeddedTemplateRepository::new();
            let discovery = EmbeddedTemplateRepository::new();
            let use_case = application::TemplateInfoUseCase::new(repository, discovery);
            
            match use_case.execute(template).await {
                Ok(output) => println!("{}", output),
                Err(application::ApplicationError::TemplateNotFound(_)) => {
                    eprintln!("Template not found: {template}");
                    eprintln!("\nRun 'agenterra templates list' to see available templates.");
                    std::process::exit(1);
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    Ok(())
}
