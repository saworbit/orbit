//! Bulletin Board: Centralized error/warning aggregation across Grid nodes
//!
//! Aggregates warnings and errors from all Stars and the Nucleus into a
//! central bulletin feed. Operators get a single pane of glass instead
//! of tailing logs on N machines.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────┐     ┌──────────┐     ┌──────────┐
//! │  Star-1  │     │  Star-2  │     │  Star-N  │
//! └────┬─────┘     └────┬─────┘     └────┬─────┘
//!      │                │                │
//!      └───────┬────────┴───────┬────────┘
//!              │  gRPC push     │
//!              ▼                ▼
//!      ┌──────────────────────────────┐
//!      │     Bulletin Board           │
//!      │  (Nucleus in-memory ring)    │
//!      ├──────────────────────────────┤
//!      │ REST API: GET /api/bulletins │
//!      │  → React Dashboard          │
//!      └──────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```
//! use orbit_connect::bulletin::{BulletinBoard, Bulletin, Severity};
//!
//! let mut board = BulletinBoard::new(1000);
//!
//! board.post(Bulletin::warning(
//!     "star-1",
//!     "transfer",
//!     "Disk usage at 87% on /data volume",
//! ));
//!
//! board.post(Bulletin::error(
//!     "star-2",
//!     "grpc",
//!     "Connection to star-3 lost, retrying...",
//! ));
//!
//! let recent = board.recent(10);
//! assert_eq!(recent.len(), 2);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Severity levels for bulletins
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational message
    Info,
    /// Warning — not yet critical but needs attention
    Warning,
    /// Error — operation failed
    Error,
}

impl Severity {
    /// String representation
    pub fn as_str(&self) -> &str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// A single bulletin (warning/error/info from a component)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bulletin {
    /// Unique bulletin ID (auto-assigned)
    pub id: u64,

    /// When this bulletin was created
    pub timestamp: SystemTime,

    /// Source component (Star ID, "nucleus", etc.)
    pub source: String,

    /// Category/subsystem (e.g., "transfer", "grpc", "disk", "sentinel")
    pub category: String,

    /// Severity level
    pub severity: Severity,

    /// Human-readable message
    pub message: String,

    /// Optional job ID this bulletin relates to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<i64>,
}

impl Bulletin {
    /// Create a new bulletin
    pub fn new(
        source: &str,
        category: &str,
        severity: Severity,
        message: &str,
    ) -> Self {
        Self {
            id: 0, // Assigned by BulletinBoard
            timestamp: SystemTime::now(),
            source: source.to_string(),
            category: category.to_string(),
            severity,
            message: message.to_string(),
            job_id: None,
        }
    }

    /// Create an info bulletin
    pub fn info(source: &str, category: &str, message: &str) -> Self {
        Self::new(source, category, Severity::Info, message)
    }

    /// Create a warning bulletin
    pub fn warning(source: &str, category: &str, message: &str) -> Self {
        Self::new(source, category, Severity::Warning, message)
    }

    /// Create an error bulletin
    pub fn error(source: &str, category: &str, message: &str) -> Self {
        Self::new(source, category, Severity::Error, message)
    }

    /// Attach a job ID
    pub fn with_job(mut self, job_id: i64) -> Self {
        self.job_id = Some(job_id);
        self
    }
}

/// Central bulletin board — a bounded ring buffer of bulletins.
///
/// Thread-safe via `RwLock` for concurrent reads from the REST API
/// and writes from gRPC handlers.
#[derive(Debug)]
pub struct BulletinBoard {
    bulletins: VecDeque<Bulletin>,
    max_capacity: usize,
    next_id: u64,
}

impl BulletinBoard {
    /// Create a new bulletin board with the given capacity
    pub fn new(max_capacity: usize) -> Self {
        Self {
            bulletins: VecDeque::with_capacity(max_capacity.min(1024)),
            max_capacity,
            next_id: 1,
        }
    }

    /// Post a bulletin to the board
    pub fn post(&mut self, mut bulletin: Bulletin) {
        bulletin.id = self.next_id;
        self.next_id += 1;

        if self.bulletins.len() >= self.max_capacity {
            self.bulletins.pop_front();
        }

        self.bulletins.push_back(bulletin);
    }

    /// Get the N most recent bulletins
    pub fn recent(&self, count: usize) -> Vec<&Bulletin> {
        self.bulletins.iter().rev().take(count).collect()
    }

    /// Get bulletins filtered by severity
    pub fn by_severity(&self, severity: Severity) -> Vec<&Bulletin> {
        self.bulletins
            .iter()
            .filter(|b| b.severity == severity)
            .collect()
    }

    /// Get bulletins filtered by source
    pub fn by_source(&self, source: &str) -> Vec<&Bulletin> {
        self.bulletins
            .iter()
            .filter(|b| b.source == source)
            .collect()
    }

    /// Get bulletins filtered by category
    pub fn by_category(&self, category: &str) -> Vec<&Bulletin> {
        self.bulletins
            .iter()
            .filter(|b| b.category == category)
            .collect()
    }

    /// Get bulletins for a specific job
    pub fn by_job(&self, job_id: i64) -> Vec<&Bulletin> {
        self.bulletins
            .iter()
            .filter(|b| b.job_id == Some(job_id))
            .collect()
    }

    /// Get all errors and warnings (no info)
    pub fn issues(&self) -> Vec<&Bulletin> {
        self.bulletins
            .iter()
            .filter(|b| b.severity >= Severity::Warning)
            .collect()
    }

    /// Get the total number of bulletins
    pub fn len(&self) -> usize {
        self.bulletins.len()
    }

    /// Check if the board is empty
    pub fn is_empty(&self) -> bool {
        self.bulletins.is_empty()
    }

    /// Clear all bulletins
    pub fn clear(&mut self) {
        self.bulletins.clear();
    }

    /// Get counts by severity
    pub fn counts(&self) -> BulletinCounts {
        let mut info = 0;
        let mut warnings = 0;
        let mut errors = 0;

        for b in &self.bulletins {
            match b.severity {
                Severity::Info => info += 1,
                Severity::Warning => warnings += 1,
                Severity::Error => errors += 1,
            }
        }

        BulletinCounts {
            total: self.bulletins.len(),
            info,
            warnings,
            errors,
        }
    }
}

/// Thread-safe wrapper around BulletinBoard
#[derive(Debug, Clone)]
pub struct SharedBulletinBoard {
    inner: Arc<RwLock<BulletinBoard>>,
}

impl SharedBulletinBoard {
    /// Create a new shared bulletin board
    pub fn new(max_capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(BulletinBoard::new(max_capacity))),
        }
    }

    /// Post a bulletin
    pub fn post(&self, bulletin: Bulletin) {
        let mut board = self.inner.write().unwrap();
        board.post(bulletin);
    }

    /// Get the N most recent bulletins
    pub fn recent(&self, count: usize) -> Vec<Bulletin> {
        let board = self.inner.read().unwrap();
        board.recent(count).into_iter().cloned().collect()
    }

    /// Get counts by severity
    pub fn counts(&self) -> BulletinCounts {
        let board = self.inner.read().unwrap();
        board.counts()
    }

    /// Get all errors and warnings
    pub fn issues(&self) -> Vec<Bulletin> {
        let board = self.inner.read().unwrap();
        board.issues().into_iter().cloned().collect()
    }

    /// Clear all bulletins
    pub fn clear(&self) {
        let mut board = self.inner.write().unwrap();
        board.clear();
    }
}

/// Bulletin counts by severity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulletinCounts {
    pub total: usize,
    pub info: usize,
    pub warnings: usize,
    pub errors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_and_recent() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("star-1", "transfer", "Started job 42"));
        board.post(Bulletin::warning("star-2", "disk", "85% full"));
        board.post(Bulletin::error("star-1", "grpc", "Connection lost"));

        let recent = board.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].severity, Severity::Error); // Most recent first
        assert_eq!(recent[1].severity, Severity::Warning);
    }

    #[test]
    fn test_capacity_overflow() {
        let mut board = BulletinBoard::new(2);

        board.post(Bulletin::info("a", "x", "first"));
        board.post(Bulletin::info("b", "x", "second"));
        board.post(Bulletin::info("c", "x", "third")); // Drops "first"

        assert_eq!(board.len(), 2);
        let all = board.recent(10);
        assert_eq!(all[0].source, "c");
        assert_eq!(all[1].source, "b");
    }

    #[test]
    fn test_filter_by_severity() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("a", "x", "info"));
        board.post(Bulletin::warning("b", "x", "warn"));
        board.post(Bulletin::error("c", "x", "err"));

        assert_eq!(board.by_severity(Severity::Error).len(), 1);
        assert_eq!(board.by_severity(Severity::Warning).len(), 1);
        assert_eq!(board.by_severity(Severity::Info).len(), 1);
    }

    #[test]
    fn test_filter_by_source() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("star-1", "x", "msg1"));
        board.post(Bulletin::info("star-2", "x", "msg2"));
        board.post(Bulletin::info("star-1", "x", "msg3"));

        assert_eq!(board.by_source("star-1").len(), 2);
        assert_eq!(board.by_source("star-2").len(), 1);
    }

    #[test]
    fn test_filter_by_job() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("a", "x", "no job"));
        board.post(Bulletin::info("a", "x", "job 1").with_job(1));
        board.post(Bulletin::info("a", "x", "job 2").with_job(2));
        board.post(Bulletin::info("a", "x", "job 1 again").with_job(1));

        assert_eq!(board.by_job(1).len(), 2);
        assert_eq!(board.by_job(2).len(), 1);
    }

    #[test]
    fn test_issues() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("a", "x", "fine"));
        board.post(Bulletin::warning("b", "x", "warn"));
        board.post(Bulletin::error("c", "x", "bad"));

        let issues = board.issues();
        assert_eq!(issues.len(), 2); // warning + error, no info
    }

    #[test]
    fn test_counts() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("a", "x", "i1"));
        board.post(Bulletin::info("a", "x", "i2"));
        board.post(Bulletin::warning("a", "x", "w1"));
        board.post(Bulletin::error("a", "x", "e1"));

        let counts = board.counts();
        assert_eq!(counts.total, 4);
        assert_eq!(counts.info, 2);
        assert_eq!(counts.warnings, 1);
        assert_eq!(counts.errors, 1);
    }

    #[test]
    fn test_auto_incrementing_ids() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("a", "x", "first"));
        board.post(Bulletin::info("a", "x", "second"));

        let recent = board.recent(2);
        assert_eq!(recent[0].id, 2);
        assert_eq!(recent[1].id, 1);
    }

    #[test]
    fn test_shared_bulletin_board() {
        let board = SharedBulletinBoard::new(100);

        board.post(Bulletin::error("star-1", "transfer", "Failed"));
        board.post(Bulletin::warning("star-2", "disk", "Low space"));

        let recent = board.recent(10);
        assert_eq!(recent.len(), 2);

        let counts = board.counts();
        assert_eq!(counts.errors, 1);
        assert_eq!(counts.warnings, 1);
    }

    #[test]
    fn test_by_category_filter() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("star-1", "transfer", "transfer msg 1"));
        board.post(Bulletin::warning("star-2", "disk", "disk msg"));
        board.post(Bulletin::error("star-1", "transfer", "transfer msg 2"));
        board.post(Bulletin::info("star-3", "grpc", "grpc msg"));

        let transfer = board.by_category("transfer");
        assert_eq!(transfer.len(), 2);
        assert!(transfer.iter().all(|b| b.category == "transfer"));

        let disk = board.by_category("disk");
        assert_eq!(disk.len(), 1);
        assert_eq!(disk[0].source, "star-2");

        let missing = board.by_category("nonexistent");
        assert!(missing.is_empty());
    }

    #[test]
    fn test_recent_zero() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("a", "x", "msg1"));
        board.post(Bulletin::info("b", "x", "msg2"));

        let recent = board.recent(0);
        assert!(recent.is_empty());
    }

    #[test]
    fn test_clear_then_post_ids_continue() {
        let mut board = BulletinBoard::new(100);

        board.post(Bulletin::info("a", "x", "first"));
        board.post(Bulletin::info("a", "x", "second"));
        assert_eq!(board.len(), 2);

        board.clear();
        assert!(board.is_empty());

        board.post(Bulletin::info("a", "x", "after clear"));
        let recent = board.recent(1);
        assert_eq!(recent.len(), 1);
        // ID should continue from 3, not reset to 1
        assert_eq!(recent[0].id, 3);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Info < Severity::Error);
    }

    #[test]
    fn test_severity_as_str() {
        assert_eq!(Severity::Info.as_str(), "info");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Error.as_str(), "error");
    }

    #[test]
    fn test_bulletin_serde_roundtrip() {
        let original = Bulletin::warning("star-1", "disk", "85% full").with_job(42);
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: Bulletin = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.id, original.id);
        assert_eq!(deserialized.source, original.source);
        assert_eq!(deserialized.category, original.category);
        assert_eq!(deserialized.severity, original.severity);
        assert_eq!(deserialized.message, original.message);
        assert_eq!(deserialized.job_id, Some(42));
    }

    #[test]
    fn test_bulletin_counts_serde_roundtrip() {
        let counts = BulletinCounts {
            total: 10,
            info: 5,
            warnings: 3,
            errors: 2,
        };
        let json = serde_json::to_string(&counts).expect("serialize");
        let deserialized: BulletinCounts = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, counts);
    }

    #[test]
    fn test_is_empty_fresh_board() {
        let board = BulletinBoard::new(100);
        assert!(board.is_empty());
    }

    #[test]
    fn test_shared_board_issues() {
        let board = SharedBulletinBoard::new(100);

        board.post(Bulletin::info("a", "x", "info msg"));
        board.post(Bulletin::warning("b", "x", "warn msg"));
        board.post(Bulletin::error("c", "x", "error msg"));
        board.post(Bulletin::info("d", "x", "another info"));

        let issues = board.issues();
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().all(|b| b.severity >= Severity::Warning));
    }

    #[test]
    fn test_shared_board_clear() {
        let board = SharedBulletinBoard::new(100);

        board.post(Bulletin::info("a", "x", "msg1"));
        board.post(Bulletin::warning("b", "x", "msg2"));
        assert_eq!(board.recent(10).len(), 2);

        board.clear();
        assert!(board.recent(10).is_empty());
    }

    #[test]
    fn test_shared_board_clone_shares_state() {
        let board1 = SharedBulletinBoard::new(100);
        let board2 = board1.clone();

        board1.post(Bulletin::info("star-1", "x", "from board1"));
        let recent = board2.recent(10);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].source, "star-1");

        board2.post(Bulletin::error("star-2", "x", "from board2"));
        let counts = board1.counts();
        assert_eq!(counts.total, 2);
    }

    #[test]
    fn test_shared_board_concurrent_access() {
        use std::thread;

        let board = SharedBulletinBoard::new(10_000);
        let num_threads = 8;
        let posts_per_thread = 100;

        let handles: Vec<_> = (0..num_threads)
            .map(|i| {
                let board = board.clone();
                thread::spawn(move || {
                    for j in 0..posts_per_thread {
                        board.post(Bulletin::info(
                            &format!("thread-{i}"),
                            "concurrent",
                            &format!("msg-{j}"),
                        ));
                        // Interleave reads to stress concurrent access
                        let _ = board.recent(5);
                        let _ = board.counts();
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread should not panic");
        }

        let counts = board.counts();
        assert_eq!(counts.total, num_threads * posts_per_thread);
    }
}
