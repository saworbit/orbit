//! Native S3 protocol implementation for Orbit
//!
//! This module provides AWS S3 support using the official AWS SDK for Rust.
//! It supports standard S3 operations as well as S3-compatible services like MinIO.
//!
//! # Features
//!
//! - Pure Rust implementation using `aws-sdk-s3`
//! - Async operations with Tokio runtime
//! - Multipart upload/download for large files
//! - Resumable transfers with checkpoint support
//! - Flexible authentication (environment, credentials file, IAM roles, explicit)
//! - Support for custom endpoints (MinIO, LocalStack, etc.)
//! - Streaming operations for memory efficiency
//!
//! # Examples
//!
//! ## Basic Upload
//!
//! ```ignore
//! use orbit::protocol::s3::{S3Client, S3Config, S3Operations};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = S3Config {
//!         bucket: "my-bucket".to_string(),
//!         region: Some("us-east-1".to_string()),
//!         ..Default::default()
//!     };
//!
//!     let client = S3Client::new(config).await?;
//!     client.upload_file(Path::new("local/file.txt"), "remote/file.txt").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Download with Resume
//!
//! ```ignore
//! use orbit::protocol::s3::{S3Client, S3Config};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = S3Config {
//!         bucket: "my-bucket".to_string(),
//!         region: Some("us-west-2".to_string()),
//!         ..Default::default()
//!     };
//!
//!     let client = S3Client::new(config).await?;
//!
//!     // Download with automatic resume on failure
//!     client.download_file_resumable("large-file.bin", Path::new("local/file.bin"), 0).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Using MinIO or S3-Compatible Storage
//!
//! ```ignore
//! use orbit::protocol::s3::{S3Client, S3Config, S3Operations};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = S3Config {
//!         bucket: "my-bucket".to_string(),
//!         endpoint: Some("http://localhost:9000".to_string()),
//!         region: Some("us-east-1".to_string()), // Required even for MinIO
//!         access_key: Some("minioadmin".to_string()),
//!         secret_key: Some("minioadmin".to_string()),
//!         ..Default::default()
//!     };
//!
//!     let client = S3Client::new(config).await?;
//!     client.list_objects("prefix/").await?;
//!
//!     Ok(())
//! }
//! ```

mod client;
mod config;
mod error;
mod multipart;
mod operations;
mod types;

// New modules for v0.4.1
pub mod batch;
pub mod progress;
pub mod recovery;
pub mod versioning;

#[cfg(test)]
mod tests;

// Re-export main types
pub use client::S3Client;
pub use config::{S3Config, S3ConfigBuilder};
pub use error::{S3Error, S3Result};
pub use types::{
    ResumeState, S3ListResult, S3Object, S3ObjectMetadata, S3ServerSideEncryption, S3StorageClass,
    UploadPartInfo,
};

// Re-export operations trait for extensibility
pub use operations::S3Operations;

// Re-export new feature traits
pub use batch::BatchOperations;
pub use versioning::VersioningOperations;

/// Default multipart chunk size (5 MB - minimum for S3)
pub const DEFAULT_CHUNK_SIZE: usize = 5 * 1024 * 1024;

/// Maximum multipart chunk size (5 GB)
pub const MAX_CHUNK_SIZE: usize = 5 * 1024 * 1024 * 1024;

/// Minimum multipart chunk size required by S3
pub const MIN_CHUNK_SIZE: usize = 5 * 1024 * 1024;

/// Default number of parallel upload parts
pub const DEFAULT_PARALLEL_PARTS: usize = 4;

/// Maximum number of parallel operations
pub const MAX_PARALLEL_OPERATIONS: usize = 16;
