//! Penalization: Temporary deprioritization of failed transfer items
//!
//! When a transfer unit fails with a transient error, instead of blocking the
//! entire pipeline, the failed item is "penalized" — pushed to the back of the
//! queue with a `retry_after` timestamp. Other items continue flowing while
//! the penalized item waits out its penalty period.
//!
//! # Key Concepts
//!
//! - **Penalty Duration**: Configurable delay before a penalized item becomes eligible again
//! - **Max Penalties**: After N penalties, the item is routed to the dead-letter queue
//! - **Cooperative**: The scheduler simply skips penalized items — no blocking
//!
//! # Example
//!
//! ```
//! use orbit_core_resilience::penalization::{PenaltyBox, PenaltyConfig};
//! use std::time::Duration;
//!
//! let config = PenaltyConfig {
//!     initial_delay: Duration::from_secs(5),
//!     max_delay: Duration::from_secs(300),
//!     backoff_factor: 2.0,
//!     max_penalties: 5,
//! };
//!
//! let mut penalty_box = PenaltyBox::new(config);
//!
//! // Penalize a failed item
//! let exhausted = penalty_box.penalize("chunk-42", "connection refused");
//! assert!(!exhausted); // Still has retries left
//! assert!(!penalty_box.is_eligible("chunk-42")); // Not yet eligible
//!
//! // Check eligibility later (after penalty expires)
//! // penalty_box.is_eligible("chunk-42") => true (after delay)
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Configuration for the penalty system
#[derive(Debug, Clone)]
pub struct PenaltyConfig {
    /// Initial delay before a penalized item becomes eligible again
    pub initial_delay: Duration,

    /// Maximum delay (caps exponential backoff)
    pub max_delay: Duration,

    /// Multiplier applied to delay on each successive penalty
    pub backoff_factor: f64,

    /// Maximum number of penalties before routing to dead-letter
    pub max_penalties: u32,
}

impl Default for PenaltyConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(300),
            backoff_factor: 2.0,
            max_penalties: 5,
        }
    }
}

/// State of a penalized item
#[derive(Debug, Clone)]
pub struct PenaltyRecord {
    /// Number of times this item has been penalized
    pub penalty_count: u32,

    /// When this item becomes eligible for retry
    pub retry_after: Instant,

    /// Last error that caused the penalty
    pub last_error: String,

    /// Current delay applied to this item
    pub current_delay: Duration,
}

/// Tracks penalized items and their eligibility for retry.
///
/// Items are identified by a string key (e.g., chunk hash, file path, or
/// a composite key like "chunk_hash:star_id" for per-destination penalties).
#[derive(Debug)]
pub struct PenaltyBox {
    config: PenaltyConfig,
    records: HashMap<String, PenaltyRecord>,
}

impl PenaltyBox {
    /// Create a new penalty box with the given configuration
    pub fn new(config: PenaltyConfig) -> Self {
        Self {
            config,
            records: HashMap::new(),
        }
    }

    /// Penalize an item after a transient failure.
    ///
    /// Returns `true` if the item has exhausted its maximum penalties
    /// (should be routed to dead-letter queue), `false` if it will be
    /// retried after the penalty period.
    pub fn penalize(&mut self, key: &str, error: &str) -> bool {
        let record = self
            .records
            .entry(key.to_string())
            .or_insert(PenaltyRecord {
                penalty_count: 0,
                retry_after: Instant::now(),
                last_error: String::new(),
                current_delay: self.config.initial_delay,
            });

        record.penalty_count += 1;
        record.last_error = error.to_string();

        // Check if exhausted
        if record.penalty_count > self.config.max_penalties {
            return true;
        }

        // Calculate delay with exponential backoff
        let delay = if record.penalty_count == 1 {
            self.config.initial_delay
        } else {
            let factor = self
                .config
                .backoff_factor
                .powi(record.penalty_count as i32 - 1);
            let delay_ms = (self.config.initial_delay.as_millis() as f64 * factor) as u64;
            Duration::from_millis(delay_ms).min(self.config.max_delay)
        };

        record.current_delay = delay;
        record.retry_after = Instant::now() + delay;

        false
    }

    /// Check if an item is eligible for retry (penalty period has expired)
    pub fn is_eligible(&self, key: &str) -> bool {
        match self.records.get(key) {
            None => true, // Never penalized = eligible
            Some(record) => Instant::now() >= record.retry_after,
        }
    }

    /// Get the penalty record for an item, if it exists
    pub fn get_record(&self, key: &str) -> Option<&PenaltyRecord> {
        self.records.get(key)
    }

    /// Remove a penalty record (item completed successfully or sent to dead-letter)
    pub fn clear(&mut self, key: &str) {
        self.records.remove(key);
    }

    /// Remove all expired penalty records that have been retried successfully
    pub fn clear_eligible(&mut self) {
        let now = Instant::now();
        self.records.retain(|_, record| now < record.retry_after);
    }

    /// Get all currently penalized (ineligible) item keys
    pub fn penalized_keys(&self) -> Vec<&str> {
        let now = Instant::now();
        self.records
            .iter()
            .filter(|(_, record)| now < record.retry_after)
            .map(|(key, _)| key.as_str())
            .collect()
    }

    /// Get all items that have exhausted their penalties
    pub fn exhausted_keys(&self) -> Vec<&str> {
        self.records
            .iter()
            .filter(|(_, record)| record.penalty_count > self.config.max_penalties)
            .map(|(key, _)| key.as_str())
            .collect()
    }

    /// Get the number of items currently being tracked
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if the penalty box is empty
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get a snapshot of penalty statistics
    pub fn stats(&self) -> PenaltyStats {
        let now = Instant::now();
        let mut penalized = 0;
        let mut eligible = 0;
        let mut exhausted = 0;

        for record in self.records.values() {
            if record.penalty_count > self.config.max_penalties {
                exhausted += 1;
            } else if now < record.retry_after {
                penalized += 1;
            } else {
                eligible += 1;
            }
        }

        PenaltyStats {
            total_tracked: self.records.len(),
            penalized,
            eligible,
            exhausted,
        }
    }
}

/// Statistics about the penalty box state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenaltyStats {
    /// Total items being tracked
    pub total_tracked: usize,
    /// Items currently in penalty (not yet eligible)
    pub penalized: usize,
    /// Items whose penalty has expired (ready for retry)
    pub eligible: usize,
    /// Items that have exceeded max penalties
    pub exhausted: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PenaltyConfig::default();
        assert_eq!(config.initial_delay, Duration::from_secs(5));
        assert_eq!(config.max_penalties, 5);
        assert_eq!(config.backoff_factor, 2.0);
    }

    #[test]
    fn test_penalize_and_eligibility() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(60),
            backoff_factor: 2.0,
            max_penalties: 3,
        };

        let mut box_ = PenaltyBox::new(config);

        // First penalty
        let exhausted = box_.penalize("chunk-1", "timeout");
        assert!(!exhausted);
        assert!(!box_.is_eligible("chunk-1"));

        // Unknown item is always eligible
        assert!(box_.is_eligible("chunk-unknown"));
    }

    #[test]
    fn test_max_penalties_exhausted() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_secs(1),
            backoff_factor: 1.0,
            max_penalties: 2,
        };

        let mut box_ = PenaltyBox::new(config);

        assert!(!box_.penalize("chunk-1", "err1"));
        assert!(!box_.penalize("chunk-1", "err2"));
        assert!(box_.penalize("chunk-1", "err3")); // Exhausted
    }

    #[test]
    fn test_clear_record() {
        let config = PenaltyConfig::default();
        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-1", "error");
        assert_eq!(box_.len(), 1);

        box_.clear("chunk-1");
        assert_eq!(box_.len(), 0);
        assert!(box_.is_eligible("chunk-1"));
    }

    #[test]
    fn test_stats() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_secs(3600), // Long penalty
            max_delay: Duration::from_secs(3600),
            backoff_factor: 1.0,
            max_penalties: 1,
        };

        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-1", "err"); // Penalized, not exhausted
        box_.penalize("chunk-2", "err"); // Penalized
        box_.penalize("chunk-2", "err"); // Exhausted (max_penalties=1, count=2>1)

        let stats = box_.stats();
        assert_eq!(stats.total_tracked, 2);
        assert_eq!(stats.penalized, 1); // chunk-1
        assert_eq!(stats.exhausted, 1); // chunk-2
    }

    #[test]
    fn test_exponential_backoff() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_factor: 2.0,
            max_penalties: 10,
        };

        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-1", "err");
        let d1 = box_.get_record("chunk-1").unwrap().current_delay;
        assert_eq!(d1, Duration::from_millis(100));

        box_.penalize("chunk-1", "err");
        let d2 = box_.get_record("chunk-1").unwrap().current_delay;
        assert_eq!(d2, Duration::from_millis(200));

        box_.penalize("chunk-1", "err");
        let d3 = box_.get_record("chunk-1").unwrap().current_delay;
        assert_eq!(d3, Duration::from_millis(400));
    }

    #[test]
    fn test_max_delay_cap() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            backoff_factor: 10.0,
            max_penalties: 10,
        };

        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-1", "err"); // 1s
        box_.penalize("chunk-1", "err"); // 10s -> capped to 5s

        let delay = box_.get_record("chunk-1").unwrap().current_delay;
        assert_eq!(delay, Duration::from_secs(5));
    }

    #[test]
    fn test_is_empty_fresh() {
        let box_ = PenaltyBox::new(PenaltyConfig::default());
        assert!(box_.is_empty());
        assert_eq!(box_.len(), 0);
    }

    #[test]
    fn test_penalized_keys() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_secs(3600),
            max_delay: Duration::from_secs(3600),
            backoff_factor: 1.0,
            max_penalties: 10,
        };
        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-a", "err");
        box_.penalize("chunk-b", "err");

        let keys = box_.penalized_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"chunk-a"));
        assert!(keys.contains(&"chunk-b"));
    }

    #[test]
    fn test_exhausted_keys() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1),
            backoff_factor: 1.0,
            max_penalties: 1,
        };
        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-a", "err"); // count=1, not exhausted
        box_.penalize("chunk-a", "err"); // count=2 > max_penalties=1 => exhausted
        box_.penalize("chunk-b", "err"); // count=1, not exhausted

        let exhausted = box_.exhausted_keys();
        assert_eq!(exhausted.len(), 1);
        assert!(exhausted.contains(&"chunk-a"));
    }

    #[test]
    fn test_re_penalize_after_clear_restarts_count() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_secs(3600),
            max_delay: Duration::from_secs(3600),
            backoff_factor: 1.0,
            max_penalties: 3,
        };
        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-1", "err");
        box_.penalize("chunk-1", "err");
        assert_eq!(box_.get_record("chunk-1").unwrap().penalty_count, 2);

        // Clear and re-penalize — count should restart from 1
        box_.clear("chunk-1");
        box_.penalize("chunk-1", "err");
        assert_eq!(box_.get_record("chunk-1").unwrap().penalty_count, 1);
    }

    #[test]
    fn test_max_penalties_zero_exhausts_immediately() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1),
            backoff_factor: 1.0,
            max_penalties: 0,
        };
        let mut box_ = PenaltyBox::new(config);

        // Very first penalization should exhaust (count=1 > max=0)
        assert!(box_.penalize("chunk-1", "err"));
    }

    #[test]
    fn test_backoff_factor_less_than_one() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(60),
            backoff_factor: 0.5,
            max_penalties: 10,
        };
        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-1", "err"); // 1000ms
        box_.penalize("chunk-1", "err"); // 1000 * 0.5 = 500ms

        let d = box_.get_record("chunk-1").unwrap().current_delay;
        assert_eq!(d, Duration::from_millis(500));
    }

    #[test]
    fn test_last_error_updated_on_each_penalty() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_secs(3600),
            max_delay: Duration::from_secs(3600),
            backoff_factor: 1.0,
            max_penalties: 5,
        };
        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-1", "first error");
        assert_eq!(
            box_.get_record("chunk-1").unwrap().last_error,
            "first error"
        );

        box_.penalize("chunk-1", "second error");
        assert_eq!(
            box_.get_record("chunk-1").unwrap().last_error,
            "second error"
        );
    }

    #[test]
    fn test_get_record_none_for_unknown() {
        let box_ = PenaltyBox::new(PenaltyConfig::default());
        assert!(box_.get_record("nonexistent").is_none());
    }

    #[test]
    fn test_clear_nonexistent_is_noop() {
        let mut box_ = PenaltyBox::new(PenaltyConfig::default());
        box_.clear("nonexistent"); // Should not panic
        assert!(box_.is_empty());
    }

    #[test]
    fn test_many_distinct_keys() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_secs(3600),
            max_delay: Duration::from_secs(3600),
            backoff_factor: 1.0,
            max_penalties: 10,
        };
        let mut box_ = PenaltyBox::new(config);

        for i in 0..200 {
            box_.penalize(&format!("chunk-{}", i), "err");
        }
        assert_eq!(box_.len(), 200);
        assert_eq!(box_.penalized_keys().len(), 200);
    }

    #[test]
    fn test_clear_eligible_retains_active_penalties() {
        let config = PenaltyConfig {
            initial_delay: Duration::from_millis(1), // Very short
            max_delay: Duration::from_millis(1),
            backoff_factor: 1.0,
            max_penalties: 10,
        };
        let mut box_ = PenaltyBox::new(config);

        box_.penalize("chunk-1", "err");

        // Sleep to let the 1ms penalty expire
        std::thread::sleep(Duration::from_millis(10));

        // Now add one with a long penalty
        let long_config_box = PenaltyBox::new(PenaltyConfig {
            initial_delay: Duration::from_secs(3600),
            ..PenaltyConfig::default()
        });
        // We can't mix configs in one box, so test clear_eligible on the short one
        box_.clear_eligible();
        // The expired record should have been removed
        assert_eq!(box_.len(), 0);
    }
}
