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

        // TODO: Register A2A, ACP, ANP protocol handlers when implemented
        // Currently only MCP is supported
        // - A2A (Agent to Agent) - Google
        // - ACP (Agent Communication Protocol) - IBM
        // - ANP (Agent Network Protocol) - Cisco

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let registry = ProtocolRegistry::new();
        let handler = Arc::new(crate::protocols::handlers::mcp::McpProtocolHandler::new());

        // Register handler
        registry
            .register(crate::protocols::Protocol::Mcp, handler.clone())
            .unwrap();

        // Get handler back
        let retrieved = registry.get(crate::protocols::Protocol::Mcp);
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().protocol(),
            crate::protocols::Protocol::Mcp
        );
    }

    #[test]
    fn test_with_defaults() {
        let registry = ProtocolRegistry::with_defaults().unwrap();

        // Should have MCP registered
        assert!(registry.get(crate::protocols::Protocol::Mcp).is_some());

        // Should not have others registered
        assert!(registry.get(crate::protocols::Protocol::A2a).is_none());
        assert!(registry.get(crate::protocols::Protocol::Acp).is_none());
        assert!(registry.get(crate::protocols::Protocol::Anp).is_none());
    }
}
