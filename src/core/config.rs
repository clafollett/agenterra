//! Configuration management for Agenterra code generation.
//!
//! This module defines the `Config` struct and related functionality for managing
//! code generation settings. The configuration can be loaded from a YAML file,
//! created programmatically, or loaded from command-line arguments.
//!
//! # Examples
//!
//! ```no_run
//! use agenterra_core::config::Config;
//!
//! // Create a new config programmatically
//! let mut config = Config::new("my-project", "openapi.yaml", "output");
//! config.template_kind = "rust".to_string();
//! config.include_all = true;
//!
//! // Or load from a config file
//! # #[cfg(feature = "yaml")]
//! # {
//! # let config = Config::from_file("agenterra.yaml").unwrap();
//! # }
//! ```

// Internal imports (std, crate)
use std::path::Path;

// External imports (alphabetized)
use crate::core::error::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;
use url::Url;

/// Configuration for Agenterra server generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Project name
    pub project_name: String,

    /// Path to the OpenAPI schema file
    pub openapi_schema_path: String,

    /// Output directory for generated code
    pub output_dir: String,

    /// Template to use for code generation
    #[serde(default = "default_template")]
    pub template_kind: String,

    /// Optional path to template directory
    #[serde(default)]
    pub template_dir: Option<String>,

    /// Whether to include all operations by default
    #[serde(default)]
    pub include_all: bool,

    /// List of operations to include (if include_all is false)
    #[serde(default)]
    pub include_operations: Vec<String>,

    /// List of operations to exclude
    #[serde(default)]
    pub exclude_operations: Vec<String>,

    /// Base URL of the OpenAPI specification (Optional)
    pub base_url: Option<Url>,
}

#[allow(dead_code)]
impl Config {
    /// Create a new Config with default values
    pub fn new(
        project_name: impl Into<String>,
        openapi_schema_path: impl Into<String>,
        output_dir: impl Into<String>,
    ) -> Self {
        Self {
            project_name: project_name.into(),
            openapi_schema_path: openapi_schema_path.into(),
            output_dir: output_dir.into(),
            template_kind: default_template(),
            template_dir: None,
            include_all: false,
            include_operations: Vec::new(),
            exclude_operations: Vec::new(),
            base_url: None,
        }
    }

    /// Load configuration from a file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a file
    pub async fn save<P: AsRef<Path>>(&self, path: P) -> crate::core::error::Result<()> {
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content).await?;
        Ok(())
    }
}

fn default_template() -> String {
    "rust".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_roundtrip() -> crate::core::error::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("config.yaml");

        let config = Config::new("agenterra-server", "openapi.json", "output");
        config.save(&file_path).await?;

        let _loaded = Config::from_file(&file_path).await?;
        assert_eq!(config.project_name, "agenterra-server");
        assert_eq!(config.openapi_schema_path, "openapi.json");
        assert_eq!(config.output_dir, "output");
        assert_eq!(config.template_kind, default_template());
        assert!(!config.include_all);
        assert_eq!(config.include_operations, Vec::<String>::new());
        assert_eq!(config.exclude_operations, Vec::<String>::new());
        assert_eq!(config.base_url, None);

        Ok(())
    }
}
