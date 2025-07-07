//! Core protocol types and enums

use crate::protocols::ProtocolError;

use serde::{Deserialize, Serialize};
use std::{fmt::Formatter, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    A2a,
    Acp,
    Anp,
    Mcp,
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

    pub fn validate_role(&self, role: &Role) -> Result<(), ProtocolError> {
        let capabilities = self.capabilities();
        if capabilities.supported_roles.contains(role) {
            Ok(())
        } else {
            Err(ProtocolError::UnsupportedRole {
                protocol: *self,
                role: role.clone(),
            })
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::A2a => write!(f, "a2a"),
            Protocol::Acp => write!(f, "acp"),
            Protocol::Anp => write!(f, "anp"),
            Protocol::Mcp => write!(f, "mcp"),
        }
    }
}

impl std::str::FromStr for Protocol {
    type Err = ProtocolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "a2a" => Ok(Protocol::A2a),
            "acp" => Ok(Protocol::Acp),
            "anp" => Ok(Protocol::Anp),
            "mcp" => Ok(Protocol::Mcp),
            _ => Err(ProtocolError::InvalidConfiguration(format!(
                "Unknown protocol: {s}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolCapabilities {
    pub protocol: Protocol,
    pub supported_roles: Vec<Role>,
    pub requires_openapi: bool,
    pub supports_streaming: bool,
    pub supports_bidirectional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Agent,
    Broker,
    Client,
    Server,
    Custom(String),
}

impl Role {}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Agent => write!(f, "agent"),
            Role::Broker => write!(f, "broker"),
            Role::Client => write!(f, "client"),
            Role::Server => write!(f, "server"),
            Role::Custom(name) => write!(f, "{name}"),
        }
    }
}

impl FromStr for Role {
    type Err = ProtocolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "agent" => Ok(Role::Agent),
            "broker" => Ok(Role::Broker),
            "client" => Ok(Role::Client),
            "server" => Ok(Role::Server),
            other => Err(ProtocolError::InvalidConfiguration(format!(
                "Unknown role: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {}
