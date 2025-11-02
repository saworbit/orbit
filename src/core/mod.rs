/*!
 * Core file copy operations
 */

// Submodules - organized by responsibility
pub mod checksum;
pub mod resume;
pub mod metadata;
pub mod validation;
pub mod disk_guardian;
pub mod zero_copy;
pub mod retry;
pub mod transfer;
pub mod buffered;
pub mod bandwidth;
pub mod directory;
pub mod progress;
pub mod delta;

use std::path::Path;
use std::time::{Duration, Instant};

use crate::config::CopyConfig;
use crate::error::{OrbitError, Result};
use validation::should_copy_file;

/// Statistics about a copy operation
#[derive(Debug, Clone)]
pub struct CopyStats {
    pub bytes_copied: u64,
    pub duration: Duration,
    pub checksum: Option<String>,
    pub compression_ratio: Option<f64>,
    pub files_copied: u64,
    pub files_skipped: u64,
    pub files_failed: u64,
    pub delta_stats: Option<delta::DeltaStats>,
}

impl CopyStats {
    pub fn new() -> Self {
        Self {
            bytes_copied: 0,
            duration: Duration::ZERO,
            checksum: None,
            compression_ratio: None,
            files_copied: 0,
            files_skipped: 0,
            files_failed: 0,
            delta_stats: None,
        }
    }

    pub fn with_delta(mut self, delta_stats: delta::DeltaStats) -> Self {
        self.delta_stats = Some(delta_stats);
        self
    }
}

impl Default for CopyStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Copy a single file with all configured options
///
/// This is the main entry point for single file copy operations.
/// It handles:
/// - Source validation
/// - Copy mode checks (skip if exists, overwrite, etc.)
/// - Dry run mode
/// - Disk space validation
/// - Retry logic with exponential backoff
/// - Metadata preservation
///
/// The actual copy is delegated to the transfer module which decides between
/// compression, zero-copy, or buffered copy based on configuration.
///
/// If `publisher` is provided, progress events will be emitted. Otherwise, no events are emitted.
pub fn copy_file(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
) -> Result<CopyStats> {
    copy_file_impl(source_path, dest_path, config, None)
}

/// Internal implementation of copy_file with optional progress publisher
pub fn copy_file_impl(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
    publisher: Option<&progress::ProgressPublisher>,
) -> Result<CopyStats> {
    let start_time = Instant::now();

    // Validate source exists
    if !source_path.exists() {
        return Err(OrbitError::SourceNotFound(source_path.to_path_buf()));
    }

    let source_metadata = std::fs::metadata(source_path)?;
    let source_size = source_metadata.len();

    // Check if we should copy based on mode
    if !should_copy_file(source_path, dest_path, config.copy_mode)? {
        return Ok(CopyStats {
            bytes_copied: 0,
            duration: start_time.elapsed(),
            checksum: None,
            compression_ratio: None,
            files_copied: 0,
            files_skipped: 1,
            files_failed: 0,
            delta_stats: None,
        });
    }

    // Dry run mode
    if config.dry_run {
        println!("Would copy: {:?} -> {:?} ({} bytes)", source_path, dest_path, source_size);
        return Ok(CopyStats {
            bytes_copied: source_size,
            duration: start_time.elapsed(),
            checksum: None,
            compression_ratio: None,
            files_copied: 1,
            files_skipped: 0,
            files_failed: 0,
            delta_stats: None,
        });
    }

    // Validate disk space
    validation::validate_disk_space(dest_path, source_size)?;

    // Use provided publisher or create a no-op one
    let noop_publisher = progress::ProgressPublisher::noop();
    let pub_ref = publisher.unwrap_or(&noop_publisher);

    // Perform copy with retry logic and metadata preservation
    retry::with_retry_and_metadata(
        source_path,
        dest_path,
        config,
        || transfer::perform_copy(source_path, dest_path, source_size, config, pub_ref)
    )
}

/// Copy a directory recursively with streaming iteration to reduce memory usage
///
/// This delegates to the directory module which handles:
/// - Recursive directory walking
/// - Parallel file copying
/// - Memory-efficient streaming with bounded channels
/// - Symlink handling
/// - Error aggregation
pub use directory::{copy_directory, copy_directory_impl};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_copy_simple_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        std::fs::write(&source, b"test data").unwrap();

        let config = CopyConfig::default();
        let stats = copy_file(&source, &dest, &config).unwrap();

        assert_eq!(stats.bytes_copied, 9);
        assert_eq!(std::fs::read(&dest).unwrap(), b"test data");
    }

    #[test]
    fn test_copy_nonexistent_source() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("nonexistent.txt");
        let dest = dir.path().join("dest.txt");

        let config = CopyConfig::default();
        let result = copy_file(&source, &dest, &config);

        assert!(matches!(result, Err(OrbitError::SourceNotFound(_))));
    }

    #[test]
    fn test_zero_copy_small_file_skipped() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("small.txt");
        let dest = dir.path().join("dest.txt");

        // Small file (< 64KB) should not use zero-copy
        std::fs::write(&source, b"small").unwrap();

        let config = CopyConfig::default();
        let use_zc = zero_copy::should_use_zero_copy(&source, &dest, &config).unwrap();

        // Small files should skip zero-copy
        assert!(!use_zc);
    }
}
