//! Command executor for running shell commands
//!
//! This module provides infrastructure for executing shell commands
//! as part of the code generation post-processing pipeline.

use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use crate::generation::GenerationError;

/// Trait for executing shell commands
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Execute a shell command in the given working directory
    async fn execute(
        &self,
        command: &str,
        working_dir: &Path,
    ) -> Result<CommandResult, GenerationError>;

    /// Execute multiple commands in sequence
    async fn execute_all(
        &self,
        commands: &[String],
        working_dir: &Path,
    ) -> Result<Vec<CommandResult>, GenerationError>;
}

/// Result of command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    /// Check if the command was successful
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Default command executor using tokio::process
pub struct ShellCommandExecutor;

impl ShellCommandExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShellCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandExecutor for ShellCommandExecutor {
    async fn execute(
        &self,
        command: &str,
        working_dir: &Path,
    ) -> Result<CommandResult, GenerationError> {
        let shell = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        };

        let shell_arg = if cfg!(target_os = "windows") {
            "/C"
        } else {
            "-c"
        };

        let output = Command::new(shell)
            .arg(shell_arg)
            .arg(command)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                GenerationError::ProcessError(format!(
                    "Failed to execute command '{}': {}",
                    command, e
                ))
            })?;

        Ok(CommandResult {
            command: command.to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn execute_all(
        &self,
        commands: &[String],
        working_dir: &Path,
    ) -> Result<Vec<CommandResult>, GenerationError> {
        let mut results = Vec::new();

        for command in commands {
            let result = self.execute(command, working_dir).await?;

            // Stop on first failure
            if !result.is_success() {
                return Err(GenerationError::ProcessError(format!(
                    "Command '{}' failed with exit code {}: {}",
                    command, result.exit_code, result.stderr
                )));
            }

            results.push(result);
        }

        Ok(results)
    }
}

/// Mock command executor for testing
#[cfg(test)]
pub struct MockCommandExecutor {
    pub results: std::collections::HashMap<String, CommandResult>,
}

#[cfg(test)]
impl MockCommandExecutor {
    pub fn new() -> Self {
        Self {
            results: std::collections::HashMap::new(),
        }
    }

    pub fn with_result(
        mut self,
        command: &str,
        exit_code: i32,
        stdout: &str,
        stderr: &str,
    ) -> Self {
        self.results.insert(
            command.to_string(),
            CommandResult {
                command: command.to_string(),
                exit_code,
                stdout: stdout.to_string(),
                stderr: stderr.to_string(),
            },
        );
        self
    }
}

#[cfg(test)]
#[async_trait]
impl CommandExecutor for MockCommandExecutor {
    async fn execute(
        &self,
        command: &str,
        _working_dir: &Path,
    ) -> Result<CommandResult, GenerationError> {
        self.results.get(command).cloned().ok_or_else(|| {
            GenerationError::ProcessError(format!(
                "Mock executor has no result for command: {}",
                command
            ))
        })
    }

    async fn execute_all(
        &self,
        commands: &[String],
        working_dir: &Path,
    ) -> Result<Vec<CommandResult>, GenerationError> {
        let mut results = Vec::new();

        for command in commands {
            let result = self.execute(command, working_dir).await?;
            if !result.is_success() {
                return Err(GenerationError::ProcessError(format!(
                    "Command '{}' failed",
                    command
                )));
            }
            results.push(result);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_shell_command_executor_success() {
        let executor = ShellCommandExecutor::new();
        let dir = tempdir().unwrap();

        // Test a simple echo command
        let result = executor.execute("echo hello", dir.path()).await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
        assert!(result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_shell_command_executor_failure() {
        let executor = ShellCommandExecutor::new();
        let dir = tempdir().unwrap();

        // Test a command that should fail
        let result = executor.execute("exit 1", dir.path()).await.unwrap();

        assert!(!result.is_success());
        assert_eq!(result.exit_code, 1);
    }

    #[tokio::test]
    async fn test_execute_all_success() {
        let executor = ShellCommandExecutor::new();
        let dir = tempdir().unwrap();

        let commands = vec!["echo first".to_string(), "echo second".to_string()];

        let results = executor.execute_all(&commands, dir.path()).await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].stdout.contains("first"));
        assert!(results[1].stdout.contains("second"));
    }

    #[tokio::test]
    async fn test_execute_all_stops_on_failure() {
        let executor = ShellCommandExecutor::new();
        let dir = tempdir().unwrap();

        let commands = vec![
            "echo first".to_string(),
            "exit 1".to_string(),
            "echo third".to_string(), // Should not be executed
        ];

        let result = executor.execute_all(&commands, dir.path()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            GenerationError::ProcessError(msg) => {
                assert!(msg.contains("exit 1"));
                assert!(msg.contains("failed"));
            }
            _ => panic!("Expected ProcessError, got: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_mock_command_executor() {
        let executor = MockCommandExecutor::new()
            .with_result("test1", 0, "output1", "")
            .with_result("test2", 1, "", "error");

        let dir = tempdir().unwrap();

        let result1 = executor.execute("test1", dir.path()).await.unwrap();
        assert!(result1.is_success());
        assert_eq!(result1.stdout, "output1");

        let result2 = executor.execute("test2", dir.path()).await.unwrap();
        assert!(!result2.is_success());
        assert_eq!(result2.stderr, "error");
    }
}
