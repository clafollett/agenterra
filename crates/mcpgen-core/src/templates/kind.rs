//! Template type definitions and discovery for MCPGen.
//!
//! This module defines the supported template types and provides functionality
//! for discovering template directories in the filesystem. It supports both
//! built-in templates and custom template paths.
//!
//! # Examples
//!
//! ```
//! use mcpgen_core::template_kind::TemplateKind;
//! use std::str::FromStr;
//!
//! // Parse a template from a string
//! let template = TemplateKind::from_str("rust_axum").unwrap();
//! assert_eq!(template, TemplateKind::RustAxum);
//! assert_eq!(template.as_str(), "rust_axum");
//!
//! // You can also use the Display trait
//! assert_eq!(template.to_string(), "rust_axum");
//!
//! // The default template is RustAxum
//! assert_eq!(TemplateKind::default(), TemplateKind::RustAxum);
//! ```
//!
//! For template directory discovery, use the `TemplateDir::discover()` method from the
//! `template_dir` module, which handles finding template directories automatically.
//!
//! # Template Discovery
//!
//! The module searches for templates in the following locations:
//! 1. Directory specified by `MCPGEN_TEMPLATE_DIR` environment variable
//! 2. `templates/` directory in the project root (for development)
//! 3. `~/.mcpgen/templates/` in the user's home directory
//! 4. `/usr/local/share/mcpgen/templates/` for system-wide installation
//! 5. `./templates/` in the current working directory

// Internal imports (std, crate)
use std::fmt;
use std::str::FromStr;

/// Supported template kinds (languages/frameworks)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TemplateKind {
    /// Rust with Axum web framework
    #[default]
    RustAxum,
    /// Python with FastAPI
    PythonFastAPI,
    /// TypeScript with Express
    TypeScriptExpress,
    /// Custom template path
    Custom,
}

impl FromStr for TemplateKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rust_axum" => Ok(TemplateKind::RustAxum),
            "python_fastapi" => Ok(TemplateKind::PythonFastAPI),
            "typescript_express" => Ok(TemplateKind::TypeScriptExpress),
            "custom" => Ok(TemplateKind::Custom),
            _ => Err(format!("Unknown template kind: {}", s)),
        }
    }
}

impl TemplateKind {
    /// Returns the template identifier as a string slice
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RustAxum => "rust_axum",
            Self::PythonFastAPI => "python_fastapi",
            Self::TypeScriptExpress => "typescript_express",
            Self::Custom => "custom",
        }
    }

    /// Returns an iterator over all available template kinds
    pub fn all() -> impl Iterator<Item = Self> {
        use TemplateKind::*;
        [RustAxum, PythonFastAPI, TypeScriptExpress, Custom]
            .iter()
            .copied()
    }
}

impl fmt::Display for TemplateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_as_str() {
        assert_eq!(TemplateKind::RustAxum.as_str(), "rust_axum");
        assert_eq!(TemplateKind::PythonFastAPI.as_str(), "python_fastapi");
        assert_eq!(
            TemplateKind::TypeScriptExpress.as_str(),
            "typescript_express"
        );
        assert_eq!(TemplateKind::Custom.as_str(), "custom");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", TemplateKind::RustAxum), "rust_axum");
        assert_eq!(format!("{}", TemplateKind::PythonFastAPI), "python_fastapi");
        assert_eq!(
            format!("{}", TemplateKind::TypeScriptExpress),
            "typescript_express"
        );
        assert_eq!(format!("{}", TemplateKind::Custom), "custom");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "rust_axum".parse::<TemplateKind>().unwrap(),
            TemplateKind::RustAxum
        );
        assert_eq!(
            "python_fastapi".parse::<TemplateKind>().unwrap(),
            TemplateKind::PythonFastAPI
        );
        assert_eq!(
            "typescript_express".parse::<TemplateKind>().unwrap(),
            TemplateKind::TypeScriptExpress
        );
        assert_eq!(
            "custom".parse::<TemplateKind>().unwrap(),
            TemplateKind::Custom
        );

        // Test case insensitivity
        assert_eq!(
            "RUST_AXUM".parse::<TemplateKind>().unwrap(),
            TemplateKind::RustAxum
        );
        assert_eq!(
            "Python_FastAPI".parse::<TemplateKind>().unwrap(),
            TemplateKind::PythonFastAPI
        );

        // Test invalid variants
        assert!("invalid".parse::<TemplateKind>().is_err());
        assert!("".parse::<TemplateKind>().is_err());
    }

    #[test]
    fn test_default() {
        assert_eq!(TemplateKind::default(), TemplateKind::RustAxum);
    }

    #[test]
    fn test_all() {
        let all_kinds: Vec<_> = TemplateKind::all().collect();
        assert_eq!(all_kinds.len(), 4);

        let unique_kinds: HashSet<_> = TemplateKind::all().collect();
        assert_eq!(unique_kinds.len(), 4);

        assert!(unique_kinds.contains(&TemplateKind::RustAxum));
        assert!(unique_kinds.contains(&TemplateKind::PythonFastAPI));
        assert!(unique_kinds.contains(&TemplateKind::TypeScriptExpress));
        assert!(unique_kinds.contains(&TemplateKind::Custom));
    }

    #[test]
    fn test_equality() {
        assert_eq!(TemplateKind::RustAxum, TemplateKind::RustAxum);
        assert_ne!(TemplateKind::RustAxum, TemplateKind::PythonFastAPI);
        assert_ne!(TemplateKind::PythonFastAPI, TemplateKind::TypeScriptExpress);
        assert_ne!(TemplateKind::TypeScriptExpress, TemplateKind::Custom);
    }

    #[test]
    fn test_clone() {
        let kind = TemplateKind::RustAxum;
        let cloned = kind;
        assert_eq!(kind, cloned);

        let boxed = Box::new(kind);
        assert_eq!(*boxed, TemplateKind::RustAxum);
    }
}
