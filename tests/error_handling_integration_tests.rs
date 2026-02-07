/*!
 * Integration tests for error handling, retries, and logging
 *
 * These tests simulate various failure scenarios to verify:
 * - Retry logic with exponential backoff
 * - Error categorization (transient vs fatal)
 * - Error handling modes (abort, skip, partial)
 * - Statistics tracking
 * - Logging integration
 */

use orbit::{
    config::{CopyConfig, ErrorMode},
    core::CopyStats,
    error::{ErrorCategory, OrbitError},
    instrumentation::OperationStats,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Simulated flaky operation that fails N times before succeeding
struct FlakyOperation {
    failures_before_success: Arc<AtomicU32>,
    current_attempt: Arc<AtomicU32>,
}

impl FlakyOperation {
    fn new(failures: u32) -> Self {
        Self {
            failures_before_success: Arc::new(AtomicU32::new(failures)),
            current_attempt: Arc::new(AtomicU32::new(0)),
        }
    }

    fn execute(&self) -> Result<CopyStats, OrbitError> {
        let attempt = self.current_attempt.fetch_add(1, Ordering::SeqCst);
        let failures_needed = self.failures_before_success.load(Ordering::SeqCst);

        if attempt < failures_needed {
            // Simulate transient error
            Err(OrbitError::Protocol(format!(
                "Simulated transient failure (attempt {})",
                attempt + 1
            )))
        } else {
            // Success
            Ok(CopyStats {
                bytes_copied: 1024,
                duration: Duration::from_millis(10),
                checksum: Some("abc123".to_string()),
                compression_ratio: None,
                files_copied: 1,
                files_skipped: 0,
                files_failed: 0,
                delta_stats: None,
                chunks_resumed: 0,
                bytes_skipped: 0,
            })
        }
    }
}

#[test]
fn test_retry_with_transient_errors() {
    // Create a flaky operation that fails twice before succeeding
    let flaky = FlakyOperation::new(2);

    let config = CopyConfig {
        retry_attempts: 3,
        retry_delay_secs: 0, // Zero delay for testing
        exponential_backoff: false,
        error_mode: ErrorMode::Partial, // Allow retries
        ..Default::default()
    };

    let start = Instant::now();
    let result = orbit::core::retry::with_retry(&config, || flaky.execute());
    let duration = start.elapsed();

    assert!(result.is_ok(), "Should succeed after retries");
    let stats = result.unwrap();
    assert_eq!(stats.bytes_copied, 1024);
    assert_eq!(stats.files_copied, 1);

    // Should have attempted 3 times total (2 failures + 1 success)
    assert_eq!(flaky.current_attempt.load(Ordering::SeqCst), 3);

    // Should be fast since no retry delay
    assert!(
        duration < Duration::from_secs(1),
        "Should complete quickly with zero delay"
    );
}

#[test]
fn test_retry_exhaustion() {
    // Create a flaky operation that always fails
    let flaky = FlakyOperation::new(100);

    let config = CopyConfig {
        retry_attempts: 3,
        retry_delay_secs: 0,
        exponential_backoff: false,
        error_mode: ErrorMode::Partial, // Allow retries
        ..Default::default()
    };

    let result = orbit::core::retry::with_retry(&config, || flaky.execute());

    assert!(result.is_err(), "Should fail after exhausting retries");

    // Should have attempted 4 times total (initial + 3 retries)
    assert_eq!(flaky.current_attempt.load(Ordering::SeqCst), 4);
}

#[test]
fn test_fatal_error_no_retry() {
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();

    let config = CopyConfig {
        retry_attempts: 5,
        retry_delay_secs: 0,
        exponential_backoff: false,
        error_mode: ErrorMode::Partial, // Allow retries (but fatal error should still stop)
        ..Default::default()
    };

    let result = orbit::core::retry::with_retry(&config, || {
        attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        Err(OrbitError::SourceNotFound(std::path::PathBuf::from(
            "/nonexistent",
        )))
    });

    assert!(result.is_err(), "Should fail immediately");

    // Should only attempt once (no retries for fatal errors)
    assert_eq!(attempt_count.load(Ordering::SeqCst), 1);

    if let Err(e) = result {
        assert!(e.is_fatal(), "Error should be marked as fatal");
        assert_eq!(e.category(), ErrorCategory::Validation);
    }
}

#[test]
fn test_exponential_backoff_timing() {
    let flaky = FlakyOperation::new(2);

    let config = CopyConfig {
        retry_attempts: 3,
        retry_delay_secs: 1, // 1 second base delay
        exponential_backoff: true,
        error_mode: ErrorMode::Partial, // Allow retries
        ..Default::default()
    };

    let start = Instant::now();
    let result = orbit::core::retry::with_retry(&config, || flaky.execute());
    let duration = start.elapsed();

    assert!(result.is_ok(), "Should succeed after retries");

    // With exponential backoff: 1s + 2s = 3s (plus jitter)
    // Allow some tolerance for jitter and system scheduling
    assert!(
        duration >= Duration::from_secs(2),
        "Should have exponential delays (got {:?})",
        duration
    );
    assert!(
        duration < Duration::from_secs(5),
        "Should not take too long (got {:?})",
        duration
    );
}

#[test]
fn test_error_mode_abort() {
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();

    let config = CopyConfig {
        retry_attempts: 5,
        retry_delay_secs: 0,
        exponential_backoff: false,
        error_mode: ErrorMode::Abort,
        ..Default::default()
    };

    let result = orbit::core::retry::with_retry(&config, || {
        attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        Err(OrbitError::Other("test error".to_string()))
    });

    assert!(result.is_err(), "Should fail in abort mode");

    // Should only attempt once in abort mode for non-fatal errors
    assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_error_mode_skip() {
    let config = CopyConfig {
        retry_attempts: 3,
        retry_delay_secs: 0,
        exponential_backoff: false,
        error_mode: ErrorMode::Skip,
        ..Default::default()
    };

    let result = orbit::core::retry::with_retry(&config, || {
        Err(OrbitError::Other("test error".to_string()))
    });

    assert!(result.is_ok(), "Should return Ok in skip mode");

    let stats = result.unwrap();
    assert_eq!(stats.bytes_copied, 0);
    assert_eq!(stats.files_copied, 0);
    assert_eq!(stats.files_skipped, 1);
    assert_eq!(stats.files_failed, 1);
}

#[test]
fn test_error_mode_partial_with_retry() {
    let flaky = FlakyOperation::new(2);

    let config = CopyConfig {
        retry_attempts: 3,
        retry_delay_secs: 0,
        exponential_backoff: false,
        error_mode: ErrorMode::Partial, // Allow retries
        ..Default::default()
    };

    let result = orbit::core::retry::with_retry(&config, || flaky.execute());

    assert!(
        result.is_ok(),
        "Should succeed with partial mode and retries"
    );

    // Should have retried and eventually succeeded
    assert_eq!(flaky.current_attempt.load(Ordering::SeqCst), 3);
}

#[test]
fn test_stats_tracking() {
    let stats_tracker = OperationStats::new();
    let flaky = FlakyOperation::new(2);

    let config = CopyConfig {
        retry_attempts: 3,
        retry_delay_secs: 0,
        exponential_backoff: false,
        error_mode: ErrorMode::Partial, // Allow retries
        ..Default::default()
    };

    let result =
        orbit::core::retry::with_retry_and_stats(&config, Some(&stats_tracker), || flaky.execute());

    assert!(result.is_ok(), "Should succeed");

    let snapshot = stats_tracker.snapshot();

    // Should have 1 successful operation
    assert_eq!(snapshot.successful_operations, 1);
    assert_eq!(snapshot.failed_operations, 0);

    // Should have recorded 2 retries
    assert_eq!(snapshot.total_retries, 2);
    assert_eq!(snapshot.max_retries_for_single_op, 2);

    // Success rate should be 100%
    assert_eq!(snapshot.success_rate(), 100.0);
}

#[test]
fn test_stats_tracking_with_failures() {
    let stats_tracker = OperationStats::new();

    let config = CopyConfig {
        retry_attempts: 2,
        retry_delay_secs: 0,
        exponential_backoff: false,
        error_mode: ErrorMode::Partial, // Allow retries for failure tests
        ..Default::default()
    };

    // First operation: succeeds immediately
    let result1 = orbit::core::retry::with_retry_and_stats(&config, Some(&stats_tracker), || {
        Ok(CopyStats {
            bytes_copied: 100,
            duration: Duration::from_millis(10),
            checksum: None,
            compression_ratio: None,
            files_copied: 1,
            files_skipped: 0,
            files_failed: 0,
            delta_stats: None,
            chunks_resumed: 0,
            bytes_skipped: 0,
        })
    });
    assert!(result1.is_ok());

    // Second operation: fails with I/O error
    let result2 = orbit::core::retry::with_retry_and_stats(&config, Some(&stats_tracker), || {
        Err(OrbitError::Io(std::io::Error::other("test")))
    });
    assert!(result2.is_err());

    // Third operation: fails with fatal error
    let result3 = orbit::core::retry::with_retry_and_stats(&config, Some(&stats_tracker), || {
        Err(OrbitError::Authentication("invalid".to_string()))
    });
    assert!(result3.is_err());

    let snapshot = stats_tracker.snapshot();

    // Should have 3 operations total
    assert_eq!(snapshot.total_operations, 3);
    assert_eq!(snapshot.successful_operations, 1);
    assert_eq!(snapshot.failed_operations, 2);

    // Should have tracked error categories
    assert_eq!(snapshot.io_errors, 1);
    assert_eq!(snapshot.fatal_errors, 1);

    // Success rate should be ~33%
    assert!((snapshot.success_rate() - 33.33).abs() < 1.0);
}

#[test]
fn test_error_categorization() {
    // Test various error categories
    let validation_error = OrbitError::SourceNotFound(std::path::PathBuf::from("/tmp"));
    assert_eq!(validation_error.category(), ErrorCategory::Validation);
    assert!(validation_error.is_fatal());
    assert!(!validation_error.is_transient());

    let io_error = OrbitError::Io(std::io::Error::other("test"));
    assert_eq!(io_error.category(), ErrorCategory::IoError);
    assert!(!io_error.is_fatal());

    let network_error = OrbitError::Protocol("timeout".to_string());
    assert_eq!(network_error.category(), ErrorCategory::Network);
    assert!(!network_error.is_fatal());
    assert!(network_error.is_transient());
    assert!(network_error.is_network_error());

    let integrity_error = OrbitError::ChecksumMismatch {
        expected: "abc".to_string(),
        actual: "def".to_string(),
    };
    assert_eq!(integrity_error.category(), ErrorCategory::Integrity);
    assert!(integrity_error.is_fatal());
    assert!(!integrity_error.is_transient());
}

#[test]
fn test_transient_io_errors() {
    // Test that certain I/O error kinds are considered transient
    let transient_kinds = [
        std::io::ErrorKind::ConnectionRefused,
        std::io::ErrorKind::ConnectionReset,
        std::io::ErrorKind::TimedOut,
        std::io::ErrorKind::Interrupted,
    ];

    for kind in &transient_kinds {
        let error = OrbitError::Io(std::io::Error::new(*kind, "test"));
        assert!(error.is_transient(), "{:?} should be transient", kind);

        // Only certain kinds are network errors
        match kind {
            std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::TimedOut => {
                assert!(
                    error.is_network_error(),
                    "{:?} should be network error",
                    kind
                );
            }
            _ => {}
        }
    }
}

#[test]
fn test_stats_snapshot_formatting() {
    let stats = OperationStats::new();

    // Record some operations
    stats.record_success();
    stats.record_success();
    stats.record_failure(&OrbitError::Io(std::io::Error::other("test")));
    stats.record_retry(1);
    stats.record_retry(2);
    stats.record_skip();

    let snapshot = stats.snapshot();

    // Verify basic stats
    assert_eq!(snapshot.total_operations, 4);
    assert_eq!(snapshot.successful_operations, 2);
    assert_eq!(snapshot.failed_operations, 1);
    assert_eq!(snapshot.skipped_operations, 1);
    assert_eq!(snapshot.total_retries, 2);

    // Test formatted summary
    let summary = snapshot.format_summary();
    assert!(summary.contains("4 total"));
    assert!(summary.contains("2 successful"));
    assert!(summary.contains("1 failed"));
    assert!(summary.contains("1 skipped"));
    assert!(summary.contains("2 total"));
}

#[test]
fn test_multiple_error_types_in_sequence() {
    let stats_tracker = OperationStats::new();
    let attempt_count = Arc::new(AtomicU32::new(0));

    let config = CopyConfig {
        retry_attempts: 5,
        retry_delay_secs: 0,
        exponential_backoff: false,
        error_mode: ErrorMode::Partial, // Allow retries
        ..Default::default()
    };

    // Simulate a sequence of different error types
    let attempt_count_clone = attempt_count.clone();
    let result = orbit::core::retry::with_retry_and_stats(&config, Some(&stats_tracker), || {
        let attempt = attempt_count_clone.fetch_add(1, Ordering::SeqCst);

        match attempt {
            0 => Err(OrbitError::Protocol("network timeout".to_string())),
            1 => Err(OrbitError::Protocol(
                "transient connection reset".to_string(),
            )),
            2 => Ok(CopyStats {
                bytes_copied: 2048,
                duration: Duration::from_millis(20),
                checksum: Some("def456".to_string()),
                compression_ratio: Some(0.5),
                files_copied: 1,
                files_skipped: 0,
                files_failed: 0,
                delta_stats: None,
                chunks_resumed: 0,
                bytes_skipped: 0,
            }),
            _ => unreachable!(),
        }
    });

    assert!(
        result.is_ok(),
        "Should succeed after encountering multiple error types"
    );
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);

    let snapshot = stats_tracker.snapshot();
    assert_eq!(snapshot.successful_operations, 1);
    assert_eq!(snapshot.total_retries, 2);
}

#[test]
fn test_concurrent_stats_tracking() {
    use std::thread;

    let stats = OperationStats::new();
    let mut handles = vec![];

    // Spawn multiple threads to record stats concurrently
    for i in 0..10 {
        let stats_clone = stats.clone();
        let handle = thread::spawn(move || {
            for j in 0..10 {
                if (i + j) % 2 == 0 {
                    stats_clone.record_success();
                } else {
                    stats_clone.record_failure(&OrbitError::Other("test".to_string()));
                }
                stats_clone.record_retry(j);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let snapshot = stats.snapshot();

    // Should have recorded 100 operations total (10 threads * 10 operations)
    assert_eq!(snapshot.total_operations, 100);
    assert_eq!(
        snapshot.successful_operations + snapshot.failed_operations,
        100
    );
    // Each thread calls record_retry 10 times (j=0 to 9), so 10 threads * 10 calls = 100 retries
    assert_eq!(snapshot.total_retries, 100);
    // Max retry for single operation should be 9 (the highest j value)
    assert_eq!(snapshot.max_retries_for_single_op, 9);
}

#[test]
fn test_copy_file_with_stats_tracking() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    std::fs::write(&source, b"test data for stats tracking").unwrap();

    let stats = OperationStats::new();
    let config = orbit::config::CopyConfig::default();

    let result = orbit::copy_file_with_stats(&source, &dest, &config, Some(&stats));
    assert!(result.is_ok());

    let snapshot = stats.snapshot();
    // Should have recorded 1 successful operation
    assert_eq!(snapshot.successful_operations, 1);
    assert_eq!(snapshot.failed_operations, 0);
    assert_eq!(snapshot.total_retries, 0); // No retries needed for successful copy
}

#[test]
fn test_copy_file_default_stats_success() {
    use tempfile::tempdir;

    // Set environment to disable emission during test to avoid output noise
    std::env::set_var("ORBIT_STATS", "off");

    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    std::fs::write(&source, b"test data").unwrap();

    let config = orbit::config::CopyConfig::default();

    // This uses default stats tracking internally
    let result = orbit::copy_file(&source, &dest, &config);
    assert!(result.is_ok());

    let stats = result.unwrap();
    assert_eq!(stats.bytes_copied, 9);
    assert_eq!(stats.files_copied, 1);

    // Clean up env var
    std::env::remove_var("ORBIT_STATS");
}

#[test]
fn test_stats_emit_only_on_noteworthy_events() {
    let stats = OperationStats::new();

    // Record only successful operations - emit() should not produce stderr output
    stats.record_success();

    let snapshot = stats.snapshot();
    assert_eq!(snapshot.total_retries, 0);
    assert_eq!(snapshot.failed_operations, 0);
    assert_eq!(snapshot.skipped_operations, 0);

    // This should be safe to call - it won't emit because there are no noteworthy events
    // (unless ORBIT_STATS=verbose is set)
    stats.emit();
}

#[test]
fn test_stats_emit_with_retries() {
    let stats = OperationStats::new();

    // Record some retries and a success
    stats.record_retry(1);
    stats.record_retry(2);
    stats.record_success();

    let snapshot = stats.snapshot();
    assert_eq!(snapshot.total_retries, 2);
    assert_eq!(snapshot.successful_operations, 1);

    // emit() will output because there are retries
    // For testing purposes, we just verify the method doesn't panic
    stats.emit();
}

#[test]
fn test_stats_has_activity() {
    let stats = OperationStats::new();

    // Initially no activity
    assert!(!stats.has_activity());

    // Record an operation
    stats.record_success();
    assert!(stats.has_activity());
}

#[test]
fn test_aggregated_stats_across_multiple_files() {
    use tempfile::tempdir;

    // Set environment to disable emission during test
    std::env::set_var("ORBIT_STATS", "off");

    let dir = tempdir().unwrap();
    let stats = OperationStats::new();
    let config = orbit::config::CopyConfig::default();

    // Copy multiple files using the same stats tracker
    for i in 0..5 {
        let source = dir.path().join(format!("source_{}.txt", i));
        let dest = dir.path().join(format!("dest_{}.txt", i));
        std::fs::write(&source, format!("content {}", i)).unwrap();

        let result = orbit::copy_file_with_stats(&source, &dest, &config, Some(&stats));
        assert!(result.is_ok());
    }

    let snapshot = stats.snapshot();
    // Should have recorded 5 successful operations
    assert_eq!(snapshot.successful_operations, 5);
    assert_eq!(snapshot.total_operations, 5);
    assert_eq!(snapshot.failed_operations, 0);

    // Clean up env var
    std::env::remove_var("ORBIT_STATS");
}
