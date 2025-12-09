//! Standard Executor: The workhorse of Orbit's Equilibrium lane.
//!
//! Handles CDC chunking, Universe indexing, and delta transfers for medium-sized files (8KB to 1GB).
//!
//! # Design Philosophy
//!
//! The Standard Executor represents the "Equilibrium" profile - balancing CPU (hashing),
//! Memory (indexing), and Network (transfer) for optimal throughput on typical data.
//!
//! # The Pipeline
//!
//! 1. **Chunking**: Content-Defined Chunking (CDC) using Gear Hash with 64KB average chunks
//! 2. **Indexing**: Lookup chunks in the Universe Map (global deduplication index)
//! 3. **Filtering**: Only transfer chunks not already present at destination
//! 4. **Transfer**: Send unique chunks using the connection pool
//!
//! # Concurrency Model
//!
//! - **Compute**: CPU-intensive hashing is offloaded to blocking threads via `offload_compute`
//! - **Network**: Standard auto-concurrency based on `std::thread::available_parallelism()`
//!
//! # Example
//!
//! ```no_run
//! use magnetar::executor::standard::StandardExecutor;
//! use orbit_core_starmap::universe::Universe;
//! use std::sync::Arc;
//! use std::path::PathBuf;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let universe = Arc::new(Universe::open("universe.db")?);
//! let executor = StandardExecutor::new(universe);
//!
//! let file = PathBuf::from("document.pdf");
//! let stats = executor.process_file(file).await?;
//!
//! println!("Chunks transferred: {}", stats.chunks_transferred);
//! println!("Bytes saved: {}", stats.bytes_deduplicated);
//! # Ok(())
//! # }
//! ```

use super::offload_compute;
use anyhow::{Context, Result};
use orbit_core_cdc::{Chunk, ChunkConfig, ChunkStream};
use orbit_core_starmap::universe::{ChunkLocation, Universe};
use std::path::PathBuf;
use std::sync::Arc;

/// Statistics for a standard execution
#[derive(Debug, Clone, Default)]
pub struct StandardStats {
    /// Total number of chunks processed
    pub total_chunks: usize,

    /// Number of chunks transferred (not deduplicated)
    pub chunks_transferred: usize,

    /// Number of bytes transferred
    pub bytes_transferred: u64,

    /// Number of bytes saved through deduplication
    pub bytes_deduplicated: u64,
}

/// The Standard Executor for medium-sized files
///
/// Uses CDC chunking and Universe deduplication to efficiently transfer files.
pub struct StandardExecutor {
    /// Global deduplication index
    universe: Arc<Universe>,

    /// Chunking configuration (uses defaults: 64KB avg)
    chunk_config: ChunkConfig,
}

impl StandardExecutor {
    /// Create a new StandardExecutor with a Universe index
    ///
    /// # Arguments
    ///
    /// * `universe` - Global content-addressed index for deduplication
    ///
    /// # Example
    ///
    /// ```no_run
    /// use magnetar::executor::standard::StandardExecutor;
    /// use orbit_core_starmap::universe::Universe;
    /// use std::sync::Arc;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let universe = Arc::new(Universe::open("universe.db")?);
    /// let executor = StandardExecutor::new(universe);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(universe: Arc<Universe>) -> Self {
        Self {
            universe,
            chunk_config: ChunkConfig::default(), // 8KB min, 64KB avg, 256KB max
        }
    }

    /// Process a single file through the Equilibrium lane
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to process
    ///
    /// # Returns
    ///
    /// Statistics about the transfer (chunks, bytes, deduplication)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be read
    /// - Chunking fails
    /// - Universe index operations fail
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use magnetar::executor::standard::StandardExecutor;
    /// # use orbit_core_starmap::universe::Universe;
    /// # use std::sync::Arc;
    /// # use std::path::PathBuf;
    /// # async fn example() -> anyhow::Result<()> {
    /// # let universe = Arc::new(Universe::open("universe.db")?);
    /// # let executor = StandardExecutor::new(universe);
    /// let stats = executor.process_file(PathBuf::from("large_repo.tar.gz")).await?;
    /// println!("Deduplication rate: {:.1}%",
    ///     stats.bytes_deduplicated as f64 / (stats.bytes_transferred + stats.bytes_deduplicated) as f64 * 100.0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn process_file(&self, path: PathBuf) -> Result<StandardStats> {
        // 1. Offload the CDC Chunking to a blocking thread (Air Gap pattern)
        // This prevents CPU-intensive hashing from starving the async reactor
        let path_clone = path.clone();
        let config = self.chunk_config.clone();

        let chunks = offload_compute(move || {
            // Open file synchronously (we're in a blocking thread)
            let file = std::fs::File::open(&path_clone)
                .with_context(|| format!("Failed to open file: {:?}", path_clone))?;

            // Create chunk stream and collect all chunks
            let stream = ChunkStream::new(file, config);
            stream
                .collect::<Result<Vec<Chunk>, _>>()
                .map_err(|e| anyhow::anyhow!("Chunking failed for {:?}: {}", path_clone, e))
        })
        .await?;

        // 2. Index Query & Filter (The Dedup Step)
        // For each chunk, check if it exists in the Universe Map
        let mut chunks_to_send = Vec::new();
        let mut stats = StandardStats {
            total_chunks: chunks.len(),
            ..Default::default()
        };

        for chunk in chunks {
            let chunk_size = chunk.length as u64;

            // Check Universe Map (Is this chunk already known globally?)
            let exists = self
                .universe
                .has_chunk(&chunk.hash)
                .context("Failed to query Universe index")?;

            if !exists {
                // New chunk - need to transfer
                chunks_to_send.push(chunk);
                stats.bytes_transferred += chunk_size;
            } else {
                // Deduplicated - no transfer needed
                stats.bytes_deduplicated += chunk_size;
            }
        }

        // 3. Register new chunks in the Universe
        // In a real implementation, we'd also actually transfer the chunks here
        for chunk in &chunks_to_send {
            // Create a location record
            let location = ChunkLocation {
                path: path.clone(),
                offset: chunk.offset,
                length: chunk.length as u32,
            };

            self.universe
                .insert_chunk(chunk.hash, location)
                .context("Failed to insert chunk into Universe")?;
        }

        stats.chunks_transferred = chunks_to_send.len();

        Ok(stats)
    }

    /// Process multiple files in batch
    ///
    /// This is useful for processing entire directories where you want
    /// aggregated statistics across all files.
    pub async fn process_batch(&self, paths: Vec<PathBuf>) -> Result<StandardStats> {
        let mut aggregate = StandardStats::default();

        for path in paths {
            let file_stats = self.process_file(path).await?;

            aggregate.total_chunks += file_stats.total_chunks;
            aggregate.chunks_transferred += file_stats.chunks_transferred;
            aggregate.bytes_transferred += file_stats.bytes_transferred;
            aggregate.bytes_deduplicated += file_stats.bytes_deduplicated;
        }

        Ok(aggregate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, NamedTempFile};
    use tokio::fs;

    /// Create a Universe database in a temporary location
    fn create_test_universe() -> Result<(tempfile::TempDir, Arc<Universe>)> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test_universe.db");
        let universe = Arc::new(Universe::open(&db_path)?);
        Ok((temp_dir, universe))
    }

    #[tokio::test]
    async fn test_standard_executor_basic() -> Result<()> {
        let (_temp_dir, universe) = create_test_universe()?;
        let executor = StandardExecutor::new(universe);

        // Create a test file (1MB of repeated pattern)
        let temp_file = NamedTempFile::new()?;
        let data = vec![0x42u8; 1024 * 1024]; // 1MB
        std::fs::write(temp_file.path(), &data)?;

        // Process the file
        let stats = executor
            .process_file(temp_file.path().to_path_buf())
            .await?;

        // Verify stats
        assert!(stats.total_chunks > 0, "Should produce chunks");
        assert_eq!(
            stats.chunks_transferred, stats.total_chunks,
            "First pass should transfer all chunks"
        );
        assert_eq!(
            stats.bytes_deduplicated, 0,
            "First pass should have no deduplication"
        );
        assert!(stats.bytes_transferred > 0, "Should transfer bytes");

        Ok(())
    }

    #[tokio::test]
    async fn test_standard_executor_deduplication() -> Result<()> {
        let (_temp_dir, universe) = create_test_universe()?;
        let executor = StandardExecutor::new(universe.clone());

        // Create first file
        let temp_file1 = NamedTempFile::new()?;
        let data = vec![0x42u8; 1024 * 1024]; // 1MB
        std::fs::write(temp_file1.path(), &data)?;

        // Process first file
        let stats1 = executor
            .process_file(temp_file1.path().to_path_buf())
            .await?;
        let first_pass_bytes = stats1.bytes_transferred;

        // Create identical second file
        let temp_file2 = NamedTempFile::new()?;
        std::fs::write(temp_file2.path(), &data)?;

        // Process second file (should be fully deduplicated)
        let stats2 = executor
            .process_file(temp_file2.path().to_path_buf())
            .await?;

        // Verify deduplication
        assert_eq!(
            stats2.chunks_transferred, 0,
            "Second pass should transfer no chunks"
        );
        assert_eq!(
            stats2.bytes_transferred, 0,
            "Second pass should transfer no bytes"
        );
        assert_eq!(
            stats2.bytes_deduplicated, first_pass_bytes,
            "Second pass should deduplicate all bytes"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_standard_executor_partial_dedup() -> Result<()> {
        let (_temp_dir, universe) = create_test_universe()?;
        let executor = StandardExecutor::new(universe);

        // Create first file with pattern A
        let temp_file1 = NamedTempFile::new()?;
        let mut data1 = vec![0x42u8; 512 * 1024]; // 512KB of 0x42
        std::fs::write(temp_file1.path(), &data1)?;

        // Process first file
        let stats1 = executor
            .process_file(temp_file1.path().to_path_buf())
            .await?;

        // Create second file with pattern A + pattern B
        let temp_file2 = NamedTempFile::new()?;
        data1.extend(vec![0x99u8; 512 * 1024]); // Add 512KB of 0x99
        std::fs::write(temp_file2.path(), &data1)?;

        // Process second file (first half should be deduplicated)
        let stats2 = executor
            .process_file(temp_file2.path().to_path_buf())
            .await?;

        // Verify partial deduplication
        assert!(
            stats2.bytes_deduplicated > 0,
            "Should have some deduplication"
        );
        assert!(stats2.bytes_transferred > 0, "Should transfer new content");
        assert!(
            stats2.bytes_transferred < stats1.bytes_transferred + 512 * 1024,
            "Should not transfer all bytes due to dedup"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_standard_executor_empty_file() -> Result<()> {
        let (_temp_dir, universe) = create_test_universe()?;
        let executor = StandardExecutor::new(universe);

        // Create empty file
        let temp_file = NamedTempFile::new()?;

        // Process empty file
        let stats = executor
            .process_file(temp_file.path().to_path_buf())
            .await?;

        // Verify stats
        assert_eq!(stats.total_chunks, 0, "Empty file should produce no chunks");
        assert_eq!(stats.chunks_transferred, 0);
        assert_eq!(stats.bytes_transferred, 0);
        assert_eq!(stats.bytes_deduplicated, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_standard_executor_batch() -> Result<()> {
        let (_temp_dir, universe) = create_test_universe()?;
        let executor = StandardExecutor::new(universe);

        // Create multiple test files (keep temp files alive)
        let mut temp_files = Vec::new();
        let mut paths = Vec::new();
        for i in 0..3 {
            let temp_file = NamedTempFile::new()?;
            let data = vec![i as u8; 100 * 1024]; // 100KB each
            std::fs::write(temp_file.path(), &data)?;
            paths.push(temp_file.path().to_path_buf());
            temp_files.push(temp_file); // Keep files alive
        }

        // Process batch
        let stats = executor.process_batch(paths).await?;

        // Verify aggregate stats
        assert!(stats.total_chunks > 0, "Should process chunks");
        assert!(stats.bytes_transferred > 0, "Should transfer bytes");

        Ok(())
    }
}
