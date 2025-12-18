//! Unified event schema for Orbit observability
//!
//! This module provides the `OrbitEvent` structure that consolidates
//! audit events, telemetry events, and tracing events into a single
//! unified schema with cryptographic integrity support.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unified event schema for all Orbit operations
///
/// OrbitEvent provides a single event structure that replaces the legacy
/// AuditEvent and TelemetryEvent types. Every event includes:
/// - Trace correlation IDs (W3C Trace Context format)
/// - Job and file-level correlation
/// - Cryptographic integrity (HMAC chain)
/// - Monotonic sequencing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbitEvent {
    /// W3C Trace Context trace ID (32-char hex, 128-bit)
    pub trace_id: String,

    /// W3C Trace Context span ID (16-char hex, 64-bit)
    pub span_id: String,

    /// Parent span ID for distributed tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,

    /// Job-level correlation ID (from Magnetar/orbit-web)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,

    /// File-level correlation ID (format: "source -> dest")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,

    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Monotonic sequence number for ordering
    pub sequence: u64,

    /// HMAC-SHA256 hash linking to previous event in chain
    /// None for first event or if chaining is disabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity_hash: Option<String>,

    /// Event-specific payload
    pub payload: EventPayload,

    /// Optional structured metadata for extensibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl OrbitEvent {
    /// Create a new event with the given payload
    ///
    /// This initializes an event with:
    /// - Current timestamp
    /// - Zero sequence (will be set by AuditChain)
    /// - No trace IDs (should be set from TraceContext)
    pub fn new(payload: EventPayload) -> Self {
        Self {
            trace_id: String::new(),
            span_id: String::new(),
            parent_span_id: None,
            job_id: None,
            file_id: None,
            timestamp: Utc::now(),
            sequence: 0,
            integrity_hash: None,
            payload,
            metadata: None,
        }
    }

    /// Set trace context for this event
    pub fn with_trace(mut self, trace_id: String, span_id: String) -> Self {
        self.trace_id = trace_id;
        self.span_id = span_id;
        self
    }

    /// Set job ID for this event
    pub fn with_job(mut self, job_id: String) -> Self {
        self.job_id = Some(job_id);
        self
    }

    /// Set file ID for this event
    pub fn with_file(mut self, file_id: String) -> Self {
        self.file_id = Some(file_id);
        self
    }

    /// Set custom metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Event payload variants
///
/// The payload enum uses serde's "tag" attribute for clean JSON representation
/// where the event type is stored in a "type" field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    /// Job started
    JobStart {
        files: u32,
        total_bytes: u64,
        protocol: String,
    },

    /// Job completed successfully
    JobComplete { duration_ms: u64, digest: String },

    /// Job failed
    JobFailed { error: String, retries: u32 },

    /// File transfer started
    FileStart {
        source: String,
        dest: String,
        bytes: u64,
    },

    /// File transfer progress update
    FileProgress {
        bytes_transferred: u64,
        total_bytes: u64,
    },

    /// File transfer completed successfully
    FileComplete {
        bytes: u64,
        duration_ms: u64,
        checksum: String,
    },

    /// File transfer failed
    FileFailed {
        error: String,
        bytes_transferred: u64,
    },

    /// Window validated successfully (manifest transfers)
    WindowOk {
        window_id: u32,
        bytes: u64,
        repair: u32,
    },

    /// Window validation failed (manifest transfers)
    WindowFail { window_id: u32, error: String },

    /// Backend read operation
    BackendRead {
        path: String,
        bytes: u64,
        duration_ms: u64,
    },

    /// Backend write operation
    BackendWrite {
        path: String,
        bytes: u64,
        duration_ms: u64,
    },

    /// Backend list operation
    BackendList {
        path: String,
        entries: u64,
        duration_ms: u64,
    },

    /// Tracing span started
    SpanStart { name: String, level: String },

    /// Tracing span ended
    SpanEnd { name: String, duration_ms: u64 },

    /// Custom event for extensibility
    Custom {
        event_type: String,
        data: serde_json::Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = OrbitEvent::new(EventPayload::JobStart {
            files: 10,
            total_bytes: 1024,
            protocol: "s3".to_string(),
        })
        .with_trace("abc123".to_string(), "def456".to_string())
        .with_job("job-1".to_string());

        assert_eq!(event.trace_id, "abc123");
        assert_eq!(event.span_id, "def456");
        assert_eq!(event.job_id, Some("job-1".to_string()));
        assert!(event.timestamp <= Utc::now());
    }

    #[test]
    fn test_event_serialization() {
        let event = OrbitEvent::new(EventPayload::FileComplete {
            bytes: 1024,
            duration_ms: 100,
            checksum: "blake3:abc123".to_string(),
        })
        .with_trace("trace1".to_string(), "span1".to_string());

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"file_complete\""));
        assert!(json.contains("\"trace_id\":\"trace1\""));

        let deserialized: OrbitEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.trace_id, "trace1");
    }

    #[test]
    fn test_all_payload_variants() {
        // Ensure all payload variants serialize correctly
        let payloads = vec![
            EventPayload::JobStart {
                files: 1,
                total_bytes: 100,
                protocol: "local".to_string(),
            },
            EventPayload::JobComplete {
                duration_ms: 50,
                digest: "abc".to_string(),
            },
            EventPayload::JobFailed {
                error: "test".to_string(),
                retries: 3,
            },
            EventPayload::FileStart {
                source: "/src".to_string(),
                dest: "/dst".to_string(),
                bytes: 100,
            },
            EventPayload::FileProgress {
                bytes_transferred: 50,
                total_bytes: 100,
            },
            EventPayload::FileComplete {
                bytes: 100,
                duration_ms: 10,
                checksum: "abc".to_string(),
            },
            EventPayload::FileFailed {
                error: "test".to_string(),
                bytes_transferred: 50,
            },
            EventPayload::WindowOk {
                window_id: 1,
                bytes: 100,
                repair: 0,
            },
            EventPayload::WindowFail {
                window_id: 1,
                error: "test".to_string(),
            },
            EventPayload::BackendRead {
                path: "/test".to_string(),
                bytes: 100,
                duration_ms: 10,
            },
            EventPayload::BackendWrite {
                path: "/test".to_string(),
                bytes: 100,
                duration_ms: 10,
            },
            EventPayload::BackendList {
                path: "/test".to_string(),
                entries: 5,
                duration_ms: 10,
            },
            EventPayload::SpanStart {
                name: "test".to_string(),
                level: "INFO".to_string(),
            },
            EventPayload::SpanEnd {
                name: "test".to_string(),
                duration_ms: 10,
            },
            EventPayload::Custom {
                event_type: "test".to_string(),
                data: serde_json::json!({"key": "value"}),
            },
        ];

        for payload in payloads {
            let event = OrbitEvent::new(payload);
            let json = serde_json::to_string(&event).unwrap();
            let _deserialized: OrbitEvent = serde_json::from_str(&json).unwrap();
        }
    }
}
