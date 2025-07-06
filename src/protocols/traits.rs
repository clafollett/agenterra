//! Protocol behavior traits and interfaces

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::path::PathBuf;

/// Input data for protocol processing
#[derive(Debug, Clone)]
pub struct ProtocolInput {
    pub openapi_path: Option<PathBuf>,
    pub openapi_spec: Option<crate::generation::OpenApiSpec>,
    pub config: ProtocolConfig,
    pub role: crate::protocols::Role,
    pub language: crate::generation::Language,
}

/// Configuration for protocol behavior
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    pub project_name: String,
    pub output_dir: PathBuf,
    pub version: Option<String>,
    pub options: std::collections::HashMap<String, JsonValue>,
}

/// Core protocol handler trait
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// Returns the protocol this handler implements
    fn protocol(&self) -> crate::protocols::Protocol;

    /// Prepares generation context from input
    async fn prepare_context(
        &self,
        input: ProtocolInput,
    ) -> Result<crate::generation::GenerationContext, crate::protocols::ProtocolError>;

    /// Validates protocol-specific configuration
    fn validate_configuration(
        &self,
        config: &ProtocolConfig,
    ) -> Result<(), crate::protocols::ProtocolError>;

    /// Returns supported features for this protocol implementation
    fn supported_features(&self) -> Vec<String> {
        vec![]
    }
}

/// Factory trait for creating protocol handlers
pub trait ProtocolHandlerFactory: Send + Sync {
    /// Creates a new instance of the protocol handler
    fn create(&self) -> Result<Box<dyn ProtocolHandler>, crate::protocols::ProtocolError>;

    /// Returns the protocol this factory creates handlers for
    fn protocol(&self) -> crate::protocols::Protocol;
}
