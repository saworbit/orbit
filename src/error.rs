/*!
 * Error types for Orbit
 */

use std::fmt;
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, OrbitError>;

#[derive(Debug)]
pub enum OrbitError {
    /// Source file or directory not found
    SourceNotFound(PathBuf),
    
    /// Invalid path
    InvalidPath(PathBuf),
    
    /// I/O error
    Io(io::Error),
    
    /// Insufficient disk space
    InsufficientDiskSpace {
        required: u64,
        available: u64,
    },
    
    /// Configuration error
    Config(String),
    
    /// Compression error
    Compression(String),
    
    /// Decompression error
    Decompression(String),
    
    /// Resume error
    Resume(String),
    
    /// Checksum verification failed
    ChecksumMismatch {
        expected: String,
        actual: String,
    },
    
    /// Symbolic link error
    Symlink(String),
    
    /// Parallel processing error
    Parallel(String),
    
    /// Retries exhausted
    RetriesExhausted {
        attempts: u32,
    },
    
    /// Zero-copy not supported (triggers fallback to buffered copy)
    ZeroCopyUnsupported,
    
    /// Protocol error (for SMB, S3, etc.)
    Protocol(String),
    
    /// Authentication error
    Authentication(String),
}

impl OrbitError {
    /// Check if this error is fatal (should not retry)
    pub fn is_fatal(&self) -> bool {
        match self {
            // These are fatal - don't retry
            OrbitError::SourceNotFound(_) => true,
            OrbitError::InvalidPath(_) => true,
            OrbitError::Config(_) => true,
            OrbitError::ChecksumMismatch { .. } => true,
            OrbitError::InsufficientDiskSpace { .. } => true,
            
            // These are not fatal - can retry
            OrbitError::Io(_) => false,
            OrbitError::Compression(_) => false,
            OrbitError::Decompression(_) => false,
            OrbitError::Resume(_) => false,
            OrbitError::Symlink(_) => false,
            OrbitError::Parallel(_) => false,
            OrbitError::RetriesExhausted { .. } => true,
            OrbitError::ZeroCopyUnsupported => false, // Not fatal, triggers fallback
            OrbitError::Protocol(_) => false,
            OrbitError::Authentication(_) => true,
        }
    }
    
    /// Check if this error indicates zero-copy is unsupported
    pub fn is_zero_copy_unsupported(&self) -> bool {
        matches!(self, OrbitError::ZeroCopyUnsupported)
    }
}

impl fmt::Display for OrbitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrbitError::SourceNotFound(path) => {
                write!(f, "Source not found: {}", path.display())
            }
            OrbitError::InvalidPath(path) => {
                write!(f, "Invalid path: {}", path.display())
            }
            OrbitError::Io(err) => {
                write!(f, "I/O error: {}", err)
            }
            OrbitError::InsufficientDiskSpace { required, available } => {
                write!(
                    f,
                    "Insufficient disk space: {} bytes required, {} bytes available",
                    required, available
                )
            }
            OrbitError::Config(msg) => {
                write!(f, "Configuration error: {}", msg)
            }
            OrbitError::Compression(msg) => {
                write!(f, "Compression error: {}", msg)
            }
            OrbitError::Decompression(msg) => {
                write!(f, "Decompression error: {}", msg)
            }
            OrbitError::Resume(msg) => {
                write!(f, "Resume error: {}", msg)
            }
            OrbitError::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Checksum verification failed: expected {}, got {}",
                    expected, actual
                )
            }
            OrbitError::Symlink(msg) => {
                write!(f, "Symbolic link error: {}", msg)
            }
            OrbitError::Parallel(msg) => {
                write!(f, "Parallel processing error: {}", msg)
            }
            OrbitError::RetriesExhausted { attempts } => {
                write!(f, "All {} retry attempts exhausted", attempts)
            }
            OrbitError::ZeroCopyUnsupported => {
                write!(f, "Zero-copy not supported on this platform/filesystem")
            }
            OrbitError::Protocol(msg) => {
                write!(f, "Protocol error: {}", msg)
            }
            OrbitError::Authentication(msg) => {
                write!(f, "Authentication error: {}", msg)
            }
        }
    }
}

impl std::error::Error for OrbitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OrbitError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for OrbitError {
    fn from(err: io::Error) -> Self {
        OrbitError::Io(err)
    }
}

impl From<serde_json::Error> for OrbitError {
    fn from(err: serde_json::Error) -> Self {
        OrbitError::Resume(format!("JSON error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fatal_errors() {
        assert!(OrbitError::SourceNotFound(PathBuf::from("/tmp")).is_fatal());
        assert!(OrbitError::Config("test".to_string()).is_fatal());
        assert!(OrbitError::ChecksumMismatch {
            expected: "abc".to_string(),
            actual: "def".to_string(),
        }.is_fatal());
    }

    #[test]
    fn test_non_fatal_errors() {
        assert!(!OrbitError::Io(io::Error::new(io::ErrorKind::Other, "test")).is_fatal());
        assert!(!OrbitError::Compression("test".to_string()).is_fatal());
        assert!(!OrbitError::ZeroCopyUnsupported.is_fatal());
    }

    #[test]
    fn test_zero_copy_unsupported_detection() {
        assert!(OrbitError::ZeroCopyUnsupported.is_zero_copy_unsupported());
        assert!(!OrbitError::Io(io::Error::new(io::ErrorKind::Other, "test")).is_zero_copy_unsupported());
    }

    #[test]
    fn test_error_display() {
        let err = OrbitError::ChecksumMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Checksum verification failed: expected abc123, got def456"
        );
    }

    #[test]
    fn test_zero_copy_unsupported_display() {
        let err = OrbitError::ZeroCopyUnsupported;
        assert_eq!(
            err.to_string(),
            "Zero-copy not supported on this platform/filesystem"
        );
    }
}