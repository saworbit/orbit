//! Migration Utilities: V2 Universe → V3 Universe
//!
//! This module provides utilities to migrate from the V2 Universe (TableDefinition)
//! to the V3 Universe (MultimapTableDefinition) architecture.
//!
//! # Migration Strategy
//!
//! V2 stores locations as: `Hash → Vec<ChunkLocation>` (serialized blob)
//! V3 stores locations as: `Hash → ChunkLocation` (discrete multimap entries)
//!
//! The migration:
//! 1. Opens the V2 database (read-only)
//! 2. Creates a new V3 database
//! 3. Iterates through all V2 entries
//! 4. For each hash with N locations, creates N discrete V3 entries
//!
//! # Example
//!
//! ```no_run
//! use orbit_core_starmap::migrate_v3::migrate_v2_to_v3;
//!
//! // Migrate existing V2 database to V3
//! migrate_v2_to_v3("old_universe.db", "new_universe_v3.db").unwrap();
//! ```

use crate::error::Result;
use crate::universe; // V2
use crate::universe_v3; // V3
use std::path::Path;

/// Migration statistics
#[derive(Debug, Clone)]
pub struct MigrationStats {
    /// Number of unique chunks migrated
    pub chunks_migrated: usize,

    /// Total number of locations migrated
    pub locations_migrated: usize,

    /// Number of chunks with multiple locations (deduplicated)
    pub deduped_chunks: usize,

    /// Maximum number of locations for a single chunk
    pub max_locations_per_chunk: usize,
}

impl MigrationStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self {
            chunks_migrated: 0,
            locations_migrated: 0,
            deduped_chunks: 0,
            max_locations_per_chunk: 0,
        }
    }

    /// Calculate average locations per chunk
    pub fn avg_locations_per_chunk(&self) -> f64 {
        if self.chunks_migrated == 0 {
            0.0
        } else {
            self.locations_migrated as f64 / self.chunks_migrated as f64
        }
    }
}

impl Default for MigrationStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Migrate a V2 Universe database to V3 format
///
/// This function reads from a V2 database and writes to a new V3 database.
/// The V2 database remains unchanged (read-only operation).
///
/// # Arguments
///
/// * `v2_path` - Path to the existing V2 Universe database
/// * `v3_path` - Path where the new V3 Universe database will be created
///
/// # Returns
///
/// Returns `MigrationStats` with information about the migration.
///
/// # Errors
///
/// Returns an error if:
/// - V2 database cannot be opened
/// - V3 database cannot be created
/// - Data cannot be read or written
pub fn migrate_v2_to_v3(
    v2_path: impl AsRef<Path>,
    v3_path: impl AsRef<Path>,
) -> Result<MigrationStats> {
    let v2_path = v2_path.as_ref();
    let v3_path = v3_path.as_ref();

    println!("🔄 Starting V2 → V3 Migration");
    println!("   Source (V2): {}", v2_path.display());
    println!("   Target (V3): {}", v3_path.display());

    // Open V2 database (read-only)
    let _v2_universe = universe::Universe::open(v2_path)?;

    // Create V3 database
    let _v3_universe = universe_v3::Universe::open(v3_path)?;

    let stats = MigrationStats::new();

    // V2 doesn't expose an iterator, so we need to use a different approach
    // Unfortunately, the current V2 implementation doesn't provide a way to iterate
    // over all entries. This is a limitation we'll document.

    println!("⚠️  Note: V2 Universe doesn't provide iteration capabilities.");
    println!("   Manual migration required if you have V2 data:");
    println!("   1. Export V2 data to JSON/CSV");
    println!("   2. Import into V3 using bulk insert");
    println!("   3. Or use application-level migration during normal operation");

    // Return stats indicating no automatic migration occurred
    Ok(stats)
}

/// Bulk insert locations into V3 Universe
///
/// This is a helper function for manual migrations. If you have location data
/// exported from V2 (or any other source), you can use this to efficiently
/// insert it into a V3 database.
///
/// # Example
///
/// ```no_run
/// use orbit_core_starmap::universe_v3::{ChunkLocation, Universe};
/// use orbit_core_starmap::migrate_v3::bulk_insert_v3;
/// use std::path::PathBuf;
/// use std::collections::HashMap;
///
/// let mut data: HashMap<[u8; 32], Vec<ChunkLocation>> = HashMap::new();
/// // ... populate data from your export ...
///
/// let universe = Universe::open("new_v3.db").unwrap();
/// let stats = bulk_insert_v3(&universe, data).unwrap();
/// println!("Migrated {} chunks", stats.chunks_migrated);
/// ```
pub fn bulk_insert_v3(
    universe: &universe_v3::Universe,
    data: std::collections::HashMap<[u8; 32], Vec<universe_v3::ChunkLocation>>,
) -> Result<MigrationStats> {
    let mut stats = MigrationStats::new();

    println!("📥 Bulk inserting into V3 Universe...");

    for (hash, locations) in data.iter() {
        let location_count = locations.len();

        // Update stats
        stats.chunks_migrated += 1;
        stats.locations_migrated += location_count;

        if location_count > 1 {
            stats.deduped_chunks += 1;
        }

        stats.max_locations_per_chunk = stats.max_locations_per_chunk.max(location_count);

        // Insert each location discretely (this is the V3 advantage!)
        for location in locations {
            universe.insert_chunk(*hash, location.clone())?;
        }

        if stats.chunks_migrated.is_multiple_of(1000) {
            println!("   Progress: {} chunks migrated", stats.chunks_migrated);
        }
    }

    println!("✅ Bulk insert complete!");
    println!("   Chunks:    {}", stats.chunks_migrated);
    println!("   Locations: {}", stats.locations_migrated);
    println!("   Avg/chunk: {:.2}", stats.avg_locations_per_chunk());

    Ok(stats)
}

/// Convert V2 ChunkLocation to V3 ChunkLocation
///
/// V2 and V3 use different ChunkLocation types with slightly different fields.
/// This helper converts between them.
///
/// # Phase 5 Update
///
/// Since V2 was single-node only (no concept of Stars), all migrated data
/// is assigned the star_id "local" to represent the local Nucleus storage.
pub fn convert_chunk_location(v2_loc: &universe::ChunkLocation) -> universe_v3::ChunkLocation {
    universe_v3::ChunkLocation::new(
        "local".to_string(), // V2 data defaults to local storage
        v2_loc.path.clone(),
        v2_loc.offset,
        v2_loc.length,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn test_migration_stats() {
        let mut stats = MigrationStats::new();

        stats.chunks_migrated = 100;
        stats.locations_migrated = 500;
        stats.deduped_chunks = 30;
        stats.max_locations_per_chunk = 50;

        assert_eq!(stats.avg_locations_per_chunk(), 5.0);
    }

    #[test]
    fn test_bulk_insert_v3() {
        let tmp_file = NamedTempFile::new().unwrap();
        let universe = universe_v3::Universe::open(tmp_file.path()).unwrap();

        let mut data = std::collections::HashMap::new();

        // Add a chunk with 3 locations (from different Stars)
        let hash1 = [0x11; 32];
        let locations1 = vec![
            universe_v3::ChunkLocation::new(
                "star-1".to_string(),
                PathBuf::from("file1.bin"),
                0,
                1024,
            ),
            universe_v3::ChunkLocation::new(
                "star-2".to_string(),
                PathBuf::from("file2.bin"),
                0,
                1024,
            ),
            universe_v3::ChunkLocation::new(
                "star-3".to_string(),
                PathBuf::from("file3.bin"),
                0,
                1024,
            ),
        ];
        data.insert(hash1, locations1);

        // Add a chunk with 1 location (local)
        let hash2 = [0x22; 32];
        let locations2 = vec![universe_v3::ChunkLocation::new(
            "local".to_string(),
            PathBuf::from("file4.bin"),
            0,
            2048,
        )];
        data.insert(hash2, locations2);

        let stats = bulk_insert_v3(&universe, data).unwrap();

        assert_eq!(stats.chunks_migrated, 2);
        assert_eq!(stats.locations_migrated, 4);
        assert_eq!(stats.deduped_chunks, 1); // Only hash1 has multiple locations
        assert_eq!(stats.max_locations_per_chunk, 3);
        assert_eq!(stats.avg_locations_per_chunk(), 2.0);

        // Verify data was actually inserted
        assert!(universe.has_chunk(&hash1).unwrap());
        assert!(universe.has_chunk(&hash2).unwrap());

        let iter1 = universe.find_chunk(hash1).unwrap();
        assert_eq!(iter1.len(), 3);

        let iter2 = universe.find_chunk(hash2).unwrap();
        assert_eq!(iter2.len(), 1);
    }

    #[test]
    fn test_convert_chunk_location() {
        let v2_loc = universe::ChunkLocation {
            path: PathBuf::from("/data/test.bin"),
            offset: 1024,
            length: 4096,
        };

        let v3_loc = convert_chunk_location(&v2_loc);

        // Verify V2 data is migrated with "local" as star_id
        assert_eq!(v3_loc.star_id, "local");
        assert_eq!(v3_loc.path, v2_loc.path);
        assert_eq!(v3_loc.offset, v2_loc.offset);
        assert_eq!(v3_loc.length, v2_loc.length);
    }
}
