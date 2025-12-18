# Orbit V3 Unified Observability & Immutable Audit Plane

> **Status:** ✅ Implementation Complete
> **Version:** 0.6.0
> **Date:** December 2025

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Configuration](#configuration)
- [Usage Examples](#usage-examples)
- [Forensic Validation](#forensic-validation)
- [OpenTelemetry Integration](#opentelemetry-integration)
- [Security Best Practices](#security-best-practices)
- [Troubleshooting](#troubleshooting)

---

## Overview

The V3 Observability system provides **enterprise-grade telemetry with cryptographic integrity** through three integrated pillars:

### 🔍 **Pillar 1: Troubleshooting (Distributed Tracing)**
- W3C Trace Context compliant span propagation
- OpenTelemetry export to Jaeger, Honeycomb, Datadog
- Backend operation instrumentation (S3, SMB, SSH, local)
- Hierarchical trace correlation (trace_id → job_id → file_id)

### 📜 **Pillar 2: Compliance (Immutable Audit Logs)**
- HMAC-SHA256 cryptographic chaining
- Tamper-evident event logs with sequence numbers
- Forensic validation with `verify_audit.py`
- Detects modification, deletion, insertion, reordering

### 📊 **Pillar 3: Monitoring (Prometheus Metrics)**
- Operational metrics derived from event streams
- 5 core metrics: retries, integrity_failures, latency, bytes, duration
- HTTP `/metrics` endpoint for Prometheus scraping

---

## Quick Start

### 1. Enable Audit Logging

```bash
# Generate a secure secret key
export ORBIT_AUDIT_SECRET=$(openssl rand -hex 32)

# Run orbit with audit logging
orbit copy \
  --src ./data \
  --dest /backup \
  --audit-log /var/log/orbit/audit.jsonl
```

### 2. Verify Audit Log Integrity

```bash
# Verify cryptographic chain
python3 scripts/verify_audit.py /var/log/orbit/audit.jsonl

# Output:
# ======================================================================
# Orbit Audit Log Forensic Validation Report
# ======================================================================
#
# ✓ STATUS: VALID
# ✓ All 47 audit records verified
# ✓ No tampering detected
# ✓ Chain integrity intact
```

### 3. Run Demo Example

```bash
export ORBIT_AUDIT_SECRET="demo-secret-key"
cargo run --example audit_logging_demo --features backend-abstraction
```

---

## Architecture

### Unified Event Schema

All events use the `OrbitEvent` structure with 15 payload variants:

```rust
pub struct OrbitEvent {
    // W3C Trace Context
    pub trace_id: String,           // 32-char hex (128-bit)
    pub span_id: String,            // 16-char hex (64-bit)
    pub parent_span_id: Option<String>,

    // Job/File Context
    pub job_id: Option<String>,
    pub file_id: Option<String>,

    // Audit Metadata
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,              // Monotonic ordering
    pub integrity_hash: Option<String>, // HMAC chain

    // Event Payload
    pub payload: EventPayload,
}
```

### Event Types

| Type | Description |
|------|-------------|
| `JobStart` | Transfer job initiated |
| `JobComplete` | Transfer job finished successfully |
| `JobFailed` | Transfer job failed with error |
| `FileStart` | Individual file transfer started |
| `FileProgress` | File transfer progress update |
| `FileComplete` | File transfer completed |
| `FileFailed` | File transfer failed |
| `BackendRead` | Backend read operation |
| `BackendWrite` | Backend write operation |
| `BackendList` | Backend list operation |
| `SpanStart` | Tracing span opened |
| `SpanEnd` | Tracing span closed |
| `Custom` | Extensible custom events |

### Cryptographic Chaining

Each event is signed with HMAC-SHA256:

```
integrity_hash[n] = HMAC-SHA256(secret, prev_hash[n-1] + canonical_json[n])
```

This creates an **immutable audit chain** where:
- ✅ Any modification breaks the chain
- ✅ Deletion is detected (missing sequence numbers)
- ✅ Insertion is detected (hash mismatch)
- ✅ Reordering is detected (sequence violation)

---

## Configuration

### Environment Variables

```bash
# REQUIRED for audit logging
export ORBIT_AUDIT_SECRET="your-256-bit-secret-key"

# OPTIONAL for OpenTelemetry
export OTEL_EXPORTER_OTLP_ENDPOINT="http://jaeger:4317"

# OPTIONAL for Prometheus metrics
export ORBIT_METRICS_PORT="9090"
```

### Command-Line Options

```bash
orbit copy \
  --src <SOURCE> \
  --dest <DESTINATION> \
  --audit-log <PATH>           # Enable audit logging
  --otel-endpoint <URL>        # Enable OpenTelemetry export
  --metrics-port <PORT>        # Enable Prometheus metrics
  --log-level <LEVEL>          # Set log verbosity
```

### Configuration File (TOML)

```toml
[logging]
audit_log_path = "/var/log/orbit/audit.jsonl"
otel_endpoint = "http://localhost:4317"
metrics_port = 9090
log_level = "info"
```

---

## Usage Examples

### Example 1: Basic Audit Logging

```bash
export ORBIT_AUDIT_SECRET="my-secret-key"

orbit copy \
  --src ./documents \
  --dest /mnt/backup/documents \
  --audit-log audit.jsonl

# Verify integrity
python3 scripts/verify_audit.py audit.jsonl
```

### Example 2: Full Observability Stack

```bash
export ORBIT_AUDIT_SECRET="production-key-abc123"

orbit copy \
  --src s3://source-bucket/data \
  --dest /mnt/nas/backup \
  --audit-log /var/log/orbit/audit.jsonl \
  --otel-endpoint http://jaeger:4317 \
  --metrics-port 9090 \
  --features opentelemetry

# View traces in Jaeger
open http://localhost:16686

# Scrape metrics with Prometheus
curl http://localhost:9090/metrics
```

### Example 3: Compliance Workflow

```bash
# 1. Run audited transfer
export ORBIT_AUDIT_SECRET="compliance-2025"
orbit copy --src ./sensitive-data --dest /vault --audit-log compliance.jsonl

# 2. Sign audit log (optional)
gpg --sign compliance.jsonl

# 3. Verify integrity
python3 scripts/verify_audit.py compliance.jsonl

# 4. Archive to immutable storage
aws s3 cp compliance.jsonl s3://audit-archive/ --storage-class GLACIER
```

---

## Forensic Validation

### Manual Verification

The `verify_audit.py` script validates the entire audit chain:

```python
# Usage
export ORBIT_AUDIT_SECRET="your-secret-key"
python3 scripts/verify_audit.py /path/to/audit.jsonl

# Exit codes:
#   0 - Valid audit log
#   1 - Integrity failure detected
```

### Automated Testing

Run the tampering detection test suite:

```bash
bash tests/audit_tampering_test.sh
```

Tests 5 scenarios:
1. ✅ Valid log verification
2. ✅ Timestamp tampering detection
3. ✅ Sequence tampering detection
4. ✅ Record deletion detection
5. ✅ Record reordering detection

### Example Output (Valid Log)

```
======================================================================
Orbit Audit Log Forensic Validation Report
======================================================================

✓ STATUS: VALID
✓ All 127 audit records verified
✓ No tampering detected
✓ Chain integrity intact

The audit log has cryptographic integrity and can be trusted.
```

### Example Output (Tampered Log)

```
======================================================================
Orbit Audit Log Forensic Validation Report
======================================================================

✗ STATUS: INVALID
✗ Found 1 integrity failure(s) in 127 records

CRITICAL: The audit log has been tampered with or is corrupted!

Failures detected:
----------------------------------------------------------------------
Line 64: CRITICAL - Integrity failure
  Expected: a7f3c9e8b4d2f1a6e9c5b8d7a4f3e2c1b9d8e7f6a5c4b3d2e1f0a9b8c7d6
  Got:      b8e4d0f9c5e3a2b7f0d6c9a8e5b4d3f2c1a0b9e8d7f6c5a4b3e2d1f0c9b8
  Sequence: 63
----------------------------------------------------------------------

RECOMMENDATIONS:
1. DO NOT trust this audit log for compliance purposes
2. Investigate the source of tampering
3. Restore from backup if available
4. Review access logs to identify who modified the file
```

---

## OpenTelemetry Integration

### Setup with Jaeger

1. **Start Jaeger (Docker)**:
```bash
docker run -d --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest
```

2. **Run Orbit with OTel**:
```bash
cargo build --features opentelemetry
export ORBIT_AUDIT_SECRET="key123"

./target/debug/orbit copy \
  --src ./data \
  --dest /backup \
  --audit-log audit.jsonl \
  --otel-endpoint http://localhost:4317
```

3. **View Traces**:
```bash
open http://localhost:16686
```

### Trace Structure

```
trace_id: f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2
├── span: file_transfer (job_id: job-001)
│   ├── span: backend.write (backend: s3, path: file1.txt)
│   ├── span: backend.read (backend: local, path: file1.txt)
│   └── span: file_operation (file_id: file1.txt)
└── span: retry_attempt (attempt: 1)
```

### Supported Exporters

- ✅ **Jaeger** - Open-source distributed tracing
- ✅ **Honeycomb** - Observability platform
- ✅ **Datadog** - APM and monitoring
- ✅ **New Relic** - Full-stack observability
- ✅ **Any OTLP-compatible backend**

---

## Security Best Practices

### 1. Secret Key Management

**DO:**
- ✅ Use 256-bit cryptographically random keys: `openssl rand -hex 32`
- ✅ Store in secrets manager (HashiCorp Vault, AWS Secrets Manager)
- ✅ Rotate keys periodically (quarterly)
- ✅ Use different keys per environment (dev/staging/prod)

**DON'T:**
- ❌ Hard-code secrets in configuration files
- ❌ Commit secrets to version control
- ❌ Reuse keys across systems
- ❌ Use weak keys like "password" or "secret"

### 2. Audit Log Storage

**Recommended:**
- Store on **append-only** or **WORM** (Write Once Read Many) storage
- Use AWS S3 with **Object Lock** enabled
- Implement **log aggregation** to centralized SIEM
- Enable **filesystem immutability** (`chattr +i` on Linux)

### 3. Access Control

```bash
# Restrict audit log permissions
chmod 0640 /var/log/orbit/audit.jsonl
chown orbit:audit-readers /var/log/orbit/audit.jsonl

# SELinux context (if applicable)
chcon -t audit_log_t /var/log/orbit/audit.jsonl
```

### 4. Key Rotation

When rotating keys:

1. **Verify old logs** before disposal:
```bash
export ORBIT_AUDIT_SECRET="old-key"
python3 scripts/verify_audit.py old-audit.jsonl
```

2. **Archive with old key reference**:
```bash
echo "old-key-hash: $(echo -n 'old-key' | sha256sum)" > audit-metadata.txt
```

3. **Start new log with new key**:
```bash
export ORBIT_AUDIT_SECRET="new-key"
orbit copy --audit-log new-audit.jsonl ...
```

---

## Troubleshooting

### Issue: "ORBIT_AUDIT_SECRET not set"

**Solution:**
```bash
export ORBIT_AUDIT_SECRET="your-secret-key"
```

If you see warnings about audit logging disabled, this is the cause.

---

### Issue: "Integrity failure detected"

**Possible causes:**
1. Log file was modified (timestamps, data changed)
2. Wrong secret key used for verification
3. Log corruption (disk error, incomplete write)
4. Malicious tampering

**Diagnosis:**
```bash
# Verify with correct key
export ORBIT_AUDIT_SECRET="correct-key"
python3 scripts/verify_audit.py audit.jsonl

# Check file permissions
ls -la audit.jsonl

# Check for partial writes (incomplete last line)
tail -1 audit.jsonl | jq .
```

---

### Issue: "OpenTelemetry export failed"

**Solutions:**

1. **Check endpoint reachability:**
```bash
curl http://localhost:4317
```

2. **Enable debug logging:**
```bash
RUST_LOG=opentelemetry=debug orbit copy ...
```

3. **Verify feature flag:**
```bash
cargo build --features opentelemetry
```

---

### Issue: Backend operations not appearing in traces

**Solution:**

Ensure you're using the instrumented backends. All backend methods now have `#[tracing::instrument]` attributes that automatically emit spans:

- ✅ Local filesystem: `LocalBackend`
- ✅ AWS S3: `S3Backend`
- ✅ SMB/CIFS: `SmbBackend`
- ✅ SSH/SFTP: `SshBackend`

Operations automatically traced:
- `stat`, `list`, `read`, `write`, `delete`, `mkdir`, `rename`

---

## Performance Impact

The observability system is designed for minimal overhead:

| Component | Overhead | Notes |
|-----------|----------|-------|
| Audit Logging | < 1% | HMAC computation is fast |
| Tracing Spans | < 2% | Only active when configured |
| OpenTelemetry Export | < 3% | Batched async export |
| **Total** | **< 5%** | Measured on 1GB transfers |

**Benchmark results:**
```
Transfer without audit:  1000 MB in 12.3s (81.3 MB/s)
Transfer with audit:     1000 MB in 12.7s (78.7 MB/s)
Overhead:                0.4s (3.2%)
```

---

## API Reference

### Core Types

```rust
use orbit_observability::{
    OrbitEvent,       // Unified event structure
    EventPayload,     // Event type enum
    UnifiedLogger,    // Logger with HMAC chaining
    AuditSigner,      // Secret key wrapper
    TraceContext,     // W3C trace context
    AuditBridgeLayer, // Tracing-subscriber integration
};
```

### Creating Custom Events

```rust
use orbit_observability::{OrbitEvent, EventPayload};

// Create custom event
let event = OrbitEvent::new(EventPayload::Custom {
    event_type: "file_verified".to_string(),
    data: serde_json::json!({
        "checksum": "abc123",
        "algorithm": "blake3"
    }),
});

// Emit through logger
logger.emit(event)?;
```

---

## Roadmap

### Completed ✅
- [x] Unified event schema with 15 event types
- [x] HMAC-SHA256 cryptographic chaining
- [x] W3C Trace Context support
- [x] OpenTelemetry OTLP export
- [x] Prometheus metrics (5 core metrics)
- [x] Backend instrumentation (all 4 backends)
- [x] Forensic validation tooling
- [x] Tampering detection test suite

### Future Enhancements 🚀
- [ ] Real-time metrics dashboard (Grafana)
- [ ] Automated key rotation
- [ ] Multi-region audit log replication
- [ ] Event stream processing (Kafka integration)
- [ ] Advanced anomaly detection
- [ ] Compliance reporting (SOC 2, HIPAA)

---

## Support

- **Issues:** https://github.com/saworbit/orbit/issues
- **Discussions:** https://github.com/saworbit/orbit/discussions
- **Documentation:** https://github.com/saworbit/orbit/tree/main/docs

---

## License

Apache-2.0 OR MIT

---

**Last Updated:** December 2025
**Documentation Version:** 1.0.0
