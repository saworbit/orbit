//! Common types for backend abstraction

use std::path::PathBuf;
use std::time::SystemTime;
use bytes::Bytes;

#[cfg(feature = "backend-abstraction")]
use futures::Stream;
#[cfg(feature = "backend-abstraction")]
use std::pin::Pin;

/// File/directory metadata across all backends
#[derive(Debug, Clone)]
pub struct Metadata {
    /// Size in bytes (0 for directories)
    pub size: u64,

    /// Is this a file?
    pub is_file: bool,

    /// Is this a directory?
    pub is_dir: bool,

    /// Is this a symbolic link?
    pub is_symlink: bool,

    /// Last modification time
    pub modified: Option<SystemTime>,

    /// Creation time
    pub created: Option<SystemTime>,

    /// Last access time
    pub accessed: Option<SystemTime>,

    /// Unix permissions (None on non-Unix systems)
    pub permissions: Option<u32>,

    /// MIME type / content type (primarily for cloud storage)
    pub content_type: Option<String>,

    /// ETag or checksum (for versioning/caching)
    pub etag: Option<String>,

    /// Custom metadata key-value pairs (for cloud storage)
    pub custom_metadata: Option<std::collections::HashMap<String, String>>,
}

impl Metadata {
    /// Create metadata for a file
    pub fn file(size: u64) -> Self {
        Self {
            size,
            is_file: true,
            is_dir: false,
            is_symlink: false,
            modified: None,
            created: None,
            accessed: None,
            permissions: None,
            content_type: None,
            etag: None,
            custom_metadata: None,
        }
    }

    /// Create metadata for a directory
    pub fn directory() -> Self {
        Self {
            size: 0,
            is_file: false,
            is_dir: true,
            is_symlink: false,
            modified: None,
            created: None,
            accessed: None,
            permissions: None,
            content_type: None,
            etag: None,
            custom_metadata: None,
        }
    }

    /// Create metadata for a symlink
    pub fn symlink(target_size: u64) -> Self {
        Self {
            size: target_size,
            is_file: false,
            is_dir: false,
            is_symlink: true,
            modified: None,
            created: None,
            accessed: None,
            permissions: None,
            content_type: None,
            etag: None,
            custom_metadata: None,
        }
    }

    /// Builder pattern: set modification time
    pub fn with_modified(mut self, modified: SystemTime) -> Self {
        self.modified = Some(modified);
        self
    }

    /// Builder pattern: set permissions
    pub fn with_permissions(mut self, permissions: u32) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Builder pattern: set content type
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type = Some(content_type);
        self
    }
}

/// Directory entry from listing operations
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Path relative to the listed directory
    pub path: PathBuf,

    /// Full path (absolute or protocol-specific)
    pub full_path: PathBuf,

    /// Entry metadata
    pub metadata: Metadata,
}

impl DirEntry {
    /// Create a new directory entry
    pub fn new(path: PathBuf, full_path: PathBuf, metadata: Metadata) -> Self {
        Self {
            path,
            full_path,
            metadata,
        }
    }

    /// Get the file name
    pub fn file_name(&self) -> Option<&std::ffi::OsStr> {
        self.path.file_name()
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.metadata.is_file
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir
    }

    /// Check if this is a symlink
    pub fn is_symlink(&self) -> bool {
        self.metadata.is_symlink
    }
}

#[cfg(feature = "backend-abstraction")]
/// Async read stream for file data
pub type ReadStream = Pin<Box<dyn Stream<Item = std::io::Result<Bytes>> + Send>>;

#[cfg(feature = "backend-abstraction")]
/// Async write sink for file data
pub trait WriteStream: Send {
    /// Write bytes to the stream
    fn write(&mut self, data: Bytes) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + '_>>;

    /// Flush any buffered data
    fn flush(&mut self) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + '_>>;

    /// Close the stream
    fn close(&mut self) -> Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + '_>>;
}

/// Options for list operations
#[derive(Debug, Clone)]
pub struct ListOptions {
    /// List recursively into subdirectories
    pub recursive: bool,

    /// Maximum depth for recursive listing (None = unlimited)
    pub max_depth: Option<usize>,

    /// Include hidden files (starting with .)
    pub include_hidden: bool,

    /// Follow symbolic links
    pub follow_symlinks: bool,

    /// Maximum number of entries to return (None = unlimited)
    pub max_entries: Option<usize>,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            recursive: false,
            max_depth: None,
            include_hidden: false,
            follow_symlinks: false,
            max_entries: None,
        }
    }
}

impl ListOptions {
    /// Create options for non-recursive listing
    pub fn shallow() -> Self {
        Self::default()
    }

    /// Create options for recursive listing
    pub fn recursive() -> Self {
        Self {
            recursive: true,
            ..Default::default()
        }
    }

    /// Set maximum depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Include hidden files
    pub fn include_hidden(mut self) -> Self {
        self.include_hidden = true;
        self
    }

    /// Follow symbolic links
    pub fn follow_symlinks(mut self) -> Self {
        self.follow_symlinks = true;
        self
    }
}

/// Options for write operations
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// Create parent directories if they don't exist
    pub create_parents: bool,

    /// Overwrite existing file
    pub overwrite: bool,

    /// Content type / MIME type (for cloud storage)
    pub content_type: Option<String>,

    /// Custom metadata (for cloud storage)
    pub metadata: Option<std::collections::HashMap<String, String>>,

    /// File permissions (Unix-style, e.g., 0o644)
    pub permissions: Option<u32>,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            create_parents: true,
            overwrite: true,
            content_type: None,
            metadata: None,
            permissions: None,
        }
    }
}

impl WriteOptions {
    /// Create with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set content type
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type = Some(content_type);
        self
    }

    /// Set custom metadata
    pub fn with_metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set permissions
    pub fn with_permissions(mut self, permissions: u32) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Disable overwriting
    pub fn no_overwrite(mut self) -> Self {
        self.overwrite = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_file() {
        let meta = Metadata::file(1024);
        assert!(meta.is_file);
        assert!(!meta.is_dir);
        assert!(!meta.is_symlink);
        assert_eq!(meta.size, 1024);
    }

    #[test]
    fn test_metadata_directory() {
        let meta = Metadata::directory();
        assert!(!meta.is_file);
        assert!(meta.is_dir);
        assert_eq!(meta.size, 0);
    }

    #[test]
    fn test_metadata_builder() {
        let meta = Metadata::file(2048)
            .with_permissions(0o644)
            .with_content_type("text/plain".to_string());

        assert_eq!(meta.size, 2048);
        assert_eq!(meta.permissions, Some(0o644));
        assert_eq!(meta.content_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_list_options() {
        let opts = ListOptions::recursive()
            .with_max_depth(3)
            .include_hidden()
            .follow_symlinks();

        assert!(opts.recursive);
        assert_eq!(opts.max_depth, Some(3));
        assert!(opts.include_hidden);
        assert!(opts.follow_symlinks);
    }

    #[test]
    fn test_write_options() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("author".to_string(), "test".to_string());

        let opts = WriteOptions::new()
            .with_content_type("application/json".to_string())
            .with_metadata(metadata.clone())
            .with_permissions(0o600);

        assert_eq!(opts.content_type, Some("application/json".to_string()));
        assert_eq!(opts.metadata, Some(metadata));
        assert_eq!(opts.permissions, Some(0o600));
    }
}
