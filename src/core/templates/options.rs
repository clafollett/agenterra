//! Configuration options for template-based code generation in AgentERRA.
//!
//! This module provides the [`TemplateOptions`] struct which controls how code is generated
//! from templates. It allows fine-grained control over:
//!
//! - Operation filtering (include/exclude specific operations)
//! - Test generation
//! - File overwrite behavior
//! - Custom template context injection
//! - Server configuration (port, logging)
//!
//! # Example
//!
//! ```rust
//! use agenterra_mcp::TemplateOptions;
//!
//! let options = TemplateOptions {
//!     all_operations: true,
//!     include_tests: true,
//!     overwrite: false,
//!     server_port: Some(8080),
//!     ..Default::default()
//! };
//! ```
//!
// Re-exports (alphabetized)
pub use serde_json::Value as JsonValue;

/// Configuration struct for controlling template-based code generation.
///
/// Provides options to customize which operations are included, whether to generate tests,
/// file overwrite behavior, and additional template context.
#[derive(Debug, Default, Clone)]
pub struct TemplateOptions {
    /// Whether to include all operations by default
    pub all_operations: bool,

    /// Whether to generate tests
    #[allow(dead_code)]
    pub include_tests: bool,

    /// Whether to overwrite existing files
    #[allow(dead_code)]
    pub overwrite: bool,

    /// Additional context to pass to templates
    pub agent_instructions: Option<JsonValue>,

    /// Specific operations to include (overrides all_operations if not empty)
    pub include_operations: Vec<String>,

    /// Operations to exclude
    pub exclude_operations: Vec<String>,

    /// Server port for the generated application
    pub server_port: Option<u16>,

    /// Log file path for the generated application
    pub log_file: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_template_options_default() {
        let options = TemplateOptions::default();
        assert!(!options.all_operations);
        assert!(!options.include_tests);
        assert!(!options.overwrite);
        assert!(options.agent_instructions.is_none());
        assert!(options.include_operations.is_empty());
        assert!(options.exclude_operations.is_empty());
        assert!(options.server_port.is_none());
        assert!(options.log_file.is_none());
    }

    #[test]
    fn test_template_options_custom() {
        let options = TemplateOptions {
            all_operations: true,
            include_tests: true,
            overwrite: false,
            agent_instructions: Some(json!({"custom": "value"})),
            include_operations: vec!["get_users".to_string(), "create_user".to_string()],
            exclude_operations: vec!["delete_user".to_string()],
            server_port: Some(8080),
            log_file: Some("app.log".to_string()),
        };

        assert!(options.all_operations);
        assert!(options.include_tests);
        assert!(!options.overwrite);
        assert!(options.agent_instructions.is_some());
        assert_eq!(options.include_operations.len(), 2);
        assert_eq!(options.exclude_operations.len(), 1);
        assert_eq!(options.server_port, Some(8080));
        assert_eq!(options.log_file, Some("app.log".to_string()));
    }

    #[test]
    fn test_template_options_clone() {
        let original = TemplateOptions {
            all_operations: true,
            include_tests: true,
            overwrite: true,
            agent_instructions: Some(json!({"test": "data"})),
            include_operations: vec!["operation1".to_string()],
            exclude_operations: vec!["operation2".to_string()],
            server_port: Some(3000),
            log_file: Some("test.log".to_string()),
        };

        let cloned = original.clone();
        assert_eq!(original.all_operations, cloned.all_operations);
        assert_eq!(original.include_tests, cloned.include_tests);
        assert_eq!(original.overwrite, cloned.overwrite);
        assert_eq!(original.server_port, cloned.server_port);
        assert_eq!(original.log_file, cloned.log_file);
        assert_eq!(original.include_operations, cloned.include_operations);
        assert_eq!(original.exclude_operations, cloned.exclude_operations);
    }

    #[test]
    fn test_template_options_debug() {
        let options = TemplateOptions {
            all_operations: true,
            server_port: Some(8080),
            ..Default::default()
        };

        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("all_operations: true"));
        assert!(debug_str.contains("server_port: Some(8080)"));
    }

    #[test]
    fn test_agent_instructions_json_value() {
        // Test with object
        let options = TemplateOptions {
            agent_instructions: Some(json!({
                "key1": "value1",
                "key2": 42,
                "nested": {
                    "array": [1, 2, 3]
                }
            })),
            ..Default::default()
        };

        assert!(options.agent_instructions.is_some());
        let instructions = options.agent_instructions.unwrap();
        assert_eq!(instructions["key1"], "value1");
        assert_eq!(instructions["key2"], 42);
        assert_eq!(instructions["nested"]["array"][0], 1);
    }

    #[test]
    fn test_operations_filtering() {
        let options = TemplateOptions {
            include_operations: vec![
                "get_users".to_string(),
                "create_user".to_string(),
                "update_user".to_string(),
            ],
            exclude_operations: vec!["delete_user".to_string(), "admin_operations".to_string()],
            ..Default::default()
        };

        assert_eq!(options.include_operations.len(), 3);
        assert_eq!(options.exclude_operations.len(), 2);
        assert!(
            options
                .include_operations
                .contains(&"get_users".to_string())
        );
        assert!(
            options
                .exclude_operations
                .contains(&"delete_user".to_string())
        );
    }
}
