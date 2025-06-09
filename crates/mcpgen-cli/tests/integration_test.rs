//! End-to-end integration tests for MCPGen CLI

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

// Test configuration
const SCAFFOLD_DIR: &str = ".mcpgen";

/// Test context containing paths and configuration
struct TestContext {
    output_dir: PathBuf,
    workspace_root: PathBuf,
}

impl TestContext {
    /// Create a new test context
    fn new() -> Result<Self> {
        let project_root = project_root()?;
        let output_dir = project_root.join(SCAFFOLD_DIR);
        let workspace_root = project_root.clone();

        // Ensure output directory exists
        std::fs::create_dir_all(&output_dir)?;

        Ok(Self {
            output_dir,
            workspace_root,
        })
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

    // Required files that must exist in the generated output
    const REQUIRED_FILES: &[&str] = &["Cargo.toml", "src/main.rs"];

    #[test]
    fn test_url_based_openapi_schema() -> Result<()> {
        test_openapi_schema(
            "https://petstore3.swagger.io/api/v3/openapi.json",
            "URL-based",
            Some("https://petstore3.swagger.io"),
        )
    }

    #[test]
    fn test_file_based_openapi_v3_schema() -> Result<()> {
        // Test with OpenAPI v3 spec
        let v3_schema_path =
            get_test_openapi_schema_path("tests/fixtures/openapi/petstore.openapi.v3.json");
        test_openapi_schema(
            &v3_schema_path,
            "OpenAPI v3 file-based",
            Some("https://petstore3.swagger.io"),
        )
    }

    #[test]
    fn test_file_based_openapi_v2_schema() -> Result<()> {
        // Test with Swagger v2 spec (no base URL - should be in schema)
        let v2_schema_path =
            get_test_openapi_schema_path("tests/fixtures/openapi/petstore.swagger.v2.json");
        test_openapi_schema(&v2_schema_path, "Swagger v2 file-based", None)
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

    fn get_test_openapi_schema_path(relative_path: &str) -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let base_path = Path::new(manifest_dir).parent().unwrap().parent().unwrap();
        base_path.join(relative_path).to_str().unwrap().to_string()
    }

    /// Common test function for OpenAPI schema processing
    fn test_openapi_schema(
        schema_path: &str,
        test_description: &str,
        base_url: Option<&str>,
    ) -> Result<()> {
        cleanup_env_vars();
        let ctx = TestContext::new()?;

        let template = "rust_axum";
        let template_dir = ctx.workspace_root.join("templates").join(template);
        let output_dir = ctx.output_path(template, schema_path);

        if output_dir.exists() {
            std::fs::remove_dir_all(&output_dir)?;
        }

        println!(
            "Testing {} OpenAPI spec with template: {}",
            test_description, template
        );
        println!("Output directory: {}", output_dir.display());

        // Build the CLI binary first to ensure we're testing with latest code
        println!("  Building mcpgen CLI in debug mode...");
        let build_status = Command::new("cargo")
            .args(["build"])
            .status()
            .context("Failed to execute cargo build for mcpgen CLI")?;

        if !build_status.success() {
            bail!("Failed to build mcpgen CLI (status: {})", build_status);
        }

        // Run the scaffold command
        let mut cmd = ctx.build_command()?;
        cmd.arg("scaffold")
            .arg("--project-name")
            .arg("petstore-mcp-server")
            .arg("--schema-path")
            .arg(schema_path)
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

        // Add base URL if provided
        if let Some(url) = base_url {
            cmd.arg("--base-url").arg(url);
        }

        println!("Running command: {:?}", cmd);

        let output = cmd.output()?;

        if !output.status.success() {
            eprintln!("Command failed with status: {}", output.status);
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            bail!("Failed to scaffold from {}", test_description);
        }

        // Verify the output was created
        assert!(output_dir.exists(), "Output directory should exist");

        // Check for required files
        for file in REQUIRED_FILES {
            let file_path = output_dir.join(file);
            assert!(file_path.exists(), "Required file {} should exist", file);
        }

        // Try to build the generated project to ensure it compiles
        println!("  Building generated project...");
        let build_status = Command::new("cargo")
            .current_dir(&output_dir)
            .args(["build"])
            .status()
            .context("Failed to execute cargo build on generated project")?;

        if !build_status.success() {
            bail!(
                "Failed to build generated project for {} - code generation produced invalid Rust code",
                test_description
            );
        }

        println!(
            "✅ Successfully scaffolded and built from {}: {}",
            test_description, schema_path
        );
        Ok(())
    }
}
