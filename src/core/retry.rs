/*!
 * Retry logic with exponential backoff
 */

use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::config::CopyConfig;
use crate::error::{OrbitError, Result};
use super::CopyStats;

/// Execute a copy operation with retry logic and exponential backoff
pub fn with_retry<F>(
    config: &CopyConfig,
    mut operation: F,
) -> Result<CopyStats>
where
    F: FnMut() -> Result<CopyStats>,
{
    let mut attempt = 0;
    let mut last_error: Option<OrbitError> = None;

    while attempt <= config.retry_attempts {
        if attempt > 0 {
            let delay = if config.exponential_backoff {
                Duration::from_secs(config.retry_delay_secs * 2_u64.pow(attempt - 1))
            } else {
                Duration::from_secs(config.retry_delay_secs)
            };

            println!("Retry attempt {} of {} after {:?}...", attempt, config.retry_attempts, delay);
            thread::sleep(delay);
        }

        match operation() {
            Ok(stats) => return Ok(stats),
            Err(e) => {
                if e.is_fatal() {
                    return Err(e);
                }
                last_error = Some(e);
                attempt += 1;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| OrbitError::RetriesExhausted {
        attempts: config.retry_attempts
    }))
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
    let stats = with_retry(config, operation)?;

    // Preserve metadata if requested
    if config.preserve_metadata {
        if let Err(e) = super::metadata::preserve_metadata(source_path, dest_path) {
            eprintln!("Warning: Failed to preserve metadata: {}", e);
        }
    }

    Ok(stats)
}
