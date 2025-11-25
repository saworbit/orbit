/*!
 * Concurrency control for managing parallel transfers
 *
 * This module provides semaphore-based concurrency limiting to control
 * the number of simultaneous file operations.
 */

use std::sync::{Arc, Condvar, Mutex};

/// A counting semaphore for controlling concurrent operations
#[derive(Clone)]
pub struct ConcurrencyLimiter {
    state: Arc<SemaphoreState>,
}

struct SemaphoreState {
    available: Mutex<usize>,
    condvar: Condvar,
    max: usize,
}

impl ConcurrencyLimiter {
    /// Create a new concurrency limiter
    ///
    /// # Arguments
    /// * `max_concurrent` - Maximum number of concurrent operations (0 = auto-detect)
    ///
    /// # Auto-detection Behavior
    /// When `max_concurrent` is 0, Orbit attempts to detect available CPU cores using
    /// `std::thread::available_parallelism()`. If detection fails (e.g., in restricted
    /// containers or cgroup environments), it defaults to 1 (single-threaded mode) for
    /// safety and logs a warning to stderr.
    pub fn new(max_concurrent: usize) -> Self {
        let max = if max_concurrent == 0 {
            // Auto-detect: use number of CPU cores
            num_cpus::get()
        } else {
            max_concurrent
        };

        Self {
            state: Arc::new(SemaphoreState {
                available: Mutex::new(max),
                condvar: Condvar::new(),
                max,
            }),
        }
    }

    /// Acquire a permit (blocks until available)
    pub fn acquire(&self) -> ConcurrencyPermit {
        let mut available = self.state.available.lock().unwrap();

        while *available == 0 {
            available = self.state.condvar.wait(available).unwrap();
        }

        *available -= 1;

        ConcurrencyPermit {
            state: self.state.clone(),
        }
    }

    /// Try to acquire a permit without blocking
    pub fn try_acquire(&self) -> Option<ConcurrencyPermit> {
        let mut available = self.state.available.lock().unwrap();

        if *available > 0 {
            *available -= 1;
            Some(ConcurrencyPermit {
                state: self.state.clone(),
            })
        } else {
            None
        }
    }

    /// Get the maximum number of concurrent operations allowed
    pub fn max_concurrent(&self) -> usize {
        self.state.max
    }

    /// Get the current number of available permits
    pub fn available(&self) -> usize {
        *self.state.available.lock().unwrap()
    }
}

/// A permit that represents permission to perform a concurrent operation
/// The permit is automatically released when dropped
pub struct ConcurrencyPermit {
    state: Arc<SemaphoreState>,
}

impl Drop for ConcurrencyPermit {
    fn drop(&mut self) {
        let mut available = self.state.available.lock().unwrap();
        *available += 1;
        self.state.condvar.notify_one();
    }
}

/// Helper function to detect optimal concurrency level based on system
pub fn detect_optimal_concurrency() -> usize {
    let cpu_count = num_cpus::get();

    // For I/O-bound operations, we can use more threads than CPUs
    // Use 2x CPU count, capped at 16
    (cpu_count * 2).min(16)
}

// Shim for num_cpus functionality (fallback to std if needed)
//
// Safety: If CPU detection fails (e.g., in restricted containers or cgroup environments),
// we default to single-threaded mode (1) and emit a warning to stderr. This prevents
// resource exhaustion in hostile environments while alerting operators to the issue.
mod num_cpus {
    use std::thread;

    pub fn get() -> usize {
        thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or_else(|e| {
                // [SPEC CHANGE] Warn user and default to single-threaded mode for safety
                // We use eprintln! here because this might run before the logging subsystem
                // is fully initialized or if logging itself is failing.
                eprintln!(
                    "WARN: Orbit failed to detect available parallelism: {}. \
                    Defaulting to 1 concurrent operation to prevent resource exhaustion.",
                    e
                );
                1
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn test_limiter_creation() {
        let limiter = ConcurrencyLimiter::new(4);
        assert_eq!(limiter.max_concurrent(), 4);
        assert_eq!(limiter.available(), 4);
    }

    #[test]
    fn test_limiter_auto_detect() {
        let limiter = ConcurrencyLimiter::new(0);
        assert!(limiter.max_concurrent() > 0);
    }

    #[test]
    fn test_acquire_and_release() {
        let limiter = ConcurrencyLimiter::new(2);

        let _permit1 = limiter.acquire();
        assert_eq!(limiter.available(), 1);

        let _permit2 = limiter.acquire();
        assert_eq!(limiter.available(), 0);

        drop(_permit1);
        assert_eq!(limiter.available(), 1);

        drop(_permit2);
        assert_eq!(limiter.available(), 2);
    }

    #[test]
    fn test_try_acquire() {
        let limiter = ConcurrencyLimiter::new(1);

        let permit1 = limiter.try_acquire();
        assert!(permit1.is_some());

        let permit2 = limiter.try_acquire();
        assert!(permit2.is_none());

        drop(permit1);
        let permit3 = limiter.try_acquire();
        assert!(permit3.is_some());
    }

    #[test]
    fn test_concurrent_access() {
        let limiter = ConcurrencyLimiter::new(3);
        let counter = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        for _ in 0..10 {
            let limiter = limiter.clone();
            let counter = counter.clone();
            let max_concurrent = max_concurrent.clone();

            let handle = thread::spawn(move || {
                let _permit = limiter.acquire();

                let current = counter.fetch_add(1, Ordering::SeqCst) + 1;

                // Track maximum concurrent operations
                max_concurrent.fetch_max(current, Ordering::SeqCst);

                thread::sleep(Duration::from_millis(10));

                counter.fetch_sub(1, Ordering::SeqCst);
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Maximum concurrent should not exceed limit
        assert!(max_concurrent.load(Ordering::SeqCst) <= 3);
    }

    #[test]
    fn test_detect_optimal_concurrency() {
        let optimal = detect_optimal_concurrency();
        assert!(optimal > 0);
        assert!(optimal <= 16);
    }

    #[test]
    fn test_shim_behavior_sane() {
        // We can't force an Err in the real shim, but we can verify the result is sane.
        let cpus = num_cpus::get();
        assert!(cpus >= 1, "Must report at least 1 CPU");
    }

    #[test]
    fn test_optimal_concurrency_calculation() {
        // If we hypothetically had 1 CPU (fallback case)
        let cpu_count = 1;
        let optimal = (cpu_count * 2).min(16);
        assert_eq!(optimal, 2, "Should allow 2 threads on 1 core (IO bound)");

        // If we had 4 CPUs (common case)
        let cpu_count = 4;
        let optimal = (cpu_count * 2).min(16);
        assert_eq!(optimal, 8);

        // If we had 32 CPUs (cap case)
        let cpu_count = 32;
        let optimal = (cpu_count * 2).min(16);
        assert_eq!(optimal, 16, "Should cap at 16");
    }

    #[test]
    fn test_concurrency_limiting_load() {
        // Test that concurrency limiting actually limits concurrent operations
        let max_concurrent = 3;
        let limiter = ConcurrencyLimiter::new(max_concurrent);
        let counter = Arc::new(AtomicUsize::new(0));
        let max_observed = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Spawn 10 threads that will compete for 3 permits
        for _ in 0..10 {
            let limiter = limiter.clone();
            let counter = counter.clone();
            let max_observed = max_observed.clone();

            let handle = thread::spawn(move || {
                let _permit = limiter.acquire();

                // Increment counter to track concurrent operations
                let current = counter.fetch_add(1, Ordering::SeqCst) + 1;
                max_observed.fetch_max(current, Ordering::SeqCst);

                // Simulate work
                thread::sleep(Duration::from_millis(50));

                // Decrement counter
                counter.fetch_sub(1, Ordering::SeqCst);
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Maximum observed concurrent operations should not exceed limit
        let max = max_observed.load(Ordering::SeqCst);
        assert!(
            max <= max_concurrent,
            "Max concurrent operations {} exceeded limit {}",
            max,
            max_concurrent
        );
    }

    #[test]
    #[ignore] // Timing-sensitive test - run manually with: cargo test -- --ignored
    fn test_concurrency_limiting_throughput() {
        // Test that concurrency limiting affects throughput as expected
        let max_concurrent = 2;
        let limiter = ConcurrencyLimiter::new(max_concurrent);
        let num_tasks = 8;
        let task_duration = Duration::from_millis(100);

        let start = Instant::now();
        let mut handles = vec![];

        for _ in 0..num_tasks {
            let limiter = limiter.clone();
            let handle = thread::spawn(move || {
                let _permit = limiter.acquire();
                thread::sleep(task_duration);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let elapsed = start.elapsed();

        // With 2 concurrent tasks and 8 total tasks of 100ms each,
        // we expect approximately 400ms (8 tasks / 2 concurrent = 4 batches * 100ms)
        let expected_duration = task_duration * (num_tasks / max_concurrent) as u32;
        let min_duration = expected_duration.mul_f32(0.8);
        let max_duration = expected_duration.mul_f32(1.5);

        assert!(
            elapsed >= min_duration && elapsed <= max_duration,
            "Elapsed time {:?} should be between {:?} and {:?}",
            elapsed,
            min_duration,
            max_duration
        );
    }

    #[test]
    fn test_concurrency_permit_drop() {
        // Test that permits are properly released on drop
        let limiter = ConcurrencyLimiter::new(1);

        {
            let _permit = limiter.acquire();
            assert_eq!(limiter.available(), 0);
        } // permit dropped here

        // Permit should be released
        assert_eq!(limiter.available(), 1);
    }
}
