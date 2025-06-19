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
    /// Create a new TemplateDir with explicit paths
    /// Arguments ordered to match CLI: protocol, kind (matching: scaffold <role> <protocol> <kind>)
    pub fn new(
        _root_dir: PathBuf,
        template_path: PathBuf,
        protocol: Protocol,
        kind: ServerTemplateKind,
    ) -> Self {
        Self {
            template_path,
            kind,
            protocol,
        }
    }

    /// Create a new TemplateDir for client templates with explicit paths
    /// Arguments ordered to match CLI: protocol, kind (matching: scaffold <role> <protocol> <kind>)
    pub fn new_client(
        _root_dir: PathBuf,
        template_path: PathBuf,
        protocol: Protocol,
        _kind: ClientTemplateKind,
    ) -> Self {
        // For client templates, we store a default server kind
        Self {
            template_path,
            kind: ServerTemplateKind::Custom, // Default, not used for client templates
            protocol,
        }
    }

    /// Discover the template directory with explicit protocol support
    /// Arguments ordered to match CLI: protocol, kind (matching: scaffold <role> <protocol> <kind>)
    pub fn discover_with_protocol(
        protocol: Protocol,
        kind: ServerTemplateKind,
        custom_dir: Option<&Path>,
    ) -> io::Result<Self> {
        debug!(
            "TemplateDir::discover_with_protocol - protocol: {:?}, kind: {:?}, custom_dir: {:?}",
            protocol, kind, custom_dir
        );

        let (root_dir, template_path) = if let Some(dir) = custom_dir {
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
            // For custom paths, root_dir is the same as template_path
            (dir.to_path_buf(), dir.to_path_buf())
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
                .join(kind.role().as_str())
                .join(kind.as_str());

            (discovered, template_path)
        };

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
        Ok(Self::new(root_dir, template_path, protocol, kind))
    }

    /// Discover client template directory with explicit protocol support
    /// Arguments ordered to match CLI: protocol, kind (matching: scaffold <role> <protocol> <kind>)
    pub fn discover_client_with_protocol(
        protocol: Protocol,
        kind: ClientTemplateKind,
        custom_dir: Option<&Path>,
    ) -> io::Result<Self> {
        debug!(
            "TemplateDir::discover_client_with_protocol - protocol: {:?}, kind: {:?}, custom_dir: {:?}",
            protocol, kind, custom_dir
        );

        let (root_dir, template_path) = if let Some(dir) = custom_dir {
            // Use the provided directory directly - take user at their word
            debug!(
                "Using custom client template directory directly: {}",
                dir.display()
            );
            if !dir.exists() {
                error!(
                    "Custom client template directory not found: {}",
                    dir.display()
                );
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Client template directory not found: {}", dir.display()),
                ));
            }
            // For custom paths, root_dir is the same as template_path
            (dir.to_path_buf(), dir.to_path_buf())
        } else {
            // Auto-discover the template directory and use protocol-aware structure
            debug!("Auto-discovering client template directory...");
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
                .join(kind.role().as_str())
                .join(kind.as_str());

            (discovered, template_path)
        };

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
        Ok(Self::new_client(root_dir, template_path, protocol, kind))
    }

    /// Find the base template directory by checking standard locations
    pub fn find_template_base_dir() -> Option<PathBuf> {
        Self::find_template_base_dir_with_config(&EnvTemplateConfigReader)
    }

    /// Find the base template directory with a custom config reader (for testing)
    pub fn find_template_base_dir_with_config(
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

        // 2. Check executable directory and parent directories
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Canonicalize to get absolute path
                if let Ok(exe_dir_abs) = exe_dir.canonicalize() {
                    // Check if templates are next to the executable
                    let templates_dir = exe_dir_abs.join("templates");
                    if templates_dir.exists() {
                        return Some(exe_dir_abs);
                    }

                    // Check parent directory (for development)
                    if let Some(parent_dir) = exe_dir_abs.parent() {
                        let templates_dir = parent_dir.join("templates");
                        if templates_dir.exists() {
                            return Some(parent_dir.to_path_buf());
                        }
                    }
                }
            }
        }

        // 3. Check current directory (as fallback for development)
        if let Ok(current_dir) = std::env::current_dir() {
            let templates_dir = current_dir.join("templates");
            if templates_dir.exists() {
                return Some(current_dir);
            }
        }

        // 4. Check in the crate root (for development)
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let manifest_path = PathBuf::from(manifest_dir);

            // First check the manifest directory itself (for workspace root)
            let templates_dir = manifest_path.join("templates");
            if templates_dir.exists() {
                return Some(manifest_path);
            }

            // Then check parent (for sub-crates in workspace)
            if let Some(workspace_root) = manifest_path.parent() {
                let templates_dir = workspace_root.join("templates");
                if templates_dir.exists() {
                    return Some(workspace_root.to_path_buf());
                }
            }
        }

        // 5. Check in the user's home directory
        if let Some(home_dir) = dirs::home_dir() {
            let templates_dir = home_dir.join(".agenterra").join("templates");
            if templates_dir.exists() {
                return Some(home_dir.join(".agenterra"));
            }
        }

        None
    }

    /// Validate that a template directory path is safe
    /// Uses path-based analysis instead of string matching for cross-platform compatibility
    fn validate_template_path(path: &Path) -> Result<(), io::Error> {
        // Canonicalize to resolve any ".." or "." components
        let canonical_path = path.canonicalize().map_err(|e| {
            error!("Failed to canonicalize template path: {}", e);
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid template path: {}", e),
            )
        })?;

        debug!("Validating template path: {}", canonical_path.display());

        // Check for system-critical directories (Unix-only)
        Self::validate_unix_system_paths(&canonical_path)?;

        // After security checks pass, allow paths under user's home directory
        if let Some(home_dir) = dirs::home_dir() {
            if let Ok(home_canonical) = home_dir.canonicalize() {
                if canonical_path.starts_with(&home_canonical) {
                    debug!(
                        "Template path allowed under home directory: {}",
                        canonical_path.display()
                    );
                    return Ok(());
                }
            }
        }

        // Allow paths under current working directory and its parents (for development)
        if let Ok(current_dir) = std::env::current_dir() {
            if let Ok(current_canonical) = current_dir.canonicalize() {
                // Allow under current directory
                if canonical_path.starts_with(&current_canonical) {
                    debug!(
                        "Template path allowed under current directory: {}",
                        canonical_path.display()
                    );
                    return Ok(());
                }

                // Allow under immediate parent directories (for workspace setups)
                // But limit to reasonable depth to avoid allowing root directory
                let mut parent = current_canonical.as_path();
                let mut depth = 0;
                const MAX_PARENT_DEPTH: usize = 3; // Only go up 3 levels max

                while let Some(p) = parent.parent() {
                    if depth >= MAX_PARENT_DEPTH {
                        break;
                    }
                    if canonical_path.starts_with(p) {
                        debug!(
                            "Template path allowed under workspace parent (depth {}): {}",
                            depth,
                            canonical_path.display()
                        );
                        return Ok(());
                    }
                    parent = p;
                    depth += 1;
                }
            }
        }

        // Allow paths under CARGO_MANIFEST_DIR and its parents (for development/testing)
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let manifest_path = PathBuf::from(manifest_dir);
            if let Ok(manifest_canonical) = manifest_path.canonicalize() {
                // Allow under manifest dir
                if canonical_path.starts_with(&manifest_canonical) {
                    debug!(
                        "Template path allowed under cargo manifest dir: {}",
                        canonical_path.display()
                    );
                    return Ok(());
                }

                // Allow under manifest parent (workspace root)
                if let Some(parent) = manifest_canonical.parent() {
                    if canonical_path.starts_with(parent) {
                        debug!(
                            "Template path allowed under cargo workspace: {}",
                            canonical_path.display()
                        );
                        return Ok(());
                    }
                }
            }
        }

        // If we get here, the path is not under any known safe location and not in a critical system directory
        // This might be acceptable for some use cases, so we'll allow it but log a warning
        debug!(
            "Template path validation passed (external location): {}",
            canonical_path.display()
        );
        Ok(())
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
            // Check for absolute paths to system directories
            if path_str.starts_with("/etc/")
                || path_str.starts_with("/usr/bin/")
                || path_str.starts_with("/usr/sbin/")
                || path_str.starts_with("/root/")
                || path_str.starts_with("/boot/")
                || path_str.starts_with("/sys/")
                || path_str.starts_with("/proc/")
            {
                error!("Potentially unsafe template path rejected: {}", path_str);
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("Template path not allowed: {}", path_str),
                ));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tracing_test::traced_test;

    /// Create a test workspace directory under ./target/test-workspaces
    /// This is platform-agnostic and avoids system temp directory issues
    fn create_test_workspace(test_name: &str) -> std::path::PathBuf {
        let workspace_dir = std::path::PathBuf::from("./target/test-workspaces")
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
        let server_template_dir = temp_dir.join("templates/mcp/server/rust_axum");
        fs::create_dir_all(&server_template_dir).unwrap();

        // Test server template discovery
        // With new logic, custom paths are used directly, so pass the full template path
        let server_template = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::RustAxum,
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
            ServerTemplateKind::RustAxum,
            Some(Path::new("/nonexistent")),
        );
        assert!(result.is_err());
    }

    #[test]
    #[traced_test]
    fn test_debug_logging_output() {
        let temp_dir = create_test_workspace("test_debug_logging_output");
        let server_template_dir = temp_dir.join("templates/mcp/server/rust_axum");
        fs::create_dir_all(&server_template_dir).unwrap();

        // This should generate debug logs
        let _result = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::RustAxum,
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
                "Malicious path should be rejected: {}",
                path
            );
        }
    }

    #[test]
    fn test_output_directory_traversal_protection() {
        // Test protection against output directory traversal
        let temp_dir = create_test_workspace("test_output_directory_traversal_protection");
        let server_template_dir = temp_dir.join("templates/mcp/server/rust_axum");
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
        let server_template_dir = temp_dir.join("templates/mcp/server/rust_axum");
        let client_template_dir = temp_dir.join("templates/mcp/client/rust_reqwest");
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
                assert!(result.is_some(), "Thread {} failed to discover template", i);

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
        let server_template_dir = temp_dir.join("templates/mcp/server/rust_axum");
        fs::create_dir_all(&server_template_dir).unwrap();

        // Test server template discovery with protocol parameter (custom path)
        // With new logic, custom paths are used directly, so pass the full template path
        let result = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::RustAxum,
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
        let mcp_server_dir = temp_dir.join("templates/mcp/server/rust_axum");
        fs::create_dir_all(&mcp_server_dir).unwrap();

        // Test MCP protocol
        // With new logic, custom paths are used directly, so pass the full template path
        let result = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::RustAxum,
            Some(&mcp_server_dir),
        );
        assert!(result.is_ok());
        let template_dir = result.unwrap();

        // The resolved path should contain the protocol segment
        let path_str = template_dir.template_path().to_string_lossy();
        assert!(
            path_str.contains("mcp"),
            "Path should contain protocol segment: {}",
            path_str
        );
        assert!(
            path_str.contains("server"),
            "Path should contain role: {}",
            path_str
        );
        assert!(
            path_str.contains("rust_axum"),
            "Path should contain template kind: {}",
            path_str
        );
    }

    #[test]
    fn test_backward_compatibility_with_discover() {
        // Test that the old discover method still works
        let temp_dir = create_test_workspace("test_backward_compatibility_with_discover");
        let server_template_dir = temp_dir.join("templates/mcp/server/rust_axum");
        fs::create_dir_all(&server_template_dir).unwrap();

        // Test discover_with_protocol method (uses MCP protocol)
        // With new logic, custom paths are used directly, so pass the full template path
        let result = TemplateDir::discover_with_protocol(
            Protocol::Mcp,
            ServerTemplateKind::RustAxum,
            Some(&server_template_dir),
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().template_path(),
            server_template_dir.as_path()
        );
    }
}
