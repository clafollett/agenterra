//! Common manifest parsing for all template sources
//!
//! This module provides the shared manifest structure and parsing logic
//! used by both embedded and filesystem template repositories.

use crate::infrastructure::templates::{
    ManifestFile, TemplateError, TemplateFileType, TemplateManifest,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_value::Value as SerdeValue;
use std::collections::HashMap;

/// Internal representation matching the manifest YAML structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ManifestData {
    /// The name of the template
    pub name: String,
    /// A short description of what the template generates
    pub description: String,
    /// The version of the template (should follow semantic versioning)
    pub version: String,
    /// The target protocol (e.g., "mcp")
    pub protocol: String,
    /// The role within the protocol (e.g., "server", "client")
    pub role: String,
    /// The target programming language (e.g., "rust", "typescript")
    pub language: String,
    /// List of files to generate
    pub files: Vec<ManifestFileData>,
    /// Optional hooks that run before/after generation
    #[serde(default)]
    pub hooks: ManifestHooks,
    /// Template variables (added for compatibility with our domain model)
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ManifestFileData {
    /// Path to the template file, relative to the template directory
    pub source: String,
    /// Destination path for the generated file, relative to the output directory
    pub destination: String,
    /// Optional directive for generating multiple files (e.g., "operation")
    #[serde(default)]
    pub for_each: Option<String>,
    /// Additional context to pass to the template
    #[serde(default)]
    pub context: serde_json::Value,
}

/// Hooks that run at specific points during code generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ManifestHooks {
    /// Commands to run before code generation
    #[serde(default, deserialize_with = "deserialize_commands")]
    pub pre_generate: Vec<String>,
    /// Commands to run after code generation
    #[serde(default, deserialize_with = "deserialize_commands")]
    pub post_generate: Vec<String>,
}

impl ManifestData {
    /// Convert the raw manifest data into the domain model
    pub fn into_domain_model(self) -> TemplateManifest {
        let mut variables = self.variables;

        // Add protocol/role/language to variables for template access
        variables.insert(
            "protocol".to_string(),
            serde_json::Value::String(self.protocol.clone()),
        );
        variables.insert(
            "role".to_string(),
            serde_json::Value::String(self.role.clone()),
        );
        variables.insert(
            "language".to_string(),
            serde_json::Value::String(self.language.clone()),
        );

        TemplateManifest {
            name: self.name,
            version: self.version,
            description: Some(self.description),
            protocol: self.protocol,
            role: self.role,
            language: self.language,
            files: self
                .files
                .into_iter()
                .map(|f| {
                    let file_type = determine_file_type(&f.source, f.for_each);

                    ManifestFile {
                        source: f.source,
                        target: f.destination,
                        file_type,
                    }
                })
                .collect(),
            variables,
        }
    }
}

/// Parse manifest YAML content into the domain model
pub fn parse_manifest_yaml(content: &str) -> Result<TemplateManifest, TemplateError> {
    let manifest_data: ManifestData = serde_yaml::from_str(content).map_err(|e| {
        TemplateError::InvalidManifest(format!("Failed to parse manifest YAML: {}", e))
    })?;

    Ok(manifest_data.into_domain_model())
}

/// Determine the file type based on extension and for_each directive
fn determine_file_type(source: &str, for_each: Option<String>) -> TemplateFileType {
    if source.ends_with(".tera") {
        if let Some(for_each) = for_each {
            TemplateFileType::Template {
                for_each: Some(for_each),
            }
        } else {
            TemplateFileType::Template { for_each: None }
        }
    } else {
        TemplateFileType::Static
    }
}

/// Helper function to deserialize either a single command or a list of commands
fn deserialize_commands<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    // Try to deserialize as a single string or a vector of strings
    let value = SerdeValue::deserialize(deserializer)?;

    match value {
        SerdeValue::String(s) => Ok(vec![s.to_owned()]),
        SerdeValue::Seq(seq) => {
            let mut result = Vec::new();
            for item in seq {
                if let SerdeValue::String(s) = item {
                    result.push(s.to_owned());
                } else {
                    return Err(serde::de::Error::custom(
                        "Expected string or array of strings",
                    ));
                }
            }
            Ok(result)
        }
        _ => Err(serde::de::Error::custom(
            "Expected string or array of strings",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest_yaml() {
        let yaml = r#"
name: "test-template"
version: "1.0.0"
description: "A test template"
protocol: "mcp"
role: "server"
language: "rust"

files:
  - source: "main.rs.tera"
    destination: "src/main.rs"
  - source: "lib.rs.tera"
    destination: "src/lib.rs"
    for_each: "operation"
    context:
      custom_key: "custom_value"
  - source: "README.md"
    destination: "README.md"

variables:
  default_port: 3000
  enable_logging: true

hooks:
  pre_generate:
    - "echo 'Starting generation'"
  post_generate:
    - "cargo fmt"
    - "cargo check"
"#;

        let manifest = parse_manifest_yaml(yaml).unwrap();

        assert_eq!(manifest.name, "test-template");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.description, Some("A test template".to_string()));
        assert_eq!(manifest.protocol, "mcp");
        assert_eq!(manifest.role, "server");
        assert_eq!(manifest.language, "rust");
        assert_eq!(manifest.files.len(), 3);

        // Check file types
        let main_file = &manifest.files[0];
        assert!(matches!(
            main_file.file_type,
            TemplateFileType::Template { for_each: None }
        ));

        let lib_file = &manifest.files[1];
        assert!(matches!(
            lib_file.file_type,
            TemplateFileType::Template { for_each: Some(ref s) } if s == "operation"
        ));

        let readme_file = &manifest.files[2];
        assert!(matches!(readme_file.file_type, TemplateFileType::Static));

        // Check variables
        assert_eq!(manifest.variables["default_port"], 3000);
        assert_eq!(manifest.variables["enable_logging"], true);
    }

    #[test]
    fn test_determine_file_type() {
        assert!(matches!(
            determine_file_type("template.rs.tera", None),
            TemplateFileType::Template { for_each: None }
        ));

        assert!(matches!(
            determine_file_type("template.rs.tera", Some("operation".to_string())),
            TemplateFileType::Template { for_each: Some(ref s) } if s == "operation"
        ));

        assert!(matches!(
            determine_file_type("README.md", None),
            TemplateFileType::Static
        ));
    }

    #[test]
    fn test_deserialize_single_command() {
        let yaml = r#"
pre_generate: "single command"
post_generate: []
"#;
        let hooks: ManifestHooks = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(hooks.pre_generate.len(), 1);
        assert_eq!(hooks.pre_generate[0], "single command");
        assert_eq!(hooks.post_generate.len(), 0);
    }

    #[test]
    fn test_deserialize_multiple_commands() {
        let yaml = r#"
pre_generate: []
post_generate:
  - "command 1"
  - "command 2"
  - "command 3"
"#;
        let hooks: ManifestHooks = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(hooks.pre_generate.len(), 0);
        assert_eq!(hooks.post_generate.len(), 3);
        assert_eq!(hooks.post_generate[0], "command 1");
        assert_eq!(hooks.post_generate[1], "command 2");
        assert_eq!(hooks.post_generate[2], "command 3");
    }
}
