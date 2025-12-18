//! Testing utilities for forensic validation
//!
//! This module provides helpers for testing audit chains and
//! validating event integrity.

use crate::chain::{AuditChain, ValidationReport};
use crate::event::OrbitEvent;
use crate::logger::UnifiedLogger;
use crate::signer::AuditSigner;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Event capture utility for testing
///
/// EventCapture intercepts events emitted during tests for later
/// inspection and validation.
///
/// ## Example
///
/// ```
/// use orbit_observability::{EventCapture, TraceContext, EventPayload};
///
/// let capture = EventCapture::new();
/// let logger = capture.logger();
/// let ctx = TraceContext::new_root();
///
/// logger.emit_with_context(&ctx, EventPayload::Custom {
///     event_type: "test".to_string(),
///     data: serde_json::json!({"key": "value"}),
/// }).unwrap();
///
/// assert_eq!(capture.event_count(), 1);
/// let events = capture.events();
/// assert_eq!(events[0].payload.to_string(), "custom");
/// ```
pub struct EventCapture {
    events: Arc<Mutex<Vec<OrbitEvent>>>,
    signer: AuditSigner,
}

impl EventCapture {
    /// Create a new event capture
    pub fn new() -> Self {
        Self::with_secret(b"test_capture_secret")
    }

    /// Create a new event capture with a specific secret
    pub fn with_secret(secret: &[u8]) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            signer: AuditSigner::from_bytes(secret),
        }
    }

    /// Get a logger that captures events
    ///
    /// Events emitted to this logger will be stored in memory
    /// for later inspection.
    pub fn logger(&self) -> CapturingLogger {
        CapturingLogger {
            events: Arc::clone(&self.events),
            logger: UnifiedLogger::disabled(),
            chain: AuditChain::new(self.signer.clone()),
        }
    }

    /// Get all captured events
    pub fn events(&self) -> Vec<OrbitEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Get the number of captured events
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Clear all captured events
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Validate the chain integrity
    pub fn assert_chain_valid(&self) -> ValidationReport {
        let events = self.events();
        AuditChain::verify_chain(&events, &self.signer).expect("Chain validation failed")
    }

    /// Assert that sequence numbers are monotonically increasing
    pub fn assert_sequence_monotonic(&self) {
        let events = self.events();
        for window in events.windows(2) {
            assert!(
                window[1].sequence > window[0].sequence,
                "Sequence not monotonic: {} -> {}",
                window[0].sequence,
                window[1].sequence
            );
        }
    }

    /// Find events matching a predicate
    pub fn find_events<F>(&self, predicate: F) -> Vec<OrbitEvent>
    where
        F: Fn(&OrbitEvent) -> bool,
    {
        self.events().into_iter().filter(predicate).collect()
    }

    /// Find events by job ID
    pub fn events_for_job(&self, job_id: &str) -> Vec<OrbitEvent> {
        self.find_events(|e| e.job_id.as_deref() == Some(job_id))
    }

    /// Find events by trace ID
    pub fn events_for_trace(&self, trace_id: &str) -> Vec<OrbitEvent> {
        self.find_events(|e| e.trace_id == trace_id)
    }
}

impl Default for EventCapture {
    fn default() -> Self {
        Self::new()
    }
}

/// Logger that captures events in memory
pub struct CapturingLogger {
    events: Arc<Mutex<Vec<OrbitEvent>>>,
    logger: UnifiedLogger,
    chain: AuditChain,
}

impl CapturingLogger {
    /// Emit an event (captures it in memory)
    pub fn emit(&self, mut event: OrbitEvent) -> crate::logger::Result<()> {
        // Sign the event
        self.chain
            .sign_event(&mut event)
            .map_err(|e| crate::logger::LoggerError::Chain(e))?;

        // Store in memory
        self.events.lock().unwrap().push(event);

        Ok(())
    }

    /// Emit with trace context
    pub fn emit_with_context(
        &self,
        ctx: &crate::context::TraceContext,
        payload: crate::event::EventPayload,
    ) -> crate::logger::Result<()> {
        let mut event = crate::event::OrbitEvent::new(payload)
            .with_trace(ctx.trace_id.clone(), ctx.span_id.clone());

        if let Some(ref job) = ctx.job_id {
            event.job_id = Some(job.clone());
        }
        if let Some(ref file) = ctx.file_id {
            event.file_id = Some(file.clone());
        }

        self.emit(event)
    }

    /// Get current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.chain.current_sequence()
    }
}

/// Load events from a JSON Lines audit log file
///
/// This is useful for forensic analysis of existing audit logs.
pub fn load_events_from_file(path: &Path) -> std::io::Result<Vec<OrbitEvent>> {
    let contents = std::fs::read_to_string(path)?;
    let mut events = Vec::new();

    for (line_num, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let event: OrbitEvent = serde_json::from_str(line).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse line {}: {}", line_num + 1, e),
            )
        })?;

        events.push(event);
    }

    Ok(events)
}

/// Validate an audit log file
///
/// Returns a ValidationReport with details about the chain integrity.
pub fn validate_audit_file(path: &Path, signer: &AuditSigner) -> std::io::Result<ValidationReport> {
    let events = load_events_from_file(path)?;
    AuditChain::verify_chain(&events, signer)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::TraceContext;
    use crate::event::EventPayload;

    #[test]
    fn test_event_capture() {
        let capture = EventCapture::new();
        let logger = capture.logger();
        let ctx = TraceContext::new_root();

        logger
            .emit_with_context(
                &ctx,
                EventPayload::Custom {
                    event_type: "test".to_string(),
                    data: serde_json::json!({"value": 123}),
                },
            )
            .unwrap();

        assert_eq!(capture.event_count(), 1);
        let events = capture.events();
        assert_eq!(events[0].trace_id, ctx.trace_id);
    }

    #[test]
    fn test_chain_validation() {
        let capture = EventCapture::new();
        let logger = capture.logger();
        let ctx = TraceContext::new_root();

        for i in 0..5 {
            logger
                .emit_with_context(
                    &ctx,
                    EventPayload::Custom {
                        event_type: format!("test_{}", i),
                        data: serde_json::json!({"value": i}),
                    },
                )
                .unwrap();
        }

        let report = capture.assert_chain_valid();
        assert!(report.is_valid());
        assert_eq!(report.valid_events, 5);
    }

    #[test]
    fn test_sequence_monotonic() {
        let capture = EventCapture::new();
        let logger = capture.logger();
        let ctx = TraceContext::new_root();

        for i in 0..10 {
            logger
                .emit_with_context(
                    &ctx,
                    EventPayload::Custom {
                        event_type: format!("test_{}", i),
                        data: serde_json::json!({}),
                    },
                )
                .unwrap();
        }

        capture.assert_sequence_monotonic();
    }

    #[test]
    fn test_find_events() {
        let capture = EventCapture::new();
        let logger = capture.logger();
        let ctx = TraceContext::new_root().with_job("job-123".to_string());

        logger
            .emit_with_context(
                &ctx,
                EventPayload::JobStart {
                    files: 1,
                    total_bytes: 100,
                    protocol: "local".to_string(),
                },
            )
            .unwrap();

        let events = capture.events_for_job("job-123");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_events_for_trace() {
        let capture = EventCapture::new();
        let logger = capture.logger();
        let ctx = TraceContext::new_root();

        logger
            .emit_with_context(
                &ctx,
                EventPayload::Custom {
                    event_type: "event1".to_string(),
                    data: serde_json::json!({}),
                },
            )
            .unwrap();

        logger
            .emit_with_context(
                &ctx,
                EventPayload::Custom {
                    event_type: "event2".to_string(),
                    data: serde_json::json!({}),
                },
            )
            .unwrap();

        let events = capture.events_for_trace(&ctx.trace_id);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_clear_events() {
        let capture = EventCapture::new();
        let logger = capture.logger();
        let ctx = TraceContext::new_root();

        logger
            .emit_with_context(
                &ctx,
                EventPayload::Custom {
                    event_type: "test".to_string(),
                    data: serde_json::json!({}),
                },
            )
            .unwrap();

        assert_eq!(capture.event_count(), 1);

        capture.clear();
        assert_eq!(capture.event_count(), 0);
    }
}
