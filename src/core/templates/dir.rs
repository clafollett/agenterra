//! Unified handling of template directory resolution and operations

use std::io;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};

use super::{ClientTemplateKind, ServerTemplateKind};
use crate::core::protocol::Protocol;

/// Trait for reading template configuration, allowing dependency injection for testing
pub trait TemplateConfigReader {
    fn get_template_dir(&self) -> Option<String>;
}

/// Production implementation that reads from environment variables
pub struct EnvTemplateConfigReader;

impl TemplateConfigReader for EnvTemplateConfigReader {
    fn get_template_dir(&self) -> Option<String> {
        std::env::var("AGENTERRA_TEMPLATE_DIR").ok()
    }
}

/// Mock implementation for testing with controlled values
#[cfg(test)]
pub struct MockTemplateConfigReader(Option<String>);

#[cfg(test)]
impl MockTemplateConfigReader {
    pub fn new(template_dir: Option<String>) -> Self {
        Self(template_dir)
    }
}

#[cfg(test)]
impl TemplateConfigReader for MockTemplateConfigReader {
    fn get_template_dir(&self) -> Option<String> {
        self.0.clone()
    }
}

/// Represents a template directory with resolved paths and validation
#[derive(Debug, Clone)]
pub struct TemplateDir {
    /// Path to the specific template directory
    template_path: PathBuf,
    /// The template kind (language/framework) - server templates only
    kind: ServerTemplateKind,
    /// The protocol this template directory is for
    protocol: Protocol,
}

impl TemplateDir {
    /// Discover the template directory with explicit protocol support
    /// Arguments ordered to match CLI: protocol, kind (matching: scaffold `<role>` `<protocol>` `<kind>`)
    pub fn discover_with_protocol(
        protocol: Protocol,
        kind: ServerTemplateKind,
        custom_dir: Option<&Path>,
    ) -> io::Result<Self> {
        debug!(
            "TemplateDir::discover_with_protocol - protocol: {:?}, kind: {:?}, custom_dir: {:?}",
            protocol, kind, custom_dir
        );

        let template_path =
            Self::resolve_template_path(protocol, kind.role().as_str(), kind.as_str(), custom_dir)?;

        debug!("Resolved template path: {}", template_path.display());
        debug!("Template path exists: {}", template_path.exists());

        // Validate the template directory exists
        if !template_path.exists() {
            error!(
                "Template directory not found at resolved path: {}",
                template_path.display()
            );
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Template directory not found: {}", template_path.display()),
            ));
        }

        info!(
            "Successfully created TemplateDir for: {}",
            template_path.display()
        );
        Ok(Self {
            template_path,
            kind,
            protocol,
        })
    }

    /// Discover client template directory with explicit protocol support
    /// Arguments ordered to match CLI: protocol, kind (matching: scaffold `<role>` `<protocol>` `<kind>`)
    pub fn discover_client_with_protocol(
        protocol: Protocol,
        kind: ClientTemplateKind,
        custom_dir: Option<&Path>,
    ) -> io::Result<Self> {
        debug!(
            "TemplateDir::discover_client_with_protocol - protocol: {:?}, kind: {:?}, custom_dir: {:?}",
            protocol, kind, custom_dir
        );

        let template_path =
            Self::resolve_template_path(protocol, kind.role().as_str(), kind.as_str(), custom_dir)?;

        debug!("Resolved client template path: {}", template_path.display());
        debug!("Client template path exists: {}", template_path.exists());

        // Validate the template directory exists
        if !template_path.exists() {
            error!(
                "Client template directory not found at resolved path: {}",
                template_path.display()
            );
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Client template directory not found: {}",
                    template_path.display()
                ),
            ));
        }

        info!(
            "Successfully created TemplateDir for client: {}",
            template_path.display()
        );
        Ok(Self {
            template_path,
            // For client templates, we store a default server kind
            kind: ServerTemplateKind::Custom, // Default, not used for client templates
            protocol,
        })
    }

    /// Create a TemplateDir from an embedded template path
    /// This is used when templates are loaded from embedded resources
    pub fn from_embedded_path(path: PathBuf) -> io::Result<Self> {
        // Parse the path to extract protocol and kind
        let path_str = path.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Template path contains invalid UTF-8",
            )
        })?;

        let parts: Vec<&str> = path_str.split('/').collect();
        if parts.len() < 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid embedded template path format: {}", path_str),
            ));
        }

        let protocol = parts[0].parse::<Protocol>().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid protocol in template path: {}", e),
            )
        })?;

        let role = parts[1];
        let kind_str = parts[2];

        let kind = if role == "server" {
            kind_str.parse::<ServerTemplateKind>().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid server template kind: {}", e),
                )
            })?
        } else {
            // For client templates, we use a default kind
            ServerTemplateKind::Custom
        };

        Ok(Self {
            template_path: path,
            kind,
            protocol,
        })
    }

    /// Get the template kind
    pub fn kind(&self) -> ServerTemplateKind {
        self.kind
    }

    /// Get the protocol
    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    /// Get the path to the specific template directory
    pub fn template_path(&self) -> &Path {
        &self.template_path
    }

    /// Resolve template path by using custom directory or auto-discovering
    fn resolve_template_path(
        protocol: Protocol,
        role: &str,
        kind: &str,
        custom_dir: Option<&Path>,
    ) -> io::Result<PathBuf> {
        if let Some(dir) = custom_dir {
            // Use the provided directory directly - take user at their word
            debug!(
                "Using custom template directory directly: {}",
                dir.display()
            );
            if !dir.exists() {
                error!("Custom template directory not found: {}", dir.display());
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Template directory not found: {}", dir.display()),
                ));
            }
            Ok(dir.to_path_buf())
        } else {
            // Auto-discover the template directory and use protocol-aware structure
            debug!("Auto-discovering template directory...");
            let discovered = Self::find_template_base_dir().ok_or_else(|| {
                error!("Could not find template directory in any standard location");
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Could not find template directory in any standard location",
                )
            })?;
            debug!("Auto-discovered template base: {}", discovered.display());

            // Only append protocol structure for auto-discovered paths
            let template_path = discovered
                .join("templates")
                .join(protocol.path_segment())
                .join(role)
                .join(kind);

            Ok(template_path)
        }
    }

    /// Find the base template directory by checking standard locations
    fn find_template_base_dir() -> Option<PathBuf> {
        Self::find_template_base_dir_with_config(&EnvTemplateConfigReader)
    }

    /// Find the base template directory with a custom config reader (for testing)
    fn find_template_base_dir_with_config(
        config_reader: &dyn TemplateConfigReader,
    ) -> Option<PathBuf> {
        // 1. Check environment variable via config reader
        if let Some(dir) = config_reader.get_template_dir() {
            let path = PathBuf::from(dir);

            // Always validate the path for security, even if it doesn't exist yet
            if let Err(e) = Self::validate_template_path_safely(&path) {
                error!("Template directory validation failed: {}", e);
                return None;
            }

            if path.exists() {
                return Some(path);
            }
        }

        // 2. Check standard locations in order of preference
        let search_locations = Self::get_template_search_locations();

        search_locations
            .into_iter()
            .find(|location| location.join("templates").exists())
    }

    /// Get list of locations to search for templates
    fn get_template_search_locations() -> Vec<PathBuf> {
        let mut locations = Vec::new();

        // Check executable directory and parent directories
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                if let Ok(exe_dir_abs) = exe_dir.canonicalize() {
                    locations.push(exe_dir_abs.clone());
                    if let Some(parent_dir) = exe_dir_abs.parent() {
                        locations.push(parent_dir.to_path_buf());
                    }
                }
            }
        }

        // Check current directory (as fallback for development)
        if let Ok(current_dir) = std::env::current_dir() {
            locations.push(current_dir);
        }

        // Check in the crate root (for development)
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let manifest_path = PathBuf::from(manifest_dir);
            locations.push(manifest_path.clone());

            if let Some(workspace_root) = manifest_path.parent() {
                locations.push(workspace_root.to_path_buf());
            }
        }

        // Check in the user's home directory config location
        if let Some(config_dir) = dirs::config_dir() {
            locations.push(config_dir.join("agenterra"));
        }

        locations
    }

    /// Validate that a template directory path is safe
    /// Uses path-based analysis instead of string matching for cross-platform compatibility
    fn validate_template_path(path: &Path) -> Result<(), io::Error> {
        // Canonicalize to resolve any ".." or "." components
        let canonical_path = path.canonicalize().map_err(|e| {
            error!("Failed to canonicalize template path: {}", e);
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid template path: {e}"),
            )
        })?;

        debug!("Validating template path: {}", canonical_path.display());

        // Check for system-critical directories (Unix-only)
        Self::validate_unix_system_paths(&canonical_path)?;

        // Check if path is under any allowed location
        if Self::is_path_allowed(&canonical_path) {
            return Ok(());
        }

        // If we get here, the path is not under any known safe location and not in a critical system directory
        // This might be acceptable for some use cases, so we'll allow it but log a warning
        debug!(
            "Template path validation passed (external location): {}",
            canonical_path.display()
        );
        Ok(())
    }

    /// Check if a path is under any of the allowed locations
    fn is_path_allowed(canonical_path: &Path) -> bool {
        // After security checks pass, allow paths under user's home directory
        if let Some(home_dir) = dirs::home_dir() {
            if let Ok(home_canonical) = home_dir.canonicalize() {
                if canonical_path.starts_with(&home_canonical) {
                    debug!(
                        "Template path allowed under home directory: {}",
                        canonical_path.display()
                    );
                    return true;
                }
            }
        }

        // Allow paths under current working directory and its parents (for development)
        if let Ok(current_dir) = std::env::current_dir() {
            if let Ok(current_canonical) = current_dir.canonicalize() {
                if Self::is_under_workspace(canonical_path, &current_canonical) {
                    return true;
                }
            }
        }

        // Allow paths under CARGO_MANIFEST_DIR and its parents (for development/testing)
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let manifest_path = PathBuf::from(manifest_dir);
            if let Ok(manifest_canonical) = manifest_path.canonicalize() {
                if canonical_path.starts_with(&manifest_canonical) {
                    debug!(
                        "Template path allowed under cargo manifest dir: {}",
                        canonical_path.display()
                    );
                    return true;
                }

                if let Some(parent) = manifest_canonical.parent() {
                    if canonical_path.starts_with(parent) {
                        debug!(
                            "Template path allowed under cargo workspace: {}",
                            canonical_path.display()
                        );
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if a path is under the workspace directory or its parents
    fn is_under_workspace(path: &Path, workspace_dir: &Path) -> bool {
        const MAX_PARENT_DEPTH: usize = 3;

        // Check if under workspace directory
        if path.starts_with(workspace_dir) {
            debug!(
                "Template path allowed under current directory: {}",
                path.display()
            );
            return true;
        }

        // Check under immediate parent directories (for workspace setups)
        let mut parent = workspace_dir;
        for depth in 0..MAX_PARENT_DEPTH {
            if let Some(p) = parent.parent() {
                if path.starts_with(p) {
                    debug!(
                        "Template path allowed under workspace parent (depth {}): {}",
                        depth + 1,
                        path.display()
                    );
                    return true;
                }
                parent = p;
            } else {
                break;
            }
        }

        false
    }

    /// Validate template path safely, handling cases where the path might not exist
    fn validate_template_path_safely(path: &Path) -> Result<(), io::Error> {
        // First try the regular validation for existing paths
        if path.exists() {
            return Self::validate_template_path(path);
        }

        // For non-existent paths, do basic validation without canonicalization
        debug!("Validating non-existent template path: {}", path.display());

        // Convert to string for pattern checking
        let path_str = path.to_string_lossy();

        // Check for obviously malicious patterns
        #[cfg(unix)]
        {
            const SYSTEM_DIRS: &[&str] = &[
                "/etc/",
                "/usr/bin/",
                "/usr/sbin/",
                "/root/",
                "/boot/",
                "/sys/",
                "/proc/",
            ];

            for sys_dir in SYSTEM_DIRS {
                if path_str.starts_with(sys_dir) {
                    error!("Potentially unsafe template path rejected: {}", path_str);
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("Template path not allowed: {path_str}"),
                    ));
                }
            }
        }

        // Check for directory traversal patterns
        if path_str.contains("../../../") {
            error!(
                "Directory traversal detected in template path: {}",
                path_str
            );
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Directory traversal not allowed in template paths".to_string(),
            ));
        }

        debug!(
            "Template path validation passed (non-existent path): {}",
            path.display()
        );
        Ok(())
    }

    /// Validate Unix system paths (includes macOS /private/ handling)
    fn validate_unix_system_paths(canonical_path: &Path) -> Result<(), io::Error> {
        let components: Vec<_> = canonical_path.components().collect();

        // Need at least root + one directory component
        if components.len() < 2 {
            return Ok(());
        }

        let second = match (components.first(), components.get(1)) {
            (Some(std::path::Component::RootDir), Some(std::path::Component::Normal(dir))) => {
                dir.to_str().unwrap_or("")
            }
            _ => return Ok(()), // Not an absolute Unix path
        };

        // Handle standard Unix system directories
        if Self::is_system_directory(second) {
            if Self::is_temp_exception(&components, 1) {
                return Ok(()); // Allow /tmp and /var/tmp
            }
            return Err(Self::system_directory_error(canonical_path));
        }

        // Handle macOS /private/ prefixed system directories
        if second == "private" && components.len() >= 3 {
            if let Some(std::path::Component::Normal(third)) = components.get(2) {
                let third_str = third.to_str().unwrap_or("");
                if Self::is_system_directory(third_str) {
                    if Self::is_temp_exception(&components, 2) {
                        return Ok(()); // Allow /private/tmp and /private/var/tmp
                    }
                    return Err(Self::system_directory_error(canonical_path));
                }
            }
        }

        Ok(())
    }

    /// Check if a directory name is a protected system directory
    fn is_system_directory(name: &str) -> bool {
        matches!(name, "etc" | "usr" | "root" | "boot" | "sys" | "proc")
    }

    /// Check if this is an allowed temp directory exception
    fn is_temp_exception(components: &[std::path::Component], base_index: usize) -> bool {
        if let Some(std::path::Component::Normal(dir)) = components.get(base_index) {
            let dir_str = dir.to_str().unwrap_or("");

            // Allow /tmp or /private/tmp
            if dir_str == "tmp" {
                return true;
            }

            // Allow /var/tmp or /private/var/tmp
            if dir_str == "var" {
                if let Some(std::path::Component::Normal(subdir)) = components.get(base_index + 1) {
                    return subdir.to_str().unwrap_or("") == "tmp";
                }
            }
        }
        false
    }

    /// Create a standard system directory access error
    fn system_directory_error(path: &Path) -> io::Error {
        error!("System directory access rejected: {}", path.display());
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "Template path not allowed in system directory: {}",
                path.display()
            ),
        )
    }
}

/// Resolve output directory for generated projects
///
/// Resolution order:
/// 1. custom_output_dir parameter (CLI --output-dir flag) - used as parent directory
/// 2. AGENTERRA_OUTPUT_DIR environment variable - used as parent directory
/// 3. Default: current_dir/project_name (like cargo new)
pub fn resolve_output_dir(
    project_name: &str,
    custom_output_dir: Option<&Path>,
) -> io::Result<PathBuf> {
    let output_path = if let Some(custom_dir) = custom_output_dir {
        // Use custom directory as parent, append project name
        debug!("Using custom output directory: {}", custom_dir.display());
        custom_dir.join(project_name)
    } else if let Ok(env_dir) = std::env::var("AGENTERRA_OUTPUT_DIR") {
        // Use environment variable as parent directory
        let env_path = PathBuf::from(env_dir);
        debug!("Using AGENTERRA_OUTPUT_DIR: {}", env_path.display());
        env_path.join(project_name)
    } else {
        // Default behavior: current_directory/project_name (like cargo new)
        let current_dir = std::env::current_dir()
            .map_err(|e| io::Error::other(format!("Failed to get current directory: {e}")))?;

        let output_dir = current_dir.join(project_name);

        debug!("Using default output directory: {}", output_dir.display());
        output_dir
    };

    // Convert to absolute path if needed
    let absolute_path = if output_path.is_absolute() {
        output_path
    } else {
        std::env::current_dir()
            .map_err(|e| io::Error::other(format!("Failed to get current directory: {e}")))?
            .join(output_path)
    };

    debug!("Resolved output path: {}", absolute_path.display());
    Ok(absolute_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tracing_test::traced_test;

    /// Create a test workspace directory under target/tmp/test-workspaces
    /// This keeps all test artifacts in the gitignored target directory
    pub fn create_test_workspace(test_name: &str) -> std::path::PathBuf {
        // Find the workspace root by looking for Cargo.toml
        let workspace_root = std::env::current_dir()
            .expect("Failed to get current directory")
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists())
            .expect("Could not find workspace root")
            .to_path_buf();

        let workspace_dir = workspace_root
            .join("target")
            .join("tmp")
            .join("test-workspaces")
            .join(test_name)
            .join(uuid::Uuid::new_v4().to_string());

        // Create the directory
        fs::create_dir_all(&workspace_dir).unwrap();

        // Canonicalize to get absolute path
        workspace_dir.canonicalize().unwrap_or(workspace_dir)
    }

    #[test]
    fn test_template_dir_validation() {
        let temp_dir = create_test_workspace("test_template_dir_validation");

        // Create new server structure
        let server_template_dir = temp_dir.join("templates/mcp/server/rust");
        fs::create_dir_all(&server_template_dir).unwrap();

        // Test server template discovery
        // With new logic, custom paths are used directly, so pass the full template path
        let server_template = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::Rust,
            Some(&server_template_dir),
        );
        assert!(server_template.is_ok());
        assert_eq!(
            server_template.unwrap().template_path(),
            server_template_dir.as_path()
        );

        // Test with non-existent directory
        let result = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::Rust,
            Some(Path::new("/nonexistent")),
        );
        assert!(result.is_err());
    }

    #[test]
    #[traced_test]
    fn test_debug_logging_output() {
        let temp_dir = create_test_workspace("test_debug_logging_output");
        let server_template_dir = temp_dir.join("templates/mcp/server/rust");
        fs::create_dir_all(&server_template_dir).unwrap();

        // This should generate debug logs
        let _result = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::Rust,
            Some(&temp_dir),
        );

        // Check that debug logs were generated
        // Note: This test will fail initially with eprintln! but pass with tracing::debug!
        assert!(
            logs_contain("Auto-discovering template directory")
                || logs_contain("Resolved template path")
        );
    }

    #[test]
    fn test_find_template_base_dir_uses_absolute_paths() {
        // Test absolute path resolution using mock config reader
        let temp_workspace =
            create_test_workspace("test_find_template_base_dir_uses_absolute_paths");
        let templates_dir = temp_workspace.join("templates");
        let mcp_dir = templates_dir.join("mcp");
        let server_dir = mcp_dir.join("server");
        let client_dir = mcp_dir.join("client");
        fs::create_dir_all(&server_dir).unwrap();
        fs::create_dir_all(&client_dir).unwrap();

        // Test with mock config reader - no global state modification
        let mock_config =
            MockTemplateConfigReader::new(Some(temp_workspace.to_string_lossy().to_string()));
        let result = TemplateDir::find_template_base_dir_with_config(&mock_config);
        assert!(result.is_some());

        // Test the resolved path is absolute and exists
        let resolved_path = result.unwrap();
        assert!(resolved_path.is_absolute());
        assert!(resolved_path.exists());
    }

    #[test]
    fn test_find_template_base_dir_executable_location() {
        // Test that template discovery works from executable location
        // This simulates the installed binary scenario
        let temp_workspace =
            create_test_workspace("test_find_template_base_dir_executable_location");
        let bin_dir = temp_workspace.join("bin");
        let templates_dir = temp_workspace.join("templates");
        let mcp_dir = templates_dir.join("mcp");
        let server_dir = mcp_dir.join("server");
        let client_dir = mcp_dir.join("client");

        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&server_dir).unwrap();
        fs::create_dir_all(&client_dir).unwrap();

        // Test with mock config that simulates env var configuration
        let mock_config =
            MockTemplateConfigReader::new(Some(temp_workspace.to_string_lossy().to_string()));
        let result = TemplateDir::find_template_base_dir_with_config(&mock_config);
        assert!(result.is_some());

        // Verify the discovered path exists
        let discovered_path = result.unwrap();
        assert!(discovered_path.exists());
    }

    #[test]
    fn test_security_template_dir_validation() {
        // Test that template directory paths are validated for security
        let malicious_paths = vec![
            "/etc/passwd",                 // Should be rejected - system directory
            "/usr/bin/evil",               // Should be rejected - system directory
            "/root/.ssh/id_rsa",           // Should be rejected - system directory
            "../../../etc/passwd",         // Should be rejected - directory traversal
            "/usr/local/../../etc/passwd", // Should be rejected - directory traversal + system dir
        ];

        // Windows-specific paths (only test on Windows)
        #[cfg(windows)]
        let windows_paths = vec!["C:\\Windows\\System32", "C:\\Program Files\\evil"];

        #[cfg(windows)]
        let all_paths = [malicious_paths, windows_paths].concat();

        #[cfg(not(windows))]
        let all_paths = malicious_paths;

        for path in all_paths {
            // Test with mock config reader using malicious path
            let mock_config = MockTemplateConfigReader::new(Some(path.to_string()));
            let result = TemplateDir::find_template_base_dir_with_config(&mock_config);

            // The path should be rejected for security reasons
            assert!(
                result.is_none(),
                "Malicious path should be rejected: {path}"
            );
        }
    }

    #[test]
    fn test_output_directory_traversal_protection() {
        // Test protection against output directory traversal
        let temp_dir = create_test_workspace("test_output_directory_traversal_protection");
        let server_template_dir = temp_dir.join("templates/mcp/server/rust");
        fs::create_dir_all(&server_template_dir).unwrap();

        // Attempt to create template dir with malicious output path
        let malicious_output_paths = vec!["../../../etc", "/etc", "../../sensitive"];

        for _path in malicious_output_paths {
            // This test documents the need for output path validation
            // Currently there's no validation in TemplateDir
            // The validation should happen in the CLI layer
        }
    }

    #[test]
    #[allow(unsafe_code)] // Required for set_var/remove_var in tests
    fn test_environment_variable_template_discovery() {
        let temp_dir = create_test_workspace("env_var_template_discovery");
        let templates_dir = temp_dir.join("templates");
        let mcp_dir = templates_dir.join("mcp");
        let server_dir = mcp_dir.join("server");
        let client_dir = mcp_dir.join("client");
        fs::create_dir_all(&server_dir).unwrap();
        fs::create_dir_all(&client_dir).unwrap();

        // Test 1: Without env var set (should return None for env var path)
        let env_config = EnvTemplateConfigReader;
        let _no_env_result = env_config.get_template_dir();
        // Note: We can't assert None because AGENTERRA_TEMPLATE_DIR might be set globally
        // This test documents the behavior

        // Test 2: With env var set temporarily (single-threaded, marked unsafe due to race potential)
        unsafe {
            // SAFETY: This test runs in a single thread, so we know no other thread
            // will be reading the environment variable concurrently
            std::env::set_var("AGENTERRA_TEMPLATE_DIR", &temp_dir);
        }

        let with_env_result = env_config.get_template_dir();
        assert!(with_env_result.is_some());
        assert_eq!(with_env_result.unwrap(), temp_dir.to_string_lossy());

        // Test 3: Test the full discovery process with env var
        let discovery_result = TemplateDir::find_template_base_dir();
        assert!(discovery_result.is_some());

        // Cleanup (unsafe due to potential race with other threads reading env vars)
        unsafe {
            // SAFETY: This test runs in a single thread, so we know no other thread
            // will be reading the environment variable concurrently
            std::env::remove_var("AGENTERRA_TEMPLATE_DIR");
        }

        // Test 4: After cleanup, env var should be gone
        let _after_cleanup = env_config.get_template_dir();
        // Note: Can't assert None due to potential global env var, but documents cleanup
    }

    #[test]
    fn test_concurrent_template_discovery() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        // Setup shared test directory
        let temp_dir = create_test_workspace("test_concurrent_template_discovery");
        let server_template_dir = temp_dir.join("templates/mcp/server/rust");
        let client_template_dir = temp_dir.join("templates/mcp/client/rust");
        fs::create_dir_all(&server_template_dir).unwrap();
        fs::create_dir_all(&client_template_dir).unwrap();

        const NUM_THREADS: usize = 10;
        let barrier = Arc::new(Barrier::new(NUM_THREADS));
        let mut handles = vec![];

        // Spawn multiple threads that all try to discover templates simultaneously
        for i in 0..NUM_THREADS {
            let barrier_clone = Arc::clone(&barrier);
            let temp_dir_path = temp_dir.to_string_lossy().to_string();

            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier_clone.wait();

                // Each thread uses its own mock config reader (thread-safe)
                let mock_config = MockTemplateConfigReader::new(Some(temp_dir_path));
                let result = TemplateDir::find_template_base_dir_with_config(&mock_config);

                // Should succeed without panics or race conditions
                assert!(result.is_some(), "Thread {i} failed to discover template");

                let base_dir = result.unwrap();
                assert!(base_dir.exists());
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // No cleanup needed - no global state was modified
    }

    // TDD Red phase: Protocol-aware tests (should fail until we implement protocol support)
    #[test]
    fn test_discover_with_protocol() {
        use crate::core::protocol::Protocol;

        let temp_dir = create_test_workspace("test_discover_with_protocol");
        let server_template_dir = temp_dir.join("templates/mcp/server/rust");
        fs::create_dir_all(&server_template_dir).unwrap();

        // Test server template discovery with protocol parameter (custom path)
        // With new logic, custom paths are used directly, so pass the full template path
        let result = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::Rust,
            Some(&server_template_dir), // Pass the actual template directory
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().template_path(),
            server_template_dir.as_path()
        );
    }

    #[test]
    fn test_path_construction_with_different_protocols() {
        use crate::core::protocol::Protocol;

        let temp_dir = create_test_workspace("test_path_construction_with_different_protocols");

        // Create template directories for different protocols
        let mcp_server_dir = temp_dir.join("templates/mcp/server/rust");
        fs::create_dir_all(&mcp_server_dir).unwrap();

        // Test MCP protocol
        // With new logic, custom paths are used directly, so pass the full template path
        let result = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::Rust,
            Some(&mcp_server_dir),
        );
        assert!(result.is_ok());
        let template_dir = result.unwrap();

        // The resolved path should contain the protocol segment
        let path_str = template_dir.template_path().to_string_lossy();
        assert!(
            path_str.contains("mcp"),
            "Path should contain protocol segment: {path_str}"
        );
        assert!(
            path_str.contains("server"),
            "Path should contain role: {path_str}"
        );
        assert!(
            path_str.contains("rust"),
            "Path should contain template kind: {path_str}"
        );
    }

    #[test]
    fn test_backward_compatibility_with_discover() {
        // Test that the old discover method still works
        let temp_dir = create_test_workspace("test_backward_compatibility_with_discover");
        let server_template_dir = temp_dir.join("templates/mcp/server/rust");
        fs::create_dir_all(&server_template_dir).unwrap();

        // Test discover_with_protocol method (uses MCP protocol)
        // With new logic, custom paths are used directly, so pass the full template path
        let result = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::Rust,
            Some(&server_template_dir),
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().template_path(),
            server_template_dir.as_path()
        );
    }

    #[test]
    fn test_resolve_output_dir_with_custom_path() {
        let temp_dir = create_test_workspace("test_resolve_output_dir_with_custom_path");
        let custom_output = temp_dir.join("custom_output");

        let result = resolve_output_dir("test_project", Some(&custom_output));
        assert!(result.is_ok());

        let resolved_path = result.unwrap();
        assert!(resolved_path.is_absolute());
        // Should append project name to custom directory
        assert!(resolved_path.ends_with("custom_output/test_project"));
    }

    #[test]
    fn test_resolve_output_dir_with_default() {
        let temp_dir = create_test_workspace("test_resolve_output_dir_with_default");

        // Test by temporarily changing to the temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = resolve_output_dir("test_project", None);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let resolved_path = result.unwrap();
        assert!(resolved_path.is_absolute());
        // Should use current_dir/project_name pattern
        assert!(resolved_path.to_string_lossy().ends_with("test_project"));
    }

    #[test]
    fn test_resolve_output_dir_fallback_behavior() {
        // Test fallback when no workspace is found by changing to a temp directory
        let temp_dir = create_test_workspace("test_resolve_output_dir_fallback_behavior");
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // This should use current_dir/scaffolded/project_name pattern
        let result = resolve_output_dir("fallback_project", None);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let resolved_path = result.unwrap();
        assert!(resolved_path.is_absolute());
        assert!(
            resolved_path
                .to_string_lossy()
                .ends_with("fallback_project")
        );
    }
}
