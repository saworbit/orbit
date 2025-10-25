//! S3 operations trait and implementations

use super::client::S3Client;
use super::error::{S3Error, S3Result};
use super::types::{S3ListResult, S3Object, S3StorageClass};
use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use bytes::Bytes;
use std::path::Path;
use std::time::SystemTime;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Trait defining S3 operations
#[async_trait]
pub trait S3Operations {
    /// List objects in the bucket with a given prefix
    async fn list_objects(&self, prefix: &str) -> S3Result<S3ListResult>;
    
    /// List objects with pagination support
    async fn list_objects_paginated(
        &self,
        prefix: &str,
        continuation_token: Option<String>,
        max_keys: Option<i32>,
    ) -> S3Result<S3ListResult>;
    
    /// Upload a file to S3
    async fn upload_file(&self, local_path: &Path, key: &str) -> S3Result<()>;
    
    /// Upload bytes to S3
    async fn upload_bytes(&self, data: Bytes, key: &str) -> S3Result<()>;
    
    /// Download a file from S3
    async fn download_file(&self, key: &str, local_path: &Path) -> S3Result<()>;
    
    /// Download bytes from S3
    async fn download_bytes(&self, key: &str) -> S3Result<Bytes>;
    
    /// Copy an object within S3 (same bucket or different)
    async fn copy_object(&self, source_key: &str, dest_key: &str) -> S3Result<()>;
}

#[async_trait]
impl S3Operations for S3Client {
    async fn list_objects(&self, prefix: &str) -> S3Result<S3ListResult> {
        self.list_objects_paginated(prefix, None, None).await
    }
    
    async fn list_objects_paginated(
        &self,
        prefix: &str,
        continuation_token: Option<String>,
        max_keys: Option<i32>,
    ) -> S3Result<S3ListResult> {
        let mut request = self.aws_client()
            .list_objects_v2()
            .bucket(self.bucket())
            .prefix(prefix);
        
        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }
        
        if let Some(max) = max_keys {
            request = request.max_keys(max);
        }
        
        let response = request.send().await.map_err(S3Error::from)?;
        
        let objects = response
            .contents()
            .iter()
            .filter_map(|obj| {
                let key = obj.key()?.to_string();
                let size = obj.size().unwrap_or(0) as u64;
                let last_modified = obj.last_modified()
                    .and_then(|dt| SystemTime::try_from(*dt).ok());
                let etag = obj.e_tag().map(|s| s.to_string());
                let storage_class = obj.storage_class().map(|sc| {
                    match sc.as_str() {
                        "STANDARD" => S3StorageClass::Standard,
                        "REDUCED_REDUNDANCY" => S3StorageClass::ReducedRedundancy,
                        "STANDARD_IA" => S3StorageClass::StandardIa,
                        "ONEZONE_IA" => S3StorageClass::OnezoneIa,
                        "INTELLIGENT_TIERING" => S3StorageClass::IntelligentTiering,
                        "GLACIER_IR" => S3StorageClass::GlacierInstantRetrieval,
                        "GLACIER" => S3StorageClass::GlacierFlexibleRetrieval,
                        "DEEP_ARCHIVE" => S3StorageClass::GlacierDeepArchive,
                        _ => S3StorageClass::Standard,
                    }
                });
                
                Some(S3Object {
                    key,
                    size,
                    last_modified,
                    etag,
                    storage_class,
                    content_type: None,
                })
            })
            .collect();
        
        let common_prefixes = response
            .common_prefixes()
            .iter()
            .filter_map(|cp| cp.prefix().map(|s| s.to_string()))
            .collect();
        
        let is_truncated = response.is_truncated().unwrap_or(false);
        let continuation_token = response.next_continuation_token()
            .map(|s| s.to_string());
        
        Ok(S3ListResult {
            objects,
            common_prefixes,
            continuation_token,
            is_truncated,
        })
    }
    
    async fn upload_file(&self, local_path: &Path, key: &str) -> S3Result<()> {
        // Read the file
        let mut file = File::open(local_path).await?;
        let metadata = file.metadata().await?;
        let file_size = metadata.len();
        
        // For small files, use simple upload
        if file_size < self.config().chunk_size as u64 {
            let mut buffer = Vec::with_capacity(file_size as usize);
            file.read_to_end(&mut buffer).await?;
            return self.upload_bytes(Bytes::from(buffer), key).await;
        }
        
        // For large files, use multipart upload (handled in multipart.rs)
        // This will be implemented in the next step
        Err(S3Error::MultipartUpload(
            "Large file upload requires multipart implementation".to_string()
        ))
    }
    
    async fn upload_bytes(&self, data: Bytes, key: &str) -> S3Result<()> {
        let byte_stream = ByteStream::from(data);
        
        let mut request = self.aws_client()
            .put_object()
            .bucket(self.bucket())
            .key(key)
            .body(byte_stream);
        
        // Set storage class
        request = request.storage_class(self.config().storage_class.to_aws());
        
        // Set server-side encryption
        if let Some(sse) = self.config().server_side_encryption.to_aws() {
            request = request.server_side_encryption(sse);
            
            // If using KMS, set the KMS key ID
            if let super::types::S3ServerSideEncryption::AwsKms { key_id: Some(kid) } = 
                &self.config().server_side_encryption 
            {
                request = request.ssekms_key_id(kid);
            }
        }
        
        request.send().await.map_err(S3Error::from)?;
        Ok(())
    }
    
    async fn download_file(&self, key: &str, local_path: &Path) -> S3Result<()> {
        let response = self.aws_client()
            .get_object()
            .bucket(self.bucket())
            .key(key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("NoSuchKey") {
                    S3Error::NotFound {
                        bucket: self.bucket().to_string(),
                        key: key.to_string(),
                    }
                } else {
                    S3Error::from(e)
                }
            })?;
        
        // Create parent directories if they don't exist
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Stream the data to file
        let mut file = File::create(local_path).await?;
        let mut body = response.body;
        
        while let Some(bytes) = body.try_next().await.map_err(|e| {
            S3Error::Network(format!("Failed to read response body: {}", e))
        })? {
            file.write_all(&bytes).await?;
        }
        
        file.flush().await?;
        Ok(())
    }
    
    async fn download_bytes(&self, key: &str) -> S3Result<Bytes> {
        let response = self.aws_client()
            .get_object()
            .bucket(self.bucket())
            .key(key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("NoSuchKey") {
                    S3Error::NotFound {
                        bucket: self.bucket().to_string(),
                        key: key.to_string(),
                    }
                } else {
                    S3Error::from(e)
                }
            })?;
        
        let body = response.body.collect().await.map_err(|e| {
            S3Error::Network(format!("Failed to collect response body: {}", e))
        })?;
        
        Ok(body.into_bytes())
    }
    
    async fn copy_object(&self, source_key: &str, dest_key: &str) -> S3Result<()> {
        let copy_source = format!("{}/{}", self.bucket(), source_key);
        
        self.aws_client()
            .copy_object()
            .bucket(self.bucket())
            .copy_source(&copy_source)
            .key(dest_key)
            .send()
            .await
            .map_err(S3Error::from)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::config::S3Config;

    #[tokio::test]
    async fn test_operations_trait_exists() {
        // This test just verifies the trait compiles
        let config = S3Config::new("test-bucket".to_string());
        let client = S3Client::new(config).await.unwrap();
        
        // Verify the client implements S3Operations
        fn assert_impl<T: S3Operations>(_: &T) {}
        assert_impl(&client);
    }
}