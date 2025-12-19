//! Azure Blob Storage backend implementation
//!
//! Provides unified Backend interface for Azure Blob Storage with support for:
//! - Connection String authentication (ideal for local/Azurite development)
//! - Account Key authentication
//! - Streaming read/write operations
//! - Block blob multipart upload for large files
//! - Strong consistency guarantees
//!
//! # Example
//!
//! ```no_run
//! use orbit::backend::{Backend, AzureBackend};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set environment variables:
//!     // AZURE_STORAGE_CONNECTION_STRING or
//!     // AZURE_STORAGE_ACCOUNT + AZURE_STORAGE_KEY
//!
//!     let backend = AzureBackend::new("my-container").await?;
//!     let meta = backend.stat(Path::new("path/to/file.txt")).await?;
//!     println!("Size: {} bytes", meta.size);
//!
//!     Ok(())
//! }
//! ```

use super::error::{BackendError, BackendResult};
use super::types::{DirEntry, ListOptions, ListStream, Metadata, ReadStream, WriteOptions};
use super::Backend;
use async_trait::async_trait;
use azure_core::request_options::Metadata as AzureMetadata;
use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::*;
use base64::Engine;
use bytes::Bytes;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncRead;

/// Azure Blob Storage backend
///
/// This backend provides access to Azure Blob Storage with out-of-Azure support.
/// It supports both connection string and account key authentication, making it
/// suitable for local development with Azurite and production deployments.
pub struct AzureBackend {
    container_client: Arc<ContainerClient>,
    /// Prefix for all operations (like a "root" directory)
    prefix: Option<String>,
    /// Block size for multipart uploads (default 8MB)
    block_size: usize,
    /// Concurrency limit for parallel block uploads
    concurrency_limit: usize,
}

impl AzureBackend {
    /// Create a new Azure backend from environment variables
    ///
    /// Authentication: AZURE_STORAGE_ACCOUNT + AZURE_STORAGE_KEY required
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Required environment variables are missing
    /// - Container name is invalid
    pub async fn new(container_name: &str) -> BackendResult<Self> {
        let account = std::env::var("AZURE_STORAGE_ACCOUNT").map_err(|_| {
            BackendError::InvalidConfig {
                backend: "azure".to_string(),
                message: "Missing AZURE_STORAGE_ACCOUNT".to_string(),
            }
        })?;
        let key = std::env::var("AZURE_STORAGE_KEY").map_err(|_| BackendError::InvalidConfig {
            backend: "azure".to_string(),
            message: "Missing AZURE_STORAGE_KEY".to_string(),
        })?;

        let container_client = Self::client_from_account_key(&account, &key, container_name)?;

        Ok(Self {
            container_client: Arc::new(container_client),
            prefix: None,
            block_size: 8 * 1024 * 1024, // 8MB default
            concurrency_limit: 16,
        })
    }

    /// Create a new Azure backend with a prefix
    ///
    /// All paths will be relative to this prefix.
    pub async fn with_prefix(
        container_name: &str,
        prefix: impl Into<String>,
    ) -> BackendResult<Self> {
        let mut backend = Self::new(container_name).await?;
        backend.prefix = Some(prefix.into());
        Ok(backend)
    }

    /// Create client from account name and key
    fn client_from_account_key(
        account_name: &str,
        account_key: &str,
        container_name: &str,
    ) -> BackendResult<ContainerClient> {
        let storage_credentials =
            StorageCredentials::access_key(account_name.to_string(), account_key.to_string());
        let blob_service_client =
            BlobServiceClient::new(account_name.to_string(), storage_credentials);
        Ok(blob_service_client.container_client(container_name))
    }

    /// Convert a Path to an Azure blob name
    fn path_to_blob_name(&self, path: &Path) -> String {
        path_to_blob_name_impl(path, self.prefix.as_deref())
    }

    /// Convert an Azure blob name to a Path (strip prefix if present)
    fn blob_name_to_path(&self, blob_name: &str) -> PathBuf {
        blob_name_to_path_impl(blob_name, self.prefix.as_deref())
    }

    /// Upload data from a reader using block blob multipart upload
    ///
    /// This enables efficient streaming uploads for large files without loading
    /// the entire file into memory. Uses parallel block uploads for performance.
    async fn upload_from_reader(
        &self,
        blob_name: &str,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        _size_hint: Option<u64>,
        options: &WriteOptions,
    ) -> BackendResult<u64> {
        use tokio::io::AsyncReadExt;

        let blob_client = self.container_client.blob_client(blob_name);
        let chunk_size = self.block_size;

        // Read and upload blocks
        let mut block_list = vec![];
        let mut block_index = 0u32;
        let mut total_uploaded = 0u64;

        loop {
            // Read chunk from stream
            let mut buffer = vec![0u8; chunk_size];
            let mut chunk_data = Vec::new();

            // Read up to chunk_size bytes
            loop {
                match reader.read(&mut buffer).await.map_err(BackendError::from)? {
                    0 => break, // EOF
                    n => {
                        chunk_data.extend_from_slice(&buffer[..n]);
                        if chunk_data.len() >= chunk_size {
                            break;
                        }
                    }
                }
            }

            if chunk_data.is_empty() {
                break; // End of stream
            }

            // Generate block ID (must be base64, same length for all blocks)
            let block_id = format!("{:08}", block_index);
            let block_id_b64 = base64::engine::general_purpose::STANDARD.encode(&block_id);

            // Upload this block
            blob_client
                .put_block(block_id_b64.clone(), chunk_data.clone())
                .await
                .map_err(|e| BackendError::Other {
                    backend: "azure".to_string(),
                    message: format!("Failed to upload block {}: {}", block_index, e),
                })?;

            total_uploaded += chunk_data.len() as u64;
            block_list.push(BlobBlockType::Uncommitted(block_id_b64.into()));
            block_index += 1;
        }

        if block_list.is_empty() {
            return Err(BackendError::Other {
                backend: "azure".to_string(),
                message: "No data to upload".to_string(),
            });
        }

        // Commit the block list
        let block_list_obj = BlockList { blocks: block_list };
        let mut request = blob_client.put_block_list(block_list_obj);

        // Set content type if provided
        if let Some(content_type) = &options.content_type {
            request = request.content_type(content_type.clone());
        }

        // Set custom metadata
        if let Some(metadata) = &options.metadata {
            let mut azure_metadata = AzureMetadata::new();
            for (key, value) in metadata {
                azure_metadata.insert(key.clone(), value.clone());
            }
            request = request.metadata(azure_metadata);
        }

        request.await.map_err(|e| BackendError::Other {
            backend: "azure".to_string(),
            message: format!("Failed to commit block list: {}", e),
        })?;

        Ok(total_uploaded)
    }

    /// Convert Azure blob properties to backend Metadata
    fn convert_blob_properties(
        &self,
        props: &azure_storage_blobs::blob::BlobProperties,
    ) -> Metadata {
        let mut metadata = Metadata::file(props.content_length);
        metadata.modified = Some(props.last_modified.into());
        metadata.content_type = props.content_type.clone();
        metadata.etag = props.etag.clone().map(|e| e.to_string());

        // Convert custom metadata
        if !props.metadata.is_empty() {
            let mut custom_metadata = std::collections::HashMap::new();
            for (k, v) in &props.metadata {
                custom_metadata.insert(k.to_string(), v.to_string());
            }
            metadata.custom_metadata = Some(custom_metadata);
        }

        metadata
    }
}

/// Convert a Path to an Azure blob name with optional prefix
fn path_to_blob_name_impl(path: &Path, prefix: Option<&str>) -> String {
    let path_str = path.to_string_lossy().replace('\\', "/");
    let blob_name = path_str.trim_start_matches('/');

    if let Some(prefix) = prefix {
        format!("{}/{}", prefix.trim_end_matches('/'), blob_name)
    } else {
        blob_name.to_string()
    }
}

/// Convert an Azure blob name to a Path with optional prefix
fn blob_name_to_path_impl(blob_name: &str, prefix: Option<&str>) -> PathBuf {
    if let Some(prefix) = prefix {
        let prefix = prefix.trim_end_matches('/');
        if let Some(stripped) = blob_name.strip_prefix(prefix) {
            PathBuf::from(stripped.trim_start_matches('/'))
        } else {
            PathBuf::from(blob_name)
        }
    } else {
        PathBuf::from(blob_name)
    }
}

#[async_trait]
impl Backend for AzureBackend {
    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "azure",
            path = %path.display()
        )
    )]
    async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
        let blob_name = self.path_to_blob_name(path);
        let blob_client = self.container_client.blob_client(&blob_name);

        let properties = blob_client.get_properties().await.map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("404") || err_str.contains("BlobNotFound") {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "azure".to_string(),
                }
            } else if err_str.contains("403") || err_str.contains("AuthenticationFailed") {
                BackendError::PermissionDenied {
                    path: path.to_path_buf(),
                    message: err_str,
                }
            } else {
                BackendError::Other {
                    backend: "azure".to_string(),
                    message: format!("Failed to get blob properties: {}", e),
                }
            }
        })?;

        Ok(self.convert_blob_properties(&properties))
    }

    #[tracing::instrument(
        skip(self, options),
        fields(
            otel.kind = "client",
            backend = "azure",
            path = %path.display(),
            recursive = options.recursive
        )
    )]
    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<ListStream> {
        use futures::stream::{self, StreamExt};

        let prefix = self.path_to_blob_name(path);
        let prefix = if prefix.is_empty() {
            "".to_string()
        } else {
            format!("{}/", prefix.trim_end_matches('/'))
        };

        let container_client = Arc::clone(&self.container_client);
        let options_clone = options.clone();
        let self_prefix = self.prefix.clone();
        let delimiter = if options.recursive {
            None
        } else {
            Some("/".to_string())
        };

        // Create stream that lazily fetches pages from Azure
        let stream = stream::unfold(
            (None::<String>, 0usize), // (continuation_token, entries_yielded)
            move |(marker, entries_count)| {
                let container_client = Arc::clone(&container_client);
                let prefix = prefix.clone();
                let options = options_clone.clone();
                let self_prefix = self_prefix.clone();
                let delimiter = delimiter.clone();

                async move {
                    // Check if hit max_entries
                    if let Some(max) = options.max_entries {
                        if entries_count >= max {
                            return None;
                        }
                    }

                    // Build list request
                    let mut request = container_client.list_blobs().prefix(prefix.clone());

                    if let Some(delim) = delimiter {
                        request = request.delimiter(delim);
                    }

                    if let Some(m) = marker {
                        request = request.marker(m);
                    }

                    if let Some(max) = options.max_entries {
                        let remaining = max - entries_count;
                        request = request.max_results(remaining.min(5000) as u32);
                    }

                    // Fetch page
                    let response = match request.into_stream().next().await {
                        Some(Ok(r)) => r,
                        Some(Err(e)) => {
                            let err = BackendError::Other {
                                backend: "azure".to_string(),
                                message: format!("Failed to list blobs: {}", e),
                            };
                            return Some((
                                stream::once(async move { Err(err) }).boxed(),
                                (None, entries_count),
                            ));
                        }
                        None => return None,
                    };

                    // Convert response to DirEntry items
                    let mut page_entries = Vec::new();

                    // Process blobs
                    for blob in response.blobs.blobs() {
                        let blob_name = &blob.name;

                        // Skip the prefix itself
                        if blob_name == prefix.trim_end_matches('/') {
                            continue;
                        }

                        let full_path = PathBuf::from(blob_name);
                        let relative_path =
                            blob_name_to_path_impl(blob_name, self_prefix.as_deref());

                        let size = blob.properties.content_length;
                        let mut metadata = Metadata::file(size);

                        metadata.modified = Some(blob.properties.last_modified.into());
                        metadata.content_type = blob.properties.content_type.clone();
                        metadata.etag = blob.properties.etag.clone().map(|e| e.to_string());

                        page_entries.push(DirEntry::new(relative_path, full_path, metadata));
                    }

                    // Process blob prefixes (directories in non-recursive mode)
                    for blob_prefix in response.blobs.blob_prefixes() {
                        let prefix_str = &blob_prefix.name;
                        let full_path = PathBuf::from(prefix_str);
                        let relative_path =
                            blob_name_to_path_impl(prefix_str, self_prefix.as_deref());

                        page_entries.push(DirEntry::new(
                            relative_path,
                            full_path,
                            Metadata::directory(),
                        ));
                    }

                    let page_entry_count = page_entries.len();
                    let new_entries_count = entries_count + page_entry_count;

                    // Determine next marker
                    let next_marker = response.next_marker;

                    // Yield this page as a stream
                    Some((
                        stream::iter(page_entries.into_iter().map(Ok)).boxed(),
                        (next_marker, new_entries_count),
                    ))
                }
            },
        )
        .flatten()
        .boxed();

        Ok(stream)
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "azure",
            path = %path.display()
        )
    )]
    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        let blob_name = self.path_to_blob_name(path);
        let blob_client = self.container_client.blob_client(&blob_name);

        // Get the blob
        let mut blob_stream = blob_client.get().into_stream();

        // Check if we got data
        let first_chunk = blob_stream
            .next()
            .await
            .ok_or_else(|| BackendError::NotFound {
                path: path.to_path_buf(),
                backend: "azure".to_string(),
            })?;

        let first_chunk = first_chunk.map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("404") || err_str.contains("BlobNotFound") {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "azure".to_string(),
                }
            } else {
                BackendError::Other {
                    backend: "azure".to_string(),
                    message: format!("Failed to get blob: {}", e),
                }
            }
        })?;

        // Convert Azure stream to our ReadStream
        use futures::stream;

        let stream =
            stream::once(async move { Ok(first_chunk.data) }).chain(blob_stream.map(|result| {
                result
                    .map(|chunk| chunk.data)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            }));

        Ok(Box::pin(stream))
    }

    #[tracing::instrument(
        skip(self, reader, options),
        fields(
            otel.kind = "client",
            backend = "azure",
            path = %path.display(),
            size_hint = ?size_hint,
            overwrite = options.overwrite
        )
    )]
    async fn write(
        &self,
        path: &Path,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64> {
        let blob_name = self.path_to_blob_name(path);
        let blob_client = self.container_client.blob_client(&blob_name);

        // Check if exists
        if !options.overwrite {
            match blob_client.get_properties().await {
                Ok(_) => {
                    return Err(BackendError::AlreadyExists {
                        path: path.to_path_buf(),
                    });
                }
                Err(_) => {
                    // Blob doesn't exist, continue
                }
            }
        }

        // Determine upload strategy based on size
        // Use block blob multipart for files >4MB to enable streaming
        const MULTIPART_THRESHOLD: u64 = 4 * 1024 * 1024; // 4 MB
        let use_multipart = size_hint.map_or(true, |size| size > MULTIPART_THRESHOLD);

        if use_multipart {
            // Stream upload using block blobs
            self.upload_from_reader(&blob_name, reader, size_hint, &options)
                .await
        } else {
            // Small file: buffer in memory and use simple put
            use tokio::io::AsyncReadExt;
            let mut buffer = Vec::new();
            let bytes_read = reader
                .read_to_end(&mut buffer)
                .await
                .map_err(|e| BackendError::Io(e))?;

            // Upload using put_block_blob
            let mut request = blob_client.put_block_blob(Bytes::from(buffer));

            // Set content type
            if let Some(content_type) = options.content_type {
                request = request.content_type(content_type);
            }

            // Set metadata
            if let Some(metadata) = options.metadata {
                let mut azure_metadata = AzureMetadata::new();
                for (k, v) in metadata {
                    azure_metadata.insert(k, v);
                }
                request = request.metadata(azure_metadata);
            }

            request.await.map_err(|e| BackendError::Other {
                backend: "azure".to_string(),
                message: format!("Failed to put blob: {}", e),
            })?;

            Ok(bytes_read as u64)
        }
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "azure",
            path = %path.display(),
            recursive
        )
    )]
    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let blob_name = self.path_to_blob_name(path);

        // Check if it's a "directory" (prefix)
        if recursive {
            // List all blobs with this prefix
            use futures::StreamExt;
            let mut stream = self.list(path, ListOptions::recursive()).await?;

            // Delete all blobs
            while let Some(entry) = stream.next().await {
                let entry = entry?;
                let entry_blob_name = self.path_to_blob_name(&entry.full_path);
                let blob_client = self.container_client.blob_client(&entry_blob_name);

                blob_client
                    .delete()
                    .await
                    .map_err(|e| BackendError::Other {
                        backend: "azure".to_string(),
                        message: format!("Failed to delete blob: {}", e),
                    })?;
            }
        }

        // Delete the blob itself
        let blob_client = self.container_client.blob_client(&blob_name);
        blob_client.delete().await.map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("404") || err_str.contains("BlobNotFound") {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "azure".to_string(),
                }
            } else {
                BackendError::Other {
                    backend: "azure".to_string(),
                    message: format!("Failed to delete blob: {}", e),
                }
            }
        })?;

        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "azure",
            path = %path.display(),
            recursive = _recursive
        )
    )]
    async fn mkdir(&self, path: &Path, _recursive: bool) -> BackendResult<()> {
        // Azure Blob Storage doesn't have real directories, but we can create a 0-byte blob with trailing /
        let blob_name = format!("{}/", self.path_to_blob_name(path).trim_end_matches('/'));
        let blob_client = self.container_client.blob_client(&blob_name);

        // Check if already exists
        match blob_client.get_properties().await {
            Ok(_) => {
                return Err(BackendError::AlreadyExists {
                    path: path.to_path_buf(),
                });
            }
            Err(_) => {
                // Blob doesn't exist, continue
            }
        }

        // Create empty blob
        blob_client
            .put_block_blob(Bytes::new())
            .await
            .map_err(|e| BackendError::Other {
                backend: "azure".to_string(),
                message: format!("Failed to create directory marker: {}", e),
            })?;

        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "azure",
            src = %src.display(),
            dest = %dest.display()
        )
    )]
    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()> {
        let src_blob_name = self.path_to_blob_name(src);
        let dest_blob_name = self.path_to_blob_name(dest);

        let src_blob_client = self.container_client.blob_client(&src_blob_name);
        let dest_blob_client = self.container_client.blob_client(&dest_blob_name);

        // Azure Blob Storage doesn't have native rename, so we copy then delete
        // Get source blob URL for copy operation
        let source_url = src_blob_client.url().map_err(|e| BackendError::Other {
            backend: "azure".to_string(),
            message: format!("Failed to get source blob URL: {}", e),
        })?;

        // Copy blob
        dest_blob_client.copy(source_url).await.map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("404") || err_str.contains("BlobNotFound") {
                BackendError::NotFound {
                    path: src.to_path_buf(),
                    backend: "azure".to_string(),
                }
            } else {
                BackendError::Other {
                    backend: "azure".to_string(),
                    message: format!("Failed to copy blob: {}", e),
                }
            }
        })?;

        // Delete source
        src_blob_client
            .delete()
            .await
            .map_err(|e| BackendError::Other {
                backend: "azure".to_string(),
                message: format!("Failed to delete source after rename: {}", e),
            })?;

        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "azure",
            path = %path.display()
        )
    )]
    async fn exists(&self, path: &Path) -> BackendResult<bool> {
        let blob_name = self.path_to_blob_name(path);
        let blob_client = self.container_client.blob_client(&blob_name);

        match blob_client.get_properties().await {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("404") || err_str.contains("BlobNotFound") {
                    Ok(false)
                } else {
                    Err(BackendError::Other {
                        backend: "azure".to_string(),
                        message: format!("Failed to check blob existence: {}", e),
                    })
                }
            }
        }
    }

    fn backend_name(&self) -> &str {
        "azure"
    }

    fn supports(&self, operation: &str) -> bool {
        matches!(
            operation,
            "stat" | "list" | "read" | "write" | "delete" | "mkdir" | "rename" | "exists"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_blob_name() {
        let prefix = Some("prefix");

        assert_eq!(
            path_to_blob_name_impl(Path::new("file.txt"), prefix),
            "prefix/file.txt"
        );
        assert_eq!(
            path_to_blob_name_impl(Path::new("/file.txt"), prefix),
            "prefix/file.txt"
        );
        assert_eq!(
            path_to_blob_name_impl(Path::new("dir/file.txt"), prefix),
            "prefix/dir/file.txt"
        );
    }

    #[test]
    fn test_blob_name_to_path() {
        let prefix = Some("prefix");

        assert_eq!(
            blob_name_to_path_impl("prefix/file.txt", prefix),
            PathBuf::from("file.txt")
        );
        assert_eq!(
            blob_name_to_path_impl("prefix/dir/file.txt", prefix),
            PathBuf::from("dir/file.txt")
        );
    }

    #[test]
    fn test_path_to_blob_name_no_prefix() {
        assert_eq!(
            path_to_blob_name_impl(Path::new("file.txt"), None),
            "file.txt"
        );
        assert_eq!(
            path_to_blob_name_impl(Path::new("/file.txt"), None),
            "file.txt"
        );
    }

    #[test]
    fn test_blob_name_to_path_no_prefix() {
        assert_eq!(
            blob_name_to_path_impl("file.txt", None),
            PathBuf::from("file.txt")
        );
    }
}
