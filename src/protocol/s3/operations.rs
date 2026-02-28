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

/// Extract the static prefix from a pattern up to the first wildcard character.
/// For example, "data/2024-*.parquet" returns "data/2024-".
/// This allows us to narrow the S3 ListObjects call to a smaller prefix,
/// then filter results in-memory against the full pattern.
fn extract_prefix_before_wildcard(pattern: &str) -> &str {
    let first_wildcard = pattern
        .find(|c: char| c == '*' || c == '?')
        .unwrap_or(pattern.len());
    &pattern[..first_wildcard]
}

/// Check if a pattern contains wildcard characters
pub fn has_wildcards(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?')
}

/// Match a key against a simple glob pattern (supports * and ? wildcards)
fn glob_match(pattern: &str, text: &str) -> bool {
    let p = pattern.chars().collect::<Vec<_>>();
    let t = text.chars().collect::<Vec<_>>();

    glob_match_inner(&p, &t, 0, 0)
}

fn glob_match_inner(pattern: &[char], text: &[char], pi: usize, ti: usize) -> bool {
    let mut pi = pi;
    let mut ti = ti;

    while pi < pattern.len() {
        match pattern[pi] {
            '*' => {
                // Skip consecutive stars
                while pi < pattern.len() && pattern[pi] == '*' {
                    pi += 1;
                }
                // Star at end matches everything
                if pi == pattern.len() {
                    return true;
                }
                // Try matching rest of pattern at each position
                while ti <= text.len() {
                    if glob_match_inner(pattern, text, pi, ti) {
                        return true;
                    }
                    ti += 1;
                }
                return false;
            }
            '?' => {
                if ti >= text.len() {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
            c => {
                if ti >= text.len() || text[ti] != c {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
        }
    }

    pi == pattern.len() && ti == text.len()
}

#[async_trait]
impl S3Operations for S3Client {
    async fn list_objects(&self, prefix: &str) -> S3Result<S3ListResult> {
        // Optimization: if the prefix contains wildcards, extract the static
        // prefix portion and use it for the API call, then filter in-memory.
        if has_wildcards(prefix) {
            return self.list_objects_with_wildcard(prefix).await;
        }
        self.list_objects_paginated(prefix, None, None).await
    }

    async fn list_objects_paginated(
        &self,
        prefix: &str,
        continuation_token: Option<String>,
        max_keys: Option<i32>,
    ) -> S3Result<S3ListResult> {
        let mut request = self
            .aws_client()
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
                let last_modified = obj
                    .last_modified()
                    .and_then(|dt| SystemTime::try_from(*dt).ok());
                let etag = obj.e_tag().map(|s| s.to_string());
                let storage_class = obj.storage_class().map(|sc| match sc.as_str() {
                    "STANDARD" => S3StorageClass::Standard,
                    "REDUCED_REDUNDANCY" => S3StorageClass::ReducedRedundancy,
                    "STANDARD_IA" => S3StorageClass::StandardIa,
                    "ONEZONE_IA" => S3StorageClass::OnezoneIa,
                    "INTELLIGENT_TIERING" => S3StorageClass::IntelligentTiering,
                    "GLACIER_IR" => S3StorageClass::GlacierInstantRetrieval,
                    "GLACIER" => S3StorageClass::GlacierFlexibleRetrieval,
                    "DEEP_ARCHIVE" => S3StorageClass::GlacierDeepArchive,
                    _ => S3StorageClass::Standard,
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
        let continuation_token = response.next_continuation_token().map(|s| s.to_string());

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

        // For small files, use simple upload with content headers
        if file_size < self.config().chunk_size as u64 {
            let mut buffer = Vec::with_capacity(file_size as usize);
            file.read_to_end(&mut buffer).await?;

            let byte_stream = ByteStream::from(Bytes::from(buffer));

            let mut request = self
                .aws_client()
                .put_object()
                .bucket(self.bucket())
                .key(key)
                .body(byte_stream);

            // Set storage class
            request = request.storage_class(self.config().storage_class.to_aws());

            // Set server-side encryption
            if let Some(sse) = self.config().server_side_encryption.to_aws() {
                request = request.server_side_encryption(sse);
                if let super::types::S3ServerSideEncryption::AwsKms { key_id: Some(kid) } =
                    &self.config().server_side_encryption
                {
                    request = request.ssekms_key_id(kid);
                }
            }

            // Auto-detect or use explicit Content-Type
            let content_type = if let Some(ref ct) = self.config().content_type {
                ct.clone()
            } else {
                // Auto-detect from file extension using mime_guess
                mime_guess::from_path(local_path)
                    .first()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string())
            };
            request = request.content_type(content_type);

            // Apply content headers from config
            if let Some(ref val) = self.config().content_encoding {
                request = request.content_encoding(val);
            }
            if let Some(ref val) = self.config().content_disposition {
                request = request.content_disposition(val);
            }
            if let Some(ref val) = self.config().cache_control {
                request = request.cache_control(val);
            }

            // Apply user metadata
            if !self.config().user_metadata.is_empty() {
                for (k, v) in &self.config().user_metadata {
                    request = request.metadata(k, v);
                }
            }

            // Apply ACL
            if let Some(ref acl_str) = self.config().acl {
                use aws_sdk_s3::types::ObjectCannedAcl;
                let acl = match acl_str.as_str() {
                    "private" => ObjectCannedAcl::Private,
                    "public-read" => ObjectCannedAcl::PublicRead,
                    "public-read-write" => ObjectCannedAcl::PublicReadWrite,
                    "authenticated-read" => ObjectCannedAcl::AuthenticatedRead,
                    "aws-exec-read" => ObjectCannedAcl::AwsExecRead,
                    "bucket-owner-read" => ObjectCannedAcl::BucketOwnerRead,
                    "bucket-owner-full-control" => ObjectCannedAcl::BucketOwnerFullControl,
                    other => ObjectCannedAcl::from(other),
                };
                request = request.acl(acl);
            }

            // TODO: Apply request_payer if config.request_payer is true
            // request = request.request_payer(aws_sdk_s3::types::RequestPayer::Requester);

            request.send().await.map_err(S3Error::from)?;
            return Ok(());
        }

        // For large files, use multipart upload (handled in multipart.rs)
        // This will be implemented in the next step
        Err(S3Error::MultipartUpload(
            "Large file upload requires multipart implementation".to_string(),
        ))
    }

    async fn upload_bytes(&self, data: Bytes, key: &str) -> S3Result<()> {
        let byte_stream = ByteStream::from(data);

        let mut request = self
            .aws_client()
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

        // Apply content headers from config
        if let Some(ref val) = self.config().content_type {
            request = request.content_type(val);
        }
        if let Some(ref val) = self.config().content_encoding {
            request = request.content_encoding(val);
        }
        if let Some(ref val) = self.config().content_disposition {
            request = request.content_disposition(val);
        }
        if let Some(ref val) = self.config().cache_control {
            request = request.cache_control(val);
        }

        // Apply user metadata
        if !self.config().user_metadata.is_empty() {
            for (k, v) in &self.config().user_metadata {
                request = request.metadata(k, v);
            }
        }

        // Apply ACL
        if let Some(ref acl_str) = self.config().acl {
            use aws_sdk_s3::types::ObjectCannedAcl;
            let acl = match acl_str.as_str() {
                "private" => ObjectCannedAcl::Private,
                "public-read" => ObjectCannedAcl::PublicRead,
                "public-read-write" => ObjectCannedAcl::PublicReadWrite,
                "authenticated-read" => ObjectCannedAcl::AuthenticatedRead,
                "aws-exec-read" => ObjectCannedAcl::AwsExecRead,
                "bucket-owner-read" => ObjectCannedAcl::BucketOwnerRead,
                "bucket-owner-full-control" => ObjectCannedAcl::BucketOwnerFullControl,
                other => ObjectCannedAcl::from(other),
            };
            request = request.acl(acl);
        }

        // TODO: Apply request_payer if config.request_payer is true
        // request = request.request_payer(aws_sdk_s3::types::RequestPayer::Requester);

        request.send().await.map_err(S3Error::from)?;
        Ok(())
    }

    async fn download_file(&self, key: &str, local_path: &Path) -> S3Result<()> {
        let response = self
            .aws_client()
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

        while let Some(bytes) = body
            .try_next()
            .await
            .map_err(|e| S3Error::Network(format!("Failed to read response body: {}", e)))?
        {
            file.write_all(&bytes).await?;
        }

        file.flush().await?;
        Ok(())
    }

    async fn download_bytes(&self, key: &str) -> S3Result<Bytes> {
        let response = self
            .aws_client()
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

        let body = response
            .body
            .collect()
            .await
            .map_err(|e| S3Error::Network(format!("Failed to collect response body: {}", e)))?;

        Ok(body.into_bytes())
    }

    async fn copy_object(&self, source_key: &str, dest_key: &str) -> S3Result<()> {
        let copy_source = format!("{}/{}", self.bucket(), source_key);

        let mut request = self
            .aws_client()
            .copy_object()
            .bucket(self.bucket())
            .copy_source(&copy_source)
            .key(dest_key);

        // Apply metadata directive if set (COPY or REPLACE)
        if let Some(ref directive) = self.config().metadata_directive {
            use aws_sdk_s3::types::MetadataDirective;
            let md = match directive.to_uppercase().as_str() {
                "COPY" => MetadataDirective::Copy,
                "REPLACE" => MetadataDirective::Replace,
                _ => MetadataDirective::Copy,
            };
            request = request.metadata_directive(md);
        }

        // TODO: Apply request_payer if config.request_payer is true
        // request = request.request_payer(aws_sdk_s3::types::RequestPayer::Requester);

        request.send().await.map_err(S3Error::from)?;

        Ok(())
    }
}

impl S3Client {
    /// List objects using a wildcard pattern with prefix optimization.
    /// Extracts the static prefix before the first wildcard, uses it to
    /// narrow the ListObjectsV2 API call, then filters in-memory.
    pub async fn list_objects_with_wildcard(&self, pattern: &str) -> S3Result<S3ListResult> {
        let static_prefix = extract_prefix_before_wildcard(pattern);

        // List with the narrowed prefix
        let mut all_objects = Vec::new();
        let mut continuation_token = None;

        loop {
            let result = self
                .list_objects_paginated(static_prefix, continuation_token, None)
                .await?;

            // Filter objects against the full wildcard pattern
            for obj in result.objects {
                if glob_match(pattern, &obj.key) {
                    all_objects.push(obj);
                }
            }

            if result.is_truncated {
                continuation_token = result.continuation_token;
            } else {
                break;
            }
        }

        Ok(S3ListResult {
            objects: all_objects,
            common_prefixes: Vec::new(),
            continuation_token: None,
            is_truncated: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::config::S3Config;
    use super::*;

    #[tokio::test]
    async fn test_operations_trait_exists() {
        // This test just verifies the trait compiles
        let config = S3Config::new("test-bucket".to_string());
        let client = S3Client::new(config).await.unwrap();

        // Verify the client implements S3Operations
        fn assert_impl<T: S3Operations>(_: &T) {}
        assert_impl(&client);
    }

    #[test]
    fn test_extract_prefix_before_wildcard() {
        assert_eq!(extract_prefix_before_wildcard("data/2024-*.parquet"), "data/2024-");
        assert_eq!(extract_prefix_before_wildcard("logs/*/error.log"), "logs/");
        assert_eq!(extract_prefix_before_wildcard("*.txt"), "");
        assert_eq!(extract_prefix_before_wildcard("exact-key"), "exact-key");
        assert_eq!(extract_prefix_before_wildcard("prefix/sub?file"), "prefix/sub");
    }

    #[test]
    fn test_has_wildcards() {
        assert!(has_wildcards("*.txt"));
        assert!(has_wildcards("data/?"));
        assert!(!has_wildcards("data/exact/path"));
        assert!(!has_wildcards(""));
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.txt", "file.txt"));
        assert!(glob_match("*.txt", "path/to/file.txt"));
        assert!(!glob_match("*.txt", "file.csv"));
        assert!(glob_match("data/2024-*", "data/2024-01-01.csv"));
        assert!(!glob_match("data/2024-*", "data/2023-01-01.csv"));
        assert!(glob_match("data/*/file.txt", "data/sub/file.txt"));
        assert!(glob_match("?oo", "foo"));
        assert!(!glob_match("?oo", "fooo"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("prefix/*/*.log", "prefix/app/error.log"));
    }
}
