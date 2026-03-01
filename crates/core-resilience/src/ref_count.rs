//! Reference-Counted GC: WAL-gated safe garbage collection for shared chunks
//!
//! When chunks are deduplicated across multiple jobs, reference counting
//! determines when a chunk can be safely garbage collected. The key invariant:
//! chunks are only marked as deletable AFTER the Magnetar WAL has committed
//! the removal — preventing the catastrophic case where a crash between
//! deleting content and updating the index leaves dangling references.
//!
//! # Design
//!
//! This module provides the pure-logic reference counting. The actual
//! deletion is deferred to the caller (who knows about storage). The
//! `RefCountMap` tracks counts in memory, and the `GarbageCollector`
//! enforces the WAL-sync gating rule.
//!
//! # Example
//!
//! ```
//! use orbit_core_resilience::ref_count::{RefCountMap, GarbageCollector};
//!
//! let mut refs = RefCountMap::new();
//!
//! // Job 1 and Job 2 both reference the same chunk
//! refs.increment("chunk_abc123");
//! refs.increment("chunk_abc123");
//! assert_eq!(refs.count("chunk_abc123"), 2);
//!
//! // Job 1 completes, decrements reference
//! refs.decrement("chunk_abc123");
//! assert_eq!(refs.count("chunk_abc123"), 1);
//!
//! // Job 2 completes, chunk is now unreferenced
//! refs.decrement("chunk_abc123");
//! assert_eq!(refs.count("chunk_abc123"), 0);
//!
//! // GC identifies it as reclaimable
//! let mut gc = GarbageCollector::new();
//! gc.mark_reclaimable("chunk_abc123");
//!
//! // Only collect AFTER WAL has synced
//! gc.confirm_wal_synced();
//! let reclaimable = gc.collect();
//! assert_eq!(reclaimable, vec!["chunk_abc123"]);
//! ```

use std::collections::{HashMap, HashSet};

/// Tracks reference counts for shared chunks.
///
/// Each chunk is identified by a string key (typically a hex-encoded BLAKE3 hash).
/// The count represents how many active jobs/locations reference this chunk.
#[derive(Debug, Default)]
pub struct RefCountMap {
    counts: HashMap<String, u64>,
}

impl RefCountMap {
    /// Create a new empty reference count map
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the reference count for a chunk
    pub fn increment(&mut self, chunk_key: &str) {
        *self.counts.entry(chunk_key.to_string()).or_insert(0) += 1;
    }

    /// Decrement the reference count for a chunk.
    ///
    /// Returns the new count. If the count reaches zero, the entry is kept
    /// (not removed) so the GC can identify it as reclaimable.
    pub fn decrement(&mut self, chunk_key: &str) -> u64 {
        if let Some(count) = self.counts.get_mut(chunk_key) {
            *count = count.saturating_sub(1);
            *count
        } else {
            0
        }
    }

    /// Get the current reference count for a chunk
    pub fn count(&self, chunk_key: &str) -> u64 {
        self.counts.get(chunk_key).copied().unwrap_or(0)
    }

    /// Get all chunks with zero references (candidates for GC)
    pub fn zero_ref_chunks(&self) -> Vec<&str> {
        self.counts
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(key, _)| key.as_str())
            .collect()
    }

    /// Remove a chunk from tracking (after it has been garbage collected)
    pub fn remove(&mut self, chunk_key: &str) {
        self.counts.remove(chunk_key);
    }

    /// Bulk-load reference counts (e.g., from a database scan on startup)
    pub fn load(&mut self, key: String, count: u64) {
        self.counts.insert(key, count);
    }

    /// Get the total number of tracked chunks
    pub fn len(&self) -> usize {
        self.counts.len()
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    /// Get statistics
    pub fn stats(&self) -> RefCountStats {
        let total = self.counts.len();
        let zero_refs = self.counts.values().filter(|&&c| c == 0).count();
        let total_refs: u64 = self.counts.values().sum();

        RefCountStats {
            total_chunks: total,
            zero_ref_chunks: zero_refs,
            total_references: total_refs,
        }
    }
}

/// Statistics about reference counts
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefCountStats {
    /// Total chunks being tracked
    pub total_chunks: usize,
    /// Chunks with zero references (GC candidates)
    pub zero_ref_chunks: usize,
    /// Sum of all reference counts
    pub total_references: u64,
}

/// WAL-gated garbage collector.
///
/// Enforces the invariant that chunks are only deleted AFTER the WAL has
/// been synced. This prevents the crash scenario where content is deleted
/// but the index still references it.
///
/// # Lifecycle
///
/// 1. Caller identifies zero-ref chunks via `RefCountMap::zero_ref_chunks()`
/// 2. Caller calls `mark_reclaimable()` for each
/// 3. Caller writes the removal to Magnetar WAL and commits
/// 4. Caller calls `confirm_wal_synced()`
/// 5. Caller calls `collect()` to get the list of chunks safe to delete
/// 6. Caller deletes the actual content
/// 7. Caller calls `acknowledge()` to remove from tracking
#[derive(Debug, Default)]
pub struct GarbageCollector {
    /// Chunks marked as reclaimable (pending WAL sync)
    pending: HashSet<String>,

    /// Chunks confirmed safe to delete (WAL synced)
    ready: Vec<String>,

    /// Whether the WAL has been synced since the last batch of marks
    wal_synced: bool,

    /// Total chunks collected over lifetime
    total_collected: u64,
}

impl GarbageCollector {
    /// Create a new garbage collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a chunk as reclaimable (zero references).
    ///
    /// The chunk will NOT be returned by `collect()` until `confirm_wal_synced()`
    /// has been called — this is the WAL-gating invariant.
    pub fn mark_reclaimable(&mut self, chunk_key: &str) {
        self.pending.insert(chunk_key.to_string());
        self.wal_synced = false; // Reset WAL sync flag for new batch
    }

    /// Confirm that the Magnetar WAL has been synced.
    ///
    /// This moves all pending chunks to the "ready" state, making them
    /// available for collection.
    pub fn confirm_wal_synced(&mut self) {
        self.wal_synced = true;
        self.ready.extend(self.pending.drain());
    }

    /// Collect chunks that are safe to delete.
    ///
    /// Returns the list of chunk keys. The caller should delete the actual
    /// content, then call `acknowledge()`.
    ///
    /// Returns empty if WAL has not been synced.
    pub fn collect(&mut self) -> Vec<String> {
        std::mem::take(&mut self.ready)
    }

    /// Acknowledge that chunks have been deleted.
    pub fn acknowledge(&mut self, count: u64) {
        self.total_collected += count;
    }

    /// Get the number of chunks pending WAL sync
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get the number of chunks ready for collection
    pub fn ready_count(&self) -> usize {
        self.ready.len()
    }

    /// Get total chunks collected over lifetime
    pub fn total_collected(&self) -> u64 {
        self.total_collected
    }

    /// Check if WAL has been synced for current batch
    pub fn is_wal_synced(&self) -> bool {
        self.wal_synced
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_count_basic() {
        let mut refs = RefCountMap::new();

        refs.increment("chunk_a");
        refs.increment("chunk_a");
        refs.increment("chunk_b");

        assert_eq!(refs.count("chunk_a"), 2);
        assert_eq!(refs.count("chunk_b"), 1);
        assert_eq!(refs.count("chunk_c"), 0); // Not tracked

        refs.decrement("chunk_a");
        assert_eq!(refs.count("chunk_a"), 1);
    }

    #[test]
    fn test_zero_ref_detection() {
        let mut refs = RefCountMap::new();

        refs.increment("chunk_a");
        refs.increment("chunk_b");

        refs.decrement("chunk_a"); // Now zero

        let zero = refs.zero_ref_chunks();
        assert_eq!(zero.len(), 1);
        assert_eq!(zero[0], "chunk_a");
    }

    #[test]
    fn test_decrement_saturates() {
        let mut refs = RefCountMap::new();

        // Decrement on unknown key
        assert_eq!(refs.decrement("chunk_x"), 0);

        // Over-decrement
        refs.increment("chunk_a");
        refs.decrement("chunk_a");
        refs.decrement("chunk_a"); // Would be -1, but saturates to 0
        assert_eq!(refs.count("chunk_a"), 0);
    }

    #[test]
    fn test_gc_wal_gating() {
        let mut gc = GarbageCollector::new();

        gc.mark_reclaimable("chunk_a");
        gc.mark_reclaimable("chunk_b");

        // Without WAL sync, collect returns nothing
        assert_eq!(gc.pending_count(), 2);
        let collected = gc.collect();
        assert!(collected.is_empty());

        // After WAL sync, chunks become collectible
        gc.confirm_wal_synced();
        assert_eq!(gc.ready_count(), 2);
        assert_eq!(gc.pending_count(), 0);

        let collected = gc.collect();
        assert_eq!(collected.len(), 2);
        gc.acknowledge(2);
        assert_eq!(gc.total_collected(), 2);
    }

    #[test]
    fn test_gc_incremental_batches() {
        let mut gc = GarbageCollector::new();

        // Batch 1
        gc.mark_reclaimable("chunk_a");
        gc.confirm_wal_synced();
        let batch1 = gc.collect();
        assert_eq!(batch1.len(), 1);
        gc.acknowledge(1);

        // Batch 2
        gc.mark_reclaimable("chunk_b");
        gc.mark_reclaimable("chunk_c");

        // Not yet synced
        let empty = gc.collect();
        assert!(empty.is_empty());

        gc.confirm_wal_synced();
        let batch2 = gc.collect();
        assert_eq!(batch2.len(), 2);
        gc.acknowledge(2);

        assert_eq!(gc.total_collected(), 3);
    }

    #[test]
    fn test_ref_count_stats() {
        let mut refs = RefCountMap::new();

        refs.increment("a");
        refs.increment("a");
        refs.increment("b");
        refs.load("c".to_string(), 0);

        let stats = refs.stats();
        assert_eq!(stats.total_chunks, 3);
        assert_eq!(stats.zero_ref_chunks, 1); // "c"
        assert_eq!(stats.total_references, 3); // 2 + 1 + 0
    }

    #[test]
    fn test_ref_count_remove() {
        let mut refs = RefCountMap::new();

        refs.increment("a");
        refs.remove("a");
        assert_eq!(refs.len(), 0);
        assert_eq!(refs.count("a"), 0);
    }

    #[test]
    fn test_increment_remove_increment_restarts() {
        let mut refs = RefCountMap::new();

        refs.increment("k");
        refs.remove("k");
        refs.increment("k");
        assert_eq!(refs.count("k"), 1);
    }

    #[test]
    fn test_load_overwrites_existing() {
        let mut refs = RefCountMap::new();

        refs.increment("k");
        assert_eq!(refs.count("k"), 1);

        refs.load("k".to_string(), 42);
        assert_eq!(refs.count("k"), 42);
    }

    #[test]
    fn test_load_with_zero_appears_in_zero_refs() {
        let mut refs = RefCountMap::new();

        refs.load("k".to_string(), 0);
        let zeros = refs.zero_ref_chunks();
        assert!(zeros.contains(&"k"));
    }

    #[test]
    fn test_zero_ref_chunks_when_none_at_zero() {
        let mut refs = RefCountMap::new();

        refs.increment("a");
        refs.increment("b");
        refs.increment("c");

        let zeros = refs.zero_ref_chunks();
        assert!(zeros.is_empty());
    }

    #[test]
    fn test_zero_ref_chunks_all_at_zero() {
        let mut refs = RefCountMap::new();

        refs.load("a".to_string(), 0);
        refs.load("b".to_string(), 0);
        refs.load("c".to_string(), 0);

        let mut zeros = refs.zero_ref_chunks();
        zeros.sort();
        assert_eq!(zeros, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_remove_nonexistent_is_noop() {
        let mut refs = RefCountMap::new();

        refs.remove("unknown");
        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn test_gc_mark_same_key_twice_deduplicates() {
        let mut gc = GarbageCollector::new();

        gc.mark_reclaimable("chunk_x");
        gc.mark_reclaimable("chunk_x");
        gc.confirm_wal_synced();
        let collected = gc.collect();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0], "chunk_x");
    }

    #[test]
    fn test_gc_collect_twice_second_is_empty() {
        let mut gc = GarbageCollector::new();

        gc.mark_reclaimable("chunk_a");
        gc.confirm_wal_synced();

        let first = gc.collect();
        assert_eq!(first.len(), 1);

        let second = gc.collect();
        assert!(second.is_empty());
    }

    #[test]
    fn test_gc_confirm_wal_without_pending() {
        let mut gc = GarbageCollector::new();

        gc.confirm_wal_synced();
        assert_eq!(gc.ready_count(), 0);
    }

    #[test]
    fn test_gc_acknowledge_zero() {
        let mut gc = GarbageCollector::new();

        gc.acknowledge(0);
        assert_eq!(gc.total_collected(), 0);
    }

    #[test]
    fn test_gc_wal_synced_resets_on_mark() {
        let mut gc = GarbageCollector::new();

        gc.confirm_wal_synced();
        assert!(gc.is_wal_synced());

        gc.mark_reclaimable("chunk_a");
        assert!(!gc.is_wal_synced());
    }

    #[test]
    fn test_full_lifecycle_refcount_to_gc() {
        let mut refs = RefCountMap::new();
        let mut gc = GarbageCollector::new();

        // Increment twice
        refs.increment("chunk_z");
        refs.increment("chunk_z");
        assert_eq!(refs.count("chunk_z"), 2);

        // Decrement twice to zero
        refs.decrement("chunk_z");
        refs.decrement("chunk_z");
        assert_eq!(refs.count("chunk_z"), 0);

        // Mark reclaimable
        for key in refs.zero_ref_chunks() {
            gc.mark_reclaimable(key);
        }

        // WAL sync
        gc.confirm_wal_synced();

        // Collect
        let collected = gc.collect();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0], "chunk_z");

        // Acknowledge
        gc.acknowledge(collected.len() as u64);
        assert_eq!(gc.total_collected(), 1);
    }
}
