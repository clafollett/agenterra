//! Embedded template management for binary distribution
//!
//! This module provides access to templates embedded in the binary at compile time,
//! allowing Agenterra to work immediately after `cargo install` without requiring
//! separate template files.

use std::io;
use std::path::{Path, PathBuf};

/// Domain service for accessing embedded templates
pub trait EmbeddedTemplateRepository {
    /// List all available embedded templates
    fn list_templates(&self) -> Vec<EmbeddedTemplate>;
    
    /// Get a specific template by path
    fn get_template(&self, path: &str) -> Option<EmbeddedTemplate>;
    
    /// Check if a template exists
    fn has_template(&self, path: &str) -> bool;
    
    /// Get all files for a template directory
    fn get_template_files(&self, template_path: &str) -> Vec<EmbeddedTemplateFile>;
}

/// Value object representing an embedded template
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedTemplate {
    /// Path relative to templates/ directory
    pub path: String,
    /// Template type (server/client)
    pub template_type: TemplateType,
    /// Template kind (e.g., rust_axum, rust_reqwest)
    pub kind: String,
    /// Protocol (e.g., mcp)
    pub protocol: String,
    /// Template description from manifest
    pub description: Option<String>,
}

/// Template type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateType {
    Server,
    Client,
}

/// Represents a single file within an embedded template
#[derive(Debug, Clone)]
pub struct EmbeddedTemplateFile {
    /// Relative path within the template
    pub relative_path: String,
    /// File contents
    pub contents: Vec<u8>,
}

/// Domain service for exporting templates to filesystem
pub trait TemplateExporter {
    /// Export a single template to the specified directory
    fn export_template(&self, template: &EmbeddedTemplate, output_dir: &Path) -> io::Result<()>;
    
    /// Export all templates to the specified directory
    fn export_all_templates(&self, output_dir: &Path) -> io::Result<usize>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Mock implementation for testing
    struct MockEmbeddedTemplateRepository {
        templates: Vec<EmbeddedTemplate>,
    }
    
    impl MockEmbeddedTemplateRepository {
        fn new() -> Self {
            Self {
                templates: vec![
                    EmbeddedTemplate {
                        path: "mcp/server/rust_axum".to_string(),
                        template_type: TemplateType::Server,
                        kind: "rust_axum".to_string(),
                        protocol: "mcp".to_string(),
                        description: Some("Rust MCP server using Axum framework".to_string()),
                    },
                    EmbeddedTemplate {
                        path: "mcp/client/rust_reqwest".to_string(),
                        template_type: TemplateType::Client,
                        kind: "rust_reqwest".to_string(),
                        protocol: "mcp".to_string(),
                        description: Some("Rust MCP client with REPL interface".to_string()),
                    },
                ],
            }
        }
    }
    
    impl EmbeddedTemplateRepository for MockEmbeddedTemplateRepository {
        fn list_templates(&self) -> Vec<EmbeddedTemplate> {
            self.templates.clone()
        }
        
        fn get_template(&self, path: &str) -> Option<EmbeddedTemplate> {
            self.templates.iter().find(|t| t.path == path).cloned()
        }
        
        fn has_template(&self, path: &str) -> bool {
            self.templates.iter().any(|t| t.path == path)
        }
        
        fn get_template_files(&self, _template_path: &str) -> Vec<EmbeddedTemplateFile> {
            vec![]
        }
    }
    
    #[test]
    fn test_list_embedded_templates() {
        let repo = MockEmbeddedTemplateRepository::new();
        let templates = repo.list_templates();
        
        assert_eq!(templates.len(), 2);
        assert!(templates.iter().any(|t| t.kind == "rust_axum"));
        assert!(templates.iter().any(|t| t.kind == "rust_reqwest"));
    }
    
    #[test]
    fn test_get_template_by_path() {
        let repo = MockEmbeddedTemplateRepository::new();
        
        let template = repo.get_template("mcp/server/rust_axum");
        assert!(template.is_some());
        
        let template = template.unwrap();
        assert_eq!(template.kind, "rust_axum");
        assert_eq!(template.template_type, TemplateType::Server);
    }
    
    #[test]
    fn test_has_template() {
        let repo = MockEmbeddedTemplateRepository::new();
        
        assert!(repo.has_template("mcp/server/rust_axum"));
        assert!(repo.has_template("mcp/client/rust_reqwest"));
        assert!(!repo.has_template("nonexistent/template"));
    }
    
    #[test]
    fn test_template_not_found() {
        let repo = MockEmbeddedTemplateRepository::new();
        
        let template = repo.get_template("invalid/path");
        assert!(template.is_none());
    }
    
    // This test will fail until we implement the real repository
    #[test]
    #[ignore] // Remove this when implementing the real repository
    fn test_real_embedded_templates_available() {
        // This test will verify that actual templates are embedded
        // It will fail in RED phase and pass once we implement rust-embed
        panic!("Real embedded template repository not yet implemented");
    }
    
    // Tests for template export functionality
    mod export_tests {
        use super::*;
        use std::fs;
        use tempfile::TempDir;
        
        struct MockTemplateExporter;
        
        impl TemplateExporter for MockTemplateExporter {
            fn export_template(&self, template: &EmbeddedTemplate, output_dir: &Path) -> io::Result<()> {
                let template_dir = output_dir.join(&template.path);
                fs::create_dir_all(&template_dir)?;
                
                // Create a dummy file to verify export worked
                let dummy_file = template_dir.join("exported.txt");
                fs::write(dummy_file, b"exported template")?;
                
                Ok(())
            }
            
            fn export_all_templates(&self, output_dir: &Path) -> io::Result<usize> {
                let repo = MockEmbeddedTemplateRepository::new();
                let templates = repo.list_templates();
                
                for template in &templates {
                    self.export_template(template, output_dir)?;
                }
                
                Ok(templates.len())
            }
        }
        
        #[test]
        fn test_export_single_template() {
            let temp_dir = TempDir::new().unwrap();
            let exporter = MockTemplateExporter;
            
            let template = EmbeddedTemplate {
                path: "mcp/server/rust_axum".to_string(),
                template_type: TemplateType::Server,
                kind: "rust_axum".to_string(),
                protocol: "mcp".to_string(),
                description: Some("Test template".to_string()),
            };
            
            let result = exporter.export_template(&template, temp_dir.path());
            assert!(result.is_ok());
            
            // Verify the template was exported
            let exported_path = temp_dir.path().join("mcp/server/rust_axum/exported.txt");
            assert!(exported_path.exists());
        }
        
        #[test]
        fn test_export_all_templates() {
            let temp_dir = TempDir::new().unwrap();
            let exporter = MockTemplateExporter;
            
            let result = exporter.export_all_templates(temp_dir.path());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);
            
            // Verify both templates were exported
            assert!(temp_dir.path().join("mcp/server/rust_axum/exported.txt").exists());
            assert!(temp_dir.path().join("mcp/client/rust_reqwest/exported.txt").exists());
        }
        
        #[test]
        #[ignore] // Remove when implementing real exporter
        fn test_real_template_export_preserves_structure() {
            // This test will verify that the real exporter preserves
            // the full template directory structure
            panic!("Real template exporter not yet implemented");
        }
    }
}