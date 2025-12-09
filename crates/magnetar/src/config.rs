//! Configuration module for Magnetar executor settings
//!
//! Provides configuration types for controlling concurrency, resource usage,
//! and execution behavior.

use std::thread;

/// Configuration for concurrency behavior in Magnetar executors
///
/// This configuration determines how many worker threads are used for
/// parallel operations. The defaults are tuned for mixed CPU/IO workloads
/// common in file synchronization scenarios.
///
/// # Examples
///
/// ```
/// use magnetar::config::ConcurrencyConfig;
///
/// // Use auto-detected defaults
/// let config = ConcurrencyConfig::default();
/// println!("Worker threads: {}", config.worker_threads);
///
/// // Or specify manually
/// let config = ConcurrencyConfig::new(8);
/// ```
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    /// Number of worker threads for parallel operations
    pub worker_threads: usize,
}

impl ConcurrencyConfig {
    /// Create a new concurrency configuration with specified worker threads
    ///
    /// # Arguments
    ///
    /// * `worker_threads` - Number of worker threads (minimum 1)
    ///
    /// # Example
    ///
    /// ```
    /// use magnetar::config::ConcurrencyConfig;
    ///
    /// let config = ConcurrencyConfig::new(4);
    /// assert_eq!(config.worker_threads, 4);
    /// ```
    pub fn new(worker_threads: usize) -> Self {
        Self {
            worker_threads: worker_threads.max(1),
        }
    }

    /// Auto-detect optimal worker thread count based on available CPU cores
    ///
    /// This method attempts to automatically detect the number of available
    /// CPU cores and returns a sensible default for mixed workloads.
    ///
    /// # Algorithm
    ///
    /// 1. Detect available CPU cores using `std::thread::available_parallelism()`
    /// 2. For mixed workloads (CPU hashing + Network I/O), use detected cores
    ///    with a minimum of 2 to keep pipelines full
    /// 3. Fall back to 1 if detection fails (as per PERFORMANCE.md)
    ///
    /// # Example
    ///
    /// ```
    /// use magnetar::config::ConcurrencyConfig;
    ///
    /// let config = ConcurrencyConfig::auto_detect();
    /// assert!(config.worker_threads >= 1);
    /// ```
    pub fn auto_detect() -> Self {
        let detected = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1); // Fallback as per PERFORMANCE.md

        // For mixed workloads (CPU hashing + Network I/O),
        // the detected core count is often optimal.
        // Minimum of 2 to ensure pipeline throughput.
        let optimal = detected.max(2);

        Self {
            worker_threads: optimal,
        }
    }
}

impl Default for ConcurrencyConfig {
    /// Default concurrency configuration using auto-detection
    ///
    /// Orbit attempts to automatically detect the number of available CPU cores
    /// and configure concurrency appropriately for mixed CPU/IO workloads.
    ///
    /// # Example
    ///
    /// ```
    /// use magnetar::config::ConcurrencyConfig;
    ///
    /// let config = ConcurrencyConfig::default();
    /// assert!(config.worker_threads >= 2);
    /// ```
    fn default() -> Self {
        Self::auto_detect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrency_config_new() {
        let config = ConcurrencyConfig::new(4);
        assert_eq!(config.worker_threads, 4);
    }

    #[test]
    fn test_concurrency_config_minimum() {
        // Should clamp to minimum of 1
        let config = ConcurrencyConfig::new(0);
        assert_eq!(config.worker_threads, 1);
    }

    #[test]
    fn test_concurrency_config_auto_detect() {
        let config = ConcurrencyConfig::auto_detect();

        // Should be at least 2 (our minimum for good pipeline behavior)
        assert!(
            config.worker_threads >= 2,
            "Auto-detect should provide at least 2 workers, got {}",
            config.worker_threads
        );

        // Should be reasonable (not more than 1024 cores, which would be unusual)
        assert!(
            config.worker_threads <= 1024,
            "Auto-detect returned unreasonable value: {}",
            config.worker_threads
        );
    }

    #[test]
    fn test_concurrency_config_default() {
        let config = ConcurrencyConfig::default();

        // Default should use auto-detect
        assert!(config.worker_threads >= 2);
    }

    #[test]
    fn test_concurrency_config_clone() {
        let config1 = ConcurrencyConfig::new(8);
        let config2 = config1.clone();

        assert_eq!(config1.worker_threads, config2.worker_threads);
    }
}
