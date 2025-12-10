//! Orbit Core Interface: Universal I/O Abstraction
//!
//! This crate defines the `OrbitSystem` trait, which abstracts filesystem and compute operations
//! to enable both local (standalone) and distributed (Grid/Star) topologies.
//!
//! # Architecture
//!
//! The `OrbitSystem` trait provides three categories of operations:
//!
//! 1. **Discovery**: Check existence, get metadata, list directories
//! 2. **Data Access**: Stream-based reading and writing
//! 3. **Compute Offloading**: Optimized operations like header reads and hash calculation
//!
//! # Example
//!
//! ```rust,no_run
//! use orbit_core_interface::{OrbitSystem, FileMetadata};
//! use std::path::Path;
//!
//! async fn analyze_file<S: OrbitSystem>(system: &S, path: &Path) -> anyhow::Result<()> {
//!     // Check if file exists
//!     if !system.exists(path).await {
//!         return Ok(());
//!     }
//!
//!     // Get metadata
//!     let meta = system.metadata(path).await?;
//!     println!("File size: {} bytes", meta.len);
//!
//!     // Read first 512 bytes for analysis
//!     let header = system.read_header(path, 512).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Design Philosophy
//!
//! The trait is designed to:
//! - Support zero-cost abstractions (monomorphization)
//! - Enable efficient compute offloading to remote nodes
//! - Minimize network round-trips for common operations
//! - Provide async-first APIs for all I/O

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrbitSystemError {
    #[error("File not found: {0}")]
    NotFound(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("System error: {0}")]
    System(String),
}

pub type Result<T> = std::result::Result<T, OrbitSystemError>;

/// Metadata for a file or directory in the Orbit System
///
/// This is intentionally minimal to work across different backends
/// (local filesystem, remote systems, object storage, etc.)
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Full path to the file/directory
    pub path: PathBuf,

    /// Size in bytes (0 for directories)
    pub len: u64,

    /// Whether this is a directory
    pub is_dir: bool,

    /// Last modification time
    pub modified: SystemTime,
    // Future expansion: permissions, owner, extended attributes
}

impl FileMetadata {
    /// Create metadata for a file
    pub fn file(path: impl Into<PathBuf>, len: u64, modified: SystemTime) -> Self {
        Self {
            path: path.into(),
            len,
            is_dir: false,
            modified,
        }
    }

    /// Create metadata for a directory
    pub fn directory(path: impl Into<PathBuf>, modified: SystemTime) -> Self {
        Self {
            path: path.into(),
            len: 0,
            is_dir: true,
            modified,
        }
    }
}

/// The Universal Interface for Orbit I/O and Compute
///
/// This trait abstracts all filesystem and compute-heavy operations to enable:
/// - **LocalSystem**: Direct local filesystem access (Standalone mode)
/// - **RemoteSystem**: Network-based operations via Star nodes (Grid mode)
/// - **MockSystem**: In-memory testing without real filesystem
///
/// # Design Principles
///
/// 1. **Async-first**: All operations return `Future`s for non-blocking I/O
/// 2. **Compute offloading**: Operations like hashing can run on the data side
/// 3. **Stream-based**: Large files use async readers/writers, not buffers
/// 4. **Minimal round-trips**: Batch operations where possible
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync + 'static` to work across async boundaries.
#[async_trait]
pub trait OrbitSystem: Send + Sync + 'static {
    // ═══════════════════════════════════════════════════════════════════════
    // 1. Discovery Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Check if a path exists
    ///
    /// This is optimized to avoid fetching full metadata when only existence matters.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use orbit_core_interface::OrbitSystem;
    /// # use std::path::Path;
    /// # async fn example<S: OrbitSystem>(system: &S) {
    /// if system.exists(Path::new("/data/config.toml")).await {
    ///     println!("Config file exists");
    /// }
    /// # }
    /// ```
    async fn exists(&self, path: &Path) -> bool;

    /// Get metadata for a file or directory
    ///
    /// # Errors
    ///
    /// Returns `OrbitSystemError::NotFound` if the path doesn't exist.
    /// Returns `OrbitSystemError::PermissionDenied` if access is denied.
    async fn metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// List directory contents (non-recursive)
    ///
    /// Returns metadata for all direct children of the directory.
    /// For recursive traversal, call this method repeatedly.
    ///
    /// # Errors
    ///
    /// Returns error if path is not a directory or cannot be read.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use orbit_core_interface::OrbitSystem;
    /// # use std::path::Path;
    /// # async fn example<S: OrbitSystem>(system: &S) -> anyhow::Result<()> {
    /// let entries = system.read_dir(Path::new("/data")).await?;
    /// for entry in entries {
    ///     println!("{}: {} bytes", entry.path.display(), entry.len);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn read_dir(&self, path: &Path) -> Result<Vec<FileMetadata>>;

    // ═══════════════════════════════════════════════════════════════════════
    // 2. Data Access Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Open a file for reading (streaming)
    ///
    /// Returns an async reader that implements `AsyncRead`.
    /// For large files, this avoids loading the entire file into memory.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use orbit_core_interface::OrbitSystem;
    /// # use std::path::Path;
    /// # use tokio::io::AsyncReadExt;
    /// # async fn example<S: OrbitSystem>(system: &S) -> anyhow::Result<()> {
    /// let mut reader = system.reader(Path::new("/data/large.bin")).await?;
    /// let mut buffer = vec![0u8; 4096];
    /// let n = reader.read(&mut buffer).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn reader(&self, path: &Path) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>>;

    /// Open a file for writing
    ///
    /// Returns an async writer that implements `AsyncWrite`.
    /// Creates the file if it doesn't exist, truncates if it does.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use orbit_core_interface::OrbitSystem;
    /// # use std::path::Path;
    /// # use tokio::io::AsyncWriteExt;
    /// # async fn example<S: OrbitSystem>(system: &S) -> anyhow::Result<()> {
    /// let mut writer = system.writer(Path::new("/data/output.bin")).await?;
    /// writer.write_all(b"Hello, Orbit!").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn writer(&self, path: &Path) -> Result<Box<dyn tokio::io::AsyncWrite + Unpin + Send>>;

    // ═══════════════════════════════════════════════════════════════════════
    // 3. Compute Offloading Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Read the first N bytes of a file (optimized for semantic analysis)
    ///
    /// This is a critical optimization for the "Star" topology:
    /// - **LocalSystem**: Simple file read
    /// - **RemoteSystem**: Star node reads locally and sends just the header
    ///
    /// This avoids streaming entire files over the network when only metadata
    /// or magic numbers are needed (e.g., determining file type).
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `len` - Maximum number of bytes to read (typically 512-4096)
    ///
    /// # Returns
    ///
    /// A vector containing the read bytes. May be shorter than `len` if the
    /// file is smaller.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use orbit_core_interface::OrbitSystem;
    /// # use std::path::Path;
    /// # async fn example<S: OrbitSystem>(system: &S) -> anyhow::Result<()> {
    /// // Read first 512 bytes for magic number detection
    /// let header = system.read_header(Path::new("/data/file.bin"), 512).await?;
    ///
    /// // Check for PNG magic number
    /// if header.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
    ///     println!("This is a PNG file");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn read_header(&self, path: &Path, len: usize) -> Result<Vec<u8>>;

    /// Calculate hash of a specific file range (optimized for CDC)
    ///
    /// This is THE critical path for distributed performance:
    /// - **LocalSystem**: Hash using local CPU
    /// - **RemoteSystem**: Send command to Star, which hashes locally
    ///
    /// This is essential for Content-Defined Chunking (CDC) in the Grid topology.
    /// Instead of transferring chunks to compute their hash, we compute the hash
    /// on the data side and only transfer the hash (32 bytes vs potentially MB).
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `offset` - Byte offset to start hashing from
    /// * `len` - Number of bytes to hash
    ///
    /// # Returns
    ///
    /// A 32-byte BLAKE3 hash of the specified range.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use orbit_core_interface::OrbitSystem;
    /// # use std::path::Path;
    /// # async fn example<S: OrbitSystem>(system: &S) -> anyhow::Result<()> {
    /// // Hash first 1MB chunk
    /// let hash1 = system.calculate_hash(Path::new("/data/large.bin"), 0, 1024*1024).await?;
    ///
    /// // Hash second 1MB chunk
    /// let hash2 = system.calculate_hash(Path::new("/data/large.bin"), 1024*1024, 1024*1024).await?;
    ///
    /// // Compare hashes to detect if chunks differ
    /// if hash1 == hash2 {
    ///     println!("First two chunks are identical");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn calculate_hash(&self, path: &Path, offset: u64, len: u64) -> Result<[u8; 32]>;
}

/// Helper trait for implementing additional convenience methods
///
/// This is a separate trait to avoid requiring all implementations to
/// implement convenience methods that can be derived from the core trait.
#[async_trait]
pub trait OrbitSystemExt: OrbitSystem {
    /// Read entire file contents into memory
    ///
    /// **Warning**: This loads the entire file into RAM. Use with caution
    /// for large files. Prefer `reader()` for streaming.
    async fn read_all(&self, path: &Path) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;
        let mut reader = self.reader(path).await?;
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }

    /// Write entire buffer to a file
    ///
    /// Creates or truncates the file and writes all data.
    async fn write_all(&self, path: &Path, data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        let mut writer = self.writer(path).await?;
        writer.write_all(data).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Calculate hash of entire file
    ///
    /// Convenience method that hashes from offset 0 to file size.
    async fn calculate_file_hash(&self, path: &Path) -> Result<[u8; 32]> {
        let meta = self.metadata(path).await?;
        self.calculate_hash(path, 0, meta.len).await
    }
}

// Blanket implementation for all OrbitSystem implementations
impl<T: OrbitSystem + ?Sized> OrbitSystemExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    // These are trait definition tests - actual implementations tested in their own crates

    #[test]
    fn test_file_metadata_constructors() {
        let file = FileMetadata::file("/data/test.txt", 1024, SystemTime::now());
        assert!(!file.is_dir);
        assert_eq!(file.len, 1024);

        let dir = FileMetadata::directory("/data", SystemTime::now());
        assert!(dir.is_dir);
        assert_eq!(dir.len, 0);
    }
}
