//! Unified template source abstraction for filesystem and embedded templates

#![allow(dead_code)]

use std::io;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use super::{
    ClientTemplateKind, TemplateRepository, ServerTemplateKind, TemplateDir,
    EmbeddedTemplateRepository,
};
use crate::core::protocol::Protocol;

/// Represents the source of templates
#[derive(Debug, Clone)]
pub enum TemplateSource {
    /// Templates from filesystem
    Filesystem(PathBuf),
    /// Templates embedded in binary
    Embedded,
}

/// Unified template provider that can work with both filesystem and embedded templates
pub struct TemplateProvider {
    embedded_repo: EmbeddedTemplateRepository,
}

impl TemplateProvider {
    /// Create a new template provider
    pub fn new() -> Self {
        Self {
            embedded_repo: EmbeddedTemplateRepository::new(),
        }
    }

    /// Discover server template with protocol support
    /// Checks embedded templates first, then falls back to filesystem
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

    /// Discover client template with protocol support
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

    /// Get embedded template repository
    pub fn embedded_repository(&self) -> &EmbeddedTemplateRepository {
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
            provider.discover_server_template(Protocol::Mcp, ServerTemplateKind::RustAxum, None);

        assert!(result.is_ok());
        let (source, path) = result.unwrap();

        // Should find embedded template
        assert!(matches!(source, TemplateSource::Embedded));
        assert_eq!(path.to_str().unwrap(), "mcp/server/rust_axum");
    }

    #[test]
    fn test_discover_embedded_client_template() {
        let provider = TemplateProvider::new();

        let result =
            provider.discover_client_template(Protocol::Mcp, ClientTemplateKind::RustReqwest, None);

        assert!(result.is_ok());
        let (source, path) = result.unwrap();

        // Should find embedded template
        assert!(matches!(source, TemplateSource::Embedded));
        assert_eq!(path.to_str().unwrap(), "mcp/client/rust_reqwest");
    }

    #[test]
    fn test_custom_dir_takes_precedence() {
        let provider = TemplateProvider::new();
        let temp_dir = tempfile::tempdir().unwrap();

        let result = provider.discover_server_template(
            Protocol::Mcp,
            ServerTemplateKind::RustAxum,
            Some(temp_dir.path()),
        );

        assert!(result.is_ok());
        let (source, path) = result.unwrap();

        // Should use filesystem for custom directory
        assert!(matches!(source, TemplateSource::Filesystem(_)));
        assert_eq!(path, temp_dir.path());
    }
}
