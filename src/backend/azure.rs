//! Azure Blob Storage backend implementation
//!
//! Provides unified Backend interface for Azure Blob Storage with support for:
//! - Connection String authentication (ideal for local/Azurite development)
//! - Account Key authentication
//! - Streaming read/write operations
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
use bytes::Bytes;
use futures::stream::{StreamExt, TryStreamExt};
use object_store::azure::MicrosoftAzureBuilder;
use object_store::{path::Path as ObjectPath, ObjectMeta, ObjectStore};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncRead;

/// Azure Blob Storage backend using the object_store crate
///
/// This backend provides access to Azure Blob Storage with out-of-Azure support.
/// It supports both connection string and account key authentication, making it
/// suitable for local development with Azurite and production deployments.
pub struct AzureBackend {
    store: Arc<dyn ObjectStore>,
    /// Prefix for all operations (like a "root" directory)
    prefix: Option<String>,
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
        let store = MicrosoftAzureBuilder::new()
            .with_container_name(container_name)
            .build()
            .map_err(|e| BackendError::InvalidConfig {
                backend: "azure".to_string(),
                message: format!("Failed to create Azure client: {}", e),
            })?;

        Ok(Self {
            store: Arc::new(store),
            prefix: None,
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

    /// Convert a Path to an Azure blob name
    fn path_to_blob_name(&self, path: &Path) -> ObjectPath {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let blob_name = path_str.trim_start_matches('/');

        let full_path = if let Some(prefix) = &self.prefix {
            format!("{}/{}", prefix.trim_end_matches('/'), blob_name)
        } else {
            blob_name.to_string()
        };

        ObjectPath::from(full_path)
    }

    /// Convert an Azure blob name to a Path (strip prefix if present)
    #[allow(dead_code)]
    fn blob_name_to_path(&self, object_path: &ObjectPath) -> PathBuf {
        let blob_name = object_path.as_ref();

        if let Some(prefix) = &self.prefix {
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

    /// Convert object_store ObjectMeta to Backend Metadata
    fn convert_object_meta(&self, meta: &ObjectMeta) -> Metadata {
        let mut metadata = Metadata::file(meta.size as u64);
        metadata.modified = Some(meta.last_modified.into());
        metadata.etag = meta.e_tag.clone();
        metadata
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
        let object_path = self.path_to_blob_name(path);

        let meta = self.store.head(&object_path).await.map_err(|e| {
            if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "azure".to_string(),
                }
            } else if e.to_string().contains("403") || e.to_string().contains("Forbidden") {
                BackendError::PermissionDenied {
                    path: path.to_path_buf(),
                    message: e.to_string(),
                }
            } else {
                BackendError::Other {
                    backend: "azure".to_string(),
                    message: format!("Failed to get blob metadata: {}", e),
                }
            }
        })?;

        Ok(self.convert_object_meta(&meta))
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
        use futures::stream;

        let prefix = self.path_to_blob_name(path);
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
                                let blob_name = meta.location.as_ref();
                                if let Some(stripped) = blob_name.strip_prefix(prefix) {
                                    PathBuf::from(stripped.trim_start_matches('/'))
                                } else {
                                    PathBuf::from(blob_name)
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
                    let blob_name = meta.location.as_ref();
                    if let Some(stripped) = blob_name.strip_prefix(prefix) {
                        PathBuf::from(stripped.trim_start_matches('/'))
                    } else {
                        PathBuf::from(blob_name)
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
                    let blob_name = prefix_path.as_ref();
                    if let Some(stripped) = blob_name.strip_prefix(prefix) {
                        PathBuf::from(stripped.trim_start_matches('/'))
                    } else {
                        PathBuf::from(blob_name)
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
            backend = "azure",
            path = %path.display()
        )
    )]
    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        let object_path = self.path_to_blob_name(path);

        let stream = self
            .store
            .get(&object_path)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
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
            backend = "azure",
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
        let object_path = self.path_to_blob_name(path);

        // Check if exists
        if !options.overwrite {
            match self.store.head(&object_path).await {
                Ok(_) => {
                    return Err(BackendError::AlreadyExists {
                        path: path.to_path_buf(),
                    });
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("404") || msg.contains("NotFound") {
                        // Blob doesn't exist, continue
                    } else {
                        return Err(BackendError::Other {
                            backend: "azure".to_string(),
                            message: format!("Failed to check blob existence: {}", e),
                        });
                    }
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

        // Upload to Azure
        self.store
            .put(&object_path, Bytes::from(buffer).into())
            .await
            .map_err(|e| BackendError::Other {
                backend: "azure".to_string(),
                message: format!("Failed to put blob: {}", e),
            })?;

        Ok(bytes_read as u64)
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
        let object_path = self.path_to_blob_name(path);

        if recursive {
            // List all objects with this prefix and delete them
            let list = self.store.list(Some(&object_path));
            futures::pin_mut!(list);

            while let Some(meta) = list.next().await {
                let meta = meta.map_err(|e| BackendError::Other {
                    backend: "azure".to_string(),
                    message: format!("Failed to list objects for deletion: {}", e),
                })?;

                self.store
                    .delete(&meta.location)
                    .await
                    .map_err(|e| BackendError::Other {
                        backend: "azure".to_string(),
                        message: format!("Failed to delete object: {}", e),
                    })?;
            }
        }

        // Delete the object itself
        self.store.delete(&object_path).await.map_err(|e| {
            if e.to_string().contains("404") || e.to_string().contains("NotFound") {
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
        let blob_name = format!("{}/", path.to_string_lossy().replace('\\', "/"));
        let object_path = if let Some(prefix) = &self.prefix {
            ObjectPath::from(format!(
                "{}/{}",
                prefix.trim_end_matches('/'),
                blob_name.trim_start_matches('/')
            ))
        } else {
            ObjectPath::from(blob_name)
        };

        // Check if already exists
        match self.store.head(&object_path).await {
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
        self.store
            .put(&object_path, Bytes::new().into())
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
        let src_path = self.path_to_blob_name(src);
        let dest_path = self.path_to_blob_name(dest);

        // Azure Blob Storage doesn't have native rename, so we copy then delete
        self.store.copy(&src_path, &dest_path).await.map_err(|e| {
            if e.to_string().contains("404") || e.to_string().contains("NotFound") {
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
        self.store
            .delete(&src_path)
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
        let object_path = self.path_to_blob_name(path);

        match self.store.head(&object_path).await {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
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
        let backend = AzureBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: Some("prefix".to_string()),
        };

        assert_eq!(
            backend.path_to_blob_name(Path::new("file.txt")).as_ref(),
            "prefix/file.txt"
        );
        assert_eq!(
            backend.path_to_blob_name(Path::new("/file.txt")).as_ref(),
            "prefix/file.txt"
        );
        assert_eq!(
            backend
                .path_to_blob_name(Path::new("dir/file.txt"))
                .as_ref(),
            "prefix/dir/file.txt"
        );
    }

    #[test]
    fn test_blob_name_to_path() {
        let backend = AzureBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: Some("prefix".to_string()),
        };

        assert_eq!(
            backend.blob_name_to_path(&ObjectPath::from("prefix/file.txt")),
            PathBuf::from("file.txt")
        );
        assert_eq!(
            backend.blob_name_to_path(&ObjectPath::from("prefix/dir/file.txt")),
            PathBuf::from("dir/file.txt")
        );
    }

    #[test]
    fn test_path_to_blob_name_no_prefix() {
        let backend = AzureBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: None,
        };

        assert_eq!(
            backend.path_to_blob_name(Path::new("file.txt")).as_ref(),
            "file.txt"
        );
        assert_eq!(
            backend.path_to_blob_name(Path::new("/file.txt")).as_ref(),
            "file.txt"
        );
    }

    #[test]
    fn test_blob_name_to_path_no_prefix() {
        let backend = AzureBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: None,
        };

        assert_eq!(
            backend.blob_name_to_path(&ObjectPath::from("file.txt")),
            PathBuf::from("file.txt")
        );
    }
}
