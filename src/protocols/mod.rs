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
        assert!(capabilities.requires_openapi);
        assert!(capabilities.supports_streaming);
        assert!(capabilities.supports_bidirectional);
    }

    #[test]
    fn test_a2a_protocol_capabilities() {
        let capabilities = Protocol::A2a.capabilities();
        assert_eq!(capabilities.protocol, Protocol::A2a);
        assert_eq!(capabilities.supported_roles, vec![Role::Agent]);
        assert!(!capabilities.requires_openapi);
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
        assert!(registry.get(Protocol::Mcp).is_none());

        let handler = MockProtocolHandler::new(Protocol::Mcp);
        registry
            .register(Protocol::Mcp, Arc::new(handler))
            .expect("Failed to register handler");

        assert!(registry.get(Protocol::Mcp).is_some());
        assert!(registry.get(Protocol::A2a).is_none());
    }

    #[test]
    fn test_protocol_registry_get_handler() {
        let registry = ProtocolRegistry::new();
        let handler = MockProtocolHandler::new(Protocol::Mcp);
        registry
            .register(Protocol::Mcp, Arc::new(handler))
            .expect("Failed to register handler");

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
        assert!(registry.get(Protocol::Mcp).is_some());

        let handler = registry.get(Protocol::Mcp);
        assert!(handler.is_some());

        let handler = handler.expect("MCP handler should be registered");
        assert_eq!(handler.protocol(), Protocol::Mcp);

        // Other protocols should not be registered
        assert!(registry.get(Protocol::A2a).is_none());
        assert!(registry.get(Protocol::Acp).is_none());
        assert!(registry.get(Protocol::Anp).is_none());
    }

    #[test]
    fn test_mock_protocol_handler_validation() {
        let handler = MockProtocolHandler::new(Protocol::Mcp);

        // Test valid configuration
        let valid_config = ProtocolConfig {
            project_name: "test-project".to_string(),
            version: Some("1.0.0".to_string()),
            options: std::collections::HashMap::new(),
        };

        assert!(handler.validate_configuration(&valid_config).is_ok());
        assert_eq!(handler.validation_call_count(), 1);

        // Test invalid configuration (empty project name)
        let invalid_config = ProtocolConfig {
            project_name: "".to_string(),
            version: None,
            options: std::collections::HashMap::new(),
        };

        let result = handler.validate_configuration(&invalid_config);
        assert!(result.is_err());
        match result {
            Err(ProtocolError::InvalidConfiguration(msg)) => {
                assert_eq!(msg, "Project name cannot be empty");
            }
            _ => panic!("Expected invalid configuration error"),
        }
        assert_eq!(handler.validation_call_count(), 2);
    }

    // Mock implementation for testing
    struct MockProtocolHandler {
        protocol: Protocol,
        validation_calls: std::sync::Mutex<Vec<ProtocolConfig>>,
    }

    impl MockProtocolHandler {
        fn new(protocol: Protocol) -> Self {
            Self {
                protocol,
                validation_calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn validation_call_count(&self) -> usize {
            self.validation_calls.lock().unwrap().len()
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
            // This is intentionally unimplemented as these tests focus on registry behavior
            // not context preparation
            unimplemented!("Mock implementation - not used in registry tests")
        }

        fn validate_configuration(&self, config: &ProtocolConfig) -> Result<(), ProtocolError> {
            // Track that validation was called
            self.validation_calls.lock().unwrap().push(config.clone());

            // Perform basic validation to make this test meaningful
            if config.project_name.is_empty() {
                return Err(ProtocolError::InvalidConfiguration(
                    "Project name cannot be empty".to_string(),
                ));
            }

            // Validate protocol-specific requirements
            match self.protocol {
                Protocol::Mcp => {
                    // MCP-specific validation could go here
                    Ok(())
                }
                _ => Err(ProtocolError::NotImplemented(self.protocol)),
            }
        }
    }
}
