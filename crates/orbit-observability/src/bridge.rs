//! Tracing-to-Audit bridge layer
//!
//! This module provides `AuditBridgeLayer` which implements the
//! `tracing_subscriber::Layer` trait to convert tracing events
//! into audit events.

use crate::context::TraceContext;
use crate::event::EventPayload;
use crate::logger::UnifiedLogger;
use std::sync::Arc;
use std::time::Instant;
use tracing::span::{Attributes, Id};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Bridge layer that converts tracing events to audit events
///
/// AuditBridgeLayer implements the `Layer` trait to intercept
/// tracing events and forward them to the UnifiedLogger.
///
/// ## Example
///
/// ```no_run
/// use orbit_observability::{AuditBridgeLayer, UnifiedLogger, AuditSigner};
/// use tracing_subscriber::prelude::*;
/// use std::path::Path;
///
/// let signer = AuditSigner::from_bytes(b"secret");
/// let logger = UnifiedLogger::new(Some(Path::new("audit.jsonl")), signer).unwrap();
/// let audit_layer = AuditBridgeLayer::new(logger);
///
/// tracing_subscriber::registry()
///     .with(audit_layer)
///     .init();
/// ```
pub struct AuditBridgeLayer {
    logger: Arc<UnifiedLogger>,
    emit_spans: bool,
}

impl AuditBridgeLayer {
    /// Create a new audit bridge layer
    pub fn new(logger: UnifiedLogger) -> Self {
        Self {
            logger: Arc::new(logger),
            emit_spans: false,
        }
    }

    /// Enable span event emission
    ///
    /// When enabled, the bridge will emit SpanStart and SpanEnd events
    /// in addition to regular tracing events.
    pub fn with_span_events(mut self, enabled: bool) -> Self {
        self.emit_spans = enabled;
        self
    }
}

/// Extension data stored in span extensions
struct SpanData {
    trace_ctx: TraceContext,
    start_time: Instant,
}

impl<S> Layer<S> for AuditBridgeLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found");

        // Check if parent span has a trace context
        let trace_ctx = if let Some(parent_span) = span.parent() {
            // Inherit from parent and create child span
            parent_span
                .extensions()
                .get::<SpanData>()
                .map(|data| data.trace_ctx.child_span())
                .unwrap_or_else(TraceContext::new_root)
        } else {
            // Create new root context
            TraceContext::new_root()
        };

        // Extract job_id and file_id from span fields if present
        let mut trace_ctx = trace_ctx;
        let mut visitor = FieldVisitor {
            job_id: None,
            file_id: None,
        };
        attrs.record(&mut visitor);

        if let Some(job_id) = visitor.job_id {
            trace_ctx = trace_ctx.with_job(job_id);
        }
        if let Some(file_id) = visitor.file_id {
            trace_ctx = trace_ctx.with_file_id(file_id);
        }

        // Store trace context in span extensions
        let span_data = SpanData {
            trace_ctx: trace_ctx.clone(),
            start_time: Instant::now(),
        };
        span.extensions_mut().insert(span_data);

        // Emit span start event if enabled
        if self.emit_spans {
            let metadata = attrs.metadata();
            let _ = self.logger.emit_with_context(
                &trace_ctx,
                EventPayload::SpanStart {
                    name: metadata.name().to_string(),
                    level: metadata.level().to_string(),
                },
            );
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // Get trace context from current span
        let trace_ctx = ctx
            .event_span(event)
            .and_then(|span| {
                span.extensions()
                    .get::<SpanData>()
                    .map(|d| d.trace_ctx.clone())
            })
            .unwrap_or_else(TraceContext::new_root);

        // Extract event fields
        let mut visitor = EventFieldVisitor::default();
        event.record(&mut visitor);

        // Create custom event payload
        let payload = EventPayload::Custom {
            event_type: event.metadata().name().to_string(),
            data: visitor.fields,
        };

        // Emit to audit logger
        let _ = self.logger.emit_with_context(&trace_ctx, payload);
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        if !self.emit_spans {
            return;
        }

        let span = ctx.span(&id).expect("Span not found");
        let extensions = span.extensions();

        if let Some(span_data) = extensions.get::<SpanData>() {
            let duration = span_data.start_time.elapsed();
            let metadata = span.metadata();

            let _ = self.logger.emit_with_context(
                &span_data.trace_ctx,
                EventPayload::SpanEnd {
                    name: metadata.name().to_string(),
                    duration_ms: duration.as_millis() as u64,
                },
            );
        }
    }
}

/// Visitor for extracting job_id and file_id from span fields
struct FieldVisitor {
    job_id: Option<String>,
    file_id: Option<String>,
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "job_id" => self.job_id = Some(format!("{:?}", value).trim_matches('"').to_string()),
            "file_id" => self.file_id = Some(format!("{:?}", value).trim_matches('"').to_string()),
            _ => {}
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "job_id" => self.job_id = Some(value.to_string()),
            "file_id" => self.file_id = Some(value.to_string()),
            _ => {}
        }
    }
}

/// Visitor for extracting all fields from events
struct EventFieldVisitor {
    fields: serde_json::Value,
}

impl EventFieldVisitor {
    fn insert(&mut self, key: &str, value: serde_json::Value) {
        if let serde_json::Value::Object(ref mut map) = self.fields {
            map.insert(key.to_string(), value);
        }
    }
}

impl Default for EventFieldVisitor {
    fn default() -> Self {
        Self {
            fields: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

impl tracing::field::Visit for EventFieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.insert(
            field.name(),
            serde_json::Value::String(format!("{:?}", value)),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.insert(field.name(), serde_json::Value::String(value.to_string()));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.insert(field.name(), serde_json::Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.insert(field.name(), serde_json::Value::Number(value.into()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.insert(field.name(), serde_json::Value::Bool(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if let Some(num) = serde_json::Number::from_f64(value) {
            self.insert(field.name(), serde_json::Value::Number(num));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signer::AuditSigner;
    use tempfile::NamedTempFile;
    use tracing_subscriber::prelude::*;

    #[test]
    fn test_bridge_layer_creation() {
        let signer = AuditSigner::from_bytes(b"test");
        let logger = UnifiedLogger::disabled();
        let _layer = AuditBridgeLayer::new(logger);
    }

    #[test]
    fn test_bridge_with_span_events() {
        let signer = AuditSigner::from_bytes(b"test");
        let logger = UnifiedLogger::disabled();
        let layer = AuditBridgeLayer::new(logger).with_span_events(true);
        assert!(layer.emit_spans);
    }

    #[test]
    fn test_event_emission() {
        let temp_file = NamedTempFile::new().unwrap();
        let signer = AuditSigner::from_bytes(b"test_secret");
        let logger = UnifiedLogger::new(Some(temp_file.path()), signer).unwrap();
        let layer = AuditBridgeLayer::new(logger.clone());

        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(test_field = "test_value", "Test event");
        });

        logger.flush().unwrap();
        let contents = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(contents.contains("\"type\":\"custom\""));
        assert!(contents.contains("test_value"));
    }

    #[test]
    fn test_span_events() {
        let temp_file = NamedTempFile::new().unwrap();
        let signer = AuditSigner::from_bytes(b"test_secret");
        let logger = UnifiedLogger::new(Some(temp_file.path()), signer).unwrap();
        let layer = AuditBridgeLayer::new(logger.clone()).with_span_events(true);

        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("test_span");
            let _enter = span.enter();
        });

        logger.flush().unwrap();
        let contents = std::fs::read_to_string(temp_file.path()).unwrap();
        let lines: Vec<&str> = contents.lines().collect();

        // Should have SpanStart and SpanEnd
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"type\":\"span_start\""));
        assert!(lines[1].contains("\"type\":\"span_end\""));
    }

    #[test]
    fn test_trace_context_propagation() {
        let temp_file = NamedTempFile::new().unwrap();
        let signer = AuditSigner::from_bytes(b"test_secret");
        let logger = UnifiedLogger::new(Some(temp_file.path()), signer).unwrap();
        let layer = AuditBridgeLayer::new(logger.clone());

        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("parent");
            let _enter = span.enter();

            tracing::info!("Parent event");

            {
                let child_span = tracing::info_span!("child");
                let _child_enter = child_span.enter();
                tracing::info!("Child event");
            }
        });

        logger.flush().unwrap();
        let contents = std::fs::read_to_string(temp_file.path()).unwrap();
        let lines: Vec<&str> = contents.lines().collect();

        let parent_event: OrbitEvent = serde_json::from_str(lines[0]).unwrap();
        let child_event: OrbitEvent = serde_json::from_str(lines[1]).unwrap();

        // Child should have same trace_id but different span_id
        assert_eq!(parent_event.trace_id, child_event.trace_id);
        assert_ne!(parent_event.span_id, child_event.span_id);
    }

    #[test]
    fn test_job_id_extraction() {
        let temp_file = NamedTempFile::new().unwrap();
        let signer = AuditSigner::from_bytes(b"test_secret");
        let logger = UnifiedLogger::new(Some(temp_file.path()), signer).unwrap();
        let layer = AuditBridgeLayer::new(logger.clone());

        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("transfer", job_id = "job-123");
            let _enter = span.enter();
            tracing::info!("Transfer started");
        });

        logger.flush().unwrap();
        let contents = std::fs::read_to_string(temp_file.path()).unwrap();
        let event: OrbitEvent = serde_json::from_str(contents.lines().next().unwrap()).unwrap();

        assert_eq!(event.job_id, Some("job-123".to_string()));
    }
}
