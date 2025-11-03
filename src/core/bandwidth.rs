/*!
 * Bandwidth throttling utilities with token bucket rate limiting
 */

use governor::{Quota, RateLimiter as GovernorRateLimiter, clock::DefaultClock, state::{InMemoryState, NotKeyed}};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Bandwidth rate limiter using token bucket algorithm
#[derive(Clone)]
pub struct BandwidthLimiter {
    limiter: Option<Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    bytes_per_token: u64,
}

impl BandwidthLimiter {
    /// Create a new bandwidth limiter
    ///
    /// # Arguments
    /// * `max_bytes_per_sec` - Maximum bytes per second (0 = unlimited)
    pub fn new(max_bytes_per_sec: u64) -> Self {
        if max_bytes_per_sec == 0 {
            return Self {
                limiter: None,
                bytes_per_token: 0,
            };
        }

        // Configure token bucket: replenish at rate to achieve target bandwidth
        // Use 1000 tokens/sec, each token = bytes_per_sec/1000 bytes
        let tokens_per_sec = 1000u32;
        let bytes_per_token = max_bytes_per_sec / tokens_per_sec as u64;

        let quota = Quota::per_second(
            NonZeroU32::new(tokens_per_sec).unwrap()
        );

        Self {
            limiter: Some(Arc::new(GovernorRateLimiter::direct(quota))),
            bytes_per_token,
        }
    }

    /// Wait until we can transfer the given number of bytes
    pub fn wait_for_capacity(&self, bytes: u64) {
        if let Some(ref limiter) = self.limiter {
            if self.bytes_per_token > 0 {
                // Calculate tokens needed
                let tokens_needed = (bytes / self.bytes_per_token).max(1) as u32;

                // Wait for tokens to become available
                if let Some(tokens) = NonZeroU32::new(tokens_needed) {
                    while limiter.check_n(tokens).is_err() {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            }
        }
    }

    /// Check if bandwidth limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.limiter.is_some()
    }
}

/// Legacy function for backward compatibility
/// Apply bandwidth limiting to slow down transfer rate
///
/// # Arguments
/// * `bytes_written` - Number of bytes written in this chunk
/// * `max_bandwidth` - Maximum bytes per second allowed
/// * `last_check` - Timestamp of last bandwidth check (updated by this function)
pub fn apply_limit(bytes_written: u64, max_bandwidth: u64, last_check: &mut Instant) {
    if max_bandwidth == 0 {
        return;
    }

    let elapsed = last_check.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();

    if elapsed_secs > 0.0 {
        let bytes_per_sec = bytes_written as f64 / elapsed_secs;
        if bytes_per_sec > max_bandwidth as f64 {
            let sleep_time = Duration::from_secs_f64(
                (bytes_written as f64 / max_bandwidth as f64) - elapsed_secs
            );
            if sleep_time > Duration::ZERO {
                thread::sleep(sleep_time);
            }
        }
    }

    if elapsed >= Duration::from_secs(1) {
        *last_check = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_creation_unlimited() {
        let limiter = BandwidthLimiter::new(0);
        assert!(!limiter.is_enabled());
    }

    #[test]
    fn test_limiter_creation_limited() {
        let limiter = BandwidthLimiter::new(1_048_576); // 1 MB/s
        assert!(limiter.is_enabled());
    }

    #[test]
    fn test_limiter_wait() {
        let limiter = BandwidthLimiter::new(10_485_760); // 10 MB/s
        let start = Instant::now();
        limiter.wait_for_capacity(1024);
        let elapsed = start.elapsed();
        // Should complete quickly for small amounts
        assert!(elapsed < Duration::from_millis(100));
    }

    #[test]
    fn test_unlimited_no_wait() {
        let limiter = BandwidthLimiter::new(0);
        let start = Instant::now();
        limiter.wait_for_capacity(1_000_000);
        let elapsed = start.elapsed();
        // Should be instant
        assert!(elapsed < Duration::from_millis(1));
    }
}
