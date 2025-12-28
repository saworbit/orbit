/*!
 * Instrumentation for tracking operation statistics
 */

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::error::{ErrorCategory, OrbitError};

/// Thread-safe statistics tracker for operations
#[derive(Debug, Clone)]
pub struct OperationStats {
    inner: Arc<OperationStatsInner>,
}

#[derive(Debug)]
struct OperationStatsInner {
    // Operation counts
    total_operations: AtomicU64,
    successful_operations: AtomicU64,
    failed_operations: AtomicU64,
    skipped_operations: AtomicU64,

    // Retry statistics
    total_retries: AtomicU64,
    max_retries_for_single_op: AtomicU64,

    // Error categorization
    validation_errors: AtomicU64,
    io_errors: AtomicU64,
    network_errors: AtomicU64,
    resource_errors: AtomicU64,
    integrity_errors: AtomicU64,
    transient_errors: AtomicU64,
    fatal_errors: AtomicU64,

    // Timing
    start_time: Instant,
}

impl OperationStats {
    /// Create a new statistics tracker
    pub fn new() -> Self {
        Self {
            inner: Arc::new(OperationStatsInner {
                total_operations: AtomicU64::new(0),
                successful_operations: AtomicU64::new(0),
                failed_operations: AtomicU64::new(0),
                skipped_operations: AtomicU64::new(0),
                total_retries: AtomicU64::new(0),
                max_retries_for_single_op: AtomicU64::new(0),
                validation_errors: AtomicU64::new(0),
                io_errors: AtomicU64::new(0),
                network_errors: AtomicU64::new(0),
                resource_errors: AtomicU64::new(0),
                integrity_errors: AtomicU64::new(0),
                transient_errors: AtomicU64::new(0),
                fatal_errors: AtomicU64::new(0),
                start_time: Instant::now(),
            }),
        }
    }

    /// Record a successful operation
    pub fn record_success(&self) {
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
        self.inner
            .successful_operations
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed operation
    pub fn record_failure(&self, error: &OrbitError) {
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
        self.inner.failed_operations.fetch_add(1, Ordering::Relaxed);

        // Categorize the error
        match error.category() {
            ErrorCategory::Validation => {
                self.inner.validation_errors.fetch_add(1, Ordering::Relaxed)
            }
            ErrorCategory::IoError => self.inner.io_errors.fetch_add(1, Ordering::Relaxed),
            ErrorCategory::Network => self.inner.network_errors.fetch_add(1, Ordering::Relaxed),
            ErrorCategory::Resource => self.inner.resource_errors.fetch_add(1, Ordering::Relaxed),
            ErrorCategory::Integrity => self.inner.integrity_errors.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };

        if error.is_transient() {
            self.inner.transient_errors.fetch_add(1, Ordering::Relaxed);
        }

        if error.is_fatal() {
            self.inner.fatal_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a skipped operation
    pub fn record_skip(&self) {
        self.inner.total_operations.fetch_add(1, Ordering::Relaxed);
        self.inner
            .skipped_operations
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a retry attempt
    pub fn record_retry(&self, attempt_number: u32) {
        self.inner.total_retries.fetch_add(1, Ordering::Relaxed);

        // Update max retries if this is higher
        let current_max = self.inner.max_retries_for_single_op.load(Ordering::Relaxed);
        if (attempt_number as u64) > current_max {
            self.inner
                .max_retries_for_single_op
                .store(attempt_number as u64, Ordering::Relaxed);
        }
    }

    /// Get a snapshot of current statistics
    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            total_operations: self.inner.total_operations.load(Ordering::Relaxed),
            successful_operations: self.inner.successful_operations.load(Ordering::Relaxed),
            failed_operations: self.inner.failed_operations.load(Ordering::Relaxed),
            skipped_operations: self.inner.skipped_operations.load(Ordering::Relaxed),
            total_retries: self.inner.total_retries.load(Ordering::Relaxed),
            max_retries_for_single_op: self.inner.max_retries_for_single_op.load(Ordering::Relaxed),
            validation_errors: self.inner.validation_errors.load(Ordering::Relaxed),
            io_errors: self.inner.io_errors.load(Ordering::Relaxed),
            network_errors: self.inner.network_errors.load(Ordering::Relaxed),
            resource_errors: self.inner.resource_errors.load(Ordering::Relaxed),
            integrity_errors: self.inner.integrity_errors.load(Ordering::Relaxed),
            transient_errors: self.inner.transient_errors.load(Ordering::Relaxed),
            fatal_errors: self.inner.fatal_errors.load(Ordering::Relaxed),
            elapsed_secs: self.inner.start_time.elapsed().as_secs(),
        }
    }

    /// Emit statistics to stderr if any retries, failures, or notable events occurred.
    ///
    /// This method is called automatically when using default statistics tracking
    /// in copy operations. It only outputs if there's something noteworthy to report.
    ///
    /// Behavior can be controlled via the `ORBIT_STATS` environment variable:
    /// - `ORBIT_STATS=off` or `ORBIT_STATS=0` - Disable default emission
    /// - `ORBIT_STATS=verbose` - Always emit, even for successful operations with no retries
    pub fn emit(&self) {
        // Check if emission is disabled via environment variable
        if let Ok(val) = std::env::var("ORBIT_STATS") {
            let val_lower = val.to_lowercase();
            if val_lower == "off" || val_lower == "0" || val_lower == "false" {
                return;
            }
        }

        let snapshot = self.snapshot();

        // Determine if we should emit based on verbosity setting
        let verbose = std::env::var("ORBIT_STATS")
            .map(|v| v.to_lowercase() == "verbose")
            .unwrap_or(false);

        // Only emit if there's something noteworthy (retries, failures, skips)
        // or if verbose mode is enabled
        let has_noteworthy_events = snapshot.total_retries > 0
            || snapshot.failed_operations > 0
            || snapshot.skipped_operations > 0;

        if !has_noteworthy_events && !verbose {
            return;
        }

        // Emit to stderr with tracing if available, otherwise eprintln
        let summary = snapshot.format_summary();
        tracing::info!(target: "orbit::stats", "{}", summary);

        // Also emit to stderr for visibility in non-tracing contexts
        if snapshot.total_retries > 0 || snapshot.failed_operations > 0 {
            eprintln!(
                "[orbit] Retry metrics: {} retries, {} successful, {} failed, {} skipped",
                snapshot.total_retries,
                snapshot.successful_operations,
                snapshot.failed_operations,
                snapshot.skipped_operations
            );
        }
    }

    /// Check if any operations have been recorded
    pub fn has_activity(&self) -> bool {
        self.inner.total_operations.load(Ordering::Relaxed) > 0
    }
}

impl Default for OperationStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable snapshot of statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSnapshot {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub skipped_operations: u64,
    pub total_retries: u64,
    pub max_retries_for_single_op: u64,
    pub validation_errors: u64,
    pub io_errors: u64,
    pub network_errors: u64,
    pub resource_errors: u64,
    pub integrity_errors: u64,
    pub transient_errors: u64,
    pub fatal_errors: u64,
    pub elapsed_secs: u64,
}

impl StatsSnapshot {
    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            (self.successful_operations as f64 / self.total_operations as f64) * 100.0
        }
    }

    /// Get average retries per operation
    pub fn avg_retries_per_op(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.total_retries as f64 / self.total_operations as f64
        }
    }

    /// Format statistics as a human-readable string
    pub fn format_summary(&self) -> String {
        format!(
            "Operations: {} total, {} successful ({:.1}%), {} failed, {} skipped\n\
             Retries: {} total, {} max for single operation, {:.2} avg per operation\n\
             Errors: {} transient, {} fatal, {} validation, {} I/O, {} network, {} resource, {} integrity\n\
             Elapsed: {} seconds",
            self.total_operations,
            self.successful_operations,
            self.success_rate(),
            self.failed_operations,
            self.skipped_operations,
            self.total_retries,
            self.max_retries_for_single_op,
            self.avg_retries_per_op(),
            self.transient_errors,
            self.fatal_errors,
            self.validation_errors,
            self.io_errors,
            self.network_errors,
            self.resource_errors,
            self.integrity_errors,
            self.elapsed_secs
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_record_success() {
        let stats = OperationStats::new();
        stats.record_success();
        stats.record_success();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.total_operations, 2);
        assert_eq!(snapshot.successful_operations, 2);
        assert_eq!(snapshot.failed_operations, 0);
    }

    #[test]
    fn test_record_failure() {
        let stats = OperationStats::new();
        let error = OrbitError::Io(std::io::Error::other("test"));
        stats.record_failure(&error);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.total_operations, 1);
        assert_eq!(snapshot.failed_operations, 1);
        assert_eq!(snapshot.io_errors, 1);
    }

    #[test]
    fn test_record_skip() {
        let stats = OperationStats::new();
        stats.record_skip();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.total_operations, 1);
        assert_eq!(snapshot.skipped_operations, 1);
    }

    #[test]
    fn test_record_retry() {
        let stats = OperationStats::new();
        stats.record_retry(1);
        stats.record_retry(2);
        stats.record_retry(3);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.total_retries, 3);
        assert_eq!(snapshot.max_retries_for_single_op, 3);
    }

    #[test]
    fn test_error_categorization() {
        let stats = OperationStats::new();

        stats.record_failure(&OrbitError::SourceNotFound(PathBuf::from("/tmp")));
        stats.record_failure(&OrbitError::Io(std::io::Error::other(
            "test",
        )));
        stats.record_failure(&OrbitError::Protocol("network error".to_string()));

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.validation_errors, 1);
        assert_eq!(snapshot.io_errors, 1);
        assert_eq!(snapshot.network_errors, 1);
    }

    #[test]
    fn test_transient_fatal_tracking() {
        let stats = OperationStats::new();

        stats.record_failure(&OrbitError::Protocol("timeout".to_string()));
        stats.record_failure(&OrbitError::Authentication("invalid".to_string()));

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.transient_errors, 1);
        assert_eq!(snapshot.fatal_errors, 1);
    }

    #[test]
    fn test_success_rate() {
        let stats = OperationStats::new();
        stats.record_success();
        stats.record_success();
        stats.record_failure(&OrbitError::Other("test".to_string()));

        let snapshot = stats.snapshot();
        assert!((snapshot.success_rate() - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_avg_retries() {
        let stats = OperationStats::new();
        stats.record_success();
        stats.record_retry(1);
        stats.record_retry(2);
        stats.record_success();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.avg_retries_per_op(), 1.0);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let stats = OperationStats::new();
        let stats_clone = stats.clone();

        let handle = thread::spawn(move || {
            for _ in 0..100 {
                stats_clone.record_success();
            }
        });

        for _ in 0..100 {
            stats.record_success();
        }

        handle.join().unwrap();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.successful_operations, 200);
    }

    #[test]
    fn test_has_activity() {
        let stats = OperationStats::new();
        assert!(!stats.has_activity());

        stats.record_success();
        assert!(stats.has_activity());
    }

    #[test]
    fn test_emit_no_panic_on_success_only() {
        // Test that emit() doesn't panic when there are only successes
        let stats = OperationStats::new();
        stats.record_success();

        // Should not emit anything (no retries, failures, or skips)
        // but should not panic
        stats.emit();
    }

    #[test]
    fn test_emit_with_noteworthy_events() {
        let stats = OperationStats::new();
        stats.record_retry(1);
        stats.record_success();

        // emit() should work without panicking
        stats.emit();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.total_retries, 1);
    }

    #[test]
    fn test_emit_env_var_off() {
        // Disable emission via environment variable
        std::env::set_var("ORBIT_STATS", "off");

        let stats = OperationStats::new();
        stats.record_retry(1);
        stats.record_failure(&OrbitError::Other("test".to_string()));

        // Should not emit anything due to env var
        stats.emit();

        // Clean up
        std::env::remove_var("ORBIT_STATS");
    }
}
