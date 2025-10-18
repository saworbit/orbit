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

Orbit has grown beyond a single binary copy utility.  
It is now a **modular, intelligent transfer framework** built around performance, reliability, and observability.

### 🌌 Core Additions

| Feature | Description |
|----------|--------------|
| 🗂 **Manifest System** | Define multi-job transfers in TOML and run declaratively |
| 🧩 **Modular Crates** | Core logic split into focused Rust crates for reuse and clarity |
| 🕵️ **Watcher / Beacon** | Lightweight monitor that checks Orbit’s health and uptime |
| 📊 **Enhanced Audit Logs** | JSON Lines with throughput, duration, compression ratio, and checksum status |
| 🧠 **Telemetry Engine** | Structured runtime metrics for dashboards or automation |
| 🔄 **Improved Resume Logic** | Chunk-level resume tracking and alerting |
| ⚙️ **Configurable Defaults** | Global options via `orbit.toml` or environment variables |
| 🧪 **Wormhole Module (Experimental)** | Forward-error correction and erasure coding for lossy networks |

---

## ✨ Highlights

- 🚄 **3× Faster** — zero-copy transfers at device speed  
- 🛡️ **Bulletproof** — resume, checksums, retries  
- 🧠 **Smart** — automatic strategy selection (zero-copy, compression, or buffered)  
- 📊 **Auditable** — detailed per-file logs  
- 🧩 **Modular** — clean crate boundaries for contributors  
- 🌍 **Cross-Platform** — Linux, macOS, Windows

---

## 🗂 Manifest System

Orbit can run complex transfer jobs from a single manifest file.  
Perfect for backups, migrations, or repeatable batch jobs.

Example `orbit.manifest.toml`:

```toml
[defaults]
checksum = "sha256"
compression = "zstd:6"
resume = true
concurrency = 4
audit_log = "audit.log"

[[job]]
name = "project-backup"
source = "/data/projects/"
destination = "/mnt/backup/projects/"
include = ["**/*.rs", "**/*.toml"]
exclude = ["target/**", ".git/**"]

[[job]]
name = "media-archive"
source = "/media/camera/"
destination = "/tank/archive/"
compression = "zstd:1"
checksum = "sha256"
resume = true
```

Run it:
```bash
orbit run --manifest orbit.manifest.toml
```

✅ Multiple jobs per file  
✅ Per-job overrides  
✅ Glob include/exclude  
✅ Validation and audit integration  

---

## 🕵️ Watcher / Beacon

A small companion process that keeps an eye on Orbit jobs.  

- Monitors process status and log freshness  
- Emits JSON telemetry for monitoring tools  
- Can alert, restart, or record incidents  
- Designed to evolve into a self-healing component  

Example:
```bash
orbit watcher --status
```

---

## 📊 Audit and Telemetry

Orbit produces structured audit entries like:
```json
{"job":"project-backup","file":"src/main.rs","bytes":1048576,"status":"ok","checksum_match":true,"duration_ms":1184,"compression":"zstd:6"}
```

**Now includes:**
- Duration and throughput  
- Compression ratio  
- Retry count and resume state  
- Resource snapshots  

Audit files are line-delimited JSON and ingestible by ELK, Loki, Datadog, etc.

---

## ⚙️ Modular Architecture

| Crate | Purpose |
|-------|----------|
| 🧩 `core-manifest` | Manifest parsing and orchestration |
| 🧩 `core-audit` | Structured telemetry and logging |
| 🧩 `core-checksum` | File hashing and verification |
| 🧩 `core-zero-copy` | OS-level optimised copy paths |
| 🧩 `core-compress` | Compression and decompression logic |
| 🧩 `core-watcher` *(planned)* | Beacon monitoring service |
| 🧩 `wormhole` *(dev)* | Forward-error correction for harsh networks |

This structure improves reliability and allows external crates to reuse modules independently.

---

## ⚡ Performance Benchmarks

| Size | Traditional | Orbit (Zero-Copy) | Speedup | CPU |
|------|--------------|------------------|----------|------|
| 10 MB | 12 ms | 8 ms | 1.5× | ↓ 65 % |
| 1 GB | 980 ms | 340 ms | 2.9× | ↓ 78 % |
| 10 GB | 9.8 s | 3.4 s | 2.9× | ↓ 80 % |

**Compression:**  
Zstd 3 → 2.3× faster over network  
LZ4 → near-realtime stream speed  

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

### Transfer Example
```bash
orbit -s /data/source -d /backup/target --resume --checksum sha256
```

### Run a Manifest
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

Orbit analyses file size, path, and environment to decide automatically.

---

## 🔐 Security

- Safe path and permission handling  
- Optional checksum verification for tamper detection  
- Planned FIPS-compliant crypto library  
- No telemetry phone-home or hidden data collection  

---

## 🧩 Configuration

Global defaults can be set in `orbit.toml` or via environment variables.

Example:
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
- Zero-copy engine  
- Compression and checksum modules  
- Manifest system  
- Structured audit logs  
- Modular crate structure  
- Resume and retry improvements  

### 🧠 In Progress
- Watcher and telemetry service  
- Object storage connectors (S3, Azure, GCS)  
- Wormhole error-correction module  

### 🚧 Planned
- REST API for orchestration  
- Scheduler for timed jobs  
- Disk-space pre-check before compression  
- Plugin framework for external protocols  

---

## 🦀 Contributing

We welcome pull requests!  
Read `CONTRIBUTING.md` for coding style and branch flow.

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
