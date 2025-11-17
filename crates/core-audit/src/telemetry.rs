//! Telemetry logging for transfer operations
//!
//! Provides append-only JSON Lines logging of transfer events for audit,
//! compliance, and performance analysis.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

/// Telemetry logger for append-only event logging
///
/// # Thread Safety
/// The logger uses internal locking and is safe to share across threads.
///
/// # Example
/// ```no_run
/// use orbit_core_audit::TelemetryLogger;
///
/// let mut logger = TelemetryLogger::new("audit.jsonl").unwrap();
/// logger.log_job_start("job-123", 5, 1024000).unwrap();
/// logger.log_window_ok("job-123", "file.bin", 0, 16777216, 2).unwrap();
/// logger.log_job_complete("job-123", "sha256:abc", 5, 1024000).unwrap();
/// ```
pub struct TelemetryLogger {
    /// Path to the log file
    path: PathBuf,
    /// Buffered writer (wrapped in Arc<Mutex> for thread safety)
    writer: Arc<Mutex<BufWriter<File>>>,
}

/// Event types for telemetry logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Job planning completed
    Plan,
    /// Job started
    JobStart,
    /// File transfer started
    FileStart,
    /// Window completed successfully
    WindowOk,
    /// Window failed or required repair
    WindowFail,
    /// Job completed with digest
    JobComplete,
    /// Custom event
    Custom,
}

impl EventType {
    /// Convert event type to string
    pub fn as_str(&self) -> &str {
        match self {
            EventType::Plan => "plan",
            EventType::JobStart => "job_start",
            EventType::FileStart => "file_start",
            EventType::WindowOk => "window_ok",
            EventType::WindowFail => "window_fail",
            EventType::JobComplete => "job_complete",
            EventType::Custom => "custom",
        }
    }
}

impl FromStr for EventType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "plan" => Ok(EventType::Plan),
            "job_start" => Ok(EventType::JobStart),
            "file_start" => Ok(EventType::FileStart),
            "window_ok" => Ok(EventType::WindowOk),
            "window_fail" => Ok(EventType::WindowFail),
            "job_complete" => Ok(EventType::JobComplete),
            "custom" => Ok(EventType::Custom),
            _ => Err(Error::invalid_event_type(s)),
        }
    }
}

/// Telemetry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Timestamp (UTC)
    pub ts: DateTime<Utc>,

    /// Job ID
    pub job: String,

    /// Event type
    pub event: EventType,

    /// Optional file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Optional window ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_id: Option<u32>,

    /// Number of files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<u32>,

    /// Number of bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,

    /// Number of chunks repaired
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair: Option<u32>,

    /// Job digest (final hash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,

    /// Custom message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl TelemetryLogger {
    /// Create a new telemetry logger
    ///
    /// Opens or creates the log file in append mode.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Open file in append mode
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|_| Error::create_failed(&path))?;

        let writer = Arc::new(Mutex::new(BufWriter::new(file)));

        Ok(Self { path, writer })
    }

    /// Log a generic event
    pub fn log_event(&mut self, event: TelemetryEvent) -> Result<()> {
        let json = serde_json::to_string(&event)?;

        let mut writer = self.writer.lock().unwrap();
        writeln!(writer, "{}", json)?;
        writer.flush()?; // Always flush for durability

        Ok(())
    }

    /// Log job planning event
    pub fn log_plan(&mut self, job_id: &str, files: u32, bytes: u64) -> Result<()> {
        self.log_event(TelemetryEvent {
            ts: Utc::now(),
            job: job_id.to_string(),
            event: EventType::Plan,
            path: None,
            window_id: None,
            files: Some(files),
            bytes: Some(bytes),
            repair: None,
            digest: None,
            message: None,
        })
    }

    /// Log job start event
    pub fn log_job_start(&mut self, job_id: &str, files: u32, bytes: u64) -> Result<()> {
        self.log_event(TelemetryEvent {
            ts: Utc::now(),
            job: job_id.to_string(),
            event: EventType::JobStart,
            path: None,
            window_id: None,
            files: Some(files),
            bytes: Some(bytes),
            repair: None,
            digest: None,
            message: None,
        })
    }

    /// Log file start event
    pub fn log_file_start(&mut self, job_id: &str, path: &str, bytes: u64) -> Result<()> {
        self.log_event(TelemetryEvent {
            ts: Utc::now(),
            job: job_id.to_string(),
            event: EventType::FileStart,
            path: Some(path.to_string()),
            window_id: None,
            files: None,
            bytes: Some(bytes),
            repair: None,
            digest: None,
            message: None,
        })
    }

    /// Log successful window completion
    pub fn log_window_ok(
        &mut self,
        job_id: &str,
        path: &str,
        window_id: u32,
        bytes: u64,
        repair: u32,
    ) -> Result<()> {
        self.log_event(TelemetryEvent {
            ts: Utc::now(),
            job: job_id.to_string(),
            event: EventType::WindowOk,
            path: Some(path.to_string()),
            window_id: Some(window_id),
            files: None,
            bytes: Some(bytes),
            repair: Some(repair),
            digest: None,
            message: None,
        })
    }

    /// Log window failure
    pub fn log_window_fail(
        &mut self,
        job_id: &str,
        path: &str,
        window_id: u32,
        message: &str,
    ) -> Result<()> {
        self.log_event(TelemetryEvent {
            ts: Utc::now(),
            job: job_id.to_string(),
            event: EventType::WindowFail,
            path: Some(path.to_string()),
            window_id: Some(window_id),
            files: None,
            bytes: None,
            repair: None,
            digest: None,
            message: Some(message.to_string()),
        })
    }

    /// Log job completion with final digest
    pub fn log_job_complete(
        &mut self,
        job_id: &str,
        digest: &str,
        files: u32,
        bytes: u64,
    ) -> Result<()> {
        self.log_event(TelemetryEvent {
            ts: Utc::now(),
            job: job_id.to_string(),
            event: EventType::JobComplete,
            path: None,
            window_id: None,
            files: Some(files),
            bytes: Some(bytes),
            repair: None,
            digest: Some(digest.to_string()),
            message: None,
        })
    }

    /// Log a custom event with a message
    pub fn log_custom(&mut self, job_id: &str, message: &str) -> Result<()> {
        self.log_event(TelemetryEvent {
            ts: Utc::now(),
            job: job_id.to_string(),
            event: EventType::Custom,
            path: None,
            window_id: None,
            files: None,
            bytes: None,
            repair: None,
            digest: None,
            message: Some(message.to_string()),
        })
    }

    /// Get the path to the log file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Flush the log file
    pub fn flush(&mut self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.flush()?;
        Ok(())
    }
}

/// Parse telemetry events from a JSON Lines file
pub fn parse_telemetry_log<P: AsRef<Path>>(path: P) -> Result<Vec<TelemetryEvent>> {
    let contents = std::fs::read_to_string(path)?;
    let mut events = Vec::new();

    for (line_num, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let event: TelemetryEvent = serde_json::from_str(line)
            .map_err(|e| Error::invalid_entry(line_num + 1, &e.to_string()))?;

        events.push(event);
    }

    Ok(events)
}

// TelemetryLogger is thread-safe
unsafe impl Send for TelemetryLogger {}
unsafe impl Sync for TelemetryLogger {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::WindowOk;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"window_ok\"");

        let parsed: EventType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, EventType::WindowOk);
    }

    #[test]
    fn test_event_type_from_str() {
        assert_eq!(EventType::from_str("plan").unwrap(), EventType::Plan);
        assert_eq!(
            EventType::from_str("job_start").unwrap(),
            EventType::JobStart
        );
        assert_eq!(
            EventType::from_str("window_ok").unwrap(),
            EventType::WindowOk
        );

        let result = EventType::from_str("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_logger_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = TelemetryLogger::new(temp_file.path()).unwrap();
        assert_eq!(logger.path(), temp_file.path());
    }

    #[test]
    fn test_log_job_start() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut logger = TelemetryLogger::new(temp_file.path()).unwrap();

        logger.log_job_start("job-123", 5, 1024000).unwrap();
        logger.flush().unwrap();

        let events = parse_telemetry_log(temp_file.path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].job, "job-123");
        assert_eq!(events[0].event, EventType::JobStart);
        assert_eq!(events[0].files, Some(5));
        assert_eq!(events[0].bytes, Some(1024000));
    }

    #[test]
    fn test_log_window_ok() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut logger = TelemetryLogger::new(temp_file.path()).unwrap();

        logger
            .log_window_ok("job-123", "file.bin", 0, 16777216, 2)
            .unwrap();
        logger.flush().unwrap();

        let events = parse_telemetry_log(temp_file.path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, EventType::WindowOk);
        assert_eq!(events[0].path, Some("file.bin".to_string()));
        assert_eq!(events[0].window_id, Some(0));
        assert_eq!(events[0].bytes, Some(16777216));
        assert_eq!(events[0].repair, Some(2));
    }

    #[test]
    fn test_log_job_complete() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut logger = TelemetryLogger::new(temp_file.path()).unwrap();

        logger
            .log_job_complete("job-123", "sha256:abc123", 5, 1024000)
            .unwrap();
        logger.flush().unwrap();

        let events = parse_telemetry_log(temp_file.path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, EventType::JobComplete);
        assert_eq!(events[0].digest, Some("sha256:abc123".to_string()));
        assert_eq!(events[0].files, Some(5));
    }

    #[test]
    fn test_multiple_events() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut logger = TelemetryLogger::new(temp_file.path()).unwrap();

        logger.log_job_start("job-123", 2, 8192).unwrap();
        logger.log_file_start("job-123", "file1.bin", 4096).unwrap();
        logger
            .log_window_ok("job-123", "file1.bin", 0, 4096, 0)
            .unwrap();
        logger
            .log_job_complete("job-123", "sha256:final", 2, 8192)
            .unwrap();
        logger.flush().unwrap();

        let events = parse_telemetry_log(temp_file.path()).unwrap();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].event, EventType::JobStart);
        assert_eq!(events[1].event, EventType::FileStart);
        assert_eq!(events[2].event, EventType::WindowOk);
        assert_eq!(events[3].event, EventType::JobComplete);
    }

    #[test]
    fn test_parse_empty_lines() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), "{\"ts\":\"2025-10-18T12:00:00Z\",\"job\":\"job-1\",\"event\":\"job_start\",\"files\":1,\"bytes\":100}\n\n\n").unwrap();

        let events = parse_telemetry_log(temp_file.path()).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_custom_event() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut logger = TelemetryLogger::new(temp_file.path()).unwrap();

        logger
            .log_custom("job-123", "Custom checkpoint reached")
            .unwrap();
        logger.flush().unwrap();

        let events = parse_telemetry_log(temp_file.path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, EventType::Custom);
        assert_eq!(
            events[0].message,
            Some("Custom checkpoint reached".to_string())
        );
    }
}
