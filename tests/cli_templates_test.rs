//! Integration tests for the CLI templates subcommand

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_templates_list_command() {
    let mut cmd = Command::cargo_bin("agenterra").unwrap();

    cmd.arg("templates")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available embedded templates"))
        .stdout(predicate::str::contains("Server Templates"))
        .stdout(predicate::str::contains("mcp/server/rust"))
        .stdout(predicate::str::contains("Client Templates"))
        .stdout(predicate::str::contains("mcp/client/rust"));
}

#[test]
fn test_templates_export_command() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("agenterra").unwrap();

    cmd.arg("templates")
        .arg("export")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Exported"))
        .stdout(predicate::str::contains("templates"));

    // Verify files were actually exported
    assert!(
        temp_dir
            .path()
            .join("mcp/server/rust/manifest.yml")
            .exists()
    );
    assert!(
        temp_dir
            .path()
            .join("mcp/client/rust/manifest.yml")
            .exists()
    );
}

#[test]
fn test_templates_info_command() {
    let mut cmd = Command::cargo_bin("agenterra").unwrap();

    cmd.arg("templates")
        .arg("info")
        .arg("mcp/server/rust")
        .assert()
        .success()
        .stdout(predicate::str::contains("Template: mcp/server/rust"))
        .stdout(predicate::str::contains("Protocol: mcp"))
        .stdout(predicate::str::contains("Role: Server"))
        .stdout(predicate::str::contains("Language: rust"))
        .stdout(predicate::str::contains("Template files:"))
        .stdout(predicate::str::contains("- Cargo.toml.tera -> Cargo.toml"))
        .stdout(predicate::str::contains("- LICENSE -> LICENSE"))
        .stdout(predicate::str::contains("- README.md.tera -> README.md"))
        .stdout(predicate::str::contains(
            "- common.rs.tera -> src/common.rs",
        ))
        .stdout(predicate::str::contains(
            "- config.rs.tera -> src/config.rs",
        ))
        .stdout(predicate::str::contains(
            "- handler.rs.tera -> src/handlers/{endpoint}.rs",
        ))
        .stdout(predicate::str::contains(
            "- handlers_mod.rs.tera -> src/handlers/mod.rs",
        ))
        .stdout(predicate::str::contains("- main.rs.tera -> src/main.rs"))
        .stdout(predicate::str::contains(
            "- server.rs.tera -> src/server.rs",
        ))
        .stdout(predicate::str::contains(
            "- signal.rs.tera -> src/signal.rs",
        ))
        .stdout(predicate::str::contains(
            "- transport.rs.tera -> src/transport.rs",
        ));
}

#[test]
fn test_templates_info_nonexistent() {
    let mut cmd = Command::cargo_bin("agenterra").unwrap();

    cmd.arg("templates")
        .arg("info")
        .arg("nonexistent/template")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Template not found"));
}

#[test]
fn test_templates_export_to_nonexistent_directory() {
    let mut cmd = Command::cargo_bin("agenterra").unwrap();
    let non_existent = "/tmp/test_agenterra_templates_export_12345";

    cmd.arg("templates")
        .arg("export")
        .arg(non_existent)
        .assert()
        .success();

    // Verify directory was created and templates exported
    assert!(std::path::Path::new(non_existent).exists());

    // Clean up
    std::fs::remove_dir_all(non_existent).ok();
}

#[test]
fn test_templates_export_single_template() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("agenterra").unwrap();

    cmd.arg("templates")
        .arg("export")
        .arg(temp_dir.path())
        .arg("--template")
        .arg("mcp/server/rust")
        .assert()
        .success()
        .stdout(predicate::str::contains("Exported"))
        .stdout(predicate::str::contains("template"));

    // Verify only the specified template was exported
    assert!(
        temp_dir
            .path()
            .join("mcp/server/rust/manifest.yml")
            .exists()
    );
    assert!(
        !temp_dir
            .path()
            .join("mcp/client/rust/manifest.yml")
            .exists()
    );
}

#[test]
fn test_templates_export_nonexistent_template() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("agenterra").unwrap();

    cmd.arg("templates")
        .arg("export")
        .arg(temp_dir.path())
        .arg("--template")
        .arg("nonexistent/template")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Template not found"));
}
