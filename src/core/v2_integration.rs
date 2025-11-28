//! V2 Integration Module - Bridges V2 Architecture with V1 Transfer Pipeline
//!
//! This module provides integration hooks to use V2 components (CDC, Semantic, Universe Map)
//! within the existing Orbit transfer infrastructure.
//!
//! # Usage
//!
//! ```no_run
//! use orbit::core::v2_integration::{V2Context, TransferJob};
//! use std::path::Path;
//!
//! // Initialize V2 context
//! let mut ctx = V2Context::new();
//!
//! // Analyze files and create prioritized jobs
//! let jobs = ctx.analyze_and_queue(vec![
//!     Path::new("app.toml"),
//!     Path::new("src/main.rs"),
//!     Path::new("video.mp4"),
//! ]).unwrap();
//!
//! // Jobs are sorted by priority (critical first)
//! for job in jobs {
//!     println!("Transfer: {} (priority: {:?})", job.path.display(), job.priority);
//! }
//! ```

use orbit_core_cdc::{ChunkConfig, ChunkStream};
use orbit_core_semantic::{Priority, ReplicationIntent, SemanticRegistry, SyncStrategy};
use orbit_core_starmap::{Location, UniverseMap};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum V2Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Starmap error: {0}")]
    Starmap(#[from] orbit_core_starmap::Error),

    #[error("CDC error: {0}")]
    Cdc(#[from] orbit_core_cdc::CdcError),

    #[error("Analysis failed: {0}")]
    Analysis(String),
}

pub type Result<T> = std::result::Result<T, V2Error>;

/// Transfer job with V2 metadata
#[derive(Debug, Clone)]
pub struct TransferJob {
    /// File path
    pub path: PathBuf,

    /// File size in bytes
    pub size: u64,

    /// Replication priority
    pub priority: Priority,

    /// Sync strategy
    pub strategy: SyncStrategy,

    /// Human-readable description
    pub description: String,
}

impl TransferJob {
    /// Create a new transfer job
    pub fn new(
        path: PathBuf,
        size: u64,
        priority: Priority,
        strategy: SyncStrategy,
        description: String,
    ) -> Self {
        Self {
            path,
            size,
            priority,
            strategy,
            description,
        }
    }

    /// Create from a path and intent
    pub fn from_intent(path: PathBuf, size: u64, intent: ReplicationIntent) -> Self {
        Self {
            path,
            size,
            priority: intent.priority,
            strategy: intent.strategy,
            description: intent.description,
        }
    }
}

/// V2 Context - Maintains semantic registry and universe map
pub struct V2Context {
    /// Semantic analyzer for file prioritization
    registry: SemanticRegistry,

    /// Global deduplication index
    universe: UniverseMap,

    /// CDC configuration
    chunk_config: ChunkConfig,
}

impl V2Context {
    /// Create a new V2 context with default settings
    pub fn new() -> Self {
        Self {
            registry: SemanticRegistry::default(),
            universe: UniverseMap::new(),
            chunk_config: ChunkConfig::default(),
        }
    }

    /// Create with custom chunk configuration
    pub fn with_chunk_config(chunk_config: ChunkConfig) -> Self {
        Self {
            registry: SemanticRegistry::default(),
            universe: UniverseMap::new(),
            chunk_config,
        }
    }

    /// Analyze a file and determine its replication intent
    pub fn analyze_file(&self, path: &Path) -> Result<ReplicationIntent> {
        // Read first 4KB for magic number detection
        let mut file = File::open(path)?;
        let mut head_bytes = vec![0u8; 4096];
        let bytes_read = file.read(&mut head_bytes)?;
        head_bytes.truncate(bytes_read);

        Ok(self.registry.determine_intent(path, &head_bytes))
    }

    /// Analyze multiple files and create prioritized transfer jobs
    pub fn analyze_and_queue(&self, paths: Vec<&Path>) -> Result<Vec<TransferJob>> {
        let mut jobs = Vec::new();

        for path in paths {
            let metadata = std::fs::metadata(path)?;
            let size = metadata.len();

            let intent = self.analyze_file(path)?;

            jobs.push(TransferJob::from_intent(path.to_path_buf(), size, intent));
        }

        // Sort by priority (lower value = higher priority)
        jobs.sort_by_key(|job| job.priority);

        Ok(jobs)
    }

    /// Index a file using CDC and add to universe map
    pub fn index_file(&mut self, path: &Path) -> Result<usize> {
        let file = File::open(path)?;
        let file_id = self.universe.register_file(path.to_string_lossy());

        let reader = BufReader::new(file);
        let stream = ChunkStream::new(reader, self.chunk_config);

        let mut chunk_count = 0;
        for chunk in stream {
            let chunk = chunk?;

            self.universe.add_chunk(
                &chunk.meta.hash,
                Location::new(file_id, chunk.meta.offset, chunk.meta.length as u32),
            );

            chunk_count += 1;
        }

        Ok(chunk_count)
    }

    /// Check if a chunk exists in the universe map (for dedup)
    pub fn has_chunk(&self, content_id: &[u8; 32]) -> bool {
        self.universe.has_chunk(content_id)
    }

    /// Get universe map reference
    pub fn universe(&self) -> &UniverseMap {
        &self.universe
    }

    /// Get mutable universe map reference
    pub fn universe_mut(&mut self) -> &mut UniverseMap {
        &mut self.universe
    }

    /// Get deduplication statistics
    pub fn dedup_stats(&self) -> orbit_core_starmap::DedupStats {
        self.universe.dedup_stats()
    }

    /// Save universe map to disk
    pub fn save_universe(&self, path: impl AsRef<Path>) -> Result<()> {
        Ok(self.universe.save(path)?)
    }

    /// Load universe map from disk
    pub fn load_universe(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.universe = UniverseMap::load(path)?;
        Ok(())
    }
}

impl Default for V2Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_v2_context_creation() {
        let ctx = V2Context::new();
        assert_eq!(ctx.universe.chunk_count(), 0);
    }

    #[test]
    fn test_analyze_file() {
        let mut config_file = NamedTempFile::new().unwrap();
        config_file.write_all(b"[config]\nkey = value\n").unwrap();
        config_file.flush().unwrap();

        // Rename to .toml
        let config_path = config_file.path().with_extension("toml");
        std::fs::copy(config_file.path(), &config_path).unwrap();

        let ctx = V2Context::new();
        let intent = ctx.analyze_file(&config_path).unwrap();

        assert_eq!(intent.priority, Priority::Critical);
        assert_eq!(intent.strategy, SyncStrategy::AtomicReplace);

        std::fs::remove_file(config_path).ok();
    }

    #[test]
    fn test_analyze_and_queue() {
        let mut file1 = NamedTempFile::new().unwrap();
        let mut file2 = NamedTempFile::new().unwrap();

        file1.write_all(b"config data").unwrap();
        file2.write_all(b"video data").unwrap();

        let config_path = file1.path().with_extension("toml");
        let video_path = file2.path().with_extension("mp4");

        std::fs::copy(file1.path(), &config_path).unwrap();
        std::fs::copy(file2.path(), &video_path).unwrap();

        let ctx = V2Context::new();
        let jobs = ctx
            .analyze_and_queue(vec![&video_path, &config_path])
            .unwrap();

        // Config should come first (higher priority)
        assert_eq!(jobs[0].priority, Priority::Critical);
        assert!(jobs[0].path.to_string_lossy().contains("toml"));

        assert_eq!(jobs[1].priority, Priority::Low);
        assert!(jobs[1].path.to_string_lossy().contains("mp4"));

        std::fs::remove_file(config_path).ok();
        std::fs::remove_file(video_path).ok();
    }

    #[test]
    fn test_index_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&vec![0xAB; 10_000]).unwrap();
        temp_file.flush().unwrap();

        let mut ctx = V2Context::new();
        let chunk_count = ctx.index_file(temp_file.path()).unwrap();

        assert!(chunk_count > 0, "Should have created chunks");
        // Verify chunks were added to universe
        assert!(
            ctx.universe.chunk_count() > 0,
            "Universe should have indexed chunks"
        );
    }
}
