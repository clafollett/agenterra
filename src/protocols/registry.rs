//! Protocol registry for dynamic protocol handler registration

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct ProtocolRegistry {
    handlers: Arc<
        RwLock<HashMap<crate::protocols::Protocol, Arc<dyn crate::protocols::ProtocolHandler>>>,
    >,
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new registry with default protocol handlers registered
    pub fn with_defaults() -> Result<Self, crate::protocols::ProtocolError> {
        let registry = Self::new();

        // Register MCP handler
        registry.register(
            crate::protocols::Protocol::Mcp,
            Arc::new(crate::protocols::handlers::mcp::McpProtocolHandler::new()),
        )?;

        Ok(registry)
    }

    pub fn register(
        &self,
        protocol: crate::protocols::Protocol,
        handler: Arc<dyn crate::protocols::ProtocolHandler>,
    ) -> Result<(), crate::protocols::ProtocolError> {
        self.handlers
            .write()
            .map_err(|_| {
                crate::protocols::ProtocolError::InternalError(
                    "Failed to acquire write lock".to_string(),
                )
            })?
            .insert(protocol, handler);
        Ok(())
    }

    pub fn get(
        &self,
        protocol: crate::protocols::Protocol,
    ) -> Option<Arc<dyn crate::protocols::ProtocolHandler>> {
        self.handlers.read().ok()?.get(&protocol).cloned()
    }

    pub fn list(&self) -> Vec<crate::protocols::Protocol> {
        self.handlers
            .read()
            .ok()
            .map(|guard| guard.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn is_implemented(&self, protocol: crate::protocols::Protocol) -> bool {
        self.handlers
            .read()
            .ok()
            .map(|guard| guard.contains_key(&protocol))
            .unwrap_or(false)
    }
}
