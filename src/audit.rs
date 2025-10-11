/*!
 * Audit logging for Orbit operations
 */

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::AuditFormat;
use crate::core::CopyStats;
use crate::error::{OrbitError, Result};

/// Audit log entry
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
                entry.compression_ratio.map(|r| r.to_string()).unwrap_or_default(),
                entry.status,
                entry.attempts,
                entry.error.as_deref().unwrap_or("")
            ).map_err(|e| OrbitError::AuditLog(format!("Failed to write CSV entry: {}", e)))?;
        }
    }
    
    log_file.flush()
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
            let entries: Vec<AuditEntry> = content
                .lines()
                .skip(1)
                .filter_map(|line| parse_csv_line(line))
                .collect();
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
        checksum: if parts[5].is_empty() { None } else { Some(parts[5].to_string()) },
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
        ).unwrap();
        
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
        ).unwrap();
        
        let content = std::fs::read_to_string(temp.path()).unwrap();
        assert!(content.contains("timestamp,source,destination"));
        assert!(content.contains("2048"));
    }
}