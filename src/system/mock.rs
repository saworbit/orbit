//! Mock filesystem implementation for testing
//!
//! This provides an in-memory implementation of OrbitSystem that can be used
//! in unit tests without requiring actual filesystem operations.

use orbit_core_interface::{FileMetadata, OrbitSystem, OrbitSystemError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use tokio::io::{AsyncRead, AsyncWrite};

/// In-memory file data
#[derive(Debug, Clone)]
struct MockFile {
    data: Vec<u8>,
    modified: SystemTime,
    is_dir: bool,
}

/// Mock filesystem implementation for testing
///
/// This implementation stores files in memory and allows tests to run
/// without touching the actual filesystem.
///
/// # Example
///
/// ```rust
/// use orbit::system::MockSystem;
/// use orbit_core_interface::OrbitSystem;
/// use std::path::Path;
///
/// #[tokio::test]
/// async fn test_with_mock() {
///     let system = MockSystem::new();
///     system.add_file("/test.txt", b"Hello, World!");
///
///     let exists = system.exists(Path::new("/test.txt")).await;
///     assert!(exists);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MockSystem {
    files: Arc<RwLock<HashMap<PathBuf, MockFile>>>,
}

impl MockSystem {
    /// Create a new empty mock filesystem
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a file with the given content
    pub fn add_file(&self, path: impl Into<PathBuf>, data: &[u8]) {
        let path = path.into();
        let file = MockFile {
            data: data.to_vec(),
            modified: SystemTime::now(),
            is_dir: false,
        };
        self.files.write().unwrap().insert(path, file);
    }

    /// Add a directory
    pub fn add_dir(&self, path: impl Into<PathBuf>) {
        let path = path.into();
        let dir = MockFile {
            data: Vec::new(),
            modified: SystemTime::now(),
            is_dir: true,
        };
        self.files.write().unwrap().insert(path, dir);
    }

    /// Remove a file or directory
    pub fn remove(&self, path: &Path) {
        self.files.write().unwrap().remove(path);
    }

    /// Clear all files
    pub fn clear(&self) {
        self.files.write().unwrap().clear();
    }

    /// Get file data (for testing)
    pub fn get_data(&self, path: &Path) -> Option<Vec<u8>> {
        self.files.read().unwrap().get(path).map(|f| f.data.clone())
    }
}

impl Default for MockSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl OrbitSystem for MockSystem {
    async fn exists(&self, path: &Path) -> bool {
        self.files.read().unwrap().contains_key(path)
    }

    async fn metadata(&self, path: &Path) -> Result<FileMetadata> {
        let files = self.files.read().unwrap();
        let file = files
            .get(path)
            .ok_or_else(|| OrbitSystemError::NotFound(path.to_path_buf()))?;

        Ok(FileMetadata {
            path: path.to_path_buf(),
            len: file.data.len() as u64,
            is_dir: file.is_dir,
            modified: file.modified,
        })
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<FileMetadata>> {
        let files = self.files.read().unwrap();

        // Check if directory exists
        if !files.contains_key(path) {
            return Err(OrbitSystemError::NotFound(path.to_path_buf()));
        }

        // Find all direct children
        let mut entries = Vec::new();
        for (file_path, file) in files.iter() {
            if let Some(parent) = file_path.parent() {
                if parent == path {
                    entries.push(FileMetadata {
                        path: file_path.clone(),
                        len: file.data.len() as u64,
                        is_dir: file.is_dir,
                        modified: file.modified,
                    });
                }
            }
        }

        Ok(entries)
    }

    async fn reader(&self, path: &Path) -> Result<Box<dyn AsyncRead + Unpin + Send>> {
        let files = self.files.read().unwrap();
        let file = files
            .get(path)
            .ok_or_else(|| OrbitSystemError::NotFound(path.to_path_buf()))?;

        // Create a cursor that implements AsyncRead
        let cursor = tokio::io::BufReader::new(std::io::Cursor::new(file.data.clone()));
        Ok(Box::new(cursor))
    }

    async fn writer(&self, path: &Path) -> Result<Box<dyn AsyncWrite + Unpin + Send>> {
        // For the mock, we create a buffer that will be written back to the HashMap
        // when dropped. For simplicity, we'll just create a vector.
        // In a real implementation, we'd use a custom writer that updates the HashMap.

        // For now, create parent directories if needed
        if let Some(parent) = path.parent() {
            if !self.exists(parent).await {
                self.add_dir(parent);
            }
        }

        // Return a buffer writer
        let buffer = Vec::new();
        Ok(Box::new(tokio::io::BufWriter::new(std::io::Cursor::new(
            buffer,
        ))))
    }

    async fn read_header(&self, path: &Path, len: usize) -> Result<Vec<u8>> {
        let files = self.files.read().unwrap();
        let file = files
            .get(path)
            .ok_or_else(|| OrbitSystemError::NotFound(path.to_path_buf()))?;

        let end = std::cmp::min(len, file.data.len());
        Ok(file.data[..end].to_vec())
    }

    async fn calculate_hash(&self, path: &Path, offset: u64, len: u64) -> Result<[u8; 32]> {
        let files = self.files.read().unwrap();
        let file = files
            .get(path)
            .ok_or_else(|| OrbitSystemError::NotFound(path.to_path_buf()))?;

        let start = offset as usize;
        let end = std::cmp::min(start + len as usize, file.data.len());

        if start >= file.data.len() {
            // Return hash of empty data if offset is beyond file size
            return Ok(*blake3::hash(&[]).as_bytes());
        }

        let hash = blake3::hash(&file.data[start..end]);
        Ok(*hash.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_core_interface::OrbitSystemExt;

    #[tokio::test]
    async fn test_mock_basic_operations() {
        let system = MockSystem::new();

        // Add a file
        system.add_file("/test.txt", b"Hello, World!");

        // Check existence
        assert!(system.exists(Path::new("/test.txt")).await);
        assert!(!system.exists(Path::new("/nonexistent.txt")).await);

        // Get metadata
        let meta = system.metadata(Path::new("/test.txt")).await.unwrap();
        assert_eq!(meta.len, 13);
        assert!(!meta.is_dir);

        // Read data
        let data = system.read_all(Path::new("/test.txt")).await.unwrap();
        assert_eq!(data, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_mock_read_header() {
        let system = MockSystem::new();
        system.add_file("/test.txt", b"Hello, World!");

        let header = system.read_header(Path::new("/test.txt"), 5).await.unwrap();
        assert_eq!(header, b"Hello");
    }

    #[tokio::test]
    async fn test_mock_calculate_hash() {
        let system = MockSystem::new();
        system.add_file("/test.txt", b"Hello, World!");

        // Hash entire file
        let hash1 = system
            .calculate_hash(Path::new("/test.txt"), 0, 13)
            .await
            .unwrap();

        // Hash first 5 bytes
        let hash2 = system
            .calculate_hash(Path::new("/test.txt"), 0, 5)
            .await
            .unwrap();

        assert_ne!(hash1, hash2);

        // Verify hash of "Hello"
        let expected = blake3::hash(b"Hello");
        assert_eq!(hash2, *expected.as_bytes());
    }

    #[tokio::test]
    async fn test_mock_directories() {
        let system = MockSystem::new();

        // Add directory structure
        system.add_dir("/data");
        system.add_file("/data/file1.txt", b"content1");
        system.add_file("/data/file2.txt", b"content2");
        system.add_dir("/data/subdir");

        // List directory
        let entries = system.read_dir(Path::new("/data")).await.unwrap();
        assert_eq!(entries.len(), 3);

        // Verify entries
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"file1.txt".to_string()));
        assert!(names.contains(&"file2.txt".to_string()));
        assert!(names.contains(&"subdir".to_string()));
    }

    #[tokio::test]
    async fn test_mock_remove() {
        let system = MockSystem::new();
        system.add_file("/test.txt", b"data");

        assert!(system.exists(Path::new("/test.txt")).await);

        system.remove(Path::new("/test.txt"));

        assert!(!system.exists(Path::new("/test.txt")).await);
    }

    #[tokio::test]
    async fn test_mock_clear() {
        let system = MockSystem::new();
        system.add_file("/file1.txt", b"data1");
        system.add_file("/file2.txt", b"data2");

        system.clear();

        assert!(!system.exists(Path::new("/file1.txt")).await);
        assert!(!system.exists(Path::new("/file2.txt")).await);
    }
}
