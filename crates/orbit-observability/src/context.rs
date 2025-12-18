//! Trace context and correlation IDs
//!
//! This module provides `TraceContext` for managing distributed trace
//! correlation using W3C Trace Context format trace IDs and span IDs.
//!
//! The trace context maintains a hierarchy of correlation IDs:
//! - `trace_id`: Top-level operation identifier (W3C format: 32-char hex)
//! - `span_id`: Current operation span (W3C format: 16-char hex)
//! - `job_id`: Job-level correlation (from Magnetar/orbit-web)
//! - `file_id`: File-level correlation (format: "source -> dest")

use rand::Rng;
use std::path::Path;

/// Trace context for distributed correlation
///
/// TraceContext implements a hierarchical correlation model compatible
/// with OpenTelemetry and W3C Trace Context specification.
///
/// ## Example
///
/// ```
/// use orbit_observability::TraceContext;
/// use std::path::Path;
///
/// // Create root context for a new operation
/// let ctx = TraceContext::new_root();
/// println!("Trace ID: {}", ctx.trace_id);
///
/// // Add job-level correlation
/// let ctx = ctx.with_job("job-123".to_string());
///
/// // Add file-level correlation
/// let src = Path::new("/source/file.txt");
/// let dst = Path::new("/dest/file.txt");
/// let ctx = ctx.with_file(src, dst);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    /// W3C Trace Context trace ID (32-char hex, 128-bit)
    ///
    /// This ID is shared across all spans in a distributed trace.
    /// Format: 32 hexadecimal characters (e.g., "4bf92f3577b34da6a3ce929d0e0e4736")
    pub trace_id: String,

    /// W3C Trace Context span ID (16-char hex, 64-bit)
    ///
    /// Identifies the current span within the trace.
    /// Format: 16 hexadecimal characters (e.g., "00f067aa0ba902b7")
    pub span_id: String,

    /// Parent span ID (for hierarchical spans)
    ///
    /// When creating a child span, set this to the parent's span_id
    pub parent_span_id: Option<String>,

    /// Job-level correlation ID
    ///
    /// This is typically assigned by the Magnetar scheduler or orbit-web API
    /// and groups all file transfers within a single job.
    pub job_id: Option<String>,

    /// File-level correlation ID
    ///
    /// Format: "source_path -> dest_path"
    /// Used to correlate all events for a specific file transfer.
    pub file_id: Option<String>,
}

impl TraceContext {
    /// Create a new root trace context
    ///
    /// This generates a new random trace_id and span_id following
    /// the W3C Trace Context specification.
    pub fn new_root() -> Self {
        Self {
            trace_id: generate_trace_id(),
            span_id: generate_span_id(),
            parent_span_id: None,
            job_id: None,
            file_id: None,
        }
    }

    /// Create a child span context
    ///
    /// This creates a new span with the same trace_id but a new span_id,
    /// setting the current span_id as the parent.
    pub fn child_span(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: generate_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            job_id: self.job_id.clone(),
            file_id: self.file_id.clone(),
        }
    }

    /// Add job ID to this context
    pub fn with_job(mut self, job_id: String) -> Self {
        self.job_id = Some(job_id);
        self
    }

    /// Add file ID to this context
    ///
    /// The file ID is formatted as "source -> dest"
    pub fn with_file(mut self, source: &Path, dest: &Path) -> Self {
        self.file_id = Some(format!("{} -> {}", source.display(), dest.display()));
        self
    }

    /// Add file ID from string
    pub fn with_file_id(mut self, file_id: String) -> Self {
        self.file_id = Some(file_id);
        self
    }

    /// Set parent span ID
    pub fn with_parent_span(mut self, parent_span_id: String) -> Self {
        self.parent_span_id = Some(parent_span_id);
        self
    }

    /// Create context from existing trace and span IDs
    ///
    /// This is useful when propagating context from external systems
    /// (e.g., incoming HTTP headers with traceparent)
    pub fn from_ids(trace_id: String, span_id: String) -> Self {
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            job_id: None,
            file_id: None,
        }
    }

    /// Format as W3C traceparent header
    ///
    /// Format: 00-{trace_id}-{span_id}-{flags}
    /// flags are always "01" (sampled)
    pub fn to_traceparent(&self) -> String {
        format!("00-{}-{}-01", self.trace_id, self.span_id)
    }

    /// Parse from W3C traceparent header
    ///
    /// Expected format: 00-{trace_id}-{span_id}-{flags}
    pub fn from_traceparent(traceparent: &str) -> Option<Self> {
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() != 4 || parts[0] != "00" {
            return None;
        }

        Some(Self::from_ids(parts[1].to_string(), parts[2].to_string()))
    }
}

/// Generate a W3C Trace Context trace ID
///
/// Format: 32 hexadecimal characters (128 bits of randomness)
/// Example: "4bf92f3577b34da6a3ce929d0e0e4736"
fn generate_trace_id() -> String {
    let mut rng = rand::rng();
    let bytes: [u8; 16] = rng.random();
    hex::encode(bytes)
}

/// Generate a W3C Trace Context span ID
///
/// Format: 16 hexadecimal characters (64 bits of randomness)
/// Example: "00f067aa0ba902b7"
fn generate_span_id() -> String {
    let mut rng = rand::rng();
    let bytes: [u8; 8] = rng.random();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_new_root() {
        let ctx = TraceContext::new_root();

        assert_eq!(ctx.trace_id.len(), 32); // 16 bytes * 2 hex chars
        assert_eq!(ctx.span_id.len(), 16); // 8 bytes * 2 hex chars
        assert!(ctx.parent_span_id.is_none());
        assert!(ctx.job_id.is_none());
        assert!(ctx.file_id.is_none());

        // Verify hex format
        assert!(ctx.trace_id.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(ctx.span_id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_with_job() {
        let ctx = TraceContext::new_root().with_job("job-123".to_string());

        assert_eq!(ctx.job_id, Some("job-123".to_string()));
    }

    #[test]
    fn test_with_file() {
        let src = PathBuf::from("/source/file.txt");
        let dst = PathBuf::from("/dest/file.txt");

        let ctx = TraceContext::new_root().with_file(&src, &dst);

        assert_eq!(
            ctx.file_id,
            Some("/source/file.txt -> /dest/file.txt".to_string())
        );
    }

    #[test]
    fn test_with_file_id() {
        let ctx = TraceContext::new_root().with_file_id("custom-file-id".to_string());

        assert_eq!(ctx.file_id, Some("custom-file-id".to_string()));
    }

    #[test]
    fn test_child_span() {
        let parent = TraceContext::new_root();
        let child = parent.child_span();

        // Same trace ID
        assert_eq!(child.trace_id, parent.trace_id);

        // Different span ID
        assert_ne!(child.span_id, parent.span_id);

        // Parent span ID set
        assert_eq!(child.parent_span_id, Some(parent.span_id.clone()));
    }

    #[test]
    fn test_child_span_preserves_correlation() {
        let parent = TraceContext::new_root()
            .with_job("job-456".to_string())
            .with_file_id("file-789".to_string());

        let child = parent.child_span();

        assert_eq!(child.job_id, parent.job_id);
        assert_eq!(child.file_id, parent.file_id);
    }

    #[test]
    fn test_from_ids() {
        let ctx = TraceContext::from_ids(
            "4bf92f3577b34da6a3ce929d0e0e4736".to_string(),
            "00f067aa0ba902b7".to_string(),
        );

        assert_eq!(ctx.trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(ctx.span_id, "00f067aa0ba902b7");
    }

    #[test]
    fn test_to_traceparent() {
        let ctx = TraceContext::from_ids(
            "4bf92f3577b34da6a3ce929d0e0e4736".to_string(),
            "00f067aa0ba902b7".to_string(),
        );

        let traceparent = ctx.to_traceparent();
        assert_eq!(
            traceparent,
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        );
    }

    #[test]
    fn test_from_traceparent() {
        let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let ctx = TraceContext::from_traceparent(traceparent).unwrap();

        assert_eq!(ctx.trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(ctx.span_id, "00f067aa0ba902b7");
    }

    #[test]
    fn test_from_traceparent_invalid() {
        assert!(TraceContext::from_traceparent("invalid").is_none());
        assert!(TraceContext::from_traceparent("01-abc-def-01").is_none());
        assert!(TraceContext::from_traceparent("00-abc").is_none());
    }

    #[test]
    fn test_roundtrip_traceparent() {
        let ctx1 = TraceContext::new_root();
        let traceparent = ctx1.to_traceparent();
        let ctx2 = TraceContext::from_traceparent(&traceparent).unwrap();

        assert_eq!(ctx1.trace_id, ctx2.trace_id);
        assert_eq!(ctx1.span_id, ctx2.span_id);
    }

    #[test]
    fn test_generate_unique_ids() {
        let ctx1 = TraceContext::new_root();
        let ctx2 = TraceContext::new_root();

        // Different contexts should have different IDs
        assert_ne!(ctx1.trace_id, ctx2.trace_id);
        assert_ne!(ctx1.span_id, ctx2.span_id);
    }

    #[test]
    fn test_clone_preserves_all_fields() {
        let ctx1 = TraceContext::new_root()
            .with_job("job-1".to_string())
            .with_file_id("file-1".to_string())
            .with_parent_span("parent-span".to_string());

        let ctx2 = ctx1.clone();

        assert_eq!(ctx1, ctx2);
    }
}
