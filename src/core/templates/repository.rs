//! Domain definitions for template system
//!
//! This module contains the core abstractions for working with templates in Agenterra.
//! It defines traits and types that allow for multiple implementations of template
//! storage and access (e.g., embedded in binary, filesystem, remote repositories).
//!
//! # Core Concepts
//!
//! - **Template**: A collection of files that form a project scaffold
//! - **Template Repository**: A source that provides access to templates
//! - **Template Exporter**: A service that can write templates to the filesystem
//!
//! # Design Principles
//!
//! The interfaces are designed to be:
//! - **Storage-agnostic**: Templates can come from anywhere
//! - **Lazy-loading**: Metadata can be retrieved without loading file contents
//! - **Type-safe**: Strong typing for template types and protocols

/// Trait for exporting templates from a repository to the filesystem.
///
/// This trait defines the interface for services that can take templates
/// from any source and write them to disk, preserving directory structure
/// and file contents.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use agenterra::core::templates::{TemplateExporter, TemplateMetadata};
///
/// fn export_templates(exporter: &impl TemplateExporter) -> std::io::Result<()> {
///     // Export all templates to a directory
///     let count = exporter.export_all_templates(Path::new("/tmp/templates"))?;
///     println!("Exported {} templates", count);
///     Ok(())
/// }
/// ```
pub trait TemplateExporter {
    /// Export a single template to the specified directory.
    ///
    /// This method creates the necessary directory structure under `output_dir`
    /// and writes all files belonging to the template.
    ///
    /// # Arguments
    ///
    /// * `template` - The template metadata describing which template to export
    /// * `output_dir` - The base directory where the template should be written
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the export succeeded, or an `io::Error` if any
    /// file operations failed.
    ///
    /// # Directory Structure
    ///
    /// The template will be exported to `{output_dir}/{template.path}/`.
    /// For example, a template with path "mcp/server/rust" exported to
    /// "/tmp" would create files under "/tmp/mcp/server/rust/".
    fn export_template(
        &self,
        template: &TemplateMetadata,
        output_dir: &std::path::Path,
    ) -> std::io::Result<()>;

    /// Export all available templates to the specified directory.
    ///
    /// This method exports every template available in the repository to
    /// the filesystem, maintaining the original directory structure.
    ///
    /// # Arguments
    ///
    /// * `output_dir` - The base directory where all templates should be written
    ///
    /// # Returns
    ///
    /// Returns the number of templates successfully exported, or an `io::Error`
    /// if any export operation failed.
    fn export_all_templates(&self, output_dir: &std::path::Path) -> std::io::Result<usize>;
}

/// Trait for accessing templates from various sources.
///
/// This trait defines the interface for template repositories, which provide
/// access to template metadata and file contents. Implementations might read
/// from embedded resources, filesystem, git repositories, or remote servers.
///
/// # Design Notes
///
/// The trait is designed to support lazy loading - you can list and query
/// templates without loading all file contents into memory.
///
/// # Examples
///
/// ```no_run
/// use agenterra::core::templates::TemplateRepository;
///
/// fn list_server_templates(repo: &impl TemplateRepository) {
///     let templates = repo.list_templates();
///     for template in templates {
///         if template.template_type == TemplateType::Server {
///             println!("Server template: {} - {}", template.kind,
///                      template.description.as_deref().unwrap_or("No description"));
///         }
///     }
/// }
/// ```
pub trait TemplateRepository {
    /// List all available templates in the repository.
    ///
    /// This method returns metadata for all templates without loading
    /// their file contents, making it efficient for displaying template
    /// catalogs or selection menus.
    ///
    /// # Returns
    ///
    /// A vector of template metadata, typically sorted by path for
    /// consistent ordering.
    fn list_templates(&self) -> Vec<TemplateMetadata>;

    /// Get metadata for a specific template by its path.
    ///
    /// # Arguments
    ///
    /// * `path` - The template path (e.g., "mcp/server/rust")
    ///
    /// # Returns
    ///
    /// Returns `Some(metadata)` if the template exists, or `None` if not found.
    fn get_template(&self, path: &str) -> Option<TemplateMetadata>;

    /// Check if a template exists at the given path.
    ///
    /// This is a convenience method that can be more efficient than
    /// calling `get_template` if you only need to check existence.
    ///
    /// # Arguments
    ///
    /// * `path` - The template path to check
    ///
    /// # Returns
    ///
    /// `true` if the template exists, `false` otherwise.
    fn has_template(&self, path: &str) -> bool;

    /// Get all files belonging to a template.
    ///
    /// This method loads the actual file contents for a template, which
    /// is necessary when generating a project or exporting templates.
    ///
    /// # Arguments
    ///
    /// * `template_path` - The path of the template directory
    ///
    /// # Returns
    ///
    /// A vector of files with their relative paths and contents.
    /// The paths are relative to the template directory.
    fn get_template_files(&self, template_path: &str) -> Vec<TemplateFile>;
}

/// Categorization of templates by their role in a distributed system.
///
/// This enum distinguishes between server-side and client-side templates,
/// which is important for project generation and documentation.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateType {
    /// Server-side template (e.g., API servers, backend services)
    Server,
    /// Client-side template (e.g., CLI clients, SDKs, frontend apps)
    Client,
}

/// Metadata describing a template without including file contents.
///
/// This struct contains all the information needed to display and select
/// templates without the overhead of loading actual file contents.
///
/// # Examples
///
/// ```
/// use agenterra::core::templates::{TemplateMetadata, TemplateType};
///
/// let template = TemplateMetadata {
///     path: "mcp/server/rust".to_string(),
///     template_type: TemplateType::Server,
///     kind: "rust".to_string(),
///     protocol: "mcp".to_string(),
///     description: Some("Rust MCP server using Axum framework".to_string()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateMetadata {
    /// Path relative to templates/ directory (e.g., "mcp/server/rust").
    ///
    /// This path follows the convention: `{protocol}/{role}/{kind}`
    pub path: String,

    /// The role this template serves (Server or Client).
    pub template_type: TemplateType,

    /// The specific template variant (e.g., "rust", "python_fastapi").
    ///
    /// This typically indicates the language and framework combination.
    pub kind: String,

    /// The protocol this template implements (e.g., "mcp", "rest", "grpc").
    pub protocol: String,

    /// Human-readable description from the template's manifest file.
    ///
    /// This is typically loaded from the manifest.yml file and provides
    /// a brief explanation of what the template creates.
    pub description: Option<String>,
}

/// Represents a single file within a template directory.
///
/// This struct contains both the file path (relative to the template root)
/// and the actual file contents as bytes. Using `Vec<u8>` for contents
/// allows handling both text and binary files.
///
/// # Examples
///
/// ```
/// use agenterra::core::templates::TemplateFile;
///
/// let cargo_toml = TemplateFile {
///     relative_path: "Cargo.toml.tera".to_string(),
///     contents: b"[package]\nname = \"{{ project_name }}\"\n".to_vec(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct TemplateFile {
    /// Path relative to the template directory.
    ///
    /// For example, "src/main.rs.tera" or "config/manifest.yml".
    /// Note that template files often have a ".tera" extension to indicate
    /// they contain Tera template syntax.
    pub relative_path: String,

    /// The raw contents of the file as bytes.
    ///
    /// Using `Vec<u8>` allows us to handle any file type, including
    /// binary files like images or compiled assets.
    pub contents: Vec<u8>,
}
