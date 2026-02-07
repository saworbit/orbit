/*!
 * Bandwidth throttling utilities with token bucket rate limiting
 */

use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
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
        // Guard against very low bandwidth values where integer division would yield 0
        let tokens_per_sec = 1000u32;
        let bytes_per_token = (max_bytes_per_sec / tokens_per_sec as u64).max(1);

        let quota = Quota::per_second(NonZeroU32::new(tokens_per_sec).unwrap());

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
                (bytes_written as f64 / max_bandwidth as f64) - elapsed_secs,
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

    #[test]
    #[ignore] // Timing-sensitive test - run manually with: cargo test -- --ignored
    fn test_bandwidth_limiting_load() {
        // Test that bandwidth limiting actually limits throughput
        let bandwidth_limit = 1_048_576u64; // 1 MB/s
        let limiter = BandwidthLimiter::new(bandwidth_limit);

        let start = Instant::now();
        let chunk_size = 262_144u64; // 256 KB chunks
        let num_chunks = 8; // Total 2 MB

        for _ in 0..num_chunks {
            limiter.wait_for_capacity(chunk_size);
        }

        let elapsed = start.elapsed();
        let total_bytes = chunk_size * num_chunks;

        // Should take approximately 2 seconds (2 MB / 1 MB/s)
        // Allow some tolerance for overhead
        let expected_duration =
            Duration::from_secs_f64(total_bytes as f64 / bandwidth_limit as f64);
        let min_duration = expected_duration.mul_f32(0.8); // 80% of expected
        let max_duration = expected_duration.mul_f32(1.5); // 150% of expected

        assert!(
            elapsed >= min_duration && elapsed <= max_duration,
            "Elapsed time {:?} should be between {:?} and {:?}",
            elapsed,
            min_duration,
            max_duration
        );
    }

    #[test]
    #[ignore] // Timing-sensitive test - run manually with: cargo test -- --ignored
    fn test_bandwidth_limiting_concurrent() {
        use std::sync::Arc;
        use std::thread;

        // Test bandwidth limiting with multiple threads
        let bandwidth_limit = 2_097_152u64; // 2 MB/s
        let limiter = Arc::new(BandwidthLimiter::new(bandwidth_limit));
        let chunk_size = 262_144u64; // 256 KB

        let start = Instant::now();
        let mut handles = vec![];

        for _ in 0..4 {
            let limiter = limiter.clone();
            let handle = thread::spawn(move || {
                for _ in 0..2 {
                    limiter.wait_for_capacity(chunk_size);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let elapsed = start.elapsed();
        let total_bytes = chunk_size * 4 * 2; // 2 MB total

        // Should take approximately 1 second
        let expected_duration =
            Duration::from_secs_f64(total_bytes as f64 / bandwidth_limit as f64);
        let min_duration = expected_duration.mul_f32(0.7);
        let max_duration = expected_duration.mul_f32(1.5);

        assert!(
            elapsed >= min_duration && elapsed <= max_duration,
            "Elapsed time {:?} should be between {:?} and {:?}",
            elapsed,
            min_duration,
            max_duration
        );
    }
}
