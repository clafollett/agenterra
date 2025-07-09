//! Template repository traits for the infrastructure layer

use crate::infrastructure::{TemplateError, types::*};
use async_trait::async_trait;
use std::io;
use std::path::Path;

/// Trait for accessing templates from various sources
///
/// This trait is used by the CLI for template operations like list, info, and export
pub trait TemplateRepository: Send + Sync {
    /// List all available template manifests in the repository
    fn list_manifests(&self) -> Vec<TemplateManifest>;

    /// Get manifest for a specific template by its path (lightweight - no file contents)
    fn get_manifest(&self, path: &str) -> Result<Option<TemplateManifest>, TemplateError>;

    /// Check if a template exists at the given path
    /// Used for template validation before operations
    fn has_template(&self, path: &str) -> bool;

    /// Get all files belonging to a template as raw bytes
    fn get_template_files(&self, template_path: &str) -> Vec<RawTemplateFile>;
}

/// Trait for discovering templates based on attributes
///
/// This trait is used by the generation domain to find templates
#[async_trait]
pub trait TemplateDiscovery: Send + Sync {
    /// Find a template by its attributes
    async fn discover(
        &self,
        protocol: crate::protocols::Protocol,
        role: crate::protocols::Role,
        language: crate::generation::Language,
    ) -> Result<Template, TemplateError>;
}

/// Trait for exporting templates from a repository to the filesystem
pub trait TemplateExporter: Send + Sync {
    /// Export a single template to the specified directory
    fn export_template(&self, template: &TemplateManifest, output_dir: &Path) -> io::Result<()>;

    /// Export all available templates to the specified directory
    fn export_all_templates(&self, output_dir: &Path) -> io::Result<usize>;
}

/// Trait for loading a single template bundle from a path
///
/// This trait is used when the user provides a --template-dir flag
/// to load a specific template bundle from the filesystem
#[async_trait]
pub trait TemplateLoader: Send + Sync {
    /// Load a template bundle from a specific path
    async fn load_template(&self, path: &Path) -> Result<Template, TemplateError>;
}
