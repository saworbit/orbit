//! Error types for Star Map operations

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Result type for Star Map operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during Star Map operations
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error occurred
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Star Map file not found
    #[error("Star Map not found: {path}")]
    NotFound { path: PathBuf },

    /// Invalid Star Map format
    #[error("Invalid Star Map format: {reason}")]
    InvalidFormat { reason: String },

    /// Version mismatch
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u16, found: u16 },

    /// Invalid magic number (file is not a Star Map)
    #[error("Invalid magic number: expected {expected:?}, found {found:?}")]
    InvalidMagic { expected: Vec<u8>, found: Vec<u8> },

    /// Chunk index out of bounds
    #[error("Chunk index out of bounds: {index} >= {count}")]
    ChunkIndexOutOfBounds { index: u32, count: u32 },

    /// Window index out of bounds
    #[error("Window index out of bounds: {index} >= {count}")]
    WindowIndexOutOfBounds { index: u32, count: u32 },

    /// Invalid content ID size
    #[error("Invalid content ID size: expected {expected}, found {found}")]
    InvalidContentIdSize { expected: usize, found: usize },

    /// Invalid merkle root size
    #[error("Invalid merkle root size: expected {expected}, found {found}")]
    InvalidMerkleRootSize { expected: usize, found: usize },

    /// Memory mapping failed
    #[error("Memory mapping failed: {0}")]
    MemoryMapFailed(String),

    /// Bloom filter error
    #[error("Bloom filter error: {0}")]
    BloomFilter(String),

    /// Bitmap error
    #[error("Bitmap error: {0}")]
    Bitmap(String),

    /// Data corruption detected
    #[error("Data corruption detected: {reason}")]
    CorruptData { reason: String },

    /// Empty Star Map (no chunks or windows)
    #[error("Empty Star Map: must contain at least one chunk and one window")]
    Empty,

    /// Invalid window configuration
    #[error("Invalid window configuration: {0}")]
    InvalidWindow(String),

    /// Generic error with context
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create an invalid format error
    pub fn invalid_format<S: Into<String>>(reason: S) -> Self {
        Error::InvalidFormat {
            reason: reason.into(),
        }
    }

    /// Create a version mismatch error
    pub fn version_mismatch(expected: u16, found: u16) -> Self {
        Error::VersionMismatch { expected, found }
    }

    /// Create a not found error
    pub fn not_found<P: Into<PathBuf>>(path: P) -> Self {
        Error::NotFound { path: path.into() }
    }

    /// Create a chunk index out of bounds error
    pub fn chunk_index_out_of_bounds(index: u32, count: u32) -> Self {
        Error::ChunkIndexOutOfBounds { index, count }
    }

    /// Create a window index out of bounds error
    pub fn window_index_out_of_bounds(index: u32, count: u32) -> Self {
        Error::WindowIndexOutOfBounds { index, count }
    }

    /// Create an invalid content ID size error
    pub fn invalid_content_id_size(expected: usize, found: usize) -> Self {
        Error::InvalidContentIdSize { expected, found }
    }

    /// Create an invalid merkle root size error
    pub fn invalid_merkle_root_size(expected: usize, found: usize) -> Self {
        Error::InvalidMerkleRootSize { expected, found }
    }

    /// Create a memory mapping error
    pub fn memory_map_failed<S: Into<String>>(message: S) -> Self {
        Error::MemoryMapFailed(message.into())
    }

    /// Create a bloom filter error
    pub fn bloom_filter<S: Into<String>>(message: S) -> Self {
        Error::BloomFilter(message.into())
    }

    /// Create a bitmap error
    pub fn bitmap<S: Into<String>>(message: S) -> Self {
        Error::Bitmap(message.into())
    }

    /// Create a corrupt data error
    pub fn corrupt_data<S: Into<String>>(reason: S) -> Self {
        Error::CorruptData {
            reason: reason.into(),
        }
    }

    /// Create an invalid window error
    pub fn invalid_window<S: Into<String>>(message: S) -> Self {
        Error::InvalidWindow(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_format_error() {
        let err = Error::invalid_format("bad magic number");
        assert!(matches!(err, Error::InvalidFormat { .. }));
        assert_eq!(err.to_string(), "Invalid Star Map format: bad magic number");
    }

    #[test]
    fn test_version_mismatch_error() {
        let err = Error::version_mismatch(1, 2);
        assert!(matches!(err, Error::VersionMismatch { .. }));
        assert!(err.to_string().contains("expected 1"));
        assert!(err.to_string().contains("found 2"));
    }

    #[test]
    fn test_chunk_index_out_of_bounds() {
        let err = Error::chunk_index_out_of_bounds(100, 50);
        assert!(matches!(err, Error::ChunkIndexOutOfBounds { .. }));
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn test_window_index_out_of_bounds() {
        let err = Error::window_index_out_of_bounds(10, 5);
        assert!(matches!(err, Error::WindowIndexOutOfBounds { .. }));
        assert!(err.to_string().contains("10"));
        assert!(err.to_string().contains("5"));
    }

    #[test]
    fn test_invalid_content_id_size() {
        let err = Error::invalid_content_id_size(32, 16);
        assert!(matches!(err, Error::InvalidContentIdSize { .. }));
        assert!(err.to_string().contains("32"));
        assert!(err.to_string().contains("16"));
    }

    #[test]
    fn test_memory_map_failed() {
        let err = Error::memory_map_failed("permission denied");
        assert!(matches!(err, Error::MemoryMapFailed(_)));
        assert!(err.to_string().contains("permission denied"));
    }

    #[test]
    fn test_bloom_filter_error() {
        let err = Error::bloom_filter("invalid hash count");
        assert!(matches!(err, Error::BloomFilter(_)));
        assert!(err.to_string().contains("invalid hash count"));
    }

    #[test]
    fn test_corrupt_data_error() {
        let err = Error::corrupt_data("CRC mismatch");
        assert!(matches!(err, Error::CorruptData { .. }));
        assert!(err.to_string().contains("CRC mismatch"));
    }

    #[test]
    fn test_empty_error() {
        let err = Error::Empty;
        assert!(matches!(err, Error::Empty));
        assert!(err.to_string().contains("at least one"));
    }
}
