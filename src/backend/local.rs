//! Local filesystem backend implementation
//!
//! Provides async access to the local filesystem using Tokio's async I/O.

use super::error::{BackendError, BackendResult};
use super::types::{DirEntry, ListOptions, Metadata, ReadStream, WriteOptions};
use super::Backend;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{self, StreamExt};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Local filesystem backend
///
/// This backend provides async access to the local filesystem using Tokio.
/// It supports all standard filesystem operations including reading, writing,
/// listing, and metadata queries.
///
/// # Example
///
/// ```no_run
/// use orbit::backend::{Backend, LocalBackend};
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let backend = LocalBackend::new();
///
///     // Get file metadata
///     let meta = backend.stat(Path::new("/tmp/file.txt")).await?;
///     println!("Size: {} bytes", meta.size);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct LocalBackend {
    /// Root directory for this backend (optional constraint)
    root: Option<PathBuf>,
}

impl LocalBackend {
    /// Create a new local backend with no root constraint
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Create a new local backend rooted at a specific directory
    ///
    /// All paths will be resolved relative to this root.
    pub fn with_root<P: Into<PathBuf>>(root: P) -> Self {
        Self {
            root: Some(root.into()),
        }
    }

    /// Resolve a path relative to the root (if set)
    fn resolve_path(&self, path: &Path) -> PathBuf {
        if let Some(root) = &self.root {
            root.join(path.strip_prefix("/").unwrap_or(path))
        } else {
            path.to_path_buf()
        }
    }

    /// Convert std::fs::Metadata to backend Metadata
    fn convert_metadata(&self, path: &Path, meta: std::fs::Metadata) -> Metadata {
        let mut metadata = if meta.is_file() {
            Metadata::file(meta.len())
        } else if meta.is_dir() {
            Metadata::directory()
        } else {
            // Symlink or other
            Metadata::symlink(meta.len())
        };

        metadata.modified = meta.modified().ok();
        metadata.created = meta.created().ok();
        metadata.accessed = meta.accessed().ok();

        // Get Unix permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions = Some(meta.permissions().mode());
        }

        // Guess content type from extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            metadata.content_type = match ext {
                "txt" => Some("text/plain".to_string()),
                "html" | "htm" => Some("text/html".to_string()),
                "json" => Some("application/json".to_string()),
                "xml" => Some("application/xml".to_string()),
                "pdf" => Some("application/pdf".to_string()),
                "jpg" | "jpeg" => Some("image/jpeg".to_string()),
                "png" => Some("image/png".to_string()),
                "gif" => Some("image/gif".to_string()),
                _ => None,
            };
        }

        metadata
    }

    /// Recursively list directory entries
    fn list_recursive<'a>(
        &'a self,
        path: &'a Path,
        base_path: &'a Path,
        options: &'a ListOptions,
        current_depth: usize,
        entries: &'a mut Vec<DirEntry>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BackendResult<()>> + Send + 'a>> {
        Box::pin(async move {
        // Check max depth
        if let Some(max_depth) = options.max_depth {
            if current_depth >= max_depth {
                return Ok(());
            }
        }

        // Check max entries
        if let Some(max_entries) = options.max_entries {
            if entries.len() >= max_entries {
                return Ok(());
            }
        }

        let resolved = self.resolve_path(path);
        let mut read_dir = fs::read_dir(&resolved).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "local".to_string(),
                }
            } else {
                BackendError::from(e)
            }
        })?;

        while let Some(entry) = read_dir.next_entry().await.map_err(BackendError::from)? {
            let entry_path = entry.path();
            let file_name = entry.file_name();

            // Skip hidden files if not included
            if !options.include_hidden {
                if let Some(name) = file_name.to_str() {
                    if name.starts_with('.') {
                        continue;
                    }
                }
            }

            let metadata = entry.metadata().await.map_err(BackendError::from)?;
            let relative_path = entry_path.strip_prefix(base_path).unwrap_or(&entry_path);

            let is_symlink = metadata.is_symlink();
            let should_follow = options.follow_symlinks && is_symlink;

            // Get actual metadata if following symlinks
            let actual_metadata = if should_follow {
                fs::metadata(&entry_path)
                    .await
                    .map_err(BackendError::from)?
            } else {
                metadata.clone()
            };

            let backend_meta = self.convert_metadata(&entry_path, actual_metadata.clone());

            entries.push(DirEntry::new(
                relative_path.to_path_buf(),
                entry_path.clone(),
                backend_meta.clone(),
            ));

            // Recurse into directories
            if options.recursive && actual_metadata.is_dir() {
                self.list_recursive(&entry_path, base_path, options, current_depth + 1, entries)
                    .await?;
            }
        }

        Ok(())
        })
    }
}

impl Default for LocalBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Backend for LocalBackend {
    async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
        let resolved = self.resolve_path(path);
        let meta = fs::metadata(&resolved).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "local".to_string(),
                }
            } else {
                BackendError::from(e)
            }
        })?;

        Ok(self.convert_metadata(&resolved, meta))
    }

    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<Vec<DirEntry>> {
        let resolved = self.resolve_path(path);

        // Verify it's a directory
        let meta = fs::metadata(&resolved).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "local".to_string(),
                }
            } else {
                BackendError::from(e)
            }
        })?;

        if !meta.is_dir() {
            return Err(BackendError::InvalidPath {
                path: path.to_path_buf(),
                reason: "not a directory".to_string(),
            });
        }

        let mut entries = Vec::new();

        if options.recursive {
            self.list_recursive(&resolved, &resolved, &options, 0, &mut entries)
                .await?;
        } else {
            self.list_recursive(&resolved, &resolved, &options, 0, &mut entries)
                .await?;
            // Filter to only direct children
            entries.retain(|e| {
                e.path.components().count() == 1 || e.path == PathBuf::from("")
            });
        }

        Ok(entries)
    }

    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        let resolved = self.resolve_path(path);
        let file = fs::File::open(&resolved).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "local".to_string(),
                }
            } else {
                BackendError::from(e)
            }
        })?;

        // Create a stream that reads the file in chunks
        const CHUNK_SIZE: usize = 64 * 1024; // 64 KB chunks

        let stream = stream::unfold((file, vec![0u8; CHUNK_SIZE]), |(mut file, mut buffer)| async move {
            match file.read(&mut buffer).await {
                Ok(0) => None, // EOF
                Ok(n) => {
                    let data = Bytes::copy_from_slice(&buffer[..n]);
                    Some((Ok(data), (file, buffer)))
                }
                Err(e) => Some((Err(e), (file, buffer))),
            }
        });

        Ok(Box::pin(stream))
    }

    async fn write(&self, path: &Path, data: Bytes, options: WriteOptions) -> BackendResult<u64> {
        let resolved = self.resolve_path(path);

        // Create parent directories if needed
        if options.create_parents {
            if let Some(parent) = resolved.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(BackendError::from)?;
            }
        }

        // Check if file exists
        if !options.overwrite && resolved.exists() {
            return Err(BackendError::AlreadyExists {
                path: path.to_path_buf(),
            });
        }

        // Write the file
        let mut file = fs::File::create(&resolved)
            .await
            .map_err(BackendError::from)?;

        file.write_all(&data).await.map_err(BackendError::from)?;
        file.flush().await.map_err(BackendError::from)?;

        // Set permissions if specified
        #[cfg(unix)]
        if let Some(perms) = options.permissions {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(perms);
            fs::set_permissions(&resolved, permissions)
                .await
                .map_err(BackendError::from)?;
        }

        Ok(data.len() as u64)
    }

    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let resolved = self.resolve_path(path);

        let meta = fs::metadata(&resolved).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackendError::NotFound {
                    path: path.to_path_buf(),
                    backend: "local".to_string(),
                }
            } else {
                BackendError::from(e)
            }
        })?;

        if meta.is_dir() {
            if recursive {
                fs::remove_dir_all(&resolved)
                    .await
                    .map_err(BackendError::from)?;
            } else {
                fs::remove_dir(&resolved).await.map_err(|e| {
                    if e.kind() == std::io::ErrorKind::Other
                        || e.to_string().contains("not empty")
                    {
                        BackendError::DirectoryNotEmpty {
                            path: path.to_path_buf(),
                        }
                    } else {
                        BackendError::from(e)
                    }
                })?;
            }
        } else {
            fs::remove_file(&resolved)
                .await
                .map_err(BackendError::from)?;
        }

        Ok(())
    }

    async fn mkdir(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let resolved = self.resolve_path(path);

        if resolved.exists() {
            return Err(BackendError::AlreadyExists {
                path: path.to_path_buf(),
            });
        }

        if recursive {
            fs::create_dir_all(&resolved)
                .await
                .map_err(BackendError::from)?;
        } else {
            fs::create_dir(&resolved).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BackendError::NotFound {
                        path: path.parent().unwrap_or(path).to_path_buf(),
                        backend: "local".to_string(),
                    }
                } else {
                    BackendError::from(e)
                }
            })?;
        }

        Ok(())
    }

    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()> {
        let resolved_src = self.resolve_path(src);
        let resolved_dest = self.resolve_path(dest);

        // Check source exists
        if !resolved_src.exists() {
            return Err(BackendError::NotFound {
                path: src.to_path_buf(),
                backend: "local".to_string(),
            });
        }

        // Check destination doesn't exist
        if resolved_dest.exists() {
            return Err(BackendError::AlreadyExists {
                path: dest.to_path_buf(),
            });
        }

        fs::rename(&resolved_src, &resolved_dest)
            .await
            .map_err(BackendError::from)?;

        Ok(())
    }

    fn backend_name(&self) -> &str {
        "local"
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
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_local_backend_stat() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create a test file
        let mut file = fs::File::create(&file_path).await.unwrap();
        file.write_all(b"test data").await.unwrap();
        drop(file);

        let backend = LocalBackend::new();
        let meta = backend.stat(&file_path).await.unwrap();

        assert!(meta.is_file);
        assert_eq!(meta.size, 9);
    }

    #[tokio::test]
    async fn test_local_backend_list() {
        let temp_dir = TempDir::new().unwrap();

        // Create some files
        fs::File::create(temp_dir.path().join("file1.txt"))
            .await
            .unwrap();
        fs::File::create(temp_dir.path().join("file2.txt"))
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("subdir"))
            .await
            .unwrap();

        let backend = LocalBackend::new();
        let entries = backend
            .list(temp_dir.path(), ListOptions::shallow())
            .await
            .unwrap();

        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn test_local_backend_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let backend = LocalBackend::new();
        let data = Bytes::from("test data");

        // Write
        let written = backend
            .write(&file_path, data.clone(), WriteOptions::default())
            .await
            .unwrap();
        assert_eq!(written, 9);

        // Read
        let mut stream = backend.read(&file_path).await.unwrap();
        let mut result = Vec::new();
        while let Some(chunk) = stream.next().await {
            result.extend_from_slice(&chunk.unwrap());
        }

        assert_eq!(result, b"test data");
    }

    #[tokio::test]
    async fn test_local_backend_mkdir_delete() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("newdir");

        let backend = LocalBackend::new();

        // Create directory
        backend.mkdir(&dir_path, false).await.unwrap();
        assert!(dir_path.exists());

        // Delete directory
        backend.delete(&dir_path, false).await.unwrap();
        assert!(!dir_path.exists());
    }

    #[tokio::test]
    async fn test_local_backend_rename() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dest_path = temp_dir.path().join("dest.txt");

        // Create source file
        fs::File::create(&src_path).await.unwrap();

        let backend = LocalBackend::new();
        backend.rename(&src_path, &dest_path).await.unwrap();

        assert!(!src_path.exists());
        assert!(dest_path.exists());
    }

    #[tokio::test]
    async fn test_local_backend_with_root() {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalBackend::with_root(temp_dir.path());

        // Create file using relative path
        let data = Bytes::from("test");
        backend
            .write(Path::new("test.txt"), data, WriteOptions::default())
            .await
            .unwrap();

        // Verify file exists in root
        assert!(temp_dir.path().join("test.txt").exists());
    }
}
