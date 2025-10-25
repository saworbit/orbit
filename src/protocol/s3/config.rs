//! Configuration types for S3 client

use super::error::{S3Error, S3Result};
use super::types::{S3ServerSideEncryption, S3StorageClass};
use serde::{Deserialize, Serialize};

/// S3 client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,
    
    /// AWS region (e.g., "us-east-1")
    pub region: Option<String>,
    
    /// Custom endpoint URL (for S3-compatible services like MinIO)
    pub endpoint: Option<String>,
    
    /// AWS access key ID (optional - uses credential chain if not provided)
    pub access_key: Option<String>,
    
    /// AWS secret access key (optional - uses credential chain if not provided)
    pub secret_key: Option<String>,
    
    /// Session token (for temporary credentials)
    pub session_token: Option<String>,
    
    /// Path-style addressing (required for some S3-compatible services)
    pub force_path_style: bool,
    
    /// Default storage class for uploads
    pub storage_class: S3StorageClass,
    
    /// Server-side encryption
    pub server_side_encryption: S3ServerSideEncryption,
    
    /// Chunk size for multipart uploads (default: 5MB)
    pub chunk_size: usize,
    
    /// Number of parallel operations
    pub parallel_operations: usize,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Enable checksum verification
    pub verify_checksums: bool,
}

impl S3Config {
    /// Create a new S3 config with required parameters
    pub fn new(bucket: String) -> Self {
        Self {
            bucket,
            region: None,
            endpoint: None,
            access_key: None,
            secret_key: None,
            session_token: None,
            force_path_style: false,
            storage_class: S3StorageClass::Standard,
            server_side_encryption: S3ServerSideEncryption::None,
            chunk_size: super::DEFAULT_CHUNK_SIZE,
            parallel_operations: super::DEFAULT_PARALLEL_PARTS,
            timeout_seconds: 300, // 5 minutes
            max_retries: 3,
            verify_checksums: true,
        }
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> S3Result<()> {
        // Validate bucket name
        if self.bucket.is_empty() {
            return Err(S3Error::InvalidBucketName("Bucket name cannot be empty".to_string()));
        }
        
        if !is_valid_bucket_name(&self.bucket) {
            return Err(S3Error::InvalidBucketName(format!(
                "Invalid bucket name: {}. Bucket names must be 3-63 characters, \
                 lowercase letters, numbers, hyphens, and periods only",
                self.bucket
            )));
        }
        
        // Validate chunk size
        if self.chunk_size < super::MIN_CHUNK_SIZE {
            return Err(S3Error::InvalidConfig(format!(
                "Chunk size {} is below minimum {}",
                self.chunk_size,
                super::MIN_CHUNK_SIZE
            )));
        }
        
        if self.chunk_size > super::MAX_CHUNK_SIZE {
            return Err(S3Error::InvalidConfig(format!(
                "Chunk size {} exceeds maximum {}",
                self.chunk_size,
                super::MAX_CHUNK_SIZE
            )));
        }
        
        // Validate parallel operations
        if self.parallel_operations == 0 {
            return Err(S3Error::InvalidConfig(
                "Parallel operations must be at least 1".to_string()
            ));
        }
        
        if self.parallel_operations > super::MAX_PARALLEL_OPERATIONS {
            return Err(S3Error::InvalidConfig(format!(
                "Parallel operations {} exceeds maximum {}",
                self.parallel_operations,
                super::MAX_PARALLEL_OPERATIONS
            )));
        }
        
        // Validate credentials consistency
        if self.access_key.is_some() != self.secret_key.is_some() {
            return Err(S3Error::InvalidConfig(
                "Both access_key and secret_key must be provided together".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Check if using custom endpoint (S3-compatible service)
    pub fn is_custom_endpoint(&self) -> bool {
        self.endpoint.is_some()
    }
    
    /// Check if using explicit credentials
    pub fn has_explicit_credentials(&self) -> bool {
        self.access_key.is_some() && self.secret_key.is_some()
    }
}

impl Default for S3Config {
    fn default() -> Self {
        Self::new("".to_string())
    }
}

/// Builder for S3Config
pub struct S3ConfigBuilder {
    config: S3Config,
}

impl S3ConfigBuilder {
    /// Create a new builder with bucket name
    pub fn new(bucket: String) -> Self {
        Self {
            config: S3Config::new(bucket),
        }
    }
    
    /// Set the AWS region
    pub fn region(mut self, region: String) -> Self {
        self.config.region = Some(region);
        self
    }
    
    /// Set custom endpoint (for MinIO, LocalStack, etc.)
    pub fn endpoint(mut self, endpoint: String) -> Self {
        self.config.endpoint = Some(endpoint);
        self
    }
    
    /// Set AWS credentials explicitly
    pub fn credentials(mut self, access_key: String, secret_key: String) -> Self {
        self.config.access_key = Some(access_key);
        self.config.secret_key = Some(secret_key);
        self
    }
    
    /// Set session token (for temporary credentials)
    pub fn session_token(mut self, token: String) -> Self {
        self.config.session_token = Some(token);
        self
    }
    
    /// Enable path-style addressing
    pub fn force_path_style(mut self, force: bool) -> Self {
        self.config.force_path_style = force;
        self
    }
    
    /// Set default storage class
    pub fn storage_class(mut self, storage_class: S3StorageClass) -> Self {
        self.config.storage_class = storage_class;
        self
    }
    
    /// Set server-side encryption
    pub fn server_side_encryption(mut self, sse: S3ServerSideEncryption) -> Self {
        self.config.server_side_encryption = sse;
        self
    }
    
    /// Set chunk size for multipart uploads
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.config.chunk_size = size;
        self
    }
    
    /// Set number of parallel operations
    pub fn parallel_operations(mut self, count: usize) -> Self {
        self.config.parallel_operations = count;
        self
    }
    
    /// Set request timeout
    pub fn timeout_seconds(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = seconds;
        self
    }
    
    /// Set maximum retry attempts
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }
    
    /// Enable or disable checksum verification
    pub fn verify_checksums(mut self, verify: bool) -> Self {
        self.config.verify_checksums = verify;
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> S3Result<S3Config> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Validate S3 bucket name according to AWS rules
fn is_valid_bucket_name(name: &str) -> bool {
    let len = name.len();
    
    // Length check: 3-63 characters
    if len < 3 || len > 63 {
        return false;
    }
    
    // Must start and end with lowercase letter or number
    let first = name.chars().next().unwrap();
    let last = name.chars().last().unwrap();
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    if !last.is_ascii_lowercase() && !last.is_ascii_digit() {
        return false;
    }
    
    // Only lowercase letters, numbers, hyphens, and periods
    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' && c != '.' {
            return false;
        }
    }
    
    // Cannot have consecutive periods
    if name.contains("..") {
        return false;
    }
    
    // Cannot be formatted as IP address
    if name.split('.').count() == 4 && name.split('.').all(|s| s.parse::<u8>().is_ok()) {
        return false;
    }
    
    // Cannot start with "xn--" (reserved for internationalized domain names)
    if name.starts_with("xn--") {
        return false;
    }
    
    // Cannot end with "-s3alias" (reserved)
    if name.ends_with("-s3alias") {
        return false;
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_bucket_names() {
        assert!(is_valid_bucket_name("my-bucket"));
        assert!(is_valid_bucket_name("my.bucket"));
        assert!(is_valid_bucket_name("my-bucket-123"));
        assert!(is_valid_bucket_name("abc"));
        assert!(is_valid_bucket_name("a" .repeat(63).as_str()));
    }

    #[test]
    fn test_invalid_bucket_names() {
        assert!(!is_valid_bucket_name("ab")); // Too short
        assert!(!is_valid_bucket_name(&"a".repeat(64))); // Too long
        assert!(!is_valid_bucket_name("My-Bucket")); // Uppercase
        assert!(!is_valid_bucket_name("my_bucket")); // Underscore
        assert!(!is_valid_bucket_name("my..bucket")); // Consecutive periods
        assert!(!is_valid_bucket_name("192.168.1.1")); // IP address format
        assert!(!is_valid_bucket_name("xn--bucket")); // Reserved prefix
        assert!(!is_valid_bucket_name("bucket-s3alias")); // Reserved suffix
        assert!(!is_valid_bucket_name("-bucket")); // Starts with hyphen
        assert!(!is_valid_bucket_name("bucket-")); // Ends with hyphen
    }

    #[test]
    fn test_config_validation() {
        let config = S3Config::new("valid-bucket".to_string());
        assert!(config.validate().is_ok());
        
        let config = S3Config::new("".to_string());
        assert!(config.validate().is_err());
        
        let mut config = S3Config::new("valid-bucket".to_string());
        config.chunk_size = 1024; // Too small
        assert!(config.validate().is_err());
        
        let mut config = S3Config::new("valid-bucket".to_string());
        config.parallel_operations = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = S3ConfigBuilder::new("test-bucket".to_string())
            .region("us-west-2".to_string())
            .chunk_size(10 * 1024 * 1024)
            .parallel_operations(8)
            .verify_checksums(true)
            .build()
            .unwrap();
        
        assert_eq!(config.bucket, "test-bucket");
        assert_eq!(config.region, Some("us-west-2".to_string()));
        assert_eq!(config.chunk_size, 10 * 1024 * 1024);
        assert_eq!(config.parallel_operations, 8);
        assert!(config.verify_checksums);
    }

    #[test]
    fn test_credentials_consistency() {
        let mut config = S3Config::new("test-bucket".to_string());
        config.access_key = Some("key".to_string());
        // Missing secret_key
        assert!(config.validate().is_err());
        
        config.secret_key = Some("secret".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_custom_endpoint_detection() {
        let mut config = S3Config::new("test-bucket".to_string());
        assert!(!config.is_custom_endpoint());
        
        config.endpoint = Some("http://localhost:9000".to_string());
        assert!(config.is_custom_endpoint());
    }

    #[test]
    fn test_explicit_credentials_detection() {
        let mut config = S3Config::new("test-bucket".to_string());
        assert!(!config.has_explicit_credentials());
        
        config.access_key = Some("key".to_string());
        config.secret_key = Some("secret".to_string());
        assert!(config.has_explicit_credentials());
    }
}