# ORBIT Wormhole Design Overview

## 1. Purpose
Wormhole extends the ORBIT project with a transport layer optimised for high-speed, resilient data transfer across unreliable or bandwidth-constrained links. It balances integrity, efficiency, and simplicity, providing a modular component for data replication, migration, or bulk transfer in both connected and disconnected environments.

---

## 2. Goals
- **Resilience:** Maintain data integrity and continuity over lossy UDP paths.
- **Efficiency:** Fully utilise available bandwidth with minimal retransmissions.
- **Auditability:** Provide cryptographic proof of delivery and integrity.
- **Simplicity:** Expose intuitive modes and minimal operational tuning.
- **Security:** Employ modern AEAD encryption and integrity mechanisms.

---

## 3. Architecture Summary

### Core Concepts
- **Transport Layer:** QUIC (UDP-based) with optional raw UDP for extreme conditions.
- **Chunking:** Content-defined chunking (CDC) with rolling hash for flexible boundaries.
- **Windows:** Sliding window of `N` data chunks + `K` parity chunks (RaptorQ-style FEC).
- **Integrity:** BLAKE3 per chunk, Merkle tree per window, AEAD per window.
- **Resume:** Manifest-based job tracking with resumable bitmap.
- **Audit:** JSON Lines log of all job events and verifications.

---

## 4. Transfer Modes
| Mode | Description | Use Case |
|------|--------------|-----------|
| **Stream** | QUIC reliable streams only. | Clean, high-quality links. |
| **Resilient** | QUIC + adaptive FEC with parity datagrams. | High-latency, lossy, or unpredictable paths. |
| **Extreme** | Raw UDP + fountain codes. | One-way or restricted network paths. |

---

## 5. Key Features and Differentiators
- **Merkle-Fountain Encoding:** Combines Merkle integrity with fountain-code recovery.
- **Adaptive Parity:** Dynamic tuning of `K` based on real-time path metrics.
- **Multipath Redundancy:** Distribute parity traffic across alternate links.
- **Dark-Site Courier Mode:** Write manifests and encrypted windows to media for partial offline transport.
- **Policy-Aware Encryption:** Optional redaction, per-window AEAD, and PQC-ready envelopes.
- **Audit Trail:** Tamper-evident event log for compliance and forensics.

---

## 6. Module Layout
```text
orbit/
  src/
    wormhole/
      mod.rs
      config.rs
      transport/
        quic.rs
        udp.rs
      fec/
        raptorq.rs
      chunk/
        cdc.rs
        fixed.rs
        merkle.rs
      crypto/
        aead.rs
      manifest/
        model.rs
        io.rs
      telemetry/
        reporter.rs
      cli.rs
      tests/
        fault_inject.rs
        golden_vectors.rs
```

---

## 7. Configuration (TOML Example)
```toml
[wormhole]
mode = "resilient"
target_bandwidth_mbps = 0 # auto
target_rtt_ms = 0 # auto
mtu = 0 # discover

[wormhole.window]
data_chunks = 64
min_parity = 4
max_parity = 16
adaptive = true

[wormhole.chunking]
type = "cdc"
avg_size_kib = 256

[wormhole.crypto]
aead = "aes256-gcm"
key_source = "env:ORBIT_KEY"

[wormhole.telemetry]
interval_ms = 250
jsonl_path = "/var/log/orbit/wormhole_audit.jsonl"
```

---

## 8. CLI Commands
```bash
orbit wormhole send --dest 10.0.0.2:8443 --path /data/inbox --mode resilient
orbit wormhole recv --bind 0.0.0.0:8443 --dest-root /data/landing --mode resilient
orbit wormhole probe --dest 10.0.0.2:8443
```

---

## 9. Adaptive Parity Algorithm
```rust
fn tune_k(history: &PathStats, k: usize, min_k: usize, max_k: usize) -> usize {
    if history.loss_pct > 2.0 || history.rto_p95_ms > 250 { (k+2).min(max_k) }
    else if history.loss_pct < 0.2 && history.rtt_p50_ms.stable() { k.saturating_sub(1).max(min_k) }
    else { k }
}
```

---

## 10. Telemetry and Observability
Metrics include:
- Goodput, repair rate, parity overhead.
- RTT, reorder %, and loss %.
- Time to integrity-confirmed window.
- Resume efficiency and policy compliance.

Audit lines are appended as JSON Lines for easy ingestion into SIEM or logging systems.

---

## 11. Staged Delivery Plan
### MVP (Weeks 1–4)
- QUIC-based transport wrapper.
- Fixed chunking.
- Sliding window with static FEC (RaptorQ).
- AEAD encryption and manifest resume.
- JSONL audit.

### Phase 2
- CDC chunking.
- Adaptive parity and telemetry feedback.
- Bandwidth self-tuning and Prometheus export.
- Courier mode (offline/online hybrid).

### Phase 3
- Raw UDP extreme mode.
- Relay service and store-forward cache.
- Policy-aware redaction and PQC envelopes.

---

## 12. Data Structures
```rust
pub struct Chunk { id: u64, offset: u64, len: u32, hash: [u8; 32], data: Bytes }

pub struct Window { id: u64, chunks: Vec<ChunkMeta>, merkle_root: [u8; 32], parity: Vec<FecSymbol> }

pub struct PathStats { rtt_ms: f64, loss_pct: f64, reorder_pct: f64, goodput_mbps: f64 }

pub struct AuditSummary { job_id: String, files: usize, bytes_total: u64, bytes_parity: u64, window_ok: u64, window_fail: u64 }
```

---

## 13. Test and Validation
- Fault injector with loss, jitter, reordering, and MTU changes.
- Deterministic seeds for reproducibility.
- Golden vectors for Merkle roots and AEAD integrity.

---

## 14. Operator Defaults
| Setting | Default | Notes |
|----------|----------|-------|
| Mode | Resilient | Balances performance and protection. |
| Chunks | 64 | Reasonable per window baseline. |
| Min Parity | 4 | Start point for loss < 1%. |
| Max Parity | 12 | For >5% loss. |
| CDC Avg Size | 256 KiB | Balanced chunking. |
| Telemetry Interval | 250 ms | Smooth feedback. |
| Resume | Enabled | Recommended always. |

---

## 15. Documentation Checklist
- **Quick Start:** Explain the three modes clearly.
- **Hardening Guide:** Cover encryption, audit handling, and key rotation.
- **Runbook:** Probe, choose bandwidth, monitor repair rate, validate audit digest.

---

## 16. Summary
Wormhole elevates ORBIT from a file mover to a **self-healing data transport framework** capable of:
- Surviving lossy networks with forward-error correction.
- Providing verifiable end-to-end data integrity.
- Delivering adaptive performance tuned by live network feedback.
- Operating seamlessly across both online and partially disconnected environments.

This foundation allows ORBIT to serve high-assurance domains, dark sites, and high-performance networks equally well, embodying the guiding principle: **"Stream fast when clean, heal when rough."**

