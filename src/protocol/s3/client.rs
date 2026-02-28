//! S3 client implementation

use super::config::S3Config;
use super::error::{S3Error, S3Result};
use super::types::S3ObjectMetadata;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::Client as AwsS3Client;
use std::time::Duration;
use std::time::SystemTime;

/// S3 client for interacting with AWS S3 and S3-compatible storage
#[derive(Clone)]
pub struct S3Client {
    /// AWS S3 client
    client: AwsS3Client,

    /// Client configuration
    config: S3Config,
}

impl S3Client {
    /// Create a new S3 client with the given configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use orbit::protocol::s3::{S3Client, S3Config};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = S3Config::new("my-bucket".to_string());
    ///     let client = S3Client::new(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(config: S3Config) -> S3Result<Self> {
        // Validate configuration
        config.validate()?;

        // Build AWS SDK client
        let client = Self::build_aws_client(&config).await?;

        Ok(Self { client, config })
    }

    /// Build the AWS SDK S3 client from configuration
    async fn build_aws_client(config: &S3Config) -> S3Result<AwsS3Client> {
        // Start with default AWS config loader
        let mut aws_config_loader = aws_config::defaults(BehaviorVersion::latest());

        // Set region
        let region_provider = if let Some(region_str) = &config.region {
            RegionProviderChain::first_try(Region::new(region_str.clone()))
        } else {
            RegionProviderChain::default_provider()
        };
        aws_config_loader = aws_config_loader.region(region_provider);

        // Phase 4B: Set AWS profile if specified
        if let Some(ref profile) = config.aws_profile {
            aws_config_loader = aws_config_loader.profile_name(profile);
        }

        // Phase 4A: Handle no-sign-request (anonymous credentials for public buckets)
        if config.no_sign_request {
            let anonymous_credentials = Credentials::new("", "", None, None, "anonymous");
            aws_config_loader = aws_config_loader.credentials_provider(anonymous_credentials);
        } else if let (Some(access_key), Some(secret_key)) =
            (&config.access_key, &config.secret_key)
        {
            // Set explicit credentials if provided
            let credentials = if let Some(token) = &config.session_token {
                Credentials::new(
                    access_key,
                    secret_key,
                    Some(token.clone()),
                    None,
                    "orbit-s3-explicit",
                )
            } else {
                Credentials::new(access_key, secret_key, None, None, "orbit-s3-explicit")
            };
            aws_config_loader = aws_config_loader.credentials_provider(credentials);
        }

        // Phase 4B: credentials_file support
        // Note: The AWS SDK credentials file path can be set via environment variable
        // AWS_SHARED_CREDENTIALS_FILE. For programmatic support, profile_files
        // configuration would be needed, which requires more complex setup.
        // For now, we set the env var if credentials_file is specified.
        if let Some(ref creds_path) = config.credentials_file {
            std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", creds_path);
        }

        // Load AWS config
        let aws_config = aws_config_loader.load().await;

        // Build S3-specific config
        let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&aws_config);

        // Set custom endpoint if provided
        if let Some(endpoint) = &config.endpoint {
            s3_config_builder = s3_config_builder.endpoint_url(endpoint);
        }

        // Force path-style addressing if configured (required for MinIO, LocalStack)
        if config.force_path_style {
            s3_config_builder = s3_config_builder.force_path_style(true);
        }

        // Phase 4C: Enable S3 Transfer Acceleration
        if config.use_acceleration {
            s3_config_builder = s3_config_builder.accelerate(true);
        }

        // Phase 4E: SSL verification bypass warning
        // Note: Disabling SSL certificate verification with rustls requires a custom
        // TlsConnector configuration with dangerous_configuration feature enabled.
        // This is intentionally complex to discourage casual use.
        if config.no_verify_ssl {
            eprintln!(
                "WARNING: --no-verify-ssl is set. SSL certificate verification is NOT disabled \
                 in this build. Rustls certificate bypass requires custom TlsConnector configuration. \
                 Consider using a proper CA bundle or --endpoint with HTTP instead."
            );
        }

        // Set timeout
        let timeout_config = aws_sdk_s3::config::timeout::TimeoutConfig::builder()
            .operation_timeout(Duration::from_secs(config.timeout_seconds))
            .build();
        s3_config_builder = s3_config_builder.timeout_config(timeout_config);

        // Build the client
        let s3_config = s3_config_builder.build();
        Ok(AwsS3Client::from_conf(s3_config))
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &S3Config {
        &self.config
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.config.bucket
    }

    /// Get a reference to the underlying AWS S3 client
    pub fn aws_client(&self) -> &AwsS3Client {
        &self.client
    }

    /// Test the connection by attempting to head the bucket
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit::protocol::s3::{S3Client, S3Config};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = S3Config::new("my-bucket".to_string());
    /// let client = S3Client::new(config).await?;
    ///
    /// if client.test_connection().await.is_ok() {
    ///     println!("Successfully connected to S3!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn test_connection(&self) -> S3Result<()> {
        self.client
            .head_bucket()
            .bucket(&self.config.bucket)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("NoSuchBucket") {
                    S3Error::BucketNotFound(self.config.bucket.clone())
                } else if e.to_string().contains("403") || e.to_string().contains("AccessDenied") {
                    S3Error::AccessDenied(format!("Cannot access bucket: {}", self.config.bucket))
                } else {
                    S3Error::from(e)
                }
            })?;
        Ok(())
    }

    /// Check if an object exists in the bucket
    ///
    /// # Arguments
    ///
    /// * `key` - The object key to check
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit::protocol::s3::{S3Client, S3Config};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = S3Config::new("my-bucket".to_string());
    /// let client = S3Client::new(config).await?;
    ///
    /// if client.exists("path/to/file.txt").await? {
    ///     println!("File exists!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn exists(&self, key: &str) -> S3Result<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(S3Error::from(e))
                }
            }
        }
    }

    /// Get metadata for an object
    ///
    /// # Arguments
    ///
    /// * `key` - The object key
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit::protocol::s3::{S3Client, S3Config};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = S3Config::new("my-bucket".to_string());
    /// let client = S3Client::new(config).await?;
    ///
    /// let metadata = client.get_metadata("file.txt").await?;
    /// println!("Size: {} bytes", metadata.size);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_metadata(&self, key: &str) -> S3Result<S3ObjectMetadata> {
        let response = self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                    S3Error::NotFound {
                        bucket: self.config.bucket.clone(),
                        key: key.to_string(),
                    }
                } else {
                    S3Error::from(e)
                }
            })?;

        let size = response.content_length().unwrap_or(0) as u64;
        let last_modified = response
            .last_modified()
            .and_then(|dt| SystemTime::try_from(*dt).ok());

        let storage_class = response
            .storage_class()
            .map(super::types::S3StorageClass::from_aws);

        let server_side_encryption = response.server_side_encryption().map(|sse| match sse {
            aws_sdk_s3::types::ServerSideEncryption::Aes256 => {
                super::types::S3ServerSideEncryption::Aes256
            }
            aws_sdk_s3::types::ServerSideEncryption::AwsKms => {
                let key_id = response.ssekms_key_id().map(|s| s.to_string());
                super::types::S3ServerSideEncryption::AwsKms { key_id }
            }
            _ => super::types::S3ServerSideEncryption::None,
        });

        let metadata = response
            .metadata()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        Ok(S3ObjectMetadata {
            key: key.to_string(),
            size,
            last_modified,
            etag: response.e_tag().map(|s| s.to_string()),
            storage_class,
            content_type: response.content_type().map(|s| s.to_string()),
            content_encoding: response.content_encoding().map(|s| s.to_string()),
            cache_control: response.cache_control().map(|s| s.to_string()),
            content_disposition: response.content_disposition().map(|s| s.to_string()),
            metadata,
            server_side_encryption,
            version_id: response.version_id().map(|s| s.to_string()),
        })
    }

    /// Delete an object from the bucket
    ///
    /// # Arguments
    ///
    /// * `key` - The object key to delete
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit::protocol::s3::{S3Client, S3Config};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = S3Config::new("my-bucket".to_string());
    /// let client = S3Client::new(config).await?;
    /// client.delete("old-file.txt").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, key: &str) -> S3Result<()> {
        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
            .map_err(S3Error::from)?;
        Ok(())
    }

    /// Generate a pre-signed GET URL for an S3 object
    ///
    /// # Arguments
    ///
    /// * `key` - The object key
    /// * `expires_in` - Duration until the URL expires
    pub async fn presign_get(&self, key: &str, expires_in: Duration) -> S3Result<String> {
        use aws_sdk_s3::presigning::PresigningConfig;

        let presign_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| S3Error::InvalidConfig(format!("Invalid presigning duration: {}", e)))?;

        let presigned = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .presigned(presign_config)
            .await
            .map_err(|e| S3Error::Sdk(format!("Failed to presign URL: {}", e)))?;

        Ok(presigned.uri().to_string())
    }

    /// Detect the region of an S3 bucket using HeadBucket.
    /// Useful for cross-region operations where the user didn't specify a region.
    pub async fn detect_bucket_region(bucket: &str) -> S3Result<Option<String>> {
        // Use a default client with no specific region to call HeadBucket
        let aws_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
        let client = AwsS3Client::new(&aws_config);

        match client.head_bucket().bucket(bucket).send().await {
            Ok(response) => {
                // The region is in the x-amz-bucket-region header
                let region = response.bucket_region().map(|s| s.to_string());
                Ok(region)
            }
            Err(e) => {
                // Check for redirect which also contains region info
                let err_str = e.to_string();
                if err_str.contains("PermanentRedirect") || err_str.contains("301") {
                    // Parse region from error message if possible
                    Ok(None)
                } else {
                    Err(S3Error::from(e))
                }
            }
        }
    }

    /// Create an S3 bucket
    ///
    /// # Arguments
    ///
    /// * `bucket` - The bucket name to create
    pub async fn create_bucket(&self, bucket: &str) -> S3Result<()> {
        self.client
            .create_bucket()
            .bucket(bucket)
            .send()
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("BucketAlreadyOwnedByYou")
                    || err_str.contains("BucketAlreadyExists")
                {
                    S3Error::Sdk(format!("Bucket already exists: {}", bucket))
                } else if err_str.contains("AccessDenied") || err_str.contains("403") {
                    S3Error::AccessDenied(format!("Cannot create bucket: {}", bucket))
                } else {
                    S3Error::from(e)
                }
            })?;
        Ok(())
    }

    /// Delete an S3 bucket
    ///
    /// The bucket must be empty before it can be deleted.
    ///
    /// # Arguments
    ///
    /// * `bucket` - The bucket name to delete
    pub async fn delete_bucket(&self, bucket: &str) -> S3Result<()> {
        self.client
            .delete_bucket()
            .bucket(bucket)
            .send()
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("NoSuchBucket") || err_str.contains("404") {
                    S3Error::BucketNotFound(bucket.to_string())
                } else if err_str.contains("BucketNotEmpty") {
                    S3Error::Sdk(format!(
                        "Bucket is not empty: {}. Delete all objects first.",
                        bucket
                    ))
                } else if err_str.contains("AccessDenied") || err_str.contains("403") {
                    S3Error::AccessDenied(format!("Cannot delete bucket: {}", bucket))
                } else {
                    S3Error::from(e)
                }
            })?;
        Ok(())
    }

    /// Delete a specific version of an object
    ///
    /// # Arguments
    ///
    /// * `key` - The object key
    /// * `version_id` - The version ID to delete
    pub async fn delete_object_version(&self, key: &str, version_id: &str) -> S3Result<()> {
        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(key)
            .version_id(version_id)
            .send()
            .await
            .map_err(S3Error::from)?;
        Ok(())
    }

    /// Delete multiple objects from the bucket in concurrent batch requests
    ///
    /// Uses the S3 DeleteObjects API to delete up to 1000 keys per request.
    /// Chunks are processed concurrently with a semaphore limiting to 10
    /// in-flight requests. More efficient than calling `delete()` in a loop.
    pub async fn delete_batch(&self, keys: &[String]) -> S3Result<()> {
        use aws_sdk_s3::types::{Delete, ObjectIdentifier};

        if keys.is_empty() {
            return Ok(());
        }

        let chunks: Vec<Vec<String>> = keys.chunks(1000).map(|c| c.to_vec()).collect();

        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10));
        let mut handles = Vec::new();

        for chunk in chunks {
            let client = self.client.clone();
            let bucket = self.config.bucket.clone();
            let sem = semaphore.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem
                    .acquire()
                    .await
                    .map_err(|e| S3Error::Sdk(format!("Semaphore error: {}", e)))?;

                let objects: Vec<ObjectIdentifier> = chunk
                    .iter()
                    .map(|key| ObjectIdentifier::builder().key(key).build())
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| {
                        S3Error::Sdk(format!("Failed to build object identifier: {}", e))
                    })?;

                let delete = Delete::builder()
                    .set_objects(Some(objects))
                    .build()
                    .map_err(|e| S3Error::Sdk(format!("Failed to build delete request: {}", e)))?;

                client
                    .delete_objects()
                    .bucket(&bucket)
                    .delete(delete)
                    .send()
                    .await
                    .map_err(S3Error::from)?;

                Ok::<(), S3Error>(())
            }));
        }

        for handle in handles {
            handle
                .await
                .map_err(|e| S3Error::Sdk(format!("Task join error: {}", e)))??;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let config = S3Config::new("test-bucket".to_string());
        let result = S3Client::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_client_with_invalid_bucket() {
        let config = S3Config::new("".to_string());
        let result = S3Client::new(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_client_config_access() {
        let config = S3Config::new("test-bucket".to_string());
        let client = S3Client::new(config).await.unwrap();
        assert_eq!(client.bucket(), "test-bucket");
        assert_eq!(client.config().bucket, "test-bucket");
    }
}
