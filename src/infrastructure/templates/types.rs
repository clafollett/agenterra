//! Core template types for the infrastructure layer
//!
//! These types are storage-agnostic and can be used by any template repository
//! implementation (embedded, filesystem, remote, etc.)

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;

/// Template descriptor for locating templates
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateDescriptor {
    pub protocol: crate::protocols::Protocol,
    pub role: crate::protocols::Role,
    pub language: crate::generation::Language,
}

impl TemplateDescriptor {
    pub fn new(
        protocol: crate::protocols::Protocol,
        role: crate::protocols::Role,
        language: crate::generation::Language,
    ) -> Self {
        Self {
            protocol,
            role,
            language,
        }
    }

    /// Get the template path for this descriptor
    pub fn path(&self) -> String {
        format!(
            "{}/{}/{}",
            self.protocol.as_str(),
            self.role_as_str(),
            self.language.as_str()
        )
    }

    // TODO: The role should implement the traits to convert between a string and a role
    fn role_as_str(&self) -> &str {
        match &self.role {
            crate::protocols::Role::Server => "server",
            crate::protocols::Role::Client => "client",
            crate::protocols::Role::Agent => "agent",
            crate::protocols::Role::Broker => "broker",
            crate::protocols::Role::Custom(s) => s,
        }
    }

    /// Parse a template descriptor from a path string (e.g., "mcp/server/rust")
    pub fn from_path(path: &str) -> Option<Self> {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() != 3 {
            return None;
        }

        let protocol = match parts[0] {
            "mcp" => crate::protocols::Protocol::Mcp,
            "a2a" => crate::protocols::Protocol::A2a,
            _ => return None,
        };

        let role = match parts[1] {
            "server" => crate::protocols::Role::Server,
            "client" => crate::protocols::Role::Client,
            "agent" => crate::protocols::Role::Agent,
            "broker" => crate::protocols::Role::Broker,
            other => crate::protocols::Role::Custom(other.to_string()),
        };

        let language = match parts[2] {
            "rust" => crate::generation::Language::Rust,
            "python" => crate::generation::Language::Python,
            "typescript" => crate::generation::Language::TypeScript,
            "go" => crate::generation::Language::Go,
            "java" => crate::generation::Language::Java,
            _ => return None,
        };

        Some(Self::new(protocol, role, language))
    }

    /// Create a descriptor from a template manifest
    pub fn from_manifest(
        manifest: &TemplateManifest,
    ) -> Result<Self, crate::infrastructure::templates::TemplateError> {
        // For filesystem templates, we need to infer the descriptor from the manifest
        // The manifest must have the required metadata

        // Get protocol, role, and language directly from manifest fields
        let protocol = &manifest.protocol;
        let role = &manifest.role;
        let language = &manifest.language;

        // Parse the values
        let protocol = match protocol.as_str() {
            "mcp" => crate::protocols::Protocol::Mcp,
            "a2a" => crate::protocols::Protocol::A2a,
            _ => {
                return Err(
                    crate::infrastructure::templates::TemplateError::InvalidManifest(format!(
                        "Unknown protocol: {}",
                        protocol
                    )),
                );
            }
        };

        let role = match role.as_str() {
            "server" => crate::protocols::Role::Server,
            "client" => crate::protocols::Role::Client,
            "agent" => crate::protocols::Role::Agent,
            "broker" => crate::protocols::Role::Broker,
            other => crate::protocols::Role::Custom(other.to_string()),
        };

        let language = language
            .parse::<crate::generation::Language>()
            .map_err(|_| {
                crate::infrastructure::templates::TemplateError::InvalidManifest(format!(
                    "Unknown language: {}",
                    language
                ))
            })?;

        Ok(Self::new(protocol, role, language))
    }
}

/// Source of a template
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
    Embedded,
    FileSystem(PathBuf),
}

/// A complete template with all its files
#[derive(Debug, Clone)]
pub struct Template {
    pub descriptor: TemplateDescriptor,
    pub manifest: TemplateManifest,
    pub files: Vec<TemplateFile>,
    pub source: TemplateSource,
}

/// Template manifest (simplified for now)
#[derive(Debug, Clone, Default)]
pub struct TemplateManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub protocol: String,
    pub role: String,
    pub language: String,
    pub files: Vec<ManifestFile>,
    pub variables: HashMap<String, JsonValue>,
}

/// File entry in manifest
#[derive(Debug, Clone)]
pub struct ManifestFile {
    pub source: String,
    pub target: String,
    pub file_type: TemplateFileType,
}

/// Type of template file
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateFileType {
    Template { for_each: Option<String> },
    Static,
    Configuration,
}

/// A single template file - storage agnostic representation
/// This represents a template file with its relative path and content
#[derive(Debug, Clone)]
pub struct TemplateFile {
    /// Path relative to the template directory
    pub path: PathBuf,
    /// Content as a string (templates are text files)
    pub content: String,
    /// The type of this file
    pub file_type: TemplateFileType,
}

/// Metadata describing a template without including file contents
/// Used for listing and displaying templates
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateMetadata {
    /// Path relative to templates/ directory (e.g., "mcp/server/rust")
    pub path: String,
    /// The role this template serves
    pub template_type: TemplateType,
    /// The specific template variant (e.g., "rust", "python")
    pub kind: String,
    /// The protocol this template implements
    pub protocol: String,
    /// Human-readable description
    pub description: Option<String>,
}

// TODO: If these are mapped to roles, then why do we need the type?
/// Categorization of templates by their role
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateType {
    /// Server-side template
    Server,
    /// Client-side template
    Client,
}

impl std::str::FromStr for TemplateType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "server" => Ok(TemplateType::Server),
            "client" => Ok(TemplateType::Client),
            _ => Err(format!("Invalid template type: {}", s)),
        }
    }
}

/// Raw template file as stored in repositories
/// This is what repositories return - raw bytes that can be text or binary
#[derive(Debug, Clone)]
pub struct RawTemplateFile {
    /// Path relative to the template directory
    pub relative_path: String,
    /// The raw contents as bytes (can handle any file type)
    pub contents: Vec<u8>,
}
