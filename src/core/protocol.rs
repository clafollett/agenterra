//! Protocol abstraction for Agenterra
//!
//! This module defines the Protocol trait and implementations for different
//! communication protocols supported by Agenterra. Each protocol defines
//! how templates are organized and how code generation behaves.

use std::fmt::{self, Display};
use std::str::FromStr;

/// Supported protocols for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// Model Context Protocol (MCP)
    Mcp,
}

impl Protocol {
    /// Get the string identifier for this protocol
    pub fn name(self) -> &'static str {
        match self {
            Protocol::Mcp => "mcp",
        }
    }

    /// Get the path segment used in template directory structure
    pub fn path_segment(self) -> &'static str {
        match self {
            Protocol::Mcp => "mcp",
        }
    }

    /// Get all available protocols
    pub fn all() -> &'static [Protocol] {
        &[Protocol::Mcp]
    }
}

impl Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for Protocol {
    type Err = ProtocolParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mcp" => Ok(Protocol::Mcp),
            _ => Err(ProtocolParseError::Unknown(s.to_string())),
        }
    }
}

/// Error type for protocol parsing
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolParseError {
    Unknown(String),
}

impl Display for ProtocolParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolParseError::Unknown(protocol) => {
                write!(
                    f,
                    "Unknown protocol: '{}'. Available protocols: {}",
                    protocol,
                    Protocol::all()
                        .iter()
                        .map(|p| p.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl std::error::Error for ProtocolParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_name() {
        assert_eq!(Protocol::Mcp.name(), "mcp");
    }

    #[test]
    fn test_protocol_path_segment() {
        assert_eq!(Protocol::Mcp.path_segment(), "mcp");
    }

    #[test]
    fn test_protocol_display() {
        assert_eq!(format!("{}", Protocol::Mcp), "mcp");
    }

    #[test]
    fn test_protocol_from_str() {
        assert_eq!("mcp".parse::<Protocol>().unwrap(), Protocol::Mcp);
        assert_eq!("MCP".parse::<Protocol>().unwrap(), Protocol::Mcp);
    }

    #[test]
    fn test_protocol_from_str_invalid() {
        let result = "invalid".parse::<Protocol>();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ProtocolParseError::Unknown("invalid".to_string())
        );
    }

    #[test]
    fn test_protocol_all() {
        let protocols = Protocol::all();
        assert_eq!(protocols.len(), 1);
        assert!(protocols.contains(&Protocol::Mcp));
    }

    #[test]
    fn test_protocol_parse_error_display() {
        let error = ProtocolParseError::Unknown("test".to_string());
        let message = format!("{}", error);
        assert!(message.contains("Unknown protocol: 'test'"));
        assert!(message.contains("Available protocols: mcp"));
    }
}
