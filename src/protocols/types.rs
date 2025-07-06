//! Core protocol types and enums

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    Mcp,
    A2a,
    Acp,
    Anp,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Server,
    Client,
    Agent,
    Broker,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolCapabilities {
    pub protocol: Protocol,
    pub supported_roles: Vec<Role>,
    pub requires_openapi: bool,
    pub supports_streaming: bool,
    pub supports_bidirectional: bool,
}

impl Protocol {
    pub fn capabilities(&self) -> ProtocolCapabilities {
        match self {
            Protocol::Mcp => ProtocolCapabilities {
                protocol: Protocol::Mcp,
                supported_roles: vec![Role::Server, Role::Client],
                requires_openapi: true,
                supports_streaming: true,
                supports_bidirectional: true,
            },
            Protocol::A2a => ProtocolCapabilities {
                protocol: Protocol::A2a,
                supported_roles: vec![Role::Agent],
                requires_openapi: false,
                supports_streaming: true,
                supports_bidirectional: true,
            },
            Protocol::Acp => ProtocolCapabilities {
                protocol: Protocol::Acp,
                supported_roles: vec![Role::Server, Role::Client, Role::Broker],
                requires_openapi: false,
                supports_streaming: true,
                supports_bidirectional: false,
            },
            Protocol::Anp => ProtocolCapabilities {
                protocol: Protocol::Anp,
                supported_roles: vec![Role::Agent],
                requires_openapi: false,
                supports_streaming: false,
                supports_bidirectional: false,
            },
        }
    }

    pub fn validate_role(&self, role: &Role) -> Result<(), crate::protocols::ProtocolError> {
        let capabilities = self.capabilities();
        if capabilities.supported_roles.contains(role) {
            Ok(())
        } else {
            Err(crate::protocols::ProtocolError::UnsupportedRole {
                protocol: *self,
                role: role.clone(),
            })
        }
    }

    /// Returns all available protocols
    pub fn all() -> Vec<Protocol> {
        vec![Protocol::Mcp, Protocol::A2a, Protocol::Acp, Protocol::Anp]
    }

    /// Returns the protocol identifier as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Mcp => "mcp",
            Protocol::A2a => "a2a",
            Protocol::Acp => "acp",
            Protocol::Anp => "anp",
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Protocol {
    type Err = crate::protocols::ProtocolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mcp" => Ok(Protocol::Mcp),
            "a2a" => Ok(Protocol::A2a),
            "acp" => Ok(Protocol::Acp),
            "anp" => Ok(Protocol::Anp),
            _ => Err(crate::protocols::ProtocolError::InvalidConfiguration(
                format!("Unknown protocol: {}", s),
            )),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Server => write!(f, "server"),
            Role::Client => write!(f, "client"),
            Role::Agent => write!(f, "agent"),
            Role::Broker => write!(f, "broker"),
            Role::Custom(name) => write!(f, "{}", name),
        }
    }
}
