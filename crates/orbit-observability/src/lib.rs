//! Orbit Unified Observability & Immutable Audit Plane
//!
//! This crate provides enterprise-grade observability for Orbit with:
//! - **Cryptographic integrity** via HMAC-SHA256 audit chaining
//! - **Distributed tracing** with OpenTelemetry integration
//! - **Unified event schema** replacing separate AuditEvent/TelemetryEvent
//! - **Prometheus metrics** for operational monitoring
//!
//! ## Architecture
//!
//! The observability system has three pillars:
//!
//! 1. **Troubleshooting (Standard Out/File)**: Use `RUST_LOG=debug` to capture
//!    low-level system logs with trace IDs for correlation.
//!
//! 2. **Compliance (Audit Plane)**: Immutable audit trail at `/var/log/orbit/audit.jsonl`
//!    with cryptographic chaining for tamper detection.
//!
//! 3. **Real-Time Monitoring (Prometheus)**: Metrics endpoint exports operational
//!    metrics like `orbit_transfer_retries_total` and `orbit_backend_latency_seconds`.
//!
//! ## Quick Start
//!
//! ```no_run
//! use orbit_observability::{
//!     AuditSigner, UnifiedLogger, AuditBridgeLayer, TraceContext, EventPayload,
//! };
//! use tracing_subscriber::prelude::*;
//! use std::path::Path;
//!
//! // 1. Initialize audit logger with HMAC secret
//! let signer = AuditSigner::from_env().expect("ORBIT_AUDIT_SECRET not set");
//! let logger = UnifiedLogger::new(Some(Path::new("audit.jsonl")), signer).unwrap();
//!
//! // 2. Set up tracing-to-audit bridge
//! let audit_layer = AuditBridgeLayer::new(logger.clone());
//!
//! tracing_subscriber::registry()
//!     .with(audit_layer)
//!     .init();
//!
//! // 3. Emit events with trace context
//! let ctx = TraceContext::new_root().with_job("job-123".to_string());
//! logger.emit_with_context(&ctx, EventPayload::JobStart {
//!     files: 10,
//!     total_bytes: 1024000,
//!     protocol: "s3".to_string(),
//! }).unwrap();
//! ```
//!
//! ## Cryptographic Audit Chaining
//!
//! Each event is signed with HMAC-SHA256, linking it to the previous event:
//!
//! ```text
//! Event 0: HMAC(secret, initial_hash || event_0_bytes) -> hash_0
//! Event 1: HMAC(secret, hash_0 || event_1_bytes) -> hash_1
//! Event 2: HMAC(secret, hash_1 || event_2_bytes) -> hash_2
//! ```
//!
//! Any tampering breaks the chain and is detected by the forensic validator.
//!
//! ## Forensic Validation
//!
//! Use the provided Python script to verify audit log integrity:
//!
//! ```bash
//! export ORBIT_AUDIT_SECRET="your_secret_key"
//! python3 scripts/verify_audit.py /var/log/orbit/audit.jsonl
//! ```
//!
//! Or use the Rust API:
//!
//! ```no_run
//! use orbit_observability::{AuditSigner, testing::validate_audit_file};
//! use std::path::Path;
//!
//! let signer = AuditSigner::from_bytes(b"secret");
//! let report = validate_audit_file(Path::new("audit.jsonl"), &signer).unwrap();
//!
//! if report.is_valid() {
//!     println!("✓ Audit log verified: {} records immutable", report.valid_events);
//! } else {
//!     eprintln!("✗ Integrity failures: {:?}", report.failures);
//! }
//! ```
//!
//! ## OpenTelemetry Integration
//!
//! The `TraceContext` follows W3C Trace Context specification and integrates
//! with OpenTelemetry exporters:
//!
//! ```no_run
//! use orbit_observability::TraceContext;
//!
//! let ctx = TraceContext::new_root();
//! println!("Traceparent: {}", ctx.to_traceparent());
//! // Output: 00-{trace_id}-{span_id}-01
//! ```
//!
//! ## Prometheus Metrics
//!
//! Export metrics for Prometheus scraping:
//!
//! ```
//! use orbit_observability::metrics;
//!
//! // Record metrics
//! metrics::inc_transfer_retry("s3");
//! metrics::record_backend_latency("s3", "write", 0.05);
//! metrics::inc_transfer_bytes("s3", "success", 1024);
//!
//! // Get metrics in Prometheus format
//! let metrics_text = metrics::metrics_text();
//! ```

// Core modules
pub mod chain;
pub mod context;
pub mod event;
pub mod logger;
pub mod signer;

// Integration modules
pub mod bridge;
pub mod metrics;

// Testing utilities
pub mod testing;

// Re-export commonly used types
pub use chain::{AuditChain, ChainError, ValidationReport};
pub use context::TraceContext;
pub use event::{EventPayload, OrbitEvent};
pub use logger::{LoggerError, UnifiedLogger};
pub use signer::{AuditSigner, SignerError};

// Re-export bridge layer
pub use bridge::AuditBridgeLayer;

// Re-export testing utilities
pub use testing::{load_events_from_file, validate_audit_file, EventCapture};

/// Prelude module for convenient imports
///
/// ```
/// use orbit_observability::prelude::*;
/// ```
pub mod prelude {
    pub use crate::bridge::AuditBridgeLayer;
    pub use crate::chain::{AuditChain, ValidationReport};
    pub use crate::context::TraceContext;
    pub use crate::event::{EventPayload, OrbitEvent};
    pub use crate::logger::UnifiedLogger;
    pub use crate::signer::AuditSigner;
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_end_to_end_audit_chain() {
        let temp_file = NamedTempFile::new().unwrap();
        let signer = AuditSigner::from_bytes(b"integration_test_secret");
        let logger = UnifiedLogger::new(Some(temp_file.path()), signer.clone()).unwrap();

        // Create a trace context
        let ctx = TraceContext::new_root().with_job("integration-job-1".to_string());

        // Emit a sequence of events
        logger.emit_job_start(&ctx, 5, 5000, "s3").unwrap();

        for i in 0..5 {
            let file_ctx = ctx.clone().with_file_id(format!("file-{}", i));
            logger
                .emit_file_start(
                    &file_ctx,
                    &format!("/src/{}", i),
                    &format!("/dst/{}", i),
                    1000,
                )
                .unwrap();
            logger
                .emit_file_complete(&file_ctx, 1000, 100, "blake3:abc")
                .unwrap();
        }

        logger.emit_job_complete(&ctx, 500, "final_digest").unwrap();

        logger.flush().unwrap();

        // Verify the chain
        let report = validate_audit_file(temp_file.path(), &signer).unwrap();
        assert!(report.is_valid());
        assert_eq!(report.valid_events, 12); // 1 job_start + 5*(file_start + file_complete) + 1 job_complete
    }

    #[test]
    fn test_trace_context_w3c_compliance() {
        let ctx = TraceContext::new_root();

        // Trace ID should be 32 hex chars (128 bits)
        assert_eq!(ctx.trace_id.len(), 32);
        assert!(ctx.trace_id.chars().all(|c| c.is_ascii_hexdigit()));

        // Span ID should be 16 hex chars (64 bits)
        assert_eq!(ctx.span_id.len(), 16);
        assert!(ctx.span_id.chars().all(|c| c.is_ascii_hexdigit()));

        // Traceparent format: 00-{trace_id}-{span_id}-01
        let traceparent = ctx.to_traceparent();
        assert!(traceparent.starts_with("00-"));
        assert!(traceparent.ends_with("-01"));

        // Round-trip test
        let ctx2 = TraceContext::from_traceparent(&traceparent).unwrap();
        assert_eq!(ctx.trace_id, ctx2.trace_id);
        assert_eq!(ctx.span_id, ctx2.span_id);
    }

    #[test]
    fn test_event_serialization_stability() {
        // Ensure event serialization is stable (important for HMAC)
        let event = OrbitEvent::new(EventPayload::JobStart {
            files: 10,
            total_bytes: 1024,
            protocol: "s3".to_string(),
        })
        .with_trace("abc123".to_string(), "def456".to_string())
        .with_job("job-1".to_string());

        let json1 = serde_json::to_string(&event).unwrap();
        let json2 = serde_json::to_string(&event).unwrap();

        assert_eq!(json1, json2);
    }

    #[test]
    fn test_metrics_registration() {
        // Initialize metrics
        let _ = metrics::registry();

        // Record some metrics
        metrics::inc_transfer_retry("s3");
        metrics::record_backend_latency("s3", "write", 0.05);
        metrics::inc_transfer_bytes("s3", "success", 1024);
        metrics::record_job_duration("s3", "success", 30.5);

        // Get metrics text
        let output = metrics::metrics_text();
        assert!(output.contains("orbit_transfer_retries_total"));
        assert!(output.contains("orbit_backend_latency_seconds"));
        assert!(output.contains("orbit_transfer_bytes_total"));
        assert!(output.contains("orbit_job_duration_seconds"));
    }
}
