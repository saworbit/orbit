//! Prometheus metrics derived from orbit events
//!
//! This module provides Prometheus metrics for monitoring Orbit operations.
//! Metrics are derived from the audit event stream.

use prometheus::{Counter, CounterVec, Histogram, HistogramOpts, HistogramVec, Opts, Registry};
use std::sync::OnceLock;

/// Global Prometheus registry for Orbit metrics
pub static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Get or initialize the global registry
pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let r = Registry::new();
        register_metrics(&r);
        r
    })
}

/// Transfer retry counter
///
/// Incremented each time a transfer operation is retried.
/// Labels: protocol (s3, smb, ssh, local)
pub static TRANSFER_RETRIES_TOTAL: OnceLock<CounterVec> = OnceLock::new();

/// Audit integrity failure counter
///
/// Incremented when audit chain validation fails.
/// This is a CRITICAL metric that should trigger alerts.
pub static AUDIT_INTEGRITY_FAILURES: OnceLock<Counter> = OnceLock::new();

/// Backend operation latency histogram
///
/// Records latency of backend operations (read, write, list).
/// Labels: backend (s3, smb, ssh, local), operation (read, write, list)
/// Buckets: 10ms, 50ms, 100ms, 500ms, 1s, 5s, 10s
pub static BACKEND_LATENCY_SECONDS: OnceLock<HistogramVec> = OnceLock::new();

/// Transfer bytes total counter
///
/// Total bytes transferred across all operations.
/// Labels: protocol, status (success, failure)
pub static TRANSFER_BYTES_TOTAL: OnceLock<CounterVec> = OnceLock::new();

/// Job duration histogram
///
/// Records job execution time.
/// Labels: protocol, status (success, failure)
/// Buckets: 1s, 10s, 30s, 1m, 5m, 15m, 30m, 1h
pub static JOB_DURATION_SECONDS: OnceLock<HistogramVec> = OnceLock::new();

/// Register all metrics with the registry
fn register_metrics(registry: &Registry) {
    // Transfer retries
    let retries = CounterVec::new(
        Opts::new(
            "orbit_transfer_retries_total",
            "Total number of transfer retry attempts",
        ),
        &["protocol"],
    )
    .expect("Failed to create transfer_retries metric");
    registry
        .register(Box::new(retries.clone()))
        .expect("Failed to register transfer_retries");
    TRANSFER_RETRIES_TOTAL.set(retries).ok();

    // Audit integrity failures
    let integrity = Counter::with_opts(Opts::new(
        "orbit_audit_integrity_failures_total",
        "Number of audit chain integrity failures (CRITICAL)",
    ))
    .expect("Failed to create audit_integrity_failures metric");
    registry
        .register(Box::new(integrity.clone()))
        .expect("Failed to register audit_integrity_failures");
    AUDIT_INTEGRITY_FAILURES.set(integrity).ok();

    // Backend latency
    let latency = HistogramVec::new(
        HistogramOpts::new("orbit_backend_latency_seconds", "Backend operation latency")
            .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
        &["backend", "operation"],
    )
    .expect("Failed to create backend_latency metric");
    registry
        .register(Box::new(latency.clone()))
        .expect("Failed to register backend_latency");
    BACKEND_LATENCY_SECONDS.set(latency).ok();

    // Transfer bytes
    let bytes = CounterVec::new(
        Opts::new(
            "orbit_transfer_bytes_total",
            "Total bytes transferred across all operations",
        ),
        &["protocol", "status"],
    )
    .expect("Failed to create transfer_bytes metric");
    registry
        .register(Box::new(bytes.clone()))
        .expect("Failed to register transfer_bytes");
    TRANSFER_BYTES_TOTAL.set(bytes).ok();

    // Job duration
    let duration = HistogramVec::new(
        HistogramOpts::new("orbit_job_duration_seconds", "Job execution time")
            .buckets(vec![1.0, 10.0, 30.0, 60.0, 300.0, 900.0, 1800.0, 3600.0]),
        &["protocol", "status"],
    )
    .expect("Failed to create job_duration metric");
    registry
        .register(Box::new(duration.clone()))
        .expect("Failed to register job_duration");
    JOB_DURATION_SECONDS.set(duration).ok();
}

/// Increment transfer retry counter
pub fn inc_transfer_retry(protocol: &str) {
    if let Some(counter) = TRANSFER_RETRIES_TOTAL.get() {
        counter.with_label_values(&[protocol]).inc();
    }
}

/// Increment audit integrity failure counter
pub fn inc_audit_integrity_failure() {
    if let Some(counter) = AUDIT_INTEGRITY_FAILURES.get() {
        counter.inc();
    }
}

/// Record backend operation latency
pub fn record_backend_latency(backend: &str, operation: &str, duration_secs: f64) {
    if let Some(histogram) = BACKEND_LATENCY_SECONDS.get() {
        histogram
            .with_label_values(&[backend, operation])
            .observe(duration_secs);
    }
}

/// Increment transfer bytes counter
pub fn inc_transfer_bytes(protocol: &str, status: &str, bytes: u64) {
    if let Some(counter) = TRANSFER_BYTES_TOTAL.get() {
        counter
            .with_label_values(&[protocol, status])
            .inc_by(bytes as f64);
    }
}

/// Record job duration
pub fn record_job_duration(protocol: &str, status: &str, duration_secs: f64) {
    if let Some(histogram) = JOB_DURATION_SECONDS.get() {
        histogram
            .with_label_values(&[protocol, status])
            .observe(duration_secs);
    }
}

/// Get metrics in Prometheus text format
pub fn metrics_text() -> String {
    use prometheus::{Encoder, TextEncoder};

    let encoder = TextEncoder::new();
    let metric_families = registry().gather();
    let mut buffer = Vec::new();

    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let reg = registry();
        let metrics = reg.gather();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_inc_transfer_retry() {
        inc_transfer_retry("s3");
        inc_transfer_retry("s3");
        inc_transfer_retry("smb");

        let metrics_output = metrics_text();
        assert!(metrics_output.contains("orbit_transfer_retries_total"));
    }

    #[test]
    fn test_inc_audit_integrity_failure() {
        inc_audit_integrity_failure();

        let metrics_output = metrics_text();
        assert!(metrics_output.contains("orbit_audit_integrity_failures_total"));
    }

    #[test]
    fn test_record_backend_latency() {
        record_backend_latency("s3", "read", 0.05);
        record_backend_latency("s3", "write", 0.15);

        let metrics_output = metrics_text();
        assert!(metrics_output.contains("orbit_backend_latency_seconds"));
    }

    #[test]
    fn test_inc_transfer_bytes() {
        inc_transfer_bytes("s3", "success", 1024);
        inc_transfer_bytes("s3", "success", 2048);

        let metrics_output = metrics_text();
        assert!(metrics_output.contains("orbit_transfer_bytes_total"));
    }

    #[test]
    fn test_record_job_duration() {
        record_job_duration("s3", "success", 45.5);
        record_job_duration("smb", "failure", 10.2);

        let metrics_output = metrics_text();
        assert!(metrics_output.contains("orbit_job_duration_seconds"));
    }

    #[test]
    fn test_metrics_text_format() {
        inc_transfer_retry("local");

        let output = metrics_text();
        assert!(output.starts_with("# HELP") || output.contains("orbit_"));
    }
}
