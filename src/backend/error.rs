//! Error types for unified backend abstraction
//!
//! This module provides comprehensive error handling for all backend operations,
//! including I/O errors, authentication failures, network issues, and more.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Result type alias for backend operations
pub type BackendResult<T> = std::result::Result<T, BackendError>;

/// Unified error type for backend operations
#[derive(Debug)]
pub enum BackendError {
    /// I/O error occurred during backend operation
    Io(io::Error),

    /// Path not found on backend
    NotFound { path: PathBuf, backend: String },

    /// Permission denied accessing resource
    PermissionDenied { path: PathBuf, message: String },

    /// Authentication failed
    AuthenticationFailed { backend: String, message: String },

    /// Connection failed to remote backend
    ConnectionFailed {
        backend: String,
        endpoint: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Operation timed out
    Timeout {
        operation: String,
        duration_secs: u64,
    },

    /// Invalid configuration for backend
    InvalidConfig { backend: String, message: String },

    /// Backend operation not supported
    Unsupported { backend: String, operation: String },

    /// Path is invalid or malformed
    InvalidPath { path: PathBuf, reason: String },

    /// Resource already exists (e.g., during exclusive create)
    AlreadyExists { path: PathBuf },

    /// Directory is not empty (e.g., during delete)
    DirectoryNotEmpty { path: PathBuf },

    /// Quota or space limit exceeded
    QuotaExceeded { backend: String, message: String },

    /// Network error during remote operation
    Network {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Serialization/deserialization error
    Serialization { message: String },

    /// Generic backend error with context
    Other { backend: String, message: String },
}

impl BackendError {
    /// Check if this error is retriable (transient)
    pub fn is_retriable(&self) -> bool {
        match self {
            // Retriable errors - typically network or temporary issues
            BackendError::Timeout { .. } => true,
            BackendError::ConnectionFailed { .. } => true,
            BackendError::Network { .. } => true,
            BackendError::Io(e) => {
                matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::ConnectionAborted
                )
            }

            // Non-retriable errors
            BackendError::NotFound { .. } => false,
            BackendError::PermissionDenied { .. } => false,
            BackendError::AuthenticationFailed { .. } => false,
            BackendError::InvalidConfig { .. } => false,
            BackendError::Unsupported { .. } => false,
            BackendError::InvalidPath { .. } => false,
            BackendError::AlreadyExists { .. } => false,
            BackendError::DirectoryNotEmpty { .. } => false,
            BackendError::QuotaExceeded { .. } => false,
            BackendError::Serialization { .. } => false,
            BackendError::Other { .. } => false,
        }
    }

    /// Check if this error indicates the resource was not found
    pub fn is_not_found(&self) -> bool {
        matches!(self, BackendError::NotFound { .. })
    }

    /// Check if this error is related to authentication
    pub fn is_auth_error(&self) -> bool {
        matches!(self, BackendError::AuthenticationFailed { .. })
    }

    /// Get the backend name associated with this error, if any
    pub fn backend_name(&self) -> Option<&str> {
        match self {
            BackendError::NotFound { backend, .. } => Some(backend),
            BackendError::AuthenticationFailed { backend, .. } => Some(backend),
            BackendError::ConnectionFailed { backend, .. } => Some(backend),
            BackendError::InvalidConfig { backend, .. } => Some(backend),
            BackendError::Unsupported { backend, .. } => Some(backend),
            BackendError::QuotaExceeded { backend, .. } => Some(backend),
            BackendError::Other { backend, .. } => Some(backend),
            _ => None,
        }
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::Io(err) => write!(f, "I/O error: {}", err),
            BackendError::NotFound { path, backend } => {
                write!(f, "Path not found on {}: {}", backend, path.display())
            }
            BackendError::PermissionDenied { path, message } => {
                write!(f, "Permission denied for {}: {}", path.display(), message)
            }
            BackendError::AuthenticationFailed { backend, message } => {
                write!(f, "Authentication failed for {}: {}", backend, message)
            }
            BackendError::ConnectionFailed {
                backend,
                endpoint,
                source,
            } => {
                if let Some(src) = source {
                    write!(
                        f,
                        "Connection to {} ({}) failed: {}",
                        backend, endpoint, src
                    )
                } else {
                    write!(f, "Connection to {} ({}) failed", backend, endpoint)
                }
            }
            BackendError::Timeout {
                operation,
                duration_secs,
            } => {
                write!(
                    f,
                    "Operation '{}' timed out after {} seconds",
                    operation, duration_secs
                )
            }
            BackendError::InvalidConfig { backend, message } => {
                write!(f, "Invalid configuration for {}: {}", backend, message)
            }
            BackendError::Unsupported { backend, operation } => {
                write!(
                    f,
                    "Operation '{}' not supported by backend {}",
                    operation, backend
                )
            }
            BackendError::InvalidPath { path, reason } => {
                write!(f, "Invalid path {}: {}", path.display(), reason)
            }
            BackendError::AlreadyExists { path } => {
                write!(f, "Path already exists: {}", path.display())
            }
            BackendError::DirectoryNotEmpty { path } => {
                write!(f, "Directory not empty: {}", path.display())
            }
            BackendError::QuotaExceeded { backend, message } => {
                write!(f, "Quota exceeded on {}: {}", backend, message)
            }
            BackendError::Network { message, source } => {
                if let Some(src) = source {
                    write!(f, "Network error: {} ({})", message, src)
                } else {
                    write!(f, "Network error: {}", message)
                }
            }
            BackendError::Serialization { message } => {
                write!(f, "Serialization error: {}", message)
            }
            BackendError::Other { backend, message } => {
                write!(f, "Backend error on {}: {}", backend, message)
            }
        }
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BackendError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for BackendError {
    fn from(err: io::Error) -> Self {
        // Map specific I/O errors to more specific backend errors
        match err.kind() {
            io::ErrorKind::NotFound => BackendError::NotFound {
                path: PathBuf::new(),
                backend: "unknown".to_string(),
            },
            io::ErrorKind::PermissionDenied => BackendError::PermissionDenied {
                path: PathBuf::new(),
                message: err.to_string(),
            },
            io::ErrorKind::AlreadyExists => BackendError::AlreadyExists {
                path: PathBuf::new(),
            },
            _ => BackendError::Io(err),
        }
    }
}

// Integration with main OrbitError
impl From<BackendError> for crate::error::OrbitError {
    fn from(err: BackendError) -> Self {
        match err {
            BackendError::Io(e) => crate::error::OrbitError::Io(e),
            BackendError::AuthenticationFailed { message, .. } => {
                crate::error::OrbitError::Authentication(message)
            }
            BackendError::NotFound { path, .. } => crate::error::OrbitError::SourceNotFound(path),
            BackendError::PermissionDenied { message, .. } => {
                crate::error::OrbitError::Protocol(message)
            }
            other => crate::error::OrbitError::Protocol(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retriable() {
        let err = BackendError::Timeout {
            operation: "read".to_string(),
            duration_secs: 30,
        };
        assert!(err.is_retriable());

        let err = BackendError::NotFound {
            path: PathBuf::from("/test"),
            backend: "local".to_string(),
        };
        assert!(!err.is_retriable());
    }

    #[test]
    fn test_error_display() {
        let err = BackendError::AuthenticationFailed {
            backend: "ssh".to_string(),
            message: "invalid credentials".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Authentication failed for ssh: invalid credentials"
        );
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let backend_err: BackendError = io_err.into();
        assert!(backend_err.is_not_found());
    }

    #[test]
    fn test_backend_name() {
        let err = BackendError::ConnectionFailed {
            backend: "s3".to_string(),
            endpoint: "s3.amazonaws.com".to_string(),
            source: None,
        };
        assert_eq!(err.backend_name(), Some("s3"));
    }
}
