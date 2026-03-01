//! Composable Prioritizers: Chainable sort criteria for transfer scheduling
//!
//! Instead of a single priority dimension, prioritizers can be stacked:
//! 1. Semantic priority (Critical > High > Normal > Low)
//! 2. Size preference (e.g., smallest-first within same tier)
//! 3. Age tiebreaker (oldest-first)
//!
//! This is more flexible than a single `BinaryHeap` key and allows
//! per-connection customization in the Grid architecture.
//!
//! # Example
//!
//! ```
//! use orbit_core_semantic::prioritizer::{
//!     TransferItem, ComposablePrioritizer, SemanticPrioritizer,
//!     SmallestFirstPrioritizer, OldestFirstPrioritizer,
//! };
//! use orbit_core_semantic::Priority;
//! use std::time::SystemTime;
//!
//! let mut items = vec![
//!     TransferItem::new("large_video.mp4", Priority::Low, 1_000_000, SystemTime::now()),
//!     TransferItem::new("config.toml", Priority::Critical, 512, SystemTime::now()),
//!     TransferItem::new("readme.md", Priority::Normal, 2048, SystemTime::now()),
//! ];
//!
//! // Chain prioritizers: semantic first, then smallest, then oldest
//! let prioritizer = ComposablePrioritizer::new(vec![
//!     Box::new(SemanticPrioritizer),
//!     Box::new(SmallestFirstPrioritizer),
//!     Box::new(OldestFirstPrioritizer),
//! ]);
//!
//! items.sort_by(|a, b| prioritizer.compare(a, b));
//!
//! assert_eq!(items[0].path, "config.toml");   // Critical + smallest
//! assert_eq!(items[1].path, "readme.md");      // Normal
//! assert_eq!(items[2].path, "large_video.mp4"); // Low priority
//! ```

use super::Priority;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::SystemTime;

/// A transfer item with metadata used for prioritization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferItem {
    /// File path (used as identifier)
    pub path: String,

    /// Semantic priority tier
    pub priority: Priority,

    /// File size in bytes
    pub size: u64,

    /// When this item was discovered/queued
    pub queued_at: SystemTime,

    /// Number of retry attempts
    pub retry_count: u32,
}

impl TransferItem {
    /// Create a new transfer item
    pub fn new(path: &str, priority: Priority, size: u64, queued_at: SystemTime) -> Self {
        Self {
            path: path.to_string(),
            priority,
            size,
            queued_at,
            retry_count: 0,
        }
    }
}

/// Trait for a single prioritization criterion.
///
/// Returns `Ordering::Equal` if this criterion can't distinguish the two items,
/// allowing the next prioritizer in the chain to break the tie.
pub trait Prioritizer: Send + Sync {
    /// Compare two transfer items. `Less` means `a` should come first.
    fn compare(&self, a: &TransferItem, b: &TransferItem) -> Ordering;

    /// Human-readable name of this prioritizer
    fn name(&self) -> &str;
}

/// Chains multiple prioritizers: first non-Equal result wins.
pub struct ComposablePrioritizer {
    chain: Vec<Box<dyn Prioritizer>>,
}

impl ComposablePrioritizer {
    /// Create a new composable prioritizer from an ordered list
    pub fn new(chain: Vec<Box<dyn Prioritizer>>) -> Self {
        Self { chain }
    }

    /// Compare two items using the chain of prioritizers
    pub fn compare(&self, a: &TransferItem, b: &TransferItem) -> Ordering {
        for p in &self.chain {
            let ord = p.compare(a, b);
            if ord != Ordering::Equal {
                return ord;
            }
        }
        Ordering::Equal
    }

    /// Get the names of all prioritizers in the chain
    pub fn chain_names(&self) -> Vec<&str> {
        self.chain.iter().map(|p| p.name()).collect()
    }
}

/// Default chain: semantic priority, then smallest-first, then oldest-first
impl Default for ComposablePrioritizer {
    fn default() -> Self {
        Self::new(vec![
            Box::new(SemanticPrioritizer),
            Box::new(SmallestFirstPrioritizer),
            Box::new(OldestFirstPrioritizer),
        ])
    }
}

// ── Built-in Prioritizers ──

/// Prioritize by semantic tier (Critical < High < Normal < Low)
pub struct SemanticPrioritizer;

impl Prioritizer for SemanticPrioritizer {
    fn compare(&self, a: &TransferItem, b: &TransferItem) -> Ordering {
        a.priority.cmp(&b.priority)
    }

    fn name(&self) -> &str {
        "semantic"
    }
}

/// Prioritize smaller files first (maximizes completed-file count)
pub struct SmallestFirstPrioritizer;

impl Prioritizer for SmallestFirstPrioritizer {
    fn compare(&self, a: &TransferItem, b: &TransferItem) -> Ordering {
        a.size.cmp(&b.size)
    }

    fn name(&self) -> &str {
        "smallest_first"
    }
}

/// Prioritize larger files first (maximizes throughput utilization)
pub struct LargestFirstPrioritizer;

impl Prioritizer for LargestFirstPrioritizer {
    fn compare(&self, a: &TransferItem, b: &TransferItem) -> Ordering {
        b.size.cmp(&a.size) // Reversed
    }

    fn name(&self) -> &str {
        "largest_first"
    }
}

/// Prioritize oldest items first (FIFO within same tier)
pub struct OldestFirstPrioritizer;

impl Prioritizer for OldestFirstPrioritizer {
    fn compare(&self, a: &TransferItem, b: &TransferItem) -> Ordering {
        a.queued_at.cmp(&b.queued_at)
    }

    fn name(&self) -> &str {
        "oldest_first"
    }
}

/// Prioritize newest items first (real-time dashboard use case)
pub struct NewestFirstPrioritizer;

impl Prioritizer for NewestFirstPrioritizer {
    fn compare(&self, a: &TransferItem, b: &TransferItem) -> Ordering {
        b.queued_at.cmp(&a.queued_at) // Reversed
    }

    fn name(&self) -> &str {
        "newest_first"
    }
}

/// Prioritize items with fewer retries (favor fresh items over retried ones)
pub struct FewestRetriesPrioritizer;

impl Prioritizer for FewestRetriesPrioritizer {
    fn compare(&self, a: &TransferItem, b: &TransferItem) -> Ordering {
        a.retry_count.cmp(&b.retry_count)
    }

    fn name(&self) -> &str {
        "fewest_retries"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn now() -> SystemTime {
        SystemTime::now()
    }

    fn past(secs: u64) -> SystemTime {
        SystemTime::now() - Duration::from_secs(secs)
    }

    #[test]
    fn test_semantic_prioritizer() {
        let p = SemanticPrioritizer;
        let critical = TransferItem::new("a", Priority::Critical, 100, now());
        let low = TransferItem::new("b", Priority::Low, 100, now());

        assert_eq!(p.compare(&critical, &low), Ordering::Less);
        assert_eq!(p.compare(&low, &critical), Ordering::Greater);
    }

    #[test]
    fn test_smallest_first() {
        let p = SmallestFirstPrioritizer;
        let small = TransferItem::new("a", Priority::Normal, 100, now());
        let large = TransferItem::new("b", Priority::Normal, 10000, now());

        assert_eq!(p.compare(&small, &large), Ordering::Less);
    }

    #[test]
    fn test_oldest_first() {
        let p = OldestFirstPrioritizer;
        let old = TransferItem::new("a", Priority::Normal, 100, past(60));
        let new = TransferItem::new("b", Priority::Normal, 100, now());

        assert_eq!(p.compare(&old, &new), Ordering::Less);
    }

    #[test]
    fn test_composable_chain() {
        let mut items = vec![
            TransferItem::new("big_critical", Priority::Critical, 1_000_000, now()),
            TransferItem::new("small_critical", Priority::Critical, 100, now()),
            TransferItem::new("normal", Priority::Normal, 500, now()),
            TransferItem::new("low", Priority::Low, 50, now()),
        ];

        let prioritizer = ComposablePrioritizer::default();
        items.sort_by(|a, b| prioritizer.compare(a, b));

        // Critical items first, smaller critical before larger
        assert_eq!(items[0].path, "small_critical");
        assert_eq!(items[1].path, "big_critical");
        assert_eq!(items[2].path, "normal");
        assert_eq!(items[3].path, "low");
    }

    #[test]
    fn test_default_chain_names() {
        let p = ComposablePrioritizer::default();
        assert_eq!(
            p.chain_names(),
            vec!["semantic", "smallest_first", "oldest_first"]
        );
    }

    #[test]
    fn test_custom_chain() {
        let prioritizer = ComposablePrioritizer::new(vec![
            Box::new(LargestFirstPrioritizer),
            Box::new(NewestFirstPrioritizer),
        ]);

        let mut items = vec![
            TransferItem::new("small", Priority::Normal, 100, now()),
            TransferItem::new("large", Priority::Normal, 10000, now()),
        ];

        items.sort_by(|a, b| prioritizer.compare(a, b));
        assert_eq!(items[0].path, "large"); // Largest first
    }

    #[test]
    fn test_fewest_retries() {
        let p = FewestRetriesPrioritizer;

        let mut fresh = TransferItem::new("a", Priority::Normal, 100, now());
        fresh.retry_count = 0;

        let mut retried = TransferItem::new("b", Priority::Normal, 100, now());
        retried.retry_count = 3;

        assert_eq!(p.compare(&fresh, &retried), Ordering::Less);
    }

    #[test]
    fn test_largest_first_standalone() {
        let p = LargestFirstPrioritizer;
        let small = TransferItem::new("small", Priority::Normal, 100, now());
        let large = TransferItem::new("large", Priority::Normal, 50_000, now());

        assert_eq!(p.compare(&large, &small), Ordering::Less);
        assert_eq!(p.compare(&small, &large), Ordering::Greater);
        assert_eq!(p.compare(&large, &large), Ordering::Equal);
    }

    #[test]
    fn test_newest_first_standalone() {
        let p = NewestFirstPrioritizer;
        let old_item = TransferItem::new("old", Priority::Normal, 100, past(120));
        let new_item = TransferItem::new("new", Priority::Normal, 100, now());

        assert_eq!(p.compare(&new_item, &old_item), Ordering::Less);
        assert_eq!(p.compare(&old_item, &new_item), Ordering::Greater);
    }

    #[test]
    fn test_equal_items_return_equal() {
        let t = now();
        let a = TransferItem::new("file_a", Priority::Normal, 1024, t);
        let b = TransferItem::new("file_b", Priority::Normal, 1024, t);

        let prioritizer = ComposablePrioritizer::default();
        assert_eq!(prioritizer.compare(&a, &b), Ordering::Equal);
    }

    #[test]
    fn test_empty_chain_always_equal() {
        let prioritizer = ComposablePrioritizer::new(vec![]);

        let a = TransferItem::new("a", Priority::Critical, 100, past(60));
        let b = TransferItem::new("b", Priority::Low, 999_999, now());

        assert_eq!(prioritizer.compare(&a, &b), Ordering::Equal);
        assert_eq!(prioritizer.compare(&b, &a), Ordering::Equal);
    }

    #[test]
    fn test_single_element_chain() {
        let prioritizer = ComposablePrioritizer::new(vec![Box::new(SmallestFirstPrioritizer)]);

        let mut items = vec![
            TransferItem::new("big", Priority::Normal, 5000, now()),
            TransferItem::new("tiny", Priority::Normal, 10, now()),
            TransferItem::new("mid", Priority::Normal, 500, now()),
        ];

        items.sort_by(|a, b| prioritizer.compare(a, b));

        assert_eq!(items[0].path, "tiny");
        assert_eq!(items[1].path, "mid");
        assert_eq!(items[2].path, "big");
    }

    #[test]
    fn test_sort_stability() {
        let t = now();
        let mut items = vec![
            TransferItem::new("first", Priority::Normal, 100, t),
            TransferItem::new("second", Priority::Normal, 100, t),
        ];

        let prioritizer = ComposablePrioritizer::default();
        items.sort_by(|a, b| prioritizer.compare(a, b));

        // Rust's sort_by is stable, so equal items keep their original order
        assert_eq!(items[0].path, "first");
        assert_eq!(items[1].path, "second");
    }

    #[test]
    fn test_same_priority_different_size() {
        let t = now();
        let mut items = vec![
            TransferItem::new("large_critical", Priority::Critical, 50_000, t),
            TransferItem::new("small_critical", Priority::Critical, 256, t),
        ];

        let prioritizer = ComposablePrioritizer::default();
        items.sort_by(|a, b| prioritizer.compare(a, b));

        // Default chain: semantic first (equal), then smallest_first
        assert_eq!(items[0].path, "small_critical");
        assert_eq!(items[1].path, "large_critical");
    }

    #[test]
    fn test_same_priority_same_size_different_age() {
        let mut items = vec![
            TransferItem::new("newer", Priority::Normal, 1024, past(10)),
            TransferItem::new("older", Priority::Normal, 1024, past(60)),
        ];

        let prioritizer = ComposablePrioritizer::default();
        items.sort_by(|a, b| prioritizer.compare(a, b));

        // Default chain: semantic (equal), smallest_first (equal), oldest_first
        assert_eq!(items[0].path, "older");
        assert_eq!(items[1].path, "newer");
    }

    #[test]
    fn test_retry_count_in_chain() {
        let t = now();
        let prioritizer = ComposablePrioritizer::new(vec![Box::new(FewestRetriesPrioritizer)]);

        let mut fresh = TransferItem::new("fresh", Priority::Normal, 100, t);
        fresh.retry_count = 0;

        let mut moderate = TransferItem::new("moderate", Priority::Normal, 100, t);
        moderate.retry_count = 2;

        let mut heavy = TransferItem::new("heavy", Priority::Normal, 100, t);
        heavy.retry_count = 5;

        let mut items = vec![heavy, fresh, moderate];
        items.sort_by(|a, b| prioritizer.compare(a, b));

        assert_eq!(items[0].path, "fresh");
        assert_eq!(items[1].path, "moderate");
        assert_eq!(items[2].path, "heavy");
    }

    #[test]
    fn test_transfer_item_new_defaults() {
        let item = TransferItem::new("test.txt", Priority::Normal, 2048, now());

        assert_eq!(item.retry_count, 0);
        assert_eq!(item.path, "test.txt");
        assert_eq!(item.size, 2048);
    }
}
