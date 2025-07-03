//! Unified template source abstraction for filesystem and embedded templates.
//!
//! This module provides a unified interface for discovering and accessing templates
//! regardless of their source (embedded in binary or on filesystem). It implements
//! a fallback strategy that prefers embedded templates but can use filesystem
//! templates when needed.
//!
//! # Template Discovery Strategy
//!
//! 1. If a custom directory is provided, use it directly (filesystem)
//! 2. Check for embedded templates matching the protocol/kind
//! 3. Fall back to filesystem discovery if not found embedded
//!
//! # Use Cases
//!
//! - **Production**: Uses embedded templates from the binary
//! - **Development**: Can use filesystem templates for testing
//! - **Customization**: Users can provide custom template directories

use std::io;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use super::{
    ClientTemplateKind, EmbeddedTemplates, ServerTemplateKind, TemplateDir, TemplateRepository,
};
use crate::core::protocol::Protocol;

/// Represents the source location of templates.
///
/// This enum distinguishes between templates that are embedded in the binary
/// at compile time versus those that exist on the filesystem at runtime.
#[derive(Debug, Clone)]
pub enum TemplateSource {
    /// Templates located on the filesystem.
    ///
    /// The path points to the root directory of the template.
    Filesystem(PathBuf),

    /// Templates embedded in the binary at compile time.
    ///
    /// These templates are always available and version-consistent with the CLI.
    Embedded,
}

/// Provides unified access to templates from multiple sources.
///
/// The `TemplateProvider` implements a discovery strategy that checks embedded
/// templates first, then falls back to filesystem templates. This allows the
/// CLI to work out-of-the-box with embedded templates while still supporting
/// custom template directories.
///
/// # Example
///
/// ```no_run
/// use agenterra::core::templates::{TemplateProvider, ServerTemplateKind};
/// use agenterra::core::protocol::Protocol;
///
/// let provider = TemplateProvider::new();
/// let (source, path) = provider.discover_server_template(
///     Protocol::Mcp,
///     ServerTemplateKind::Rust,
///     None, // No custom directory
/// )?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct TemplateProvider {
    embedded_repo: EmbeddedTemplates,
}

impl TemplateProvider {
    /// Create a new template provider with default embedded repository.
    pub fn new() -> Self {
        Self {
            embedded_repo: EmbeddedTemplates::new(),
        }
    }

    /// Discover a server template with the specified protocol and kind.
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol to use (e.g., MCP, REST, gRPC)
    /// * `kind` - The server template kind (e.g., Rust, Python, TypeScript)
    /// * `custom_dir` - Optional custom directory to use instead of discovery
    ///
    /// # Returns
    ///
    /// Returns a tuple of:
    /// - The template source (Embedded or Filesystem)
    /// - The path to the template directory
    ///
    /// # Discovery Process
    ///
    /// 1. If `custom_dir` is provided, use it directly as a filesystem source
    /// 2. Check if the template exists in embedded resources
    /// 3. Fall back to filesystem discovery using standard paths
    pub fn discover_server_template(
        &self,
        protocol: Protocol,
        kind: ServerTemplateKind,
        custom_dir: Option<&Path>,
    ) -> io::Result<(TemplateSource, PathBuf)> {
        // If custom directory is provided, use filesystem directly
        if let Some(dir) = custom_dir {
            debug!("Using custom template directory: {}", dir.display());
            return Ok((
                TemplateSource::Filesystem(dir.to_path_buf()),
                dir.to_path_buf(),
            ));
        }

        // Check embedded templates first
        let template_path = format!(
            "{}/{}/{}",
            protocol.path_segment(),
            kind.role().as_str(),
            kind.as_str()
        );

        debug!("Checking for embedded template: {}", template_path);

        if self.embedded_repo.has_template(&template_path) {
            info!("Using embedded template: {}", template_path);
            // For embedded templates, we return a virtual path
            return Ok((TemplateSource::Embedded, PathBuf::from(&template_path)));
        }

        // Fall back to filesystem
        debug!("Embedded template not found, falling back to filesystem");
        let template_dir = TemplateDir::discover_with_protocol(protocol, kind, None)?;
        Ok((
            TemplateSource::Filesystem(template_dir.template_path().to_path_buf()),
            template_dir.template_path().to_path_buf(),
        ))
    }

    /// Discover a client template with the specified protocol and kind.
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol to use (e.g., MCP, REST, gRPC)
    /// * `kind` - The client template kind (e.g., Rust, Python, TypeScript)
    /// * `custom_dir` - Optional custom directory to use instead of discovery
    ///
    /// # Returns
    ///
    /// Returns a tuple of:
    /// - The template source (Embedded or Filesystem)
    /// - The path to the template directory
    ///
    /// # Discovery Process
    ///
    /// Same as `discover_server_template` but for client templates.
    pub fn discover_client_template(
        &self,
        protocol: Protocol,
        kind: ClientTemplateKind,
        custom_dir: Option<&Path>,
    ) -> io::Result<(TemplateSource, PathBuf)> {
        // If custom directory is provided, use filesystem directly
        if let Some(dir) = custom_dir {
            debug!("Using custom client template directory: {}", dir.display());
            return Ok((
                TemplateSource::Filesystem(dir.to_path_buf()),
                dir.to_path_buf(),
            ));
        }

        // Check embedded templates first
        let template_path = format!(
            "{}/{}/{}",
            protocol.path_segment(),
            kind.role().as_str(),
            kind.as_str()
        );

        debug!("Checking for embedded client template: {}", template_path);

        if self.embedded_repo.has_template(&template_path) {
            info!("Using embedded client template: {}", template_path);
            return Ok((TemplateSource::Embedded, PathBuf::from(&template_path)));
        }

        // Fall back to filesystem
        debug!("Embedded client template not found, falling back to filesystem");
        let template_dir = TemplateDir::discover_client_with_protocol(protocol, kind, None)?;
        Ok((
            TemplateSource::Filesystem(template_dir.template_path().to_path_buf()),
            template_dir.template_path().to_path_buf(),
        ))
    }

    /// Get a reference to the embedded template repository.
    ///
    /// This is useful when you need direct access to embedded templates
    /// without going through the discovery process.
    pub fn embedded_repository(&self) -> &EmbeddedTemplates {
        &self.embedded_repo
    }
}

impl Default for TemplateProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_embedded_server_template() {
        let provider = TemplateProvider::new();

        let result =
            provider.discover_server_template(Protocol::Mcp, ServerTemplateKind::Rust, None);

        assert!(result.is_ok());
        let (source, path) = result.unwrap();

        // Should find embedded template
        assert!(matches!(source, TemplateSource::Embedded));
        assert_eq!(path.to_str().unwrap(), "mcp/server/rust");
    }

    #[test]
    fn test_discover_embedded_client_template() {
        let provider = TemplateProvider::new();

        let result =
            provider.discover_client_template(Protocol::Mcp, ClientTemplateKind::Rust, None);

        assert!(result.is_ok());
        let (source, path) = result.unwrap();

        // Should find embedded template
        assert!(matches!(source, TemplateSource::Embedded));
        assert_eq!(path.to_str().unwrap(), "mcp/client/rust");
    }

    #[test]
    fn test_custom_dir_takes_precedence() {
        let provider = TemplateProvider::new();
        let temp_dir = tempfile::tempdir().unwrap();

        let result = provider.discover_server_template(
            Protocol::Mcp,
            ServerTemplateKind::Rust,
            Some(temp_dir.path()),
        );

        assert!(result.is_ok());
        let (source, path) = result.unwrap();

        // Should use filesystem for custom directory
        assert!(matches!(source, TemplateSource::Filesystem(_)));
        assert_eq!(path, temp_dir.path());
    }
}
