//! Core types for the generation domain

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

/// Supported implementation languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    Go,
    Java,
    CSharp,
}

impl Language {
    /// Get the display name for this language
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::TypeScript => "TypeScript",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::CSharp => "C#",
        }
    }

    /// Get the file extension for this language
    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::Python => "py",
            Language::TypeScript => "ts",
            Language::Go => "go",
            Language::Java => "java",
            Language::CSharp => "cs",
        }
    }

    /// Get all supported languages
    pub fn all() -> Vec<Language> {
        vec![
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::Go,
            Language::Java,
            Language::CSharp,
        ]
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Rust => write!(f, "rust"),
            Language::Python => write!(f, "python"),
            Language::TypeScript => write!(f, "typescript"),
            Language::Go => write!(f, "go"),
            Language::Java => write!(f, "java"),
            Language::CSharp => write!(f, "csharp"),
        }
    }
}

impl FromStr for Language {
    type Err = crate::generation::GenerationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rust" => Ok(Language::Rust),
            "python" | "py" => Ok(Language::Python),
            "typescript" | "ts" => Ok(Language::TypeScript),
            "go" | "golang" => Ok(Language::Go),
            "java" => Ok(Language::Java),
            "csharp" | "c#" | "cs" => Ok(Language::CSharp),
            _ => Err(crate::generation::GenerationError::InvalidLanguage(
                s.to_string(),
            )),
        }
    }
}

/// Generated artifact
#[derive(Debug, Clone)]
pub struct Artifact {
    pub path: PathBuf,
    pub content: String,
    pub permissions: Option<u32>,
}

/// Result of generation
#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub artifacts: Vec<Artifact>,
    pub metadata: crate::generation::GenerationMetadata,
}

// Re-export OpenAPI types from infrastructure module
pub use crate::infrastructure::openapi::{
    ApiInfo, Components, OpenApiContext, Operation, Parameter, ParameterLocation, RequestBody,
    Response, Schema, Server,
};

/// Protocol-specific context data
#[derive(Debug, Clone)]
pub enum ProtocolContext {
    /// MCP Server context with OpenAPI specification
    McpServer {
        /// The full OpenAPI specification
        openapi_spec: OpenApiContext,
        /// Operations extracted from OpenAPI that become MCP endpoints/tools
        endpoints: Vec<Operation>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_language_from_str() {
        // Test exact matches
        assert_eq!(Language::from_str("rust").unwrap(), Language::Rust);
        assert_eq!(Language::from_str("python").unwrap(), Language::Python);
        assert_eq!(
            Language::from_str("typescript").unwrap(),
            Language::TypeScript
        );
        assert_eq!(Language::from_str("go").unwrap(), Language::Go);
        assert_eq!(Language::from_str("java").unwrap(), Language::Java);
        assert_eq!(Language::from_str("csharp").unwrap(), Language::CSharp);

        // Test aliases
        assert_eq!(Language::from_str("py").unwrap(), Language::Python);
        assert_eq!(Language::from_str("ts").unwrap(), Language::TypeScript);
        assert_eq!(Language::from_str("golang").unwrap(), Language::Go);
        assert_eq!(Language::from_str("c#").unwrap(), Language::CSharp);
        assert_eq!(Language::from_str("cs").unwrap(), Language::CSharp);

        // Test case insensitivity
        assert_eq!(Language::from_str("RUST").unwrap(), Language::Rust);
        assert_eq!(Language::from_str("Python").unwrap(), Language::Python);
        assert_eq!(
            Language::from_str("TypeScript").unwrap(),
            Language::TypeScript
        );

        // Test invalid input
        assert!(Language::from_str("javascript").is_err());
        assert!(Language::from_str("ruby").is_err());
        assert!(Language::from_str("").is_err());
    }

    #[test]
    fn test_language_display() {
        assert_eq!(Language::Rust.to_string(), "rust");
        assert_eq!(Language::Python.to_string(), "python");
        assert_eq!(Language::TypeScript.to_string(), "typescript");
        assert_eq!(Language::Go.to_string(), "go");
        assert_eq!(Language::Java.to_string(), "java");
        assert_eq!(Language::CSharp.to_string(), "csharp");
    }

    #[test]
    fn test_language_display_name() {
        assert_eq!(Language::Rust.display_name(), "Rust");
        assert_eq!(Language::Python.display_name(), "Python");
        assert_eq!(Language::TypeScript.display_name(), "TypeScript");
        assert_eq!(Language::Go.display_name(), "Go");
        assert_eq!(Language::Java.display_name(), "Java");
        assert_eq!(Language::CSharp.display_name(), "C#");
    }

    #[test]
    fn test_language_file_extension() {
        assert_eq!(Language::Rust.file_extension(), "rs");
        assert_eq!(Language::Python.file_extension(), "py");
        assert_eq!(Language::TypeScript.file_extension(), "ts");
        assert_eq!(Language::Go.file_extension(), "go");
        assert_eq!(Language::Java.file_extension(), "java");
        assert_eq!(Language::CSharp.file_extension(), "cs");
    }

    #[test]
    fn test_language_all() {
        let all_languages = Language::all();
        assert_eq!(all_languages.len(), 6);
        assert!(all_languages.contains(&Language::Rust));
        assert!(all_languages.contains(&Language::Python));
        assert!(all_languages.contains(&Language::TypeScript));
        assert!(all_languages.contains(&Language::Go));
        assert!(all_languages.contains(&Language::Java));
        assert!(all_languages.contains(&Language::CSharp));
    }

    #[test]
    fn test_language_display_trait() {
        // Test Display trait implementation
        assert_eq!(format!("{}", Language::Rust), "rust");
        assert_eq!(format!("{}", Language::CSharp), "csharp");
    }
}
