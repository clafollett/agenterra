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
    /// Get the string identifier for this language
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::Go => "go",
            Language::Java => "java",
            Language::CSharp => "csharp",
        }
    }

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
        write!(f, "{}", self.as_str())
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
    pub post_commands: Vec<String>,
}

/// Result of generation
#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub artifacts: Vec<Artifact>,
    pub metadata: crate::generation::GenerationMetadata,
}

/// OpenAPI operation representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique string used to identify the operation
    #[serde(rename = "operationId")]
    pub id: String,
    /// The path where this operation is defined (e.g., "/pet/findByStatus")
    pub path: String,
    /// The HTTP method for this operation
    pub method: String,
    /// A list of tags for API documentation control
    pub tags: Option<Vec<String>>,
    /// A short summary of what the operation does
    pub summary: Option<String>,
    /// A verbose explanation of the operation behavior
    pub description: Option<String>,
    /// Additional external documentation for this operation
    #[serde(rename = "externalDocs")]
    pub external_docs: Option<serde_json::Value>,
    /// A list of parameters that are applicable for this operation
    pub parameters: Vec<Parameter>,
    /// The request body applicable for this operation
    pub request_body: Option<RequestBody>,
    /// The list of possible responses
    pub responses: Vec<Response>,
    /// A map of possible out-of band callbacks related to the parent operation
    pub callbacks: Option<serde_json::Value>,
    /// Declares this operation to be deprecated
    pub deprecated: Option<bool>,
    /// A declaration of which security mechanisms can be used for this operation
    pub security: Option<Vec<serde_json::Value>>,
    /// An alternative server array to service this operation
    pub servers: Option<Vec<serde_json::Value>>,
    /// Specification extensions (fields starting with `x-`)
    #[serde(flatten)]
    pub vendor_extensions: std::collections::HashMap<String, serde_json::Value>,
}

/// Operation parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub schema: Schema,
    pub description: Option<String>,
}

/// Parameter location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

/// Request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub required: bool,
    pub content: serde_json::Value,
    pub description: Option<String>,
}

/// Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status_code: String,
    pub description: String,
    pub content: Option<serde_json::Value>,
}

/// Schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub format: Option<String>,
    pub items: Option<Box<Schema>>,
    pub properties: Option<serde_json::Value>,
    pub required: Option<Vec<String>>,
}

/// OpenAPI specification
#[derive(Debug, Clone)]
pub struct OpenApiSpec {
    pub version: String,
    pub info: ApiInfo,
    pub servers: Vec<Server>,
    pub operations: Vec<Operation>,
    pub components: Option<Components>,
}

/// API information
#[derive(Debug, Clone)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

/// Server definition
#[derive(Debug, Clone)]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
}

/// Components section
#[derive(Debug, Clone)]
pub struct Components {
    pub schemas: serde_json::Value,
}
