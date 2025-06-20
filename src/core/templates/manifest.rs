//! Manifest file format for Agenterra templates.
//!
//! This module defines the structure of the `template.yaml` file that describes
//! how to generate code from templates.

use serde::{Deserialize, Deserializer, Serialize};
use serde_value::Value as SerdeValue;
use tokio::fs;
use tracing::debug;

/// The root manifest structure for a template.
///
/// This describes the template's metadata and the files it contains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    /// The name of the template
    pub name: String,

    /// A short description of what the template generates
    pub description: String,

    /// The version of the template (should follow semantic versioning)
    pub version: String,

    /// The target programming language (e.g., "rust", "typescript")
    pub language: String,

    /// List of files to generate
    pub files: Vec<TemplateFile>,

    /// Optional hooks that run before/after generation
    #[serde(default)]
    pub hooks: TemplateHooks,
}

/// Describes a single file to be generated from a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
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
pub struct TemplateHooks {
    /// Commands to run before code generation
    #[serde(default, deserialize_with = "deserialize_commands")]
    pub pre_generate: Vec<String>,

    /// Commands to run after code generation
    #[serde(default, deserialize_with = "deserialize_commands")]
    pub post_generate: Vec<String>,
}

impl Default for TemplateManifest {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            description: String::from("Default template"),
            version: env!("CARGO_PKG_VERSION").to_string(),
            language: String::from("rust"),
            files: Vec::new(),
            hooks: TemplateHooks::default(),
        }
    }
}

impl Default for TemplateFile {
    fn default() -> Self {
        Self {
            source: String::new(),
            destination: String::new(),
            for_each: None,
            context: serde_json::Value::Null,
        }
    }
}

impl TemplateManifest {
    /// Load a template manifest from a directory.
    ///
    /// Looks for a `manifest.yml` file in the specified directory and parses it.
    ///
    /// # Errors
    ///
    /// Returns an error if the file doesn't exist, can't be read, or contains invalid YAML.
    pub async fn load_from_dir(
        template_dir: &std::path::Path,
    ) -> Result<Self, crate::core::error::Error> {
        let manifest_path = template_dir.join("manifest.yml");

        debug!(
            manifest_path = %manifest_path.display(),
            "Attempting to read template manifest"
        );
        // Read the file content and log it for debugging
        let content = fs::read_to_string(&manifest_path).await.map_err(|e| {
            crate::core::error::Error::Template(format!(
                "Failed to read template manifest at full path {}: {}",
                manifest_path.display(),
                e
            ))
        })?;

        // Log the content for debugging
        debug!(
            content_length = content.len(),
            "Successfully read template manifest content"
        );

        // Try to parse the YAML content
        let manifest: Self = serde_yaml::from_str(&content).map_err(|e| {
            crate::core::error::Error::Template(format!(
                "Invalid YAML in template manifest at {}: {}\nContent:\n{}",
                manifest_path.display(),
                e,
                content
            ))
        })?;

        Ok(manifest)
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
    use serde_json::json;
    use tempfile::tempdir;
    use tokio::fs;

    #[test]
    fn test_template_manifest_default() {
        let manifest = TemplateManifest::default();
        assert_eq!(manifest.name, "default");
        assert_eq!(manifest.description, "Default template");
        assert_eq!(manifest.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(manifest.language, "rust");
        assert!(manifest.files.is_empty());
        assert!(manifest.hooks.pre_generate.is_empty());
        assert!(manifest.hooks.post_generate.is_empty());
    }

    #[test]
    fn test_template_file_default() {
        let file = TemplateFile::default();
        assert!(file.source.is_empty());
        assert!(file.destination.is_empty());
        assert!(file.for_each.is_none());
        assert_eq!(file.context, serde_json::Value::Null);
    }

    #[test]
    fn test_template_hooks_default() {
        let hooks = TemplateHooks::default();
        assert!(hooks.pre_generate.is_empty());
        assert!(hooks.post_generate.is_empty());
    }

    #[tokio::test]
    async fn test_load_manifest_from_valid_yaml() {
        let temp_dir = tempdir().unwrap();
        let manifest_content = r#"
name: "test_template"
description: "A test template"
version: "1.0.0"
language: "rust"

files:
  - source: "main.rs.tera"
    destination: "src/main.rs"
  - source: "lib.rs.tera"
    destination: "src/lib.rs"
    for_each: "operation"
    context:
      custom_key: "custom_value"

hooks:
  pre_generate:
    - "echo 'Starting generation'"
  post_generate:
    - "cargo fmt"
    - "cargo check"
"#;

        let manifest_path = temp_dir.path().join("manifest.yml");
        fs::write(&manifest_path, manifest_content).await.unwrap();

        let manifest = TemplateManifest::load_from_dir(temp_dir.path())
            .await
            .unwrap();

        assert_eq!(manifest.name, "test_template");
        assert_eq!(manifest.description, "A test template");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.language, "rust");
        assert_eq!(manifest.files.len(), 2);

        let first_file = &manifest.files[0];
        assert_eq!(first_file.source, "main.rs.tera");
        assert_eq!(first_file.destination, "src/main.rs");
        assert!(first_file.for_each.is_none());

        let second_file = &manifest.files[1];
        assert_eq!(second_file.source, "lib.rs.tera");
        assert_eq!(second_file.destination, "src/lib.rs");
        assert_eq!(second_file.for_each.as_ref().unwrap(), "operation");
        assert_eq!(second_file.context["custom_key"], "custom_value");

        assert_eq!(manifest.hooks.pre_generate.len(), 1);
        assert_eq!(manifest.hooks.pre_generate[0], "echo 'Starting generation'");
        assert_eq!(manifest.hooks.post_generate.len(), 2);
        assert_eq!(manifest.hooks.post_generate[0], "cargo fmt");
        assert_eq!(manifest.hooks.post_generate[1], "cargo check");
    }

    #[tokio::test]
    async fn test_load_manifest_missing_file() {
        let temp_dir = tempdir().unwrap();
        let result = TemplateManifest::load_from_dir(temp_dir.path()).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Failed to read template manifest")
        );
    }

    #[tokio::test]
    async fn test_load_manifest_invalid_yaml() {
        let temp_dir = tempdir().unwrap();
        let invalid_yaml = r#"
name: "test"
description: "test"
invalid_yaml: [
"#;

        let manifest_path = temp_dir.path().join("manifest.yml");
        fs::write(&manifest_path, invalid_yaml).await.unwrap();

        let result = TemplateManifest::load_from_dir(temp_dir.path()).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid YAML"));
    }

    #[test]
    fn test_deserialize_single_command() {
        let yaml = r#"
pre_generate: "single command"
post_generate: []
"#;
        let hooks: TemplateHooks = serde_yaml::from_str(yaml).unwrap();
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
        let hooks: TemplateHooks = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(hooks.pre_generate.len(), 0);
        assert_eq!(hooks.post_generate.len(), 3);
        assert_eq!(hooks.post_generate[0], "command 1");
        assert_eq!(hooks.post_generate[1], "command 2");
        assert_eq!(hooks.post_generate[2], "command 3");
    }

    #[test]
    fn test_serialize_manifest() {
        let manifest = TemplateManifest {
            name: "test".to_string(),
            description: "Test template".to_string(),
            version: "1.0.0".to_string(),
            language: "rust".to_string(),
            files: vec![TemplateFile {
                source: "main.rs.tera".to_string(),
                destination: "src/main.rs".to_string(),
                for_each: None,
                context: json!({"key": "value"}),
            }],
            hooks: TemplateHooks {
                pre_generate: vec!["pre command".to_string()],
                post_generate: vec!["post command".to_string()],
            },
        };

        let yaml = serde_yaml::to_string(&manifest).unwrap();
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("description: Test template"));
        assert!(yaml.contains("version: 1.0.0"));
        assert!(yaml.contains("language: rust"));
    }

    #[test]
    fn test_manifest_clone() {
        let original = TemplateManifest {
            name: "original".to_string(),
            description: "Original template".to_string(),
            version: "1.0.0".to_string(),
            language: "rust".to_string(),
            files: vec![TemplateFile::default()],
            hooks: TemplateHooks::default(),
        };

        let cloned = original.clone();
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.description, cloned.description);
        assert_eq!(original.version, cloned.version);
        assert_eq!(original.language, cloned.language);
        assert_eq!(original.files.len(), cloned.files.len());
    }
}
