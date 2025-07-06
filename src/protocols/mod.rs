//! Protocol domain module - handles all protocol-specific logic and validation
//!
//! This module implements the protocol abstraction layer, allowing different
//! communication protocols (MCP, A2A, ANP, ACP) to be plugged in dynamically.

pub mod errors;
pub mod handlers;
pub mod registry;
pub mod traits;
pub mod types;

pub use errors::*;
pub use registry::*;
pub use traits::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_mcp_protocol_capabilities() {
        let capabilities = Protocol::Mcp.capabilities();
        assert_eq!(capabilities.protocol, Protocol::Mcp);
        assert_eq!(
            capabilities.supported_roles,
            vec![Role::Server, Role::Client]
        );
        assert_eq!(capabilities.requires_openapi, true);
        assert_eq!(capabilities.supports_streaming, true);
        assert_eq!(capabilities.supports_bidirectional, true);
    }

    #[test]
    fn test_a2a_protocol_capabilities() {
        let capabilities = Protocol::A2a.capabilities();
        assert_eq!(capabilities.protocol, Protocol::A2a);
        assert_eq!(capabilities.supported_roles, vec![Role::Agent]);
        assert_eq!(capabilities.requires_openapi, false);
    }

    #[test]
    fn test_protocol_role_validation_success() {
        assert!(Protocol::Mcp.validate_role(&Role::Server).is_ok());
        assert!(Protocol::Mcp.validate_role(&Role::Client).is_ok());
        assert!(Protocol::A2a.validate_role(&Role::Agent).is_ok());
        assert!(Protocol::Acp.validate_role(&Role::Broker).is_ok());
    }

    #[test]
    fn test_protocol_role_validation_failure() {
        let result = Protocol::Mcp.validate_role(&Role::Agent);
        assert!(result.is_err());

        if let Err(ProtocolError::UnsupportedRole { protocol, role }) = result {
            assert_eq!(protocol, Protocol::Mcp);
            assert_eq!(role, Role::Agent);
        } else {
            panic!("Expected UnsupportedRole error");
        }
    }

    #[test]
    fn test_protocol_registry_registration() {
        let registry = ProtocolRegistry::new();
        assert!(!registry.is_implemented(Protocol::Mcp));

        let handler = MockProtocolHandler::new(Protocol::Mcp);
        registry.register(Protocol::Mcp, Arc::new(handler));

        assert!(registry.is_implemented(Protocol::Mcp));
        assert!(!registry.is_implemented(Protocol::A2a));
    }

    #[test]
    fn test_protocol_registry_get_handler() {
        let registry = ProtocolRegistry::new();
        let handler = MockProtocolHandler::new(Protocol::Mcp);
        registry.register(Protocol::Mcp, Arc::new(handler));

        let retrieved = registry.get(Protocol::Mcp);
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.expect("Handler should exist").protocol(),
            Protocol::Mcp
        );

        let missing = registry.get(Protocol::A2a);
        assert!(missing.is_none());
    }

    #[test]
    fn test_protocol_registry_with_defaults() {
        let registry =
            ProtocolRegistry::with_defaults().expect("Failed to create registry with defaults");

        // MCP should be pre-registered
        assert!(registry.is_implemented(Protocol::Mcp));

        let handler = registry.get(Protocol::Mcp);
        assert!(handler.is_some());

        let handler = handler.expect("MCP handler should be registered");
        assert_eq!(handler.protocol(), Protocol::Mcp);

        // Other protocols should not be registered
        assert!(!registry.is_implemented(Protocol::A2a));
        assert!(!registry.is_implemented(Protocol::Acp));
        assert!(!registry.is_implemented(Protocol::Anp));
    }

    // Mock implementation for testing
    struct MockProtocolHandler {
        protocol: Protocol,
    }

    impl MockProtocolHandler {
        fn new(protocol: Protocol) -> Self {
            Self { protocol }
        }
    }

    #[async_trait::async_trait]
    impl ProtocolHandler for MockProtocolHandler {
        fn protocol(&self) -> Protocol {
            self.protocol
        }

        async fn prepare_context(
            &self,
            _input: ProtocolInput,
        ) -> Result<crate::generation::GenerationContext, ProtocolError> {
            unimplemented!("Mock implementation")
        }

        fn validate_configuration(&self, _config: &ProtocolConfig) -> Result<(), ProtocolError> {
            Ok(())
        }
    }
}
