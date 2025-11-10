//! Error types for S3 operations

use thiserror::Error;
use std::io;

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
            S3Error::Network(_) | S3Error::Timeout(_) | S3Error::RateLimitExceeded(_) => true,
            S3Error::Service { code, .. } => is_retryable_code(code),
            S3Error::Io(_) => true,  // I/O errors are generally retryable
            _ => false,
        }
    }

    /// Check if error is transient (safe to retry)
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            S3Error::Network(_) | S3Error::Timeout(_) | S3Error::RateLimitExceeded(_) | S3Error::Io(_)
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
fn is_retryable_code(code: &str) -> bool {
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
}