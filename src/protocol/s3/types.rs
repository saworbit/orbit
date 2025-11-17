//! Type definitions for S3 operations

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// S3 object metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Object {
    /// Object key (path within bucket)
    pub key: String,

    /// Object size in bytes
    pub size: u64,

    /// Last modified timestamp
    pub last_modified: Option<SystemTime>,

    /// ETag (entity tag) - often MD5 hash
    pub etag: Option<String>,

    /// Storage class
    pub storage_class: Option<S3StorageClass>,

    /// Content type
    pub content_type: Option<String>,
}

/// Detailed S3 object metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ObjectMetadata {
    /// Object key
    pub key: String,

    /// Object size in bytes
    pub size: u64,

    /// Last modified timestamp
    pub last_modified: Option<SystemTime>,

    /// ETag
    pub etag: Option<String>,

    /// Storage class
    pub storage_class: Option<S3StorageClass>,

    /// Content type
    pub content_type: Option<String>,

    /// Content encoding
    pub content_encoding: Option<String>,

    /// Cache control
    pub cache_control: Option<String>,

    /// Content disposition
    pub content_disposition: Option<String>,

    /// User-defined metadata
    pub metadata: std::collections::HashMap<String, String>,

    /// Server-side encryption
    pub server_side_encryption: Option<S3ServerSideEncryption>,

    /// Version ID (if versioning is enabled)
    pub version_id: Option<String>,
}

/// Result of listing objects
#[derive(Debug, Clone)]
pub struct S3ListResult {
    /// List of objects
    pub objects: Vec<S3Object>,

    /// Common prefixes (directories)
    pub common_prefixes: Vec<String>,

    /// Continuation token for pagination
    pub continuation_token: Option<String>,

    /// Whether the result is truncated
    pub is_truncated: bool,
}

/// S3 storage classes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum S3StorageClass {
    /// Standard storage class
    Standard,

    /// Reduced redundancy (deprecated but still available)
    ReducedRedundancy,

    /// Infrequent access
    StandardIa,

    /// One zone infrequent access
    OnezoneIa,

    /// Intelligent tiering
    IntelligentTiering,

    /// Glacier instant retrieval
    GlacierInstantRetrieval,

    /// Glacier flexible retrieval
    GlacierFlexibleRetrieval,

    /// Glacier deep archive
    GlacierDeepArchive,
}

impl S3StorageClass {
    /// Convert to AWS SDK storage class
    pub fn to_aws(&self) -> aws_sdk_s3::types::StorageClass {
        match self {
            S3StorageClass::Standard => aws_sdk_s3::types::StorageClass::Standard,
            S3StorageClass::ReducedRedundancy => aws_sdk_s3::types::StorageClass::ReducedRedundancy,
            S3StorageClass::StandardIa => aws_sdk_s3::types::StorageClass::StandardIa,
            S3StorageClass::OnezoneIa => aws_sdk_s3::types::StorageClass::OnezoneIa,
            S3StorageClass::IntelligentTiering => {
                aws_sdk_s3::types::StorageClass::IntelligentTiering
            }
            S3StorageClass::GlacierInstantRetrieval => aws_sdk_s3::types::StorageClass::GlacierIr,
            S3StorageClass::GlacierFlexibleRetrieval => aws_sdk_s3::types::StorageClass::Glacier,
            S3StorageClass::GlacierDeepArchive => aws_sdk_s3::types::StorageClass::DeepArchive,
        }
    }

    /// Convert from AWS SDK storage class
    pub fn from_aws(sc: &aws_sdk_s3::types::StorageClass) -> Self {
        match sc {
            aws_sdk_s3::types::StorageClass::Standard => S3StorageClass::Standard,
            aws_sdk_s3::types::StorageClass::ReducedRedundancy => S3StorageClass::ReducedRedundancy,
            aws_sdk_s3::types::StorageClass::StandardIa => S3StorageClass::StandardIa,
            aws_sdk_s3::types::StorageClass::OnezoneIa => S3StorageClass::OnezoneIa,
            aws_sdk_s3::types::StorageClass::IntelligentTiering => {
                S3StorageClass::IntelligentTiering
            }
            aws_sdk_s3::types::StorageClass::GlacierIr => S3StorageClass::GlacierInstantRetrieval,
            aws_sdk_s3::types::StorageClass::Glacier => S3StorageClass::GlacierFlexibleRetrieval,
            aws_sdk_s3::types::StorageClass::DeepArchive => S3StorageClass::GlacierDeepArchive,
            _ => S3StorageClass::Standard, // Default for unknown types
        }
    }
}

impl Default for S3StorageClass {
    fn default() -> Self {
        S3StorageClass::Standard
    }
}

impl std::fmt::Display for S3StorageClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S3StorageClass::Standard => write!(f, "STANDARD"),
            S3StorageClass::ReducedRedundancy => write!(f, "REDUCED_REDUNDANCY"),
            S3StorageClass::StandardIa => write!(f, "STANDARD_IA"),
            S3StorageClass::OnezoneIa => write!(f, "ONEZONE_IA"),
            S3StorageClass::IntelligentTiering => write!(f, "INTELLIGENT_TIERING"),
            S3StorageClass::GlacierInstantRetrieval => write!(f, "GLACIER_IR"),
            S3StorageClass::GlacierFlexibleRetrieval => write!(f, "GLACIER"),
            S3StorageClass::GlacierDeepArchive => write!(f, "DEEP_ARCHIVE"),
        }
    }
}

/// S3 server-side encryption options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum S3ServerSideEncryption {
    /// AES256 encryption
    Aes256,

    /// AWS KMS encryption
    AwsKms { key_id: Option<String> },

    /// No encryption
    None,
}

impl S3ServerSideEncryption {
    /// Convert to AWS SDK server-side encryption
    pub fn to_aws(&self) -> Option<aws_sdk_s3::types::ServerSideEncryption> {
        match self {
            S3ServerSideEncryption::Aes256 => Some(aws_sdk_s3::types::ServerSideEncryption::Aes256),
            S3ServerSideEncryption::AwsKms { .. } => {
                Some(aws_sdk_s3::types::ServerSideEncryption::AwsKms)
            }
            S3ServerSideEncryption::None => None,
        }
    }
}

impl Default for S3ServerSideEncryption {
    fn default() -> Self {
        S3ServerSideEncryption::None
    }
}

/// Resume state for multipart uploads/downloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeState {
    /// Upload ID for multipart upload
    pub upload_id: Option<String>,

    /// Completed parts information
    pub completed_parts: Vec<UploadPartInfo>,

    /// Total size of the object
    pub total_size: u64,

    /// Chunk size used
    pub chunk_size: usize,

    /// ETag of the object (for validation)
    pub etag: Option<String>,
}

impl ResumeState {
    /// Create a new resume state
    pub fn new(upload_id: String, total_size: u64, chunk_size: usize) -> Self {
        Self {
            upload_id: Some(upload_id),
            completed_parts: Vec::new(),
            total_size,
            chunk_size,
            etag: None,
        }
    }

    /// Check if any parts have been uploaded
    pub fn has_progress(&self) -> bool {
        !self.completed_parts.is_empty()
    }

    /// Get the total bytes uploaded
    pub fn bytes_uploaded(&self) -> u64 {
        self.completed_parts.iter().map(|p| p.size as u64).sum()
    }

    /// Get the next part number to upload
    pub fn next_part_number(&self) -> i32 {
        self.completed_parts
            .iter()
            .map(|p| p.part_number)
            .max()
            .unwrap_or(0)
            + 1
    }
}

/// Information about an uploaded part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadPartInfo {
    /// Part number (1-indexed)
    pub part_number: i32,

    /// ETag of the uploaded part
    pub etag: String,

    /// Size of the part in bytes
    pub size: usize,
}

impl UploadPartInfo {
    /// Create a new upload part info
    pub fn new(part_number: i32, etag: String, size: usize) -> Self {
        Self {
            part_number,
            etag,
            size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_class_display() {
        assert_eq!(S3StorageClass::Standard.to_string(), "STANDARD");
        assert_eq!(S3StorageClass::StandardIa.to_string(), "STANDARD_IA");
        assert_eq!(
            S3StorageClass::GlacierInstantRetrieval.to_string(),
            "GLACIER_IR"
        );
    }

    #[test]
    fn test_storage_class_default() {
        let sc: S3StorageClass = Default::default();
        assert_eq!(sc, S3StorageClass::Standard);
    }

    #[test]
    fn test_resume_state_progress() {
        let mut state = ResumeState::new("upload123".to_string(), 10000, 5242880);
        assert!(!state.has_progress());
        assert_eq!(state.bytes_uploaded(), 0);

        state
            .completed_parts
            .push(UploadPartInfo::new(1, "etag1".to_string(), 5242880));
        assert!(state.has_progress());
        assert_eq!(state.bytes_uploaded(), 5242880);
        assert_eq!(state.next_part_number(), 2);
    }

    #[test]
    fn test_server_side_encryption_default() {
        let sse: S3ServerSideEncryption = Default::default();
        assert_eq!(sse, S3ServerSideEncryption::None);
    }
}
