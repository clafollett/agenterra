/// Domain definitions for accessing templates
pub trait TemplateRepository {
    /// List all available templates
    fn list_templates(&self) -> Vec<Template>;

    /// Get a specific template by path
    fn get_template(&self, path: &str) -> Option<Template>;

    /// Check if a template exists
    fn has_template(&self, path: &str) -> bool;

    /// Get all files for a template directory
    fn get_template_files(&self, template_path: &str) -> Vec<TemplateFile>;
}

/// Value object representing an embedded template
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
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
pub struct TemplateFile {
    /// Relative path within the template
    pub relative_path: String,
    /// File contents
    pub contents: Vec<u8>,
}

/// Domain service for exporting templates to filesystem
#[allow(dead_code)]
pub trait TemplateExporter {
    /// Export a single template to the specified directory
    fn export_template(
        &self,
        template: &Template,
        output_dir: &std::path::Path,
    ) -> std::io::Result<()>;

    /// Export all templates to the specified directory
    fn export_all_templates(&self, output_dir: &std::path::Path) -> std::io::Result<usize>;
}
