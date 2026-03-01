//! Star Map: Binary index for efficient chunk and window queries
//!
//! The Star Map is a memory-mappable binary index that provides fast access to chunk
//! and window metadata without parsing overhead. It uses bincode for efficient
//! binary serialization and includes bloom filters and bitmaps for efficient queries.
//!
//! # Key Features
//!
//! - **Memory-mapped access**: Zero-copy reads via mmap
//! - **Fast queries**: O(1) chunk existence via bloom filter
//! - **Resume support**: Bitmap tracking of completed chunks
//! - **Compact format**: Efficient binary encoding via bincode
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │          Star Map File              │
//! ├─────────────────────────────────────┤
//! │ Magic Number (8 bytes)              │
//! │ Version (2 bytes)                   │
//! │ Header (counts, sizes)              │
//! │ Chunk Entries (offset, len, CID)    │
//! │ Window Entries (id, merkle, etc)    │
//! │ Bloom Filter (serialized)           │
//! │ Bitmaps (per-window, serialized)    │
//! └─────────────────────────────────────┘
//!          ↓ mmap
//! ┌─────────────────────────────────────┐
//! │        StarMapReader                │
//! │  - has_chunk(cid) → O(1)            │
//! │  - next_missing(window) → O(n)      │
//! │  - get_chunk(idx) → O(1)            │
//! │  - get_window(idx) → O(1)           │
//! └─────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use orbit_core_starmap::{StarMapBuilder, StarMapReader};
//!
//! // Build a Star Map
//! let mut builder = StarMapBuilder::new(1024000);
//! builder.add_chunk(0, 4096, &[0u8; 32]);
//! builder.add_window(0, 0, 10, &[0u8; 32], 0);
//! let data = builder.build().unwrap();
//!
//! // Write to disk
//! std::fs::write("file.starmap.bin", &data).unwrap();
//!
//! // Read with memory mapping
//! let reader = StarMapReader::open("file.starmap.bin").unwrap();
//! let has_chunk = reader.has_chunk(&[0u8; 32]).unwrap();
//! ```

use serde::{Deserialize, Serialize};

pub mod bitmap;
pub mod bloom;
pub mod builder;
pub mod container; // Chunk packing into container files (.orbitpak)
pub mod error;
pub mod migrate; // V2: Migration utilities (V1 → V2)
pub mod migrate_v3; // V3: Migration utilities (V2 → V3)
pub mod reader;
pub mod universe; // V2: Global content-addressed index
pub mod universe_v3; // V3: High-cardinality scalable index (Multimap)

// Re-export main types
pub use bitmap::RankSelectBitmap;
pub use bloom::BloomFilter;
pub use builder::StarMapBuilder;
pub use error::{Error, Result};
pub use reader::StarMapReader;

// V2 types
pub use universe::{ChunkLocation, DedupStats, Location, Universe, UniverseMap};

/// Current Star Map format version
pub const STARMAP_VERSION: u16 = 1;

/// Magic number for Star Map files (used for quick format detection)
pub const STARMAP_MAGIC: &[u8; 8] = b"ORBITMAP";

/// Content ID size in bytes (BLAKE3 hash)
pub const CONTENT_ID_SIZE: usize = 32;

/// Merkle root size in bytes
pub const MERKLE_ROOT_SIZE: usize = 32;

/// Chunk metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkMeta {
    /// Byte offset in source file
    pub offset: u64,
    /// Length in bytes
    pub length: u32,
    /// Content ID (BLAKE3 hash)
    pub content_id: [u8; 32],
}

/// Window metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowMeta {
    /// Window ID (sequential)
    pub id: u32,
    /// Index of first chunk
    pub first_chunk: u32,
    /// Number of chunks in window
    pub count: u16,
    /// Merkle tree root
    pub merkle_root: [u8; 32],
    /// Overlap with previous window
    pub overlap: u16,
}

/// Internal Star Map structure for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StarMapData {
    /// Schema version
    pub version: u16,
    /// File size
    pub file_size: u64,
    /// Chunk count
    pub chunk_count: u32,
    /// Window count
    pub window_count: u32,
    /// Chunk entries
    pub chunks: Vec<ChunkMeta>,
    /// Window entries
    pub windows: Vec<WindowMeta>,
    /// Bloom filter data
    pub bloom_data: Vec<u8>,
    /// Bloom filter hash count
    pub bloom_hashes: u32,
    /// Bloom filter element count
    pub bloom_elements: u32,
    /// Bloom filter bit count
    pub bloom_bits: usize,
    /// Bitmap data (one per window)
    pub bitmaps: Vec<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(STARMAP_VERSION, 1);
        assert_eq!(STARMAP_MAGIC.len(), 8);
        assert_eq!(CONTENT_ID_SIZE, 32);
        assert_eq!(MERKLE_ROOT_SIZE, 32);
    }

    #[test]
    fn test_chunk_meta_size() {
        // Ensure ChunkMeta is reasonably sized
        let size = std::mem::size_of::<ChunkMeta>();
        // offset(8) + length(4) + content_id(32) = 44 bytes (plus alignment)
        assert!(size <= 48, "ChunkMeta size: {} bytes", size);
    }

    #[test]
    fn test_window_meta_size() {
        // Ensure WindowMeta is reasonably sized
        let size = std::mem::size_of::<WindowMeta>();
        // id(4) + first_chunk(4) + count(2) + merkle_root(32) + overlap(2) = 44 bytes (plus alignment)
        assert!(size <= 48, "WindowMeta size: {} bytes", size);
    }

    #[test]
    fn test_serde_chunk_meta() {
        let chunk = ChunkMeta {
            offset: 1024,
            length: 4096,
            content_id: [42u8; 32],
        };

        let serialized = bincode::serialize(&chunk).unwrap();
        let deserialized: ChunkMeta = bincode::deserialize(&serialized).unwrap();

        assert_eq!(chunk, deserialized);
    }

    #[test]
    fn test_serde_window_meta() {
        let window = WindowMeta {
            id: 5,
            first_chunk: 10,
            count: 64,
            merkle_root: [99u8; 32],
            overlap: 4,
        };

        let serialized = bincode::serialize(&window).unwrap();
        let deserialized: WindowMeta = bincode::deserialize(&serialized).unwrap();

        assert_eq!(window, deserialized);
    }
}
