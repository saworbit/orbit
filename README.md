# 🚀 Orbit

> **O**pen **R**esilient **B**ulk **I**nformation **T**ransfer  

**The intelligent file transfer tool that never gives up** 💪  

[![Crates.io](https://img.shields.io/crates/v/orbit.svg)](https://crates.io/crates/orbit)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/saworbit/orbit/workflows/CI/badge.svg)](https://github.com/saworbit/orbit/actions)

---

## 🌟 What is Orbit?

Orbit is a **blazingly fast** 🔥 file transfer tool built in Rust that combines enterprise-grade reliability with cutting-edge performance. Whether you're backing up terabytes of data, syncing files across continents, or transferring to network shares, Orbit has you covered.

**Key Philosophy:** Intelligence, resilience, and speed without compromise.

---

## ✨ Why Orbit?

| Feature | Benefit |
|---------|---------|
| 🚄 **3× Faster** | Zero-copy system calls transfer at device speed |
| 🛡️ **Bulletproof** | Automatic resume, checksums, retries with exponential backoff |
| 🧠 **Smart** | Adapts strategy based on environment (zero-copy, compression, buffered) |
| 🌐 **Protocol Ready** | Local, SMB/CIFS, S3, Azure (expanding) |
| 📊 **Fully Auditable** | Structured JSON telemetry for every operation |
| 🧩 **Modular** | Clean architecture with reusable crates |
| 🌍 **Cross-Platform** | Linux, macOS, Windows with native optimizations |

---

## 🗂️ Manifest System + Starmap Planner

Orbit v0.4 introduces a **declarative transfer framework** where jobs are defined in TOML manifests and executed through Starmap, an intelligent dependency planner.

### Example Manifest
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
depends_on = ["source-sync"]

[[job]]
name = "smb-backup"
source = "/local/data/"
destination = "smb://server/backups/"
protocol = "smb-native"
```

### Execute
```bash
orbit run --manifest orbit.manifest.toml
```

### 🔭 Starmap Features

- **Dependency Graph** — Validates and orders jobs before execution
- **Parallel Execution** — Runs independent jobs concurrently
- **Resource Validation** — Checks space, permissions, connectivity upfront
- **Visual Planning** — Generates execution graph when enabled
- **Audit Integration** — Every job tagged for full traceability

---

## 🌐 Protocol Support

Orbit supports multiple storage backends through a unified protocol abstraction layer.

| Protocol | Status | Feature Flag | Description |
|----------|--------|--------------|-------------|
| 🗂️ **Local** | ✅ Stable | Built-in | Local filesystem with zero-copy optimization |
| 🌐 **SMB/CIFS** | 🟡 Ready* | `smb-native` | Native SMB2/3 client (pure Rust, no dependencies) |
| ☁️ **S3** | 🚧 Planned | - | Amazon S3 and compatible object storage |
| ☁️ **Azure Blob** | 🚧 Planned | - | Microsoft Azure Blob Storage |
| ☁️ **GCS** | 🚧 Planned | - | Google Cloud Storage |
| 🌐 **WebDAV** | 🚧 Planned | - | WebDAV protocol support |

**\*SMB Status:** Implementation complete (~1,900 lines) but blocked by upstream dependency conflict. See [`docs/SMB_NATIVE_STATUS.md`](docs/SMB_NATIVE_STATUS.md) for details.

### SMB/CIFS Network Shares
```bash
# Copy to SMB share (when available)
orbit cp /local/file.txt smb://user:pass@server/share/file.txt

# Sync directories over SMB
orbit sync /local/data smb://server/backup --resume --parallel 4

# With authentication
orbit cp file.txt smb://fileserver/projects/ \
  --smb-user admin \
  --smb-domain CORP
```

**SMB Features:**
- Pure Rust (no libsmbclient dependency)
- SMB2/3 only (SMBv1 disabled for security)
- Encryption support (AES-GCM, AES-CCM)
- Async/await with Tokio
- Adaptive chunking (256KB-2MB blocks)
- Integration with manifest system

---

## 📊 Audit and Telemetry

Every operation emits structured audit events for full observability.

### Example Audit Log
```json
{
  "timestamp": "2025-10-25T16:42:19Z",
  "job": "media-archive",
  "source": "/media/camera/",
  "destination": "/tank/archive/",
  "bytes_transferred": 104857600,
  "duration_ms": 2341,
  "compression": "zstd:1",
  "compression_ratio": 2.3,
  "checksum_algorithm": "sha256",
  "checksum_match": true,
  "status": "success",
  "retries": 0,
  "starmap_node": "orbit.node.media-archive"
}
```

**Audit Features:**
- JSON Lines format (machine-parseable)
- Timestamped with nanosecond precision
- Full job context and metadata
- Ready for ELK, Loki, Datadog ingestion
- Starmap node correlation

---

## ⚡ Performance Benchmarks

| File Size | Traditional cp | Orbit (Zero-Copy) | Speedup | CPU Usage |
|-----------|----------------|-------------------|---------|-----------|
| 10 MB | 12 ms | 8 ms | 1.5× | ↓ 65% |
| 1 GB | 980 ms | 340 ms | 2.9× | ↓ 78% |
| 10 GB | 9.8 s | 3.4 s | 2.9× | ↓ 80% |

**Compression Performance:**
- Zstd level 3 → 2.3× faster over networks
- LZ4 → near-realtime local copies
- Adaptive selection based on link speed

---

## 🚀 Quick Start

### Install
```bash
# From crates.io
cargo install orbit

# From source
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build --release
sudo cp target/release/orbit /usr/local/bin/
```

### Basic Usage
```bash
# Simple copy
orbit cp source.txt destination.txt

# Copy with resume and checksum
orbit cp large-file.iso /backup/ --resume --checksum sha256

# Recursive directory copy with compression
orbit cp /data/photos /backup/photos -R --compress zstd:5

# Sync with parallel transfers
orbit sync /source /destination --parallel 8 --mode sync

# Run manifest
orbit run --manifest my-jobs.toml
```

---

## 🧠 Smart Strategy Selection

Orbit automatically selects the optimal transfer strategy:
```
Same-disk large file  → Zero-copy (copy_file_range, sendfile)
Cross-filesystem      → Streaming with buffer pool
Slow network link     → Compression (zstd/lz4)
Unreliable network    → Resume + exponential backoff retry
Critical data         → SHA-256 checksum + audit log
```

You can override with explicit flags when needed.

---

## 🧩 Modular Architecture

Orbit is built from clean, reusable crates:

| Crate | Purpose | Status |
|-------|---------|--------|
| 🧩 `core-manifest` | Manifest parsing and job orchestration | ✅ Stable |
| 🌌 `core-starmap` | Job planner and dependency graph | ✅ Stable |
| 📊 `core-audit` | Structured logging and telemetry | ✅ Stable |
| ⚡ `core-zero-copy` | OS-level optimized I/O | ✅ Stable |
| 🗜️ `core-compress` | Compression and decompression | ✅ Stable |
| 🌐 `protocols` | Network protocol implementations | 🟡 Partial |
| 🕵️ `core-watcher` | Monitoring beacon | 🚧 Planned |
| 🧪 `wormhole` | Forward-error correction | 🚧 Dev |

This structure ensures isolation, testability, and reusability.

---

## 🕵️ Watcher / Beacon

A companion service that monitors Orbit runtime health:
```bash
# Check transfer status
orbit watcher --status

# Monitor for stalls
orbit watcher --alert-on-stall 300s

# Export metrics
orbit watcher --metrics-port 9090
```

Features:
- Detects stalled transfers
- Tracks telemetry and throughput
- Triggers recovery actions
- Prometheus-compatible metrics

---

## ⚙️ Configuration

Persistent defaults via `orbit.toml` or environment variables:
```toml
# ~/.orbit/orbit.toml or ./orbit.toml

[defaults]
compression = "zstd:5"
checksum = "sha256"
concurrency = 8
resume = true
audit_log = "/var/log/orbit_audit.log"
telemetry = true

[exclude]
patterns = [
    "*.tmp",
    "*.log",
    ".git/*",
    "node_modules/*",
]

[smb]
timeout_seconds = 30
max_retries = 5
```

**Configuration Priority:**
1. CLI arguments (highest)
2. Environment variables
3. `./orbit.toml` (project)
4. `~/.orbit/orbit.toml` (user)
5. Built-in defaults (lowest)

---

## 🔐 Security

- **Safe Path Handling** — Prevents traversal attacks
- **Checksum Verification** — SHA-256, BLAKE3 for integrity
- **Credential Protection** — Memory scrubbing on drop
- **No Telemetry Phone-Home** — All data stays local
- **Future FIPS Support** — Compliance-ready crypto modules

---

## 📈 Use Cases

### Enterprise Backup
```bash
orbit run --manifest backup.toml --audit /var/log/backup_audit.log
```

Benefits: Resume, checksums, parallel jobs, full audit trail

### Remote Development
```bash
orbit sync ./local-project user@remote:/project --watch --compress lz4
```

Benefits: Fast incremental sync, compression, file watching

### Data Migration
```bash
orbit cp /old-storage /new-storage -R --parallel 16 --verify
```

Benefits: Parallel streams, verification, progress tracking

### Network Shares
```bash
orbit sync /local/files smb://nas/backup --resume --retry 10
```

Benefits: Native SMB, automatic resume, exponential backoff

---

## 🧪 Roadmap

### ✅ Completed (v0.4.0)

- Zero-copy and compression engines
- Manifest + Starmap + Audit integration
- Structured telemetry with JSON Lines
- Modular crate architecture
- Resume and retry improvements
- SMB2/3 native implementation (awaiting upstream fix)

### 🧠 In Progress (v0.4.1)

- Watcher beacon service
- SMB dependency resolution
- Enhanced error recovery
- Performance optimizations

### 🚧 Planned (v0.5.0+)

- Object storage connectors (S3, Azure, GCS)
- WebDAV protocol support
- Wormhole FEC module for lossy networks
- REST orchestration API
- Job scheduler with cron-like syntax
- Plugin framework for custom protocols
- Disk-space pre-check
- Delta sync algorithm

---

## 🦀 Contributing

Pull requests welcome! See `CONTRIBUTING.md` for code style and guidelines.

### Development
```bash
# Clone and build
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build

# Run tests
cargo test

# Run with SMB (when available)
cargo build --features smb-native

# Format and lint
cargo fmt
cargo clippy
```

### Areas We Need Help

- 🌐 Resolving SMB upstream dependencies
- ☁️ Cloud storage protocol implementations
- 🧪 Testing on various platforms
- 📚 Documentation improvements
- 🐛 Bug reports and fixes

---

## 📚 Documentation

- **Quick Start:** This README
- **Configuration:** [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md)
- **SMB Status:** [`docs/SMB_NATIVE_STATUS.md`](docs/SMB_NATIVE_STATUS.md)
- **API Reference:** Run `cargo doc --open`
- **Examples:** [`examples/`](examples/) directory

---

## 📜 License

Licensed under both:
- 📄 **MIT License** — See [LICENSE-MIT](LICENSE-MIT)
- 📄 **Apache License 2.0** — See [LICENSE-APACHE](LICENSE-APACHE)

You may choose either license for your use.

---

## 🙏 Acknowledgments

- Built with ❤️ in Rust
- Inspired by rsync, rclone, and modern transfer tools
- Thanks to the Rust community for excellent crates
- Special thanks to contributors and testers

---

<div align="center">

### Made with ❤️ and 🦀 by [Shane Wall](https://github.com/saworbit)

**Orbit — because your data deserves to travel in style.** ✨  

[⬆ Back to Top](#-orbit)

</div>