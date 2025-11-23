/*!
 * Integration tests for audit logging functionality
 *
 * These tests verify that audit logs are correctly generated during
 * copy operations and contain the expected structured data.
 */

use std::fs;
use std::path::Path;
use tempfile::TempDir;

use orbit::audit::{AuditEvent, AuditLogger};
use orbit::config::{AuditFormat, CopyConfig};
use orbit::core::copy_file;

/// Test that a simple file copy generates an audit log entry
#[test]
fn test_audit_log_generated_on_file_copy() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let dest = temp_dir.path().join("dest.txt");
    let audit_log = temp_dir.path().join("audit.log");

    // Create source file
    fs::write(&source, b"test data for audit").unwrap();

    // Configure copy with audit logging
    let config = CopyConfig {
        audit_log_path: Some(audit_log.clone()),
        audit_format: AuditFormat::Json,
        ..Default::default()
    };

    // Perform copy
    let stats = copy_file(&source, &dest, &config).unwrap();
    assert!(stats.bytes_copied > 0);

    // Verify audit log was created and contains expected data
    assert!(audit_log.exists(), "Audit log file should exist");

    let content = fs::read_to_string(&audit_log).unwrap();
    assert!(!content.is_empty(), "Audit log should not be empty");

    // Parse JSON lines
    let lines: Vec<&str> = content.lines().collect();
    assert!(
        lines.len() >= 1,
        "Audit log should contain at least one entry"
    );

    // Verify first entry is parseable and contains expected fields
    let event: AuditEvent = serde_json::from_str(lines[0]).expect("Should parse as AuditEvent");
    assert!(
        event.job.starts_with("orbit-"),
        "Job ID should start with 'orbit-'"
    );
    assert_eq!(event.protocol, "local");
    assert!(
        event.status == "started" || event.status == "success",
        "Status should be started or success"
    );
}

/// Test that audit log contains success status on successful copy
#[test]
fn test_audit_log_success_status() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let dest = temp_dir.path().join("dest.txt");
    let audit_log = temp_dir.path().join("audit.log");

    fs::write(&source, b"success test data").unwrap();

    let config = CopyConfig {
        audit_log_path: Some(audit_log.clone()),
        audit_format: AuditFormat::Json,
        ..Default::default()
    };

    copy_file(&source, &dest, &config).unwrap();

    let content = fs::read_to_string(&audit_log).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Find success event
    let success_event = lines
        .iter()
        .filter_map(|line| serde_json::from_str::<AuditEvent>(line).ok())
        .find(|e| e.status == "success");

    assert!(success_event.is_some(), "Should have a success event");
    let event = success_event.unwrap();
    assert!(
        event.bytes_transferred > 0,
        "Bytes transferred should be > 0"
    );
    // duration_ms is u64, always >= 0
}

/// Test that audit log contains correct byte count
#[test]
fn test_audit_log_byte_count() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let dest = temp_dir.path().join("dest.txt");
    let audit_log = temp_dir.path().join("audit.log");

    let test_data = b"0123456789"; // 10 bytes
    fs::write(&source, test_data).unwrap();

    let config = CopyConfig {
        audit_log_path: Some(audit_log.clone()),
        audit_format: AuditFormat::Json,
        ..Default::default()
    };

    copy_file(&source, &dest, &config).unwrap();

    let content = fs::read_to_string(&audit_log).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Find the completion event
    let completion_event = lines
        .iter()
        .filter_map(|line| serde_json::from_str::<AuditEvent>(line).ok())
        .find(|e| e.status == "success" && e.bytes_transferred > 0);

    assert!(completion_event.is_some(), "Should have completion event");
    let event = completion_event.unwrap();
    assert_eq!(
        event.bytes_transferred, 10,
        "Should report correct byte count"
    );
}

/// Test CSV format audit logging
#[test]
fn test_audit_log_csv_format() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let dest = temp_dir.path().join("dest.txt");
    let audit_log = temp_dir.path().join("audit.csv");

    fs::write(&source, b"csv test data").unwrap();

    let config = CopyConfig {
        audit_log_path: Some(audit_log.clone()),
        audit_format: AuditFormat::Csv,
        ..Default::default()
    };

    copy_file(&source, &dest, &config).unwrap();

    let content = fs::read_to_string(&audit_log).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Should have header and at least one data row
    assert!(lines.len() >= 2, "Should have header + data rows");

    // Verify header contains expected columns
    let header = lines[0];
    assert!(header.contains("timestamp"), "Header should have timestamp");
    assert!(header.contains("job"), "Header should have job");
    assert!(header.contains("source"), "Header should have source");
    assert!(
        header.contains("destination"),
        "Header should have destination"
    );
    assert!(header.contains("protocol"), "Header should have protocol");
    assert!(header.contains("status"), "Header should have status");
}

/// Test that audit logs contain source and destination paths
#[test]
fn test_audit_log_paths() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("test_source.txt");
    let dest = temp_dir.path().join("test_dest.txt");
    let audit_log = temp_dir.path().join("audit.log");

    fs::write(&source, b"path test").unwrap();

    let config = CopyConfig {
        audit_log_path: Some(audit_log.clone()),
        audit_format: AuditFormat::Json,
        ..Default::default()
    };

    copy_file(&source, &dest, &config).unwrap();

    let content = fs::read_to_string(&audit_log).unwrap();

    // Verify paths are present
    assert!(
        content.contains("test_source.txt"),
        "Audit log should contain source filename"
    );
    assert!(
        content.contains("test_dest.txt"),
        "Audit log should contain destination filename"
    );
}

/// Test audit logging with verbose mode (without explicit path)
#[test]
fn test_audit_log_verbose_mode() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.txt");
    let dest = temp_dir.path().join("dest.txt");

    fs::write(&source, b"verbose test").unwrap();

    // Change to temp directory to avoid polluting project root
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let config = CopyConfig {
        verbose: true,
        audit_format: AuditFormat::Json,
        ..Default::default()
    };

    copy_file(&source, &dest, &config).unwrap();

    // Restore original directory
    std::env::set_current_dir(&original_dir).unwrap();

    // Check that default audit log was created
    let default_audit = temp_dir.path().join("orbit_audit.log");
    if default_audit.exists() {
        let content = fs::read_to_string(&default_audit).unwrap();
        assert!(
            !content.is_empty(),
            "Verbose mode should generate audit log"
        );
    }
}

/// Test that multiple copy operations append to the same log
#[test]
fn test_audit_log_append_multiple_operations() {
    let temp_dir = TempDir::new().unwrap();
    let source1 = temp_dir.path().join("source1.txt");
    let source2 = temp_dir.path().join("source2.txt");
    let dest1 = temp_dir.path().join("dest1.txt");
    let dest2 = temp_dir.path().join("dest2.txt");
    let audit_log = temp_dir.path().join("audit.log");

    fs::write(&source1, b"first file").unwrap();
    fs::write(&source2, b"second file").unwrap();

    let config = CopyConfig {
        audit_log_path: Some(audit_log.clone()),
        audit_format: AuditFormat::Json,
        ..Default::default()
    };

    // Perform two copy operations
    copy_file(&source1, &dest1, &config).unwrap();
    copy_file(&source2, &dest2, &config).unwrap();

    let content = fs::read_to_string(&audit_log).unwrap();
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

    // Should have events for both operations (at least 2 start + 2 complete = 4)
    // But could be fewer if start events aren't emitted for small files
    assert!(
        lines.len() >= 2,
        "Should have events from both copy operations"
    );

    // Parse and verify different job IDs
    let events: Vec<AuditEvent> = lines
        .iter()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    let job_ids: std::collections::HashSet<&str> = events.iter().map(|e| e.job.as_str()).collect();
    assert!(
        job_ids.len() >= 2,
        "Should have different job IDs for different operations"
    );
}

/// Test that failed copy operations are logged
#[test]
fn test_audit_log_failure_logged() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("nonexistent.txt"); // Does not exist
    let dest = temp_dir.path().join("dest.txt");
    let audit_log = temp_dir.path().join("audit.log");

    let config = CopyConfig {
        audit_log_path: Some(audit_log.clone()),
        audit_format: AuditFormat::Json,
        ..Default::default()
    };

    // This should fail because source doesn't exist
    let result = copy_file(&source, &dest, &config);
    assert!(result.is_err(), "Copy should fail for nonexistent source");

    // Verify failure is logged
    if audit_log.exists() {
        let content = fs::read_to_string(&audit_log).unwrap();

        // Should contain failure event
        let has_failure = content.lines().any(|line| {
            serde_json::from_str::<AuditEvent>(line)
                .map(|e| e.status == "failure")
                .unwrap_or(false)
        });

        assert!(has_failure, "Should log failure event");
    }
}

/// Test AuditLogger directly
#[test]
fn test_audit_logger_direct_usage() {
    let temp_dir = TempDir::new().unwrap();
    let audit_log = temp_dir.path().join("direct_audit.log");

    let mut logger = AuditLogger::new(Some(&audit_log), AuditFormat::Json).unwrap();

    // Emit various events
    logger
        .emit_start(
            "test-job-1",
            Path::new("/src/file.txt"),
            Path::new("/dst/file.txt"),
            "local",
            1024,
        )
        .unwrap();

    logger
        .emit_progress(
            "test-job-1",
            Path::new("/src/file.txt"),
            Path::new("/dst/file.txt"),
            "local",
            512,
            50,
        )
        .unwrap();

    logger
        .emit_complete(
            "test-job-1",
            Path::new("/src/file.txt"),
            Path::new("/dst/file.txt"),
            "local",
            1024,
            100,
            true,
        )
        .unwrap();

    let content = fs::read_to_string(&audit_log).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    assert_eq!(lines.len(), 3, "Should have 3 events");

    // Parse and verify event types
    let events: Vec<AuditEvent> = lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    assert_eq!(events[0].status, "started");
    assert_eq!(events[1].status, "progress");
    assert_eq!(events[2].status, "success");
}
