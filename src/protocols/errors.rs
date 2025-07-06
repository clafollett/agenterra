//! Protocol-specific error types

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Unsupported role {role:?} for protocol {protocol:?}")]
    UnsupportedRole {
        protocol: crate::protocols::Protocol,
        role: crate::protocols::Role,
    },

    #[error("Protocol {0:?} not implemented")]
    NotImplemented(crate::protocols::Protocol),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
