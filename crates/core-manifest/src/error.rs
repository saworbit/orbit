//! Error types for manifest operations

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Result type for manifest operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during manifest operations
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error occurred
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Manifest validation failed
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Schema version mismatch
    #[error("Schema version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },

    /// Invalid path provided
    #[error("Invalid path: {path}")]
    InvalidPath { path: PathBuf },

    /// Manifest file not found
    #[error("Manifest not found: {path}")]
    ManifestNotFound { path: PathBuf },

    /// Invalid content ID format
    #[error("Invalid content ID: {0}")]
    InvalidContentId(String),

    /// Missing required field
    #[error("Missing required field: {field}")]
    MissingField { field: String },

    /// Invalid endpoint type
    #[error("Invalid endpoint type: {0}")]
    InvalidEndpointType(String),

    /// Invalid chunking configuration
    #[error("Invalid chunking configuration: {0}")]
    InvalidChunking(String),

    /// Invalid policy configuration
    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),

    /// Merkle root mismatch during verification
    #[error("Merkle root mismatch for window {window_id}: expected {expected}, found {found}")]
    MerkleRootMismatch {
        window_id: u32,
        expected: String,
        found: String,
    },

    /// File digest mismatch
    #[error("File digest mismatch for {path}: expected {expected}, found {found}")]
    DigestMismatch {
        path: String,
        expected: String,
        found: String,
    },

    /// Job digest not finalized
    #[error("Job digest not finalized - manifest is incomplete")]
    JobNotFinalized,

    /// Generic error with context
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a validation error with a message
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Error::Validation {
            message: message.into(),
        }
    }

    /// Create a version mismatch error
    pub fn version_mismatch<S: Into<String>>(expected: S, found: S) -> Self {
        Error::VersionMismatch {
            expected: expected.into(),
            found: found.into(),
        }
    }

    /// Create an invalid path error
    pub fn invalid_path<P: Into<PathBuf>>(path: P) -> Self {
        Error::InvalidPath { path: path.into() }
    }

    /// Create a manifest not found error
    pub fn manifest_not_found<P: Into<PathBuf>>(path: P) -> Self {
        Error::ManifestNotFound { path: path.into() }
    }

    /// Create a missing field error
    pub fn missing_field<S: Into<String>>(field: S) -> Self {
        Error::MissingField {
            field: field.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let err = Error::validation("test message");
        assert!(matches!(err, Error::Validation { .. }));
        assert_eq!(err.to_string(), "Validation error: test message");
    }

    #[test]
    fn test_version_mismatch_error() {
        let err = Error::version_mismatch("v1", "v2");
        assert!(matches!(err, Error::VersionMismatch { .. }));
        assert!(err.to_string().contains("expected v1"));
        assert!(err.to_string().contains("found v2"));
    }

    #[test]
    fn test_invalid_path_error() {
        let err = Error::invalid_path("/invalid/path");
        assert!(matches!(err, Error::InvalidPath { .. }));
    }

    #[test]
    fn test_merkle_root_mismatch() {
        let err = Error::MerkleRootMismatch {
            window_id: 5,
            expected: "abc123".to_string(),
            found: "def456".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("window 5"));
        assert!(msg.contains("abc123"));
        assert!(msg.contains("def456"));
    }
}
