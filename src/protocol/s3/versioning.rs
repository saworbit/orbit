//! S3 Object Versioning Support
//!
//! This module provides comprehensive support for S3 object versioning, allowing
//! you to work with versioned objects, list versions, restore previous versions,
//! and manage lifecycle policies.
//!
//! # Overview
//!
//! When versioning is enabled on an S3 bucket, S3 keeps multiple variants of an
//! object in the same bucket. This allows you to preserve, retrieve, and restore
//! every version of every object stored in your bucket.
//!
//! # Features
//!
//! - List all versions of an object
//! - Download specific versions
//! - Upload with version awareness
//! - Delete specific versions or delete markers
//! - Restore previous versions
//! - Compare versions
//! - Batch version operations
//!
//! # Examples
//!
//! ## List Object Versions
//!
//! ```ignore
//! use orbit::protocol::s3::{S3Client, S3Config};
//! use orbit::protocol::s3::versioning::VersioningOperations;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = S3Client::new(S3Config {
//!         bucket: "my-versioned-bucket".to_string(),
//!         ..Default::default()
//!     }).await?;
//!
//!     let result = client.list_object_versions("my-file.txt").await?;
//!     for version in result.versions {
//!         println!("Version: {} ({:?})", version.version_id, version.last_modified);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Download Specific Version
//!
//! ```ignore
//! use orbit::protocol::s3::{S3Client, S3Config};
//! use orbit::protocol::s3::versioning::VersioningOperations;
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = S3Client::new(S3Config::default()).await?;
//!     // Download version from yesterday
//!     client.download_version(
//!         "my-file.txt",
//!         "version-id-here",
//!         Path::new("restored-file.txt")
//!     ).await?;
//!     Ok(())
//! }
//! ```

use super::client::S3Client;
use super::error::{S3Error, S3Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

/// Object version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectVersion {
    /// Version ID
    pub version_id: String,

    /// Object key
    pub key: String,

    /// Size in bytes
    pub size: u64,

    /// Last modified timestamp
    pub last_modified: SystemTime,

    /// ETag
    pub etag: String,

    /// Whether this is the latest version
    pub is_latest: bool,

    /// Storage class
    pub storage_class: Option<String>,

    /// Owner ID (if available)
    pub owner_id: Option<String>,
}

impl ObjectVersion {
    /// Create a new object version
    pub fn new(
        version_id: String,
        key: String,
        size: u64,
        last_modified: SystemTime,
        etag: String,
    ) -> Self {
        Self {
            version_id,
            key,
            size,
            last_modified,
            etag,
            is_latest: false,
            storage_class: None,
            owner_id: None,
        }
    }

    /// Check if this version is newer than another
    pub fn is_newer_than(&self, other: &ObjectVersion) -> bool {
        self.last_modified > other.last_modified
    }

    /// Get age of this version as duration
    pub fn age(&self) -> std::time::Duration {
        SystemTime::now()
            .duration_since(self.last_modified)
            .unwrap_or_default()
    }
}

/// Delete marker information (represents a deleted version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMarker {
    /// Version ID of the delete marker
    pub version_id: String,

    /// Object key
    pub key: String,

    /// When it was deleted
    pub last_modified: SystemTime,

    /// Whether this is the latest
    pub is_latest: bool,

    /// Owner ID (if available)
    pub owner_id: Option<String>,
}

impl DeleteMarker {
    /// Create a new delete marker
    pub fn new(version_id: String, key: String, last_modified: SystemTime) -> Self {
        Self {
            version_id,
            key,
            last_modified,
            is_latest: false,
            owner_id: None,
        }
    }
}

/// Result of listing object versions
#[derive(Debug, Clone, Default)]
pub struct VersionsListResult {
    /// Object versions
    pub versions: Vec<ObjectVersion>,

    /// Delete markers
    pub delete_markers: Vec<DeleteMarker>,

    /// Continuation token for pagination
    pub next_key_marker: Option<String>,

    /// Next version ID marker
    pub next_version_id_marker: Option<String>,

    /// Whether results are truncated
    pub is_truncated: bool,
}

impl VersionsListResult {
    /// Get all versions sorted by last modified (newest first)
    pub fn versions_sorted(&self) -> Vec<ObjectVersion> {
        let mut versions = self.versions.clone();
        versions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
        versions
    }

    /// Get the latest non-deleted version
    pub fn latest_version(&self) -> Option<&ObjectVersion> {
        self.versions.iter().find(|v| v.is_latest)
    }

    /// Get total size across all versions
    pub fn total_size(&self) -> u64 {
        self.versions.iter().map(|v| v.size).sum()
    }

    /// Count versions
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }

    /// Check if object is currently deleted
    pub fn is_deleted(&self) -> bool {
        self.delete_markers.iter().any(|dm| dm.is_latest)
    }
}

/// Options for version restoration
#[derive(Debug, Clone)]
pub struct RestoreOptions {
    /// Target key (if different from source)
    pub target_key: Option<String>,

    /// Whether to make this the current version
    pub make_current: bool,

    /// Storage class for restored object
    pub storage_class: Option<String>,

    /// Metadata to apply
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

impl Default for RestoreOptions {
    fn default() -> Self {
        Self {
            target_key: None,
            make_current: true,
            storage_class: None,
            metadata: None,
        }
    }
}

/// Trait for S3 versioning operations
#[async_trait]
pub trait VersioningOperations {
    /// List all versions of an object
    async fn list_object_versions(&self, key: &str) -> S3Result<VersionsListResult>;

    /// List versions with pagination
    async fn list_object_versions_paginated(
        &self,
        key: &str,
        key_marker: Option<String>,
        version_id_marker: Option<String>,
        max_keys: Option<i32>,
    ) -> S3Result<VersionsListResult>;

    /// Download a specific version of an object
    async fn download_version(
        &self,
        key: &str,
        version_id: &str,
        local_path: &Path,
    ) -> S3Result<()>;

    /// Get metadata for a specific version
    async fn get_version_metadata(&self, key: &str, version_id: &str) -> S3Result<ObjectVersion>;

    /// Delete a specific version permanently
    async fn delete_version(&self, key: &str, version_id: &str) -> S3Result<()>;

    /// Restore a previous version (copies it to make it current)
    async fn restore_version(
        &self,
        key: &str,
        version_id: &str,
        options: Option<RestoreOptions>,
    ) -> S3Result<String>; // Returns new version ID

    /// Compare two versions (returns size difference and modified fields)
    async fn compare_versions(
        &self,
        key: &str,
        version_id1: &str,
        version_id2: &str,
    ) -> S3Result<VersionComparison>;

    /// Enable versioning on the bucket
    async fn enable_versioning(&self) -> S3Result<()>;

    /// Suspend versioning on the bucket
    async fn suspend_versioning(&self) -> S3Result<()>;

    /// Get bucket versioning status
    async fn get_versioning_status(&self) -> S3Result<VersioningStatus>;
}

/// Versioning status of a bucket
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersioningStatus {
    /// Versioning is not enabled
    Disabled,

    /// Versioning is enabled
    Enabled,

    /// Versioning is suspended
    Suspended,
}

impl std::fmt::Display for VersioningStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersioningStatus::Disabled => write!(f, "Disabled"),
            VersioningStatus::Enabled => write!(f, "Enabled"),
            VersioningStatus::Suspended => write!(f, "Suspended"),
        }
    }
}

/// Result of comparing two versions
#[derive(Debug, Clone)]
pub struct VersionComparison {
    /// Size difference (positive if version2 is larger)
    pub size_diff: i64,

    /// Time difference
    pub time_diff: std::time::Duration,

    /// Whether ETags match
    pub etags_match: bool,

    /// Version 1 info
    pub version1: ObjectVersion,

    /// Version 2 info
    pub version2: ObjectVersion,
}

impl VersionComparison {
    /// Check if versions are identical
    pub fn are_identical(&self) -> bool {
        self.etags_match && self.size_diff == 0
    }

    /// Get which version is newer
    pub fn newer_version(&self) -> &ObjectVersion {
        if self.version2.last_modified > self.version1.last_modified {
            &self.version2
        } else {
            &self.version1
        }
    }
}

/// Implementation of versioning operations for S3Client
#[async_trait]
impl VersioningOperations for S3Client {
    async fn list_object_versions(&self, key: &str) -> S3Result<VersionsListResult> {
        self.list_object_versions_paginated(key, None, None, None)
            .await
    }

    async fn list_object_versions_paginated(
        &self,
        key: &str,
        key_marker: Option<String>,
        version_id_marker: Option<String>,
        max_keys: Option<i32>,
    ) -> S3Result<VersionsListResult> {
        let mut request = self
            .aws_client()
            .list_object_versions()
            .bucket(self.bucket())
            .prefix(key);

        if let Some(marker) = key_marker {
            request = request.key_marker(marker);
        }

        if let Some(version_marker) = version_id_marker {
            request = request.version_id_marker(version_marker);
        }

        if let Some(max) = max_keys {
            request = request.max_keys(max);
        }

        let response = request.send().await.map_err(S3Error::from)?;

        // Parse versions
        let versions = response
            .versions()
            .iter()
            .filter_map(|v| {
                let version_id = v.version_id()?.to_string();
                let key = v.key()?.to_string();
                let size = v.size().unwrap_or(0) as u64;
                let last_modified = v
                    .last_modified()
                    .and_then(|dt| SystemTime::try_from(*dt).ok())?;
                let etag = v.e_tag()?.to_string();

                let mut obj_version =
                    ObjectVersion::new(version_id, key, size, last_modified, etag);

                obj_version.is_latest = v.is_latest().unwrap_or(false);
                obj_version.storage_class = v.storage_class().map(|sc| sc.as_str().to_string());
                obj_version.owner_id = v.owner().and_then(|o| o.id().map(|s| s.to_string()));

                Some(obj_version)
            })
            .collect();

        // Parse delete markers
        let delete_markers = response
            .delete_markers()
            .iter()
            .filter_map(|dm| {
                let version_id = dm.version_id()?.to_string();
                let key = dm.key()?.to_string();
                let last_modified = dm
                    .last_modified()
                    .and_then(|dt| SystemTime::try_from(*dt).ok())?;

                let mut marker = DeleteMarker::new(version_id, key, last_modified);
                marker.is_latest = dm.is_latest().unwrap_or(false);
                marker.owner_id = dm.owner().and_then(|o| o.id().map(|s| s.to_string()));

                Some(marker)
            })
            .collect();

        Ok(VersionsListResult {
            versions,
            delete_markers,
            next_key_marker: response.next_key_marker().map(|s| s.to_string()),
            next_version_id_marker: response.next_version_id_marker().map(|s| s.to_string()),
            is_truncated: response.is_truncated().unwrap_or(false),
        })
    }

    async fn download_version(
        &self,
        key: &str,
        version_id: &str,
        local_path: &Path,
    ) -> S3Result<()> {
        use tokio::io::AsyncWriteExt;

        let response = self
            .aws_client()
            .get_object()
            .bucket(self.bucket())
            .key(key)
            .version_id(version_id)
            .send()
            .await
            .map_err(S3Error::from)?;

        let mut file = tokio::fs::File::create(local_path)
            .await
            .map_err(|e| S3Error::Io(e.to_string()))?;

        let mut stream = response.body.into_async_read();
        tokio::io::copy(&mut stream, &mut file)
            .await
            .map_err(|e| S3Error::Io(e.to_string()))?;

        file.flush().await.map_err(|e| S3Error::Io(e.to_string()))?;

        Ok(())
    }

    async fn get_version_metadata(&self, key: &str, version_id: &str) -> S3Result<ObjectVersion> {
        let response = self
            .aws_client()
            .head_object()
            .bucket(self.bucket())
            .key(key)
            .version_id(version_id)
            .send()
            .await
            .map_err(S3Error::from)?;

        let size = response.content_length().unwrap_or(0) as u64;
        let last_modified = response
            .last_modified()
            .and_then(|dt| SystemTime::try_from(*dt).ok())
            .unwrap_or_else(SystemTime::now);
        let etag = response.e_tag().unwrap_or("unknown").to_string();
        let version_id_str = response.version_id().unwrap_or(version_id).to_string();

        let mut version =
            ObjectVersion::new(version_id_str, key.to_string(), size, last_modified, etag);

        version.storage_class = response.storage_class().map(|sc| sc.as_str().to_string());

        Ok(version)
    }

    async fn delete_version(&self, key: &str, version_id: &str) -> S3Result<()> {
        self.aws_client()
            .delete_object()
            .bucket(self.bucket())
            .key(key)
            .version_id(version_id)
            .send()
            .await
            .map_err(S3Error::from)?;

        Ok(())
    }

    async fn restore_version(
        &self,
        key: &str,
        version_id: &str,
        options: Option<RestoreOptions>,
    ) -> S3Result<String> {
        let opts = options.unwrap_or_default();
        let target_key = opts.target_key.as_deref().unwrap_or(key);

        // Copy the old version to become the new current version
        // Include version ID in the copy source string
        let source = format!("{}/{}?versionId={}", self.bucket(), key, version_id);
        let mut request = self
            .aws_client()
            .copy_object()
            .bucket(self.bucket())
            .copy_source(source)
            .key(target_key);

        if let Some(storage_class) = opts.storage_class {
            request = request.storage_class(aws_sdk_s3::types::StorageClass::from(
                storage_class.as_str(),
            ));
        }

        if let Some(metadata) = opts.metadata {
            request = request.metadata_directive(aws_sdk_s3::types::MetadataDirective::Replace);
            for (k, v) in metadata {
                request = request.metadata(k, v);
            }
        }

        let response = request.send().await.map_err(S3Error::from)?;

        let new_version_id = response.version_id().unwrap_or("unknown").to_string();

        Ok(new_version_id)
    }

    async fn compare_versions(
        &self,
        key: &str,
        version_id1: &str,
        version_id2: &str,
    ) -> S3Result<VersionComparison> {
        let version1 = self.get_version_metadata(key, version_id1).await?;
        let version2 = self.get_version_metadata(key, version_id2).await?;

        let size_diff = version2.size as i64 - version1.size as i64;
        let time_diff = if version2.last_modified > version1.last_modified {
            version2
                .last_modified
                .duration_since(version1.last_modified)
                .unwrap_or_default()
        } else {
            version1
                .last_modified
                .duration_since(version2.last_modified)
                .unwrap_or_default()
        };
        let etags_match = version1.etag == version2.etag;

        Ok(VersionComparison {
            size_diff,
            time_diff,
            etags_match,
            version1,
            version2,
        })
    }

    async fn enable_versioning(&self) -> S3Result<()> {
        self.aws_client()
            .put_bucket_versioning()
            .bucket(self.bucket())
            .versioning_configuration(
                aws_sdk_s3::types::VersioningConfiguration::builder()
                    .status(aws_sdk_s3::types::BucketVersioningStatus::Enabled)
                    .build(),
            )
            .send()
            .await
            .map_err(S3Error::from)?;

        Ok(())
    }

    async fn suspend_versioning(&self) -> S3Result<()> {
        self.aws_client()
            .put_bucket_versioning()
            .bucket(self.bucket())
            .versioning_configuration(
                aws_sdk_s3::types::VersioningConfiguration::builder()
                    .status(aws_sdk_s3::types::BucketVersioningStatus::Suspended)
                    .build(),
            )
            .send()
            .await
            .map_err(S3Error::from)?;

        Ok(())
    }

    async fn get_versioning_status(&self) -> S3Result<VersioningStatus> {
        let response = self
            .aws_client()
            .get_bucket_versioning()
            .bucket(self.bucket())
            .send()
            .await
            .map_err(S3Error::from)?;

        match response.status() {
            Some(aws_sdk_s3::types::BucketVersioningStatus::Enabled) => {
                Ok(VersioningStatus::Enabled)
            }
            Some(aws_sdk_s3::types::BucketVersioningStatus::Suspended) => {
                Ok(VersioningStatus::Suspended)
            }
            _ => Ok(VersioningStatus::Disabled),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_version_age() {
        let past = SystemTime::now() - std::time::Duration::from_secs(3600);
        let version = ObjectVersion::new(
            "v1".to_string(),
            "key".to_string(),
            100,
            past,
            "etag".to_string(),
        );

        let age = version.age();
        assert!(age.as_secs() >= 3600 && age.as_secs() < 3700);
    }

    #[test]
    fn test_versions_list_sorting() {
        let now = SystemTime::now();
        let older = now - std::time::Duration::from_secs(3600);

        let mut result = VersionsListResult::default();
        result.versions.push(ObjectVersion::new(
            "v1".to_string(),
            "key".to_string(),
            100,
            older,
            "etag1".to_string(),
        ));
        result.versions.push(ObjectVersion::new(
            "v2".to_string(),
            "key".to_string(),
            200,
            now,
            "etag2".to_string(),
        ));

        let sorted = result.versions_sorted();
        assert_eq!(sorted[0].version_id, "v2"); // Newest first
        assert_eq!(sorted[1].version_id, "v1");
    }

    #[test]
    fn test_total_size() {
        let mut result = VersionsListResult::default();
        result.versions.push(ObjectVersion::new(
            "v1".to_string(),
            "key".to_string(),
            100,
            SystemTime::now(),
            "etag1".to_string(),
        ));
        result.versions.push(ObjectVersion::new(
            "v2".to_string(),
            "key".to_string(),
            200,
            SystemTime::now(),
            "etag2".to_string(),
        ));

        assert_eq!(result.total_size(), 300);
    }

    #[test]
    fn test_version_comparison_identical() {
        let now = SystemTime::now();
        let v1 = ObjectVersion::new(
            "v1".to_string(),
            "key".to_string(),
            100,
            now,
            "etag".to_string(),
        );
        let v2 = ObjectVersion::new(
            "v2".to_string(),
            "key".to_string(),
            100,
            now,
            "etag".to_string(),
        );

        let comparison = VersionComparison {
            size_diff: 0,
            time_diff: std::time::Duration::from_secs(0),
            etags_match: true,
            version1: v1,
            version2: v2,
        };

        assert!(comparison.are_identical());
    }
}
