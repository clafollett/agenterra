//! Business rules for the generation domain

use crate::generation::{GenerationError, Language};
use crate::protocols::{Protocol, Role};

/// Validates language support for a given protocol and role
pub fn validate_language_support(
    protocol: Protocol,
    role: &Role,
    language: Language,
) -> Result<(), GenerationError> {
    // Define supported languages per protocol/role combination
    match (protocol, role) {
        (Protocol::Mcp, Role::Server) => match language {
            Language::Rust => Ok(()),
            _ => Err(GenerationError::UnsupportedLanguageForProtocol { language, protocol }),
        },
        (Protocol::Mcp, Role::Client) => match language {
            Language::Rust => Ok(()),
            _ => Err(GenerationError::UnsupportedLanguageForProtocol { language, protocol }),
        },
        _ => {
            // For now, only MCP is implemented
            Err(GenerationError::ValidationError(format!(
                "Protocol {} is not yet implemented",
                protocol
            )))
        }
    }
}

/// Determines if OpenAPI is required for a protocol/role combination
pub fn requires_openapi(protocol: Protocol, role: &Role) -> bool {
    matches!((protocol, role), (Protocol::Mcp, Role::Server))
}

/// Gets default metadata values
pub fn default_version() -> String {
    "0.1.0".to_string()
}

/// Validates project name format
pub fn validate_project_name(name: &str) -> Result<(), GenerationError> {
    if name.is_empty() {
        return Err(GenerationError::ValidationError(
            "Project name cannot be empty".to_string(),
        ));
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(GenerationError::ValidationError(
            "Project name must contain only alphanumeric characters, dashes, and underscores"
                .to_string(),
        ));
    }

    if name.starts_with('-') || name.starts_with('_') {
        return Err(GenerationError::ValidationError(
            "Project name cannot start with a dash or underscore".to_string(),
        ));
    }

    Ok(())
}

/// Gets recommended file permissions for different artifact types
pub fn get_artifact_permissions(path: &std::path::Path, language: Language) -> Option<u32> {
    let extension = path.extension()?.to_str()?;

    match (language, extension) {
        // Executable scripts
        (_, "sh") | (_, "bash") => Some(0o755),
        (Language::Python, "py") if path.file_name()?.to_str()?.contains("cli") => Some(0o755),
        // Regular files
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_language_support() {
        // MCP Server supports Rust, Python, TypeScript
        assert!(validate_language_support(Protocol::Mcp, &Role::Server, Language::Rust).is_ok());
        assert!(validate_language_support(Protocol::Mcp, &Role::Server, Language::Python).is_err());
        assert!(
            validate_language_support(Protocol::Mcp, &Role::Server, Language::TypeScript).is_err()
        );
        assert!(validate_language_support(Protocol::Mcp, &Role::Server, Language::Go).is_err());

        // MCP Client supports more languages
        assert!(validate_language_support(Protocol::Mcp, &Role::Client, Language::Go).is_err());
    }

    #[test]
    fn test_requires_openapi() {
        assert!(requires_openapi(Protocol::Mcp, &Role::Server));
        assert!(!requires_openapi(Protocol::Mcp, &Role::Client));
        assert!(!requires_openapi(Protocol::Mcp, &Role::Agent));
    }

    #[test]
    fn test_validate_project_name() {
        assert!(validate_project_name("my-project").is_ok());
        assert!(validate_project_name("my_project").is_ok());
        assert!(validate_project_name("project123").is_ok());

        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("-project").is_err());
        assert!(validate_project_name("_project").is_err());
        assert!(validate_project_name("my project").is_err());
        assert!(validate_project_name("my@project").is_err());
    }
}
