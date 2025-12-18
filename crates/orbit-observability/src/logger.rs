//! Unified logger for audit events with cryptographic chaining
//!
//! This module provides the `UnifiedLogger` which combines event emission
//! with cryptographic audit chaining and persistent storage.

use crate::chain::AuditChain;
use crate::context::TraceContext;
use crate::event::{EventPayload, OrbitEvent};
use crate::signer::AuditSigner;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Errors that can occur during logging operations
#[derive(Debug, Error)]
pub enum LoggerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Chain error: {0}")]
    Chain(#[from] crate::chain::ChainError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Logger is disabled")]
    Disabled,
}

/// Result type for logger operations
pub type Result<T> = std::result::Result<T, LoggerError>;

/// Unified logger for Orbit observability events
///
/// UnifiedLogger provides:
/// - Event emission with automatic trace context
/// - Cryptographic chaining via AuditChain
/// - Persistent storage in JSON Lines format
/// - Thread-safe operation
///
/// ## Example
///
/// ```no_run
/// use orbit_observability::{UnifiedLogger, AuditSigner, EventPayload, TraceContext};
/// use std::path::Path;
///
/// let signer = AuditSigner::from_bytes(b"secret");
/// let logger = UnifiedLogger::new(Some(Path::new("audit.jsonl")), signer).unwrap();
///
/// let ctx = TraceContext::new_root().with_job("job-1".to_string());
/// logger.emit_with_context(
///     &ctx,
///     EventPayload::JobStart {
///         files: 10,
///         total_bytes: 1024000,
///         protocol: "s3".to_string(),
///     },
/// ).unwrap();
/// ```
pub struct UnifiedLogger {
    inner: Arc<Mutex<LoggerInner>>,
}

struct LoggerInner {
    writer: Option<BufWriter<File>>,
    chain: AuditChain,
    path: Option<PathBuf>,
}

impl UnifiedLogger {
    /// Create a new unified logger
    ///
    /// If `path` is None, events are not persisted to disk (useful for testing).
    /// If `path` is Some, events are appended to the file in JSON Lines format.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created/opened.
    pub fn new(path: Option<&Path>, signer: AuditSigner) -> Result<Self> {
        let writer = if let Some(p) = path {
            // Create parent directories if needed
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file = OpenOptions::new().create(true).append(true).open(p)?;

            Some(BufWriter::new(file))
        } else {
            None
        };

        let chain = AuditChain::new(signer);

        Ok(Self {
            inner: Arc::new(Mutex::new(LoggerInner {
                writer,
                chain,
                path: path.map(|p| p.to_path_buf()),
            })),
        })
    }

    /// Create a no-op logger that discards all events
    ///
    /// This is useful when audit logging is disabled but the code still
    /// needs a logger instance.
    pub fn disabled() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LoggerInner {
                writer: None,
                chain: AuditChain::new(AuditSigner::from_bytes(b"disabled")),
                path: None,
            })),
        }
    }

    /// Emit an event with the given trace context
    ///
    /// This is the primary method for emitting events. It:
    /// 1. Creates an OrbitEvent from the payload
    /// 2. Attaches the trace context
    /// 3. Signs the event with the audit chain
    /// 4. Writes to the log file
    pub fn emit_with_context(&self, ctx: &TraceContext, payload: EventPayload) -> Result<()> {
        let mut event =
            OrbitEvent::new(payload).with_trace(ctx.trace_id.clone(), ctx.span_id.clone());

        if let Some(ref parent) = ctx.parent_span_id {
            event.parent_span_id = Some(parent.clone());
        }
        if let Some(ref job) = ctx.job_id {
            event.job_id = Some(job.clone());
        }
        if let Some(ref file) = ctx.file_id {
            event.file_id = Some(file.clone());
        }

        self.emit(event)
    }

    /// Emit a pre-constructed event
    ///
    /// This method signs the event and writes it to the log file.
    /// Prefer `emit_with_context` for most use cases.
    pub fn emit(&self, mut event: OrbitEvent) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();

        // Sign the event with the audit chain
        inner.chain.sign_event(&mut event)?;

        // Write to file if enabled
        if let Some(ref mut writer) = inner.writer {
            let json = serde_json::to_string(&event)?;
            writeln!(writer, "{}", json)?;
            writer.flush()?; // Ensure durability
        }

        Ok(())
    }

    /// Flush the underlying writer
    ///
    /// This ensures all buffered events are written to disk.
    pub fn flush(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(ref mut writer) = inner.writer {
            writer.flush()?;
        }
        Ok(())
    }

    /// Get the current sequence number
    pub fn current_sequence(&self) -> u64 {
        let inner = self.inner.lock().unwrap();
        inner.chain.current_sequence()
    }

    /// Get the log file path (if any)
    pub fn path(&self) -> Option<PathBuf> {
        let inner = self.inner.lock().unwrap();
        inner.path.clone()
    }

    // Convenience methods for common event types

    /// Emit a job start event
    pub fn emit_job_start(
        &self,
        ctx: &TraceContext,
        files: u32,
        total_bytes: u64,
        protocol: &str,
    ) -> Result<()> {
        self.emit_with_context(
            ctx,
            EventPayload::JobStart {
                files,
                total_bytes,
                protocol: protocol.to_string(),
            },
        )
    }

    /// Emit a job complete event
    pub fn emit_job_complete(
        &self,
        ctx: &TraceContext,
        duration_ms: u64,
        digest: &str,
    ) -> Result<()> {
        self.emit_with_context(
            ctx,
            EventPayload::JobComplete {
                duration_ms,
                digest: digest.to_string(),
            },
        )
    }

    /// Emit a job failed event
    pub fn emit_job_failed(&self, ctx: &TraceContext, error: &str, retries: u32) -> Result<()> {
        self.emit_with_context(
            ctx,
            EventPayload::JobFailed {
                error: error.to_string(),
                retries,
            },
        )
    }

    /// Emit a file start event
    pub fn emit_file_start(
        &self,
        ctx: &TraceContext,
        source: &str,
        dest: &str,
        bytes: u64,
    ) -> Result<()> {
        self.emit_with_context(
            ctx,
            EventPayload::FileStart {
                source: source.to_string(),
                dest: dest.to_string(),
                bytes,
            },
        )
    }

    /// Emit a file complete event
    pub fn emit_file_complete(
        &self,
        ctx: &TraceContext,
        bytes: u64,
        duration_ms: u64,
        checksum: &str,
    ) -> Result<()> {
        self.emit_with_context(
            ctx,
            EventPayload::FileComplete {
                bytes,
                duration_ms,
                checksum: checksum.to_string(),
            },
        )
    }

    /// Emit a file failed event
    pub fn emit_file_failed(
        &self,
        ctx: &TraceContext,
        error: &str,
        bytes_transferred: u64,
    ) -> Result<()> {
        self.emit_with_context(
            ctx,
            EventPayload::FileFailed {
                error: error.to_string(),
                bytes_transferred,
            },
        )
    }
}

// Allow cloning to share logger across threads
impl Clone for UnifiedLogger {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_logger() -> (UnifiedLogger, NamedTempFile) {
        let temp_file = NamedTempFile::new().unwrap();
        let signer = AuditSigner::from_bytes(b"test_secret");
        let logger = UnifiedLogger::new(Some(temp_file.path()), signer).unwrap();
        (logger, temp_file)
    }

    #[test]
    fn test_create_logger() {
        let (logger, _temp) = create_test_logger();
        assert_eq!(logger.current_sequence(), 0);
    }

    #[test]
    fn test_emit_event() {
        let (logger, temp) = create_test_logger();
        let ctx = TraceContext::new_root().with_job("job-1".to_string());

        logger.emit_job_start(&ctx, 10, 1024, "s3").unwrap();

        assert_eq!(logger.current_sequence(), 1);

        // Verify file contents
        logger.flush().unwrap();
        let contents = std::fs::read_to_string(temp.path()).unwrap();
        assert!(contents.contains("\"type\":\"job_start\""));
        assert!(contents.contains("\"job_id\":\"job-1\""));
        assert!(contents.contains("\"integrity_hash\""));
    }

    #[test]
    fn test_emit_multiple_events() {
        let (logger, temp) = create_test_logger();
        let ctx = TraceContext::new_root();

        logger.emit_job_start(&ctx, 5, 512, "local").unwrap();
        logger.emit_file_start(&ctx, "/src", "/dst", 100).unwrap();
        logger
            .emit_file_complete(&ctx, 100, 50, "blake3:abc")
            .unwrap();

        assert_eq!(logger.current_sequence(), 3);

        logger.flush().unwrap();
        let contents = std::fs::read_to_string(temp.path()).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_events_are_chained() {
        let (logger, temp) = create_test_logger();
        let ctx = TraceContext::new_root();

        logger.emit_job_start(&ctx, 1, 100, "local").unwrap();
        logger.emit_job_complete(&ctx, 50, "digest").unwrap();

        logger.flush().unwrap();
        let contents = std::fs::read_to_string(temp.path()).unwrap();

        let lines: Vec<&str> = contents.lines().collect();
        let event1: OrbitEvent = serde_json::from_str(lines[0]).unwrap();
        let event2: OrbitEvent = serde_json::from_str(lines[1]).unwrap();

        // Events should have different hashes
        assert_ne!(event1.integrity_hash, event2.integrity_hash);

        // Sequences should be sequential
        assert_eq!(event1.sequence, 0);
        assert_eq!(event2.sequence, 1);
    }

    #[test]
    fn test_trace_context_propagation() {
        let (logger, temp) = create_test_logger();
        let ctx = TraceContext::new_root()
            .with_job("job-123".to_string())
            .with_file_id("file-456".to_string());

        logger.emit_file_start(&ctx, "/a", "/b", 100).unwrap();

        logger.flush().unwrap();
        let contents = std::fs::read_to_string(temp.path()).unwrap();
        let event: OrbitEvent = serde_json::from_str(contents.lines().next().unwrap()).unwrap();

        assert_eq!(event.trace_id, ctx.trace_id);
        assert_eq!(event.span_id, ctx.span_id);
        assert_eq!(event.job_id, Some("job-123".to_string()));
        assert_eq!(event.file_id, Some("file-456".to_string()));
    }

    #[test]
    fn test_disabled_logger() {
        let logger = UnifiedLogger::disabled();
        let ctx = TraceContext::new_root();

        // Should not panic
        logger.emit_job_start(&ctx, 1, 100, "local").unwrap();
        assert_eq!(logger.current_sequence(), 1);
        assert!(logger.path().is_none());
    }

    #[test]
    fn test_clone_shares_state() {
        let (logger1, temp) = create_test_logger();
        let logger2 = logger1.clone();
        let ctx = TraceContext::new_root();

        logger1.emit_job_start(&ctx, 1, 100, "local").unwrap();
        logger2.emit_job_complete(&ctx, 50, "digest").unwrap();

        // Both loggers should share the same sequence
        assert_eq!(logger1.current_sequence(), 2);
        assert_eq!(logger2.current_sequence(), 2);

        logger1.flush().unwrap();
        let contents = std::fs::read_to_string(temp.path()).unwrap();
        assert_eq!(contents.lines().count(), 2);
    }

    #[test]
    fn test_convenience_methods() {
        let (logger, _temp) = create_test_logger();
        let ctx = TraceContext::new_root();

        logger.emit_job_start(&ctx, 1, 100, "s3").unwrap();
        logger.emit_job_complete(&ctx, 50, "digest").unwrap();
        logger.emit_job_failed(&ctx, "error", 3).unwrap();
        logger.emit_file_start(&ctx, "/a", "/b", 100).unwrap();
        logger
            .emit_file_complete(&ctx, 100, 10, "checksum")
            .unwrap();
        logger.emit_file_failed(&ctx, "error", 50).unwrap();

        assert_eq!(logger.current_sequence(), 6);
    }
}
