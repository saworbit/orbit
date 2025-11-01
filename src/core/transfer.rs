/*!
 * Direct copy orchestration - dispatches to compression/zero-copy/buffered implementations
 */

use std::path::Path;

use crate::config::{CopyConfig, CompressionType};
use crate::error::{OrbitError, Result};
use crate::compression;
use super::CopyStats;
use super::zero_copy;
use super::buffered;

/// Internal copy implementation (called by retry logic)
///
/// Dispatches to appropriate copy method based on configuration:
/// - Compression (LZ4/Zstd) if enabled
/// - Zero-copy optimization if favorable
/// - Buffered copy as fallback
pub fn perform_copy(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
) -> Result<CopyStats> {
    match config.compression {
        CompressionType::None => {
            copy_direct(source_path, dest_path, source_size, config)
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
) -> Result<CopyStats> {
    // Determine if we should attempt zero-copy
    let use_zero_copy = zero_copy::should_use_zero_copy(source_path, dest_path, config)?;

    if use_zero_copy {
        // Try zero-copy first
        match zero_copy::try_zero_copy_direct(source_path, dest_path, source_size, config) {
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
    buffered::copy_buffered(source_path, dest_path, source_size, config)
}
