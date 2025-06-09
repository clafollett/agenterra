//! Unified handling of template directory resolution and operations

use std::io;
use std::path::{Path, PathBuf};

use crate::template_kind::TemplateKind;

/// Represents a template directory with resolved paths and validation
#[derive(Debug, Clone)]
pub struct TemplateDir {
    /// Root directory containing the templates
    root_dir: PathBuf,
    /// Path to the specific template directory (root_dir/template_name)
    template_path: PathBuf,
    /// The template kind (language/framework)
    kind: TemplateKind,
}

impl TemplateDir {
    /// Create a new TemplateDir with explicit paths
    pub fn new(root_dir: PathBuf, template_path: PathBuf, kind: TemplateKind) -> Self {
        Self {
            root_dir,
            template_path,
            kind,
        }
    }

    /// Returns the template path as a string slice
    pub fn to_string_lossy(&self) -> std::borrow::Cow<'_, str> {
        self.template_path.to_string_lossy()
    }

    /// Returns a displayable version of the template path
    pub fn display(&self) -> std::path::Display<'_> {
        self.template_path.display()
    }

    /// Discover the template directory based on the template kind and optional override
    pub fn discover(kind: TemplateKind, custom_dir: Option<&Path>) -> io::Result<Self> {
        let root_dir = if let Some(dir) = custom_dir {
            // Use the provided directory directly
            if !dir.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Template directory not found: {}", dir.display()),
                ));
            }
            dir.to_path_buf()
        } else {
            // Auto-discover the template directory
            Self::find_template_base_dir().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Could not find template directory in any standard location",
                )
            })?
        };

        let template_path = root_dir.join(kind.as_str());

        // Validate the template directory exists
        if !template_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Template directory not found: {}", template_path.display()),
            ));
        }

        Ok(Self::new(root_dir, template_path, kind))
    }

    /// Find the base template directory by checking standard locations
    fn find_template_base_dir() -> Option<PathBuf> {
        // 1. Check environment variable
        if let Ok(dir) = std::env::var("MCPGEN_TEMPLATE_DIR") {
            let path = PathBuf::from(dir);
            if path.exists() {
                return Some(path);
            }
        }

        // 2. Check for root templates directory (for development)
        let root_dir = PathBuf::from("..");
        let templates_dir = root_dir.join("templates");
        if templates_dir.exists() {
            return Some(root_dir);
        }

        // 3. Check current directory
        let current_dir = PathBuf::from(".");
        let templates_dir = current_dir.join("templates");
        if templates_dir.exists() {
            return Some(current_dir);
        }

        // 4. Check in the crate root (for development)
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let manifest_path = PathBuf::from(manifest_dir);
            if let Some(workspace_root) = manifest_path.parent() {
                let templates_dir = workspace_root.join("templates");
                if templates_dir.exists() {
                    return Some(workspace_root.to_path_buf());
                }
            }
        }

        // 5. Check in the user's home directory
        if let Some(home_dir) = dirs::home_dir() {
            let templates_dir = home_dir.join(".mcpgen").join("templates");
            if templates_dir.exists() {
                return Some(home_dir.join(".mcpgen"));
            }
        }

        None
    }

    /// Get the root directory containing the templates
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Get the template kind
    pub fn kind(&self) -> TemplateKind {
        self.kind
    }

    /// Get the path to the specific template directory
    pub fn template_path(&self) -> &Path {
        &self.template_path
    }

    /// Convert to PathBuf
    pub fn into_path_buf(self) -> PathBuf {
        self.template_path
    }

    /// Check if the template directory exists
    pub fn exists(&self) -> bool {
        self.template_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_template_dir_validation() {
        let temp_dir = tempdir().unwrap();
        let template_dir = temp_dir.path().join("templates/rust-axum");
        fs::create_dir_all(&template_dir).unwrap();

        // Test with explicit directory
        let template = TemplateDir::discover(
            TemplateKind::RustAxum,
            Some(temp_dir.path().join("templates").as_path()),
        );
        assert!(template.is_ok());
        assert_eq!(template.unwrap().template_path(), template_dir.as_path());

        // Test with non-existent directory
        let result = TemplateDir::discover(TemplateKind::RustAxum, Some(Path::new("/nonexistent")));
        assert!(result.is_err());
    }
}
