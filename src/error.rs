/*!
 * Error types for Orbit
 */

use std::fmt;
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, OrbitError>;

/// Exit code constants for structured process exit
pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_PARTIAL: i32 = 1;
pub const EXIT_FATAL: i32 = 2;
pub const EXIT_INTEGRITY: i32 = 3;

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
    /// Get the process exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            // Fatal errors: config, auth, source not found
            OrbitError::SourceNotFound(_)
            | OrbitError::InvalidPath(_)
            | OrbitError::Config(_)
            | OrbitError::Authentication(_)
            | OrbitError::InsufficientDiskSpace { .. } => EXIT_FATAL,
            // Integrity errors: checksum mismatch
            OrbitError::ChecksumMismatch { .. } => EXIT_INTEGRITY,
            // Partial failures: retries exhausted, parallel errors
            OrbitError::RetriesExhausted { .. } | OrbitError::Parallel(_) => EXIT_PARTIAL,
            // Everything else: partial failure
            _ => EXIT_PARTIAL,
        }
    }

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
            // Compression/decompression errors are typically permanent (corrupt data, bad format)
            OrbitError::Compression(_) => false,
            OrbitError::Decompression(_) => false,
            OrbitError::Resume(_) => false,
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
        OrbitError::Config(format!("JSON parse error: {}", err))
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
        assert!(!OrbitError::Io(io::Error::other("test")).is_fatal());
        assert!(!OrbitError::Compression("test".to_string()).is_fatal());
        assert!(!OrbitError::ZeroCopyUnsupported.is_fatal());
        assert!(!OrbitError::Other("test".to_string()).is_fatal());
    }

    #[test]
    fn test_zero_copy_unsupported_detection() {
        assert!(OrbitError::ZeroCopyUnsupported.is_zero_copy_unsupported());
        assert!(!OrbitError::Io(io::Error::other("test")).is_zero_copy_unsupported());
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
        // Compression/decompression/resume errors are permanent (corrupt data, bad format)
        assert!(!OrbitError::Compression("temp failure".to_string()).is_transient());
        assert!(!OrbitError::Decompression("temp failure".to_string()).is_transient());
        assert!(!OrbitError::Resume("partial".to_string()).is_transient());
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
            OrbitError::Io(io::Error::other("test")).category(),
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

    #[test]
    fn test_permanent_io_errors() {
        // Permission denied is NOT fatal (so it could be skipped),
        // but it is NOT transient (so it should not be retried).
        let perm_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let orbit_err = OrbitError::Io(perm_err);

        assert!(!orbit_err.is_fatal());
        assert!(!orbit_err.is_transient());

        // Already exists
        let exists_err = io::Error::new(io::ErrorKind::AlreadyExists, "exists");
        let orbit_err_exists = OrbitError::Io(exists_err);
        assert!(!orbit_err_exists.is_transient());

        // Not found (I/O variant, not the enum variant)
        let not_found_err = io::Error::new(io::ErrorKind::NotFound, "not found");
        let orbit_err_not_found = OrbitError::Io(not_found_err);
        assert!(!orbit_err_not_found.is_transient());
    }

    #[test]
    fn test_transient_io_errors() {
        // These I/O errors should be retried
        let timeout_err = io::Error::new(io::ErrorKind::TimedOut, "timed out");
        assert!(OrbitError::Io(timeout_err).is_transient());

        let conn_refused = io::Error::new(io::ErrorKind::ConnectionRefused, "refused");
        assert!(OrbitError::Io(conn_refused).is_transient());

        let interrupted = io::Error::new(io::ErrorKind::Interrupted, "interrupted");
        assert!(OrbitError::Io(interrupted).is_transient());
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(
            OrbitError::SourceNotFound(PathBuf::from("/tmp")).exit_code(),
            EXIT_FATAL
        );
        assert_eq!(
            OrbitError::Config("bad".to_string()).exit_code(),
            EXIT_FATAL
        );
        assert_eq!(
            OrbitError::Authentication("denied".to_string()).exit_code(),
            EXIT_FATAL
        );
        assert_eq!(
            OrbitError::ChecksumMismatch {
                expected: "a".to_string(),
                actual: "b".to_string()
            }
            .exit_code(),
            EXIT_INTEGRITY
        );
        assert_eq!(
            OrbitError::RetriesExhausted { attempts: 3 }.exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::Protocol("timeout".to_string()).exit_code(),
            EXIT_PARTIAL
        );
    }

    #[test]
    fn test_exit_code_constants() {
        assert_eq!(EXIT_SUCCESS, 0);
        assert_eq!(EXIT_PARTIAL, 1);
        assert_eq!(EXIT_FATAL, 2);
        assert_eq!(EXIT_INTEGRITY, 3);
    }

    #[test]
    fn test_exit_code_all_variants() {
        // Fatal errors → EXIT_FATAL
        assert_eq!(
            OrbitError::SourceNotFound(PathBuf::from("/missing")).exit_code(),
            EXIT_FATAL
        );
        assert_eq!(
            OrbitError::InvalidPath(PathBuf::from("/bad\0path")).exit_code(),
            EXIT_FATAL
        );
        assert_eq!(
            OrbitError::Config("bad config".to_string()).exit_code(),
            EXIT_FATAL
        );
        assert_eq!(
            OrbitError::Authentication("invalid".to_string()).exit_code(),
            EXIT_FATAL
        );
        assert_eq!(
            OrbitError::InsufficientDiskSpace {
                required: 1000,
                available: 100
            }
            .exit_code(),
            EXIT_FATAL
        );

        // Integrity errors → EXIT_INTEGRITY
        assert_eq!(
            OrbitError::ChecksumMismatch {
                expected: "aaa".to_string(),
                actual: "bbb".to_string()
            }
            .exit_code(),
            EXIT_INTEGRITY
        );

        // Partial errors → EXIT_PARTIAL
        assert_eq!(
            OrbitError::RetriesExhausted { attempts: 5 }.exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::Parallel("worker panic".to_string()).exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::Io(io::Error::other("disk read")).exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::Compression("lz4 fail".to_string()).exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::Decompression("zstd fail".to_string()).exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::Resume("checkpoint corrupt".to_string()).exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::Symlink("broken link".to_string()).exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(OrbitError::ZeroCopyUnsupported.exit_code(), EXIT_PARTIAL);
        assert_eq!(
            OrbitError::Protocol("smb timeout".to_string()).exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::MetadataFailed("chmod failed".to_string()).exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::AuditLog("write failed".to_string()).exit_code(),
            EXIT_PARTIAL
        );
        assert_eq!(
            OrbitError::Other("something went wrong".to_string()).exit_code(),
            EXIT_PARTIAL
        );
    }

    #[test]
    fn test_is_fatal_all_variants() {
        // Fatal variants
        assert!(OrbitError::SourceNotFound(PathBuf::from("/gone")).is_fatal());
        assert!(OrbitError::InvalidPath(PathBuf::from("")).is_fatal());
        assert!(OrbitError::Config("missing field".to_string()).is_fatal());
        assert!(OrbitError::ChecksumMismatch {
            expected: "x".to_string(),
            actual: "y".to_string(),
        }
        .is_fatal());
        assert!(OrbitError::InsufficientDiskSpace {
            required: 500,
            available: 10,
        }
        .is_fatal());
        assert!(OrbitError::Authentication("bad creds".to_string()).is_fatal());
        assert!(OrbitError::RetriesExhausted { attempts: 10 }.is_fatal());

        // Non-fatal variants
        assert!(!OrbitError::Io(io::Error::other("oops")).is_fatal());
        assert!(!OrbitError::Compression("bad data".to_string()).is_fatal());
        assert!(!OrbitError::Decompression("bad frame".to_string()).is_fatal());
        assert!(!OrbitError::Resume("stale checkpoint".to_string()).is_fatal());
        assert!(!OrbitError::Symlink("dangling".to_string()).is_fatal());
        assert!(!OrbitError::Parallel("thread panic".to_string()).is_fatal());
        assert!(!OrbitError::ZeroCopyUnsupported.is_fatal());
        assert!(!OrbitError::Protocol("connection reset".to_string()).is_fatal());
        assert!(!OrbitError::MetadataFailed("xattr not supported".to_string()).is_fatal());
        assert!(!OrbitError::AuditLog("disk full".to_string()).is_fatal());
        assert!(!OrbitError::Other("unknown issue".to_string()).is_fatal());
    }

    #[test]
    fn test_category_all_variants() {
        assert_eq!(
            OrbitError::SourceNotFound(PathBuf::from("/a")).category(),
            ErrorCategory::Validation
        );
        assert_eq!(
            OrbitError::InvalidPath(PathBuf::from("/b")).category(),
            ErrorCategory::Validation
        );
        assert_eq!(
            OrbitError::Io(io::Error::other("err")).category(),
            ErrorCategory::IoError
        );
        assert_eq!(
            OrbitError::InsufficientDiskSpace {
                required: 1,
                available: 0
            }
            .category(),
            ErrorCategory::Resource
        );
        assert_eq!(
            OrbitError::Config("x".to_string()).category(),
            ErrorCategory::Configuration
        );
        assert_eq!(
            OrbitError::Compression("x".to_string()).category(),
            ErrorCategory::Codec
        );
        assert_eq!(
            OrbitError::Decompression("x".to_string()).category(),
            ErrorCategory::Codec
        );
        assert_eq!(
            OrbitError::Resume("x".to_string()).category(),
            ErrorCategory::Resume
        );
        assert_eq!(
            OrbitError::ChecksumMismatch {
                expected: "e".to_string(),
                actual: "a".to_string()
            }
            .category(),
            ErrorCategory::Integrity
        );
        assert_eq!(
            OrbitError::Symlink("x".to_string()).category(),
            ErrorCategory::Filesystem
        );
        assert_eq!(
            OrbitError::Parallel("x".to_string()).category(),
            ErrorCategory::Concurrency
        );
        assert_eq!(
            OrbitError::RetriesExhausted { attempts: 1 }.category(),
            ErrorCategory::Retry
        );
        assert_eq!(
            OrbitError::ZeroCopyUnsupported.category(),
            ErrorCategory::Optimization
        );
        assert_eq!(
            OrbitError::Protocol("x".to_string()).category(),
            ErrorCategory::Network
        );
        assert_eq!(
            OrbitError::Authentication("x".to_string()).category(),
            ErrorCategory::Security
        );
        assert_eq!(
            OrbitError::MetadataFailed("x".to_string()).category(),
            ErrorCategory::Metadata
        );
        assert_eq!(
            OrbitError::AuditLog("x".to_string()).category(),
            ErrorCategory::Audit
        );
        assert_eq!(
            OrbitError::Other("x".to_string()).category(),
            ErrorCategory::Unknown
        );
    }

    #[test]
    fn test_error_category_display_all() {
        assert_eq!(ErrorCategory::Validation.to_string(), "validation");
        assert_eq!(ErrorCategory::IoError.to_string(), "io");
        assert_eq!(ErrorCategory::Resource.to_string(), "resource");
        assert_eq!(ErrorCategory::Configuration.to_string(), "configuration");
        assert_eq!(ErrorCategory::Codec.to_string(), "codec");
        assert_eq!(ErrorCategory::Resume.to_string(), "resume");
        assert_eq!(ErrorCategory::Integrity.to_string(), "integrity");
        assert_eq!(ErrorCategory::Filesystem.to_string(), "filesystem");
        assert_eq!(ErrorCategory::Concurrency.to_string(), "concurrency");
        assert_eq!(ErrorCategory::Retry.to_string(), "retry");
        assert_eq!(ErrorCategory::Optimization.to_string(), "optimization");
        assert_eq!(ErrorCategory::Network.to_string(), "network");
        assert_eq!(ErrorCategory::Security.to_string(), "security");
        assert_eq!(ErrorCategory::Metadata.to_string(), "metadata");
        assert_eq!(ErrorCategory::Audit.to_string(), "audit");
        assert_eq!(ErrorCategory::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_display_all_variants() {
        // SourceNotFound
        let err = OrbitError::SourceNotFound(PathBuf::from("/tmp/missing"));
        assert!(err.to_string().contains("Source not found"));
        assert!(
            err.to_string().contains("/tmp/missing") || err.to_string().contains("\\tmp\\missing")
        );

        // InvalidPath
        let err = OrbitError::InvalidPath(PathBuf::from("/bad/path"));
        assert!(err.to_string().contains("Invalid path"));

        // Io
        let err = OrbitError::Io(io::Error::new(io::ErrorKind::NotFound, "file gone"));
        let display = err.to_string();
        assert!(display.contains("I/O error"));
        assert!(display.contains("file gone"));

        // InsufficientDiskSpace
        let err = OrbitError::InsufficientDiskSpace {
            required: 2048,
            available: 512,
        };
        let display = err.to_string();
        assert!(display.contains("Insufficient disk space"));
        assert!(display.contains("2048"));
        assert!(display.contains("512"));

        // Config
        let err = OrbitError::Config("missing key".to_string());
        let display = err.to_string();
        assert!(display.contains("Configuration error"));
        assert!(display.contains("missing key"));

        // Compression
        let err = OrbitError::Compression("lz4 frame error".to_string());
        let display = err.to_string();
        assert!(display.contains("Compression error"));
        assert!(display.contains("lz4 frame error"));

        // Decompression
        let err = OrbitError::Decompression("zstd invalid magic".to_string());
        let display = err.to_string();
        assert!(display.contains("Decompression error"));
        assert!(display.contains("zstd invalid magic"));

        // Resume
        let err = OrbitError::Resume("checkpoint not found".to_string());
        let display = err.to_string();
        assert!(display.contains("Resume error"));
        assert!(display.contains("checkpoint not found"));

        // ChecksumMismatch
        let err = OrbitError::ChecksumMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Checksum verification failed"));
        assert!(display.contains("abc123"));
        assert!(display.contains("def456"));

        // Symlink
        let err = OrbitError::Symlink("broken symlink target".to_string());
        let display = err.to_string();
        assert!(display.contains("Symbolic link error"));
        assert!(display.contains("broken symlink target"));

        // Parallel
        let err = OrbitError::Parallel("worker 3 panicked".to_string());
        let display = err.to_string();
        assert!(display.contains("Parallel processing error"));
        assert!(display.contains("worker 3 panicked"));

        // RetriesExhausted
        let err = OrbitError::RetriesExhausted { attempts: 7 };
        let display = err.to_string();
        assert!(display.contains("7"));
        assert!(display.contains("retry attempts exhausted"));

        // ZeroCopyUnsupported
        let err = OrbitError::ZeroCopyUnsupported;
        assert!(err.to_string().contains("Zero-copy not supported"));

        // Protocol
        let err = OrbitError::Protocol("SMB negotiate failed".to_string());
        let display = err.to_string();
        assert!(display.contains("Protocol error"));
        assert!(display.contains("SMB negotiate failed"));

        // Authentication
        let err = OrbitError::Authentication("invalid token".to_string());
        let display = err.to_string();
        assert!(display.contains("Authentication error"));
        assert!(display.contains("invalid token"));

        // MetadataFailed
        let err = OrbitError::MetadataFailed("utimensat failed".to_string());
        let display = err.to_string();
        assert!(display.contains("Metadata operation failed"));
        assert!(display.contains("utimensat failed"));

        // AuditLog
        let err = OrbitError::AuditLog("log rotation failed".to_string());
        let display = err.to_string();
        assert!(display.contains("Audit log error"));
        assert!(display.contains("log rotation failed"));

        // Other
        let err = OrbitError::Other("something unexpected".to_string());
        assert_eq!(err.to_string(), "something unexpected");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let orbit_err: OrbitError = io_err.into();

        // Should be the Io variant
        match &orbit_err {
            OrbitError::Io(inner) => {
                assert_eq!(inner.kind(), io::ErrorKind::PermissionDenied);
                assert!(inner.to_string().contains("access denied"));
            }
            other => panic!("Expected OrbitError::Io, got {:?}", other),
        }

        // Verify the display includes the original error message
        assert!(orbit_err.to_string().contains("access denied"));
    }

    #[test]
    fn test_from_serde_json_error() {
        // Create a real serde_json::Error by attempting to parse invalid JSON
        let json_err = serde_json::from_str::<serde_json::Value>("not valid json")
            .expect_err("should fail to parse invalid JSON");

        let orbit_err: OrbitError = json_err.into();

        // Should be the Config variant with a JSON parse error message
        match &orbit_err {
            OrbitError::Config(msg) => {
                assert!(
                    msg.contains("JSON parse error"),
                    "Expected message to contain 'JSON parse error', got: {}",
                    msg
                );
            }
            other => panic!("Expected OrbitError::Config, got {:?}", other),
        }

        // Should be categorized as Configuration
        assert_eq!(orbit_err.category(), ErrorCategory::Configuration);

        // Should be fatal (Config errors are fatal)
        assert!(orbit_err.is_fatal());
    }

    #[test]
    fn test_error_source() {
        use std::error::Error;

        // Io variant should return Some source
        let io_err = io::Error::new(io::ErrorKind::BrokenPipe, "pipe broken");
        let orbit_io = OrbitError::Io(io_err);
        let source = orbit_io.source();
        assert!(source.is_some(), "Io variant should have a source");
        assert!(source.unwrap().to_string().contains("pipe broken"));

        // All other variants should return None
        assert!(OrbitError::SourceNotFound(PathBuf::from("/x"))
            .source()
            .is_none());
        assert!(OrbitError::InvalidPath(PathBuf::from("/y"))
            .source()
            .is_none());
        assert!(OrbitError::Config("c".to_string()).source().is_none());
        assert!(OrbitError::Authentication("a".to_string())
            .source()
            .is_none());
        assert!(OrbitError::InsufficientDiskSpace {
            required: 1,
            available: 0
        }
        .source()
        .is_none());
        assert!(OrbitError::Compression("c".to_string()).source().is_none());
        assert!(OrbitError::Decompression("d".to_string())
            .source()
            .is_none());
        assert!(OrbitError::Resume("r".to_string()).source().is_none());
        assert!(OrbitError::ChecksumMismatch {
            expected: "e".to_string(),
            actual: "a".to_string()
        }
        .source()
        .is_none());
        assert!(OrbitError::Symlink("s".to_string()).source().is_none());
        assert!(OrbitError::Parallel("p".to_string()).source().is_none());
        assert!(OrbitError::RetriesExhausted { attempts: 1 }
            .source()
            .is_none());
        assert!(OrbitError::ZeroCopyUnsupported.source().is_none());
        assert!(OrbitError::Protocol("p".to_string()).source().is_none());
        assert!(OrbitError::MetadataFailed("m".to_string())
            .source()
            .is_none());
        assert!(OrbitError::AuditLog("a".to_string()).source().is_none());
        assert!(OrbitError::Other("o".to_string()).source().is_none());
    }
}
