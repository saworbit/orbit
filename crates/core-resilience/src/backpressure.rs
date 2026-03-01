//! Backpressure: Dual-threshold flow control for transfer destinations
//!
//! Each connection/queue in the pipeline has two independent thresholds:
//! - **Object count**: Maximum number of items in the queue
//! - **Byte size**: Maximum total bytes of queued items
//!
//! When either threshold is breached, the upstream producer is signaled to
//! pause. This is cooperative — the producer checks `can_accept()` before
//! dispatching new work.
//!
//! # Example
//!
//! ```
//! use orbit_core_resilience::backpressure::{BackpressureGuard, BackpressureConfig};
//!
//! let config = BackpressureConfig {
//!     max_object_count: 10_000,
//!     max_byte_size: 1_073_741_824, // 1 GiB
//! };
//!
//! let guard = BackpressureGuard::new("star-1", config);
//!
//! // Producer checks before dispatching
//! assert!(guard.can_accept());
//!
//! // Record incoming items
//! guard.record_enqueue(1, 4096);
//! assert!(guard.can_accept());
//!
//! // Record item completion
//! guard.record_dequeue(1, 4096);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Configuration for backpressure thresholds
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// Maximum number of objects allowed in the queue
    pub max_object_count: u64,

    /// Maximum total bytes allowed in the queue
    pub max_byte_size: u64,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_object_count: 10_000,
            max_byte_size: 1_073_741_824, // 1 GiB
        }
    }
}

/// Current state of a backpressure-monitored queue
#[derive(Debug, Clone)]
pub struct BackpressureState {
    /// Current object count in the queue
    pub object_count: u64,

    /// Current total bytes in the queue
    pub byte_size: u64,

    /// Whether the queue is currently applying backpressure
    pub is_backpressured: bool,

    /// Percentage of object count threshold used (0.0 - 1.0+)
    pub object_utilization: f64,

    /// Percentage of byte size threshold used (0.0 - 1.0+)
    pub byte_utilization: f64,
}

/// Tracks queue depth for a single connection/destination and signals backpressure.
///
/// Thread-safe via atomic operations — no locking required.
#[derive(Debug)]
pub struct BackpressureGuard {
    /// Identifier for this queue (e.g., Star ID)
    name: String,

    /// Configuration thresholds
    config: BackpressureConfig,

    /// Current object count
    object_count: AtomicU64,

    /// Current byte size
    byte_size: AtomicU64,
}

impl BackpressureGuard {
    /// Create a new backpressure guard for a named queue
    pub fn new(name: impl Into<String>, config: BackpressureConfig) -> Self {
        Self {
            name: name.into(),
            config,
            object_count: AtomicU64::new(0),
            byte_size: AtomicU64::new(0),
        }
    }

    /// Check if the queue can accept more items (neither threshold breached)
    pub fn can_accept(&self) -> bool {
        let count = self.object_count.load(Ordering::Relaxed);
        let size = self.byte_size.load(Ordering::Relaxed);

        count < self.config.max_object_count && size < self.config.max_byte_size
    }

    /// Record items being enqueued
    pub fn record_enqueue(&self, count: u64, bytes: u64) {
        self.object_count.fetch_add(count, Ordering::Relaxed);
        self.byte_size.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record items being dequeued (completed or removed)
    pub fn record_dequeue(&self, count: u64, bytes: u64) {
        // Use saturating subtraction to prevent underflow
        let _ = self
            .object_count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(count))
            });
        let _ = self
            .byte_size
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(bytes))
            });
    }

    /// Get the current state of this queue
    pub fn state(&self) -> BackpressureState {
        let object_count = self.object_count.load(Ordering::Relaxed);
        let byte_size = self.byte_size.load(Ordering::Relaxed);

        BackpressureState {
            object_count,
            byte_size,
            is_backpressured: object_count >= self.config.max_object_count
                || byte_size >= self.config.max_byte_size,
            object_utilization: object_count as f64 / self.config.max_object_count as f64,
            byte_utilization: byte_size as f64 / self.config.max_byte_size as f64,
        }
    }

    /// Get the name of this queue
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the configuration
    pub fn config(&self) -> &BackpressureConfig {
        &self.config
    }

    /// Reset counters (e.g., after a reconnection)
    pub fn reset(&self) {
        self.object_count.store(0, Ordering::Relaxed);
        self.byte_size.store(0, Ordering::Relaxed);
    }
}

/// Manages backpressure guards for multiple destinations (e.g., Stars in a Grid)
#[derive(Debug, Clone)]
pub struct BackpressureRegistry {
    guards: Arc<std::sync::RwLock<Vec<Arc<BackpressureGuard>>>>,
}

impl BackpressureRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            guards: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    /// Register a new backpressure guard
    pub fn register(&self, guard: Arc<BackpressureGuard>) {
        let mut guards = self.guards.write().unwrap();
        guards.push(guard);
    }

    /// Find a guard by name
    pub fn get(&self, name: &str) -> Option<Arc<BackpressureGuard>> {
        let guards = self.guards.read().unwrap();
        guards.iter().find(|g| g.name() == name).cloned()
    }

    /// Get all guards that can accept more work
    pub fn available_destinations(&self) -> Vec<Arc<BackpressureGuard>> {
        let guards = self.guards.read().unwrap();
        guards.iter().filter(|g| g.can_accept()).cloned().collect()
    }

    /// Get a snapshot of all queue states
    pub fn all_states(&self) -> Vec<(String, BackpressureState)> {
        let guards = self.guards.read().unwrap();
        guards
            .iter()
            .map(|g| (g.name().to_string(), g.state()))
            .collect()
    }

    /// Check if any destination is backpressured
    pub fn any_backpressured(&self) -> bool {
        let guards = self.guards.read().unwrap();
        guards.iter().any(|g| !g.can_accept())
    }

    /// Remove a guard by name
    pub fn remove(&self, name: &str) {
        let mut guards = self.guards.write().unwrap();
        guards.retain(|g| g.name() != name);
    }
}

impl Default for BackpressureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BackpressureConfig::default();
        assert_eq!(config.max_object_count, 10_000);
        assert_eq!(config.max_byte_size, 1_073_741_824);
    }

    #[test]
    fn test_can_accept() {
        let guard = BackpressureGuard::new(
            "test",
            BackpressureConfig {
                max_object_count: 2,
                max_byte_size: 1024,
            },
        );

        assert!(guard.can_accept());

        guard.record_enqueue(1, 512);
        assert!(guard.can_accept());

        guard.record_enqueue(1, 512);
        assert!(!guard.can_accept()); // 2 objects = threshold reached
    }

    #[test]
    fn test_byte_threshold() {
        let guard = BackpressureGuard::new(
            "test",
            BackpressureConfig {
                max_object_count: 1000,
                max_byte_size: 100,
            },
        );

        guard.record_enqueue(1, 100);
        assert!(!guard.can_accept()); // Byte threshold reached
    }

    #[test]
    fn test_dequeue_releases() {
        let guard = BackpressureGuard::new(
            "test",
            BackpressureConfig {
                max_object_count: 2,
                max_byte_size: 1024,
            },
        );

        guard.record_enqueue(2, 512);
        assert!(!guard.can_accept());

        guard.record_dequeue(1, 256);
        assert!(guard.can_accept());
    }

    #[test]
    fn test_dequeue_no_underflow() {
        let guard = BackpressureGuard::new("test", BackpressureConfig::default());

        // Dequeue more than enqueued should saturate at 0
        guard.record_enqueue(1, 100);
        guard.record_dequeue(5, 500);

        let state = guard.state();
        assert_eq!(state.object_count, 0);
        assert_eq!(state.byte_size, 0);
    }

    #[test]
    fn test_state_utilization() {
        let guard = BackpressureGuard::new(
            "test",
            BackpressureConfig {
                max_object_count: 100,
                max_byte_size: 1000,
            },
        );

        guard.record_enqueue(50, 500);

        let state = guard.state();
        assert!((state.object_utilization - 0.5).abs() < f64::EPSILON);
        assert!((state.byte_utilization - 0.5).abs() < f64::EPSILON);
        assert!(!state.is_backpressured);
    }

    #[test]
    fn test_reset() {
        let guard = BackpressureGuard::new(
            "test",
            BackpressureConfig {
                max_object_count: 2,
                max_byte_size: 100,
            },
        );

        guard.record_enqueue(10, 1000);
        assert!(!guard.can_accept());

        guard.reset();
        assert!(guard.can_accept());
        assert_eq!(guard.state().object_count, 0);
    }

    #[test]
    fn test_registry() {
        let registry = BackpressureRegistry::new();

        let g1 = Arc::new(BackpressureGuard::new(
            "star-1",
            BackpressureConfig {
                max_object_count: 2,
                max_byte_size: 1024,
            },
        ));
        let g2 = Arc::new(BackpressureGuard::new(
            "star-2",
            BackpressureConfig::default(),
        ));

        registry.register(g1.clone());
        registry.register(g2.clone());

        // Both available
        assert_eq!(registry.available_destinations().len(), 2);

        // Backpressure star-1
        g1.record_enqueue(2, 0);
        assert_eq!(registry.available_destinations().len(), 1);
        assert!(registry.any_backpressured());

        // Find by name
        let found = registry.get("star-1").unwrap();
        assert!(!found.can_accept());
    }

    #[test]
    fn test_exact_threshold_boundary() {
        let guard = BackpressureGuard::new(
            "boundary",
            BackpressureConfig {
                max_object_count: 5,
                max_byte_size: 1024,
            },
        );

        // object_count = max - 1 => should accept
        guard.record_enqueue(4, 0);
        assert!(
            guard.can_accept(),
            "object_count at max-1 should still accept"
        );

        // object_count = max => should NOT accept
        guard.record_enqueue(1, 0);
        assert!(!guard.can_accept(), "object_count at max should not accept");
    }

    #[test]
    fn test_both_thresholds_exceeded() {
        let guard = BackpressureGuard::new(
            "both",
            BackpressureConfig {
                max_object_count: 5,
                max_byte_size: 100,
            },
        );

        // Exceed both thresholds
        guard.record_enqueue(10, 200);
        assert!(!guard.can_accept(), "both thresholds exceeded");

        let state = guard.state();
        assert!(state.is_backpressured);

        // Dequeue enough to drop below object threshold only
        guard.record_dequeue(6, 0);
        assert!(
            !guard.can_accept(),
            "byte threshold still exceeded, should not accept"
        );

        let state = guard.state();
        assert_eq!(
            state.object_count, 4,
            "object count should be 4 after dequeue"
        );
        assert_eq!(state.byte_size, 200, "byte size unchanged");
        assert!(state.is_backpressured, "still backpressured due to bytes");
    }

    #[test]
    fn test_enqueue_dequeue_zero() {
        let guard = BackpressureGuard::new("zero-ops", BackpressureConfig::default());

        // Enqueue zero items and zero bytes — state should remain unchanged
        guard.record_enqueue(0, 0);
        let state = guard.state();
        assert_eq!(
            state.object_count, 0,
            "enqueue(0,0) should be a no-op for count"
        );
        assert_eq!(
            state.byte_size, 0,
            "enqueue(0,0) should be a no-op for bytes"
        );

        // Add some items, then dequeue zero — state should remain the same
        guard.record_enqueue(3, 42);
        guard.record_dequeue(0, 0);
        let state = guard.state();
        assert_eq!(
            state.object_count, 3,
            "dequeue(0,0) should be a no-op for count"
        );
        assert_eq!(
            state.byte_size, 42,
            "dequeue(0,0) should be a no-op for bytes"
        );
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = BackpressureRegistry::new();
        registry.register(Arc::new(BackpressureGuard::new(
            "exists",
            BackpressureConfig::default(),
        )));

        let result = registry.get("does-not-exist");
        assert!(
            result.is_none(),
            "get() for a nonexistent name should return None"
        );
    }

    #[test]
    fn test_registry_remove_nonexistent() {
        let registry = BackpressureRegistry::new();
        let guard = Arc::new(BackpressureGuard::new(
            "keep-me",
            BackpressureConfig::default(),
        ));
        registry.register(guard);

        // Removing a name that does not exist should not panic or alter the registry
        registry.remove("ghost");

        assert_eq!(
            registry.all_states().len(),
            1,
            "registry should still contain the original guard"
        );
        assert!(
            registry.get("keep-me").is_some(),
            "original guard should remain after removing nonexistent name"
        );
    }

    #[test]
    fn test_registry_available_when_all_backpressured() {
        let registry = BackpressureRegistry::new();

        let g1 = Arc::new(BackpressureGuard::new(
            "full-1",
            BackpressureConfig {
                max_object_count: 1,
                max_byte_size: 1024,
            },
        ));
        let g2 = Arc::new(BackpressureGuard::new(
            "full-2",
            BackpressureConfig {
                max_object_count: 1,
                max_byte_size: 1024,
            },
        ));

        // Fill both guards to their limits
        g1.record_enqueue(1, 0);
        g2.record_enqueue(1, 0);

        registry.register(g1);
        registry.register(g2);

        let available = registry.available_destinations();
        assert!(
            available.is_empty(),
            "no destinations should be available when all are backpressured"
        );
    }

    #[test]
    fn test_registry_all_states_empty() {
        let registry = BackpressureRegistry::new();
        let states = registry.all_states();
        assert!(
            states.is_empty(),
            "empty registry should return empty Vec from all_states()"
        );
    }

    #[test]
    fn test_registry_any_backpressured_empty() {
        let registry = BackpressureRegistry::new();
        assert!(
            !registry.any_backpressured(),
            "empty registry should return false for any_backpressured()"
        );
    }

    #[test]
    fn test_name_accessor() {
        let guard = BackpressureGuard::new("my-queue-name", BackpressureConfig::default());
        assert_eq!(
            guard.name(),
            "my-queue-name",
            "name() should return the name passed to the constructor"
        );
    }

    #[test]
    fn test_concurrent_enqueue_dequeue() {
        let guard = Arc::new(BackpressureGuard::new(
            "concurrent",
            BackpressureConfig {
                max_object_count: u64::MAX,
                max_byte_size: u64::MAX,
            },
        ));

        let num_threads = 8;
        let ops_per_thread = 1_000u64;
        let mut handles = Vec::new();

        // Spawn threads that enqueue
        for _ in 0..num_threads {
            let g = Arc::clone(&guard);
            handles.push(std::thread::spawn(move || {
                for _ in 0..ops_per_thread {
                    g.record_enqueue(1, 10);
                }
            }));
        }

        // Spawn threads that dequeue
        for _ in 0..num_threads {
            let g = Arc::clone(&guard);
            handles.push(std::thread::spawn(move || {
                for _ in 0..ops_per_thread {
                    g.record_dequeue(1, 10);
                }
            }));
        }

        for h in handles {
            h.join().expect("thread should not panic");
        }

        let state = guard.state();
        // Each enqueue adds 1 count / 10 bytes, each dequeue removes 1 / 10.
        // Equal number of enqueue and dequeue threads with equal ops, so final
        // counts should be 0 (saturating subtraction prevents underflow).
        // With relaxed ordering the intermediate values may vary, but the final
        // result after all threads join must be consistent and non-negative.
        assert!(
            state.object_count <= num_threads * ops_per_thread,
            "object_count should be bounded; got {}",
            state.object_count
        );
        assert!(
            state.byte_size <= num_threads * ops_per_thread * 10,
            "byte_size should be bounded; got {}",
            state.byte_size
        );
    }
}
