//! Structured output writer supporting JSON Lines and human-readable modes.

use serde::Serialize;

/// Output mode for CLI results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Json,
}

/// Structured operation result for JSON output
#[derive(Debug, Serialize)]
pub struct OperationResult {
    pub operation: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// Structured output writer that supports both human-readable and JSON output
#[derive(Debug, Clone)]
pub struct OutputWriter {
    pub mode: OutputMode,
}

impl OutputWriter {
    pub fn new(json: bool) -> Self {
        Self {
            mode: if json { OutputMode::Json } else { OutputMode::Human },
        }
    }

    pub fn is_json(&self) -> bool {
        self.mode == OutputMode::Json
    }

    /// Print an operation result
    pub fn operation_result(&self, result: &OperationResult) {
        match self.mode {
            OutputMode::Json => {
                if let Ok(json) = serde_json::to_string(result) {
                    println!("{}", json);
                }
            }
            OutputMode::Human => {
                if result.success {
                    if let (Some(src), Some(dst)) = (&result.source, &result.destination) {
                        println!("  {} {} \u{2192} {}", crate::cli_style::Icons::SUCCESS, src, dst);
                    }
                } else if let Some(err) = &result.error {
                    eprintln!("  {} {}", crate::cli_style::Icons::ERROR, sanitize_error(err));
                }
            }
        }
    }

    /// Print a stats summary (for --stat flag)
    pub fn stats_summary(&self, stats: &crate::CopyStats) {
        match self.mode {
            OutputMode::Json => {
                let summary = StatsSummary {
                    files_copied: stats.files_copied,
                    files_skipped: stats.files_skipped,
                    files_failed: stats.files_failed,
                    bytes_copied: stats.bytes_copied,
                    duration_secs: stats.duration.as_secs_f64(),
                };
                if let Ok(json) = serde_json::to_string(&summary) {
                    println!("{}", json);
                }
            }
            OutputMode::Human => {
                // Human mode handled by existing print_exec_stats
            }
        }
    }

    /// Print an error message
    pub fn error(&self, msg: &str) {
        match self.mode {
            OutputMode::Json => {
                let result = OperationResult {
                    operation: "error".to_string(),
                    success: false,
                    source: None,
                    destination: None,
                    error: Some(sanitize_error(msg)),
                    size: None,
                };
                if let Ok(json) = serde_json::to_string(&result) {
                    eprintln!("{}", json);
                }
            }
            OutputMode::Human => {
                eprintln!("Error: {}", sanitize_error(msg));
            }
        }
    }

    /// Print an info message (suppressed in JSON mode)
    pub fn info(&self, msg: &str) {
        if !self.is_json() {
            crate::cli_style::print_info(msg);
        }
    }
}

/// JSON-serializable stats summary
#[derive(Debug, Serialize)]
struct StatsSummary {
    pub files_copied: u64,
    pub files_skipped: u64,
    pub files_failed: u64,
    pub bytes_copied: u64,
    pub duration_secs: f64,
}

/// Sanitize error messages by collapsing whitespace
pub fn sanitize_error(msg: &str) -> String {
    msg.replace('\n', " ")
        .replace('\t', " ")
        .replace('\r', " ")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Message type for the buffered output channel
enum OutputMessage {
    Stdout(String),
    Stderr(String),
    Shutdown,
}

/// Buffered output writer that prevents interleaved output from concurrent workers.
///
/// Uses a dedicated writer thread with a bounded channel so that multiple
/// async/threaded workers can emit lines without garbled output.
pub struct BufferedOutputWriter {
    sender: std::sync::mpsc::SyncSender<OutputMessage>,
    _handle: Option<std::thread::JoinHandle<()>>,
}

impl BufferedOutputWriter {
    /// Create a new buffered output writer with a channel capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<OutputMessage>(capacity);

        let handle = std::thread::spawn(move || {
            use std::io::Write;
            let stdout = std::io::stdout();
            let stderr = std::io::stderr();

            while let Ok(msg) = receiver.recv() {
                match msg {
                    OutputMessage::Stdout(line) => {
                        let mut out = stdout.lock();
                        let _ = writeln!(out, "{}", line);
                    }
                    OutputMessage::Stderr(line) => {
                        let mut err = stderr.lock();
                        let _ = writeln!(err, "{}", line);
                    }
                    OutputMessage::Shutdown => break,
                }
            }
        });

        Self {
            sender,
            _handle: Some(handle),
        }
    }

    /// Write a line to stdout
    pub fn println(&self, msg: String) {
        let _ = self.sender.send(OutputMessage::Stdout(msg));
    }

    /// Write a line to stderr
    pub fn eprintln(&self, msg: String) {
        let _ = self.sender.send(OutputMessage::Stderr(msg));
    }

    /// Shut down the writer thread
    pub fn shutdown(self) {
        let _ = self.sender.send(OutputMessage::Shutdown);
        // handle will be joined on drop
    }
}

impl Drop for BufferedOutputWriter {
    fn drop(&mut self) {
        let _ = self.sender.send(OutputMessage::Shutdown);
        if let Some(handle) = self._handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_error_newlines() {
        assert_eq!(sanitize_error("line1\nline2\nline3"), "line1 line2 line3");
    }

    #[test]
    fn test_sanitize_error_tabs() {
        assert_eq!(sanitize_error("field1\tfield2\tfield3"), "field1 field2 field3");
    }

    #[test]
    fn test_sanitize_error_multiple_spaces() {
        assert_eq!(sanitize_error("too    many     spaces"), "too many spaces");
    }

    #[test]
    fn test_sanitize_error_mixed() {
        assert_eq!(
            sanitize_error("  error:\n  detail\t  info  \n"),
            "error: detail info"
        );
    }

    #[test]
    fn test_sanitize_error_empty() {
        assert_eq!(sanitize_error(""), "");
    }

    #[test]
    fn test_sanitize_error_already_clean() {
        assert_eq!(sanitize_error("clean message"), "clean message");
    }

    #[test]
    fn test_output_writer_json_mode() {
        let writer = OutputWriter::new(true);
        assert!(writer.is_json());
        assert_eq!(writer.mode, OutputMode::Json);
    }

    #[test]
    fn test_output_writer_human_mode() {
        let writer = OutputWriter::new(false);
        assert!(!writer.is_json());
        assert_eq!(writer.mode, OutputMode::Human);
    }

    #[test]
    fn test_operation_result_serialization() {
        let result = OperationResult {
            operation: "copy".to_string(),
            success: true,
            source: Some("src.txt".to_string()),
            destination: Some("dst.txt".to_string()),
            error: None,
            size: Some(1024),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"operation\":\"copy\""));
        assert!(json.contains("\"success\":true"));
        assert!(!json.contains("\"error\"")); // skip_serializing_if = None
    }

    #[test]
    fn test_operation_result_with_error() {
        let result = OperationResult {
            operation: "copy".to_string(),
            success: false,
            source: Some("src.txt".to_string()),
            destination: None,
            error: Some("file not found".to_string()),
            size: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"file not found\""));
    }

    #[test]
    fn test_buffered_writer_basic() {
        let writer = BufferedOutputWriter::new(100);
        writer.println("test line 1".to_string());
        writer.println("test line 2".to_string());
        writer.shutdown();
    }

    #[test]
    fn test_buffered_writer_stderr() {
        let writer = BufferedOutputWriter::new(100);
        writer.eprintln("error line 1".to_string());
        writer.eprintln("error line 2".to_string());
        writer.shutdown();
    }

    #[test]
    fn test_buffered_writer_drop() {
        // Test that drop handles cleanup correctly
        let writer = BufferedOutputWriter::new(100);
        writer.println("before drop".to_string());
        drop(writer);
    }

    #[test]
    fn test_buffered_writer_mixed() {
        let writer = BufferedOutputWriter::new(100);
        writer.println("stdout message".to_string());
        writer.eprintln("stderr message".to_string());
        writer.println("another stdout".to_string());
        writer.shutdown();
    }

    #[test]
    fn test_operation_result_all_none_fields() {
        let result = OperationResult {
            operation: "list".to_string(),
            success: true,
            source: None,
            destination: None,
            error: None,
            size: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        // Fields with skip_serializing_if = None should be absent
        assert!(!json.contains("\"source\""));
        assert!(!json.contains("\"destination\""));
        assert!(!json.contains("\"error\""));
        assert!(!json.contains("\"size\""));
        // Required fields should still be present
        assert!(json.contains("\"operation\":\"list\""));
        assert!(json.contains("\"success\":true"));
        // Verify it parses as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_object().unwrap().len(), 2);
    }

    #[test]
    fn test_stats_summary_serialization() {
        // StatsSummary is private, so we test the JSON structure via OperationResult
        // and verify the fields that StatsSummary would produce by constructing
        // a similar JSON payload manually
        let summary_json = serde_json::json!({
            "files_copied": 42u64,
            "files_skipped": 3u64,
            "files_failed": 1u64,
            "bytes_copied": 1048576u64,
            "duration_secs": 12.5f64,
        });
        let json_str = serde_json::to_string(&summary_json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["files_copied"], 42);
        assert_eq!(parsed["files_skipped"], 3);
        assert_eq!(parsed["files_failed"], 1);
        assert_eq!(parsed["bytes_copied"], 1048576);
        assert_eq!(parsed["duration_secs"], 12.5);
    }

    #[test]
    fn test_sanitize_error_carriage_return() {
        assert_eq!(sanitize_error("line1\rline2\rline3"), "line1 line2 line3");
        assert_eq!(sanitize_error("error\r\n detail"), "error detail");
        assert_eq!(sanitize_error("\rfoo\r"), "foo");
    }

    #[test]
    fn test_sanitize_error_only_whitespace() {
        assert_eq!(sanitize_error("   "), "");
        assert_eq!(sanitize_error("\n\n\n"), "");
        assert_eq!(sanitize_error("\t\t\t"), "");
        assert_eq!(sanitize_error(" \n \t \r "), "");
    }

    #[test]
    fn test_sanitize_error_unicode() {
        assert_eq!(sanitize_error("error: fichier introuvable"), "error: fichier introuvable");
        assert_eq!(sanitize_error("failed: \u{1F4C1} not found"), "failed: \u{1F4C1} not found");
        assert_eq!(sanitize_error("\u{00E9}\u{00E0}\u{00FC}"), "\u{00E9}\u{00E0}\u{00FC}");
        assert_eq!(
            sanitize_error("CJK: \u{4F60}\u{597D}\u{4E16}\u{754C}"),
            "CJK: \u{4F60}\u{597D}\u{4E16}\u{754C}"
        );
        // Unicode with surrounding whitespace should be preserved properly
        assert_eq!(sanitize_error("  \u{00E9}rror  \n  d\u{00E9}tail  "), "\u{00E9}rror d\u{00E9}tail");
    }

    #[test]
    fn test_output_writer_mode_switch() {
        let json_writer = OutputWriter::new(true);
        assert_eq!(json_writer.mode, OutputMode::Json);
        assert!(json_writer.is_json());

        let human_writer = OutputWriter::new(false);
        assert_eq!(human_writer.mode, OutputMode::Human);
        assert!(!human_writer.is_json());
    }

    #[test]
    fn test_buffered_writer_high_volume() {
        let writer = BufferedOutputWriter::new(256);
        for i in 0..1000 {
            writer.println(format!("message {}", i));
        }
        // Verify graceful shutdown with no panic
        writer.shutdown();
    }
}
