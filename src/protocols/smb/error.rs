//! SMB error types

use thiserror::Error;

/// SMB-specific errors
///
/// All errors that can occur during SMB operations.
#[derive(Error, Debug)]
pub enum SmbError {
    /// Authentication failed
    #[error("authentication failed")]
    Auth,
    
    /// Permission denied
    #[error("permission denied: {0}")]
    Permission(String),
    
    /// Path not found
    #[error("path not found: {0}")]
    NotFound(String),
    
    /// Encryption required but not negotiated
    #[error("encryption required but not negotiated")]
    EncryptionRequired,
    
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Protocol error
    #[error("protocol error: {0}")]
    Protocol(&'static str),
    
    /// Operation timed out
    #[error("timeout")]
    Timeout,
    
    /// Unsupported feature or operation
    #[error("unsupported feature: {0}")]
    Unsupported(&'static str),
    
    /// Connection error
    #[error("connection error: {0}")]
    Connection(String),
    
    /// Invalid path or configuration
    #[error("invalid path: {0}")]
    InvalidPath(String),
    
    /// Generic error
    #[error("other: {0}")]
    Other(String),
}

/// Result type for SMB operations
pub type Result<T> = std::result::Result<T, SmbError>;

impl SmbError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SmbError::Timeout | SmbError::Connection(_) | SmbError::Io(_)
        )
    }
    
    /// Check if this error is a permission issue
    pub fn is_permission_error(&self) -> bool {
        matches!(self, SmbError::Permission(_) | SmbError::Auth)
    }
    
    /// Check if this error indicates the resource doesn't exist
    pub fn is_not_found(&self) -> bool {
        matches!(self, SmbError::NotFound(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SmbError::Auth;
        assert_eq!(err.to_string(), "authentication failed");
        
        let err = SmbError::NotFound("/path/to/file".to_string());
        assert_eq!(err.to_string(), "path not found: /path/to/file");
        
        let err = SmbError::Protocol("invalid response");
        assert_eq!(err.to_string(), "protocol error: invalid response");
    }

    #[test]
    fn test_retryable() {
        assert!(SmbError::Timeout.is_retryable());
        assert!(SmbError::Connection("test".to_string()).is_retryable());
        assert!(!SmbError::Auth.is_retryable());
        assert!(!SmbError::NotFound("test".to_string()).is_retryable());
    }

    #[test]
    fn test_permission_error() {
        assert!(SmbError::Auth.is_permission_error());
        assert!(SmbError::Permission("test".to_string()).is_permission_error());
        assert!(!SmbError::NotFound("test".to_string()).is_permission_error());
    }

    #[test]
    fn test_not_found() {
        assert!(SmbError::NotFound("test".to_string()).is_not_found());
        assert!(!SmbError::Auth.is_not_found());
        assert!(!SmbError::Timeout.is_not_found());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let smb_err: SmbError = io_err.into();
        
        assert!(matches!(smb_err, SmbError::Io(_)));
        assert!(smb_err.to_string().contains("I/O error"));
    }
}