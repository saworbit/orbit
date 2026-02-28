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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum S3StorageClass {
    /// Standard storage class
    #[default]
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

    /// Check if this storage class is a Glacier tier
    pub fn is_glacier(&self) -> bool {
        matches!(
            self,
            S3StorageClass::GlacierInstantRetrieval
                | S3StorageClass::GlacierFlexibleRetrieval
                | S3StorageClass::GlacierDeepArchive
        )
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum S3ServerSideEncryption {
    /// AES256 encryption
    Aes256,

    /// AWS KMS encryption
    AwsKms { key_id: Option<String> },

    /// No encryption
    #[default]
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
    fn test_is_glacier() {
        assert!(S3StorageClass::GlacierInstantRetrieval.is_glacier());
        assert!(S3StorageClass::GlacierFlexibleRetrieval.is_glacier());
        assert!(S3StorageClass::GlacierDeepArchive.is_glacier());
        assert!(!S3StorageClass::Standard.is_glacier());
        assert!(!S3StorageClass::StandardIa.is_glacier());
        assert!(!S3StorageClass::OnezoneIa.is_glacier());
        assert!(!S3StorageClass::IntelligentTiering.is_glacier());
        assert!(!S3StorageClass::ReducedRedundancy.is_glacier());
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

    #[test]
    fn test_storage_class_display_all() {
        assert_eq!(S3StorageClass::Standard.to_string(), "STANDARD");
        assert_eq!(
            S3StorageClass::ReducedRedundancy.to_string(),
            "REDUCED_REDUNDANCY"
        );
        assert_eq!(S3StorageClass::StandardIa.to_string(), "STANDARD_IA");
        assert_eq!(S3StorageClass::OnezoneIa.to_string(), "ONEZONE_IA");
        assert_eq!(
            S3StorageClass::IntelligentTiering.to_string(),
            "INTELLIGENT_TIERING"
        );
        assert_eq!(
            S3StorageClass::GlacierInstantRetrieval.to_string(),
            "GLACIER_IR"
        );
        assert_eq!(
            S3StorageClass::GlacierFlexibleRetrieval.to_string(),
            "GLACIER"
        );
        assert_eq!(
            S3StorageClass::GlacierDeepArchive.to_string(),
            "DEEP_ARCHIVE"
        );
    }

    #[test]
    fn test_storage_class_to_aws_all() {
        assert_eq!(
            S3StorageClass::Standard.to_aws(),
            aws_sdk_s3::types::StorageClass::Standard
        );
        assert_eq!(
            S3StorageClass::ReducedRedundancy.to_aws(),
            aws_sdk_s3::types::StorageClass::ReducedRedundancy
        );
        assert_eq!(
            S3StorageClass::StandardIa.to_aws(),
            aws_sdk_s3::types::StorageClass::StandardIa
        );
        assert_eq!(
            S3StorageClass::OnezoneIa.to_aws(),
            aws_sdk_s3::types::StorageClass::OnezoneIa
        );
        assert_eq!(
            S3StorageClass::IntelligentTiering.to_aws(),
            aws_sdk_s3::types::StorageClass::IntelligentTiering
        );
        assert_eq!(
            S3StorageClass::GlacierInstantRetrieval.to_aws(),
            aws_sdk_s3::types::StorageClass::GlacierIr
        );
        assert_eq!(
            S3StorageClass::GlacierFlexibleRetrieval.to_aws(),
            aws_sdk_s3::types::StorageClass::Glacier
        );
        assert_eq!(
            S3StorageClass::GlacierDeepArchive.to_aws(),
            aws_sdk_s3::types::StorageClass::DeepArchive
        );
    }

    #[test]
    fn test_storage_class_from_aws_all() {
        assert_eq!(
            S3StorageClass::from_aws(&aws_sdk_s3::types::StorageClass::Standard),
            S3StorageClass::Standard
        );
        assert_eq!(
            S3StorageClass::from_aws(&aws_sdk_s3::types::StorageClass::ReducedRedundancy),
            S3StorageClass::ReducedRedundancy
        );
        assert_eq!(
            S3StorageClass::from_aws(&aws_sdk_s3::types::StorageClass::StandardIa),
            S3StorageClass::StandardIa
        );
        assert_eq!(
            S3StorageClass::from_aws(&aws_sdk_s3::types::StorageClass::OnezoneIa),
            S3StorageClass::OnezoneIa
        );
        assert_eq!(
            S3StorageClass::from_aws(&aws_sdk_s3::types::StorageClass::IntelligentTiering),
            S3StorageClass::IntelligentTiering
        );
        assert_eq!(
            S3StorageClass::from_aws(&aws_sdk_s3::types::StorageClass::GlacierIr),
            S3StorageClass::GlacierInstantRetrieval
        );
        assert_eq!(
            S3StorageClass::from_aws(&aws_sdk_s3::types::StorageClass::Glacier),
            S3StorageClass::GlacierFlexibleRetrieval
        );
        assert_eq!(
            S3StorageClass::from_aws(&aws_sdk_s3::types::StorageClass::DeepArchive),
            S3StorageClass::GlacierDeepArchive
        );
    }

    #[test]
    fn test_storage_class_from_aws_unknown() {
        // Use an unknown variant to test the default fallback
        let unknown = aws_sdk_s3::types::StorageClass::from("SOME_UNKNOWN_CLASS");
        assert_eq!(S3StorageClass::from_aws(&unknown), S3StorageClass::Standard);
    }

    #[test]
    fn test_server_side_encryption_to_aws() {
        assert_eq!(
            S3ServerSideEncryption::Aes256.to_aws(),
            Some(aws_sdk_s3::types::ServerSideEncryption::Aes256)
        );
        assert_eq!(
            S3ServerSideEncryption::AwsKms {
                key_id: Some("my-key".to_string())
            }
            .to_aws(),
            Some(aws_sdk_s3::types::ServerSideEncryption::AwsKms)
        );
        assert_eq!(S3ServerSideEncryption::None.to_aws(), None);
    }

    #[test]
    fn test_upload_part_info_new() {
        let info = UploadPartInfo::new(3, "etag-abc".to_string(), 1024);
        assert_eq!(info.part_number, 3);
        assert_eq!(info.etag, "etag-abc");
        assert_eq!(info.size, 1024);
    }

    #[test]
    fn test_resume_state_new() {
        let state = ResumeState::new("upload-xyz".to_string(), 50000, 5242880);
        assert_eq!(state.upload_id, Some("upload-xyz".to_string()));
        assert!(state.completed_parts.is_empty());
        assert_eq!(state.total_size, 50000);
        assert_eq!(state.chunk_size, 5242880);
        assert_eq!(state.etag, None);
    }

    #[test]
    fn test_s3_object_serialization() {
        let obj = S3Object {
            key: "test/file.txt".to_string(),
            size: 12345,
            last_modified: None,
            etag: Some("abc123".to_string()),
            storage_class: Some(S3StorageClass::Standard),
            content_type: Some("text/plain".to_string()),
        };

        let json = serde_json::to_string(&obj).expect("Failed to serialize S3Object");
        let deserialized: S3Object =
            serde_json::from_str(&json).expect("Failed to deserialize S3Object");

        assert_eq!(deserialized.key, "test/file.txt");
        assert_eq!(deserialized.size, 12345);
        assert_eq!(deserialized.etag, Some("abc123".to_string()));
        assert_eq!(deserialized.storage_class, Some(S3StorageClass::Standard));
        assert_eq!(deserialized.content_type, Some("text/plain".to_string()));
    }
}
