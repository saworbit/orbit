//! Error types for audit operations

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Result type for audit operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during audit operations
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error occurred
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid log path
    #[error("Invalid log path: {path}")]
    InvalidPath { path: PathBuf },

    /// Failed to create log file
    #[error("Failed to create log file: {path}")]
    CreateFailed { path: PathBuf },

    /// Failed to append to log
    #[error("Failed to append to log: {0}")]
    AppendFailed(String),

    /// Log entry is invalid or corrupted
    #[error("Invalid log entry at line {line}: {reason}")]
    InvalidEntry { line: usize, reason: String },

    /// Missing required field in event
    #[error("Missing required field: {field}")]
    MissingField { field: String },

    /// Invalid event type
    #[error("Invalid event type: {0}")]
    InvalidEventType(String),

    /// Beacon signature error
    #[error("Beacon signature error: {0}")]
    SignatureError(String),

    /// Beacon not finalized
    #[error("Beacon not finalized - missing required fields")]
    BeaconNotFinalized,

    /// Generic error with context
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create an invalid path error
    pub fn invalid_path<P: Into<PathBuf>>(path: P) -> Self {
        Error::InvalidPath { path: path.into() }
    }

    /// Create a create failed error
    pub fn create_failed<P: Into<PathBuf>>(path: P) -> Self {
        Error::CreateFailed { path: path.into() }
    }

    /// Create an append failed error
    pub fn append_failed<S: Into<String>>(message: S) -> Self {
        Error::AppendFailed(message.into())
    }

    /// Create an invalid entry error
    pub fn invalid_entry(line: usize, reason: &str) -> Self {
        Error::InvalidEntry {
            line,
            reason: reason.to_string(),
        }
    }

    /// Create a missing field error
    pub fn missing_field<S: Into<String>>(field: S) -> Self {
        Error::MissingField {
            field: field.into(),
        }
    }

    /// Create an invalid event type error
    pub fn invalid_event_type<S: Into<String>>(event_type: S) -> Self {
        Error::InvalidEventType(event_type.into())
    }

    /// Create a signature error
    pub fn signature_error<S: Into<String>>(message: S) -> Self {
        Error::SignatureError(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_path_error() {
        let err = Error::invalid_path("/invalid/path");
        assert!(matches!(err, Error::InvalidPath { .. }));
        assert!(err.to_string().contains("/invalid/path"));
    }

    #[test]
    fn test_create_failed_error() {
        let err = Error::create_failed("/tmp/test.log");
        assert!(matches!(err, Error::CreateFailed { .. }));
        assert!(err.to_string().contains("/tmp/test.log"));
    }

    #[test]
    fn test_append_failed_error() {
        let err = Error::append_failed("disk full");
        assert!(matches!(err, Error::AppendFailed(_)));
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn test_invalid_entry_error() {
        let err = Error::invalid_entry(42, "malformed JSON");
        assert!(matches!(err, Error::InvalidEntry { .. }));
        let msg = err.to_string();
        assert!(msg.contains("42"));
        assert!(msg.contains("malformed JSON"));
    }

    #[test]
    fn test_missing_field_error() {
        let err = Error::missing_field("job_id");
        assert!(matches!(err, Error::MissingField { .. }));
        assert!(err.to_string().contains("job_id"));
    }

    #[test]
    fn test_invalid_event_type() {
        let err = Error::invalid_event_type("unknown_event");
        assert!(matches!(err, Error::InvalidEventType(_)));
        assert!(err.to_string().contains("unknown_event"));
    }

    #[test]
    fn test_signature_error() {
        let err = Error::signature_error("invalid key");
        assert!(matches!(err, Error::SignatureError(_)));
        assert!(err.to_string().contains("invalid key"));
    }

    #[test]
    fn test_beacon_not_finalized() {
        let err = Error::BeaconNotFinalized;
        assert!(matches!(err, Error::BeaconNotFinalized));
        assert!(err.to_string().contains("not finalized"));
    }
}
