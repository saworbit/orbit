//! Error types for the orbit-connect crate

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectError {
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("gRPC status error: {0}")]
    Status(#[from] tonic::Status),

    #[error("Invalid session metadata: {0}")]
    InvalidMetadata(#[from] tonic::metadata::errors::InvalidMetadataValue),

    #[error("Invalid hash length received from Star (expected 32 bytes, got {0})")]
    InvalidHashLength(usize),

    #[error("Star not found: {0}")]
    StarNotFound(String),

    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("Connection to Star {star_id} failed: {reason}")]
    ConnectionFailed { star_id: String, reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<ConnectError> for orbit_core_interface::OrbitSystemError {
    fn from(err: ConnectError) -> Self {
        match err {
            ConnectError::Io(e) => orbit_core_interface::OrbitSystemError::Io(e),
            other => orbit_core_interface::OrbitSystemError::System(other.to_string()),
        }
    }
}
