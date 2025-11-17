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

        // Set explicit credentials if provided
        if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
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
