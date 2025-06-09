//! End-to-end integration tests for MCPGen CLI

use anyhow::{Context, Result, bail};
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// Test configuration
const SCAFFOLD_DIR: &str = ".mcpgen";

/// Test context containing paths and configuration
struct TestContext {
    templates_dir: PathBuf,
    output_dir: PathBuf,
    workspace_root: PathBuf,
}

impl TestContext {
    /// Create a new test context
    fn new() -> Result<Self> {
        let project_root = project_root()?;
        let templates_dir = project_root.join("templates");
        let output_dir = project_root.join(SCAFFOLD_DIR);
        let workspace_root = project_root.clone();

        // Ensure output directory exists
        std::fs::create_dir_all(&output_dir)?;

        Ok(Self {
            templates_dir,
            output_dir,
            workspace_root,
        })
    }

    /// Get a list of all template names in the templates directory
    fn list_templates(&self) -> Result<Vec<String>> {
        let mut templates = Vec::new();

        for entry in std::fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(std::ffi::OsStr::to_str) {
                    // Skip hidden directories
                    if !name.starts_with('.') {
                        templates.push(name.to_string());
                    }
                }
            }
        }

        Ok(templates)
    }

    /// Get the path to a specific template
    fn template_path(&self, template_name: &str) -> PathBuf {
        self.templates_dir.join(template_name)
    }

    /// Get the output path for a specific template and spec
    fn output_path(&self, template_name: &str, spec_name: &str) -> PathBuf {
        let spec_stem = if spec_name.starts_with("http://") || spec_name.starts_with("https://") {
            // For URLs, extract a meaningful name from the path
            let url_path = spec_name
                .trim_start_matches("http://")
                .trim_start_matches("https://");

            // Get the last path component or domain if no path
            let parts: Vec<&str> = url_path.split('/').collect();
            let name_part = if parts.len() > 1 && !parts.last().unwrap().is_empty() {
                // Use the last path component (e.g., "openapi.json" from ".../api/v3/openapi.json")
                parts.last().unwrap()
            } else {
                // Use the domain name if no meaningful path
                parts[0].split('.').next().unwrap_or("unknown")
            };

            // Remove file extension and clean up
            Path::new(name_part)
                .file_stem()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("unknown")
                .replace('.', "_")
                + "_from_url"
        } else {
            // For file paths, use the existing logic
            Path::new(spec_name)
                .file_stem()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("unknown")
                .replace('.', "_")
        };

        self.output_dir
            .join(format!("{}_{}", template_name, spec_stem))
    }

    fn build_command(&self) -> Result<Command> {
        let binary_path = self.workspace_root.join("target/debug/mcpgen");
        Ok(Command::new(binary_path))
    }
}

/// Get the project root directory
fn project_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .context("Failed to determine project root directory")
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    // Test fixtures
    fn get_test_spec_path(relative_path: &str) -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let base_path = Path::new(manifest_dir).parent().unwrap().parent().unwrap();
        base_path.join(relative_path).to_str().unwrap().to_string()
    }

    // Helper function to clean up environment variables after test
    fn cleanup_env_vars() {
        let env_vars = [
            "MCPGEN_ALL_OPERATIONS",
            "MCPGEN_SERVER_PORT",
            "MCPGEN_LOG_FILE",
            "MCPGEN_TEMPLATE_CONTEXT",
        ];

        // Use unsafe block for remove_var as it modifies process state
        unsafe {
            for var in env_vars {
                std::env::remove_var(var);
            }
        }
    }

    lazy_static! {
        static ref OPENAPI_V3_URL: String =
            "https://petstore3.swagger.io/api/v3/openapi.json".to_string();
        static ref OPENAPI_V3_SPEC: String =
            get_test_spec_path("tests/fixtures/openapi/petstore.openapi.v3.json");
        static ref SWAGGER_V2_SPEC: String =
            get_test_spec_path("tests/fixtures/openapi/petstore.swagger.v2.json");
    }

    // Required files that must exist in the generated output
    const REQUIRED_FILES: &[&str] = &["Cargo.toml", "src/main.rs"];

    #[test]
    fn test_url_based_openapi_spec() -> Result<()> {
        // This test verifies that the CLI can load OpenAPI specs from URLs
        // and successfully generate a server

        cleanup_env_vars();
        let ctx = TestContext::new()?;

        let template = "rust_axum";
        let template_dir = ctx.workspace_root.join("templates").join(template);
        let output_dir = ctx.output_path(template, OPENAPI_V3_URL.as_str());

        if output_dir.exists() {
            std::fs::remove_dir_all(&output_dir)?;
        }

        println!("Testing URL-based OpenAPI spec with template: {}", template);
        println!("Output directory: {}", output_dir.display());

        // Run the scaffold command with a URL
        let mut cmd = ctx.build_command()?;
        cmd.arg("scaffold")
            .arg("--project-name")
            .arg("petstore-mcp-server")
            .arg("--spec")
            .arg(OPENAPI_V3_URL.as_str())
            .arg("--template-kind")
            .arg(template)
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--log-file")
            .arg("test.log")
            .arg("--template-dir")
            .arg(&template_dir)
            .arg("--port")
            .arg("8080");

        println!("Running command: {:?}", cmd);

        let output = cmd.output()?;

        if !output.status.success() {
            eprintln!("Command failed with status: {}", output.status);
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            bail!("Failed to scaffold from URL");
        }

        // Verify the output was created
        assert!(output_dir.exists(), "Output directory should exist");

        // Check for required files
        for file in REQUIRED_FILES {
            let file_path = output_dir.join(file);
            assert!(file_path.exists(), "Required file {} should exist", file);
        }

        println!("✅ Successfully scaffolded from URL: {}", *OPENAPI_V3_URL);
        Ok(())
    }

    #[test]
    fn test_all_templates_with_openapi_specs() -> Result<()> {
        // Clean up any existing environment variables
        cleanup_env_vars();
        let ctx = TestContext::new()?;
        let templates = ctx.list_templates()?;

        if templates.is_empty() {
            return Err(anyhow::anyhow!("No templates found in templates directory"));
        }

        println!("Found {} templates: {:?}", templates.len(), templates);

        // Test each template with both OpenAPI specs
        for template in templates {
            println!("\nTesting template: {}", template);

            // Test with OpenAPI v3 spec
            test_template_with_spec(&ctx, &template, &OPENAPI_V3_SPEC).with_context(|| {
                format!("Failed testing template {} with OpenAPI v3 spec", template)
            })?;

            // Test with Swagger v2 spec
            test_template_with_spec(&ctx, &template, &SWAGGER_V2_SPEC).with_context(|| {
                format!("Failed testing template {} with Swagger v2 spec", template)
            })?;
        }

        Ok(())
    }

    /// Test a specific template with a given OpenAPI spec
    fn test_template_with_spec(
        ctx: &TestContext,
        template_name: &str,
        spec_path: &str,
    ) -> Result<()> {
        let output_dir = ctx.output_path(template_name, spec_path);
        let template_path = ctx.template_path(template_name);

        // Clean up any previous output
        if output_dir.exists() {
            println!(
                "  Removing existing output directory: {}",
                output_dir.display()
            );
            std::fs::remove_dir_all(&output_dir)
                .with_context(|| format!("Failed to remove directory: {}", output_dir.display()))?;
        }

        // Ensure the parent directory of the output directory exists
        if let Some(parent) = output_dir.parent() {
            if !parent.exists() {
                println!("  Creating parent directory: {}", parent.display());
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent directory: {}", parent.display())
                })?;
            }
        }

        // Let the CLI handle the actual directory creation to avoid race conditions
        println!("  Output will be generated in: {}", output_dir.display());

        println!("  Testing with spec: {}", spec_path);
        println!("  Output directory: {}", output_dir.display());

        // Verify template directory exists and list its contents
        println!("  Template directory: {}", template_path.display());
        if !template_path.exists() {
            return Err(anyhow::anyhow!(
                "Template directory not found: {}",
                template_path.display()
            ));
        }

        // List template files for debugging
        println!("  Template files:");
        for entry in std::fs::read_dir(&template_path)? {
            let entry = entry?;
            println!("    - {}", entry.file_name().to_string_lossy());
        }

        // Build the CLI binary first
        println!("  Building mcpgen-cli in debug mode...");
        let build_status = Command::new("cargo")
            .args(["build"])
            .status()
            .context("Failed to execute cargo build")?;

        if !build_status.success() {
            return Err(anyhow::anyhow!(
                "Failed to build mcpgen-cli (status: {})",
                build_status
            ));
        }

        // Use the built binary from the workspace's target/debug directory
        let binary_path = ctx.workspace_root.join("target/debug/mcpgen");

        println!("Using binary at: {}", binary_path.display());

        if !binary_path.exists() {
            return Err(anyhow::anyhow!(
                "Binary not found at: {}",
                binary_path.display()
            ));
        }

        // Clean up any existing env vars and set server options
        cleanup_env_vars();
        // set server options
        unsafe {
            std::env::set_var("MCPGEN_SERVER_PORT", "8080");
            std::env::set_var("MCPGEN_LOG_FILE", "log.txt");
        }

        // Get the absolute path to the template directory
        let workspace_root = &ctx.workspace_root;
        let template_dir = workspace_root.join("templates").join(template_name);

        // Print debug information
        println!("Workspace root: {}", workspace_root.display());
        println!("Template directory: {}", template_dir.display());

        if !template_dir.exists() {
            return Err(anyhow!(
                "Template directory does not exist: {}",
                template_dir.display()
            ));
        }

        // Build the command with required arguments
        let mut cmd = Command::new(binary_path);
        cmd.arg("scaffold")
            .arg("--spec")
            .arg(spec_path)
            .arg("--template-kind")
            .arg(template_name)
            .arg("--template-dir")
            .arg(&template_dir) // Pass the specific template directory, not the parent
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--port")
            .arg("8080")
            .arg("--log-file")
            .arg("log.txt");

        // Print the full command being executed for debugging
        println!("Executing command: {:?}", cmd.get_program());
        for arg in cmd.get_args() {
            println!("  {:?}", arg);
        }

        // Execute the command with stderr capture
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute mcpgen-cli")?;

        // Always print stdout and stderr for debugging
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        println!("=== Command Output ===");
        println!("Status: {}", output.status);
        println!("=== STDOUT ===\n{}", stdout);
        println!("=== STDERR ===\n{}", stderr);

        // Check if command succeeded
        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Scaffold failed for template {} with spec {}\nStatus: {}\n=== STDOUT ===\n{}\n=== STDERR ===\n{}",
                template_name,
                spec_path,
                output.status,
                stdout,
                stderr
            ));
        }

        // Verify the output directory was created
        if !output_dir.exists() {
            return Err(anyhow::anyhow!(
                "Output directory was not created for template {} with spec {}",
                template_name,
                spec_path
            ));
        }

        // Check for required files
        for file in REQUIRED_FILES {
            let path = output_dir.join(file);
            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "Required file '{}' not found in output for template {} with spec {}",
                    file,
                    template_name,
                    spec_path
                ));
            }
        }

        // Try to build the generated project
        println!("  Building generated project...");
        let build_status = Command::new("cargo")
            .current_dir(&output_dir)
            .args(["build"])
            .status()
            .context("Failed to execute cargo build")?;

        if !build_status.success() {
            return Err(anyhow::anyhow!(
                "Failed to build generated project for template {} with spec {}",
                template_name,
                spec_path
            ));
        }

        println!("  Success!");
        Ok(())
    }
}
