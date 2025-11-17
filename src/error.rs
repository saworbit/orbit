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
    InsufficientDiskSpace { required: u64, available: u64 },

    /// Configuration error
    Config(String),

    /// Compression error
    Compression(String),

    /// Decompression error
    Decompression(String),

    /// Resume error
    Resume(String),

    /// Checksum verification failed
    ChecksumMismatch { expected: String, actual: String },

    /// Symbolic link error
    Symlink(String),

    /// Parallel processing error
    Parallel(String),

    /// Retries exhausted
    RetriesExhausted { attempts: u32 },

    /// Zero-copy not supported (triggers fallback to buffered copy)
    ZeroCopyUnsupported,

    /// Protocol error (for SMB, S3, etc.)
    Protocol(String),

    /// Authentication error
    Authentication(String),

    /// Metadata operation failed
    MetadataFailed(String),

    /// Audit log error
    AuditLog(String),

    /// Generic error with message
    Other(String),
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
            OrbitError::Authentication(_) => true,
            OrbitError::RetriesExhausted { .. } => true,

            // These are not fatal - can retry
            OrbitError::Io(_) => false,
            OrbitError::Compression(_) => false,
            OrbitError::Decompression(_) => false,
            OrbitError::Resume(_) => false,
            OrbitError::Symlink(_) => false,
            OrbitError::Parallel(_) => false,
            OrbitError::ZeroCopyUnsupported => false, // Not fatal, triggers fallback
            OrbitError::Protocol(_) => false,
            OrbitError::MetadataFailed(_) => false,
            OrbitError::AuditLog(_) => false,
            OrbitError::Other(_) => false,
        }
    }

    /// Check if this error is transient (temporary, worth retrying)
    pub fn is_transient(&self) -> bool {
        match self {
            // Transient errors that often resolve on retry
            OrbitError::Io(io_err) => Self::is_io_transient(io_err),
            OrbitError::Protocol(_) => true,
            OrbitError::Compression(_) => true,
            OrbitError::Decompression(_) => true,
            OrbitError::Resume(_) => true,
            OrbitError::MetadataFailed(_) => true,

            // Not transient
            _ => false,
        }
    }

    /// Check if an I/O error is transient
    fn is_io_transient(io_err: &io::Error) -> bool {
        use io::ErrorKind::*;
        matches!(
            io_err.kind(),
            ConnectionRefused
                | ConnectionReset
                | ConnectionAborted
                | NotConnected
                | BrokenPipe
                | TimedOut
                | Interrupted
                | WouldBlock
                | WriteZero
        )
    }

    /// Check if this error is a network-related error
    pub fn is_network_error(&self) -> bool {
        match self {
            OrbitError::Io(io_err) => {
                use io::ErrorKind::*;
                matches!(
                    io_err.kind(),
                    ConnectionRefused
                        | ConnectionReset
                        | ConnectionAborted
                        | NotConnected
                        | BrokenPipe
                        | TimedOut
                )
            }
            OrbitError::Protocol(_) => true,
            _ => false,
        }
    }

    /// Check if this error indicates zero-copy is unsupported
    pub fn is_zero_copy_unsupported(&self) -> bool {
        matches!(self, OrbitError::ZeroCopyUnsupported)
    }

    /// Get error category for logging and instrumentation
    pub fn category(&self) -> ErrorCategory {
        match self {
            OrbitError::SourceNotFound(_) | OrbitError::InvalidPath(_) => ErrorCategory::Validation,
            OrbitError::Io(_) => ErrorCategory::IoError,
            OrbitError::InsufficientDiskSpace { .. } => ErrorCategory::Resource,
            OrbitError::Config(_) => ErrorCategory::Configuration,
            OrbitError::Compression(_) | OrbitError::Decompression(_) => ErrorCategory::Codec,
            OrbitError::Resume(_) => ErrorCategory::Resume,
            OrbitError::ChecksumMismatch { .. } => ErrorCategory::Integrity,
            OrbitError::Symlink(_) => ErrorCategory::Filesystem,
            OrbitError::Parallel(_) => ErrorCategory::Concurrency,
            OrbitError::RetriesExhausted { .. } => ErrorCategory::Retry,
            OrbitError::ZeroCopyUnsupported => ErrorCategory::Optimization,
            OrbitError::Protocol(_) => ErrorCategory::Network,
            OrbitError::Authentication(_) => ErrorCategory::Security,
            OrbitError::MetadataFailed(_) => ErrorCategory::Metadata,
            OrbitError::AuditLog(_) => ErrorCategory::Audit,
            OrbitError::Other(_) => ErrorCategory::Unknown,
        }
    }
}

/// Error category for classification and reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Path validation errors
    Validation,
    /// I/O operation errors
    IoError,
    /// Resource availability errors (disk space, memory)
    Resource,
    /// Configuration errors
    Configuration,
    /// Compression/decompression errors
    Codec,
    /// Resume/checkpoint errors
    Resume,
    /// Data integrity errors (checksums)
    Integrity,
    /// Filesystem operations (symlinks, permissions)
    Filesystem,
    /// Parallel processing errors
    Concurrency,
    /// Retry exhaustion
    Retry,
    /// Optimization fallbacks
    Optimization,
    /// Network/protocol errors
    Network,
    /// Authentication/authorization errors
    Security,
    /// Metadata preservation errors
    Metadata,
    /// Audit logging errors
    Audit,
    /// Uncategorized errors
    Unknown,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Validation => write!(f, "validation"),
            ErrorCategory::IoError => write!(f, "io"),
            ErrorCategory::Resource => write!(f, "resource"),
            ErrorCategory::Configuration => write!(f, "configuration"),
            ErrorCategory::Codec => write!(f, "codec"),
            ErrorCategory::Resume => write!(f, "resume"),
            ErrorCategory::Integrity => write!(f, "integrity"),
            ErrorCategory::Filesystem => write!(f, "filesystem"),
            ErrorCategory::Concurrency => write!(f, "concurrency"),
            ErrorCategory::Retry => write!(f, "retry"),
            ErrorCategory::Optimization => write!(f, "optimization"),
            ErrorCategory::Network => write!(f, "network"),
            ErrorCategory::Security => write!(f, "security"),
            ErrorCategory::Metadata => write!(f, "metadata"),
            ErrorCategory::Audit => write!(f, "audit"),
            ErrorCategory::Unknown => write!(f, "unknown"),
        }
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
            OrbitError::InsufficientDiskSpace {
                required,
                available,
            } => {
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
            OrbitError::MetadataFailed(msg) => {
                write!(f, "Metadata operation failed: {}", msg)
            }
            OrbitError::AuditLog(msg) => {
                write!(f, "Audit log error: {}", msg)
            }
            OrbitError::Other(msg) => {
                write!(f, "{}", msg)
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
        }
        .is_fatal());
    }

    #[test]
    fn test_non_fatal_errors() {
        assert!(!OrbitError::Io(io::Error::new(io::ErrorKind::Other, "test")).is_fatal());
        assert!(!OrbitError::Compression("test".to_string()).is_fatal());
        assert!(!OrbitError::ZeroCopyUnsupported.is_fatal());
        assert!(!OrbitError::Other("test".to_string()).is_fatal());
    }

    #[test]
    fn test_zero_copy_unsupported_detection() {
        assert!(OrbitError::ZeroCopyUnsupported.is_zero_copy_unsupported());
        assert!(
            !OrbitError::Io(io::Error::new(io::ErrorKind::Other, "test"))
                .is_zero_copy_unsupported()
        );
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

    #[test]
    fn test_other_error() {
        let err = OrbitError::Other("custom error message".to_string());
        assert_eq!(err.to_string(), "custom error message");
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_transient_errors() {
        // Transient errors
        assert!(OrbitError::Protocol("timeout".to_string()).is_transient());
        assert!(OrbitError::Compression("temp failure".to_string()).is_transient());
        assert!(OrbitError::Decompression("temp failure".to_string()).is_transient());
        assert!(OrbitError::Resume("partial".to_string()).is_transient());
        assert!(
            OrbitError::MetadataFailed("permission denied temporarily".to_string()).is_transient()
        );

        // Non-transient errors
        assert!(!OrbitError::SourceNotFound(PathBuf::from("/tmp")).is_transient());
        assert!(!OrbitError::Config("bad config".to_string()).is_transient());
        assert!(!OrbitError::Authentication("invalid creds".to_string()).is_transient());
    }

    #[test]
    fn test_network_errors() {
        assert!(OrbitError::Protocol("connection failed".to_string()).is_network_error());

        let io_err = io::Error::new(io::ErrorKind::ConnectionRefused, "refused");
        assert!(OrbitError::Io(io_err).is_network_error());

        assert!(!OrbitError::Config("test".to_string()).is_network_error());
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(
            OrbitError::SourceNotFound(PathBuf::from("/tmp")).category(),
            ErrorCategory::Validation
        );
        assert_eq!(
            OrbitError::Io(io::Error::new(io::ErrorKind::Other, "test")).category(),
            ErrorCategory::IoError
        );
        assert_eq!(
            OrbitError::InsufficientDiskSpace {
                required: 100,
                available: 50
            }
            .category(),
            ErrorCategory::Resource
        );
        assert_eq!(
            OrbitError::Config("test".to_string()).category(),
            ErrorCategory::Configuration
        );
        assert_eq!(
            OrbitError::Compression("test".to_string()).category(),
            ErrorCategory::Codec
        );
        assert_eq!(
            OrbitError::Resume("test".to_string()).category(),
            ErrorCategory::Resume
        );
        assert_eq!(
            OrbitError::ChecksumMismatch {
                expected: "a".to_string(),
                actual: "b".to_string()
            }
            .category(),
            ErrorCategory::Integrity
        );
        assert_eq!(
            OrbitError::Protocol("test".to_string()).category(),
            ErrorCategory::Network
        );
        assert_eq!(
            OrbitError::Authentication("test".to_string()).category(),
            ErrorCategory::Security
        );
    }

    #[test]
    fn test_error_category_display() {
        assert_eq!(ErrorCategory::Validation.to_string(), "validation");
        assert_eq!(ErrorCategory::Network.to_string(), "network");
        assert_eq!(ErrorCategory::Security.to_string(), "security");
    }
}
