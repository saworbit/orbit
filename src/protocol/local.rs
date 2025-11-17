/*!
 * Local filesystem backend
 */

use super::{FileMetadata, StorageBackend};
use crate::error::Result;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Local filesystem backend
pub struct LocalBackend;

impl LocalBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for LocalBackend {
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>> {
        let file = File::open(path)?;
        Ok(Box::new(file))
    }

    fn open_write(&self, path: &Path, append: bool) -> Result<Box<dyn Write + Send>> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(path)?;
        Ok(Box::new(file))
    }

    fn metadata(&self, path: &Path) -> Result<FileMetadata> {
        let meta = std::fs::metadata(path)?;

        Ok(FileMetadata {
            size: meta.len(),
            is_file: meta.is_file(),
            is_dir: meta.is_dir(),
            modified: meta.modified().ok(),
            permissions: None,
        })
    }

    fn exists(&self, path: &Path) -> Result<bool> {
        Ok(path.exists())
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let entries: Result<Vec<PathBuf>> = std::fs::read_dir(path)?
            .map(|entry| entry.map(|e| e.path()).map_err(|e| e.into()))
            .collect();
        entries
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        std::fs::remove_file(path)?;
        Ok(())
    }

    fn sync(&self, _path: &Path) -> Result<()> {
        Ok(())
    }

    fn protocol_name(&self) -> &'static str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_local_read_write() {
        let backend = LocalBackend::new();
        let temp = NamedTempFile::new().unwrap();

        let mut writer = backend.open_write(temp.path(), false).unwrap();
        writer.write_all(b"test data").unwrap();
        drop(writer);

        let mut reader = backend.open_read(temp.path()).unwrap();
        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();

        assert_eq!(content, "test data");
    }

    #[test]
    fn test_local_metadata() {
        use std::io::Write;
        let backend = LocalBackend::new();
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"test").unwrap();
        temp.flush().unwrap();

        let meta = backend.metadata(temp.path()).unwrap();
        assert_eq!(meta.size, 4);
        assert!(meta.is_file);
        assert!(!meta.is_dir);
    }

    #[test]
    fn test_local_exists() {
        let backend = LocalBackend::new();
        let temp = NamedTempFile::new().unwrap();

        assert!(backend.exists(temp.path()).unwrap());
        assert!(!backend.exists(Path::new("/nonexistent/file")).unwrap());
    }
}
