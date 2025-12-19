//! Google Cloud Storage backend implementation
//!
//! Provides unified Backend interface for Google Cloud Storage with support for:
//! - Service Account authentication (via GOOGLE_APPLICATION_CREDENTIALS)
//! - Access token authentication
//! - Streaming read/write operations
//! - Strong consistency guarantees
//!
//! # Example
//!
//! ```no_run
//! use orbit::backend::{Backend, GcsBackend};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set environment variables:
//!     // GOOGLE_APPLICATION_CREDENTIALS (path to service account JSON)
//!     // or GOOGLE_SERVICE_ACCOUNT + GOOGLE_SERVICE_ACCOUNT_KEY
//!
//!     let backend = GcsBackend::new("my-bucket").await?;
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
use bytes::Bytes;
use futures::stream::{StreamExt, TryStreamExt};
use object_store::gcp::GoogleCloudStorageBuilder;
use object_store::{path::Path as ObjectPath, ObjectMeta, ObjectStore};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncRead;

/// Google Cloud Storage backend using the object_store crate
///
/// This backend provides access to Google Cloud Storage with authentication support
/// via service accounts or access tokens, making it suitable for both development
/// and production deployments.
pub struct GcsBackend {
    store: Arc<dyn ObjectStore>,
    /// Prefix for all operations (like a "root" directory)
    prefix: Option<String>,
}

impl GcsBackend {
    /// Create a new GCS backend from environment variables
    ///
    /// Authentication: GOOGLE_APPLICATION_CREDENTIALS or GOOGLE_SERVICE_ACCOUNT + GOOGLE_SERVICE_ACCOUNT_KEY
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Required environment variables are missing
    /// - Bucket name is invalid
    pub async fn new(bucket_name: &str) -> BackendResult<Self> {
        let store = GoogleCloudStorageBuilder::new()
            .with_bucket_name(bucket_name)
            .build()
            .map_err(|e| BackendError::InvalidConfig {
                backend: "gcs".to_string(),
                message: format!("Failed to create GCS client: {}", e),
            })?;

        Ok(Self {
            store: Arc::new(store),
            prefix: None,
        })
    }

    /// Create a new GCS backend with a prefix
    ///
    /// All paths will be relative to this prefix.
    pub async fn with_prefix(bucket_name: &str, prefix: impl Into<String>) -> BackendResult<Self> {
        let mut backend = Self::new(bucket_name).await?;
        backend.prefix = Some(prefix.into());
        Ok(backend)
    }

    /// Convert a Path to a GCS object name
    fn path_to_object_name(&self, path: &Path) -> ObjectPath {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let object_name = path_str.trim_start_matches('/');

        let full_path = if let Some(prefix) = &self.prefix {
            format!("{}/{}", prefix.trim_end_matches('/'), object_name)
        } else {
            object_name.to_string()
        };

        ObjectPath::from(full_path)
    }

    /// Convert a GCS object name to a Path (strip prefix if present)
    #[allow(dead_code)]
    fn object_name_to_path(&self, object_path: &ObjectPath) -> PathBuf {
        let object_name = object_path.as_ref();

        if let Some(prefix) = &self.prefix {
            let prefix = prefix.trim_end_matches('/');
            if let Some(stripped) = object_name.strip_prefix(prefix) {
                PathBuf::from(stripped.trim_start_matches('/'))
            } else {
                PathBuf::from(object_name)
            }
        } else {
            PathBuf::from(object_name)
        }
    }

    /// Convert object_store ObjectMeta to Backend Metadata
    fn convert_object_meta(&self, meta: &ObjectMeta) -> Metadata {
        let mut metadata = Metadata::file(meta.size as u64);
        metadata.modified = Some(meta.last_modified.into());
        metadata.etag = meta.e_tag.clone();
        metadata
    }
}

#[async_trait]
impl Backend for GcsBackend {
    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "gcs",
            path = %path.display()
        )
    )]
    async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
        let object_path = self.path_to_object_name(path);

        let meta = self.store.head(&object_path).await.map_err(|e| {
            if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "gcs".to_string(),
                }
            } else if e.to_string().contains("403") || e.to_string().contains("Forbidden") {
                BackendError::PermissionDenied {
                    path: path.to_path_buf(),
                    message: e.to_string(),
                }
            } else {
                BackendError::Other {
                    backend: "gcs".to_string(),
                    message: format!("Failed to get object metadata: {}", e),
                }
            }
        })?;

        Ok(self.convert_object_meta(&meta))
    }

    #[tracing::instrument(
        skip(self, options),
        fields(
            otel.kind = "client",
            backend = "gcs",
            path = %path.display(),
            recursive = options.recursive
        )
    )]
    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<ListStream> {
        use futures::stream;

        let prefix = self.path_to_object_name(path);
        let self_prefix = self.prefix.clone();

        // For both recursive and non-recursive, we need to collect results first
        // because object_store's list() requires the store to outlive the stream
        if options.recursive {
            // Collect all items from the stream
            let list_stream = self.store.list(Some(&prefix));
            let items: Vec<_> = list_stream.collect::<Vec<_>>().await;

            let entries = items
                .into_iter()
                .map(|result| {
                    result
                        .map(|meta| {
                            let full_path = PathBuf::from(meta.location.as_ref());
                            let relative_path = if let Some(prefix) = &self_prefix {
                                let prefix = prefix.trim_end_matches('/');
                                let object_name = meta.location.as_ref();
                                if let Some(stripped) = object_name.strip_prefix(prefix) {
                                    PathBuf::from(stripped.trim_start_matches('/'))
                                } else {
                                    PathBuf::from(object_name)
                                }
                            } else {
                                PathBuf::from(meta.location.as_ref())
                            };

                            let mut metadata = Metadata::file(meta.size as u64);
                            metadata.modified = Some(meta.last_modified.into());
                            metadata.etag = meta.e_tag.clone();

                            DirEntry::new(relative_path, full_path, metadata)
                        })
                        .map_err(|e| {
                            BackendError::from(std::io::Error::other(format!("List error: {}", e)))
                        })
                })
                .take(options.max_entries.unwrap_or(usize::MAX))
                .collect::<Vec<_>>();

            Ok(stream::iter(entries).boxed())
        } else {
            // Non-recursive list with delimiter
            let result = self
                .store
                .list_with_delimiter(Some(&prefix))
                .await
                .map_err(|e| {
                    BackendError::from(std::io::Error::other(format!("List error: {}", e)))
                })?;

            let mut entries = Vec::new();

            // Add objects
            for meta in result.objects {
                let full_path = PathBuf::from(meta.location.as_ref());
                let relative_path = if let Some(prefix) = &self_prefix {
                    let prefix = prefix.trim_end_matches('/');
                    let object_name = meta.location.as_ref();
                    if let Some(stripped) = object_name.strip_prefix(prefix) {
                        PathBuf::from(stripped.trim_start_matches('/'))
                    } else {
                        PathBuf::from(object_name)
                    }
                } else {
                    PathBuf::from(meta.location.as_ref())
                };

                let mut metadata = Metadata::file(meta.size as u64);
                metadata.modified = Some(meta.last_modified.into());
                metadata.etag = meta.e_tag.clone();

                entries.push(Ok(DirEntry::new(relative_path, full_path, metadata)));
            }

            // Add common prefixes (directories)
            for prefix_path in result.common_prefixes {
                let full_path = PathBuf::from(prefix_path.as_ref());
                let relative_path = if let Some(prefix) = &self_prefix {
                    let prefix = prefix.trim_end_matches('/');
                    let object_name = prefix_path.as_ref();
                    if let Some(stripped) = object_name.strip_prefix(prefix) {
                        PathBuf::from(stripped.trim_start_matches('/'))
                    } else {
                        PathBuf::from(object_name)
                    }
                } else {
                    PathBuf::from(prefix_path.as_ref())
                };

                entries.push(Ok(DirEntry::new(
                    relative_path,
                    full_path,
                    Metadata::directory(),
                )));
            }

            Ok(stream::iter(entries)
                .take(options.max_entries.unwrap_or(usize::MAX))
                .boxed())
        }
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "gcs",
            path = %path.display()
        )
    )]
    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        let object_path = self.path_to_object_name(path);

        let stream = self
            .store
            .get(&object_path)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                    BackendError::NotFound {
                        path: path.to_path_buf(),
                        backend: "gcs".to_string(),
                    }
                } else {
                    BackendError::Other {
                        backend: "gcs".to_string(),
                        message: format!("Failed to get object: {}", e),
                    }
                }
            })?
            .into_stream()
            .map_err(|e| std::io::Error::other(e.to_string()))
            .boxed();

        Ok(stream)
    }

    #[tracing::instrument(
        skip(self, reader, options),
        fields(
            otel.kind = "client",
            backend = "gcs",
            path = %path.display(),
            overwrite = options.overwrite
        )
    )]
    async fn write(
        &self,
        path: &Path,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        _size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64> {
        let object_path = self.path_to_object_name(path);

        // Check if exists
        if !options.overwrite {
            match self.store.head(&object_path).await {
                Ok(_) => {
                    return Err(BackendError::AlreadyExists {
                        path: path.to_path_buf(),
                    });
                }
                Err(_) => {
                    // Object doesn't exist, continue
                }
            }
        }

        // Read all data into memory (object_store put requires all data upfront)
        // For large files, we could use multipart upload in the future
        use tokio::io::AsyncReadExt;
        let mut buffer = Vec::new();
        let bytes_read = reader
            .read_to_end(&mut buffer)
            .await
            .map_err(BackendError::from)?;

        // Upload to GCS
        self.store
            .put(&object_path, Bytes::from(buffer).into())
            .await
            .map_err(|e| BackendError::Other {
                backend: "gcs".to_string(),
                message: format!("Failed to put object: {}", e),
            })?;

        Ok(bytes_read as u64)
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "gcs",
            path = %path.display(),
            recursive
        )
    )]
    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let object_path = self.path_to_object_name(path);

        if recursive {
            // List all objects with this prefix and delete them
            let list = self.store.list(Some(&object_path));
            futures::pin_mut!(list);

            while let Some(meta) = list.next().await {
                let meta = meta.map_err(|e| BackendError::Other {
                    backend: "gcs".to_string(),
                    message: format!("Failed to list objects for deletion: {}", e),
                })?;

                self.store
                    .delete(&meta.location)
                    .await
                    .map_err(|e| BackendError::Other {
                        backend: "gcs".to_string(),
                        message: format!("Failed to delete object: {}", e),
                    })?;
            }
        }

        // Delete the object itself
        self.store.delete(&object_path).await.map_err(|e| {
            if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "gcs".to_string(),
                }
            } else {
                BackendError::Other {
                    backend: "gcs".to_string(),
                    message: format!("Failed to delete object: {}", e),
                }
            }
        })?;

        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "gcs",
            path = %path.display(),
            recursive = _recursive
        )
    )]
    async fn mkdir(&self, path: &Path, _recursive: bool) -> BackendResult<()> {
        // GCS doesn't have real directories, but we can create a 0-byte object with trailing /
        let object_name = format!("{}/", path.to_string_lossy().replace('\\', "/"));
        let object_path = if let Some(prefix) = &self.prefix {
            ObjectPath::from(format!(
                "{}/{}",
                prefix.trim_end_matches('/'),
                object_name.trim_start_matches('/')
            ))
        } else {
            ObjectPath::from(object_name)
        };

        // Check if already exists
        match self.store.head(&object_path).await {
            Ok(_) => {
                return Err(BackendError::AlreadyExists {
                    path: path.to_path_buf(),
                });
            }
            Err(_) => {
                // Object doesn't exist, continue
            }
        }

        // Create empty object
        self.store
            .put(&object_path, Bytes::new().into())
            .await
            .map_err(|e| BackendError::Other {
                backend: "gcs".to_string(),
                message: format!("Failed to create directory marker: {}", e),
            })?;

        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "gcs",
            src = %src.display(),
            dest = %dest.display()
        )
    )]
    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()> {
        let src_path = self.path_to_object_name(src);
        let dest_path = self.path_to_object_name(dest);

        // GCS doesn't have native rename, so we copy then delete
        self.store.copy(&src_path, &dest_path).await.map_err(|e| {
            if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                BackendError::NotFound {
                    path: src.to_path_buf(),
                    backend: "gcs".to_string(),
                }
            } else {
                BackendError::Other {
                    backend: "gcs".to_string(),
                    message: format!("Failed to copy object: {}", e),
                }
            }
        })?;

        // Delete source
        self.store
            .delete(&src_path)
            .await
            .map_err(|e| BackendError::Other {
                backend: "gcs".to_string(),
                message: format!("Failed to delete source after rename: {}", e),
            })?;

        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "gcs",
            path = %path.display()
        )
    )]
    async fn exists(&self, path: &Path) -> BackendResult<bool> {
        let object_path = self.path_to_object_name(path);

        match self.store.head(&object_path).await {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(BackendError::Other {
                        backend: "gcs".to_string(),
                        message: format!("Failed to check object existence: {}", e),
                    })
                }
            }
        }
    }

    fn backend_name(&self) -> &str {
        "gcs"
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
    fn test_path_to_object_name() {
        let backend = GcsBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: Some("prefix".to_string()),
        };

        assert_eq!(
            backend.path_to_object_name(Path::new("file.txt")).as_ref(),
            "prefix/file.txt"
        );
        assert_eq!(
            backend.path_to_object_name(Path::new("/file.txt")).as_ref(),
            "prefix/file.txt"
        );
        assert_eq!(
            backend
                .path_to_object_name(Path::new("dir/file.txt"))
                .as_ref(),
            "prefix/dir/file.txt"
        );
    }

    #[test]
    fn test_object_name_to_path() {
        let backend = GcsBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: Some("prefix".to_string()),
        };

        assert_eq!(
            backend.object_name_to_path(&ObjectPath::from("prefix/file.txt")),
            PathBuf::from("file.txt")
        );
        assert_eq!(
            backend.object_name_to_path(&ObjectPath::from("prefix/dir/file.txt")),
            PathBuf::from("dir/file.txt")
        );
    }

    #[test]
    fn test_path_to_object_name_no_prefix() {
        let backend = GcsBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: None,
        };

        assert_eq!(
            backend.path_to_object_name(Path::new("file.txt")).as_ref(),
            "file.txt"
        );
        assert_eq!(
            backend.path_to_object_name(Path::new("/file.txt")).as_ref(),
            "file.txt"
        );
    }

    #[test]
    fn test_object_name_to_path_no_prefix() {
        let backend = GcsBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: None,
        };

        assert_eq!(
            backend.object_name_to_path(&ObjectPath::from("file.txt")),
            PathBuf::from("file.txt")
        );
    }
}
