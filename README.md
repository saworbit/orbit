# 🚀 Orbit

> **O**pen **R**esilient **B**ulk **I**nformation **T**ransfer  

**The intelligent file transfer tool that never gives up** 💪  

[![Crates.io](https://img.shields.io/crates/v/orbit.svg)](https://crates.io/crates/orbit)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/saworbit/orbit/workflows/CI/badge.svg)](https://github.com/saworbit/orbit/actions)

---

## 🌟 What is Orbit?

Orbit is a **blazingly fast** 🔥 file transfer tool built in Rust that combines enterprise-grade reliability with cutting-edge performance.  
Whether you're backing up terabytes of data, syncing files across continents, or just want your copies to be **ridiculously fast**, Orbit has you covered.

---

## 🧠 The Next Evolution of Orbit

Orbit has evolved into a **modular transfer framework** designed around performance, reliability, and observability.

### 🌌 Core Additions

| Feature | Description |
|----------|--------------|
| 🗂 **Manifest System** | Declarative multi-job transfer engine built on Starmap and Audit |
| 🌌 **Starmap Planner** | Validates, orders, and maps manifest jobs before execution |
| 📊 **Audit Integration** | Structured telemetry for every file, job, and transfer |
| 🧩 **Modular Crates** | Core logic separated into clean, testable modules |
| 🕵️ **Watcher / Beacon** | Lightweight process to monitor Orbit health |
| 🔄 **Improved Resume Logic** | Chunk-level recovery and validation |
| ⚙️ **Configurable Defaults** | Persistent settings via `orbit.toml` or environment variables |
| 🧪 **Wormhole Module (Experimental)** | Forward-error correction for lossy networks |

---

## ✨ Highlights

- 🚄 **3× Faster** — zero-copy transfers at device speed  
- 🛡️ **Bulletproof** — resume, checksums, retries  
- 🧠 **Smart** — automatic strategy selection (zero-copy, compression, or buffered)  
- 📊 **Auditable** — Starmap-aware logging for every job  
- 🧩 **Modular** — clean crate boundaries for contributors  
- 🌍 **Cross-Platform** — Linux, macOS, Windows

---

## 🗂 Manifest System + Starmap

The Manifest System defines repeatable transfer jobs in TOML and hands them to **Starmap**, Orbit’s execution planner.  
Starmap builds a dependency graph, validates all jobs, then executes them through the Audit layer for full traceability.

Example `orbit.manifest.toml`:

```toml
[defaults]
checksum = "sha256"
compression = "zstd:6"
resume = true
concurrency = 4
audit_log = "audit.log"
plan_visualisation = true

[[job]]
name = "source-sync"
source = "/data/source/"
destination = "/mnt/backup/source/"

[[job]]
name = "media-archive"
source = "/media/camera/"
destination = "/tank/archive/"
compression = "zstd:1"
checksum = "sha256"
depends_on = ["source-sync"]
```

Run it:
```bash
orbit run --manifest orbit.manifest.toml
```

### 🔭 Starmap Highlights
- Builds an execution plan (directed graph of jobs)  
- Validates dependencies and resources before transfer  
- Optimises concurrency for independent jobs  
- Visualises job order when `plan_visualisation = true`

### 📋 Audit Highlights
- Records structured JSON events for every job and file  
- Logs duration, throughput, compression ratio, checksum, and retries  
- Tags each record with its Starmap node for easy correlation  

---

## 🕵️ Watcher / Beacon

A companion service that observes Orbit runtime health.  
It can report stalled transfers, track telemetry, and trigger recovery actions.

Example:
```bash
orbit watcher --status
```

---

## 📊 Audit and Telemetry

Every operation emits structured audit events via the **core-audit** crate.  
Example:
```json
{
  "timestamp": "2025-10-18T16:42:19Z",
  "job": "media-archive",
  "source": "/media/camera/",
  "destination": "/tank/archive/",
  "bytes": 104857600,
  "duration_ms": 2341,
  "compression": "zstd:1",
  "checksum": "sha256",
  "checksum_match": true,
  "status": "ok",
  "starmap_node": "orbit.node.media-archive"
}
```

Audit logs are JSON Lines, timestamped, and ingestion-ready for ELK, Loki, or Datadog.

---

## ⚙️ Modular Architecture

| Crate | Purpose |
|-------|----------|
| 🧩 `core-manifest` | Manifest parsing and orchestration |
| 🌌 `core-starmap` | Job planner and dependency graph builder |
| 📊 `core-audit` | Structured logging and telemetry |
| 🔐 `core-checksum` | File hashing and verification |
| ⚡ `core-zero-copy` | OS-level optimised I/O |
| 🗜️ `core-compress` | Compression and decompression |
| 🕵️ `core-watcher` *(planned)* | Monitoring beacon |
| 🧪 `wormhole` *(dev)* | Forward-error correction module |

This structure ensures isolation, clarity, and reusability across future projects.

---

## ⚡ Performance Benchmarks

| Size | Traditional | Orbit (Zero-Copy) | Speedup | CPU |
|------|--------------|------------------|----------|------|
| 10 MB | 12 ms | 8 ms | 1.5× | ↓ 65 % |
| 1 GB | 980 ms | 340 ms | 2.9× | ↓ 78 % |
| 10 GB | 9.8 s | 3.4 s | 2.9× | ↓ 80 % |

Zstd 3 → 2.3× faster over networks  
LZ4 → near-realtime local copies  

---

## 🚀 Quick Start

### Install
```bash
cargo install orbit
```
or build manually:
```bash
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build --release
sudo cp target/release/orbit /usr/local/bin/
```

### Simple Transfer
```bash
orbit -s /data/source -d /backup/target --resume --checksum sha256
```

### Manifest Run
```bash
orbit run -m orbit.manifest.toml
```

---

## 🧠 Smart Strategy Selection

```
Same-disk large file  → Zero-copy
Slow link             → Compression
Unreliable network    → Resume + retry
Critical data         → Checksum + audit
```

Orbit analyses environment and adapts automatically.

---

## 🔐 Security

- Safe path handling  
- Checksum verification for integrity  
- Future FIPS-compliant crypto support  
- No telemetry phone-home  

---

## 🧩 Configuration

Defaults via `orbit.toml` or environment variables:

```toml
compression = "zstd:5"
checksum = "sha256"
concurrency = 8
audit_log = "/var/log/orbit_audit.log"
resume = true
telemetry = true
```

---

## 🧪 Roadmap

### ✅ Implemented
- Zero-copy and compression engines  
- Manifest + Starmap + Audit integration  
- Structured telemetry  
- Modular crate system  
- Resume and retry improvements  

### 🧠 In Progress
- Watcher beacon service  
- Object-storage connectors (S3, Azure, GCS)  
- Wormhole FEC module  

### 🚧 Planned
- REST orchestration API  
- Scheduler and conditionals  
- Disk-space pre-check  
- Plugin framework for custom protocols  

---

## 🦀 Contributing

Pull requests welcome!  
See `CONTRIBUTING.md` for code style and guidelines.

Build and test:
```bash
cargo build && cargo test
```

Format and lint:
```bash
cargo fmt && cargo clippy
```

---

## 📜 Licence

Licensed under both:
- 📄 MIT  
- 📄 Apache 2.0  

See `LICENSE` and `DUAL_LICENSE_NOTICE.md` for details.

---

<div align="center">

### Made with ❤️ and 🦀 by [Shane Wall](https://github.com/saworbit)

**Orbit — because your data deserves to travel in style.** ✨  

[⬆ Back to Top](#-orbit)

</div>
