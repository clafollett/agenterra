//! Template type definitions and discovery for AgentERRA.
//!
//! This module defines the supported template types and provides functionality
//! for discovering template directories in the filesystem. It supports both
//! built-in templates and custom template paths.
//!
//! # Examples
//!
//! ```
//! use agenterra_mcp::ServerTemplateKind;
//! use std::str::FromStr;
//!
//! // Parse a template from a string
//! let template = ServerTemplateKind::from_str("rust").unwrap();
//! assert_eq!(template, ServerTemplateKind::Rust);
//! assert_eq!(template.as_str(), "rust");
//!
//! // You can also use the Display trait
//! assert_eq!(template.to_string(), "rust");
//!
//! // The default template is Rust
//! assert_eq!(ServerTemplateKind::default(), ServerTemplateKind::Rust);
//! ```
//!
//! For template directory discovery, use the `TemplateDir::discover()` method from the
//! `template_dir` module, which handles finding template directories automatically.
//!
//! # Template Discovery
//!
//! The module searches for templates in the following locations:
//! 1. Directory specified by `AGENTERRA_TEMPLATE_DIR` environment variable
//! 2. `templates/` directory in the project root (for development)
//! 3. `~/.config/agenterra/templates/` in the user's config directory
//! 4. `/usr/local/share/agenterra/templates/` for system-wide installation
//! 5. `./templates/` in the current working directory

// Internal imports (std, crate)
use std::fmt;
use std::str::FromStr;

/// Template role (server or client)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateRole {
    /// Server-side template
    Server,
    /// Client-side template
    #[allow(dead_code)]
    Client,
}

impl TemplateRole {
    /// Returns the role as a string slice
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::Client => "client",
        }
    }
}

impl fmt::Display for TemplateRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Server-side template kinds for MCP server generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ServerTemplateKind {
    /// Rust with Axum web framework
    #[default]
    Rust,
    /// Python with FastAPI
    Python,
    /// TypeScript with Express
    TypeScript,
    /// Custom template path
    Custom,
}

/// Client-side template kinds for client library generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClientTemplateKind {
    /// Rust with reqwest HTTP client
    #[default]
    Rust,
    /// Python with requests library
    Python,
    /// TypeScript with axios library
    TypeScript,
    /// Custom template path
    Custom,
}

impl FromStr for ServerTemplateKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rust" => Ok(ServerTemplateKind::Rust),
            "python" => Ok(ServerTemplateKind::Python),
            "typescript" => Ok(ServerTemplateKind::TypeScript),
            "custom" => Ok(ServerTemplateKind::Custom),
            _ => Err(format!("Unknown server template kind: {s}")),
        }
    }
}

impl FromStr for ClientTemplateKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rust" => Ok(ClientTemplateKind::Rust),
            "python" => Ok(ClientTemplateKind::Python),
            "typescript" => Ok(ClientTemplateKind::TypeScript),
            "custom" => Ok(ClientTemplateKind::Custom),
            _ => Err(format!("Unknown client template kind: {s}")),
        }
    }
}

impl ServerTemplateKind {
    /// Returns the template identifier as a string slice
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::TypeScript => "typescript",
            Self::Custom => "custom",
        }
    }

    /// Returns the template role (always server)
    pub fn role(&self) -> TemplateRole {
        TemplateRole::Server
    }

    /// Returns the language/framework name
    #[allow(dead_code)]
    pub fn framework(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::TypeScript => "typescript",
            Self::Custom => "custom",
        }
    }

    /// Returns an iterator over all available server template kinds
    #[allow(dead_code)]
    pub fn all() -> impl Iterator<Item = Self> {
        use ServerTemplateKind::*;
        [Rust, Python, TypeScript, Custom].iter().copied()
    }
}

impl ClientTemplateKind {
    /// Returns the template identifier as a string slice
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::TypeScript => "typescript",
            Self::Custom => "custom",
        }
    }

    /// Returns the template role (always client)
    #[allow(dead_code)]
    pub fn role(&self) -> TemplateRole {
        TemplateRole::Client
    }

    /// Returns the language/framework name
    #[allow(dead_code)]
    pub fn framework(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::TypeScript => "typescript",
            Self::Custom => "custom",
        }
    }

    /// Returns an iterator over all available client template kinds
    #[allow(dead_code)]
    pub fn all() -> impl Iterator<Item = Self> {
        use ClientTemplateKind::*;
        [Rust, Python, TypeScript, Custom].iter().copied()
    }
}

impl fmt::Display for ServerTemplateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Display for ClientTemplateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // ServerTemplateKind tests
    #[test]
    fn test_server_as_str() {
        assert_eq!(ServerTemplateKind::Rust.as_str(), "rust");
        assert_eq!(ServerTemplateKind::Python.as_str(), "python");
        assert_eq!(ServerTemplateKind::TypeScript.as_str(), "typescript");
        assert_eq!(ServerTemplateKind::Custom.as_str(), "custom");
    }

    #[test]
    fn test_server_display() {
        assert_eq!(format!("{}", ServerTemplateKind::Rust), "rust");
        assert_eq!(format!("{}", ServerTemplateKind::Python), "python");
        assert_eq!(format!("{}", ServerTemplateKind::TypeScript), "typescript");
        assert_eq!(format!("{}", ServerTemplateKind::Custom), "custom");
    }

    #[test]
    fn test_server_from_str() {
        assert_eq!(
            "rust".parse::<ServerTemplateKind>().unwrap(),
            ServerTemplateKind::Rust
        );
        assert_eq!(
            "python".parse::<ServerTemplateKind>().unwrap(),
            ServerTemplateKind::Python
        );
        assert_eq!(
            "typescript".parse::<ServerTemplateKind>().unwrap(),
            ServerTemplateKind::TypeScript
        );
        assert_eq!(
            "custom".parse::<ServerTemplateKind>().unwrap(),
            ServerTemplateKind::Custom
        );

        // Test case insensitivity
        assert_eq!(
            "RUST".parse::<ServerTemplateKind>().unwrap(),
            ServerTemplateKind::Rust
        );

        // Test invalid variants
        assert!("invalid".parse::<ServerTemplateKind>().is_err());
        assert!("RustLike".parse::<ServerTemplateKind>().is_err());
    }

    #[test]
    fn test_server_default() {
        assert_eq!(ServerTemplateKind::default(), ServerTemplateKind::Rust);
    }

    #[test]
    fn test_server_all() {
        let all_kinds: Vec<_> = ServerTemplateKind::all().collect();
        assert_eq!(all_kinds.len(), 4);

        let unique_kinds: HashSet<_> = ServerTemplateKind::all().collect();
        assert_eq!(unique_kinds.len(), 4);

        assert!(unique_kinds.contains(&ServerTemplateKind::Rust));
        assert!(unique_kinds.contains(&ServerTemplateKind::Python));
        assert!(unique_kinds.contains(&ServerTemplateKind::TypeScript));
        assert!(unique_kinds.contains(&ServerTemplateKind::Custom));
    }

    #[test]
    fn test_server_role() {
        assert_eq!(ServerTemplateKind::Rust.role(), TemplateRole::Server);
        assert_eq!(ServerTemplateKind::Python.role(), TemplateRole::Server);
        assert_eq!(ServerTemplateKind::TypeScript.role(), TemplateRole::Server);
        assert_eq!(ServerTemplateKind::Custom.role(), TemplateRole::Server);
    }

    #[test]
    fn test_server_framework() {
        assert_eq!(ServerTemplateKind::Rust.framework(), "rust");
        assert_eq!(ServerTemplateKind::Python.framework(), "python");
        assert_eq!(ServerTemplateKind::TypeScript.framework(), "typescript");
        assert_eq!(ServerTemplateKind::Custom.framework(), "custom");
    }

    // ClientTemplateKind tests
    #[test]
    fn test_client_as_str() {
        assert_eq!(ClientTemplateKind::Rust.as_str(), "rust");
        assert_eq!(ClientTemplateKind::Python.as_str(), "python");
        assert_eq!(ClientTemplateKind::TypeScript.as_str(), "typescript");
        assert_eq!(ClientTemplateKind::Custom.as_str(), "custom");
    }

    #[test]
    fn test_client_display() {
        assert_eq!(format!("{}", ClientTemplateKind::Rust), "rust");
        assert_eq!(format!("{}", ClientTemplateKind::Python), "python");
        assert_eq!(format!("{}", ClientTemplateKind::TypeScript), "typescript");
        assert_eq!(format!("{}", ClientTemplateKind::Custom), "custom");
    }

    #[test]
    fn test_client_from_str() {
        assert_eq!(
            "rust".parse::<ClientTemplateKind>().unwrap(),
            ClientTemplateKind::Rust
        );
        assert_eq!(
            "python".parse::<ClientTemplateKind>().unwrap(),
            ClientTemplateKind::Python
        );
        assert_eq!(
            "typescript".parse::<ClientTemplateKind>().unwrap(),
            ClientTemplateKind::TypeScript
        );
        assert_eq!(
            "custom".parse::<ClientTemplateKind>().unwrap(),
            ClientTemplateKind::Custom
        );

        // Test case insensitivity
        assert_eq!(
            "RUST".parse::<ClientTemplateKind>().unwrap(),
            ClientTemplateKind::Rust
        );

        // Test invalid variants
        assert!("invalid".parse::<ClientTemplateKind>().is_err());
        assert!("RustLike".parse::<ClientTemplateKind>().is_err());
    }

    #[test]
    fn test_client_default() {
        assert_eq!(ClientTemplateKind::default(), ClientTemplateKind::Rust);
    }

    #[test]
    fn test_client_all() {
        let all_kinds: Vec<_> = ClientTemplateKind::all().collect();
        assert_eq!(all_kinds.len(), 4);

        let unique_kinds: HashSet<_> = ClientTemplateKind::all().collect();
        assert_eq!(unique_kinds.len(), 4);

        assert!(unique_kinds.contains(&ClientTemplateKind::Rust));
        assert!(unique_kinds.contains(&ClientTemplateKind::Python));
        assert!(unique_kinds.contains(&ClientTemplateKind::TypeScript));
        assert!(unique_kinds.contains(&ClientTemplateKind::Custom));
    }

    #[test]
    fn test_client_role() {
        assert_eq!(ClientTemplateKind::Rust.role(), TemplateRole::Client);
        assert_eq!(ClientTemplateKind::Python.role(), TemplateRole::Client);
        assert_eq!(ClientTemplateKind::TypeScript.role(), TemplateRole::Client);
        assert_eq!(ClientTemplateKind::Custom.role(), TemplateRole::Client);
    }

    #[test]
    fn test_client_framework() {
        assert_eq!(ClientTemplateKind::Rust.framework(), "rust");
        assert_eq!(ClientTemplateKind::Python.framework(), "python");
        assert_eq!(ClientTemplateKind::TypeScript.framework(), "typescript");
        assert_eq!(ClientTemplateKind::Custom.framework(), "custom");
    }

    // TemplateRole tests
    #[test]
    fn test_template_role_as_str() {
        assert_eq!(TemplateRole::Server.as_str(), "server");
        assert_eq!(TemplateRole::Client.as_str(), "client");
    }

    #[test]
    fn test_template_role_display() {
        assert_eq!(format!("{}", TemplateRole::Server), "server");
        assert_eq!(format!("{}", TemplateRole::Client), "client");
    }

    // Protocol-aware template tests (TDD Red phase)
    #[test]
    fn test_template_kind_with_protocol() {
        use crate::core::protocol::Protocol;

        // Test that template kinds can be combined with protocols
        let server_kind = ServerTemplateKind::Rust;
        let protocol = Protocol::Mcp;

        // This should construct a path like: templates/mcp/server/rust
        let expected_path = format!(
            "templates/{}/{}/{}",
            protocol.path_segment(),
            server_kind.role().as_str(),
            server_kind.as_str()
        );

        assert_eq!(expected_path, "templates/mcp/server/rust");

        // Test client templates too
        let client_kind = ClientTemplateKind::Rust;
        let expected_client_path = format!(
            "templates/{}/{}/{}",
            protocol.path_segment(),
            client_kind.role().as_str(),
            client_kind.as_str()
        );

        assert_eq!(expected_client_path, "templates/mcp/client/rust");
    }

    #[test]
    fn test_template_kind_path_construction() {
        use crate::core::protocol::Protocol;

        // Test that we can build template paths for different protocols and kinds
        let test_cases = vec![
            (
                Protocol::Mcp,
                ServerTemplateKind::Rust,
                "templates/mcp/server/rust",
            ),
            (
                Protocol::Mcp,
                ServerTemplateKind::Python,
                "templates/mcp/server/python",
            ),
            (
                Protocol::Mcp,
                ServerTemplateKind::Custom,
                "templates/mcp/server/custom",
            ),
        ];

        for (protocol, template_kind, expected_path) in test_cases {
            let path = format!(
                "templates/{}/{}/{}",
                protocol.path_segment(),
                template_kind.role().as_str(),
                template_kind.as_str()
            );
            assert_eq!(path, expected_path);
        }
    }
}
