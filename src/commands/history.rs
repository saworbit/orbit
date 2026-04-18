/*!
 * Orbit History Command
 *
 * Shows recent transfer history from the audit log in a human-friendly format.
 * Reads JSON Lines audit data and presents a summary table.
 */

use crate::cli_style::{section_header, Icons, Theme};
use crate::error::{OrbitError, Result};
use std::io::BufRead;
use std::path::PathBuf;

/// Maximum number of entries to show by default
pub const DEFAULT_LIMIT: usize = 20;

/// A parsed transfer record from the audit log
struct TransferRecord {
    timestamp: String,
    source: String,
    dest: String,
    bytes: Option<u64>,
    files: Option<u64>,
    duration_secs: Option<f64>,
    status: String,
}

/// Find the audit log path, checking common locations.
/// If an explicit path is given, it is used as-is (no fallback).
/// Returns Err if the explicit path doesn't exist, or Ok(None) if
/// no default log is found.
fn find_audit_log(
    explicit_path: Option<&PathBuf>,
) -> std::result::Result<Option<PathBuf>, PathBuf> {
    // 1. Explicit path from argument — authoritative, no fallback
    if let Some(p) = explicit_path {
        if p.exists() {
            return Ok(Some(p.clone()));
        } else {
            return Err(p.clone());
        }
    }

    // 2. Default locations: ~/.orbit/audit.log or ~/.orbit/audit.jsonl
    if let Some(home) = dirs::home_dir() {
        let default_path = home.join(".orbit").join("audit.log");
        if default_path.exists() {
            return Ok(Some(default_path));
        }
        let jsonl_path = home.join(".orbit").join("audit.jsonl");
        if jsonl_path.exists() {
            return Ok(Some(jsonl_path));
        }
    }

    Ok(None)
}

/// UTF-8 BOM that Windows tools (PowerShell Set-Content -Encoding utf8) prepend.
const UTF8_BOM: &str = "\u{FEFF}";

/// Parse a JSON line into a TransferRecord, being tolerant of schema variations.
/// Strips a leading UTF-8 BOM if present (common on Windows).
fn parse_record(line: &str) -> Option<TransferRecord> {
    let line = line.strip_prefix(UTF8_BOM).unwrap_or(line);
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let obj = v.as_object()?;

    // Try common field names for each piece of data
    let timestamp = obj
        .get("timestamp")
        .or_else(|| obj.get("time"))
        .or_else(|| obj.get("ts"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let source = obj
        .get("source")
        .or_else(|| obj.get("src"))
        .or_else(|| obj.get("source_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string();

    let dest = obj
        .get("destination")
        .or_else(|| obj.get("dest"))
        .or_else(|| obj.get("dest_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string();

    let bytes = obj
        .get("bytes_copied")
        .or_else(|| obj.get("bytes"))
        .or_else(|| obj.get("total_bytes"))
        .and_then(|v| v.as_u64());

    let files = obj
        .get("files_copied")
        .or_else(|| obj.get("files"))
        .or_else(|| obj.get("total_files"))
        .and_then(|v| v.as_u64());

    let duration_secs = obj
        .get("duration_secs")
        .or_else(|| obj.get("duration"))
        .or_else(|| obj.get("elapsed"))
        .and_then(|v| v.as_f64());

    let status = obj
        .get("status")
        .or_else(|| obj.get("result"))
        .or_else(|| obj.get("outcome"))
        .and_then(|v| v.as_str())
        .unwrap_or("ok")
        .to_string();

    Some(TransferRecord {
        timestamp,
        source,
        dest,
        bytes,
        files,
        duration_secs,
        status,
    })
}

/// Format bytes into human-readable form
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.1} TiB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Truncate a path string for display, keeping the most useful parts.
/// Uses char boundaries to avoid panicking on multibyte UTF-8.
fn truncate_path(path: &str, max_len: usize) -> String {
    if max_len < 4 {
        return path.chars().take(max_len).collect();
    }
    let char_count = path.chars().count();
    if char_count <= max_len {
        return path.to_string();
    }
    // Keep the last (max_len - 3) characters with "..." prefix
    let skip = char_count - (max_len - 3);
    let suffix: String = path.chars().skip(skip).collect();
    format!("...{}", suffix)
}

/// Run the history command
pub fn run_history(audit_path: Option<&PathBuf>, limit: usize, json_output: bool) -> Result<()> {
    let log_path = match find_audit_log(audit_path) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Err(OrbitError::Config(
                "No audit log found. Enable with --audit-log <path> or run transfers with audit logging enabled.".to_string(),
            ));
        }
        Err(missing) => {
            return Err(OrbitError::Config(format!(
                "Audit log not found: {}",
                missing.display()
            )));
        }
    };

    let file = std::fs::File::open(&log_path).map_err(OrbitError::Io)?;
    let reader = std::io::BufReader::new(file);

    // Read all lines and parse, keeping the last N
    let records: Vec<TransferRecord> = reader
        .lines()
        .map_while(|line| line.ok())
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| parse_record(&line))
        .collect();

    let total = records.len();
    let display_records: Vec<&TransferRecord> =
        records.iter().rev().take(limit).collect::<Vec<_>>();
    let display_records: Vec<&TransferRecord> = display_records.into_iter().rev().collect();

    if json_output {
        // JSON output mode: re-emit the parsed records as JSON
        for record in &display_records {
            let obj = serde_json::json!({
                "timestamp": record.timestamp,
                "source": record.source,
                "destination": record.dest,
                "bytes": record.bytes,
                "files": record.files,
                "duration_secs": record.duration_secs,
                "status": record.status,
            });
            println!("{}", obj);
        }
        return Ok(());
    }

    // Human-readable output
    println!();
    section_header(&format!("{} Transfer History", Icons::CLOCK));
    println!();
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Log file:"),
        Theme::value(log_path.display())
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Total entries:"),
        Theme::value(total)
    );
    if total > limit {
        println!(
            "  {} {}",
            Icons::BULLET,
            Theme::muted(format!("Showing last {} entries", limit))
        );
    }
    println!();

    if display_records.is_empty() {
        println!(
            "  {} {}",
            Icons::INFO,
            Theme::muted("No transfer records found.")
        );
        println!();
        return Ok(());
    }

    // Table header
    println!(
        "  {:<20} {:<6} {:<26} {:<26} {:>10} {:>8}",
        Theme::muted("Timestamp"),
        Theme::muted("Status"),
        Theme::muted("Source"),
        Theme::muted("Destination"),
        Theme::muted("Size"),
        Theme::muted("Duration"),
    );
    println!("  {}", Theme::muted("─".repeat(100)));

    for record in &display_records {
        let status_display = match record.status.as_str() {
            "ok" | "success" | "completed" => Theme::success("✓ ok"),
            "failed" | "error" => Theme::error("✗ err"),
            "partial" => Theme::warning("◐ part"),
            other => Theme::muted(other),
        };

        let size = record
            .bytes
            .map(format_size)
            .unwrap_or_else(|| "-".to_string());

        let duration = record
            .duration_secs
            .map(|d| {
                if d >= 60.0 {
                    format!("{:.0}m{:.0}s", d / 60.0, d % 60.0)
                } else {
                    format!("{:.1}s", d)
                }
            })
            .unwrap_or_else(|| "-".to_string());

        // Truncate the timestamp to just date+time (char-safe)
        let ts: String = record.timestamp.chars().take(19).collect();

        println!(
            "  {:<20} {:<6} {:<26} {:<26} {:>10} {:>8}",
            Theme::muted(ts),
            status_display,
            truncate_path(&record.source, 25),
            truncate_path(&record.dest, 25),
            size,
            duration,
        );
    }

    println!();
    println!(
        "  {} {}",
        Icons::INFO,
        Theme::muted("Use --json for machine-readable output. Use --limit N to show more entries.")
    );
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // ── parse_record tests ──────────────────────────────────────────

    #[test]
    fn test_parse_record_basic() {
        let line = r#"{"timestamp":"2026-01-15T10:30:00Z","source":"/tmp/a","destination":"/tmp/b","bytes_copied":1024,"status":"ok"}"#;
        let rec = parse_record(line).expect("should parse");
        assert_eq!(rec.timestamp, "2026-01-15T10:30:00Z");
        assert_eq!(rec.source, "/tmp/a");
        assert_eq!(rec.dest, "/tmp/b");
        assert_eq!(rec.bytes, Some(1024));
        assert_eq!(rec.status, "ok");
    }

    #[test]
    fn test_parse_record_with_bom() {
        // UTF-8 BOM followed by valid JSON — the exact bytes PowerShell writes
        let line = "\u{FEFF}{\"timestamp\":\"2026-01-15T10:30:00Z\",\"source\":\"/a\",\"dest\":\"/b\",\"status\":\"ok\"}";
        let rec = parse_record(line).expect("BOM-prefixed line should parse");
        assert_eq!(rec.timestamp, "2026-01-15T10:30:00Z");
        assert_eq!(rec.source, "/a");
        assert_eq!(rec.dest, "/b");
    }

    #[test]
    fn test_parse_record_bom_only() {
        // A line that is just a BOM and nothing else
        assert!(parse_record("\u{FEFF}").is_none());
    }

    #[test]
    fn test_parse_record_alternate_field_names() {
        let line = r#"{"time":"2026-01-15","src":"/x","dest":"/y","bytes":512,"result":"failed"}"#;
        let rec = parse_record(line).expect("should parse alternate names");
        assert_eq!(rec.timestamp, "2026-01-15");
        assert_eq!(rec.source, "/x");
        assert_eq!(rec.dest, "/y");
        assert_eq!(rec.bytes, Some(512));
        assert_eq!(rec.status, "failed");
    }

    #[test]
    fn test_parse_record_minimal() {
        // An object with none of the expected fields
        let line = r#"{"event":"something_else"}"#;
        let rec = parse_record(line).expect("should parse with defaults");
        assert_eq!(rec.timestamp, "unknown");
        assert_eq!(rec.source, "-");
    }

    #[test]
    fn test_parse_record_invalid_json() {
        assert!(parse_record("not json at all").is_none());
        assert!(parse_record("").is_none());
        assert!(parse_record("{truncated").is_none());
    }

    // ── truncate_path tests ─────────────────────────────────────────

    #[test]
    fn test_truncate_path_short() {
        assert_eq!(truncate_path("/tmp/a", 25), "/tmp/a");
    }

    #[test]
    fn test_truncate_path_exact() {
        let path = "a".repeat(25);
        assert_eq!(truncate_path(&path, 25), path);
    }

    #[test]
    fn test_truncate_path_long() {
        let path = "/very/long/path/to/some/deeply/nested/file.txt";
        let result = truncate_path(path, 20);
        assert!(result.starts_with("..."));
        assert_eq!(result.chars().count(), 20);
    }

    #[test]
    fn test_truncate_path_multibyte_unicode() {
        // 10 emoji characters (4 bytes each in UTF-8)
        let path = "📁📁📁📁📁📁📁📁📁📁";
        assert_eq!(path.chars().count(), 10);
        // Truncate to 8 chars — must not panic
        let result = truncate_path(path, 8);
        assert_eq!(result.chars().count(), 8);
        assert!(result.starts_with("..."));
    }

    #[test]
    fn test_truncate_path_cjk() {
        let path = "/ホーム/ユーザー/ドキュメント/ファイル.txt";
        let result = truncate_path(path, 15);
        assert!(result.starts_with("..."));
        assert_eq!(result.chars().count(), 15);
    }

    #[test]
    fn test_truncate_path_tiny_max() {
        let result = truncate_path("abcdef", 2);
        assert_eq!(result.chars().count(), 2);
    }

    // ── find_audit_log tests ────────────────────────────────────────

    #[test]
    fn test_find_audit_log_explicit_missing_returns_err() {
        let missing = PathBuf::from("/nonexistent/audit.log");
        let result = find_audit_log(Some(&missing));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), missing);
    }

    #[test]
    fn test_find_audit_log_explicit_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.log");
        std::fs::write(&path, "{}").unwrap();
        let result = find_audit_log(Some(&path));
        assert_eq!(result.unwrap(), Some(path));
    }

    #[test]
    fn test_find_audit_log_no_arg_no_default() {
        // With no arg and no default file, should return Ok(None)
        let result = find_audit_log(None);
        // We can't control ~/.orbit here, but at least verify it doesn't panic
        assert!(result.is_ok());
    }

    // ── run_history integration tests ───────────────────────────────

    #[test]
    fn test_run_history_missing_explicit_file_errors() {
        let missing = PathBuf::from("/no/such/file.log");
        let result = run_history(Some(&missing), 10, false);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("not found"),
            "error should mention not found: {}",
            msg
        );
    }

    #[test]
    fn test_run_history_bom_file_parses() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        // Write BOM + valid JSON line
        writeln!(
            f,
            "\u{FEFF}{{\"timestamp\":\"2026-01-15\",\"source\":\"/a\",\"dest\":\"/b\",\"status\":\"ok\"}}"
        )
        .unwrap();
        writeln!(
            f,
            "{{\"timestamp\":\"2026-01-16\",\"source\":\"/c\",\"dest\":\"/d\",\"status\":\"ok\"}}"
        )
        .unwrap();
        drop(f);

        // JSON mode so we can capture structured output
        let result = run_history(Some(&path), 10, true);
        assert!(result.is_ok(), "BOM file should parse: {:?}", result);
    }

    #[test]
    fn test_run_history_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.log");
        std::fs::write(&path, "").unwrap();
        // Should succeed with 0 entries, not error
        let result = run_history(Some(&path), 10, false);
        assert!(result.is_ok());
    }
}
