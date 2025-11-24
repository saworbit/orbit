/*!
 * Core file copy operations
 */

// Submodules - organized by responsibility
pub mod bandwidth;
pub mod buffered;
pub mod checksum;
pub mod concurrency;
pub mod delta;
pub mod directory;
pub mod disk_guardian;
pub mod dry_run; // Dry-run simulation mode
pub mod enhanced_progress; // Multi-transfer progress bars with indicatif
pub mod file_metadata; // Comprehensive metadata preservation
pub mod filter; // Include/exclude filter patterns
pub mod metadata;
pub mod metadata_ops; // Metadata preservation orchestration
pub mod progress;
pub mod resilient_sync; // Crash-proof sync with Magnetar integration
pub mod resume;
pub mod retry;
pub mod transfer;
pub mod transform; // Metadata and path transformation
pub mod validation;
pub mod zero_copy; // Concurrency control with semaphore

use std::path::Path;
use std::time::{Duration, Instant};

use crate::audit::AuditLogger;
use crate::config::CopyConfig;
use crate::error::{OrbitError, Result};
use crate::instrumentation::OperationStats;
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
    /// Number of chunks resumed from partial manifest (delta resume)
    pub chunks_resumed: u64,
    /// Bytes skipped due to resume (already processed)
    pub bytes_skipped: u64,
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
            chunks_resumed: 0,
            bytes_skipped: 0,
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
/// - **Default statistics tracking** (retry metrics emitted automatically)
///
/// The actual copy is delegated to the transfer module which decides between
/// compression, zero-copy, or buffered copy based on configuration.
///
/// # Statistics Tracking
///
/// Retry metrics (attempts, successes, failures) are collected and emitted by default.
/// This enhances observability for data migration, transport, and storage operations.
///
/// To customize statistics tracking, use [`copy_file_with_stats`] or [`copy_file_impl`].
///
/// To disable default emission, set `ORBIT_STATS=off` environment variable.
pub fn copy_file(source_path: &Path, dest_path: &Path, config: &CopyConfig) -> Result<CopyStats> {
    copy_file_with_stats(source_path, dest_path, config, None)
}

/// Copy a single file with optional custom statistics tracking
///
/// This variant allows passing a custom `OperationStats` instance for aggregated
/// metrics across multiple operations (e.g., batch/directory copies).
///
/// # Arguments
///
/// * `source_path` - Path to the source file
/// * `dest_path` - Path to the destination file
/// * `config` - Copy configuration options
/// * `stats` - Optional statistics tracker. If `None`, a default tracker is created
///   and metrics are emitted after the operation completes.
///
/// # Example
///
/// ```ignore
/// use orbit::{CopyConfig, OperationStats, copy_file_with_stats};
///
/// // For aggregated stats across multiple files:
/// let stats = OperationStats::new();
/// for file in &files {
///     copy_file_with_stats(&file.src, &file.dest, &config, Some(&stats))?;
/// }
/// stats.emit(); // Emit once after all operations
/// ```
pub fn copy_file_with_stats(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
    stats: Option<&OperationStats>,
) -> Result<CopyStats> {
    copy_file_impl_with_stats(source_path, dest_path, config, None, stats)
}

/// Internal implementation of copy_file with optional progress publisher
pub fn copy_file_impl(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
    publisher: Option<&progress::ProgressPublisher>,
) -> Result<CopyStats> {
    copy_file_impl_with_stats(source_path, dest_path, config, publisher, None)
}

/// Internal implementation of copy_file with optional progress publisher and statistics tracking
pub fn copy_file_impl_with_stats(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
    publisher: Option<&progress::ProgressPublisher>,
    stats: Option<&OperationStats>,
) -> Result<CopyStats> {
    // Create default stats tracker if none provided
    let default_stats = OperationStats::new();
    let (stats_ref, use_default) = match stats {
        Some(s) => (s, false),
        None => (&default_stats, true),
    };

    let result = copy_file_impl_inner(source_path, dest_path, config, publisher, stats_ref);

    // If using default stats (none provided), emit metrics automatically
    if use_default {
        default_stats.emit();
    }

    result
}

/// Core implementation of copy_file with all parameters
fn copy_file_impl_inner(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
    publisher: Option<&progress::ProgressPublisher>,
    stats: &OperationStats,
) -> Result<CopyStats> {
    let start_time = Instant::now();

    // Generate a unique job ID for audit correlation
    let job_id = generate_job_id();

    // Initialize audit logger if audit log path is configured
    let mut audit_logger = if config.audit_log_path.is_some() || config.verbose {
        match AuditLogger::new(config.audit_log_path.as_deref(), config.audit_format) {
            Ok(logger) => Some(logger),
            Err(e) => {
                // Log warning but don't fail the copy operation
                tracing::warn!("Failed to initialize audit logger: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Validate source exists
    if !source_path.exists() {
        let err = OrbitError::SourceNotFound(source_path.to_path_buf());
        // Emit failure audit event
        if let Some(ref mut logger) = audit_logger {
            let _ = logger.emit_failure(
                &job_id,
                source_path,
                dest_path,
                "local",
                0,
                start_time.elapsed().as_millis() as u64,
                0,
                &err.to_string(),
            );
        }
        return Err(err);
    }

    let source_metadata = std::fs::metadata(source_path)?;
    let source_size = source_metadata.len();

    // Emit start audit event
    if let Some(ref mut logger) = audit_logger {
        if let Err(e) = logger.emit_start(&job_id, source_path, dest_path, "local", source_size) {
            tracing::warn!("Failed to emit audit start event: {}", e);
        }
    }

    // Check if we should copy based on mode
    if !should_copy_file(source_path, dest_path, config.copy_mode)? {
        let stats = CopyStats {
            bytes_copied: 0,
            duration: start_time.elapsed(),
            checksum: None,
            compression_ratio: None,
            files_copied: 0,
            files_skipped: 1,
            files_failed: 0,
            delta_stats: None,
            chunks_resumed: 0,
            bytes_skipped: 0,
        };

        // Emit skip audit event
        if let Some(ref mut logger) = audit_logger {
            let _ = logger.emit_from_stats(
                &job_id,
                source_path,
                dest_path,
                "local",
                &stats,
                config.compression,
                0,
                None,
            );
        }

        return Ok(stats);
    }

    // Dry run mode
    if config.dry_run {
        println!(
            "Would copy: {:?} -> {:?} ({} bytes)",
            source_path, dest_path, source_size
        );
        let stats = CopyStats {
            bytes_copied: source_size,
            duration: start_time.elapsed(),
            checksum: None,
            compression_ratio: None,
            files_copied: 1,
            files_skipped: 0,
            files_failed: 0,
            delta_stats: None,
            chunks_resumed: 0,
            bytes_skipped: 0,
        };

        // Emit dry-run audit event
        if let Some(ref mut logger) = audit_logger {
            let _ = logger.emit_from_stats(
                &job_id,
                source_path,
                dest_path,
                "local",
                &stats,
                config.compression,
                0,
                None,
            );
        }

        return Ok(stats);
    }

    // Validate disk space
    if let Err(e) = validation::validate_disk_space(dest_path, source_size) {
        if let Some(ref mut logger) = audit_logger {
            let _ = logger.emit_failure(
                &job_id,
                source_path,
                dest_path,
                "local",
                0,
                start_time.elapsed().as_millis() as u64,
                0,
                &e.to_string(),
            );
        }
        return Err(e);
    }

    // Use provided publisher or create a no-op one
    let noop_publisher = progress::ProgressPublisher::noop();
    let pub_ref = publisher.unwrap_or(&noop_publisher);

    // Perform copy with retry logic, metadata preservation, and statistics tracking
    let result =
        retry::with_retry_and_metadata_stats(source_path, dest_path, config, Some(stats), || {
            transfer::perform_copy(source_path, dest_path, source_size, config, pub_ref)
        });

    // Emit completion audit event
    if let Some(ref mut logger) = audit_logger {
        match &result {
            Ok(stats) => {
                let _ = logger.emit_from_stats(
                    &job_id,
                    source_path,
                    dest_path,
                    "local",
                    stats,
                    config.compression,
                    0,
                    None,
                );
            }
            Err(e) => {
                let stats = CopyStats {
                    bytes_copied: 0,
                    duration: start_time.elapsed(),
                    checksum: None,
                    compression_ratio: None,
                    files_copied: 0,
                    files_skipped: 0,
                    files_failed: 1,
                    delta_stats: None,
                    chunks_resumed: 0,
                    bytes_skipped: 0,
                };
                let _ = logger.emit_from_stats(
                    &job_id,
                    source_path,
                    dest_path,
                    "local",
                    &stats,
                    config.compression,
                    config.retry_attempts,
                    Some(&e.to_string()),
                );
            }
        }
    }

    result
}

/// Generate a unique job ID for audit event correlation
fn generate_job_id() -> String {
    use std::time::SystemTime;

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Simple but unique job ID combining timestamp and random component
    format!("orbit-{:x}-{:04x}", timestamp, rand::random::<u16>())
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
