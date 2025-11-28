//! Migration utilities for StarMap V1 → V2 (Universe Map)
//!
//! Provides tools to convert file-scoped V1 starmaps into the global
//! content-addressed V2 Universe Map.
//!
//! # Example
//!
//! ```no_run
//! use orbit_core_starmap::migrate::migrate_to_universe;
//! use std::path::Path;
//!
//! // Migrate a single V1 starmap
//! let universe = migrate_to_universe(
//!     Path::new("file1.starmap.bin"),
//!     "path/to/file1.txt"
//! ).unwrap();
//!
//! println!("Migrated {} chunks", universe.chunk_count());
//! universe.save("universe.map").unwrap();
//! ```

use crate::error::Result;
use crate::reader::StarMapReader;
use crate::universe::{Location, UniverseMap};
use std::path::Path;

/// Migrate a V1 StarMap to V2 Universe Map
///
/// # Arguments
/// * `starmap_path` - Path to the V1 .starmap.bin file
/// * `original_file_path` - Original file path (for file registry)
///
/// # Returns
/// A new UniverseMap with chunks from the V1 starmap
pub fn migrate_to_universe(
    starmap_path: impl AsRef<Path>,
    original_file_path: impl AsRef<str>,
) -> Result<UniverseMap> {
    let reader = StarMapReader::open(starmap_path)?;
    let mut universe = UniverseMap::new();

    // Register the file
    let file_id = universe.register_file(original_file_path);

    // Migrate all chunks
    let chunk_count = reader.chunk_count()?;
    for i in 0..chunk_count {
        let chunk = reader.get_chunk(i)?;

        universe.add_chunk(
            &chunk.content_id,
            Location::new(file_id, chunk.offset, chunk.length),
        );
    }

    Ok(universe)
}

/// Migrate multiple V1 StarMaps into a single Universe Map
///
/// # Arguments
/// * `starmaps` - List of (starmap_path, original_file_path) tuples
///
/// # Returns
/// A UniverseMap containing all chunks from all starmaps (with global dedup)
pub fn migrate_batch(starmaps: Vec<(impl AsRef<Path>, impl AsRef<str>)>) -> Result<UniverseMap> {
    let mut universe = UniverseMap::new();

    for (starmap_path, file_path) in starmaps {
        let reader = StarMapReader::open(starmap_path)?;
        let file_id = universe.register_file(file_path);

        let chunk_count = reader.chunk_count()?;
        for i in 0..chunk_count {
            let chunk = reader.get_chunk(i)?;

            universe.add_chunk(
                &chunk.content_id,
                Location::new(file_id, chunk.offset, chunk.length),
            );
        }
    }

    Ok(universe)
}

/// Migration statistics
#[derive(Debug, Clone)]
pub struct MigrationStats {
    /// Number of V1 starmaps processed
    pub starmaps_processed: usize,

    /// Total chunks in V1 starmaps
    pub total_chunks_v1: usize,

    /// Unique chunks in V2 universe (after dedup)
    pub unique_chunks_v2: usize,

    /// Chunks deduplicated across files
    pub chunks_deduped: usize,

    /// Space savings from deduplication
    pub dedup_ratio: f64,
}

impl MigrationStats {
    /// Calculate migration statistics
    pub fn from_universe(
        starmaps_processed: usize,
        total_chunks_v1: usize,
        universe: &UniverseMap,
    ) -> Self {
        let unique_chunks_v2 = universe.chunk_count();
        let chunks_deduped = total_chunks_v1.saturating_sub(unique_chunks_v2);
        let dedup_ratio = if total_chunks_v1 > 0 {
            chunks_deduped as f64 / total_chunks_v1 as f64
        } else {
            0.0
        };

        Self {
            starmaps_processed,
            total_chunks_v1,
            unique_chunks_v2,
            chunks_deduped,
            dedup_ratio,
        }
    }

    /// Print migration summary
    pub fn print_summary(&self) {
        println!("📦 Migration Summary:");
        println!("  StarMaps processed: {}", self.starmaps_processed);
        println!("  Total chunks (V1): {}", self.total_chunks_v1);
        println!("  Unique chunks (V2): {}", self.unique_chunks_v2);
        println!("  Chunks deduplicated: {}", self.chunks_deduped);
        println!("  Deduplication ratio: {:.1}%", self.dedup_ratio * 100.0);
    }
}

/// Migrate with statistics tracking
pub fn migrate_batch_with_stats(
    starmaps: Vec<(impl AsRef<Path>, impl AsRef<str>)>,
) -> Result<(UniverseMap, MigrationStats)> {
    let mut universe = UniverseMap::new();
    let mut total_chunks = 0;

    for (starmap_path, file_path) in &starmaps {
        let reader = StarMapReader::open(starmap_path)?;
        let file_id = universe.register_file(file_path);

        let chunk_count = reader.chunk_count()?;
        total_chunks += chunk_count as usize;

        for i in 0..chunk_count {
            let chunk = reader.get_chunk(i)?;

            universe.add_chunk(
                &chunk.content_id,
                Location::new(file_id, chunk.offset, chunk.length),
            );
        }
    }

    let stats = MigrationStats::from_universe(starmaps.len(), total_chunks, &universe);

    Ok((universe, stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::StarMapBuilder;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_migrate_single_starmap() {
        // Create a test V1 starmap
        let mut builder = StarMapBuilder::new(10000);
        builder.add_chunk(0, 4096, &[0x01; 32]).unwrap();
        builder.add_chunk(4096, 4096, &[0x02; 32]).unwrap();
        builder.add_window(0, 0, 2, &[0xAA; 32], 0).unwrap();

        let data = builder.build().unwrap();

        // Write to temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp_file, &data).unwrap();
        temp_file.flush().unwrap();

        // Migrate
        let universe = migrate_to_universe(temp_file.path(), "test_file.txt").unwrap();

        assert_eq!(universe.chunk_count(), 2);
        assert!(universe.has_chunk(&[0x01; 32]));
        assert!(universe.has_chunk(&[0x02; 32]));
        assert_eq!(universe.get_file_path(1), Some("test_file.txt"));
    }

    #[test]
    fn test_migrate_batch() {
        // Create two V1 starmaps with overlapping content
        let mut builder1 = StarMapBuilder::new(10000);
        builder1.add_chunk(0, 4096, &[0xAA; 32]).unwrap(); // Same chunk
        builder1.add_chunk(4096, 4096, &[0xBB; 32]).unwrap();
        builder1.add_window(0, 0, 2, &[0x11; 32], 0).unwrap();
        let data1 = builder1.build().unwrap();

        let mut builder2 = StarMapBuilder::new(10000);
        builder2.add_chunk(0, 4096, &[0xAA; 32]).unwrap(); // Duplicate!
        builder2.add_chunk(4096, 4096, &[0xCC; 32]).unwrap();
        builder2.add_window(0, 0, 2, &[0x22; 32], 0).unwrap();
        let data2 = builder2.build().unwrap();

        // Write to temp files
        let mut file1 = NamedTempFile::new().unwrap();
        let mut file2 = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut file1, &data1).unwrap();
        std::io::Write::write_all(&mut file2, &data2).unwrap();
        file1.flush().unwrap();
        file2.flush().unwrap();

        // Migrate batch
        let universe = migrate_batch(vec![
            (file1.path(), "file1.txt"),
            (file2.path(), "file2.txt"),
        ])
        .unwrap();

        // Total chunks in V1: 4 (2 + 2)
        // Unique chunks in V2: 3 (AA is shared)
        assert_eq!(universe.chunk_count(), 3);

        // Verify the duplicate chunk appears in both files
        let locations = universe.get_locations(&[0xAA; 32]).unwrap();
        assert_eq!(locations.len(), 2, "Shared chunk should have 2 locations");
    }

    #[test]
    fn test_migration_stats() {
        let stats = MigrationStats::from_universe(
            2,  // 2 starmaps
            10, // 10 total chunks in V1
            &{
                let mut u = UniverseMap::new();
                // Simulate 7 unique chunks (3 were deduped)
                for i in 0..7 {
                    u.add_chunk(&[i; 32], Location::new(1, 0, 4096));
                }
                u
            },
        );

        assert_eq!(stats.starmaps_processed, 2);
        assert_eq!(stats.total_chunks_v1, 10);
        assert_eq!(stats.unique_chunks_v2, 7);
        assert_eq!(stats.chunks_deduped, 3);
        assert!((stats.dedup_ratio - 0.3).abs() < 0.01); // 30% dedup
    }
}
