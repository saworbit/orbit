//! Typed Provenance Events: Structured event taxonomy for audit queries
//!
//! Instead of generic log entries, provenance events have a formal taxonomy
//! that enables structured querying like "show me all chunks that were healed
//! in the last 24h" without grep.
//!
//! # Event Types
//!
//! Each event type captures a specific lifecycle transition:
//! - **ChunkCreated**: A new chunk was produced by CDC
//! - **ChunkDeduplicated**: A chunk was recognized as duplicate (skipped transfer)
//! - **ChunkTransferred**: A chunk was sent to a destination
//! - **ChunkVerified**: A chunk's checksum was verified after transfer
//! - **ChunkHealed**: A chunk was replicated by Sentinel to restore redundancy
//! - **ChunkPenalized**: A chunk was temporarily deprioritized after failure
//! - **ChunkDeadLettered**: A chunk was routed to the dead-letter queue
//! - **FileRenamed**: A file was detected as renamed via content-aware detection
//! - **FileSkipped**: A file was skipped (up-to-date, excluded by filter, etc.)
//! - **JobCreated**: A new transfer job was created
//! - **JobResumed**: A job was resumed from a checkpoint
//! - **JobCompleted**: A job finished (possibly with partial success)
//! - **JobFailed**: A job failed entirely
//!
//! # Example
//!
//! ```
//! use orbit_core_audit::provenance::{ProvenanceEvent, ProvenanceType};
//!
//! let event = ProvenanceEvent::new(
//!     ProvenanceType::ChunkTransferred,
//!     "job-42",
//! )
//! .with_chunk_hash("abc123def456")
//! .with_source("star-1", "/data/file.bin")
//! .with_destination("star-2", "/backup/file.bin")
//! .with_bytes(4096);
//!
//! let json = serde_json::to_string(&event).unwrap();
//! assert!(json.contains("chunk_transferred"));
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Typed provenance event categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceType {
    // ── Chunk lifecycle ──
    /// A new chunk was produced by CDC
    ChunkCreated,
    /// A chunk was recognized as duplicate (transfer skipped)
    ChunkDeduplicated,
    /// A chunk was transferred to a destination
    ChunkTransferred,
    /// A chunk's integrity was verified after transfer
    ChunkVerified,
    /// A chunk was replicated by Sentinel to restore redundancy
    ChunkHealed,
    /// A chunk was temporarily deprioritized after transient failure
    ChunkPenalized,
    /// A chunk was routed to the dead-letter queue
    ChunkDeadLettered,
    /// A packed container received a new chunk (chunk packing)
    ChunkPacked,

    // ── File lifecycle ──
    /// A file was detected as renamed via content-aware detection
    FileRenamed,
    /// A file was skipped (up-to-date, filtered, etc.)
    FileSkipped,
    /// A file transfer started
    FileStarted,
    /// A file transfer completed
    FileCompleted,

    // ── Job lifecycle ──
    /// A new transfer job was created
    JobCreated,
    /// A job was resumed from a checkpoint
    JobResumed,
    /// A job finished (possibly with partial success)
    JobCompleted,
    /// A job failed entirely
    JobFailed,

    // ── Grid lifecycle ──
    /// A Star joined the Grid
    StarRegistered,
    /// A Star was scheduled to receive work
    StarScheduled,
    /// A Star is draining (finishing current work)
    StarDraining,
    /// A Star left the Grid
    StarDeregistered,
}

impl ProvenanceType {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ProvenanceType::ChunkCreated => "chunk_created",
            ProvenanceType::ChunkDeduplicated => "chunk_deduplicated",
            ProvenanceType::ChunkTransferred => "chunk_transferred",
            ProvenanceType::ChunkVerified => "chunk_verified",
            ProvenanceType::ChunkHealed => "chunk_healed",
            ProvenanceType::ChunkPenalized => "chunk_penalized",
            ProvenanceType::ChunkDeadLettered => "chunk_dead_lettered",
            ProvenanceType::ChunkPacked => "chunk_packed",
            ProvenanceType::FileRenamed => "file_renamed",
            ProvenanceType::FileSkipped => "file_skipped",
            ProvenanceType::FileStarted => "file_started",
            ProvenanceType::FileCompleted => "file_completed",
            ProvenanceType::JobCreated => "job_created",
            ProvenanceType::JobResumed => "job_resumed",
            ProvenanceType::JobCompleted => "job_completed",
            ProvenanceType::JobFailed => "job_failed",
            ProvenanceType::StarRegistered => "star_registered",
            ProvenanceType::StarScheduled => "star_scheduled",
            ProvenanceType::StarDraining => "star_draining",
            ProvenanceType::StarDeregistered => "star_deregistered",
        }
    }

    /// Is this a chunk-level event?
    pub fn is_chunk_event(&self) -> bool {
        matches!(
            self,
            ProvenanceType::ChunkCreated
                | ProvenanceType::ChunkDeduplicated
                | ProvenanceType::ChunkTransferred
                | ProvenanceType::ChunkVerified
                | ProvenanceType::ChunkHealed
                | ProvenanceType::ChunkPenalized
                | ProvenanceType::ChunkDeadLettered
                | ProvenanceType::ChunkPacked
        )
    }

    /// Is this a job-level event?
    pub fn is_job_event(&self) -> bool {
        matches!(
            self,
            ProvenanceType::JobCreated
                | ProvenanceType::JobResumed
                | ProvenanceType::JobCompleted
                | ProvenanceType::JobFailed
        )
    }
}

/// A structured provenance event with typed metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEvent {
    /// Timestamp (UTC)
    pub ts: DateTime<Utc>,

    /// Event type
    pub event_type: ProvenanceType,

    /// Job ID this event belongs to
    pub job_id: String,

    /// Chunk content hash (BLAKE3, hex-encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_hash: Option<String>,

    /// Source Star ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_star: Option<String>,

    /// Source path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,

    /// Destination Star ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_star: Option<String>,

    /// Destination path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_path: Option<String>,

    /// Bytes involved in this event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,

    /// Duration of the operation in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Error message (for failure events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Additional context (free-form key-value pairs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<std::collections::HashMap<String, String>>,
}

impl ProvenanceEvent {
    /// Create a new provenance event
    pub fn new(event_type: ProvenanceType, job_id: &str) -> Self {
        Self {
            ts: Utc::now(),
            event_type,
            job_id: job_id.to_string(),
            chunk_hash: None,
            source_star: None,
            source_path: None,
            dest_star: None,
            dest_path: None,
            bytes: None,
            duration_ms: None,
            error: None,
            context: None,
        }
    }

    /// Set the chunk hash
    pub fn with_chunk_hash(mut self, hash: &str) -> Self {
        self.chunk_hash = Some(hash.to_string());
        self
    }

    /// Set the source location
    pub fn with_source(mut self, star_id: &str, path: &str) -> Self {
        self.source_star = Some(star_id.to_string());
        self.source_path = Some(path.to_string());
        self
    }

    /// Set the destination location
    pub fn with_destination(mut self, star_id: &str, path: &str) -> Self {
        self.dest_star = Some(star_id.to_string());
        self.dest_path = Some(path.to_string());
        self
    }

    /// Set the byte count
    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes = Some(bytes);
        self
    }

    /// Set the duration in milliseconds
    pub fn with_duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    /// Set an error message
    pub fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    /// Add a context key-value pair
    pub fn with_context(mut self, key: &str, value: &str) -> Self {
        self.context
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(key.to_string(), value.to_string());
        self
    }
}

/// Provenance logger that writes typed events to a JSON Lines file
pub struct ProvenanceLogger {
    path: std::path::PathBuf,
    writer: std::sync::Arc<std::sync::Mutex<std::io::BufWriter<std::fs::File>>>,
}

impl ProvenanceLogger {
    /// Create a new provenance logger writing to the given path
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> crate::Result<Self> {
        let path = path.as_ref().to_path_buf();

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|_| crate::Error::create_failed(&path))?;

        let writer = std::sync::Arc::new(std::sync::Mutex::new(std::io::BufWriter::new(file)));

        Ok(Self { path, writer })
    }

    /// Log a provenance event
    pub fn log(&mut self, event: &ProvenanceEvent) -> crate::Result<()> {
        use std::io::Write;
        let json = serde_json::to_string(event)?;
        let mut w = self.writer.lock().unwrap();
        writeln!(w, "{}", json)?;
        w.flush()?;
        Ok(())
    }

    /// Flush the log
    pub fn flush(&mut self) -> crate::Result<()> {
        use std::io::Write;
        let mut w = self.writer.lock().unwrap();
        w.flush()?;
        Ok(())
    }

    /// Get the path to the log file
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

/// Parse provenance events from a JSON Lines file
pub fn parse_provenance_log<P: AsRef<std::path::Path>>(
    path: P,
) -> crate::Result<Vec<ProvenanceEvent>> {
    let contents = std::fs::read_to_string(path)?;
    let mut events = Vec::new();

    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Try to parse as ProvenanceEvent first
        if let Ok(event) = serde_json::from_str::<ProvenanceEvent>(line) {
            events.push(event);
        }
        // Skip non-provenance lines (legacy TelemetryEvent format)
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_type_serialization() {
        let t = ProvenanceType::ChunkTransferred;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"chunk_transferred\"");

        let parsed: ProvenanceType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ProvenanceType::ChunkTransferred);
    }

    #[test]
    fn test_provenance_type_classification() {
        assert!(ProvenanceType::ChunkCreated.is_chunk_event());
        assert!(!ProvenanceType::ChunkCreated.is_job_event());
        assert!(ProvenanceType::JobCompleted.is_job_event());
        assert!(!ProvenanceType::JobCompleted.is_chunk_event());
    }

    #[test]
    fn test_event_builder() {
        let event = ProvenanceEvent::new(ProvenanceType::ChunkTransferred, "job-42")
            .with_chunk_hash("abc123")
            .with_source("star-1", "/data/file.bin")
            .with_destination("star-2", "/backup/file.bin")
            .with_bytes(4096)
            .with_duration_ms(150);

        assert_eq!(event.event_type, ProvenanceType::ChunkTransferred);
        assert_eq!(event.job_id, "job-42");
        assert_eq!(event.chunk_hash.as_deref(), Some("abc123"));
        assert_eq!(event.source_star.as_deref(), Some("star-1"));
        assert_eq!(event.dest_star.as_deref(), Some("star-2"));
        assert_eq!(event.bytes, Some(4096));
        assert_eq!(event.duration_ms, Some(150));
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = ProvenanceEvent::new(ProvenanceType::ChunkHealed, "job-99")
            .with_chunk_hash("deadbeef")
            .with_bytes(8192)
            .with_context("reason", "below_redundancy_threshold");

        let json = serde_json::to_string(&event).unwrap();
        let parsed: ProvenanceEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.event_type, ProvenanceType::ChunkHealed);
        assert_eq!(parsed.job_id, "job-99");
        assert_eq!(parsed.chunk_hash.as_deref(), Some("deadbeef"));
        assert_eq!(
            parsed.context.as_ref().unwrap().get("reason").unwrap(),
            "below_redundancy_threshold"
        );
    }

    #[test]
    fn test_event_skips_none_fields() {
        let event = ProvenanceEvent::new(ProvenanceType::JobCreated, "job-1");
        let json = serde_json::to_string(&event).unwrap();

        // None fields should not appear in JSON
        assert!(!json.contains("chunk_hash"));
        assert!(!json.contains("source_star"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_all_provenance_types() {
        let types = vec![
            ProvenanceType::ChunkCreated,
            ProvenanceType::ChunkDeduplicated,
            ProvenanceType::ChunkTransferred,
            ProvenanceType::ChunkVerified,
            ProvenanceType::ChunkHealed,
            ProvenanceType::ChunkPenalized,
            ProvenanceType::ChunkDeadLettered,
            ProvenanceType::ChunkPacked,
            ProvenanceType::FileRenamed,
            ProvenanceType::FileSkipped,
            ProvenanceType::FileStarted,
            ProvenanceType::FileCompleted,
            ProvenanceType::JobCreated,
            ProvenanceType::JobResumed,
            ProvenanceType::JobCompleted,
            ProvenanceType::JobFailed,
            ProvenanceType::StarRegistered,
            ProvenanceType::StarScheduled,
            ProvenanceType::StarDraining,
            ProvenanceType::StarDeregistered,
        ];

        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let parsed: ProvenanceType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, t);
            assert!(!t.as_str().is_empty());
        }
    }

    #[test]
    fn test_provenance_logger_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("prov.jsonl");

        let mut logger = ProvenanceLogger::new(&log_path).unwrap();

        let e1 = ProvenanceEvent::new(ProvenanceType::ChunkCreated, "job-1")
            .with_chunk_hash("hash1")
            .with_bytes(1024);
        let e2 = ProvenanceEvent::new(ProvenanceType::ChunkTransferred, "job-1")
            .with_chunk_hash("hash2")
            .with_source("star-a", "/src/file.bin")
            .with_destination("star-b", "/dst/file.bin");
        let e3 = ProvenanceEvent::new(ProvenanceType::JobCompleted, "job-1").with_duration_ms(500);

        logger.log(&e1).unwrap();
        logger.log(&e2).unwrap();
        logger.log(&e3).unwrap();
        drop(logger);

        let contents = std::fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 3);

        let parsed1: ProvenanceEvent = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed1.event_type, ProvenanceType::ChunkCreated);
        assert_eq!(parsed1.job_id, "job-1");
        assert_eq!(parsed1.chunk_hash.as_deref(), Some("hash1"));
        assert_eq!(parsed1.bytes, Some(1024));

        let parsed2: ProvenanceEvent = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(parsed2.event_type, ProvenanceType::ChunkTransferred);
        assert_eq!(parsed2.chunk_hash.as_deref(), Some("hash2"));
        assert_eq!(parsed2.source_star.as_deref(), Some("star-a"));
        assert_eq!(parsed2.dest_star.as_deref(), Some("star-b"));

        let parsed3: ProvenanceEvent = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(parsed3.event_type, ProvenanceType::JobCompleted);
        assert_eq!(parsed3.duration_ms, Some(500));
    }

    #[test]
    fn test_parse_provenance_log_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("empty.jsonl");
        std::fs::write(&log_path, "").unwrap();

        let events = parse_provenance_log(&log_path).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_provenance_log_with_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("blanks.jsonl");

        let e1 = ProvenanceEvent::new(ProvenanceType::ChunkCreated, "job-1");
        let e2 = ProvenanceEvent::new(ProvenanceType::JobFailed, "job-2");
        let line1 = serde_json::to_string(&e1).unwrap();
        let line2 = serde_json::to_string(&e2).unwrap();

        let contents = format!("{}\n\n\n{}\n\n", line1, line2);
        std::fs::write(&log_path, contents).unwrap();

        let events = parse_provenance_log(&log_path).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, ProvenanceType::ChunkCreated);
        assert_eq!(events[1].event_type, ProvenanceType::JobFailed);
    }

    #[test]
    fn test_parse_provenance_log_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let bad_path = dir.path().join("does_not_exist.jsonl");

        let result = parse_provenance_log(&bad_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_with_error_builder() {
        let event = ProvenanceEvent::new(ProvenanceType::JobFailed, "job-err")
            .with_error("connection timed out");

        assert_eq!(event.error.as_deref(), Some("connection timed out"));
    }

    #[test]
    fn test_with_context_multiple_entries() {
        let event = ProvenanceEvent::new(ProvenanceType::ChunkHealed, "job-ctx")
            .with_context("reason", "below_threshold")
            .with_context("replica_count", "1")
            .with_context("target_count", "3");

        let ctx = event.context.as_ref().unwrap();
        assert_eq!(ctx.len(), 3);
        assert_eq!(ctx.get("reason").unwrap(), "below_threshold");
        assert_eq!(ctx.get("replica_count").unwrap(), "1");
        assert_eq!(ctx.get("target_count").unwrap(), "3");
    }

    #[test]
    fn test_with_context_duplicate_key_overwrites() {
        let event = ProvenanceEvent::new(ProvenanceType::ChunkPenalized, "job-dup")
            .with_context("retry", "first")
            .with_context("retry", "second");

        let ctx = event.context.as_ref().unwrap();
        assert_eq!(ctx.len(), 1);
        assert_eq!(ctx.get("retry").unwrap(), "second");
    }

    #[test]
    fn test_as_str_matches_serde() {
        let all_types = vec![
            ProvenanceType::ChunkCreated,
            ProvenanceType::ChunkDeduplicated,
            ProvenanceType::ChunkTransferred,
            ProvenanceType::ChunkVerified,
            ProvenanceType::ChunkHealed,
            ProvenanceType::ChunkPenalized,
            ProvenanceType::ChunkDeadLettered,
            ProvenanceType::ChunkPacked,
            ProvenanceType::FileRenamed,
            ProvenanceType::FileSkipped,
            ProvenanceType::FileStarted,
            ProvenanceType::FileCompleted,
            ProvenanceType::JobCreated,
            ProvenanceType::JobResumed,
            ProvenanceType::JobCompleted,
            ProvenanceType::JobFailed,
            ProvenanceType::StarRegistered,
            ProvenanceType::StarScheduled,
            ProvenanceType::StarDraining,
            ProvenanceType::StarDeregistered,
        ];

        for t in all_types {
            let serde_str = serde_json::to_string(&t).unwrap();
            // serde wraps in quotes, e.g. "\"chunk_created\""
            let stripped = serde_str.trim_matches('"');
            assert_eq!(
                t.as_str(),
                stripped,
                "as_str() mismatch for {:?}: as_str={}, serde={}",
                t,
                t.as_str(),
                stripped
            );
        }
    }

    #[test]
    fn test_file_events_are_neither_chunk_nor_job() {
        for t in &[ProvenanceType::FileRenamed, ProvenanceType::FileSkipped] {
            assert!(!t.is_chunk_event(), "{:?} should not be a chunk event", t);
            assert!(!t.is_job_event(), "{:?} should not be a job event", t);
        }
    }

    #[test]
    fn test_grid_events_are_neither_chunk_nor_job() {
        for t in &[
            ProvenanceType::StarRegistered,
            ProvenanceType::StarDeregistered,
        ] {
            assert!(!t.is_chunk_event(), "{:?} should not be a chunk event", t);
            assert!(!t.is_job_event(), "{:?} should not be a job event", t);
        }
    }

    #[test]
    fn test_logger_parent_directory_creation() {
        let dir = tempfile::tempdir().unwrap();
        let nested_path = dir.path().join("a").join("b").join("c").join("prov.jsonl");

        // Parent directories do not exist yet
        assert!(!nested_path.parent().unwrap().exists());

        let logger = ProvenanceLogger::new(&nested_path).unwrap();

        // Parent directories should now exist
        assert!(nested_path.parent().unwrap().exists());
        assert_eq!(logger.path(), nested_path);
    }
}
