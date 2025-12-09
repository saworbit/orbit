//! Transfer Strategy Router
//!
//! Routes files to the appropriate execution lane based on file size,
//! optimizing for throughput, deduplication, and resource usage.

use orbit_core_cdc::ChunkConfig;

/// Transfer strategy selection based on file characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStrategy {
    /// Direct transfer for small files (<8KB)
    Direct,

    /// Standard deduplication with 64KB chunks (8KB - 1GB)
    DeduplicatedStandard,

    /// Tiered deduplication with large chunks (>1GB)
    /// Uses 1-4MB chunks to prevent index explosion
    DeduplicatedTiered,
}

/// Router for selecting transfer strategies and configurations
pub struct PipelineRouter;

impl PipelineRouter {
    /// Neutrino threshold: Files below this size skip CDC entirely
    const NEUTRINO_THRESHOLD: u64 = 8 * 1024; // 8KB

    /// Gigantor threshold: Files above this size use tiered chunking
    const GIGANTOR_THRESHOLD: u64 = 1024 * 1024 * 1024; // 1GB

    /// Extra-large file threshold for even larger chunks
    const GIGANTOR_LARGE_THRESHOLD: u64 = 100 * 1024 * 1024 * 1024; // 100GB

    /// Select the optimal transfer strategy based on file size
    ///
    /// # Arguments
    ///
    /// * `file_size` - Size of the file in bytes
    ///
    /// # Returns
    ///
    /// The recommended transfer strategy for this file
    ///
    /// # Example
    ///
    /// ```
    /// use magnetar::pipeline::PipelineRouter;
    ///
    /// // Small config file
    /// let strategy = PipelineRouter::select_strategy(4096);
    /// // Returns: TransferStrategy::Direct
    ///
    /// // Medium PDF
    /// let strategy = PipelineRouter::select_strategy(5_000_000);
    /// // Returns: TransferStrategy::DeduplicatedStandard
    ///
    /// // Large VM image
    /// let strategy = PipelineRouter::select_strategy(10_000_000_000);
    /// // Returns: TransferStrategy::DeduplicatedTiered
    /// ```
    pub fn select_strategy(file_size: u64) -> TransferStrategy {
        if file_size < Self::NEUTRINO_THRESHOLD {
            TransferStrategy::Direct
        } else if file_size < Self::GIGANTOR_THRESHOLD {
            TransferStrategy::DeduplicatedStandard
        } else {
            // Files > 1GB enter the Gigantor Lane
            TransferStrategy::DeduplicatedTiered
        }
    }

    /// Calculate the optimal chunk configuration for large files
    ///
    /// This prevents "Index Explosion" in the Universe Map by using
    /// larger chunk sizes that scale with file size.
    ///
    /// # Arguments
    ///
    /// * `file_size` - Size of the file in bytes
    ///
    /// # Returns
    ///
    /// Optimal chunk configuration for this file size
    ///
    /// # Chunk Size Strategy
    ///
    /// - **1GB - 100GB**: 1MB average chunks (16x reduction vs standard)
    /// - **>100GB**: 4MB average chunks (64x reduction vs standard)
    ///
    /// This ensures that even a 10TB file produces a manageable ~2.5M
    /// chunk index entries instead of ~160M entries with standard 64KB chunks.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use magnetar::pipeline::PipelineRouter;
    ///
    /// // 5GB file
    /// let config = PipelineRouter::optimal_chunk_config(5_000_000_000);
    /// // Returns: 256KB min, 1MB avg, 4MB max
    ///
    /// // 500GB file
    /// let config = PipelineRouter::optimal_chunk_config(500_000_000_000);
    /// // Returns: 1MB min, 4MB avg, 16MB max
    /// ```
    pub fn optimal_chunk_config(file_size: u64) -> ChunkConfig {
        if file_size > Self::GIGANTOR_LARGE_THRESHOLD {
            // > 100 GB: 4MB Average
            // Reduces index size by 64x vs standard (64KB chunks)
            // 1MB min, 4MB avg, 16MB max
            ChunkConfig::new(
                1024 * 1024,      // 1MB min
                4 * 1024 * 1024,  // 4MB avg
                16 * 1024 * 1024, // 16MB max
            )
            .expect("Gigantor large chunk config is valid")
        } else {
            // 1GB - 100GB: 1MB Average
            // Reduces index size by 16x vs standard (64KB chunks)
            // 256KB min, 1MB avg, 4MB max
            ChunkConfig::new(
                256 * 1024,      // 256KB min
                1024 * 1024,     // 1MB avg
                4 * 1024 * 1024, // 4MB max
            )
            .expect("Gigantor standard chunk config is valid")
        }
    }

    /// Get a human-readable description of the strategy
    pub fn strategy_name(strategy: TransferStrategy) -> &'static str {
        match strategy {
            TransferStrategy::Direct => "Neutrino Fast Lane",
            TransferStrategy::DeduplicatedStandard => "Equilibrium Standard Lane",
            TransferStrategy::DeduplicatedTiered => "Gigantor Heavy Lift Lane",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neutrino_routing() {
        // Files < 8KB should use Direct strategy
        assert_eq!(PipelineRouter::select_strategy(0), TransferStrategy::Direct);
        assert_eq!(
            PipelineRouter::select_strategy(4096),
            TransferStrategy::Direct
        );
        assert_eq!(
            PipelineRouter::select_strategy(8191),
            TransferStrategy::Direct
        );
    }

    #[test]
    fn test_equilibrium_routing() {
        // Files 8KB - 1GB should use DeduplicatedStandard
        assert_eq!(
            PipelineRouter::select_strategy(8 * 1024),
            TransferStrategy::DeduplicatedStandard
        );
        assert_eq!(
            PipelineRouter::select_strategy(100 * 1024 * 1024),
            TransferStrategy::DeduplicatedStandard
        );
        assert_eq!(
            PipelineRouter::select_strategy(1024 * 1024 * 1024 - 1),
            TransferStrategy::DeduplicatedStandard
        );
    }

    #[test]
    fn test_gigantor_routing() {
        // Files >= 1GB should use DeduplicatedTiered
        assert_eq!(
            PipelineRouter::select_strategy(1024 * 1024 * 1024),
            TransferStrategy::DeduplicatedTiered
        );
        assert_eq!(
            PipelineRouter::select_strategy(50 * 1024 * 1024 * 1024),
            TransferStrategy::DeduplicatedTiered
        );
        assert_eq!(
            PipelineRouter::select_strategy(500 * 1024 * 1024 * 1024),
            TransferStrategy::DeduplicatedTiered
        );
    }

    #[test]
    fn test_chunk_config_for_medium_large_files() {
        // 5GB file should get 1MB chunks
        let config = PipelineRouter::optimal_chunk_config(5 * 1024 * 1024 * 1024);

        assert_eq!(config.min_size, 256 * 1024, "5GB file: min should be 256KB");
        assert_eq!(config.avg_size, 1024 * 1024, "5GB file: avg should be 1MB");
        assert_eq!(
            config.max_size,
            4 * 1024 * 1024,
            "5GB file: max should be 4MB"
        );
    }

    #[test]
    fn test_chunk_config_for_extra_large_files() {
        // 200GB file should get 4MB chunks
        let config = PipelineRouter::optimal_chunk_config(200 * 1024 * 1024 * 1024);

        assert_eq!(
            config.min_size,
            1024 * 1024,
            "200GB file: min should be 1MB"
        );
        assert_eq!(
            config.avg_size,
            4 * 1024 * 1024,
            "200GB file: avg should be 4MB"
        );
        assert_eq!(
            config.max_size,
            16 * 1024 * 1024,
            "200GB file: max should be 16MB"
        );
    }

    #[test]
    fn test_index_reduction_calculation() {
        // Verify the index reduction claims in the spec

        // Standard 64KB chunks for a 10TB file:
        // 10TB / 64KB = ~160 million chunks
        let standard_chunk_size = 64 * 1024;
        let file_size = 10u64 * 1024 * 1024 * 1024 * 1024; // 10TB
        let standard_chunks = file_size / standard_chunk_size;

        // Gigantor 4MB chunks:
        // 10TB / 4MB = ~2.5 million chunks (64x reduction)
        let gigantor_chunk_size = 4 * 1024 * 1024;
        let gigantor_chunks = file_size / gigantor_chunk_size;

        let reduction_factor = standard_chunks / gigantor_chunks;

        assert_eq!(reduction_factor, 64, "Should achieve 64x index reduction");
        assert!(
            gigantor_chunks < 3_000_000,
            "10TB file should produce < 3M chunks with Gigantor"
        );
        assert!(
            standard_chunks > 150_000_000,
            "10TB file would produce > 150M chunks with standard chunking"
        );
    }

    #[test]
    fn test_strategy_names() {
        assert_eq!(
            PipelineRouter::strategy_name(TransferStrategy::Direct),
            "Neutrino Fast Lane"
        );
        assert_eq!(
            PipelineRouter::strategy_name(TransferStrategy::DeduplicatedStandard),
            "Equilibrium Standard Lane"
        );
        assert_eq!(
            PipelineRouter::strategy_name(TransferStrategy::DeduplicatedTiered),
            "Gigantor Heavy Lift Lane"
        );
    }
}
