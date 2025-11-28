//! Universe Map: Global Content-Addressed Index for Orbit V2
//!
//! The Universe Map extends Star Map V1 with global deduplication.
//! Instead of indexing chunks per-file, it creates a repository-wide index
//! where identical content chunks (by hash) are stored only once.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────┐
//! │         Universe Map V2                │
//! ├────────────────────────────────────────┤
//! │ Magic: "UNIVERSE" (8 bytes)            │
//! │ Version: 2                             │
//! │ Chunk Count: u64                       │
//! │ Location Count: u64                    │
//! ├────────────────────────────────────────┤
//! │ Content Index:                         │
//! │   [Hash32] -> Vec<Location>            │
//! │                                        │
//! │ Location {                             │
//! │   file_id: u64,                        │
//! │   offset: u64,                         │
//! │   length: u32,                         │
//! │ }                                      │
//! └────────────────────────────────────────┘
//! ```
//!
//! # Benefits
//!
//! - **Global Dedup**: Identical chunks across different files share one entry
//! - **Rename Detection**: Moving/renaming files transfers zero bytes
//! - **Cross-Project Dedup**: Chunks can be deduplicated across entire repository
//!
//! # Example
//!
//! ```
//! use orbit_core_starmap::universe::{UniverseMap, Location};
//!
//! let mut universe = UniverseMap::new();
//!
//! // Add chunk from file 1
//! let hash1 = [0x42; 32];
//! universe.add_chunk(&hash1, Location::new(1, 0, 4096));
//!
//! // Add same chunk from file 2 (different location)
//! universe.add_chunk(&hash1, Location::new(2, 1024, 4096));
//!
//! // Query: where does this chunk exist?
//! let locations = universe.get_locations(&hash1).unwrap();
//! assert_eq!(locations.len(), 2); // Exists in 2 files!
//! ```

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Universe Map magic number
pub const UNIVERSE_MAGIC: &[u8; 8] = b"UNIVERSE";

/// Universe Map version
pub const UNIVERSE_VERSION: u16 = 2;

/// A location where a chunk exists
///
/// Multiple locations can reference the same content_id (global dedup).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// File identifier (assigned during indexing)
    pub file_id: u64,

    /// Byte offset within the file
    pub offset: u64,

    /// Length in bytes
    pub length: u32,
}

impl Location {
    /// Create a new location
    pub fn new(file_id: u64, offset: u64, length: u32) -> Self {
        Self {
            file_id,
            offset,
            length,
        }
    }
}

/// Global content-addressed index
///
/// Maps content hashes (BLAKE3) to all locations where that content exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseMap {
    /// Version number
    version: u16,

    /// Index: Content ID -> Locations
    index: HashMap<[u8; 32], Vec<Location>>,

    /// File ID -> File path mapping (for human readability)
    file_registry: HashMap<u64, String>,

    /// Next file ID to assign
    next_file_id: u64,
}

impl UniverseMap {
    /// Create a new empty Universe Map
    pub fn new() -> Self {
        Self {
            version: UNIVERSE_VERSION,
            index: HashMap::new(),
            file_registry: HashMap::new(),
            next_file_id: 1,
        }
    }

    /// Register a file and get its ID
    ///
    /// If the file is already registered, returns its existing ID.
    pub fn register_file(&mut self, path: impl AsRef<str>) -> u64 {
        let path_str = path.as_ref().to_string();

        // Check if already registered
        for (id, registered_path) in &self.file_registry {
            if registered_path == &path_str {
                return *id;
            }
        }

        // Assign new ID
        let id = self.next_file_id;
        self.file_registry.insert(id, path_str);
        self.next_file_id += 1;
        id
    }

    /// Add a chunk to the index
    ///
    /// If the chunk already exists at this location, it's a no-op.
    pub fn add_chunk(&mut self, content_id: &[u8; 32], location: Location) {
        self.index.entry(*content_id).or_default().push(location);
    }

    /// Get all locations for a content ID
    pub fn get_locations(&self, content_id: &[u8; 32]) -> Option<&Vec<Location>> {
        self.index.get(content_id)
    }

    /// Check if a chunk exists anywhere
    pub fn has_chunk(&self, content_id: &[u8; 32]) -> bool {
        self.index.contains_key(content_id)
    }

    /// Get total number of unique chunks
    pub fn chunk_count(&self) -> usize {
        self.index.len()
    }

    /// Get total number of locations (including duplicates)
    pub fn location_count(&self) -> usize {
        self.index.values().map(|v| v.len()).sum()
    }

    /// Get file path by ID
    pub fn get_file_path(&self, file_id: u64) -> Option<&str> {
        self.file_registry.get(&file_id).map(|s| s.as_str())
    }

    /// Calculate deduplication statistics
    pub fn dedup_stats(&self) -> DedupStats {
        let mut total_refs = 0;
        let mut deduped_chunks = 0;
        let mut max_refs = 0;

        for locations in self.index.values() {
            let count = locations.len();
            total_refs += count;
            if count > 1 {
                deduped_chunks += 1;
            }
            max_refs = max_refs.max(count);
        }

        DedupStats {
            unique_chunks: self.chunk_count(),
            total_references: total_refs,
            deduplicated_chunks: deduped_chunks,
            max_references: max_refs,
        }
    }

    /// Save to file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut file = File::create(path)?;

        // Write magic and version
        file.write_all(UNIVERSE_MAGIC)?;
        file.write_all(&self.version.to_le_bytes())?;

        // Serialize the entire structure using bincode
        let encoded =
            bincode::serialize(self).map_err(|e| Error::SerializationError(e.to_string()))?;

        file.write_all(&encoded)?;
        file.flush()?;

        Ok(())
    }

    /// Load from file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;

        // Read and verify magic
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)?;
        if &magic != UNIVERSE_MAGIC {
            return Err(Error::InvalidMagic {
                expected: UNIVERSE_MAGIC.to_vec(),
                found: magic.to_vec(),
            });
        }

        // Read version
        let mut version_bytes = [0u8; 2];
        file.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);

        if version != UNIVERSE_VERSION {
            return Err(Error::InvalidVersion {
                expected: UNIVERSE_VERSION,
                found: version,
            });
        }

        // Read the rest and deserialize
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let universe: UniverseMap =
            bincode::deserialize(&data).map_err(|e| Error::DeserializationError(e.to_string()))?;

        Ok(universe)
    }
}

impl Default for UniverseMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Deduplication statistics
#[derive(Debug, Clone)]
pub struct DedupStats {
    /// Number of unique content hashes
    pub unique_chunks: usize,

    /// Total number of location references
    pub total_references: usize,

    /// Number of chunks that appear in multiple locations
    pub deduplicated_chunks: usize,

    /// Maximum number of references to a single chunk
    pub max_references: usize,
}

impl DedupStats {
    /// Calculate deduplication ratio
    ///
    /// Returns 0.0 if no deduplication, 1.0 if everything is duplicated.
    pub fn dedup_ratio(&self) -> f64 {
        if self.total_references == 0 {
            return 0.0;
        }

        let duplicated_refs = self.total_references - self.unique_chunks;
        duplicated_refs as f64 / self.total_references as f64
    }

    /// Calculate space savings
    ///
    /// Percentage of redundant storage eliminated.
    pub fn space_savings_pct(&self) -> f64 {
        self.dedup_ratio() * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_universe_map_basic() {
        let mut universe = UniverseMap::new();

        let hash = [0x42; 32];
        let loc = Location::new(1, 0, 4096);

        universe.add_chunk(&hash, loc.clone());

        assert!(universe.has_chunk(&hash));
        assert_eq!(universe.chunk_count(), 1);
        assert_eq!(universe.location_count(), 1);

        let locations = universe.get_locations(&hash).unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0], loc);
    }

    #[test]
    fn test_global_dedup() {
        let mut universe = UniverseMap::new();

        let hash = [0xAB; 32];

        // Same chunk in different files
        universe.add_chunk(&hash, Location::new(1, 0, 4096));
        universe.add_chunk(&hash, Location::new(2, 1024, 4096));
        universe.add_chunk(&hash, Location::new(3, 2048, 4096));

        assert_eq!(universe.chunk_count(), 1); // Only 1 unique chunk
        assert_eq!(universe.location_count(), 3); // But 3 locations

        let locations = universe.get_locations(&hash).unwrap();
        assert_eq!(locations.len(), 3);
    }

    #[test]
    fn test_file_registry() {
        let mut universe = UniverseMap::new();

        let id1 = universe.register_file("/path/to/file1.txt");
        let id2 = universe.register_file("/path/to/file2.txt");
        let id1_again = universe.register_file("/path/to/file1.txt");

        assert_eq!(id1, id1_again); // Same file gets same ID
        assert_ne!(id1, id2); // Different files get different IDs

        assert_eq!(universe.get_file_path(id1), Some("/path/to/file1.txt"));
        assert_eq!(universe.get_file_path(id2), Some("/path/to/file2.txt"));
    }

    #[test]
    fn test_dedup_stats() {
        let mut universe = UniverseMap::new();

        // Unique chunk 1
        universe.add_chunk(&[0x01; 32], Location::new(1, 0, 4096));

        // Chunk 2 appears in 3 places (deduped)
        universe.add_chunk(&[0x02; 32], Location::new(1, 4096, 4096));
        universe.add_chunk(&[0x02; 32], Location::new(2, 0, 4096));
        universe.add_chunk(&[0x02; 32], Location::new(3, 0, 4096));

        // Unique chunk 3
        universe.add_chunk(&[0x03; 32], Location::new(1, 8192, 4096));

        let stats = universe.dedup_stats();
        assert_eq!(stats.unique_chunks, 3);
        assert_eq!(stats.total_references, 5);
        assert_eq!(stats.deduplicated_chunks, 1); // Only chunk 0x02 is deduped
        assert_eq!(stats.max_references, 3);

        // Dedup ratio: (5 - 3) / 5 = 0.4 = 40%
        assert!((stats.space_savings_pct() - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_save_and_load() {
        let mut universe = UniverseMap::new();

        let id1 = universe.register_file("file1.txt");
        let id2 = universe.register_file("file2.txt");

        universe.add_chunk(&[0xAA; 32], Location::new(id1, 0, 4096));
        universe.add_chunk(&[0xBB; 32], Location::new(id2, 1024, 8192));

        // Save to temp file
        let temp_file = NamedTempFile::new().unwrap();
        universe.save(temp_file.path()).unwrap();

        // Load it back
        let loaded = UniverseMap::load(temp_file.path()).unwrap();

        assert_eq!(loaded.chunk_count(), universe.chunk_count());
        assert_eq!(loaded.location_count(), universe.location_count());
        assert_eq!(loaded.get_file_path(id1), Some("file1.txt"));
        assert_eq!(loaded.get_file_path(id2), Some("file2.txt"));

        assert!(loaded.has_chunk(&[0xAA; 32]));
        assert!(loaded.has_chunk(&[0xBB; 32]));
    }

    #[test]
    fn test_empty_universe() {
        let universe = UniverseMap::new();

        assert_eq!(universe.chunk_count(), 0);
        assert_eq!(universe.location_count(), 0);
        assert!(!universe.has_chunk(&[0x00; 32]));

        let stats = universe.dedup_stats();
        assert_eq!(stats.unique_chunks, 0);
        assert_eq!(stats.space_savings_pct(), 0.0);
    }
}
