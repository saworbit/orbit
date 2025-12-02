/*!
 * Retry logic with exponential backoff
 */

use std::path::Path;
use std::thread;
use std::time::Duration;

use tracing::{debug, error as log_error, info, instrument, warn};

use super::CopyStats;
use crate::config::{CopyConfig, ErrorMode};
use crate::error::{OrbitError, Result};
use crate::instrumentation::OperationStats;

/// Execute a copy operation with retry logic and exponential backoff
pub fn with_retry<F>(config: &CopyConfig, operation: F) -> Result<CopyStats>
where
    F: FnMut() -> Result<CopyStats>,
{
    with_retry_and_stats(config, None, operation)
}

/// Execute a copy operation with retry logic, exponential backoff, and statistics tracking
#[instrument(skip(operation, stats))]
pub fn with_retry_and_stats<F>(
    config: &CopyConfig,
    stats: Option<&OperationStats>,
    mut operation: F,
) -> Result<CopyStats>
where
    F: FnMut() -> Result<CopyStats>,
{
    let mut attempt = 0;
    let mut last_error: Option<OrbitError> = None;

    while attempt <= config.retry_attempts {
        if attempt > 0 {
            let delay = calculate_backoff_delay(config, attempt);

            if let Some(ref err) = last_error {
                warn!(
                    "Retry attempt {} of {} after {:?} (previous error: {})",
                    attempt, config.retry_attempts, delay, err
                );
            }

            // Record retry in stats
            if let Some(stats_tracker) = stats {
                stats_tracker.record_retry(attempt);
            }

            thread::sleep(delay);
        } else {
            debug!("Starting operation (attempt 1)");
        }

        match operation() {
            Ok(copy_stats) => {
                if attempt > 0 {
                    info!("Operation succeeded after {} retries", attempt);
                }

                // Record success
                if let Some(stats_tracker) = stats {
                    stats_tracker.record_success();
                }

                return Ok(copy_stats);
            }
            Err(e) => {
                let is_permanent = !e.is_transient();

                log_error!(
                    "Operation failed on attempt {}: {} (category: {}, fatal: {}, transient: {})",
                    attempt + 1,
                    e,
                    e.category(),
                    e.is_fatal(),
                    e.is_transient()
                );

                // 1. FATAL ERROR CHECK
                if e.is_fatal() {
                    log_error!("Fatal error encountered, aborting retries");
                    if let Some(stats_tracker) = stats {
                        stats_tracker.record_failure(&e);
                    }
                    return Err(e);
                }

                // 2. PERMANENT ERROR CHECK (Optimization)
                // If the error is not transient (e.g. PermissionDenied), we should not retry it
                // regardless of the ErrorMode, because the result will not change.
                if is_permanent {
                    warn!("Error is permanent (non-transient), aborting retries to save time");

                    if let Some(stats_tracker) = stats {
                        stats_tracker.record_failure(&e);
                    }

                    // Respect the flow control of ErrorMode, but force a stop
                    match config.error_mode {
                        ErrorMode::Skip => {
                            warn!("Skipping permanent failure");
                            if let Some(stats_tracker) = stats {
                                stats_tracker.record_skip();
                            }
                            return Ok(CopyStats::skipped());
                        }
                        _ => return Err(e), // Abort or Partial treated as failure
                    }
                }

                // 3. TRANSIENT ERROR HANDLING
                // Check error mode for non-fatal, transient errors
                match config.error_mode {
                    ErrorMode::Abort => {
                        debug!("Error mode is Abort, stopping on error");

                        if let Some(stats_tracker) = stats {
                            stats_tracker.record_failure(&e);
                        }

                        return Err(e);
                    }
                    ErrorMode::Skip => {
                        warn!("Error mode is Skip, skipping failed operation");

                        if let Some(stats_tracker) = stats {
                            stats_tracker.record_skip();
                        }

                        return Ok(CopyStats::skipped());
                    }
                    ErrorMode::Partial => {
                        debug!("Error mode is Partial, will retry and keep partial files");
                        // Continue to retry logic below
                    }
                }

                last_error = Some(e);
                attempt += 1;
            }
        }
    }

    // All retries exhausted
    log_error!(
        "All {} retry attempts exhausted. Last error: {}",
        config.retry_attempts,
        last_error
            .as_ref()
            .map_or("none".to_string(), |e| e.to_string())
    );

    // Record the actual error that caused the failure, not the RetriesExhausted wrapper
    if let Some(stats_tracker) = stats {
        if let Some(ref err) = last_error {
            stats_tracker.record_failure(err);
        } else {
            let exhausted_error = OrbitError::RetriesExhausted {
                attempts: config.retry_attempts,
            };
            stats_tracker.record_failure(&exhausted_error);
        }
    }

    Err(last_error.unwrap_or(OrbitError::RetriesExhausted {
        attempts: config.retry_attempts,
    }))
}

/// Calculate backoff delay based on configuration
fn calculate_backoff_delay(config: &CopyConfig, attempt: u32) -> Duration {
    if config.exponential_backoff {
        // Exponential backoff with jitter to avoid thundering herd
        let base_delay = config.retry_delay_secs * 2_u64.pow(attempt.saturating_sub(1));
        // Cap at 5 minutes
        let capped_delay = base_delay.min(300);

        // Add up to 20% jitter (only if delay > 0)
        let jitter = if capped_delay > 0 {
            let jitter_ms = (capped_delay * 200) / 1000; // 20% in milliseconds
            if jitter_ms > 0 {
                rand::random::<u64>() % jitter_ms
            } else {
                0
            }
        } else {
            0
        };

        Duration::from_millis(capped_delay * 1000 + jitter)
    } else {
        Duration::from_secs(config.retry_delay_secs)
    }
}

/// Execute a copy operation with retry logic, preserving metadata on success
pub fn with_retry_and_metadata<F>(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
    operation: F,
) -> Result<CopyStats>
where
    F: FnMut() -> Result<CopyStats>,
{
    with_retry_and_metadata_stats(source_path, dest_path, config, None, operation)
}

/// Execute a copy operation with retry logic, metadata preservation, and statistics tracking
pub fn with_retry_and_metadata_stats<F>(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
    stats: Option<&OperationStats>,
    operation: F,
) -> Result<CopyStats>
where
    F: FnMut() -> Result<CopyStats>,
{
    let copy_stats = with_retry_and_stats(config, stats, operation)?;

    // Preserve metadata if requested
    if config.preserve_metadata {
        if let Err(e) = super::metadata::preserve_metadata(source_path, dest_path) {
            if config.strict_metadata {
                log_error!("Failed to preserve metadata (strict mode): {}", e);
                return Err(OrbitError::MetadataFailed(format!(
                    "Failed to preserve metadata: {}",
                    e
                )));
            } else {
                warn!("Failed to preserve metadata: {}", e);
            }
        } else {
            debug!("Metadata preserved successfully");
        }
    }

    Ok(copy_stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_backoff_delay() {
        let config = CopyConfig {
            retry_delay_secs: 1,
            exponential_backoff: true,
            ..Default::default()
        };

        let delay1 = calculate_backoff_delay(&config, 1);
        let delay2 = calculate_backoff_delay(&config, 2);
        let delay3 = calculate_backoff_delay(&config, 3);

        // Should increase exponentially (with jitter)
        assert!(delay1.as_millis() >= 1000); // ~1s + jitter
        assert!(delay2.as_millis() >= 2000); // ~2s + jitter
        assert!(delay3.as_millis() >= 4000); // ~4s + jitter
    }

    #[test]
    fn test_fixed_backoff_delay() {
        let config = CopyConfig {
            retry_delay_secs: 2,
            exponential_backoff: false,
            ..Default::default()
        };

        let delay1 = calculate_backoff_delay(&config, 1);
        let delay2 = calculate_backoff_delay(&config, 2);

        assert_eq!(delay1, Duration::from_secs(2));
        assert_eq!(delay2, Duration::from_secs(2));
    }

    #[test]
    fn test_error_mode_abort() {
        let config = CopyConfig {
            error_mode: ErrorMode::Abort,
            retry_attempts: 3,
            ..Default::default()
        };

        let mut attempts = 0;
        let result = with_retry(&config, || {
            attempts += 1;
            Err(OrbitError::Other("test error".to_string()))
        });

        assert!(result.is_err());
        // Should abort on first non-fatal error with Abort mode
        assert_eq!(attempts, 1);
    }

    #[test]
    fn test_error_mode_skip() {
        let config = CopyConfig {
            error_mode: ErrorMode::Skip,
            retry_attempts: 3,
            ..Default::default()
        };

        let result = with_retry(&config, || Err(OrbitError::Other("test error".to_string())));

        // Should return Ok with zero stats when skipping
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.bytes_copied, 0);
        assert_eq!(stats.files_copied, 0);
        assert_eq!(stats.files_skipped, 1);
        assert_eq!(stats.files_failed, 1);
    }
}
