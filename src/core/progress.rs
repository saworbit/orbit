/*!
 * Progress event publisher for real-time transfer monitoring
 *
 * This module provides a publish-subscribe system for tracking file transfers:
 * - File-level events (start, progress, complete)
 * - Byte-level granularity
 * - Timestamps for telemetry
 * - Support for both TUI rendering and JSON logging
 */

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique identifier for a file transfer operation
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FileId(String);

impl FileId {
    pub fn new(source: &std::path::Path, dest: &std::path::Path) -> Self {
        FileId(format!("{} -> {}", source.display(), dest.display()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Progress event types
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Transfer started
    TransferStart {
        file_id: FileId,
        source: PathBuf,
        dest: PathBuf,
        total_bytes: u64,
        timestamp: u64,
    },

    /// Progress update
    TransferProgress {
        file_id: FileId,
        bytes_transferred: u64,
        total_bytes: u64,
        timestamp: u64,
    },

    /// Transfer completed successfully
    TransferComplete {
        file_id: FileId,
        total_bytes: u64,
        duration_ms: u64,
        checksum: Option<String>,
        timestamp: u64,
    },

    /// Transfer failed
    TransferFailed {
        file_id: FileId,
        error: String,
        bytes_transferred: u64,
        timestamp: u64,
    },

    /// Directory scan started
    DirectoryScanStart {
        path: PathBuf,
        timestamp: u64,
    },

    /// Directory scan progress
    DirectoryScanProgress {
        files_found: u64,
        dirs_found: u64,
        timestamp: u64,
    },

    /// Directory scan completed
    DirectoryScanComplete {
        total_files: u64,
        total_dirs: u64,
        timestamp: u64,
    },

    /// Batch operation completed
    BatchComplete {
        files_succeeded: u64,
        files_failed: u64,
        total_bytes: u64,
        duration_ms: u64,
        timestamp: u64,
    },

    /// Resume decision made for interrupted transfer
    ResumeDecision {
        file_id: FileId,
        decision: String,
        from_offset: u64,
        verified_chunks: usize,
        reason: Option<String>,
        timestamp: u64,
    },

    /// Chunk verification started
    ChunkVerification {
        file_id: FileId,
        chunk_id: u32,
        chunk_size: u64,
        timestamp: u64,
    },

    /// Chunk verified successfully
    ChunkVerified {
        file_id: FileId,
        chunk_id: u32,
        digest: String,
        timestamp: u64,
    },
}

impl ProgressEvent {
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Create a transfer start event
    pub fn transfer_start(source: PathBuf, dest: PathBuf, total_bytes: u64) -> Self {
        let file_id = FileId::new(&source, &dest);
        ProgressEvent::TransferStart {
            file_id,
            source,
            dest,
            total_bytes,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create a progress update event
    pub fn transfer_progress(file_id: FileId, bytes_transferred: u64, total_bytes: u64) -> Self {
        ProgressEvent::TransferProgress {
            file_id,
            bytes_transferred,
            total_bytes,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create a completion event
    pub fn transfer_complete(
        file_id: FileId,
        total_bytes: u64,
        duration_ms: u64,
        checksum: Option<String>,
    ) -> Self {
        ProgressEvent::TransferComplete {
            file_id,
            total_bytes,
            duration_ms,
            checksum,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create a failure event
    pub fn transfer_failed(file_id: FileId, error: String, bytes_transferred: u64) -> Self {
        ProgressEvent::TransferFailed {
            file_id,
            error,
            bytes_transferred,
            timestamp: Self::current_timestamp(),
        }
    }
}

/// Progress publisher - sends events to subscribers
#[derive(Clone)]
pub struct ProgressPublisher {
    sender: Option<Sender<ProgressEvent>>,
}

impl ProgressPublisher {
    /// Create a new publisher with bounded channel
    pub fn new(buffer_size: usize) -> (Self, ProgressSubscriber) {
        let (tx, rx) = bounded(buffer_size);
        (
            ProgressPublisher { sender: Some(tx) },
            ProgressSubscriber { receiver: rx },
        )
    }

    /// Create a new publisher with unbounded channel
    pub fn unbounded() -> (Self, ProgressSubscriber) {
        let (tx, rx) = unbounded();
        (
            ProgressPublisher { sender: Some(tx) },
            ProgressSubscriber { receiver: rx },
        )
    }

    /// Create a no-op publisher (for when progress tracking is disabled)
    pub fn noop() -> Self {
        ProgressPublisher { sender: None }
    }

    /// Publish an event
    pub fn publish(&self, event: ProgressEvent) {
        if let Some(ref tx) = self.sender {
            let _ = tx.send(event); // Ignore send errors (subscriber may have dropped)
        }
    }

    /// Publish a transfer start event
    pub fn start_transfer(&self, source: PathBuf, dest: PathBuf, total_bytes: u64) -> FileId {
        let file_id = FileId::new(&source, &dest);
        self.publish(ProgressEvent::transfer_start(source, dest, total_bytes));
        file_id
    }

    /// Publish progress
    pub fn update_progress(&self, file_id: &FileId, bytes_transferred: u64, total_bytes: u64) {
        self.publish(ProgressEvent::transfer_progress(
            file_id.clone(),
            bytes_transferred,
            total_bytes,
        ));
    }

    /// Publish completion
    pub fn complete_transfer(
        &self,
        file_id: FileId,
        total_bytes: u64,
        duration_ms: u64,
        checksum: Option<String>,
    ) {
        self.publish(ProgressEvent::transfer_complete(
            file_id,
            total_bytes,
            duration_ms,
            checksum,
        ));
    }

    /// Publish failure
    pub fn fail_transfer(&self, file_id: FileId, error: String, bytes_transferred: u64) {
        self.publish(ProgressEvent::transfer_failed(
            file_id,
            error,
            bytes_transferred,
        ));
    }
}

/// Progress subscriber - receives events
pub struct ProgressSubscriber {
    receiver: Receiver<ProgressEvent>,
}

impl ProgressSubscriber {
    /// Get the receiver for consuming events
    pub fn receiver(&self) -> &Receiver<ProgressEvent> {
        &self.receiver
    }

    /// Try to receive an event (non-blocking)
    pub fn try_recv(&self) -> Option<ProgressEvent> {
        self.receiver.try_recv().ok()
    }

    /// Receive an event (blocking)
    pub fn recv(&self) -> Option<ProgressEvent> {
        self.receiver.recv().ok()
    }

    /// Create an iterator over events
    pub fn iter(&self) -> impl Iterator<Item = ProgressEvent> + '_ {
        self.receiver.iter()
    }
}

/// Shared progress publisher that can be cloned across threads
pub type SharedProgressPublisher = Arc<ProgressPublisher>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_file_id_creation() {
        let source = Path::new("/source/file.txt");
        let dest = Path::new("/dest/file.txt");
        let file_id = FileId::new(source, dest);
        assert!(file_id.as_str().contains("source"));
        assert!(file_id.as_str().contains("dest"));
    }

    #[test]
    fn test_publisher_subscriber() {
        let (publisher, subscriber) = ProgressPublisher::new(10);

        let source = PathBuf::from("/source/test.txt");
        let dest = PathBuf::from("/dest/test.txt");

        publisher.start_transfer(source.clone(), dest.clone(), 1024);

        let event = subscriber.try_recv().unwrap();
        match event {
            ProgressEvent::TransferStart {
                total_bytes,
                source: s,
                dest: d,
                ..
            } => {
                assert_eq!(total_bytes, 1024);
                assert_eq!(s, source);
                assert_eq!(d, dest);
            }
            _ => panic!("Expected TransferStart event"),
        }
    }

    #[test]
    fn test_noop_publisher() {
        let publisher = ProgressPublisher::noop();
        // Should not panic
        publisher.publish(ProgressEvent::transfer_start(
            PathBuf::from("/test"),
            PathBuf::from("/test2"),
            100,
        ));
    }

    #[test]
    fn test_event_sequence() {
        let (publisher, subscriber) = ProgressPublisher::unbounded();

        let source = PathBuf::from("/source/file.txt");
        let dest = PathBuf::from("/dest/file.txt");
        let file_id = publisher.start_transfer(source, dest, 1000);

        publisher.update_progress(&file_id, 500, 1000);
        publisher.complete_transfer(file_id.clone(), 1000, 100, None);

        let events: Vec<_> = subscriber.receiver.try_iter().collect();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], ProgressEvent::TransferStart { .. }));
        assert!(matches!(events[1], ProgressEvent::TransferProgress { .. }));
        assert!(matches!(events[2], ProgressEvent::TransferComplete { .. }));
    }
}
