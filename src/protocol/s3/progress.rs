//! Progress Tracking and Callbacks for S3 Operations
//!
//! This module provides comprehensive progress tracking for S3 operations,
//! enabling real-time UI updates, monitoring, and user feedback.
//!
//! # Overview
//!
//! Long-running S3 operations (uploads, downloads, batch operations) benefit
//! from progress feedback. This module provides multiple callback mechanisms:
//! - Byte-level progress for transfers
//! - Operation-level progress for batch jobs
//! - ETA calculation
//! - Throughput metrics
//! - Pause/resume support
//!
//! # Features
//!
//! - **Real-time Updates** - Progress callbacks during operations
//! - **Multiple Granularities** - Byte-level, chunk-level, file-level progress
//! - **Rich Metrics** - Transfer rate, ETA, percentage, bytes transferred
//! - **Async-First** - Non-blocking callbacks using tokio channels
//! - **Type-Safe** - Strongly typed progress events
//! - **Composable** - Chain multiple progress reporters
//!
//! # Examples
//!
//! ## Simple Progress Bar
//!
//! ```no_run
//! use orbit::protocol::s3::{S3Client, S3Config};
//! use orbit::protocol::s3::progress::{ProgressReporter, ProgressEvent};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = S3Client::new(S3Config::default()).await?;
//!
//!     // Create progress reporter
//!     let (reporter, mut receiver) = ProgressReporter::new();
//!
//!     // Spawn task to handle progress events
//!     tokio::spawn(async move {
//!         while let Some(event) = receiver.recv().await {
//!             match event {
//!                 ProgressEvent::TransferStarted { total_bytes, .. } => {
//!                     println!("Starting transfer: {} bytes", total_bytes);
//!                 }
//!                 ProgressEvent::Progress { bytes_transferred, percentage, .. } => {
//!                     println!("Progress: {:.1}%", percentage);
//!                 }
//!                 ProgressEvent::TransferCompleted { .. } => {
//!                     println!("Transfer complete!");
//!                 }
//!                 _ => {}
//!             }
//!         }
//!     });
//!
//!     // Use reporter with operations...
//!     Ok(())
//! }
//! ```
//!
//! ## With Throughput Tracking
//!
//! ```ignore
//! use orbit::protocol::s3::progress::{ProgressReporter, ProgressEvent, ThroughputTracker};
//!
//! let (reporter, mut receiver) = ProgressReporter::new();
//! let tracker = ThroughputTracker::new();
//!
//! tokio::spawn(async move {
//!     while let Some(event) = receiver.recv().await {
//!         if let ProgressEvent::Progress { bytes_transferred, .. } = event {
//!             tracker.update(bytes_transferred).await;
//!             println!("Speed: {:.2} MB/s", tracker.throughput_mbps().await);
//!             println!("ETA: {:?}", tracker.eta(1000000).await);
//!         }
//!     }
//! });
//! ```

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;

/// Progress event for S3 operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    /// Transfer started
    TransferStarted {
        /// Operation identifier
        operation_id: String,
        /// Key being transferred
        key: String,
        /// Total bytes to transfer
        total_bytes: u64,
        /// Transfer direction
        direction: TransferDirection,
    },

    /// Progress update
    Progress {
        /// Operation identifier
        operation_id: String,
        /// Bytes transferred so far
        bytes_transferred: u64,
        /// Total bytes
        total_bytes: u64,
        /// Percentage complete (0-100)
        percentage: f64,
        /// Current transfer rate in bytes/sec
        rate_bps: f64,
        /// Estimated time remaining
        eta_secs: Option<u64>,
    },

    /// Chunk completed (for multipart operations)
    ChunkCompleted {
        /// Operation identifier
        operation_id: String,
        /// Chunk number
        chunk_number: u32,
        /// Bytes in this chunk
        chunk_bytes: u64,
    },

    /// Transfer completed successfully
    TransferCompleted {
        /// Operation identifier
        operation_id: String,
        /// Total bytes transferred
        total_bytes: u64,
        /// Duration of transfer
        duration: Duration,
    },

    /// Transfer failed
    TransferFailed {
        /// Operation identifier
        operation_id: String,
        /// Error message
        error: String,
        /// Bytes transferred before failure
        bytes_transferred: u64,
    },

    /// Batch operation progress
    BatchProgress {
        /// Operation identifier
        operation_id: String,
        /// Completed items
        completed: u64,
        /// Total items
        total: u64,
        /// Succeeded items
        succeeded: u64,
        /// Failed items
        failed: u64,
    },
}

/// Transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    /// Upload to S3
    Upload,
    /// Download from S3
    Download,
}

impl std::fmt::Display for TransferDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferDirection::Upload => write!(f, "Upload"),
            TransferDirection::Download => write!(f, "Download"),
        }
    }
}

/// Progress reporter for sending progress events
#[derive(Clone)]
pub struct ProgressReporter {
    sender: Arc<UnboundedSender<ProgressEvent>>,
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new() -> (Self, UnboundedReceiver<ProgressEvent>) {
        let (sender, receiver) = unbounded_channel();
        (
            Self {
                sender: Arc::new(sender),
            },
            receiver,
        )
    }

    /// Report a progress event
    pub fn report(&self, event: ProgressEvent) {
        // Ignore send errors (receiver might be dropped)
        let _ = self.sender.send(event);
    }

    /// Report transfer started
    pub fn transfer_started(
        &self,
        operation_id: String,
        key: String,
        total_bytes: u64,
        direction: TransferDirection,
    ) {
        self.report(ProgressEvent::TransferStarted {
            operation_id,
            key,
            total_bytes,
            direction,
        });
    }

    /// Report progress update
    pub fn progress(
        &self,
        operation_id: String,
        bytes_transferred: u64,
        total_bytes: u64,
        rate_bps: f64,
        eta_secs: Option<u64>,
    ) {
        let percentage = if total_bytes > 0 {
            (bytes_transferred as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        self.report(ProgressEvent::Progress {
            operation_id,
            bytes_transferred,
            total_bytes,
            percentage,
            rate_bps,
            eta_secs,
        });
    }

    /// Report chunk completion
    pub fn chunk_completed(&self, operation_id: String, chunk_number: u32, chunk_bytes: u64) {
        self.report(ProgressEvent::ChunkCompleted {
            operation_id,
            chunk_number,
            chunk_bytes,
        });
    }

    /// Report transfer completion
    pub fn transfer_completed(&self, operation_id: String, total_bytes: u64, duration: Duration) {
        self.report(ProgressEvent::TransferCompleted {
            operation_id,
            total_bytes,
            duration,
        });
    }

    /// Report transfer failure
    pub fn transfer_failed(&self, operation_id: String, error: String, bytes_transferred: u64) {
        self.report(ProgressEvent::TransferFailed {
            operation_id,
            error,
            bytes_transferred,
        });
    }

    /// Report batch progress
    pub fn batch_progress(
        &self,
        operation_id: String,
        completed: u64,
        total: u64,
        succeeded: u64,
        failed: u64,
    ) {
        self.report(ProgressEvent::BatchProgress {
            operation_id,
            completed,
            total,
            succeeded,
            failed,
        });
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        let (sender, _) = unbounded_channel();
        Self {
            sender: Arc::new(sender),
        }
    }
}

/// Throughput tracker for calculating transfer rates
pub struct ThroughputTracker {
    start_time: Instant,
    last_update: RwLock<Instant>,
    bytes_transferred: RwLock<u64>,
    last_bytes: RwLock<u64>,
    smoothing_window: Duration,
}

impl ThroughputTracker {
    /// Create a new throughput tracker
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_update: RwLock::new(now),
            bytes_transferred: RwLock::new(0),
            last_bytes: RwLock::new(0),
            smoothing_window: Duration::from_secs(1),
        }
    }

    /// Create with custom smoothing window
    pub fn with_smoothing_window(smoothing_window: Duration) -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_update: RwLock::new(now),
            bytes_transferred: RwLock::new(0),
            last_bytes: RwLock::new(0),
            smoothing_window,
        }
    }

    /// Update with new bytes transferred
    pub async fn update(&self, bytes: u64) {
        let now = Instant::now();
        *self.last_update.write().await = now;
        *self.bytes_transferred.write().await = bytes;
    }

    /// Get current throughput in bytes per second
    pub async fn throughput_bps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }

        let bytes = *self.bytes_transferred.read().await;
        bytes as f64 / elapsed
    }

    /// Get current throughput in MB/s
    pub async fn throughput_mbps(&self) -> f64 {
        self.throughput_bps().await / 1_048_576.0
    }

    /// Get instantaneous throughput (over smoothing window)
    pub async fn instantaneous_throughput_bps(&self) -> f64 {
        let now = Instant::now();
        let last_update = *self.last_update.read().await;
        let elapsed = now.duration_since(last_update).as_secs_f64();

        if elapsed == 0.0 {
            return 0.0;
        }

        let current_bytes = *self.bytes_transferred.read().await;
        let last_bytes = *self.last_bytes.read().await;
        let delta_bytes = current_bytes.saturating_sub(last_bytes);

        *self.last_bytes.write().await = current_bytes;

        delta_bytes as f64 / elapsed
    }

    /// Estimate time remaining (ETA)
    pub async fn eta(&self, total_bytes: u64) -> Option<Duration> {
        let throughput = self.throughput_bps().await;
        if throughput == 0.0 {
            return None;
        }

        let bytes = *self.bytes_transferred.read().await;
        let remaining = total_bytes.saturating_sub(bytes);
        let eta_secs = remaining as f64 / throughput;

        Some(Duration::from_secs_f64(eta_secs))
    }

    /// Get elapsed time since start
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Reset the tracker
    pub async fn reset(&self) {
        *self.bytes_transferred.write().await = 0;
        *self.last_bytes.write().await = 0;
    }
}

impl Default for ThroughputTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress aggregator for combining multiple progress sources
pub struct ProgressAggregator {
    reporters: Arc<RwLock<Vec<ProgressReporter>>>,
}

impl ProgressAggregator {
    /// Create a new progress aggregator
    pub fn new() -> Self {
        Self {
            reporters: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a progress reporter
    pub async fn add_reporter(&self, reporter: ProgressReporter) {
        self.reporters.write().await.push(reporter);
    }

    /// Report to all reporters
    pub async fn report(&self, event: ProgressEvent) {
        let reporters = self.reporters.read().await;
        for reporter in reporters.iter() {
            reporter.report(event.clone());
        }
    }
}

impl Default for ProgressAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Transfer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStats {
    /// Total bytes transferred
    pub total_bytes: u64,

    /// Duration of transfer
    pub duration: Duration,

    /// Average throughput in bytes/sec
    pub avg_throughput_bps: f64,

    /// Peak throughput in bytes/sec
    pub peak_throughput_bps: f64,

    /// Number of chunks (if multipart)
    pub chunks: Option<u32>,

    /// Number of retries
    pub retries: u32,
}

impl TransferStats {
    /// Create new transfer stats
    pub fn new(total_bytes: u64, duration: Duration) -> Self {
        let avg_throughput_bps = if duration.as_secs_f64() > 0.0 {
            total_bytes as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        Self {
            total_bytes,
            duration,
            avg_throughput_bps,
            peak_throughput_bps: avg_throughput_bps,
            chunks: None,
            retries: 0,
        }
    }

    /// Get average throughput in MB/s
    pub fn avg_throughput_mbps(&self) -> f64 {
        self.avg_throughput_bps / 1_048_576.0
    }

    /// Get peak throughput in MB/s
    pub fn peak_throughput_mbps(&self) -> f64 {
        self.peak_throughput_bps / 1_048_576.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_reporter() {
        let (reporter, mut receiver) = ProgressReporter::new();

        reporter.transfer_started(
            "op1".to_string(),
            "test.txt".to_string(),
            1000,
            TransferDirection::Upload,
        );

        let event = receiver.recv().await.unwrap();
        match event {
            ProgressEvent::TransferStarted {
                operation_id,
                total_bytes,
                ..
            } => {
                assert_eq!(operation_id, "op1");
                assert_eq!(total_bytes, 1000);
            }
            _ => panic!("Expected TransferStarted event"),
        }
    }

    #[tokio::test]
    async fn test_throughput_tracker() {
        let tracker = ThroughputTracker::new();

        tracker.update(1_000_000).await; // 1 MB
        tokio::time::sleep(Duration::from_millis(100)).await;

        let throughput = tracker.throughput_bps().await;
        assert!(throughput > 0.0);

        let eta = tracker.eta(10_000_000).await; // 10 MB total
        assert!(eta.is_some());
    }

    #[tokio::test]
    async fn test_progress_aggregator() {
        let aggregator = ProgressAggregator::new();
        let (reporter1, mut receiver1) = ProgressReporter::new();
        let (reporter2, mut receiver2) = ProgressReporter::new();

        aggregator.add_reporter(reporter1).await;
        aggregator.add_reporter(reporter2).await;

        aggregator
            .report(ProgressEvent::TransferStarted {
                operation_id: "op1".to_string(),
                key: "test.txt".to_string(),
                total_bytes: 1000,
                direction: TransferDirection::Upload,
            })
            .await;

        // Both receivers should get the event
        assert!(receiver1.try_recv().is_ok());
        assert!(receiver2.try_recv().is_ok());
    }

    #[test]
    fn test_transfer_stats() {
        let stats = TransferStats::new(10_485_760, Duration::from_secs(10)); // 10 MB in 10 secs

        assert_eq!(stats.total_bytes, 10_485_760);
        assert_eq!(stats.avg_throughput_mbps(), 1.0); // 1 MB/s
    }

    #[test]
    fn test_transfer_direction_display() {
        assert_eq!(TransferDirection::Upload.to_string(), "Upload");
        assert_eq!(TransferDirection::Download.to_string(), "Download");
    }
}
