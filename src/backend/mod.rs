//! Unified backend abstraction for diverse data sources and destinations
//!
//! This module provides a modular abstraction layer for handling local filesystems,
//! remote protocols (SSH), and cloud storage providers (S3, etc.). It enables seamless
//! integration across different storage backends with a consistent async interface.
//!
//! # Features
//!
//! - **Async-first design**: All operations use `async/await` with Tokio runtime
//! - **Trait-based abstraction**: `Backend` trait for uniform access patterns
//! - **Multiple implementations**: Local filesystem, SSH, S3, and more
//! - **Streaming I/O**: Efficient streaming for large files
//! - **Comprehensive error handling**: Rich error types with context
//! - **Extensibility**: Plugin support for custom backends
//! - **Security**: Secure credential handling with `secrecy` crate
//!
//! # Examples
//!
//! ## Local Filesystem
//!
//! ```no_run
//! use orbit::backend::{Backend, LocalBackend};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let backend = LocalBackend::new();
//!     let metadata = backend.stat(Path::new("/tmp/file.txt")).await?;
//!     println!("File size: {} bytes", metadata.size);
//!     Ok(())
//! }
//! ```
//!
//! ## SSH Backend
//!
//! ```no_run
//! # #[cfg(feature = "ssh-backend")]
//! # {
//! use orbit::backend::{Backend, SshBackend, SshConfig};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SshConfig::new("user@example.com:22")
//!         .with_key_file("/home/user/.ssh/id_rsa");
//!     let backend = SshBackend::connect(config).await?;
//!
//!     let entries = backend.list(Path::new("/remote/dir"), Default::default()).await?;
//!     for entry in entries {
//!         println!("{}: {} bytes", entry.path.display(), entry.metadata.size);
//!     }
//!     Ok(())
//! }
//! # }
//! ```

pub mod error;
pub mod types;

#[cfg(feature = "backend-abstraction")]
mod local;

#[cfg(all(feature = "backend-abstraction", feature = "ssh-backend"))]
mod ssh;

#[cfg(all(feature = "backend-abstraction", feature = "s3-native"))]
mod s3;

#[cfg(feature = "backend-abstraction")]
mod config;

#[cfg(feature = "backend-abstraction")]
mod registry;

// Re-export main types
pub use error::{BackendError, BackendResult};
pub use types::{DirEntry, ListOptions, Metadata, WriteOptions};

#[cfg(feature = "backend-abstraction")]
pub use local::LocalBackend;

#[cfg(all(feature = "backend-abstraction", feature = "ssh-backend"))]
pub use ssh::{SshAuth, SshBackend, SshConfig};

#[cfg(all(feature = "backend-abstraction", feature = "s3-native"))]
pub use s3::S3Backend;

#[cfg(feature = "backend-abstraction")]
pub use config::{parse_uri, BackendConfig};

#[cfg(feature = "backend-abstraction")]
pub use registry::{BackendFactory, BackendRegistry};

#[cfg(feature = "backend-abstraction")]
use async_trait::async_trait;

#[cfg(feature = "backend-abstraction")]
use std::path::Path;

#[cfg(feature = "backend-abstraction")]
use types::ReadStream;

/// Unified backend trait for all storage operations
///
/// This trait provides a common interface for interacting with different storage
/// backends including local filesystems, SSH/SFTP, and cloud storage (S3, etc.).
///
/// All operations are async and return `BackendResult<T>` for consistent error handling.
///
/// # Thread Safety
///
/// Implementors must be `Send + Sync` to support concurrent access in multi-threaded
/// environments.
///
/// # Example Implementation
///
/// ```ignore
/// use orbit::backend::{Backend, BackendResult, Metadata, DirEntry};
/// use async_trait::async_trait;
/// use std::path::Path;
///
/// struct MyBackend;
///
/// #[async_trait]
/// impl Backend for MyBackend {
///     async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
///         // Implementation
///     }
///     // ... other methods
/// }
/// ```
#[cfg(feature = "backend-abstraction")]
#[async_trait]
pub trait Backend: Send + Sync {
    /// Get metadata for a file or directory
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file or directory
    ///
    /// # Returns
    ///
    /// Metadata including size, timestamps, permissions, etc.
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if the path doesn't exist.
    /// Returns `BackendError::PermissionDenied` if access is denied.
    async fn stat(&self, path: &Path) -> BackendResult<Metadata>;

    /// List contents of a directory
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path to list
    /// * `options` - Listing options (recursive, max_depth, etc.)
    ///
    /// # Returns
    ///
    /// Vector of directory entries with metadata
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if the directory doesn't exist.
    /// Returns `BackendError::InvalidPath` if path is not a directory.
    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<Vec<DirEntry>>;

    /// Open a file for reading as a stream
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to read
    ///
    /// # Returns
    ///
    /// Async stream of bytes
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if the file doesn't exist.
    /// Returns `BackendError::PermissionDenied` if read access is denied.
    async fn read(&self, path: &Path) -> BackendResult<ReadStream>;

    /// Write data to a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path where to write the file
    /// * `data` - Data to write
    /// * `options` - Write options (overwrite, permissions, etc.)
    ///
    /// # Returns
    ///
    /// Number of bytes written
    ///
    /// # Errors
    ///
    /// Returns `BackendError::PermissionDenied` if write access is denied.
    /// Returns `BackendError::AlreadyExists` if file exists and overwrite is false.
    async fn write(
        &self,
        path: &Path,
        data: bytes::Bytes,
        options: WriteOptions,
    ) -> BackendResult<u64>;

    /// Delete a file or directory
    ///
    /// # Arguments
    ///
    /// * `path` - Path to delete
    /// * `recursive` - If true, delete directories and their contents recursively
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if the path doesn't exist.
    /// Returns `BackendError::DirectoryNotEmpty` if trying to delete non-empty dir without recursive.
    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()>;

    /// Create a directory
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path to create
    /// * `recursive` - If true, create parent directories as needed
    ///
    /// # Errors
    ///
    /// Returns `BackendError::AlreadyExists` if directory already exists.
    /// Returns `BackendError::NotFound` if parent doesn't exist and recursive is false.
    async fn mkdir(&self, path: &Path, recursive: bool) -> BackendResult<()>;

    /// Rename or move a file/directory
    ///
    /// # Arguments
    ///
    /// * `src` - Source path
    /// * `dest` - Destination path
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if source doesn't exist.
    /// Returns `BackendError::AlreadyExists` if destination already exists.
    /// Returns `BackendError::Unsupported` if cross-backend rename is attempted.
    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()>;

    /// Set file permissions (Unix mode bits)
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `mode` - Unix permission bits (e.g., 0o755)
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if the path doesn't exist.
    /// Returns `BackendError::Unsupported` if the backend doesn't support permissions.
    /// Returns `BackendError::PermissionDenied` if access is denied.
    async fn set_permissions(&self, path: &Path, mode: u32) -> BackendResult<()> {
        let _ = (path, mode);
        Err(BackendError::Unsupported {
            backend: self.backend_name().to_string(),
            operation: "set_permissions".to_string(),
        })
    }

    /// Set file timestamps (access and modification times)
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `atime` - Access time (None to keep current)
    /// * `mtime` - Modification time (None to keep current)
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if the path doesn't exist.
    /// Returns `BackendError::Unsupported` if the backend doesn't support timestamps.
    async fn set_timestamps(
        &self,
        path: &Path,
        atime: Option<std::time::SystemTime>,
        mtime: Option<std::time::SystemTime>,
    ) -> BackendResult<()> {
        let _ = (path, atime, mtime);
        Err(BackendError::Unsupported {
            backend: self.backend_name().to_string(),
            operation: "set_timestamps".to_string(),
        })
    }

    /// Get extended attributes (xattrs) for a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    ///
    /// # Returns
    ///
    /// HashMap of attribute names to values
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if the path doesn't exist.
    /// Returns `BackendError::Unsupported` if the backend doesn't support xattrs.
    async fn get_xattrs(
        &self,
        path: &Path,
    ) -> BackendResult<std::collections::HashMap<String, Vec<u8>>> {
        let _ = path;
        Err(BackendError::Unsupported {
            backend: self.backend_name().to_string(),
            operation: "get_xattrs".to_string(),
        })
    }

    /// Set extended attributes (xattrs) for a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `attrs` - HashMap of attribute names to values
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if the path doesn't exist.
    /// Returns `BackendError::Unsupported` if the backend doesn't support xattrs.
    async fn set_xattrs(
        &self,
        path: &Path,
        attrs: &std::collections::HashMap<String, Vec<u8>>,
    ) -> BackendResult<()> {
        let _ = (path, attrs);
        Err(BackendError::Unsupported {
            backend: self.backend_name().to_string(),
            operation: "set_xattrs".to_string(),
        })
    }

    /// Set owner and group (Unix UID/GID)
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `uid` - User ID (None to keep current)
    /// * `gid` - Group ID (None to keep current)
    ///
    /// # Errors
    ///
    /// Returns `BackendError::NotFound` if the path doesn't exist.
    /// Returns `BackendError::Unsupported` if the backend doesn't support ownership.
    /// Returns `BackendError::PermissionDenied` if access is denied (requires privileges).
    async fn set_ownership(
        &self,
        path: &Path,
        uid: Option<u32>,
        gid: Option<u32>,
    ) -> BackendResult<()> {
        let _ = (path, uid, gid);
        Err(BackendError::Unsupported {
            backend: self.backend_name().to_string(),
            operation: "set_ownership".to_string(),
        })
    }

    /// Check if a path exists
    ///
    /// # Arguments
    ///
    /// * `path` - Path to check
    ///
    /// # Returns
    ///
    /// `true` if the path exists, `false` otherwise
    async fn exists(&self, path: &Path) -> BackendResult<bool> {
        match self.stat(path).await {
            Ok(_) => Ok(true),
            Err(BackendError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get the backend name/type
    ///
    /// # Returns
    ///
    /// String identifier for this backend (e.g., "local", "ssh", "s3")
    fn backend_name(&self) -> &str;

    /// Check if this backend supports the given operation
    ///
    /// # Arguments
    ///
    /// * `operation` - Operation name (e.g., "rename", "symlink", "set_permissions")
    ///
    /// # Returns
    ///
    /// `true` if the operation is supported
    fn supports(&self, operation: &str) -> bool {
        // Default implementations support all core operations
        // Metadata operations have default implementations that return Unsupported
        matches!(
            operation,
            "stat"
                | "list"
                | "read"
                | "write"
                | "delete"
                | "mkdir"
                | "rename"
                | "exists"
                | "set_permissions"
                | "set_timestamps"
                | "get_xattrs"
                | "set_xattrs"
                | "set_ownership"
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // Basic test to ensure module compiles
        assert!(true);
    }
}
