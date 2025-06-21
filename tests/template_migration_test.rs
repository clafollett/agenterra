//! Template Migration Tests - TDD Red Phase
//!
//! These tests define the contract that our golden reference → template migration must fulfill.
//! Following strict Red/Green/Refactor TDD methodology.

use anyhow::Result;
use std::path::PathBuf;
use std::fs;

/// Test that all golden reference files are migrated to templates
#[test]
fn test_all_golden_reference_files_migrated_to_templates() {
    let golden_reference_path = PathBuf::from("src/mcp/client");
    let template_path = PathBuf::from("templates/mcp/client/rust_reqwest/src");
    
    // RED: This will fail because we haven't migrated all files yet
    let golden_files = get_golden_reference_files(&golden_reference_path).unwrap();
    let template_files = get_template_files(&template_path).unwrap();
    
    // Map golden reference files to expected template files
    let expected_template_files = map_golden_to_template_names(&golden_files);
    
    // Assert all expected template files exist
    for expected_file in &expected_template_files {
        assert!(
            template_files.contains(expected_file),
            "Missing template file: {} (should be migrated from golden reference)",
            expected_file
        );
    }
    
    // Assert template files contain all golden reference functionality
    for (golden_file, template_file) in golden_files.iter().zip(expected_template_files.iter()) {
        assert_template_contains_golden_functionality(golden_file, template_file).unwrap();
    }
}

/// Test that critical domain structures are preserved in templates
#[test]
fn test_domain_structures_preserved_in_templates() {
    // RED: Will fail until we migrate auth.rs
    assert_template_has_struct("templates/mcp/client/rust_reqwest/src/auth.rs.tera", "SecureCredential");
    assert_template_has_struct("templates/mcp/client/rust_reqwest/src/auth.rs.tera", "AuthConfig");
    
    // RED: Will fail until we migrate registry.rs  
    assert_template_has_struct("templates/mcp/client/rust_reqwest/src/registry.rs.tera", "ToolRegistry");
    assert_template_has_struct("templates/mcp/client/rust_reqwest/src/registry.rs.tera", "ToolInfo");
    
    // RED: Will fail until we migrate session_manager.rs
    assert_template_has_struct("templates/mcp/client/rust_reqwest/src/session_manager.rs.tera", "McpSessionManager");
    
    // RED: Will fail until we migrate transport.rs
    assert_template_has_struct("templates/mcp/client/rust_reqwest/src/transport.rs.tera", "Transport");
    
    // RED: Will fail until we migrate result.rs
    assert_template_has_struct("templates/mcp/client/rust_reqwest/src/result.rs.tera", "ToolResult");
}

/// Test that Tera templating tokens are properly added
#[test] 
fn test_tera_tokens_added_for_customization() {
    let template_files = [
        "templates/mcp/client/rust_reqwest/src/auth.rs.tera",
        "templates/mcp/client/rust_reqwest/src/registry.rs.tera",
        "templates/mcp/client/rust_reqwest/src/session_manager.rs.tera",
        "templates/mcp/client/rust_reqwest/src/transport.rs.tera",
        "templates/mcp/client/rust_reqwest/src/result.rs.tera",
    ];
    
    for template_file in &template_files {
        // RED: Will fail until we add Tera tokens
        assert_template_has_tera_token(template_file, "project_name");
        assert_template_has_tera_token(template_file, "author");
        assert_template_has_tera_token(template_file, "version");
    }
}

/// Test that dependencies are included in Cargo.toml.tera
#[test]
fn test_cargo_toml_includes_golden_reference_dependencies() {
    let cargo_toml_path = "templates/mcp/client/rust_reqwest/Cargo.toml.tera";
    
    // RED: Will fail until we add these dependencies
    assert_cargo_toml_has_dependency(cargo_toml_path, "zeroize");     // For secure auth
    assert_cargo_toml_has_dependency(cargo_toml_path, "once_cell");   // For session manager
    assert_cargo_toml_has_dependency(cargo_toml_path, "clap");        // For CLI parsing
}

/// Test that template structure matches golden reference architecture
#[test]
fn test_template_structure_matches_golden_reference() {
    // RED: Will fail until we create proper lib.rs.tera
    assert_template_exists("templates/mcp/client/rust_reqwest/src/lib.rs.tera");
    
    // RED: Will fail until we enhance main.rs.tera with CLI routing
    assert_template_contains_function("templates/mcp/client/rust_reqwest/src/main.rs.tera", "main");
    
    // RED: Will fail until we create cli.rs.tera
    assert_template_exists("templates/mcp/client/rust_reqwest/src/cli.rs.tera");
    
    // RED: Will fail until we create headless.rs.tera
    assert_template_exists("templates/mcp/client/rust_reqwest/src/headless.rs.tera");
    
    // RED: Will fail until we create config.rs.tera
    assert_template_exists("templates/mcp/client/rust_reqwest/src/config.rs.tera");
}

/// Test that DDD principles are preserved in templates
#[test]
fn test_ddd_principles_preserved() {
    // Value Objects remain immutable
    assert_template_struct_is_immutable("templates/mcp/client/rust_reqwest/src/auth.rs.tera", "SecureCredential");
    
    // Entities manage their own state
    assert_template_has_state_management("templates/mcp/client/rust_reqwest/src/client.rs.tera", "McpClient");
    
    // Domain services are stateless
    assert_template_service_is_stateless("templates/mcp/client/rust_reqwest/src/session_manager.rs.tera", "McpSessionManager");
}

// ========================================
// Helper Functions (Will be implemented after RED tests fail)
// ========================================

fn get_golden_reference_files(path: &PathBuf) -> Result<Vec<String>> {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until we implement golden reference file discovery")
}

fn get_template_files(path: &PathBuf) -> Result<Vec<String>> {
    // TODO: Implement during GREEN phase  
    todo!("RED PHASE: This will fail until we implement template file discovery")
}

fn map_golden_to_template_names(golden_files: &[String]) -> Vec<String> {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until we implement name mapping")
}

fn assert_template_contains_golden_functionality(golden_file: &str, template_file: &str) -> Result<()> {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until we implement functionality comparison")
}

fn assert_template_has_struct(template_file: &str, struct_name: &str) {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until template files exist with proper structs")
}

fn assert_template_has_tera_token(template_file: &str, token_name: &str) {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until we add Tera tokens")
}

fn assert_cargo_toml_has_dependency(cargo_toml_path: &str, dependency: &str) {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until we add dependencies")
}

fn assert_template_exists(template_path: &str) {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until template files are created")
}

fn assert_template_contains_function(template_file: &str, function_name: &str) {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until we implement functions")
}

fn assert_template_struct_is_immutable(template_file: &str, struct_name: &str) {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until we preserve DDD value object patterns")
}

fn assert_template_has_state_management(template_file: &str, entity_name: &str) {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until we preserve DDD entity patterns")
}

fn assert_template_service_is_stateless(template_file: &str, service_name: &str) {
    // TODO: Implement during GREEN phase
    todo!("RED PHASE: This will fail until we preserve DDD service patterns")
}