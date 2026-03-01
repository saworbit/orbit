//! Dead-Letter Queue: Persistent storage for permanently-failed items
//!
//! When an item exhausts its maximum retry/penalty count, it is routed to
//! the dead-letter queue instead of failing the entire job. This enables
//! partial success — the job completes with most items transferred, and
//! dead-letter items can be retried manually or by the Sentinel healer.
//!
//! # Design
//!
//! The dead-letter queue is an in-memory structure that can be flushed to
//! persistent storage (Magnetar SQLite) by the caller. This keeps the
//! resilience crate pure-logic with no storage dependencies.
//!
//! # Example
//!
//! ```
//! use orbit_core_resilience::dead_letter::{DeadLetterQueue, DeadLetterEntry, FailureReason};
//!
//! let mut dlq = DeadLetterQueue::new(1000); // max 1000 entries
//!
//! dlq.push(DeadLetterEntry {
//!     item_key: "chunk-42".to_string(),
//!     job_id: 1,
//!     failure_reason: FailureReason::RetriesExhausted { attempts: 5 },
//!     last_error: "connection refused".to_string(),
//!     first_failed_at: std::time::SystemTime::now(),
//!     last_failed_at: std::time::SystemTime::now(),
//!     source_path: Some("/data/file.bin".to_string()),
//!     dest_path: Some("/backup/file.bin".to_string()),
//! });
//!
//! assert_eq!(dlq.len(), 1);
//! let entries = dlq.drain();
//! assert_eq!(entries.len(), 1);
//! ```

use std::collections::VecDeque;
use std::time::SystemTime;

/// Reason an item was sent to the dead-letter queue
#[derive(Debug, Clone)]
pub enum FailureReason {
    /// Maximum retry/penalty count exceeded
    RetriesExhausted { attempts: u32 },

    /// Permanent error (non-transient, should not retry)
    PermanentError,

    /// Checksum mismatch after transfer
    ChecksumMismatch,

    /// Source file disappeared during transfer
    SourceMissing,

    /// Destination write failure (permissions, disk full, etc.)
    DestinationError,

    /// Compression or decompression failure (corrupt data)
    DataCorruption,
}

impl std::fmt::Display for FailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureReason::RetriesExhausted { attempts } => {
                write!(f, "retries exhausted after {} attempts", attempts)
            }
            FailureReason::PermanentError => write!(f, "permanent error"),
            FailureReason::ChecksumMismatch => write!(f, "checksum mismatch"),
            FailureReason::SourceMissing => write!(f, "source missing"),
            FailureReason::DestinationError => write!(f, "destination error"),
            FailureReason::DataCorruption => write!(f, "data corruption"),
        }
    }
}

/// A single dead-letter entry
#[derive(Debug, Clone)]
pub struct DeadLetterEntry {
    /// Unique key for this item (chunk hash, file path, etc.)
    pub item_key: String,

    /// Job ID this item belongs to
    pub job_id: i64,

    /// Why this item was dead-lettered
    pub failure_reason: FailureReason,

    /// Last error message
    pub last_error: String,

    /// When this item first failed
    pub first_failed_at: SystemTime,

    /// When this item was last attempted
    pub last_failed_at: SystemTime,

    /// Optional source path
    pub source_path: Option<String>,

    /// Optional destination path
    pub dest_path: Option<String>,
}

/// In-memory dead-letter queue with bounded capacity.
///
/// Items that exceed the capacity are dropped (oldest first) to prevent
/// unbounded memory growth. The caller should periodically flush to
/// persistent storage.
#[derive(Debug)]
pub struct DeadLetterQueue {
    entries: VecDeque<DeadLetterEntry>,
    max_capacity: usize,
    total_received: u64,
    total_dropped: u64,
}

impl DeadLetterQueue {
    /// Create a new dead-letter queue with the given maximum capacity
    pub fn new(max_capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_capacity.min(1024)),
            max_capacity,
            total_received: 0,
            total_dropped: 0,
        }
    }

    /// Push an entry into the dead-letter queue.
    ///
    /// If the queue is at capacity, the oldest entry is dropped.
    pub fn push(&mut self, entry: DeadLetterEntry) {
        self.total_received += 1;

        if self.entries.len() >= self.max_capacity {
            self.entries.pop_front();
            self.total_dropped += 1;
        }

        self.entries.push_back(entry);
    }

    /// Drain all entries from the queue (for flushing to persistent storage)
    pub fn drain(&mut self) -> Vec<DeadLetterEntry> {
        self.entries.drain(..).collect()
    }

    /// Peek at all entries without removing them
    pub fn entries(&self) -> &VecDeque<DeadLetterEntry> {
        &self.entries
    }

    /// Get entries for a specific job
    pub fn entries_for_job(&self, job_id: i64) -> Vec<&DeadLetterEntry> {
        self.entries.iter().filter(|e| e.job_id == job_id).collect()
    }

    /// Number of entries currently in the queue
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get statistics
    pub fn stats(&self) -> DeadLetterStats {
        DeadLetterStats {
            current_count: self.entries.len(),
            max_capacity: self.max_capacity,
            total_received: self.total_received,
            total_dropped: self.total_dropped,
        }
    }
}

/// Statistics for the dead-letter queue
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetterStats {
    /// Current number of entries
    pub current_count: usize,
    /// Maximum capacity
    pub max_capacity: usize,
    /// Total entries ever received
    pub total_received: u64,
    /// Total entries dropped due to capacity overflow
    pub total_dropped: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(key: &str, job_id: i64) -> DeadLetterEntry {
        DeadLetterEntry {
            item_key: key.to_string(),
            job_id,
            failure_reason: FailureReason::RetriesExhausted { attempts: 5 },
            last_error: "timeout".to_string(),
            first_failed_at: SystemTime::now(),
            last_failed_at: SystemTime::now(),
            source_path: None,
            dest_path: None,
        }
    }

    #[test]
    fn test_push_and_drain() {
        let mut dlq = DeadLetterQueue::new(100);

        dlq.push(make_entry("chunk-1", 1));
        dlq.push(make_entry("chunk-2", 1));
        assert_eq!(dlq.len(), 2);

        let entries = dlq.drain();
        assert_eq!(entries.len(), 2);
        assert!(dlq.is_empty());
    }

    #[test]
    fn test_capacity_overflow_drops_oldest() {
        let mut dlq = DeadLetterQueue::new(2);

        dlq.push(make_entry("chunk-1", 1));
        dlq.push(make_entry("chunk-2", 1));
        dlq.push(make_entry("chunk-3", 1)); // Drops chunk-1

        assert_eq!(dlq.len(), 2);
        let entries = dlq.drain();
        assert_eq!(entries[0].item_key, "chunk-2");
        assert_eq!(entries[1].item_key, "chunk-3");
    }

    #[test]
    fn test_entries_for_job() {
        let mut dlq = DeadLetterQueue::new(100);

        dlq.push(make_entry("chunk-1", 1));
        dlq.push(make_entry("chunk-2", 2));
        dlq.push(make_entry("chunk-3", 1));

        let job1_entries = dlq.entries_for_job(1);
        assert_eq!(job1_entries.len(), 2);
    }

    #[test]
    fn test_stats() {
        let mut dlq = DeadLetterQueue::new(2);

        dlq.push(make_entry("a", 1));
        dlq.push(make_entry("b", 1));
        dlq.push(make_entry("c", 1)); // Overflow

        let stats = dlq.stats();
        assert_eq!(stats.current_count, 2);
        assert_eq!(stats.max_capacity, 2);
        assert_eq!(stats.total_received, 3);
        assert_eq!(stats.total_dropped, 1);
    }

    #[test]
    fn test_failure_reason_display() {
        let r = FailureReason::RetriesExhausted { attempts: 5 };
        assert_eq!(r.to_string(), "retries exhausted after 5 attempts");

        let r = FailureReason::ChecksumMismatch;
        assert_eq!(r.to_string(), "checksum mismatch");
    }

    #[test]
    fn test_all_failure_reason_display() {
        assert_eq!(FailureReason::PermanentError.to_string(), "permanent error");
        assert_eq!(FailureReason::SourceMissing.to_string(), "source missing");
        assert_eq!(
            FailureReason::DestinationError.to_string(),
            "destination error"
        );
        assert_eq!(FailureReason::DataCorruption.to_string(), "data corruption");
    }

    #[test]
    fn test_capacity_one() {
        let mut dlq = DeadLetterQueue::new(1);

        dlq.push(make_entry("a", 1));
        dlq.push(make_entry("b", 1)); // drops "a"

        assert_eq!(dlq.len(), 1);
        let entries = dlq.drain();
        assert_eq!(entries[0].item_key, "b");
    }

    #[test]
    fn test_drain_on_empty() {
        let mut dlq = DeadLetterQueue::new(10);
        let entries = dlq.drain();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_entries_peek_does_not_drain() {
        let mut dlq = DeadLetterQueue::new(10);
        dlq.push(make_entry("a", 1));

        // Peek should not remove entries
        assert_eq!(dlq.entries().len(), 1);
        assert_eq!(dlq.entries().len(), 1); // Still there

        // Drain should still work
        let entries = dlq.drain();
        assert_eq!(entries.len(), 1);
        assert!(dlq.is_empty());
    }

    #[test]
    fn test_entries_for_job_no_match() {
        let mut dlq = DeadLetterQueue::new(10);
        dlq.push(make_entry("a", 1));
        assert!(dlq.entries_for_job(999).is_empty());
    }

    #[test]
    fn test_entries_for_job_after_drain() {
        let mut dlq = DeadLetterQueue::new(10);
        dlq.push(make_entry("a", 1));
        dlq.drain();
        assert!(dlq.entries_for_job(1).is_empty());
    }

    #[test]
    fn test_is_empty_after_drain() {
        let mut dlq = DeadLetterQueue::new(10);
        dlq.push(make_entry("a", 1));
        assert!(!dlq.is_empty());
        dlq.drain();
        assert!(dlq.is_empty());
    }

    #[test]
    fn test_stats_after_many_overflows() {
        let mut dlq = DeadLetterQueue::new(2);

        for i in 0..50 {
            dlq.push(make_entry(&format!("chunk-{}", i), 1));
        }

        let stats = dlq.stats();
        assert_eq!(stats.current_count, 2);
        assert_eq!(stats.total_received, 50);
        assert_eq!(stats.total_dropped, 48); // 50 - 2
    }

    #[test]
    fn test_entry_fields_preserved_through_drain() {
        let mut dlq = DeadLetterQueue::new(10);

        let entry = DeadLetterEntry {
            item_key: "chunk-42".to_string(),
            job_id: 7,
            failure_reason: FailureReason::DataCorruption,
            last_error: "bad zstd frame".to_string(),
            first_failed_at: SystemTime::now(),
            last_failed_at: SystemTime::now(),
            source_path: Some("/src/file.bin".to_string()),
            dest_path: Some("/dst/file.bin".to_string()),
        };

        dlq.push(entry);
        let entries = dlq.drain();
        let e = &entries[0];
        assert_eq!(e.item_key, "chunk-42");
        assert_eq!(e.job_id, 7);
        assert_eq!(e.last_error, "bad zstd frame");
        assert_eq!(e.source_path.as_deref(), Some("/src/file.bin"));
        assert_eq!(e.dest_path.as_deref(), Some("/dst/file.bin"));
        assert!(matches!(e.failure_reason, FailureReason::DataCorruption));
    }

    #[test]
    fn test_different_failure_reasons() {
        let mut dlq = DeadLetterQueue::new(10);

        let reasons = vec![
            FailureReason::PermanentError,
            FailureReason::ChecksumMismatch,
            FailureReason::SourceMissing,
            FailureReason::DestinationError,
            FailureReason::DataCorruption,
            FailureReason::RetriesExhausted { attempts: 3 },
        ];

        for (i, reason) in reasons.into_iter().enumerate() {
            dlq.push(DeadLetterEntry {
                item_key: format!("chunk-{}", i),
                job_id: 1,
                failure_reason: reason,
                last_error: "test".to_string(),
                first_failed_at: SystemTime::now(),
                last_failed_at: SystemTime::now(),
                source_path: None,
                dest_path: None,
            });
        }

        assert_eq!(dlq.len(), 6);
    }

    #[test]
    fn test_fresh_queue_stats() {
        let dlq = DeadLetterQueue::new(100);
        let stats = dlq.stats();
        assert_eq!(stats.current_count, 0);
        assert_eq!(stats.max_capacity, 100);
        assert_eq!(stats.total_received, 0);
        assert_eq!(stats.total_dropped, 0);
    }
}
