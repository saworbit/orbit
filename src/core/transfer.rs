/*!
 * Direct copy orchestration - dispatches to compression/zero-copy/buffered implementations
 */

use std::path::Path;
use std::time::Instant;

use super::buffered;
use super::delta::{self, DeltaConfig};
use super::progress::ProgressPublisher;
use super::validation;
use super::zero_copy;
use super::CopyStats;
use crate::compression;
use crate::config::{CompressionType, CopyConfig};
use crate::error::{OrbitError, Result};

/// Internal copy implementation (called by retry logic)
///
/// Dispatches to appropriate copy method based on configuration:
/// - Compression (LZ4/Zstd) if enabled
/// - Zero-copy optimization if favorable
/// - Buffered copy as fallback
///
/// Progress events are emitted through the provided publisher.
pub fn perform_copy(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
    publisher: &ProgressPublisher,
) -> Result<CopyStats> {
    match config.compression {
        CompressionType::None => {
            copy_direct(source_path, dest_path, source_size, config, publisher)
        }
        CompressionType::Lz4 => {
            compression::copy_with_lz4(source_path, dest_path, source_size, config)
        }
        CompressionType::Zstd { level } => {
            compression::copy_with_zstd(source_path, dest_path, source_size, level, config)
        }
    }
}

/// Direct copy without compression (with optional zero-copy optimization)
///
/// Decision tree:
/// 1. Check if zero-copy heuristics are favorable
/// 2. If yes, attempt zero-copy (fall back to buffered on unsupported)
/// 3. If no, use buffered copy directly
fn copy_direct(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
    publisher: &ProgressPublisher,
) -> Result<CopyStats> {
    // Check if delta transfer should be used
    if validation::should_use_delta_transfer(source_path, dest_path, config)? {
        return copy_with_delta_integration(source_path, dest_path, source_size, config);
    }

    // Determine if we should attempt zero-copy
    let use_zero_copy = zero_copy::should_use_zero_copy(source_path, dest_path, config)?;

    if use_zero_copy {
        // Try zero-copy first
        match zero_copy::try_zero_copy_direct(
            source_path,
            dest_path,
            source_size,
            config,
            publisher,
        ) {
            Ok(stats) => {
                if config.show_progress {
                    println!("✓ Zero-copy transfer completed");
                }
                return Ok(stats);
            }
            Err(OrbitError::ZeroCopyUnsupported) => {
                if config.show_progress {
                    println!("Zero-copy not supported, using buffered copy");
                }
                // Fall through to buffered copy
            }
            Err(e) => {
                // Other errors should be returned
                return Err(e);
            }
        }
    }

    // Use buffered copy (either as fallback or by default)
    buffered::copy_buffered(source_path, dest_path, source_size, config, publisher)
}

/// Perform delta transfer and convert to CopyStats
fn copy_with_delta_integration(
    source_path: &Path,
    dest_path: &Path,
    _source_size: u64,
    config: &CopyConfig,
) -> Result<CopyStats> {
    let start = Instant::now();

    let delta_config = DeltaConfig {
        check_mode: config.check_mode,
        block_size: config.delta_block_size,
        whole_file: config.whole_file,
        update_manifest: config.update_manifest,
        ignore_existing: config.ignore_existing,
        hash_algorithm: config.delta_hash_algorithm,
        rolling_hash_algo: delta::RollingHashAlgo::Gear64,
        parallel_hashing: config.parallel_hashing,
        manifest_path: config.delta_manifest_path.clone(),
        resume_enabled: config.delta_resume_enabled,
        chunk_size: config.delta_chunk_size,
    };

    let (delta_stats, checksum) =
        delta::copy_with_delta_fallback(source_path, dest_path, &delta_config)?;

    let duration = start.elapsed();

    if config.show_progress {
        println!("✓ Delta transfer: {}", delta_stats);
    }

    // Extract resume metrics from delta stats
    let chunks_resumed = delta_stats.chunks_resumed;
    let bytes_skipped = delta_stats.bytes_skipped;

    Ok(CopyStats {
        bytes_copied: delta_stats.bytes_transferred,
        duration,
        checksum,
        compression_ratio: None,
        files_copied: 1,
        files_skipped: 0,
        files_failed: 0,
        delta_stats: Some(delta_stats),
        chunks_resumed,
        bytes_skipped,
    })
}
