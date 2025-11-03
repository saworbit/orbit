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
mod num_cpus {
    use std::thread;

    pub fn get() -> usize {
        thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4) // Fallback to 4 if detection fails
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

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
}
