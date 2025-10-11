/*!
 * Error types for Orbit
 */

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for Orbit operations
pub type Result<T> = std::result::Result<T, OrbitError>;

/// Comprehensive error types for Orbit operations
#[derive(Error, Debug)]
pub enum OrbitError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Checksum mismatch for file {path:?}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("Insufficient disk space: required {required} bytes, available {available} bytes")]
    InsufficientDiskSpace {
        required: u64,
        available: u64,
    },

    #[error("Source file not found: {0:?}")]
    SourceNotFound(PathBuf),

    #[error("Destination already exists: {0:?}")]
    DestinationExists(PathBuf),

    #[error("Permission denied: {0:?}")]
    PermissionDenied(PathBuf),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Resume error: {0}")]
    Resume(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid path: {0:?}")]
    InvalidPath(PathBuf),

    #[error("Symlink error: {0}")]
    Symlink(String),

    #[error("Parallel operation failed: {0}")]
    Parallel(String),

    #[error("All retry attempts exhausted after {attempts} tries")]
    RetriesExhausted { attempts: u32 },

    #[error("Operation timeout after {seconds} seconds")]
    Timeout { seconds: u64 },

    #[error("Metadata preservation failed: {0}")]
    MetadataFailed(String),

    #[error("Audit log error: {0}")]
    AuditLog(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl OrbitError {
    /// Returns true if this error is recoverable (should retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            OrbitError::Io(_) | OrbitError::Timeout { .. }
        )
    }

    /// Returns true if this error is a fatal configuration issue
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            OrbitError::Config(_)
                | OrbitError::InsufficientDiskSpace { .. }
                | OrbitError::SourceNotFound(_)
        )
    }
}