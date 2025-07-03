//! Template management and rendering for code generation.
//!
//! This module provides the core template management functionality for Agenterra.
//! It handles loading templates, rendering them with context data, and generating
//! code files based on OpenAPI specifications.
//!
//! # Architecture
//!
//! The `TemplateManager` is responsible for:
//! - Loading and caching Tera templates
//! - Managing template manifests
//! - Rendering templates with context data
//! - Processing template files based on manifest rules
//! - Executing post-generation hooks
//!
//! # Template Processing
//!
//! Templates are processed in several stages:
//! 1. **Discovery**: Find template directory based on protocol/kind
//! 2. **Loading**: Load manifest and template files into Tera
//! 3. **Context Building**: Create context with OpenAPI and language-specific data
//! 4. **Rendering**: Process Tera templates with context
//! 5. **Writing**: Generate output files in target directory
//! 6. **Post-processing**: Run any configured hooks
//!
//! # Context Variables
//!
//! Templates have access to various context variables:
//! - OpenAPI specification data
//! - Language-specific helpers and naming conventions
//! - Project metadata (name, version, etc.)
//! - Operation-specific data for endpoint generation

// Internal imports (std, crate)
use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::core::{
    config::Config,
    error::Result,
    openapi::{OpenApiContext, OpenApiOperation},
    protocol::Protocol,
    utils::to_snake_case,
};
use crate::mcp::builders::EndpointContext;

use super::{
    ClientTemplateKind, ManifestTemplateFile, ServerTemplateKind, TemplateDir, TemplateManifest,
    TemplateOptions, TemplateRepository,
    source::{TemplateProvider, TemplateSource},
};

// External imports (alphabetized)
use serde::Serialize;
use serde_json::{Map, Value as JsonValue, json};
use tera::{Context, Tera};
use tracing::{debug, error};

/// Manages loading and rendering of code generation templates
#[derive(Debug, Clone)]
pub struct TemplateManager {
    /// Cached Tera template engine instance
    tera: Arc<Tera>,
    /// Template directory
    template_dir: TemplateDir,
    /// The template manifest
    manifest: TemplateManifest,
}

impl TemplateManager {
    /// Create a new TemplateManager with explicit protocol support
    /// Arguments ordered to match CLI: protocol, template_kind (matching: scaffold `<role>` `<protocol>` `<kind>`)
    ///
    /// # Arguments
    /// * `protocol` - The protocol to use (e.g., MCP)
    /// * `template_kind` - The kind of template to use
    /// * `template_dir` - Optional path to the template directory. If None, the default location will be used.
    ///
    /// # Returns
    /// A new `TemplateManager` instance or an error if the template directory cannot be found or loaded.
    pub async fn new_server(
        protocol: Protocol,
        template_kind: ServerTemplateKind,
        template_dir: Option<PathBuf>,
    ) -> Result<Self> {
        // Use TemplateProvider to discover templates (embedded first, then filesystem)
        let provider = TemplateProvider::new();
        let (source, discovered_path) =
            provider.discover_server_template(protocol, template_kind, template_dir.as_deref())?;

        // Handle the template based on its source
        match source {
            TemplateSource::Embedded => {
                // For embedded templates, we need to load them into Tera from the embedded resources
                let mut tera = Tera::default();

                // Get the embedded repository
                let embedded_repo = provider.embedded_repository();

                // Load embedded templates into Tera
                load_embedded_templates_into_tera(
                    &mut tera,
                    discovered_path.to_str().unwrap(),
                    embedded_repo,
                )?;

                // Load the manifest from embedded resources
                let manifest =
                    load_embedded_manifest(discovered_path.to_str().unwrap(), embedded_repo)
                        .await?;

                // Create a virtual TemplateDir for compatibility
                let template_dir = TemplateDir::from_embedded_path(discovered_path)?;

                Ok(Self {
                    tera: Arc::new(tera),
                    template_dir,
                    manifest,
                })
            }
            TemplateSource::Filesystem(fs_path) => {
                // For filesystem templates, use the existing logic
                let template_dir =
                    TemplateDir::discover_with_protocol(protocol, template_kind, Some(&fs_path))?;

                // Get the template path for Tera
                let template_path = template_dir.template_path();
                let template_dir_str = template_path.to_str().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Template path contains invalid UTF-8",
                    )
                })?;

                // Load the template manifest - try YAML first, then TOML
                let yaml_manifest_path = template_path.join("manifest.yml");
                let toml_manifest_path = template_path.join("manifest.toml");

                let manifest = if yaml_manifest_path.exists() {
                    let manifest_content = tokio::fs::read_to_string(&yaml_manifest_path)
                        .await
                        .map_err(|e| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("Failed to read template manifest: {e}"),
                            )
                        })?;
                    serde_yaml::from_str(&manifest_content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Failed to parse template manifest: {e}"),
                        )
                    })?
                } else if toml_manifest_path.exists() {
                    let manifest_content = tokio::fs::read_to_string(&toml_manifest_path)
                        .await
                        .map_err(|e| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("Failed to read template manifest: {e}"),
                            )
                        })?;
                    toml::from_str(&manifest_content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Failed to parse template manifest: {e}"),
                        )
                    })?
                } else {
                    // Default empty manifest
                    TemplateManifest::default()
                };

                // Create Tera instance with the template directory
                let tera_pattern = format!("{template_dir_str}/**/*");
                debug!(
                    "TemplateManager - Creating Tera with pattern: {}",
                    tera_pattern
                );
                let tera = Tera::new(&tera_pattern).map_err(|e| {
                    error!("Failed to create Tera instance: {}", e);
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Failed to parse templates: {e}"),
                    )
                })?;

                // Return the TemplateManager for filesystem templates
                Ok(Self {
                    tera: Arc::new(tera),
                    template_dir,
                    manifest,
                })
            }
        }
    }

    /// Create a new TemplateManager for client template generation with explicit protocol support
    /// Arguments ordered to match CLI: protocol, template_kind (matching: scaffold `<role>` `<protocol>` `<kind>`)
    ///
    /// # Arguments
    /// * `protocol` - The protocol to use (e.g., MCP)
    /// * `template_kind` - The kind of client template to use
    /// * `template_dir` - Optional path to the template directory. If None, the default location will be used.
    ///
    /// # Returns
    /// A new `TemplateManager` instance or an error if the template directory cannot be found or loaded.
    pub async fn new_client(
        protocol: Protocol,
        template_kind: ClientTemplateKind,
        template_dir: Option<PathBuf>,
    ) -> Result<Self> {
        // Use TemplateProvider to discover templates (embedded first, then filesystem)
        let provider = TemplateProvider::new();
        let (source, discovered_path) =
            provider.discover_client_template(protocol, template_kind, template_dir.as_deref())?;

        // Handle the template based on its source
        match source {
            TemplateSource::Embedded => {
                // For embedded templates, we need to load them into Tera from the embedded resources
                let mut tera = Tera::default();

                // Get the embedded repository
                let embedded_repo = provider.embedded_repository();

                // Load embedded templates into Tera
                load_embedded_templates_into_tera(
                    &mut tera,
                    discovered_path.to_str().unwrap(),
                    embedded_repo,
                )?;

                // Load the manifest from embedded resources
                let manifest =
                    load_embedded_manifest(discovered_path.to_str().unwrap(), embedded_repo)
                        .await?;

                // Create a virtual TemplateDir for compatibility
                let template_dir = TemplateDir::from_embedded_path(discovered_path)?;

                Ok(Self {
                    tera: Arc::new(tera),
                    template_dir,
                    manifest,
                })
            }
            TemplateSource::Filesystem(fs_path) => {
                // For filesystem templates, use the existing logic
                let template_dir = TemplateDir::discover_client_with_protocol(
                    protocol,
                    template_kind,
                    Some(&fs_path),
                )?;

                // Get the template path for Tera
                let template_path = template_dir.template_path();
                let template_dir_str = template_path.to_str().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Template path contains invalid UTF-8",
                    )
                })?;

                // Load template manifest
                let manifest = TemplateManifest::load_from_dir(template_path).await?;

                // Create Tera instance with template files matching glob patterns
                let tera = Tera::new(&format!("{template_dir_str}/**/*")).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Failed to parse client templates: {e}"),
                    )
                })?;

                Ok(TemplateManager {
                    tera: Arc::new(tera),
                    template_dir,
                    manifest,
                })
            }
        }
    }

    /// Get the template kind this template manager is configured for
    pub fn template_kind(&self) -> ServerTemplateKind {
        self.template_dir.kind()
    }

    /// Get the protocol this template manager is configured for
    pub fn protocol(&self) -> Protocol {
        self.template_dir.protocol()
    }

    /// Reload all templates from the template directory.
    /// This is a no-op in the cached implementation since templates are loaded on demand.
    // TODO(CLI): Wire up for `agenterra templates reload` command for development workflow
    #[allow(dead_code)]
    pub async fn reload_templates(&self) -> Result<()> {
        // In the cached implementation, we don't need to do anything here
        // since templates are loaded on demand.
        Ok(())
    }

    /// Get a reference to the template manifest
    // TODO(CLI): Wire up for `agenterra templates info <template>` command to show manifest details
    #[allow(dead_code)]
    pub fn manifest(&self) -> &TemplateManifest {
        &self.manifest
    }

    /// Generate a file from a template with a custom context
    pub async fn generate_with_context<T: Serialize>(
        &self,
        template_name: &str,
        context: &T,
        output_path: impl AsRef<Path>,
    ) -> Result<()> {
        let output_path = output_path.as_ref();

        // First validate required context variables
        let context_value = serde_json::to_value(context).map_err(|e| {
            crate::core::Error::Template(format!("Failed to serialize context: {e}"))
        })?;

        let context_map = context_value.as_object().ok_or_else(|| {
            crate::core::Error::Template("Context must be a JSON object".to_string())
        })?;

        // Define required variables per template type
        let required_vars: &[&str] = &[];

        Self::validate_context(template_name, context_map, required_vars)?;

        let parent = output_path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid output path: {}", output_path.display()),
            )
        })?;

        tokio::fs::create_dir_all(parent).await?;

        // Log the template being rendered
        log::debug!("Rendering template: {template_name}");
        log::debug!("Output path: {}", output_path.display());
        log::debug!("Parent directory: {}", parent.display());

        // Build Tera Context from the already parsed context_map
        let mut tera_context = Context::new();
        for (k, v) in context_map {
            tera_context.insert(k, &v);
        }

        // Verify template exists
        log::debug!("Checking if template exists: {template_name}");
        self.tera.get_template(template_name).map_err(|e| {
            crate::core::Error::Template(format!("Template not found: {template_name} - {e}"))
        })?;

        log::debug!("Found template: {template_name}");
        log::debug!(
            "Available templates: {:?}",
            self.tera.get_template_names().collect::<Vec<_>>()
        );

        // Render the template with detailed error reporting
        let content = match self.tera.render(template_name, &tera_context) {
            Ok(content) => content,
            Err(e) => {
                // Get the template source for better error reporting
                let template_source = match std::fs::read_to_string(
                    self.template_dir.template_path().join(template_name),
                ) {
                    Ok(source) => source,
                    Err(_) => "<unable to read template file>".to_string(),
                };

                log::error!("Template rendering failed for '{template_name}': {e}");
                log::error!("Tera error details: {e:?}");
                log::error!(
                    "Available context keys: {:?}",
                    context_map.keys().collect::<Vec<_>>()
                );
                return Err(crate::core::Error::Template(format!(
                    "Failed to render template '{template_name}': {e}\nTemplate source:\n{template_source}"
                )));
            }
        };

        log::debug!(
            "Rendered content for {} ({} bytes):\n{}",
            template_name,
            content.len(),
            if content.len() > 200 {
                format!("{}... (truncated)", &content[..200])
            } else {
                content.clone()
            }
        );

        // Ensure the parent directory exists
        log::debug!("Ensuring parent directory exists: {}", parent.display());
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            log::error!("Failed to create directory: {e}");
            return Err(crate::core::Error::Io(e));
        }

        // Write the output file
        log::debug!("Writing to output file: {}", output_path.display());
        tokio::fs::write(&output_path, &content).await?;

        log::debug!("Successfully wrote template to: {}", output_path.display());
        Ok(())
    }

    /// List all available templates
    // TODO(CLI): Wire up for `agenterra templates list` command to show available templates
    #[allow(dead_code)]
    pub fn list_templates(&self) -> Vec<(String, String)> {
        self.manifest
            .files
            .iter()
            .filter(|f| self.tera.get_template(&f.source).is_ok())
            .map(|f| (f.source.clone(), f.destination.clone()))
            .collect()
    }

    /// Generate code from loaded templates based on the OpenAPI spec and options
    pub async fn generate(
        &self,
        spec: &OpenApiContext,
        config: &Config,
        template_opts: Option<TemplateOptions>,
    ) -> Result<()> {
        // Build the base context
        let (base_context, operations) = self.build_context(spec, &template_opts, config).await?;

        // Create output directory
        let output_dir = Path::new(&config.output_dir);
        tokio::fs::create_dir_all(output_dir).await?;

        // Process each template file
        for file in &self.manifest.files {
            log::debug!("Processing file: {} -> {}", file.source, file.destination);
            if let Some(for_each) = &file.for_each {
                log::debug!("File has for_each: {for_each}");
                match for_each.as_str() {
                    "endpoint" | "operation" => {
                        // Convert base_context to Tera Context for operation processing
                        let mut tera_context = Context::new();
                        if let serde_json::Value::Object(obj) = &base_context {
                            for (k, v) in obj {
                                tera_context.insert(k, v);
                            }
                        }

                        self.process_operation_file(
                            file,
                            &tera_context,
                            output_dir,
                            &operations,
                            &template_opts,
                            spec,
                        )
                        .await?;
                    }
                    _ => {
                        return Err(crate::core::Error::Template(format!(
                            "Unknown for_each directive: {for_each}"
                        )));
                    }
                }
            } else {
                // This is a single file template
                log::debug!("Processing single file template: {}", file.source);
                let dest_path = output_dir.join(&file.destination);
                self.process_single_file(file, &base_context, &dest_path)
                    .await?;
            }
        }

        // Execute post-generation hooks
        self.execute_post_generation_hooks(output_dir).await?;

        Ok(())
    }

    /// Generate client code without requiring OpenAPI specification
    ///
    /// This method generates MCP client code that can discover tools at runtime.
    /// Unlike server generation, it doesn't require an OpenAPI schema.
    pub async fn generate_client(
        &self,
        config: &Config,
        template_opts: Option<TemplateOptions>,
    ) -> Result<()> {
        // Build client-specific context (no OpenAPI needed)
        let base_context = self.build_client_context(config, &template_opts).await?;

        // Parse the client template kind from the config
        let client_kind: crate::core::templates::ClientTemplateKind =
            config.template_kind.parse().map_err(|e| {
                crate::core::Error::Template(format!(
                    "Invalid client template kind '{}': {}",
                    config.template_kind, e
                ))
            })?;

        let builder = crate::mcp::builders::EndpointContext::get_client_builder(client_kind);

        // Normalize the context for the target language
        let normalized_context = builder.build(&base_context)?;

        // Create output directory
        let output_dir = Path::new(&config.output_dir);
        tokio::fs::create_dir_all(output_dir).await?;

        // Process each template file (client templates don't use for_each typically)
        for file in &self.manifest.files {
            log::debug!(
                "Processing client file: {} -> {}",
                file.source,
                file.destination
            );

            // Client templates typically don't use for_each operations
            // since they don't depend on OpenAPI operations
            let output_path = output_dir.join(&file.destination);

            // Generate file with normalized context
            self.generate_with_context(&file.source, &normalized_context, &output_path)
                .await?;
        }

        // Execute post-generation hooks from manifest
        self.execute_post_generation_hooks(output_dir).await?;

        Ok(())
    }

    /// Build client-specific template context (no OpenAPI required)
    async fn build_client_context(
        &self,
        config: &Config,
        template_opts: &Option<TemplateOptions>,
    ) -> Result<serde_json::Value> {
        let mut base_map = serde_json::Map::new();

        // Add basic project information
        base_map.insert("project_name".to_string(), json!(config.project_name));
        base_map.insert("output_dir".to_string(), json!(config.output_dir));

        // Add protocol information
        base_map.insert("protocol".to_string(), json!(self.protocol().name()));

        // Add template options if provided
        if let Some(opts) = template_opts {
            if let Some(port) = opts.server_port {
                base_map.insert("server_port".to_string(), json!(port));
            }
            if let Some(log_file) = &opts.log_file {
                base_map.insert("log_file".to_string(), json!(log_file));
            }
        }

        // Add default values - use workspace version
        base_map.insert("version".to_string(), json!(env!("CARGO_PKG_VERSION")));
        base_map.insert(
            "description".to_string(),
            json!(format!("MCP client for {}", config.project_name)),
        );

        // Set CLI binary name to match project name by default
        base_map.insert("cli_binary_name".to_string(), json!(config.project_name));

        // Add missing required template variables
        base_map.insert("license".to_string(), json!("MIT"));
        base_map.insert(
            "contributing".to_string(),
            json!("Contributions are welcome! Please submit pull requests or open issues."),
        );

        Ok(json!(base_map))
    }

    /// Build the complete template context from OpenAPI spec
    async fn build_context(
        &self,
        openapi_context: &OpenApiContext,
        template_opts: &Option<TemplateOptions>,
        config: &Config,
    ) -> Result<(serde_json::Value, Vec<OpenApiOperation>)> {
        let mut base_map = serde_json::Map::new();

        // Add project name from config (user-specified)
        base_map.insert("project_name".to_string(), json!(config.project_name));

        // Add protocol information
        base_map.insert("protocol".to_string(), json!(self.protocol().name()));

        // Add project title and version from spec
        if let Some(title) = openapi_context.title() {
            base_map.insert("api_title".to_string(), json!(title));
            let sanitized_name = to_snake_case(title);
            base_map.insert("project_name_from_spec".to_string(), json!(sanitized_name));
        }

        // Add API version from spec
        if let Some(version) = openapi_context.version() {
            base_map.insert("api_version".to_string(), json!(version));
        }

        // Add MCP Agent instructions if provided, or default to empty
        if let Some(opts) = template_opts {
            if let Some(instructions) = &opts.agent_instructions {
                base_map.insert("agent_instructions".to_string(), instructions.clone());
            } else {
                base_map.insert("agent_instructions".to_string(), json!(""));
            }
        } else {
            base_map.insert("agent_instructions".to_string(), json!(""));
        }

        // Add the full spec to the context if needed
        if let Ok(spec_value) = serde_json::to_value(openapi_context) {
            base_map.insert("spec".to_string(), spec_value);
        }

        // Add spec file name for reference in templates
        base_map.insert("spec_file_name".to_string(), json!("openapi.json"));

        // Extract operations from the OpenAPI spec
        let operations = openapi_context.parse_operations().await?;

        // Transform endpoints using language-specific builder
        let endpoints =
            EndpointContext::transform_endpoints(self.template_kind(), operations.clone())?;
        base_map.insert("endpoints".to_string(), json!(endpoints));

        // Add server configuration variables needed by templates
        base_map.insert("log_file".to_string(), json!("agenterra"));
        base_map.insert("server_port".to_string(), json!(8080));

        // Add any template options to the context if provided
        if let Some(opts) = template_opts {
            // Override defaults with template options if provided
            if let Some(port) = opts.server_port {
                base_map.insert("server_port".to_string(), json!(port));
            }
            if let Some(log_file) = &opts.log_file {
                base_map.insert("log_file".to_string(), json!(log_file));
            }
        }

        // Add base API URL from OpenAPI spec and user-provided base URL
        if let Some(spec_url) = openapi_context.base_path() {
            let final_url = if spec_url.starts_with("http://") || spec_url.starts_with("https://") {
                // Spec contains a fully qualified URL, use it directly
                spec_url
            } else if spec_url.starts_with("/") {
                // Spec contains a relative path, combine with user-provided base URL
                if let Some(base_url) = &config.base_url {
                    let base_str = base_url.to_string();
                    let trimmed = base_str.trim_end_matches('/');
                    format!("{trimmed}{spec_url}")
                } else {
                    return Err(crate::core::Error::Template(format!(
                        "OpenAPI spec contains a relative server URL '{spec_url}', but no --base-url was provided. Please provide a base URL (e.g., --base-url https://api.example.com)"
                    )));
                }
            } else {
                return Err(crate::core::Error::Template(format!(
                    "Invalid server URL format in OpenAPI spec: '{spec_url}'. URL must be either a fully qualified URL (https://api.example.com/v1) or a relative path (/api/v1)"
                )));
            };
            base_map.insert("base_api_url".to_string(), json!(final_url));
        } else {
            return Err(crate::core::Error::Template(
                "No server URL found in OpenAPI spec. Please define at least one server in the 'servers' section (OpenAPI 3.0+) or 'host' field (Swagger 2.0) of your OpenAPI specification".to_string()
            ));
        }

        // For debugging, log the context keys
        let keys_str: Vec<String> = base_map.keys().map(|k| k.to_string()).collect();
        log::debug!("Template context keys: {}", keys_str.join(", "));

        Ok((serde_json::Value::Object(base_map), operations))
    }

    /// Process a single template file
    async fn process_single_file(
        &self,
        file: &ManifestTemplateFile,
        base_context: &serde_json::Value,
        output_path: &Path,
    ) -> Result<()> {
        log::debug!(
            "Processing single file: {} -> {}",
            file.source,
            output_path.display()
        );

        // Create the output directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                log::debug!("Creating parent directory: {}", parent.display());
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    io::Error::other(format!("Failed to create output directory: {e}"))
                })?;
            }
        }

        // Create the file context
        let file_context = self.create_file_context(base_context, file)?;
        log::debug!(
            "File context keys: {:?}",
            file_context
                .as_object()
                .map(|obj| obj.keys().collect::<Vec<_>>())
                .unwrap_or_default()
        );

        // Convert serde_json::Value to tera::Context
        let mut tera_context = Context::new();
        if let serde_json::Value::Object(ref map) = file_context {
            for (key, value) in map {
                tera_context.insert(key, value);
            }
        }

        // Log context contents for debugging
        log::debug!(
            "Tera context for {}: {:?}",
            file.source,
            tera_context
                .clone()
                .into_json()
                .as_object()
                .map(|obj| obj.keys().collect::<Vec<_>>())
                .unwrap_or_default()
        );

        // Special debug for handlers_mod.rs.tera
        if file.source == "handlers_mod.rs.tera" {
            let context_json = tera_context.clone().into_json();
            if let Some(endpoints) = context_json.get("endpoints") {
                log::debug!(
                    "Endpoints data structure: {}",
                    serde_json::to_string_pretty(endpoints)
                        .unwrap_or_else(|_| "Failed to serialize".to_string())
                );
            }
            if let Some(base_api_url) = context_json.get("base_api_url") {
                log::debug!("base_api_url value: {base_api_url:?}");
            }
        }

        // Render the template with detailed error handling
        let rendered = match self.tera.render(&file.source, &tera_context) {
            Ok(content) => {
                log::debug!("Successfully rendered template {}", file.source);
                content
            }
            Err(e) => {
                log::error!("Tera rendering error for {}: {}", file.source, e);
                log::error!("Template source: {}", file.source);
                log::error!(
                    "Available context keys: {:?}",
                    tera_context
                        .clone()
                        .into_json()
                        .as_object()
                        .map(|obj| obj.keys().collect::<Vec<_>>())
                        .unwrap_or_default()
                );

                // Check if template exists
                if let Err(template_err) = self.tera.get_template(&file.source) {
                    log::error!("Template not found: {template_err}");
                }

                // Get more specific error information
                log::error!("Tera error kind: {:?}", e.kind);
                log::error!("Full error chain: {e:#}");

                return Err(crate::core::Error::Template(format!(
                    "Failed to render template '{}': {}",
                    file.source, e
                )));
            }
        };

        // Write the file
        log::debug!("Writing rendered content to: {}", output_path.display());
        tokio::fs::write(output_path, rendered).await.map_err(|e| {
            log::error!("Failed to write file {}: {}", output_path.display(), e);
            crate::core::Error::Io(e)
        })?;

        log::debug!("Successfully processed file: {}", output_path.display());
        Ok(())
    }

    /// Process a template file for each operation
    async fn process_operation_file(
        &self,
        file: &ManifestTemplateFile,
        base_context: &Context,
        output_path: &Path,
        operations: &[OpenApiOperation],
        template_opts: &Option<TemplateOptions>,
        spec: &OpenApiContext,
    ) -> Result<()> {
        // Create schemas directory
        let schemas_dir = output_path.join("schemas");
        tokio::fs::create_dir_all(&schemas_dir)
            .await
            .map_err(|e| io::Error::other(format!("Failed to create schemas directory: {e}")))?;

        for operation in operations {
            // Language-specific fields like fn_name must be injected by a builder; OpenApiOperation is language-agnostic.
            let include = template_opts
                .as_ref()
                .map(|opts| {
                    opts.all_operations
                        || opts.include_operations.is_empty()
                        || opts.include_operations.contains(&operation.id)
                })
                .unwrap_or(true);
            let exclude = template_opts
                .as_ref()
                .map(|opts| opts.exclude_operations.contains(&operation.id))
                .unwrap_or(false);

            if include && !exclude {
                let mut context = base_context.clone();

                let builder = EndpointContext::get_builder(self.template_kind());
                let endpoint_context = builder.build(operation)?;

                // Merge the endpoint context into the template context
                if let Some(obj) = endpoint_context.as_object() {
                    for (key, value) in obj {
                        context.insert(key, &value);
                    }
                }

                // Add operation metadata
                context.insert("operation_id", &operation.id);
                context.insert("method", &operation.method);
                context.insert("path", &operation.path);

                // Insert OpenAPI-native fields
                context.insert("operation_id", &operation.id);

                // Sanitize and add text fields
                let sanitized_summary = operation.summary.as_deref().map(|s| {
                    s.chars()
                        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
                        .collect::<String>()
                        .trim()
                        .to_string()
                });

                let sanitized_description = operation.description.as_deref().map(|s| {
                    s.chars()
                        .filter(|c| {
                            c.is_ascii_alphanumeric() || c.is_whitespace() || *c == '.' || *c == ','
                        })
                        .collect::<String>()
                        .trim()
                        .to_string()
                });

                context.insert("summary", &sanitized_summary);
                context.insert("description", &sanitized_description);
                context.insert("deprecated", &operation.deprecated);

                // Add tags with proper sanitization
                let sanitized_tags: Vec<String> = operation
                    .tags
                    .as_ref()
                    .map(|tags| {
                        tags.iter()
                            .map(|t| t.trim().replace("\n", " ").replace("\r", " "))
                            .collect()
                    })
                    .unwrap_or_default();
                context.insert("tags", &sanitized_tags);

                // Extract and process parameters with proper error handling
                let parameter_info: Vec<serde_json::Value> = operation
                    .parameters
                    .as_ref()
                    .map(|params| {
                        params
                            .iter()
                            .map(|p| {
                                let mut param_obj = serde_json::Map::new();

                                // Required fields
                                param_obj.insert("name".to_string(), json!(&p.name));
                                param_obj.insert("in".to_string(), json!(&p.in_));

                                // Optional fields with their correct names
                                if let Some(desc) = &p.description {
                                    param_obj.insert("description".to_string(), json!(desc));
                                }

                                // Handle required field with path parameter default
                                let is_required = p.required.unwrap_or_else(|| p.in_ == "path");
                                param_obj.insert("required".to_string(), json!(is_required));

                                // Add schema if available
                                if let Some(schema) = &p.schema {
                                    param_obj.insert("schema".to_string(), schema.clone());
                                }

                                // Add content if available (for complex parameters)
                                if let Some(content) = &p.content {
                                    param_obj.insert("content".to_string(), json!(content));
                                }

                                // Add examples if available
                                if let Some(examples) = &p.examples {
                                    param_obj.insert("examples".to_string(), json!(examples));
                                }

                                // Add other optional fields
                                if let Some(deprecated) = p.deprecated {
                                    param_obj.insert("deprecated".to_string(), json!(deprecated));
                                }

                                if let Some(style) = &p.style {
                                    param_obj.insert("style".to_string(), json!(style));
                                }

                                if let Some(explode) = p.explode {
                                    param_obj.insert("explode".to_string(), json!(explode));
                                }

                                // Add allow_empty_value with correct serialization name
                                if let Some(allow_empty) = p.allow_empty_value {
                                    param_obj
                                        .insert("allowEmptyValue".to_string(), json!(allow_empty));
                                }

                                // Add allow_reserved with correct serialization name
                                if let Some(allow_reserved) = p.allow_reserved {
                                    param_obj
                                        .insert("allowReserved".to_string(), json!(allow_reserved));
                                }

                                // Add any vendor extensions
                                if !p.vendor_extensions.is_empty() {
                                    for (key, value) in &p.vendor_extensions {
                                        if key.starts_with("x-") {
                                            param_obj.insert(key.clone(), value.clone());
                                        }
                                    }
                                }

                                json!(param_obj)
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                // Add parameters to context
                context.insert(
                    "parameters",
                    &operation.parameters.clone().unwrap_or_default(),
                );
                context.insert("parameter_info", &parameter_info);

                // Process responses
                context.insert("responses", &operation.responses);

                // Add request body if present with sanitized properties
                if let Some(request_body) = &operation.request_body {
                    context.insert("has_request_body", &true);
                    context.insert("request_body", request_body);

                    // Use the operation's method to extract request body properties
                    match spec.extract_request_body_properties(operation) {
                        Ok((props, _)) if !props.is_null() => {
                            let property_info = OpenApiContext::extract_property_info(&props);
                            context.insert("request_properties", &property_info);
                        }
                        _ => {
                            // Fallback to basic property extraction if the above fails
                            if let Some(content) = request_body
                                .get("content")
                                .and_then(serde_json::Value::as_object)
                            {
                                for (_content_type, media_type) in content {
                                    if let Some(schema) = media_type.get("schema") {
                                        let property_info =
                                            OpenApiContext::extract_property_info(schema);
                                        context.insert("request_properties", &property_info);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    context.insert("has_request_body", &false);
                }

                // Add security requirements if present
                if let Some(security) = &operation.security {
                    context.insert("security", security);
                }

                // Add sanitized names for use in generated code
                let sanitized_operation_name = operation
                    .id
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect::<String>();
                context.insert("sanitized_operation_name", &sanitized_operation_name);

                let endpoint_fs = if let Some(endpoint_val) = endpoint_context.get("endpoint_fs") {
                    endpoint_val.as_str().unwrap_or(&operation.id)
                } else {
                    &operation.id
                };

                let endpoint_name = if let Some(endpoint_val) = endpoint_context.get("endpoint") {
                    endpoint_val.as_str().unwrap_or(&operation.id)
                } else {
                    &operation.id
                };

                let sanitized_filename = to_snake_case(endpoint_fs);
                context.insert("sanitized_filename", &sanitized_filename);

                log::debug!("Processing template for operation: {}", operation.id);

                // Generate schema file with proper schema extraction
                // Use snake_case for the filename to match MCP conventions
                let schema_filename = to_snake_case(&operation.id);
                let schema_path = schemas_dir.join(format!("{schema_filename}.json"));
                let mut schema_value = serde_json::to_value(operation)?;

                // Dereference all $ref in the schema
                Self::dereference_schema_refs(&mut schema_value, spec)?;

                // Remove null values from the schema
                schema_value
                    .as_object_mut()
                    .unwrap()
                    .retain(|_, v| v != &json!(null));

                let schema_json = serde_json::to_string_pretty(&schema_value)?;
                tokio::fs::write(&schema_path, schema_json)
                    .await
                    .map_err(|e| {
                        io::Error::other(format!(
                            "Failed to write schema file {}: {}",
                            schema_path.display(),
                            e
                        ))
                    })?;

                // Generate the output path with sanitized operation_id
                let output_file = file
                    .destination
                    .replace("{{operation_id}}", endpoint_fs)
                    .replace("{operation_id}", endpoint_fs)
                    .replace("{{endpoint}}", endpoint_name)
                    .replace("{endpoint}", endpoint_name);
                let output_path = output_path.join(&output_file);

                // Create parent directories if they don't exist
                if let Some(parent) = output_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                // Render the template
                let rendered = self.tera.render(&file.source, &context).map_err(|e| {
                    io::Error::other(format!("Failed to render template {}: {}", file.source, e))
                })?;

                // Write the file
                tokio::fs::write(&output_path, rendered)
                    .await
                    .map_err(|e| {
                        io::Error::other(format!(
                            "Failed to write file {}: {}",
                            output_path.display(),
                            e
                        ))
                    })?;
            }
        }
        Ok(())
    }

    /// Validates that all required context variables are present
    fn validate_context(
        template: &str,
        context: &Map<String, JsonValue>,
        required_vars: &[&str],
    ) -> crate::core::error::Result<()> {
        let mut missing = Vec::new();

        for var in required_vars {
            if !context.contains_key(*var) {
                missing.push(var.to_string());
            }
        }

        if !missing.is_empty() {
            return Err(crate::core::Error::Template(format!(
                "Missing required context variables for template '{}': {}",
                template,
                missing.join(", ")
            )));
        }
        Ok(())
    }

    /// Execute post-generation hooks from the manifest
    pub async fn execute_post_generation_hooks(
        &self,
        output_path: &std::path::Path,
    ) -> crate::core::error::Result<()> {
        use tokio::process::Command as AsyncCommand;

        if !self.manifest.hooks.post_generate.is_empty() {
            for command in &self.manifest.hooks.post_generate {
                log::info!("Running post-generation hook: {command}");
                let output = AsyncCommand::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(output_path)
                    .output()
                    .await
                    .map_err(|e| {
                        io::Error::other(format!(
                            "Failed to execute post-generation hook '{command}': {e}"
                        ))
                    })?;

                if !output.status.success() {
                    return Err(io::Error::other(format!(
                        "Post-generation hook '{}' failed with status {}\n{}{}",
                        command,
                        output.status,
                        String::from_utf8_lossy(&output.stderr),
                        String::from_utf8_lossy(&output.stdout)
                    ))
                    .into());
                }
            }
        }
        Ok(())
    }

    /// Merge base context with file context, giving precedence to file context keys
    pub fn create_file_context(
        &self,
        base_context: &serde_json::Value,
        file: &ManifestTemplateFile,
    ) -> crate::core::error::Result<serde_json::Value> {
        let mut context = if let serde_json::Value::Object(file_ctx) = &file.context {
            file_ctx.clone()
        } else {
            serde_json::Map::new()
        };
        if let serde_json::Value::Object(base_map) = base_context {
            for (k, v) in base_map {
                if !context.contains_key(k) {
                    context.insert(k.clone(), v.clone());
                }
            }
        }
        Ok(serde_json::Value::Object(context))
    }

    /// Dereference all $ref in a JSON value by replacing them with actual schema definitions
    fn dereference_schema_refs(value: &mut serde_json::Value, spec: &OpenApiContext) -> Result<()> {
        match value {
            serde_json::Value::Object(map) => {
                // Check if this object contains a $ref
                if let Some(ref_value) = map.get("$ref") {
                    if let Some(ref_str) = ref_value.as_str() {
                        if ref_str.starts_with("#/components/schemas/") {
                            let schema_name = ref_str.trim_start_matches("#/components/schemas/");

                            // Get the actual schema definition
                            if let Some(components) = spec.json.get("components") {
                                if let Some(schemas) = components.get("schemas") {
                                    if let Some(schema_def) = schemas.get(schema_name) {
                                        // Replace the entire object with the dereferenced schema
                                        *value = schema_def.clone();
                                        // Continue dereferencing in the new value
                                        Self::dereference_schema_refs(value, spec)?;
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }

                // Recursively process all values in the object
                for (_, v) in map.iter_mut() {
                    Self::dereference_schema_refs(v, spec)?;
                }
            }
            serde_json::Value::Array(arr) => {
                // Recursively process all items in the array
                for item in arr.iter_mut() {
                    Self::dereference_schema_refs(item, spec)?;
                }
            }
            _ => {} // Other types don't need processing
        }
        Ok(())
    }
}

/// Load embedded templates into Tera
#[allow(dead_code)]
pub fn load_embedded_templates_into_tera<T: TemplateRepository>(
    tera: &mut Tera,
    template_path: &str,
    repository: &T,
) -> io::Result<()> {
    let files = repository.get_template_files(template_path);

    tracing::info!(
        "Loading {} embedded template files for {}",
        files.len(),
        template_path
    );

    for file in files {
        let content = String::from_utf8(file.contents).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Template {} is not valid UTF-8: {}", file.relative_path, e),
            )
        })?;

        // Add template to Tera with its relative path
        tera.add_raw_template(&file.relative_path, &content)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to add template {}: {}", file.relative_path, e),
                )
            })?;

        debug!("Added embedded template: {}", file.relative_path);
    }

    Ok(())
}

/// Load manifest from embedded templates
#[allow(dead_code)]
pub async fn load_embedded_manifest<T: TemplateRepository>(
    template_path: &str,
    repository: &T,
) -> io::Result<TemplateManifest> {
    let files = repository.get_template_files(template_path);

    // Look for manifest.yml or manifest.toml
    for file in files {
        if file.relative_path == "manifest.yml" {
            let content = String::from_utf8(file.contents).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Manifest is not valid UTF-8: {e}"),
                )
            })?;

            return serde_yaml::from_str(&content).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse manifest.yml: {e}"),
                )
            });
        } else if file.relative_path == "manifest.toml" {
            let content = String::from_utf8(file.contents).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Manifest is not valid UTF-8: {e}"),
                )
            })?;

            return toml::from_str(&content).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse manifest.toml: {e}"),
                )
            });
        }
    }

    // Return default manifest if not found
    Ok(TemplateManifest::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{protocol::Protocol, templates::TemplateHooks};
    use serde_json::{Map, json};
    use tempfile;
    use tokio;

    #[test]
    fn test_validate_context() {
        let mut context = Map::new();
        context.insert("foo".to_string(), json!("bar"));
        context.insert("baz".to_string(), json!(123));

        // Test with no required vars
        assert!(TemplateManager::validate_context("test_template", &context, &[]).is_ok());

        // Test with required vars that exist
        assert!(TemplateManager::validate_context("test_template", &context, &["foo"]).is_ok());
        assert!(TemplateManager::validate_context("test_template", &context, &["baz"]).is_ok());
        assert!(
            TemplateManager::validate_context("test_template", &context, &["foo", "baz"]).is_ok()
        );

        // Test with missing required var
        let result = TemplateManager::validate_context("test_template", &context, &["missing"]);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Missing required context variables"));
            assert!(e.to_string().contains("missing"));
        }

        // Test with mix of existing and missing vars
        let result =
            TemplateManager::validate_context("test_template", &context, &["foo", "missing"]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_template_manager() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let templates_base_dir = temp_dir.path();
        let template_dir = templates_base_dir
            .join("templates")
            .join("mcp")
            .join("server")
            .join("custom");
        tokio::fs::create_dir_all(&template_dir).await?;

        // Create a simple template
        let template_content = "Hello {{ name }}!";
        let template_path = template_dir.join("test.tera");
        tokio::fs::write(&template_path, template_content).await?;

        // Create a test manifest
        let manifest = TemplateManifest {
            name: "test".to_string(),
            description: "Test template".to_string(),
            version: "0.1.1".to_string(),
            language: "rust".to_string(),
            files: vec![],
            hooks: TemplateHooks::default(),
        };
        let manifest_path = template_dir.join("manifest.yml");
        let manifest_yaml = serde_yaml::to_string(&manifest).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize manifest: {e}"),
            )
        })?;
        tokio::fs::write(&manifest_path, manifest_yaml).await?;

        // Test creating a new TemplateManager
        // Use the actual template directory since custom paths are used directly
        let manager = TemplateManager::new_server(
            Protocol::Mcp,
            ServerTemplateKind::Custom,
            Some(template_dir.to_path_buf()),
        )
        .await?;

        // Test template_kind
        assert_eq!(manager.template_kind(), ServerTemplateKind::Custom);

        // Test protocol
        assert_eq!(manager.protocol(), Protocol::Mcp);

        // Test that template path exists and is readable
        assert!(manager.template_dir.template_path().exists());

        // Test list_templates
        let templates = manager.list_templates();
        assert!(templates.is_empty()); // No files in manifest yet

        // Test template rendering
        let mut context = tera::Context::new();
        context.insert("name", "World");

        // Test rendering the template
        let output = manager.tera.render("test.tera", &context).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to render template: {e}"),
            )
        })?;

        assert_eq!(output, "Hello World!");

        Ok(())
    }

    // TDD Red phase: Protocol-aware TemplateManager tests
    #[tokio::test]
    async fn test_template_manager_with_protocol() -> Result<()> {
        use crate::core::protocol::Protocol;

        let temp_dir = tempfile::tempdir()?;
        let templates_base_dir = temp_dir.path();
        let template_dir = templates_base_dir
            .join("templates")
            .join("mcp")
            .join("server")
            .join("custom");
        tokio::fs::create_dir_all(&template_dir).await?;

        // Create a simple template
        let template_content = "Hello {{ name }}! Protocol: {{ protocol }}";
        let template_path = template_dir.join("test.tera");
        tokio::fs::write(&template_path, template_content).await?;

        // Create a test manifest
        let manifest = TemplateManifest {
            name: "test".to_string(),
            description: "Test template".to_string(),
            version: "0.1.1".to_string(),
            language: "rust".to_string(),
            files: vec![],
            hooks: TemplateHooks::default(),
        };
        let manifest_path = template_dir.join("manifest.yml");
        let manifest_yaml = serde_yaml::to_string(&manifest).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize manifest: {e}"),
            )
        })?;
        tokio::fs::write(&manifest_path, manifest_yaml).await?;

        // Test creating a new TemplateManager with protocol parameter
        let manager = TemplateManager::new_server(
            Protocol::Mcp,
            ServerTemplateKind::Custom,
            Some(templates_base_dir.to_path_buf()),
        )
        .await?;

        // Test that we can access the protocol
        assert_eq!(manager.protocol(), Protocol::Mcp);
        assert_eq!(manager.template_kind(), ServerTemplateKind::Custom);

        Ok(())
    }

    #[tokio::test]
    async fn test_client_template_manager_with_protocol() -> Result<()> {
        use crate::core::protocol::Protocol;

        let temp_dir = tempfile::tempdir()?;
        let templates_base_dir = temp_dir.path();
        let template_dir = templates_base_dir
            .join("templates")
            .join("mcp")
            .join("client")
            .join("custom");
        tokio::fs::create_dir_all(&template_dir).await?;

        // Create a simple template
        let template_content = "Hello client! Protocol: {{ protocol }}";
        let template_path = template_dir.join("test.tera");
        tokio::fs::write(&template_path, template_content).await?;

        // Create a test manifest
        let manifest = TemplateManifest {
            name: "test-client".to_string(),
            description: "Test client template".to_string(),
            version: "0.1.1".to_string(),
            language: "rust".to_string(),
            files: vec![],
            hooks: TemplateHooks::default(),
        };
        let manifest_path = template_dir.join("manifest.yml");
        let manifest_yaml = serde_yaml::to_string(&manifest).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize manifest: {e}"),
            )
        })?;
        tokio::fs::write(&manifest_path, manifest_yaml).await?;

        // Test creating a new client TemplateManager with protocol parameter
        // Use the actual template directory since custom paths are used directly
        let manager = TemplateManager::new_client(
            Protocol::Mcp,
            ClientTemplateKind::Custom,
            Some(template_dir.to_path_buf()),
        )
        .await?;

        // Test that we can access the protocol
        assert_eq!(manager.protocol(), Protocol::Mcp);

        Ok(())
    }
}
