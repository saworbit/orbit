//! Local filesystem implementation of OrbitSystem
//!
//! This provides the default implementation for standalone Orbit operation,
//! wrapping standard Tokio filesystem operations with the OrbitSystem trait.

use orbit_core_interface::{FileMetadata, OrbitSystem, OrbitSystemError, Result};
use std::path::Path;
use std::time::SystemTime;
use tokio::fs;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

/// Local filesystem implementation of OrbitSystem
///
/// This is the default provider for standalone mode, wrapping `tokio::fs`
/// operations. It performs all operations on the local machine.
///
/// # Example
///
/// ```rust,no_run
/// use orbit::system::LocalSystem;
/// use orbit_core_interface::OrbitSystem;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let system = LocalSystem;
///     let exists = system.exists(Path::new("/tmp/test.txt")).await;
///     println!("File exists: {}", exists);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LocalSystem;

impl LocalSystem {
    /// Create a new LocalSystem instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl OrbitSystem for LocalSystem {
    async fn exists(&self, path: &Path) -> bool {
        // Use try_exists for proper async handling
        // Falls back to sync path.exists() if try_exists fails
        fs::try_exists(path).await.unwrap_or_else(|_| path.exists())
    }

    async fn metadata(&self, path: &Path) -> Result<FileMetadata> {
        let meta = fs::metadata(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                OrbitSystemError::NotFound(path.to_path_buf())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                OrbitSystemError::PermissionDenied(path.to_path_buf())
            } else {
                OrbitSystemError::Io(e)
            }
        })?;

        Ok(FileMetadata {
            path: path.to_path_buf(),
            len: meta.len(),
            is_dir: meta.is_dir(),
            modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        })
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<FileMetadata>> {
        let mut entries = Vec::new();
        let mut dir = fs::read_dir(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                OrbitSystemError::NotFound(path.to_path_buf())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                OrbitSystemError::PermissionDenied(path.to_path_buf())
            } else {
                OrbitSystemError::Io(e)
            }
        })?;

        while let Some(entry) = dir.next_entry().await.map_err(OrbitSystemError::Io)? {
            let entry_path = entry.path();
            if let Ok(meta) = entry.metadata().await {
                entries.push(FileMetadata {
                    path: entry_path,
                    len: meta.len(),
                    is_dir: meta.is_dir(),
                    modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                });
            }
        }

        Ok(entries)
    }

    async fn reader(&self, path: &Path) -> Result<Box<dyn AsyncRead + Unpin + Send>> {
        let file = fs::File::open(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                OrbitSystemError::NotFound(path.to_path_buf())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                OrbitSystemError::PermissionDenied(path.to_path_buf())
            } else {
                OrbitSystemError::Io(e)
            }
        })?;

        Ok(Box::new(file))
    }

    async fn writer(&self, path: &Path) -> Result<Box<dyn AsyncWrite + Unpin + Send>> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(OrbitSystemError::Io)?;
        }

        let file = fs::File::create(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                OrbitSystemError::PermissionDenied(path.to_path_buf())
            } else {
                OrbitSystemError::Io(e)
            }
        })?;

        Ok(Box::new(file))
    }

    async fn read_header(&self, path: &Path, len: usize) -> Result<Vec<u8>> {
        let mut file = fs::File::open(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                OrbitSystemError::NotFound(path.to_path_buf())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                OrbitSystemError::PermissionDenied(path.to_path_buf())
            } else {
                OrbitSystemError::Io(e)
            }
        })?;

        let mut buffer = vec![0u8; len];
        let n = file.read(&mut buffer).await.map_err(OrbitSystemError::Io)?;
        buffer.truncate(n);

        Ok(buffer)
    }

    async fn calculate_hash(&self, path: &Path, offset: u64, len: u64) -> Result<[u8; 32]> {
        use tokio::io::AsyncSeekExt;

        let mut file = fs::File::open(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                OrbitSystemError::NotFound(path.to_path_buf())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                OrbitSystemError::PermissionDenied(path.to_path_buf())
            } else {
                OrbitSystemError::Io(e)
            }
        })?;

        // Seek to the specified offset
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .map_err(OrbitSystemError::Io)?;

        // Read the specified length and hash it
        let mut hasher = blake3::Hasher::new();
        let mut remaining = len;
        let mut buffer = vec![0u8; 8192]; // 8KB buffer for reading

        while remaining > 0 {
            let to_read = std::cmp::min(remaining, buffer.len() as u64) as usize;
            let n = file
                .read(&mut buffer[..to_read])
                .await
                .map_err(OrbitSystemError::Io)?;

            if n == 0 {
                // EOF reached before reading the requested length
                break;
            }

            hasher.update(&buffer[..n]);
            remaining -= n as u64;
        }

        Ok(*hasher.finalize().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_core_interface::OrbitSystemExt;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_exists() {
        let system = LocalSystem;

        // Create a temporary file
        let temp = NamedTempFile::new().unwrap();
        assert!(system.exists(temp.path()).await);

        // Non-existent file
        assert!(!system.exists(Path::new("/nonexistent/file.txt")).await);
    }

    #[tokio::test]
    async fn test_metadata() {
        let system = LocalSystem;

        // Create a temporary file with known content
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"Hello, World!").unwrap();
        temp.flush().unwrap();

        let meta = system.metadata(temp.path()).await.unwrap();
        assert_eq!(meta.len, 13); // "Hello, World!" is 13 bytes
        assert!(!meta.is_dir);
    }

    #[tokio::test]
    async fn test_read_header() {
        let system = LocalSystem;

        // Create a temporary file with known content
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"Hello, World! This is a test file.")
            .unwrap();
        temp.flush().unwrap();

        // Read first 13 bytes
        let header = system.read_header(temp.path(), 13).await.unwrap();
        assert_eq!(header, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_calculate_hash() {
        let system = LocalSystem;

        // Create a temporary file with known content
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"Hello, World!").unwrap();
        temp.flush().unwrap();

        // Calculate hash of entire file
        let hash1 = system.calculate_hash(temp.path(), 0, 13).await.unwrap();

        // Calculate hash of first 5 bytes
        let hash2 = system.calculate_hash(temp.path(), 0, 5).await.unwrap();

        // Hashes should be different
        assert_ne!(hash1, hash2);

        // Verify hash of "Hello"
        let expected_hash = blake3::hash(b"Hello");
        assert_eq!(hash2, *expected_hash.as_bytes());
    }

    #[tokio::test]
    async fn test_read_write() {
        let system = LocalSystem;

        // Create a temporary directory
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Write data
        let data = b"Test data for read/write";
        system.write_all(&file_path, data).await.unwrap();

        // Read data back
        let read_data = system.read_all(&file_path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_read_dir() {
        let system = LocalSystem;

        // Create a temporary directory with some files
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file1.txt"), b"test1").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), b"test2").unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        // Read directory
        let entries = system.read_dir(temp_dir.path()).await.unwrap();
        assert_eq!(entries.len(), 3);

        // Verify we have both files and the directory
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"file1.txt".to_string()));
        assert!(names.contains(&"file2.txt".to_string()));
        assert!(names.contains(&"subdir".to_string()));
    }
}
