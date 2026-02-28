//! Error types for S3 operations

use std::io;
use thiserror::Error;

/// Result type alias for S3 operations
pub type S3Result<T> = Result<T, S3Error>;

/// Errors that can occur during S3 operations
#[derive(Error, Debug, Clone)]
pub enum S3Error {
    /// AWS SDK error
    #[error("AWS SDK error: {0}")]
    Sdk(String),

    /// S3 service error with specific error code
    #[error("S3 service error ({code}): {message}")]
    Service { code: String, message: String },

    /// Object not found in bucket
    #[error("Object not found: {bucket}/{key}")]
    NotFound { bucket: String, key: String },

    /// Bucket not found or not accessible
    #[error("Bucket not found or not accessible: {0}")]
    BucketNotFound(String),

    /// Access denied error
    #[error("Access denied: {0}")]
    AccessDenied(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Invalid bucket name
    #[error("Invalid bucket name: {0}")]
    InvalidBucketName(String),

    /// Invalid object key
    #[error("Invalid object key: {0}")]
    InvalidKey(String),

    /// Multipart upload error
    #[error("Multipart upload error: {0}")]
    MultipartUpload(String),

    /// Resume state error
    #[error("Resume state error: {0}")]
    ResumeState(String),

    /// Checksum mismatch
    #[error("Checksum mismatch for {key}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        key: String,
        expected: String,
        actual: String,
    },

    /// I/O error
    #[error("I/O error: {0}")]
    Io(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Storage quota exceeded
    #[error("Storage quota exceeded: {0}")]
    QuotaExceeded(String),

    /// Invalid range request
    #[error("Invalid range: {0}")]
    InvalidRange(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Generic error with context
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        source: Box<S3Error>,
    },
}

impl S3Error {
    /// Add context to an error
    pub fn context<S: Into<String>>(self, context: S) -> Self {
        S3Error::WithContext {
            context: context.into(),
            source: Box::new(self),
        }
    }

    /// Create an error from AWS SDK error
    pub fn from_sdk<E: std::error::Error>(error: E) -> Self {
        S3Error::Sdk(error.to_string())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            S3Error::Network(_) => true,
            S3Error::Timeout(_) => true,
            S3Error::RateLimitExceeded(_) => true,
            S3Error::Io(_) => true, // I/O errors are generally retryable
            // Authentication errors are NOT retryable
            S3Error::Authentication(_) => false,
            // SDK errors: check for network-related strings
            S3Error::Sdk(msg) => {
                let lower = msg.to_lowercase();
                lower.contains("connection reset")
                    || lower.contains("connection timed out")
                    || lower.contains("broken pipe")
                    || lower.contains("connection refused")
                    || lower.contains("temporarily unavailable")
            }
            S3Error::Service { code, .. } => is_retryable_code(code),
            // Unwrap context to check inner error
            S3Error::WithContext { source, .. } => source.is_retryable(),
            _ => false,
        }
    }

    /// Check if error is transient (safe to retry)
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            S3Error::Network(_)
                | S3Error::Timeout(_)
                | S3Error::RateLimitExceeded(_)
                | S3Error::Io(_)
        )
    }
}

// Convert io::Error to S3Error
impl From<io::Error> for S3Error {
    fn from(err: io::Error) -> Self {
        S3Error::Io(err.to_string())
    }
}

/// Check if an AWS error code is retryable
pub(crate) fn is_retryable_code(code: &str) -> bool {
    matches!(
        code,
        "RequestTimeout"
            | "ServiceUnavailable"
            | "InternalError"
            | "SlowDown"
            | "RequestTimeTooSkewed"
    )
}

/// Convert AWS SDK errors to S3Error
impl<E> From<aws_sdk_s3::error::SdkError<E>> for S3Error
where
    E: std::error::Error + 'static,
{
    fn from(error: aws_sdk_s3::error::SdkError<E>) -> Self {
        match error {
            aws_sdk_s3::error::SdkError::DispatchFailure(e) => {
                S3Error::Network(format!("Network dispatch failure: {:?}", e))
            }
            aws_sdk_s3::error::SdkError::ResponseError(e) => {
                S3Error::Network(format!("Response error: {:?}", e))
            }
            aws_sdk_s3::error::SdkError::ServiceError(e) => {
                let err_str = format!("{:?}", e);

                // Check for common error patterns
                if err_str.contains("NoSuchKey") {
                    S3Error::Service {
                        code: "NoSuchKey".to_string(),
                        message: "The specified key does not exist".to_string(),
                    }
                } else if err_str.contains("NoSuchBucket") {
                    S3Error::Service {
                        code: "NoSuchBucket".to_string(),
                        message: "The specified bucket does not exist".to_string(),
                    }
                } else if err_str.contains("AccessDenied") {
                    S3Error::AccessDenied("Access denied to resource".to_string())
                } else {
                    S3Error::Service {
                        code: "Unknown".to_string(),
                        message: err_str,
                    }
                }
            }
            _ => S3Error::Sdk(format!("{:?}", error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let base_error = S3Error::InvalidKey("test".to_string());
        let with_context = base_error.context("Failed to upload");

        assert!(matches!(with_context, S3Error::WithContext { .. }));
    }

    #[test]
    fn test_retryable_errors() {
        assert!(S3Error::Network("connection lost".to_string()).is_retryable());
        assert!(S3Error::Timeout("timed out".to_string()).is_retryable());
        assert!(S3Error::RateLimitExceeded("too many requests".to_string()).is_retryable());
        assert!(!S3Error::InvalidKey("bad key".to_string()).is_retryable());
    }

    #[test]
    fn test_authentication_not_retryable() {
        assert!(!S3Error::Authentication("bad credentials".to_string()).is_retryable());
    }

    #[test]
    fn test_sdk_network_errors_retryable() {
        assert!(S3Error::Sdk("connection reset by peer".to_string()).is_retryable());
        assert!(S3Error::Sdk("Connection timed out".to_string()).is_retryable());
        assert!(S3Error::Sdk("broken pipe".to_string()).is_retryable());
        assert!(S3Error::Sdk("Connection refused".to_string()).is_retryable());
        assert!(S3Error::Sdk("resource temporarily unavailable".to_string()).is_retryable());
        // Non-network SDK errors should not be retryable
        assert!(!S3Error::Sdk("invalid argument".to_string()).is_retryable());
    }

    #[test]
    fn test_with_context_retryable_unwrap() {
        let inner = S3Error::Network("connection lost".to_string());
        let wrapped = S3Error::WithContext {
            context: "during upload".to_string(),
            source: Box::new(inner),
        };
        assert!(wrapped.is_retryable());

        let inner_non_retryable = S3Error::InvalidKey("bad".to_string());
        let wrapped_non_retryable = S3Error::WithContext {
            context: "during upload".to_string(),
            source: Box::new(inner_non_retryable),
        };
        assert!(!wrapped_non_retryable.is_retryable());
    }

    #[test]
    fn test_transient_errors() {
        assert!(S3Error::Network("network error".to_string()).is_transient());
        assert!(S3Error::Timeout("timeout".to_string()).is_transient());
        assert!(!S3Error::InvalidConfig("bad config".to_string()).is_transient());
    }

    #[test]
    fn test_retryable_codes() {
        assert!(is_retryable_code("RequestTimeout"));
        assert!(is_retryable_code("ServiceUnavailable"));
        assert!(is_retryable_code("InternalError"));
        assert!(is_retryable_code("SlowDown"));
        assert!(!is_retryable_code("NoSuchKey"));
        assert!(!is_retryable_code("AccessDenied"));
    }

    #[test]
    fn test_service_error_retryable() {
        let err = S3Error::Service {
            code: "RequestTimeout".to_string(),
            message: "timed out".to_string(),
        };
        assert!(err.is_retryable());

        let err = S3Error::Service {
            code: "ServiceUnavailable".to_string(),
            message: "503".to_string(),
        };
        assert!(err.is_retryable());

        let err = S3Error::Service {
            code: "InternalError".to_string(),
            message: "500".to_string(),
        };
        assert!(err.is_retryable());

        let err = S3Error::Service {
            code: "SlowDown".to_string(),
            message: "slow".to_string(),
        };
        assert!(err.is_retryable());

        let err = S3Error::Service {
            code: "RequestTimeTooSkewed".to_string(),
            message: "skew".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_service_error_not_retryable() {
        let err = S3Error::Service {
            code: "NoSuchKey".to_string(),
            message: "not found".to_string(),
        };
        assert!(!err.is_retryable());

        let err = S3Error::Service {
            code: "AccessDenied".to_string(),
            message: "denied".to_string(),
        };
        assert!(!err.is_retryable());

        let err = S3Error::Service {
            code: "NoSuchBucket".to_string(),
            message: "gone".to_string(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_io_error_retryable() {
        assert!(S3Error::Io("disk error".to_string()).is_retryable());
    }

    #[test]
    fn test_transient_all_types() {
        assert!(S3Error::RateLimitExceeded("rate".to_string()).is_transient());
        assert!(S3Error::Io("io".to_string()).is_transient());
        assert!(!S3Error::Authentication("auth".to_string()).is_transient());
        assert!(!S3Error::Sdk("sdk".to_string()).is_transient());
        assert!(
            !S3Error::Service {
                code: "InternalError".to_string(),
                message: "err".to_string()
            }
            .is_transient()
        );
        assert!(!S3Error::InvalidBucketName("bad".to_string()).is_transient());
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let s3_err: S3Error = io_err.into();
        assert!(matches!(s3_err, S3Error::Io(_)));
    }

    #[test]
    fn test_from_sdk_helper() {
        let err = S3Error::from_sdk(std::io::Error::other("test"));
        assert!(matches!(err, S3Error::Sdk(_)));
    }

    #[test]
    fn test_error_display_formats() {
        let err = S3Error::Network("connection lost".to_string());
        assert_eq!(format!("{}", err), "Network error: connection lost");

        let err = S3Error::Timeout("30s elapsed".to_string());
        assert_eq!(format!("{}", err), "Operation timed out: 30s elapsed");

        let err = S3Error::Service {
            code: "SlowDown".to_string(),
            message: "rate limited".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "S3 service error (SlowDown): rate limited"
        );

        let err = S3Error::NotFound {
            bucket: "my-bucket".to_string(),
            key: "my-key".to_string(),
        };
        assert_eq!(format!("{}", err), "Object not found: my-bucket/my-key");

        let err = S3Error::Io("disk full".to_string());
        assert_eq!(format!("{}", err), "I/O error: disk full");

        let err = S3Error::AccessDenied("no perms".to_string());
        assert_eq!(format!("{}", err), "Access denied: no perms");

        let err = S3Error::InvalidBucketName("bad!name".to_string());
        assert_eq!(format!("{}", err), "Invalid bucket name: bad!name");

        let err = S3Error::Authentication("expired token".to_string());
        assert_eq!(format!("{}", err), "Authentication error: expired token");

        let err = S3Error::Sdk("some sdk error".to_string());
        assert_eq!(format!("{}", err), "AWS SDK error: some sdk error");

        let err = S3Error::ChecksumMismatch {
            key: "obj".to_string(),
            expected: "aaa".to_string(),
            actual: "bbb".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Checksum mismatch for obj: expected aaa, got bbb"
        );
    }
}
