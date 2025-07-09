//! Protocol-specific error types

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Unsupported role {role:?} for protocol {protocol:?}")]
    UnsupportedRole {
        protocol: crate::protocols::Protocol,
        role: crate::protocols::Role,
    },

    /// Used for A2A, ACP, ANP protocols (future implementation)
    #[error("Protocol {0:?} not implemented")]
    NotImplemented(crate::protocols::Protocol),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
