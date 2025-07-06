//! Default post-processor implementation

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

use crate::generation::{Artifact, GenerationContext, GenerationError, PostProcessor};
use crate::infrastructure::shell::CommandExecutor;

/// Default post-processor that passes artifacts through unchanged
pub struct DefaultPostProcessor;

impl DefaultPostProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultPostProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PostProcessor for DefaultPostProcessor {
    async fn process(
        &self,
        artifacts: Vec<Artifact>,
        _context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        // Default implementation just passes artifacts through
        Ok(artifacts)
    }
}

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
        _context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        for artifact in &mut artifacts {
            // Make scripts executable
            if artifact
                .path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "sh" || ext == "py")
                .unwrap_or(false)
            {
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
        mut artifacts: Vec<Artifact>,
        context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        // Group artifacts by their working directory
        let mut commands_by_dir: std::collections::HashMap<&Path, Vec<(&str, usize)>> =
            std::collections::HashMap::new();

        for (idx, artifact) in artifacts.iter().enumerate() {
            if !artifact.post_commands.is_empty() {
                let working_dir = artifact.path.parent().unwrap_or(Path::new("."));
                for command in &artifact.post_commands {
                    commands_by_dir
                        .entry(working_dir)
                        .or_insert_with(Vec::new)
                        .push((command.as_str(), idx));
                }
            }
        }

        // Execute commands grouped by directory
        for (working_dir, commands) in commands_by_dir {
            for (command, artifact_idx) in commands {
                let result = self.executor.execute(command, working_dir).await?;

                if !result.is_success() {
                    return Err(GenerationError::PostProcessingError(format!(
                        "Post-command '{}' for artifact '{}' in project '{}' ({:?}/{:?}/{:?}) failed with exit code {}: {}",
                        command,
                        artifacts[artifact_idx].path.display(),
                        context.metadata.project_name,
                        context.protocol,
                        context.role,
                        context.language,
                        result.exit_code,
                        result.stderr
                    )));
                }

                // Log command output if present
                if !result.stdout.trim().is_empty() {
                    tracing::debug!(
                        project_name = %context.metadata.project_name,
                        command = %command,
                        output = %result.stdout.trim(),
                        "Post-command output"
                    );
                }
            }
        }

        // Clear post_commands after execution to prevent re-execution
        for artifact in &mut artifacts {
            artifact.post_commands.clear();
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
        Self { processors: vec![] }
    }

    pub fn add_processor(mut self, processor: Box<dyn PostProcessor>) -> Self {
        self.processors.push(processor);
        self
    }
}

#[async_trait]
impl PostProcessor for CompositePostProcessor {
    async fn process(
        &self,
        mut artifacts: Vec<Artifact>,
        context: &GenerationContext,
    ) -> Result<Vec<Artifact>, GenerationError> {
        for processor in &self.processors {
            artifacts = processor.process(artifacts, context).await?;
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
    async fn test_default_post_processor() {
        let processor = DefaultPostProcessor::new();
        let context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);

        let artifacts = vec![Artifact {
            path: PathBuf::from("test.txt"),
            content: "content".to_string(),
            permissions: None,
            post_commands: vec![],
        }];

        let result = processor
            .process(artifacts.clone(), &context)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, artifacts[0].content);
    }

    #[tokio::test]
    async fn test_permissions_post_processor() {
        let processor = PermissionsPostProcessor::new();
        let context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Rust);

        let artifacts = vec![
            Artifact {
                path: PathBuf::from("script.sh"),
                content: "#!/bin/bash\necho hello".to_string(),
                permissions: None,
                post_commands: vec![],
            },
            Artifact {
                path: PathBuf::from("run.py"),
                content: "print('hello')".to_string(),
                permissions: None,
                post_commands: vec![],
            },
            Artifact {
                path: PathBuf::from("README.md"),
                content: "# README".to_string(),
                permissions: None,
                post_commands: vec![],
            },
        ];

        let result = processor.process(artifacts, &context).await.unwrap();

        assert_eq!(result[0].permissions, Some(0o755)); // script.sh
        assert_eq!(result[1].permissions, Some(0o755)); // run.py
        assert_eq!(result[2].permissions, None); // README.md
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
                post_commands: vec!["npm install".to_string()],
            },
            Artifact {
                path: PathBuf::from("script.sh"),
                content: "#!/bin/bash".to_string(),
                permissions: None,
                post_commands: vec!["chmod +x script.sh".to_string()],
            },
        ];

        let result = processor.process(artifacts, &context).await.unwrap();

        // Commands should be cleared after execution
        assert!(result[0].post_commands.is_empty());
        assert!(result[1].post_commands.is_empty());
    }

    #[tokio::test]
    async fn test_command_post_processor_failure() {
        let mock_executor =
            MockCommandExecutor::new().with_result("npm install", 1, "", "npm not found");

        let processor = CommandPostProcessor::new(Arc::new(mock_executor));
        let context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::TypeScript);

        let artifacts = vec![Artifact {
            path: PathBuf::from("package.json"),
            content: "{}".to_string(),
            permissions: None,
            post_commands: vec!["npm install".to_string()],
        }];

        let result = processor.process(artifacts, &context).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            GenerationError::PostProcessingError(msg) => {
                assert!(msg.contains("npm install"));
                assert!(msg.contains("failed"));
                assert!(msg.contains("npm not found"));
            }
            _ => panic!("Expected PostProcessingError"),
        }
    }

    #[tokio::test]
    async fn test_composite_post_processor() {
        let permissions_processor = Box::new(PermissionsPostProcessor::new());

        let mock_executor = MockCommandExecutor::new().with_result("echo done", 0, "done", "");
        let command_processor = Box::new(CommandPostProcessor::new(Arc::new(mock_executor)));

        let composite = CompositePostProcessor::new()
            .add_processor(permissions_processor)
            .add_processor(command_processor);

        let context = GenerationContext::new(Protocol::Mcp, Role::Server, Language::Python);

        let artifacts = vec![Artifact {
            path: PathBuf::from("script.sh"),
            content: "#!/bin/bash".to_string(),
            permissions: None,
            post_commands: vec!["echo done".to_string()],
        }];

        let result = composite.process(artifacts, &context).await.unwrap();

        // Should have permissions set by PermissionsPostProcessor
        assert_eq!(result[0].permissions, Some(0o755));
        // Should have commands cleared by CommandPostProcessor
        assert!(result[0].post_commands.is_empty());
    }
}
