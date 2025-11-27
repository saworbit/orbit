//! S3 backend implementation
//!
//! Wraps the existing S3Client to provide the unified Backend interface.

use super::error::{BackendError, BackendResult};
use super::types::{DirEntry, ListOptions, ListStream, Metadata, ReadStream, WriteOptions};
use super::Backend;
use async_trait::async_trait;
use bytes::Bytes;
use std::path::{Path, PathBuf};
use tokio::io::AsyncRead;

use crate::protocol::s3::{S3Client, S3Config};

/// S3 backend adapter
///
/// This backend provides access to AWS S3 and S3-compatible storage services.
/// It wraps the existing `S3Client` implementation to conform to the unified
/// `Backend` trait.
///
/// # Example
///
/// ```no_run
/// use orbit::backend::{Backend, S3Backend};
/// use orbit::protocol::s3::S3Config;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = S3Config {
///         bucket: "my-bucket".to_string(),
///         region: Some("us-east-1".to_string()),
///         ..Default::default()
///     };
///
///     let backend = S3Backend::new(config).await?;
///     let meta = backend.stat(Path::new("path/to/file.txt")).await?;
///     println!("Size: {} bytes", meta.size);
///
///     Ok(())
/// }
/// ```
pub struct S3Backend {
    client: S3Client,
    /// Prefix for all operations (like a "root" directory)
    prefix: Option<String>,
}

impl S3Backend {
    /// Create a new S3 backend from configuration
    pub async fn new(config: S3Config) -> BackendResult<Self> {
        let client = S3Client::new(config)
            .await
            .map_err(|e| BackendError::InvalidConfig {
                backend: "s3".to_string(),
                message: e.to_string(),
            })?;

        Ok(Self {
            client,
            prefix: None,
        })
    }

    /// Create a new S3 backend with a prefix
    ///
    /// All paths will be relative to this prefix.
    pub async fn with_prefix(config: S3Config, prefix: impl Into<String>) -> BackendResult<Self> {
        let mut backend = Self::new(config).await?;
        backend.prefix = Some(prefix.into());
        Ok(backend)
    }

    /// Convert a Path to an S3 key
    fn path_to_key(&self, path: &Path) -> String {
        path_to_key_impl(path, self.prefix.as_deref())
    }

    /// Convert an S3 key to a Path (strip prefix if present)
    fn key_to_path(&self, key: &str) -> PathBuf {
        key_to_path_impl(key, self.prefix.as_deref())
    }
}

/// Convert a Path to an S3 key with optional prefix (standalone for testing)
fn path_to_key_impl(path: &Path, prefix: Option<&str>) -> String {
    let path_str = path.to_string_lossy().replace('\\', "/");
    let key = path_str.trim_start_matches('/');

    if let Some(prefix) = prefix {
        format!("{}/{}", prefix.trim_end_matches('/'), key)
    } else {
        key.to_string()
    }
}

/// Convert an S3 key to a Path with optional prefix (standalone for testing)
fn key_to_path_impl(key: &str, prefix: Option<&str>) -> PathBuf {
    if let Some(prefix) = prefix {
        let prefix = prefix.trim_end_matches('/');
        if let Some(stripped) = key.strip_prefix(prefix) {
            PathBuf::from(stripped.trim_start_matches('/'))
        } else {
            PathBuf::from(key)
        }
    } else {
        PathBuf::from(key)
    }
}

impl S3Backend {
    /// Convert S3ObjectMetadata to backend Metadata
    fn convert_metadata(&self, s3_meta: crate::protocol::s3::S3ObjectMetadata) -> Metadata {
        let mut metadata = Metadata::file(s3_meta.size);
        metadata.modified = s3_meta.last_modified;
        metadata.content_type = s3_meta.content_type;
        metadata.etag = s3_meta.etag;
        metadata.custom_metadata = Some(s3_meta.metadata);
        metadata
    }

    /// Upload data from a reader using multipart upload
    ///
    /// This enables efficient streaming uploads for large files without loading
    /// the entire file into memory.
    async fn upload_from_reader(
        &self,
        key: &str,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        _size_hint: Option<u64>,
        options: &WriteOptions,
    ) -> BackendResult<u64> {
        use tokio::io::AsyncReadExt;

        // Initiate multipart upload
        let upload_id = self
            .initiate_multipart_upload_with_options(key, options)
            .await?;

        let chunk_size = self.client.config().chunk_size;
        let mut part_number = 1i32;
        let mut completed_parts = Vec::new();
        let mut total_uploaded = 0u64;

        // Upload parts in sequence (streaming from reader)
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

            // Upload this part
            let part_info = self
                .upload_part(key, &upload_id, part_number, Bytes::from(chunk_data))
                .await?;

            total_uploaded += part_info.size as u64;
            completed_parts.push(part_info);
            part_number += 1;
        }

        // Ensure at least one part was uploaded
        if completed_parts.is_empty() {
            // Abort the upload
            self.abort_multipart_upload(key, &upload_id).await?;
            return Err(BackendError::Other {
                backend: "s3".to_string(),
                message: "No data to upload".to_string(),
            });
        }

        // Complete the multipart upload
        self.complete_multipart_upload(key, &upload_id, &completed_parts)
            .await?;

        Ok(total_uploaded)
    }

    /// Initiate a multipart upload with WriteOptions
    async fn initiate_multipart_upload_with_options(
        &self,
        key: &str,
        options: &WriteOptions,
    ) -> BackendResult<String> {
        let mut request = self
            .client
            .aws_client()
            .create_multipart_upload()
            .bucket(self.client.bucket())
            .key(key);

        // Set content type
        if let Some(content_type) = &options.content_type {
            request = request.content_type(content_type);
        }

        // Set metadata
        if let Some(metadata) = &options.metadata {
            for (k, v) in metadata {
                request = request.metadata(k.clone(), v.clone());
            }
        }

        // Set storage class
        request = request.storage_class(self.client.config().storage_class.to_aws());

        // Set server-side encryption
        if let Some(sse) = self.client.config().server_side_encryption.to_aws() {
            request = request.server_side_encryption(sse);

            if let crate::protocol::s3::S3ServerSideEncryption::AwsKms { key_id: Some(kid) } =
                &self.client.config().server_side_encryption
            {
                request = request.ssekms_key_id(kid);
            }
        }

        let response = request.send().await.map_err(|e| BackendError::Other {
            backend: "s3".to_string(),
            message: format!("Failed to initiate multipart upload: {}", e),
        })?;

        response
            .upload_id()
            .ok_or_else(|| BackendError::Other {
                backend: "s3".to_string(),
                message: "No upload ID returned".to_string(),
            })
            .map(|s| s.to_string())
    }

    /// Upload a single part
    async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
    ) -> BackendResult<crate::protocol::s3::UploadPartInfo> {
        use crate::protocol::s3::UploadPartInfo;
        use aws_sdk_s3::primitives::ByteStream;

        let size = data.len();
        let byte_stream = ByteStream::from(data);

        let response = self
            .client
            .aws_client()
            .upload_part()
            .bucket(self.client.bucket())
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(byte_stream)
            .send()
            .await
            .map_err(|e| BackendError::Other {
                backend: "s3".to_string(),
                message: format!("Failed to upload part {}: {}", part_number, e),
            })?;

        let etag = response
            .e_tag()
            .ok_or_else(|| BackendError::Other {
                backend: "s3".to_string(),
                message: format!("No ETag returned for part {}", part_number),
            })?
            .to_string();

        Ok(UploadPartInfo::new(part_number, etag, size))
    }

    /// Complete a multipart upload
    async fn complete_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        parts: &[crate::protocol::s3::UploadPartInfo],
    ) -> BackendResult<()> {
        use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};

        let completed_parts: Vec<CompletedPart> = parts
            .iter()
            .map(|p| {
                CompletedPart::builder()
                    .part_number(p.part_number)
                    .e_tag(&p.etag)
                    .build()
            })
            .collect();

        let multipart_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.client
            .aws_client()
            .complete_multipart_upload()
            .bucket(self.client.bucket())
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(multipart_upload)
            .send()
            .await
            .map_err(|e| BackendError::Other {
                backend: "s3".to_string(),
                message: format!("Failed to complete multipart upload: {}", e),
            })?;

        Ok(())
    }

    /// Abort a multipart upload
    async fn abort_multipart_upload(&self, key: &str, upload_id: &str) -> BackendResult<()> {
        self.client
            .aws_client()
            .abort_multipart_upload()
            .bucket(self.client.bucket())
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|e| BackendError::Other {
                backend: "s3".to_string(),
                message: format!("Failed to abort multipart upload: {}", e),
            })?;

        Ok(())
    }
}

#[async_trait]
impl Backend for S3Backend {
    async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
        let key = self.path_to_key(path);

        let s3_meta = self.client.get_metadata(&key).await.map_err(|e| {
            use crate::protocol::s3::S3Error;
            match e {
                S3Error::NotFound { .. } => BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "s3".to_string(),
                },
                S3Error::AccessDenied(msg) => BackendError::PermissionDenied {
                    path: path.to_path_buf(),
                    message: msg,
                },
                other => BackendError::Other {
                    backend: "s3".to_string(),
                    message: other.to_string(),
                },
            }
        })?;

        Ok(self.convert_metadata(s3_meta))
    }

    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<ListStream> {
        use futures::stream::{self, StreamExt};

        let prefix = self.path_to_key(path);
        let prefix = if prefix.is_empty() {
            "".to_string()
        } else {
            format!("{}/", prefix.trim_end_matches('/'))
        };

        let client = self.client.clone();
        let options_clone = options.clone();
        let self_prefix = self.prefix.clone();

        // Create stream that lazily fetches pages from S3
        let stream = stream::unfold(
            (Some(None::<String>), 0usize), // (continuation_token, entries_yielded)
            move |(token_state, entries_count)| {
                let client = client.clone();
                let prefix = prefix.clone();
                let options = options_clone.clone();
                let self_prefix = self_prefix.clone();

                async move {
                    // Check if we're done or hit max_entries
                    if token_state.is_none() {
                        return None;
                    }

                    if let Some(max) = options.max_entries {
                        if entries_count >= max {
                            return None;
                        }
                    }

                    let token = token_state.unwrap();

                    // Build list request
                    let mut request = client
                        .aws_client()
                        .list_objects_v2()
                        .bucket(client.bucket())
                        .prefix(&prefix);

                    if !options.recursive {
                        request = request.delimiter("/");
                    }

                    if let Some(ref t) = token {
                        request = request.continuation_token(t);
                    }

                    if let Some(max) = options.max_entries {
                        let remaining = max - entries_count;
                        request = request.max_keys(remaining.min(1000) as i32);
                    }

                    // Fetch page
                    let response = match request.send().await {
                        Ok(r) => r,
                        Err(e) => {
                            let err = BackendError::Other {
                                backend: "s3".to_string(),
                                message: format!("Failed to list objects: {}", e),
                            };
                            return Some((
                                stream::once(async move { Err(err) }).boxed(),
                                (None, entries_count),
                            ));
                        }
                    };

                    // Convert response to DirEntry items
                    let mut page_entries = Vec::new();

                    // Process objects
                    for object in response.contents() {
                        if let Some(key) = object.key() {
                            // Skip the prefix itself
                            if key == prefix.trim_end_matches('/') {
                                continue;
                            }

                            let full_path = PathBuf::from(key);
                            let relative_path = match self_prefix.as_ref() {
                                Some(p) => {
                                    let p = p.trim_end_matches('/');
                                    if let Some(stripped) = key.strip_prefix(p) {
                                        PathBuf::from(stripped.trim_start_matches('/'))
                                    } else {
                                        PathBuf::from(key)
                                    }
                                }
                                None => PathBuf::from(key),
                            };

                            let size = object.size().unwrap_or(0) as u64;
                            let mut metadata = Metadata::file(size);

                            if let Some(last_modified) = object.last_modified() {
                                if let Ok(system_time) =
                                    std::time::SystemTime::try_from(*last_modified)
                                {
                                    metadata.modified = Some(system_time);
                                }
                            }

                            metadata.etag = object.e_tag().map(|s| s.to_string());

                            page_entries.push(DirEntry::new(relative_path, full_path, metadata));
                        }
                    }

                    // Process common prefixes (directories in non-recursive mode)
                    if !options.recursive {
                        for common_prefix in response.common_prefixes() {
                            if let Some(prefix_str) = common_prefix.prefix() {
                                let full_path = PathBuf::from(prefix_str);
                                let relative_path = match self_prefix.as_ref() {
                                    Some(p) => {
                                        let p = p.trim_end_matches('/');
                                        if let Some(stripped) = prefix_str.strip_prefix(p) {
                                            PathBuf::from(stripped.trim_start_matches('/'))
                                        } else {
                                            PathBuf::from(prefix_str)
                                        }
                                    }
                                    None => PathBuf::from(prefix_str),
                                };

                                page_entries.push(DirEntry::new(
                                    relative_path,
                                    full_path,
                                    Metadata::directory(),
                                ));
                            }
                        }
                    }

                    let page_entry_count = page_entries.len();
                    let new_entries_count = entries_count + page_entry_count;

                    // Determine next state
                    let next_token = if response.is_truncated().unwrap_or(false) {
                        Some(response.next_continuation_token().map(|s| s.to_string()))
                    } else {
                        None // Done
                    };

                    // Yield this page as a stream
                    Some((
                        stream::iter(page_entries.into_iter().map(Ok)).boxed(),
                        (next_token, new_entries_count),
                    ))
                }
            },
        )
        .flatten()
        .boxed();

        Ok(stream)
    }

    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        let key = self.path_to_key(path);

        // Get the object
        let output = self
            .client
            .aws_client()
            .get_object()
            .bucket(self.client.bucket())
            .key(&key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NoSuchKey") || e.to_string().contains("404") {
                    BackendError::NotFound {
                        path: path.to_path_buf(),
                        backend: "s3".to_string(),
                    }
                } else {
                    BackendError::Other {
                        backend: "s3".to_string(),
                        message: format!("Failed to get object: {}", e),
                    }
                }
            })?;

        // Convert AWS ByteStream to our ReadStream
        use futures::stream;
        use tokio::io::AsyncReadExt;

        let reader = output.body.into_async_read();
        const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB chunks

        let stream = stream::unfold(
            (reader, vec![0u8; CHUNK_SIZE]),
            |(mut reader, mut buffer)| async move {
                match reader.read(&mut buffer).await {
                    Ok(0) => None, // EOF
                    Ok(n) => {
                        let data = Bytes::copy_from_slice(&buffer[..n]);
                        Some((Ok(data), (reader, buffer)))
                    }
                    Err(e) => Some((Err(e), (reader, buffer))),
                }
            },
        );

        Ok(Box::pin(stream))
    }

    async fn write(
        &self,
        path: &Path,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64> {
        let key = self.path_to_key(path);

        // Check if exists
        if !options.overwrite && self.client.exists(&key).await.unwrap_or(false) {
            return Err(BackendError::AlreadyExists {
                path: path.to_path_buf(),
            });
        }

        // Determine upload strategy based on size
        // Use multipart for files >5MB to avoid PutObject limits and enable resumption
        const MULTIPART_THRESHOLD: u64 = 5 * 1024 * 1024; // 5 MB
        let use_multipart = size_hint.map_or(true, |size| size > MULTIPART_THRESHOLD);

        if use_multipart {
            // Stream upload using multipart
            self.upload_from_reader(&key, reader, size_hint, &options)
                .await
        } else {
            // Small file: buffer in memory and use PutObject for efficiency
            use tokio::io::AsyncReadExt;
            let mut buffer = Vec::new();
            let bytes_read = reader
                .read_to_end(&mut buffer)
                .await
                .map_err(|e| BackendError::Io(e))?;

            // Upload using PutObject
            let mut request = self
                .client
                .aws_client()
                .put_object()
                .bucket(self.client.bucket())
                .key(&key)
                .body(Bytes::from(buffer).into());

            // Set content type
            if let Some(content_type) = options.content_type {
                request = request.content_type(content_type);
            }

            // Set metadata
            if let Some(metadata) = options.metadata {
                for (k, v) in metadata {
                    request = request.metadata(k, v);
                }
            }

            request.send().await.map_err(|e| BackendError::Other {
                backend: "s3".to_string(),
                message: format!("Failed to put object: {}", e),
            })?;

            Ok(bytes_read as u64)
        }
    }

    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let key = self.path_to_key(path);

        // Check if it's a "directory" (prefix)
        if recursive {
            // List all objects with this prefix
            use futures::StreamExt;
            let mut stream = self.list(path, ListOptions::recursive()).await?;

            // Delete all objects
            while let Some(entry) = stream.next().await {
                let entry = entry?;
                let entry_key = self.path_to_key(&entry.full_path);
                self.client
                    .delete(&entry_key)
                    .await
                    .map_err(|e| BackendError::Other {
                        backend: "s3".to_string(),
                        message: format!("Failed to delete object: {}", e),
                    })?;
            }
        }

        // Delete the object/prefix itself
        self.client
            .delete(&key)
            .await
            .map_err(|e| BackendError::Other {
                backend: "s3".to_string(),
                message: format!("Failed to delete object: {}", e),
            })?;

        Ok(())
    }

    async fn mkdir(&self, path: &Path, _recursive: bool) -> BackendResult<()> {
        // S3 doesn't have real directories, but we can create a 0-byte object with trailing /
        let key = format!("{}/", self.path_to_key(path).trim_end_matches('/'));

        // Check if already exists
        if self.client.exists(&key).await.unwrap_or(false) {
            return Err(BackendError::AlreadyExists {
                path: path.to_path_buf(),
            });
        }

        // Create empty object
        self.client
            .aws_client()
            .put_object()
            .bucket(self.client.bucket())
            .key(&key)
            .body(Bytes::new().into())
            .send()
            .await
            .map_err(|e| BackendError::Other {
                backend: "s3".to_string(),
                message: format!("Failed to create directory marker: {}", e),
            })?;

        Ok(())
    }

    /// Rename via copy+delete; limited to single-call copy size (5GB) until multipart copy support is added.
    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()> {
        let src_key = self.path_to_key(src);
        let dest_key = self.path_to_key(dest);

        let meta = self.stat(src).await?;
        const MAX_COPY_SIZE: u64 = 5_368_709_120; // 5GB CopyObject limit

        if meta.size > MAX_COPY_SIZE {
            // TODO: add multipart copy for >5GB rename support
            return Err(BackendError::Other {
                backend: "s3".to_string(),
                message: format!(
                    "Cannot rename '{}': File size ({} bytes) exceeds the 5GB limit for atomic S3 copy operations. Please use manual multipart upload/copy for files larger than 5GB.",
                    src.display(),
                    meta.size
                ),
            });
        }

        // S3 doesn't have native rename, so we copy then delete
        self.client
            .aws_client()
            .copy_object()
            .bucket(self.client.bucket())
            .copy_source(format!("{}/{}", self.client.bucket(), src_key))
            .key(&dest_key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NoSuchKey") {
                    BackendError::NotFound {
                        path: src.to_path_buf(),
                        backend: "s3".to_string(),
                    }
                } else {
                    BackendError::Other {
                        backend: "s3".to_string(),
                        message: format!("Failed to copy object: {}", e),
                    }
                }
            })?;

        // Delete source
        self.client
            .delete(&src_key)
            .await
            .map_err(|e| BackendError::Other {
                backend: "s3".to_string(),
                message: format!("Failed to delete source after rename: {}", e),
            })?;

        Ok(())
    }

    async fn exists(&self, path: &Path) -> BackendResult<bool> {
        let key = self.path_to_key(path);
        self.client
            .exists(&key)
            .await
            .map_err(|e| BackendError::Other {
                backend: "s3".to_string(),
                message: e.to_string(),
            })
    }

    fn backend_name(&self) -> &str {
        "s3"
    }

    fn supports(&self, operation: &str) -> bool {
        // S3 supports all operations except native rename (we emulate it with copy+delete)
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
    fn test_path_to_key() {
        let prefix = Some("prefix");

        assert_eq!(
            path_to_key_impl(Path::new("file.txt"), prefix),
            "prefix/file.txt"
        );
        assert_eq!(
            path_to_key_impl(Path::new("/file.txt"), prefix),
            "prefix/file.txt"
        );
        assert_eq!(
            path_to_key_impl(Path::new("dir/file.txt"), prefix),
            "prefix/dir/file.txt"
        );
    }

    #[test]
    fn test_key_to_path() {
        let prefix = Some("prefix");

        assert_eq!(
            key_to_path_impl("prefix/file.txt", prefix),
            PathBuf::from("file.txt")
        );
        assert_eq!(
            key_to_path_impl("prefix/dir/file.txt", prefix),
            PathBuf::from("dir/file.txt")
        );
    }

    #[test]
    fn test_path_to_key_no_prefix() {
        assert_eq!(path_to_key_impl(Path::new("file.txt"), None), "file.txt");
        assert_eq!(path_to_key_impl(Path::new("/file.txt"), None), "file.txt");
    }

    #[test]
    fn test_key_to_path_no_prefix() {
        assert_eq!(
            key_to_path_impl("file.txt", None),
            PathBuf::from("file.txt")
        );
    }
}
