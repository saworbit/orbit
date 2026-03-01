//! Universe V3: High-Cardinality Deduplication Index
//!
//! This module implements the "Multimap" architecture for the Universe index.
//! It solves the O(N²) write amplification issue of V2 by storing chunk locations
//! as discrete entries in a B-Tree, allowing O(log N) appends and O(1) memory reads.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────┐
//! │      Universe V3 (Multimap)            │
//! ├────────────────────────────────────────┤
//! │ Key: [u8; 32] (BLAKE3 hash)            │
//! │ Values: Multiple entries per key       │
//! │   Entry 1: bincode(ChunkLocation)      │
//! │   Entry 2: bincode(ChunkLocation)      │
//! │   Entry N: bincode(ChunkLocation)      │
//! └────────────────────────────────────────┘
//! ```
//!
//! # Key Improvements Over V2
//!
//! - **O(log N) Inserts:** No need to read/deserialize/reserialize entire list
//! - **Streaming Reads:** Iterator-based access prevents memory exhaustion
//! - **Scalability:** Handles millions of duplicates per chunk efficiently
//!
//! # Example
//!
//! ```no_run
//! use orbit_core_starmap::universe_v3::{Universe, ChunkLocation};
//! use std::path::PathBuf;
//!
//! let universe = Universe::open("universe_v3.db").unwrap();
//!
//! // Insert a chunk location (O(log N) regardless of duplicate count)
//! let hash = [0x42; 32];
//! let loc = ChunkLocation::new("star-1".to_string(), PathBuf::from("/data/file.bin"), 0, 4096);
//! universe.insert_chunk(hash, loc).unwrap();
//!
//! // Check existence (O(1))
//! assert!(universe.has_chunk(&hash).unwrap());
//!
//! // Stream process all locations (O(1) memory)
//! universe.scan_chunk(&hash, |location| {
//!     println!("Found at: {:?}", location.path);
//!     true // Continue iteration
//! }).unwrap();
//! ```

use crate::error::{Error, Result};
use redb::{Database, MultimapTableDefinition, ReadableTableMetadata};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Universe V3 version number
pub const UNIVERSE_V3_VERSION: u16 = 3;

/// Table definition for chunk locations (Multimap)
/// Key: [u8; 32] (BLAKE3 hash)
/// Value: Vec<u8> (bincode-serialized single ChunkLocation)
const CHUNKS_TABLE_V3: MultimapTableDefinition<&[u8; 32], &[u8]> =
    MultimapTableDefinition::new("chunks_v3");

/// A location where a chunk exists in the Orbit Grid
///
/// This is serialized individually for each entry in the multimap,
/// avoiding the O(N²) write amplification of the V2 approach.
///
/// # Phase 5 Enhancement
///
/// The `star_id` field enables the Sentinel to identify which Star
/// owns each chunk for resilience monitoring and healing operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkLocation {
    /// The ID of the Star holding this chunk
    ///
    /// For local files on the Nucleus, use "local" or the Nucleus UUID.
    /// For remote Stars, this is the Star's unique identifier.
    pub star_id: String,

    /// Full path to the file on that Star
    pub path: PathBuf,

    /// Byte offset within the file
    pub offset: u64,

    /// Length in bytes
    pub length: u32,
}

impl ChunkLocation {
    /// Create a new chunk location
    ///
    /// # Arguments
    ///
    /// * `star_id` - Unique identifier of the Star holding this chunk
    /// * `path` - Full path to the file on that Star
    /// * `offset` - Byte offset within the file
    /// * `length` - Length in bytes
    pub fn new(star_id: String, path: PathBuf, offset: u64, length: u32) -> Self {
        Self {
            star_id,
            path,
            offset,
            length,
        }
    }
}

/// Persistent Universe: ACID-compliant global deduplication index
///
/// Uses redb's MultimapTable for efficient handling of high-cardinality keys
/// (chunks with millions of duplicate references).
pub struct Universe {
    db: Database,
}

impl Universe {
    /// Open or create a Universe database at the given path
    ///
    /// This initializes the V3 multimap table structure.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Database::create(path.as_ref())
            .map_err(|e| Error::Other(format!("Failed to open Universe DB: {}", e)))?;

        // Initialize multimap table
        let write_txn = db
            .begin_write()
            .map_err(|e| Error::Other(format!("Failed to begin transaction: {}", e)))?;
        {
            let _ = write_txn
                .open_multimap_table(CHUNKS_TABLE_V3)
                .map_err(|e| Error::Other(format!("Failed to open table: {}", e)))?;
        }
        write_txn
            .commit()
            .map_err(|e| Error::Other(format!("Failed to commit: {}", e)))?;

        Ok(Self { db })
    }

    /// Insert a chunk location into the database
    ///
    /// This operation is **O(log N)** regardless of how many duplicates exist.
    /// This is the key improvement over V2's O(N) approach.
    ///
    /// # Arguments
    ///
    /// * `hash` - The BLAKE3 hash of the chunk content
    /// * `location` - Where this chunk is located
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit_core_starmap::universe_v3::{Universe, ChunkLocation};
    /// # use std::path::PathBuf;
    /// let universe = Universe::open("db.redb").unwrap();
    /// let hash = [0xAA; 32];
    /// let loc = ChunkLocation::new("star-1".to_string(), PathBuf::from("file.bin"), 0, 4096);
    /// universe.insert_chunk(hash, loc).unwrap();
    /// ```
    pub fn insert_chunk(&self, hash: [u8; 32], location: ChunkLocation) -> Result<()> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| Error::Other(format!("Failed to begin write: {}", e)))?;

        {
            let mut table = write_txn
                .open_multimap_table(CHUNKS_TABLE_V3)
                .map_err(|e| Error::Other(format!("Failed to open table: {}", e)))?;

            // Serialize just this single location (not the entire list!)
            let serialized = bincode::serialize(&location)
                .map_err(|e| Error::SerializationError(e.to_string()))?;

            // Insert as a new discrete entry for this key
            // This is O(log N) B-Tree insertion, not O(N) blob rewrite
            table
                .insert(&hash, serialized.as_slice())
                .map_err(|e| Error::Other(format!("Failed to insert: {}", e)))?;
        }

        write_txn
            .commit()
            .map_err(|e| Error::Other(format!("Failed to commit: {}", e)))?;

        Ok(())
    }

    /// Check if a chunk exists
    ///
    /// Returns true if at least one location exists for this hash.
    pub fn has_chunk(&self, hash: &[u8; 32]) -> Result<bool> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| Error::Other(format!("Failed to begin read: {}", e)))?;

        let table = read_txn
            .open_multimap_table(CHUNKS_TABLE_V3)
            .map_err(|e| Error::Other(format!("Failed to open table: {}", e)))?;

        // Access the iterator and check if it has any items
        let mut iter = table
            .get(hash)
            .map_err(|e| Error::Other(format!("Failed to get chunk: {}", e)))?;

        Ok(iter.next().is_some())
    }

    /// Find all locations for a chunk, returning a lazy iterator
    ///
    /// This solves the memory exhaustion risk by not loading all locations at once.
    ///
    /// **Note:** The current implementation collects into a Vec for API simplicity,
    /// but the interface allows future optimization to true zero-copy iteration.
    /// For maximum memory efficiency with very large result sets, use `scan_chunk` instead.
    pub fn find_chunk(&self, hash: [u8; 32]) -> Result<LocationIter> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| Error::Other(format!("Failed to begin read: {}", e)))?;

        let table = read_txn
            .open_multimap_table(CHUNKS_TABLE_V3)
            .map_err(|e| Error::Other(format!("Failed to open table: {}", e)))?;

        let mut locations = Vec::new();
        let iter = table
            .get(&hash)
            .map_err(|e| Error::Other(format!("Failed to get chunk: {}", e)))?;

        for item in iter {
            let item = item.map_err(|e| Error::Other(format!("DB error: {}", e)))?;
            let loc: ChunkLocation = bincode::deserialize(item.value())
                .map_err(|e| Error::DeserializationError(e.to_string()))?;
            locations.push(loc);
        }

        Ok(LocationIter { locations })
    }

    /// Stream processing version of find_chunk
    ///
    /// Use this for massive reads to avoid RAM spikes. The callback receives
    /// each location one at a time.
    ///
    /// # Arguments
    ///
    /// * `hash` - The chunk hash to look up
    /// * `callback` - Called for each location. Return `false` to stop iteration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit_core_starmap::universe_v3::Universe;
    /// let universe = Universe::open("db.redb").unwrap();
    /// let hash = [0xBB; 32];
    ///
    /// // Find first local location and stop
    /// let mut found_local = None;
    /// universe.scan_chunk(&hash, |loc| {
    ///     if loc.path.starts_with("/local") {
    ///         found_local = Some(loc);
    ///         false // Stop iteration
    ///     } else {
    ///         true // Continue
    ///     }
    /// }).unwrap();
    /// ```
    pub fn scan_chunk<F>(&self, hash: &[u8; 32], mut callback: F) -> Result<()>
    where
        F: FnMut(ChunkLocation) -> bool, // Return false to stop
    {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| Error::Other(format!("Failed to begin read: {}", e)))?;

        let table = read_txn
            .open_multimap_table(CHUNKS_TABLE_V3)
            .map_err(|e| Error::Other(format!("Failed to open table: {}", e)))?;

        let iter = table
            .get(hash)
            .map_err(|e| Error::Other(format!("Failed to get chunk: {}", e)))?;

        for item in iter {
            let item = item.map_err(|e| Error::Other(format!("DB error: {}", e)))?;
            let loc: ChunkLocation = bincode::deserialize(item.value())
                .map_err(|e| Error::DeserializationError(e.to_string()))?;

            if !callback(loc) {
                break;
            }
        }

        Ok(())
    }

    /// Get the total number of unique chunks (distinct hashes)
    ///
    /// **Note:** This performs a full table scan and may be slow on large databases.
    pub fn chunk_count(&self) -> Result<usize> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| Error::Other(format!("Failed to begin read: {}", e)))?;

        let table = read_txn
            .open_multimap_table(CHUNKS_TABLE_V3)
            .map_err(|e| Error::Other(format!("Failed to open table: {}", e)))?;

        let count = table
            .len()
            .map_err(|e| Error::Other(format!("Failed to get length: {}", e)))?;

        Ok(count as usize)
    }

    /// Iterate over all unique chunk hashes in the database
    ///
    /// This is used by the Sentinel to scan the entire Universe for health checks.
    ///
    /// # Returns
    ///
    /// A vector of all chunk hashes. For very large databases, consider using
    /// `scan_all_chunks` with a callback to avoid memory overhead.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit_core_starmap::universe_v3::Universe;
    /// let universe = Universe::open("db.redb").unwrap();
    ///
    /// for hash in universe.iter_all_hashes().unwrap() {
    ///     println!("Chunk: {:x?}", hash);
    /// }
    /// ```
    pub fn iter_all_hashes(&self) -> Result<Vec<[u8; 32]>> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| Error::Other(format!("Failed to begin read: {}", e)))?;

        let table = read_txn
            .open_multimap_table(CHUNKS_TABLE_V3)
            .map_err(|e| Error::Other(format!("Failed to open table: {}", e)))?;

        let mut hashes = Vec::new();

        // Use range(..) to iterate over all keys in the multimap
        let range = table
            .range::<&[u8; 32]>(..)
            .map_err(|e| Error::Other(format!("Failed to create range: {}", e)))?;

        for item in range {
            let (hash_ref, _) = item.map_err(|e| Error::Other(format!("DB error: {}", e)))?;
            hashes.push(*hash_ref.value());
        }

        Ok(hashes)
    }

    /// Scan all chunks with a callback function
    ///
    /// This is the most memory-efficient way to process all chunks in the Universe.
    /// The callback receives the hash and all its locations, one chunk at a time.
    ///
    /// # Phase 5: Sentinel Usage
    ///
    /// The Sentinel uses this method to perform health sweeps without loading
    /// the entire database into memory.
    ///
    /// # Arguments
    ///
    /// * `callback` - Called for each chunk with its hash and locations.
    ///   Return `false` to stop iteration early.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit_core_starmap::universe_v3::Universe;
    /// let universe = Universe::open("db.redb").unwrap();
    ///
    /// universe.scan_all_chunks(|hash, locations| {
    ///     println!("Chunk {:x?} has {} locations", hash, locations.len());
    ///     for loc in locations {
    ///         println!("  - Star: {}, Path: {:?}", loc.star_id, loc.path);
    ///     }
    ///     true // Continue scanning
    /// }).unwrap();
    /// ```
    pub fn scan_all_chunks<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut([u8; 32], Vec<ChunkLocation>) -> bool,
    {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| Error::Other(format!("Failed to begin read: {}", e)))?;

        let table = read_txn
            .open_multimap_table(CHUNKS_TABLE_V3)
            .map_err(|e| Error::Other(format!("Failed to open table: {}", e)))?;

        // Use range(..) to iterate over all keys in the multimap
        let range = table
            .range::<&[u8; 32]>(..)
            .map_err(|e| Error::Other(format!("Failed to create range: {}", e)))?;

        for item in range {
            let (hash_ref, locations_iter) =
                item.map_err(|e| Error::Other(format!("DB error: {}", e)))?;
            let hash = *hash_ref.value();

            // Collect all locations for this hash
            let mut locations = Vec::new();
            for location_entry in locations_iter {
                let location_ref =
                    location_entry.map_err(|e| Error::Other(format!("DB error: {}", e)))?;
                let loc: ChunkLocation = bincode::deserialize(location_ref.value())
                    .map_err(|e| Error::DeserializationError(e.to_string()))?;
                locations.push(loc);
            }

            // Call the callback with this chunk's data
            if !callback(hash, locations) {
                break;
            }
        }

        Ok(())
    }
}

/// Iterator wrapper for locations
///
/// Currently wraps a Vec, but the interface allows future optimization to
/// a self-referential struct with zero-copy reads if needed.
pub struct LocationIter {
    locations: Vec<ChunkLocation>,
}

impl Iterator for LocationIter {
    type Item = ChunkLocation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.locations.is_empty() {
            None
        } else {
            Some(self.locations.remove(0))
        }
    }
}

impl LocationIter {
    /// Check if the iterator is empty
    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }

    /// Get the number of remaining items
    pub fn len(&self) -> usize {
        self.locations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_basic_insert_and_retrieve() {
        let tmp_file = NamedTempFile::new().unwrap();
        let universe = Universe::open(tmp_file.path()).unwrap();

        let hash = [0x42; 32];
        let loc = ChunkLocation::new(
            "star-1".to_string(),
            PathBuf::from("/test/file.bin"),
            0,
            4096,
        );

        universe.insert_chunk(hash, loc.clone()).unwrap();

        assert!(universe.has_chunk(&hash).unwrap());

        let iter = universe.find_chunk(hash).unwrap();
        let locations: Vec<_> = iter.collect();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].star_id, "star-1");
        assert_eq!(locations[0].path, PathBuf::from("/test/file.bin"));
    }

    #[test]
    fn test_multiple_locations_same_hash() {
        let tmp_file = NamedTempFile::new().unwrap();
        let universe = Universe::open(tmp_file.path()).unwrap();

        let hash = [0xAB; 32];

        // Insert 3 different locations for the same hash (different Stars)
        universe
            .insert_chunk(
                hash,
                ChunkLocation::new("star-1".to_string(), PathBuf::from("file1.bin"), 0, 1024),
            )
            .unwrap();
        universe
            .insert_chunk(
                hash,
                ChunkLocation::new("star-2".to_string(), PathBuf::from("file2.bin"), 4096, 1024),
            )
            .unwrap();
        universe
            .insert_chunk(
                hash,
                ChunkLocation::new("star-3".to_string(), PathBuf::from("file3.bin"), 8192, 1024),
            )
            .unwrap();

        let iter = universe.find_chunk(hash).unwrap();
        let locations: Vec<_> = iter.collect();
        assert_eq!(locations.len(), 3);
    }

    #[test]
    fn test_scan_chunk_with_early_exit() {
        let tmp_file = NamedTempFile::new().unwrap();
        let universe = Universe::open(tmp_file.path()).unwrap();

        let hash = [0xCD; 32];

        // Insert 5 locations
        for i in 0..5 {
            universe
                .insert_chunk(
                    hash,
                    ChunkLocation::new(
                        format!("star-{}", i),
                        PathBuf::from(format!("file{}.bin", i)),
                        0,
                        100,
                    ),
                )
                .unwrap();
        }

        // Scan and stop after 2 items
        let mut count = 0;
        universe
            .scan_chunk(&hash, |_| {
                count += 1;
                count < 2 // Stop after 2 items
            })
            .unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn test_nonexistent_chunk() {
        let tmp_file = NamedTempFile::new().unwrap();
        let universe = Universe::open(tmp_file.path()).unwrap();

        let hash = [0xFF; 32];

        assert!(!universe.has_chunk(&hash).unwrap());

        let iter = universe.find_chunk(hash).unwrap();
        assert_eq!(iter.len(), 0);
    }

    #[test]
    fn test_chunk_location_serialization() {
        let loc = ChunkLocation::new(
            "star-42".to_string(),
            PathBuf::from("/data/test.bin"),
            1024,
            4096,
        );

        let serialized = bincode::serialize(&loc).unwrap();
        let deserialized: ChunkLocation = bincode::deserialize(&serialized).unwrap();

        assert_eq!(loc, deserialized);
        assert_eq!(deserialized.star_id, "star-42");
    }
}
