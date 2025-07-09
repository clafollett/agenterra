//! Default post-processor implementation

use async_trait::async_trait;
use std::sync::Arc;

use crate::generation::{Artifact, GenerationContext, GenerationError, PostProcessor};
use crate::infrastructure::shell::CommandExecutor;

/// Post-processor that adds file permissions based on file type
pub struct PermissionsPostProcessor;

impl PermissionsPostProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PostProcessor for PermissionsPostProcessor {
    async fn process(
        &self,
        mut artifacts: Vec<Artifact>,
        context: &GenerationContext,
        _post_generation_commands: &[String],
    ) -> Result<Vec<Artifact>, GenerationError> {
        for artifact in &mut artifacts {
            // Make scripts executable - including language-specific executable extensions
            let should_make_executable = artifact
                .path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    // Traditional script extensions
                    ext == "sh" || ext == "py" ||
                    // Language-specific extensions that might be executable
                    ext == context.language.file_extension()
                })
                .unwrap_or(false);

            if should_make_executable {
                artifact.permissions = Some(0o755);
            }

            // Check for shebang
            if artifact.content.starts_with("#!") {
                artifact.permissions = Some(0o755);
            }
        }

        Ok(artifacts)
    }
}

/// Post-processor that executes commands after artifact generation
pub struct CommandPostProcessor {
    executor: Arc<dyn CommandExecutor>,
}

impl CommandPostProcessor {
    pub fn new(executor: Arc<dyn CommandExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl PostProcessor for CommandPostProcessor {
    async fn process(
        &self,
        artifacts: Vec<Artifact>,
        context: &GenerationContext,
        post_generation_commands: &[String],
    ) -> Result<Vec<Artifact>, GenerationError> {
        // Execute post-generation commands in the output directory
        for command in post_generation_commands {
            tracing::info!(
                project_name = %context.metadata.project_name,
                command = %command,
                "Executing post-generation command"
            );

            let result = self
                .executor
                .execute(command, std::path::Path::new("."))
                .await;

            match result {
                Ok(cmd_result) => {
                    if cmd_result.is_success() {
                        tracing::debug!(
                            project_name = %context.metadata.project_name,
                            command = %command,
                            "Post-generation command completed successfully"
                        );

                        // Log command output if present
                        if !cmd_result.stdout.trim().is_empty() {
                            tracing::debug!(
                                project_name = %context.metadata.project_name,
                                command = %command,
                                output = %cmd_result.stdout.trim(),
                                "Post-generation command output"
                            );
                        }
                    } else {
                        tracing::error!(
                            project_name = %context.metadata.project_name,
                            command = %command,
                            exit_code = cmd_result.exit_code,
                            stderr = %cmd_result.stderr,
                            "Post-generation command failed"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        project_name = %context.metadata.project_name,
                        command = %command,
                        error = %e,
                        "Failed to execute post-generation command"
                    );
                }
            }
        }

        Ok(artifacts)
    }
}

/// Composite post-processor that runs multiple processors in sequence
pub struct CompositePostProcessor {
    processors: Vec<Box<dyn PostProcessor>>,
}

impl CompositePostProcessor {
    pub fn new() -> Self {
        // Actually wire up the processors!
        Self {
            processors: vec![
                Box::new(PermissionsPostProcessor::new()),
                Box::new(CommandPostProcessor::new(Arc::new(
                    crate::infrastructure::ShellCommandExecutor::new(),
                ))),
            ],
        }
    }
}

#[async_trait]
impl PostProcessor for CompositePostProcessor {
    async fn process(
        &self,
        mut artifacts: Vec<Artifact>,
        context: &GenerationContext,
        post_generation_commands: &[String],
    ) -> Result<Vec<Artifact>, GenerationError> {
        for processor in &self.processors {
            artifacts = processor
                .process(artifacts, context, post_generation_commands)
                .await?;
        }
        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::Language;
    use crate::infrastructure::shell::MockCommandExecutor;
    use crate::protocols::{Protocol, Role};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_permissions_post_processor() {
        let processor = PermissionsPostProcessor::new();
        let context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);

        let artifacts = vec![
            Artifact {
                path: PathBuf::from("script.sh"),
                content: "#!/bin/bash\necho hello".to_string(),
                permissions: None,
            },
            Artifact {
                path: PathBuf::from("run.py"),
                content: "print('hello')".to_string(),
                permissions: None,
            },
            Artifact {
                path: PathBuf::from("main.rs"),
                content: "fn main() {}".to_string(),
                permissions: None,
            },
            Artifact {
                path: PathBuf::from("README.md"),
                content: "# README".to_string(),
                permissions: None,
            },
        ];

        let result = processor.process(artifacts, &context, &[]).await.unwrap();

        assert_eq!(result[0].permissions, Some(0o755)); // script.sh
        assert_eq!(result[1].permissions, Some(0o755)); // run.py
        assert_eq!(result[2].permissions, Some(0o755)); // main.rs (matches context.language.file_extension())
        assert_eq!(result[3].permissions, None); // README.md
    }

    #[tokio::test]
    async fn test_command_post_processor_success() {
        let mock_executor = MockCommandExecutor::new()
            .with_result("npm install", 0, "packages installed", "")
            .with_result("chmod +x script.sh", 0, "", "");

        let processor = CommandPostProcessor::new(Arc::new(mock_executor));
        let context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::TypeScript);

        let artifacts = vec![
            Artifact {
                path: PathBuf::from("package.json"),
                content: "{}".to_string(),
                permissions: None,
            },
            Artifact {
                path: PathBuf::from("script.sh"),
                content: "#!/bin/bash".to_string(),
                permissions: None,
            },
        ];

        let result = processor.process(artifacts, &context, &[]).await.unwrap();

        // Commands processing completed successfully
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, PathBuf::from("package.json"));
        assert_eq!(result[1].path, PathBuf::from("script.sh"));
    }

    #[tokio::test]
    async fn test_command_post_processor_no_commands() {
        let mock_executor =
            MockCommandExecutor::new().with_result("npm install", 1, "", "npm not found");

        let processor = CommandPostProcessor::new(Arc::new(mock_executor));
        let context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::TypeScript);

        let artifacts = vec![Artifact {
            path: PathBuf::from("package.json"),
            content: "{}".to_string(),
            permissions: None,
        }];

        let result = processor.process(artifacts, &context, &[]).await;

        // Since artifacts no longer have post_commands, processing should succeed
        assert!(result.is_ok());
        let processed_artifacts = result.unwrap();
        assert_eq!(processed_artifacts.len(), 1);
        assert_eq!(processed_artifacts[0].path, PathBuf::from("package.json"));
    }

    #[tokio::test]
    async fn test_composite_post_processor() {
        // CompositePostProcessor is now created with built-in processors
        let composite = CompositePostProcessor::new();

        let context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Python);

        let artifacts = vec![Artifact {
            path: PathBuf::from("script.sh"),
            content: "#!/bin/bash".to_string(),
            permissions: None,
        }];

        let result = composite.process(artifacts, &context, &[]).await.unwrap();

        // Should have permissions set by PermissionsPostProcessor
        assert_eq!(result[0].permissions, Some(0o755));
        // Command processing completed successfully
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("script.sh"));
    }
}
