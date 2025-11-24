/*!
 * Audit logging for Orbit operations
 *
 * This module provides structured audit logging for copy operations,
 * emitting JSON Lines or CSV records for compliance, observability,
 * and forensic analysis.
 *
 * # Example
 *
 * ```no_run
 * use orbit::audit::AuditLogger;
 * use orbit::config::AuditFormat;
 * use std::path::Path;
 *
 * let mut logger = AuditLogger::new(
 *     Some(Path::new("audit.log")),
 *     AuditFormat::Json,
 * ).unwrap();
 *
 * logger.emit_start("job-123", Path::new("/src"), Path::new("/dest"), "local", 1024).unwrap();
 * // ... perform copy ...
 * logger.emit_complete("job-123", Path::new("/src"), Path::new("/dest"), "local", 1024, 150, true).unwrap();
 * ```
 */

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::{AuditFormat, CompressionType};
use crate::core::CopyStats;
use crate::error::{OrbitError, Result};

/// Audit event matching the README specification
///
/// This structure captures all relevant details for audit compliance,
/// including source/destination, protocol, compression, checksums, and timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// ISO 8601 timestamp with timezone
    pub timestamp: String,

    /// Job identifier for correlating related events
    pub job: String,

    /// Source path or URI
    pub source: String,

    /// Destination path or URI
    pub destination: String,

    /// Protocol used (local, s3, smb, ssh)
    pub protocol: String,

    /// Number of bytes transferred
    pub bytes_transferred: u64,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Compression algorithm used (none, lz4, zstd)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,

    /// Compression ratio (original/compressed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_ratio: Option<f64>,

    /// Hash algorithm used for checksum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_algorithm: Option<String>,

    /// Whether checksum verification passed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_match: Option<bool>,

    /// Storage class for cloud destinations (S3, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_class: Option<String>,

    /// Number of multipart upload parts (for S3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multipart_parts: Option<u32>,

    /// Event status: started, progress, success, failure
    pub status: String,

    /// Number of retry attempts
    pub retries: u32,

    /// Starmap node ID for distributed correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starmap_node: Option<String>,

    /// Error message if status is failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Number of files (for directory operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_count: Option<u64>,
}

impl AuditEvent {
    /// Create a new audit event with required fields
    pub fn new(job: &str, source: &Path, destination: &Path, protocol: &str, status: &str) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            job: job.to_string(),
            source: source.to_string_lossy().to_string(),
            destination: destination.to_string_lossy().to_string(),
            protocol: protocol.to_string(),
            bytes_transferred: 0,
            duration_ms: 0,
            compression: None,
            compression_ratio: None,
            checksum_algorithm: None,
            checksum_match: None,
            storage_class: None,
            multipart_parts: None,
            status: status.to_string(),
            retries: 0,
            starmap_node: None,
            error: None,
            files_count: None,
        }
    }

    /// Set bytes transferred
    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes_transferred = bytes;
        self
    }

    /// Set duration in milliseconds
    pub fn with_duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Set compression info
    pub fn with_compression(mut self, algo: &str, ratio: Option<f64>) -> Self {
        self.compression = Some(algo.to_string());
        self.compression_ratio = ratio;
        self
    }

    /// Set checksum info
    pub fn with_checksum(mut self, algorithm: &str, matched: bool) -> Self {
        self.checksum_algorithm = Some(algorithm.to_string());
        self.checksum_match = Some(matched);
        self
    }

    /// Set retry count
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Set error message
    pub fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    /// Set storage class
    pub fn with_storage_class(mut self, class: &str) -> Self {
        self.storage_class = Some(class.to_string());
        self
    }

    /// Set multipart parts count
    pub fn with_multipart_parts(mut self, parts: u32) -> Self {
        self.multipart_parts = Some(parts);
        self
    }

    /// Set starmap node for correlation
    pub fn with_starmap_node(mut self, node: &str) -> Self {
        self.starmap_node = Some(node.to_string());
        self
    }

    /// Set files count for directory operations
    pub fn with_files_count(mut self, count: u64) -> Self {
        self.files_count = Some(count);
        self
    }
}

/// Audit logger for emitting structured audit events
///
/// Thread-safe logger that writes audit events to a file in JSON Lines or CSV format.
/// The logger handles file operations and ensures crash-resistant append-only writes.
pub struct AuditLogger {
    /// Path to the audit log file
    path: PathBuf,
    /// Output format (JSON or CSV)
    format: AuditFormat,
    /// Buffered writer (wrapped in Arc<Mutex> for thread safety)
    writer: Arc<Mutex<BufWriter<File>>>,
    /// Whether CSV header has been written
    csv_header_written: Arc<Mutex<bool>>,
}

impl AuditLogger {
    /// Create a new audit logger
    ///
    /// Opens or creates the log file in append mode. If `path` is None,
    /// defaults to "orbit_audit.log" in the current directory.
    ///
    /// # Errors
    ///
    /// Returns `OrbitError::AuditLog` if the file cannot be opened or created.
    pub fn new(path: Option<&Path>, format: AuditFormat) -> Result<Self> {
        let default_path = PathBuf::from("orbit_audit.log");
        let log_path = path.map(|p| p.to_path_buf()).unwrap_or(default_path);

        // Create parent directory if needed
        if let Some(parent) = log_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    OrbitError::AuditLog(format!("Failed to create audit log directory: {}", e))
                })?;
            }
        }

        // Check if file exists and has content (for CSV header logic)
        let file_exists_with_content = log_path.exists()
            && std::fs::metadata(&log_path)
                .map(|m| m.len() > 0)
                .unwrap_or(false);

        // Open file in append mode
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| OrbitError::AuditLog(format!("Failed to open audit log: {}", e)))?;

        let writer = Arc::new(Mutex::new(BufWriter::new(file)));

        Ok(Self {
            path: log_path,
            format,
            writer,
            csv_header_written: Arc::new(Mutex::new(file_exists_with_content)),
        })
    }

    /// Get the path to the audit log file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the audit format
    pub fn format(&self) -> AuditFormat {
        self.format
    }

    /// Emit a generic audit event
    pub fn emit(&mut self, event: &AuditEvent) -> Result<()> {
        match self.format {
            AuditFormat::Json => self.emit_json(event),
            AuditFormat::Csv => self.emit_csv(event),
        }
    }

    /// Emit event as JSON Lines
    fn emit_json(&mut self, event: &AuditEvent) -> Result<()> {
        let json = serde_json::to_string(event)
            .map_err(|e| OrbitError::AuditLog(format!("Failed to serialize event: {}", e)))?;

        let mut writer = self.writer.lock().unwrap();
        writeln!(writer, "{}", json)
            .map_err(|e| OrbitError::AuditLog(format!("Failed to write audit log: {}", e)))?;
        writer
            .flush()
            .map_err(|e| OrbitError::AuditLog(format!("Failed to flush audit log: {}", e)))?;

        Ok(())
    }

    /// Emit event as CSV
    fn emit_csv(&mut self, event: &AuditEvent) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        let mut header_written = self.csv_header_written.lock().unwrap();

        // Write header if this is the first entry
        if !*header_written {
            writeln!(
                writer,
                "timestamp,job,source,destination,protocol,bytes_transferred,duration_ms,compression,compression_ratio,checksum_algorithm,checksum_match,storage_class,multipart_parts,status,retries,starmap_node,error,files_count"
            )
            .map_err(|e| OrbitError::AuditLog(format!("Failed to write CSV header: {}", e)))?;
            *header_written = true;
        }

        // Escape CSV fields that might contain commas or quotes
        let escape_csv = |s: &str| {
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };

        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            escape_csv(&event.timestamp),
            escape_csv(&event.job),
            escape_csv(&event.source),
            escape_csv(&event.destination),
            escape_csv(&event.protocol),
            event.bytes_transferred,
            event.duration_ms,
            event.compression.as_deref().unwrap_or(""),
            event
                .compression_ratio
                .map(|r| r.to_string())
                .unwrap_or_default(),
            event.checksum_algorithm.as_deref().unwrap_or(""),
            event
                .checksum_match
                .map(|m| m.to_string())
                .unwrap_or_default(),
            event.storage_class.as_deref().unwrap_or(""),
            event
                .multipart_parts
                .map(|p| p.to_string())
                .unwrap_or_default(),
            escape_csv(&event.status),
            event.retries,
            event.starmap_node.as_deref().unwrap_or(""),
            event.error.as_deref().map(escape_csv).unwrap_or_default(),
            event.files_count.map(|c| c.to_string()).unwrap_or_default(),
        )
        .map_err(|e| OrbitError::AuditLog(format!("Failed to write CSV entry: {}", e)))?;

        writer
            .flush()
            .map_err(|e| OrbitError::AuditLog(format!("Failed to flush audit log: {}", e)))?;

        Ok(())
    }

    /// Emit a "started" event for a copy operation
    pub fn emit_start(
        &mut self,
        job: &str,
        source: &Path,
        destination: &Path,
        protocol: &str,
        expected_bytes: u64,
    ) -> Result<()> {
        let event = AuditEvent::new(job, source, destination, protocol, "started")
            .with_bytes(expected_bytes);
        self.emit(&event)
    }

    /// Emit a "progress" event during a copy operation
    pub fn emit_progress(
        &mut self,
        job: &str,
        source: &Path,
        destination: &Path,
        protocol: &str,
        bytes_so_far: u64,
        duration_ms: u64,
    ) -> Result<()> {
        let event = AuditEvent::new(job, source, destination, protocol, "progress")
            .with_bytes(bytes_so_far)
            .with_duration_ms(duration_ms);
        self.emit(&event)
    }

    /// Emit a "success" event for a completed copy operation
    #[allow(clippy::too_many_arguments)]
    pub fn emit_complete(
        &mut self,
        job: &str,
        source: &Path,
        destination: &Path,
        protocol: &str,
        bytes: u64,
        duration_ms: u64,
        checksum_match: bool,
    ) -> Result<()> {
        let event = AuditEvent::new(job, source, destination, protocol, "success")
            .with_bytes(bytes)
            .with_duration_ms(duration_ms)
            .with_checksum("blake3", checksum_match);
        self.emit(&event)
    }

    /// Emit a "failure" event for a failed copy operation
    #[allow(clippy::too_many_arguments)]
    pub fn emit_failure(
        &mut self,
        job: &str,
        source: &Path,
        destination: &Path,
        protocol: &str,
        bytes: u64,
        duration_ms: u64,
        retries: u32,
        error: &str,
    ) -> Result<()> {
        let event = AuditEvent::new(job, source, destination, protocol, "failure")
            .with_bytes(bytes)
            .with_duration_ms(duration_ms)
            .with_retries(retries)
            .with_error(error);
        self.emit(&event)
    }

    /// Emit a complete event from CopyStats
    #[allow(clippy::too_many_arguments)]
    pub fn emit_from_stats(
        &mut self,
        job: &str,
        source: &Path,
        destination: &Path,
        protocol: &str,
        stats: &CopyStats,
        compression: CompressionType,
        retries: u32,
        error: Option<&str>,
    ) -> Result<()> {
        let status = if error.is_some() {
            "failure"
        } else {
            "success"
        };

        let mut event = AuditEvent::new(job, source, destination, protocol, status)
            .with_bytes(stats.bytes_copied)
            .with_duration_ms(stats.duration.as_millis() as u64)
            .with_retries(retries);

        // Add compression info
        let compression_str = match compression {
            CompressionType::None => "none",
            CompressionType::Lz4 => "lz4",
            CompressionType::Zstd { .. } => "zstd",
        };
        event = event.with_compression(compression_str, stats.compression_ratio);

        // Add checksum info if available
        if let Some(ref checksum) = stats.checksum {
            event = event.with_checksum("blake3", !checksum.is_empty());
        }

        // Add error if present
        if let Some(err) = error {
            event = event.with_error(err);
        }

        // Add file count for directory operations
        if stats.files_copied > 0 || stats.files_skipped > 0 || stats.files_failed > 0 {
            event = event.with_files_count(stats.files_copied + stats.files_skipped);
        }

        self.emit(&event)
    }

    /// Flush the log file
    pub fn flush(&mut self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer
            .flush()
            .map_err(|e| OrbitError::AuditLog(format!("Failed to flush audit log: {}", e)))
    }
}

// Thread safety markers - BufWriter<File> is Send but not Sync
// We wrap it in Mutex for interior mutability
unsafe impl Send for AuditLogger {}
unsafe impl Sync for AuditLogger {}

/// Legacy audit log entry (kept for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub bytes_copied: u64,
    pub duration_ms: u64,
    pub checksum: Option<String>,
    pub compression_ratio: Option<f64>,
    pub status: String,
    pub attempts: u32,
    pub error: Option<String>,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(
        source: &Path,
        destination: &Path,
        stats: &CopyStats,
        status: &str,
        attempts: u32,
        error: Option<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            bytes_copied: stats.bytes_copied,
            duration_ms: stats.duration.as_millis() as u64,
            checksum: stats.checksum.clone(),
            compression_ratio: stats.compression_ratio,
            status: status.to_string(),
            attempts,
            error,
        }
    }
}

/// Write an audit log entry
#[allow(clippy::too_many_arguments)]
pub fn write_audit_log(
    source: &Path,
    destination: &Path,
    stats: &CopyStats,
    status: &str,
    attempts: u32,
    error: Option<String>,
    format: AuditFormat,
    log_path: Option<&Path>,
) -> Result<()> {
    let entry = AuditEntry::new(source, destination, stats, status, attempts, error);

    let default_path = PathBuf::from("orbit_audit.log");
    let audit_path = log_path.unwrap_or(&default_path);

    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_path)
        .map_err(|e| OrbitError::AuditLog(format!("Failed to open audit log: {}", e)))?;

    match format {
        AuditFormat::Json => {
            let json = serde_json::to_string(&entry)
                .map_err(|e| OrbitError::AuditLog(format!("Failed to serialize JSON: {}", e)))?;
            writeln!(log_file, "{}", json)
                .map_err(|e| OrbitError::AuditLog(format!("Failed to write audit log: {}", e)))?;
        }
        AuditFormat::Csv => {
            // Write CSV header if file is new/empty
            if std::fs::metadata(audit_path)
                .map(|m| m.len() == 0)
                .unwrap_or(true)
            {
                writeln!(
                    log_file,
                    "timestamp,source,destination,bytes_copied,duration_ms,checksum,compression_ratio,status,attempts,error"
                ).map_err(|e| OrbitError::AuditLog(format!("Failed to write CSV header: {}", e)))?;
            }

            writeln!(
                log_file,
                "{},{:?},{:?},{},{},{},{},{},{},{}",
                entry.timestamp,
                entry.source,
                entry.destination,
                entry.bytes_copied,
                entry.duration_ms,
                entry.checksum.as_deref().unwrap_or(""),
                entry
                    .compression_ratio
                    .map(|r| r.to_string())
                    .unwrap_or_default(),
                entry.status,
                entry.attempts,
                entry.error.as_deref().unwrap_or("")
            )
            .map_err(|e| OrbitError::AuditLog(format!("Failed to write CSV entry: {}", e)))?;
        }
    }

    log_file
        .flush()
        .map_err(|e| OrbitError::AuditLog(format!("Failed to flush audit log: {}", e)))?;

    Ok(())
}

/// Read audit log entries
pub fn read_audit_log(log_path: &Path, format: AuditFormat) -> Result<Vec<AuditEntry>> {
    if !log_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(log_path)
        .map_err(|e| OrbitError::AuditLog(format!("Failed to read audit log: {}", e)))?;

    match format {
        AuditFormat::Json => {
            let entries: Vec<AuditEntry> = content
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect();
            Ok(entries)
        }
        AuditFormat::Csv => {
            // Skip header line for CSV
            let entries: Vec<AuditEntry> =
                content.lines().skip(1).filter_map(parse_csv_line).collect();
            Ok(entries)
        }
    }
}

/// Parse a single CSV line into an AuditEntry (simplified)
fn parse_csv_line(line: &str) -> Option<AuditEntry> {
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() < 9 {
        return None;
    }

    Some(AuditEntry {
        timestamp: parts[0].to_string(),
        source: PathBuf::from(parts[1].trim_matches('"')),
        destination: PathBuf::from(parts[2].trim_matches('"')),
        bytes_copied: parts[3].parse().ok()?,
        duration_ms: parts[4].parse().ok()?,
        checksum: if parts[5].is_empty() {
            None
        } else {
            Some(parts[5].to_string())
        },
        compression_ratio: parts[6].parse().ok(),
        status: parts[7].to_string(),
        attempts: parts[8].parse().ok()?,
        error: if parts.len() > 9 && !parts[9].is_empty() {
            Some(parts[9].to_string())
        } else {
            None
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_and_read_json_audit() {
        let temp = NamedTempFile::new().unwrap();

        let stats = CopyStats {
            bytes_copied: 1024,
            duration: Duration::from_secs(5),
            checksum: Some("abc123".to_string()),
            compression_ratio: Some(50.0),
            files_copied: 1,
            files_skipped: 0,
            files_failed: 0,
            delta_stats: None,
            chunks_resumed: 0,
            bytes_skipped: 0,
        };

        let source = Path::new("/tmp/source.txt");
        let dest = Path::new("/tmp/dest.txt");

        write_audit_log(
            source,
            dest,
            &stats,
            "success",
            1,
            None,
            AuditFormat::Json,
            Some(temp.path()),
        )
        .unwrap();

        let entries = read_audit_log(temp.path(), AuditFormat::Json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].bytes_copied, 1024);
        assert_eq!(entries[0].status, "success");
    }

    #[test]
    fn test_write_csv_audit() {
        let temp = NamedTempFile::new().unwrap();

        let stats = CopyStats {
            bytes_copied: 2048,
            duration: Duration::from_millis(500),
            checksum: None,
            compression_ratio: None,
            files_copied: 1,
            files_skipped: 0,
            files_failed: 0,
            delta_stats: None,
            chunks_resumed: 0,
            bytes_skipped: 0,
        };

        let source = Path::new("/tmp/source.txt");
        let dest = Path::new("/tmp/dest.txt");

        write_audit_log(
            source,
            dest,
            &stats,
            "success",
            1,
            None,
            AuditFormat::Csv,
            Some(temp.path()),
        )
        .unwrap();

        let content = std::fs::read_to_string(temp.path()).unwrap();
        assert!(content.contains("timestamp,source,destination"));
        assert!(content.contains("2048"));
    }

    // Tests for new AuditLogger

    #[test]
    fn test_audit_logger_json_start_event() {
        let temp = NamedTempFile::new().unwrap();
        let mut logger = AuditLogger::new(Some(temp.path()), AuditFormat::Json).unwrap();

        logger
            .emit_start(
                "job-123",
                Path::new("/src/file.txt"),
                Path::new("/dst/file.txt"),
                "local",
                1024,
            )
            .unwrap();

        let content = std::fs::read_to_string(temp.path()).unwrap();
        assert!(content.contains("\"job\":\"job-123\""));
        assert!(content.contains("\"status\":\"started\""));
        assert!(content.contains("\"protocol\":\"local\""));
        assert!(content.contains("\"bytes_transferred\":1024"));
    }

    #[test]
    fn test_audit_logger_json_complete_event() {
        let temp = NamedTempFile::new().unwrap();
        let mut logger = AuditLogger::new(Some(temp.path()), AuditFormat::Json).unwrap();

        logger
            .emit_complete(
                "job-456",
                Path::new("/src/file.txt"),
                Path::new("/dst/file.txt"),
                "local",
                2048,
                150,
                true,
            )
            .unwrap();

        let content = std::fs::read_to_string(temp.path()).unwrap();
        assert!(content.contains("\"status\":\"success\""));
        assert!(content.contains("\"bytes_transferred\":2048"));
        assert!(content.contains("\"duration_ms\":150"));
        assert!(content.contains("\"checksum_match\":true"));
    }

    #[test]
    fn test_audit_logger_json_failure_event() {
        let temp = NamedTempFile::new().unwrap();
        let mut logger = AuditLogger::new(Some(temp.path()), AuditFormat::Json).unwrap();

        logger
            .emit_failure(
                "job-789",
                Path::new("/src/file.txt"),
                Path::new("/dst/file.txt"),
                "s3",
                512,
                50,
                3,
                "Connection timeout",
            )
            .unwrap();

        let content = std::fs::read_to_string(temp.path()).unwrap();
        assert!(content.contains("\"status\":\"failure\""));
        assert!(content.contains("\"retries\":3"));
        assert!(content.contains("\"error\":\"Connection timeout\""));
        assert!(content.contains("\"protocol\":\"s3\""));
    }

    #[test]
    fn test_audit_logger_csv_format() {
        let temp = NamedTempFile::new().unwrap();
        let mut logger = AuditLogger::new(Some(temp.path()), AuditFormat::Csv).unwrap();

        logger
            .emit_start(
                "job-csv",
                Path::new("/src/file.txt"),
                Path::new("/dst/file.txt"),
                "local",
                4096,
            )
            .unwrap();

        let content = std::fs::read_to_string(temp.path()).unwrap();
        // Check for CSV header
        assert!(content.contains("timestamp,job,source,destination,protocol"));
        // Check for data row
        assert!(content.contains("job-csv"));
        assert!(content.contains("4096"));
        assert!(content.contains("started"));
    }

    #[test]
    fn test_audit_event_builder_pattern() {
        let event = AuditEvent::new(
            "job-builder",
            Path::new("/src"),
            Path::new("/dst"),
            "smb",
            "success",
        )
        .with_bytes(8192)
        .with_duration_ms(200)
        .with_compression("zstd", Some(0.75))
        .with_checksum("blake3", true)
        .with_retries(2)
        .with_storage_class("STANDARD")
        .with_multipart_parts(5)
        .with_starmap_node("node-001")
        .with_files_count(10);

        assert_eq!(event.job, "job-builder");
        assert_eq!(event.bytes_transferred, 8192);
        assert_eq!(event.duration_ms, 200);
        assert_eq!(event.compression, Some("zstd".to_string()));
        assert_eq!(event.compression_ratio, Some(0.75));
        assert_eq!(event.checksum_algorithm, Some("blake3".to_string()));
        assert_eq!(event.checksum_match, Some(true));
        assert_eq!(event.retries, 2);
        assert_eq!(event.storage_class, Some("STANDARD".to_string()));
        assert_eq!(event.multipart_parts, Some(5));
        assert_eq!(event.starmap_node, Some("node-001".to_string()));
        assert_eq!(event.files_count, Some(10));
    }

    #[test]
    fn test_audit_logger_multiple_events() {
        let temp = NamedTempFile::new().unwrap();
        let mut logger = AuditLogger::new(Some(temp.path()), AuditFormat::Json).unwrap();

        // Emit start
        logger
            .emit_start(
                "job-multi",
                Path::new("/src"),
                Path::new("/dst"),
                "local",
                1000,
            )
            .unwrap();

        // Emit progress
        logger
            .emit_progress(
                "job-multi",
                Path::new("/src"),
                Path::new("/dst"),
                "local",
                500,
                50,
            )
            .unwrap();

        // Emit complete
        logger
            .emit_complete(
                "job-multi",
                Path::new("/src"),
                Path::new("/dst"),
                "local",
                1000,
                100,
                true,
            )
            .unwrap();

        let content = std::fs::read_to_string(temp.path()).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);

        // Verify each line is valid JSON
        for line in &lines {
            let event: AuditEvent = serde_json::from_str(line).unwrap();
            assert_eq!(event.job, "job-multi");
        }
    }

    #[test]
    fn test_audit_logger_emit_from_stats() {
        let temp = NamedTempFile::new().unwrap();
        let mut logger = AuditLogger::new(Some(temp.path()), AuditFormat::Json).unwrap();

        let stats = CopyStats {
            bytes_copied: 5000,
            duration: Duration::from_millis(250),
            checksum: Some("abc123".to_string()),
            compression_ratio: Some(0.6),
            files_copied: 5,
            files_skipped: 2,
            files_failed: 0,
            delta_stats: None,
            chunks_resumed: 0,
            bytes_skipped: 0,
        };

        logger
            .emit_from_stats(
                "job-stats",
                Path::new("/src"),
                Path::new("/dst"),
                "local",
                &stats,
                CompressionType::Zstd { level: 3 },
                1,
                None,
            )
            .unwrap();

        let content = std::fs::read_to_string(temp.path()).unwrap();
        let event: AuditEvent = serde_json::from_str(&content).unwrap();

        assert_eq!(event.job, "job-stats");
        assert_eq!(event.status, "success");
        assert_eq!(event.bytes_transferred, 5000);
        assert_eq!(event.duration_ms, 250);
        assert_eq!(event.compression, Some("zstd".to_string()));
        assert_eq!(event.files_count, Some(7)); // 5 copied + 2 skipped
    }

    #[test]
    fn test_audit_logger_default_path() {
        // Create logger without path - uses default
        let logger = AuditLogger::new(None, AuditFormat::Json).unwrap();
        assert_eq!(logger.path(), Path::new("orbit_audit.log"));

        // Clean up default file if created
        let _ = std::fs::remove_file("orbit_audit.log");
    }

    #[test]
    fn test_audit_event_serialization() {
        let event = AuditEvent::new(
            "job-serial",
            Path::new("/src/file.txt"),
            Path::new("/dst/file.txt"),
            "local",
            "success",
        )
        .with_bytes(1024)
        .with_duration_ms(100);

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AuditEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.job, "job-serial");
        assert_eq!(parsed.bytes_transferred, 1024);
        assert_eq!(parsed.status, "success");
    }
}
