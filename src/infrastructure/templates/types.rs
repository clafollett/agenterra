//! Core template types for the infrastructure layer
//!
//! These types are storage-agnostic and can be used by any template repository
//! implementation (embedded, filesystem, remote, etc.)

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::{
    generation::Language,
    protocols::{Protocol, Role},
};

use super::TemplateError;

/// Source of a template
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
    Embedded,
    FileSystem(PathBuf),
}

impl std::fmt::Display for TemplateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateSource::Embedded => write!(f, "Embedded"),
            TemplateSource::FileSystem(path) => write!(f, "FileSystem({})", path.display()),
        }
    }
}

/// A complete template with all its files
#[derive(Debug, Clone)]
pub struct Template {
    pub manifest: TemplateManifest,
    pub files: Vec<TemplateFile>,
    pub source: TemplateSource,
}

/// Template manifest - represents the manifest.yml file with strongly typed fields
#[derive(Debug, Clone)]
pub struct TemplateManifest {
    // Basic metadata
    pub name: String,
    pub version: String,
    pub description: Option<String>,

    // Identity with proper types
    pub path: String, // e.g. "mcp/server/rust"
    pub protocol: Protocol,
    pub role: Role,
    pub language: Language,

    // Template details
    pub files: Vec<ManifestFile>,
    pub variables: HashMap<String, JsonValue>,
    pub post_generate_hooks: Vec<String>,
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

/// Raw template file as stored in repositories
/// This is what repositories return - raw bytes that can be text or binary
#[derive(Debug, Clone)]
pub struct RawTemplateFile {
    /// Path relative to the template directory
    pub relative_path: String,
    /// The raw contents as bytes (can handle any file type)
    pub contents: Vec<u8>,
}

impl TemplateManifest {
    /// Parse a manifest.yml content into a strongly-typed TemplateManifest
    pub fn from_yaml(content: &str, path: &str) -> Result<Self, TemplateError> {
        use std::str::FromStr;

        // Parse to serde_yaml::Value first for explicit field extraction
        let yaml: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| TemplateError::manifest_parse_error(path, e))?;

        // Extract basic metadata
        let name = yaml
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TemplateError::manifest_parse_error(path, "missing 'name' field"))?
            .to_string();

        let version = yaml
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TemplateError::manifest_parse_error(path, "missing 'version' field"))?
            .to_string();

        let description = yaml
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Convert string fields to enums
        let protocol_str = yaml
            .get("protocol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TemplateError::manifest_parse_error(path, "missing 'protocol' field"))?;
        let protocol = Protocol::from_str(protocol_str)
            .map_err(|e| TemplateError::manifest_parse_error(path, e))?;

        let role_str = yaml
            .get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TemplateError::manifest_parse_error(path, "missing 'role' field"))?;
        let role =
            Role::from_str(role_str).map_err(|e| TemplateError::manifest_parse_error(path, e))?;

        let language_str = yaml
            .get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TemplateError::manifest_parse_error(path, "missing 'language' field"))?;
        let language = Language::from_str(language_str)
            .map_err(|e| TemplateError::manifest_parse_error(path, e))?;

        // Parse files array
        let files = if let Some(files_yaml) = yaml.get("files") {
            parse_manifest_files(files_yaml, path)?
        } else {
            Vec::new()
        };

        // Parse variables (default to empty if not present)
        let variables = if let Some(vars_yaml) = yaml.get("variables") {
            serde_yaml::from_value(vars_yaml.clone()).map_err(|e| {
                TemplateError::manifest_parse_error(path, format!("invalid variables: {e}"))
            })?
        } else {
            HashMap::new()
        };

        // Parse hooks
        let post_generate_hooks = parse_hooks(&yaml, "hooks", "post_generate")
            .or_else(|_| parse_hooks(&yaml, "post_generate_hooks", ""))
            .unwrap_or_default();

        Ok(TemplateManifest {
            name,
            version,
            description,
            path: path.to_string(),
            protocol,
            role,
            language,
            files,
            variables,
            post_generate_hooks,
        })
    }
}

// Helper function to parse manifest files
fn parse_manifest_files(
    files_yaml: &serde_yaml::Value,
    manifest_path: &str,
) -> Result<Vec<ManifestFile>, TemplateError> {
    let files_array = files_yaml.as_sequence().ok_or_else(|| {
        TemplateError::manifest_parse_error(manifest_path, "'files' must be an array")
    })?;

    let mut files = Vec::new();
    for file_yaml in files_array {
        let source = file_yaml
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                TemplateError::manifest_parse_error(manifest_path, "file entry missing 'source'")
            })?
            .to_string();

        let target = file_yaml
            .get("destination")
            .or_else(|| file_yaml.get("target")) // Support both for compatibility
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                TemplateError::manifest_parse_error(
                    manifest_path,
                    "file entry missing 'destination' or 'target'",
                )
            })?
            .to_string();

        let for_each = file_yaml
            .get("for_each")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let file_type = if source.ends_with(".tera") {
            TemplateFileType::Template { for_each }
        } else if is_configuration_file(&source) {
            TemplateFileType::Configuration
        } else {
            TemplateFileType::Static
        };

        files.push(ManifestFile {
            source,
            target,
            file_type,
        });
    }

    Ok(files)
}

// Helper function to parse hooks
fn parse_hooks(
    yaml: &serde_yaml::Value,
    parent_key: &str,
    child_key: &str,
) -> Result<Vec<String>, TemplateError> {
    let hooks_value = if child_key.is_empty() {
        yaml.get(parent_key)
    } else {
        yaml.get(parent_key).and_then(|p| p.get(child_key))
    };

    match hooks_value {
        Some(serde_yaml::Value::String(s)) => Ok(vec![s.clone()]),
        Some(serde_yaml::Value::Sequence(seq)) => seq
            .iter()
            .map(|v| {
                v.as_str().map(|s| s.to_string()).ok_or_else(|| {
                    TemplateError::InvalidManifest("hook must be a string".to_string())
                })
            })
            .collect(),
        _ => Ok(Vec::new()),
    }
}

// Check if a file is a configuration file based on its name/extension
fn is_configuration_file(source: &str) -> bool {
    source.ends_with(".json")
        || source.ends_with(".yaml")
        || source.ends_with(".yml")
        || source.ends_with(".toml")
        || source.ends_with(".xml")
        || source.ends_with(".properties")
        || source.ends_with(".ini")
        || source.ends_with(".conf")
        || source.ends_with(".config")
        || source == "Cargo.toml"
        || source == "package.json"
        || source == "pyproject.toml"
        || source == "tsconfig.json"
        || source == ".env"
        || source == ".gitignore"
}
