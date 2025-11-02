# 🚀 Orbit

> **O**pen **R**esilient **B**ulk **I**nformation **T**ransfer  

**The intelligent file transfer tool that never gives up** 💪  

[![Crates.io](https://img.shields.io/crates/v/orbit.svg)](https://crates.io/crates/orbit)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/saworbit/orbit/workflows/CI/badge.svg)](https://github.com/saworbit/orbit/actions)

---

## 🌟 What is Orbit?

Orbit is a **blazingly fast** 🔥 file transfer tool built in Rust that combines enterprise-grade reliability with cutting-edge performance. Whether you're backing up terabytes of data, syncing files across continents, transferring to network shares, or moving data to the cloud, Orbit has you covered.

**Key Philosophy:** Intelligence, resilience, and speed without compromise.

---

## ✨ Why Orbit?

| Feature | Benefit |
|---------|---------|
| 🚄 **3× Faster** | Zero-copy system calls transfer at device speed |
| 🛡️ **Bulletproof** | Smart resume with chunk verification, checksums, corruption detection |
| 🧠 **Smart** | Adapts strategy based on environment (zero-copy, compression, buffered) |
| 🌐 **Protocol Ready** | Local, SMB/CIFS, **S3**, Azure (expanding) |
| 📊 **Fully Auditable** | Structured JSON telemetry for every operation |
| 🧩 **Modular** | Clean architecture with reusable crates |
| 🌍 **Cross-Platform** | Linux, macOS, Windows with native optimizations |

---

## 🗂️ Manifest System + Starmap Planner

Orbit v0.4 introduces a **manifest-based transfer framework** with flight plans, cargo manifests, and verification tools.

### Current Workflow (v0.4.1)
```bash
# 1. Create flight plan (transfer metadata)
orbit manifest plan --source /data --dest /backup --output ./manifests

# 2. Execute transfer with manifest generation
orbit --source /data --dest /backup --recursive \
  --generate-manifest --manifest-dir ./manifests

# 3. Verify transfer integrity
orbit manifest verify --manifest-dir ./manifests
```

### 🔭 Current Starmap Features

- **Flight Plans** — JSON-based transfer metadata and file tracking
- **Cargo Manifests** — Per-file chunk-level verification
- **Verification Tools** — Post-transfer integrity checking
- **Diff Support** — Compare manifests with target directories
- **Audit Integration** — Full traceability for every operation

### 🚧 Planned: Declarative Manifests (v0.6.0+)

**Future support for TOML-based job definitions:**

```toml
# orbit.manifest.toml (PLANNED)
[defaults]
checksum = "sha256"
compression = "zstd:6"
resume = true

[[job]]
name = "source-sync"
source = "/data/source/"
destination = "/mnt/backup/source/"

[[job]]
name = "media-archive"
source = "/media/camera/"
destination = "/tank/archive/"
depends_on = ["source-sync"]  # Dependency ordering
```

**Planned execution:**
```bash
orbit run --manifest orbit.manifest.toml  # Coming in v0.6.0+
```

**Planned features:**
- Dependency graphs with automatic ordering
- Parallel execution of independent jobs
- Resource validation before execution
- Visual planning and execution graphs

---

## 🌐 Protocol Support

Orbit supports multiple storage backends through a unified protocol abstraction layer.

| Protocol | Status | Feature Flag | Description |
|----------|--------|--------------|-------------|
| 🗂️ **Local** | ✅ Stable | Built-in | Local filesystem with zero-copy optimization |
| 🌐 **SMB/CIFS** | 🟡 Ready* | `smb-native` | Native SMB2/3 client (pure Rust, no dependencies) |
| ☁️ **S3** | ✅ **NEW!** | `s3-native` | Amazon S3 and compatible object storage (MinIO, LocalStack) |
| ☁️ **Azure Blob** | 🚧 Planned | - | Microsoft Azure Blob Storage |
| ☁️ **GCS** | 🚧 Planned | - | Google Cloud Storage |
| 🌐 **WebDAV** | 🚧 Planned | - | WebDAV protocol support |

**\*SMB Status:** Implementation complete (~1,900 lines) but blocked by upstream dependency conflict. See [`docs/SMB_NATIVE_STATUS.md`](docs/SMB_NATIVE_STATUS.md) for details.

### 🆕 S3 Cloud Storage (v0.4.1)

Transfer files seamlessly to AWS S3 and S3-compatible storage services with advanced features:

```bash
# Upload to S3
orbit --source /local/dataset.tar.gz --dest s3://my-bucket/backups/dataset.tar.gz

# Download from S3
orbit --source s3://my-bucket/data/report.pdf --dest ./report.pdf

# Sync directory to S3 with compression
orbit --source /local/photos --dest s3://my-bucket/photos/ \
  --mode sync --compress zstd:5 --recursive

# Use with MinIO
export S3_ENDPOINT=http://localhost:9000
orbit --source file.txt --dest s3://my-bucket/file.txt
```

**Note:** S3-specific flags (`--region`, `--storage-class`) are planned for v0.6.0. Currently configure via environment variables or configuration file.

**S3 Features:**
- ✅ Pure Rust (no AWS CLI dependency)
- ✅ Multipart upload/download for large files (>5MB)
- ✅ Resumable transfers with checkpoint support
- ✅ Parallel chunk transfers (configurable)
- ✅ All storage classes (Standard, IA, Glacier, etc.)
- ✅ Server-side encryption (AES-256, AWS KMS)
- ✅ S3-compatible services (MinIO, LocalStack, DigitalOcean Spaces)
- ✅ Flexible authentication (env vars, credentials file, IAM roles)
- ✅ Full integration with manifest system
- ✅ **Object versioning** - Full version lifecycle management (v0.4.1)
- ✅ **Batch operations** - Concurrent processing with rate limiting (v0.4.1)
- ✅ **Enhanced error recovery** - Circuit breaker and exponential backoff (v0.4.1)
- ✅ **Progress callbacks** - Real-time transfer tracking (v0.4.1)

**Quick Example:**
```rust
use orbit::protocol::s3::{S3Client, S3Config};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = S3Config::new("my-bucket".to_string());
    let client = S3Client::new(config).await?;
    
    // Upload
    client.upload_bytes(Bytes::from("Hello, S3!"), "hello.txt").await?;
    
    // Download
    let data = client.download_bytes("hello.txt").await?;
    println!("Downloaded: {}", String::from_utf8_lossy(&data));
    
    Ok(())
}
```

📖 **Full Documentation:** See [`docs/S3_USER_GUIDE.md`](docs/S3_USER_GUIDE.md) for complete guide including authentication, configuration, multipart transfers, and S3-compatible storage setup.

### SMB/CIFS Network Shares
```bash
# Copy to SMB share (when available)
orbit --source /local/file.txt --dest smb://user:pass@server/share/file.txt

# Sync directories over SMB
orbit --source /local/data --dest smb://server/backup \
  --mode sync --resume --parallel 4 --recursive
```

**Note:** SMB-specific flags (`--smb-user`, `--smb-domain`) are planned for future releases. Currently embed credentials in URI or use configuration file.

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
  "job": "cloud-backup",
  "source": "/local/data/",
  "destination": "s3://my-bucket/backups/",
  "protocol": "s3",
  "bytes_transferred": 104857600,
  "duration_ms": 2341,
  "compression": "zstd:1",
  "compression_ratio": 2.3,
  "checksum_algorithm": "sha256",
  "checksum_match": true,
  "storage_class": "INTELLIGENT_TIERING",
  "multipart_parts": 20,
  "status": "success",
  "retries": 0,
  "starmap_node": "orbit.node.cloud-backup"
}
```

**Audit Features:**
- JSON Lines format (machine-parseable)
- Timestamped with nanosecond precision
- Full job context and metadata
- Protocol-specific metrics (S3 storage class, multipart info, etc.)
- Ready for ELK, Loki, Datadog ingestion
- Starmap node correlation

---

## ⚡ Performance Benchmarks

| File Size | Traditional cp | Orbit (Zero-Copy) | Speedup | CPU Usage |
|-----------|----------------|-------------------|---------|-----------|
| 10 MB | 12 ms | 8 ms | 1.5× | ↓ 65% |
| 1 GB | 980 ms | 340 ms | 2.9× | ↓ 78% |
| 10 GB | 9.8 s | 3.4 s | 2.9× | ↓ 80% |

**S3 Transfer Performance:**
- **Multipart Upload:** 500+ MB/s on high-bandwidth links
- **Parallel Operations:** 4-16 concurrent chunks (configurable)
- **Adaptive Chunking:** 5MB-2GB chunks based on file size
- **Resume Efficiency:** Chunk-level verification with intelligent restart decisions

**Compression Performance:**
- Zstd level 3 → 2.3× faster over networks
- LZ4 → near-realtime local copies
- Adaptive selection based on link speed

---

## 📖 CLI Quick Reference

**Current syntax (v0.4.1):**
```bash
orbit --source <PATH> --dest <PATH> [FLAGS]
orbit manifest <plan|verify|diff|info> [OPTIONS]
orbit <stats|presets|capabilities>
```

**Planned syntax (v0.6.0+):**
```bash
orbit cp <SOURCE> <DEST> [FLAGS]          # Friendly alias
orbit sync <SOURCE> <DEST> [FLAGS]        # Sync mode alias
orbit run --manifest <FILE>               # Execute from manifest (planned)
```

> **Note:** The current release uses flag-based syntax. User-friendly subcommands like `cp`, `sync`, and `run` are planned for v0.6.0.

---

## 🚀 Quick Start

### Install
```bash
# From crates.io
cargo install orbit

# With S3 support
cargo install orbit --features s3-native

# From source
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build --release --features s3-native
sudo cp target/release/orbit /usr/local/bin/
```

### Basic Usage
```bash
# Simple copy
orbit --source source.txt --dest destination.txt

# Copy with resume and checksum verification (checksum is enabled by default)
orbit --source large-file.iso --dest /backup/large-file.iso --resume

# Recursive directory copy with compression
orbit --source /data/photos --dest /backup/photos --recursive --compress zstd:5

# Sync with parallel transfers
orbit --source /source --dest /destination --mode sync --parallel 8 --recursive

# Upload to S3
orbit --source dataset.tar.gz --dest s3://my-bucket/backups/dataset.tar.gz

# Create flight plan manifest
orbit manifest plan --source /data --dest /backup --output ./manifests
```

---

## 🧠 Smart Strategy Selection

Orbit automatically selects the optimal transfer strategy:
```
Same-disk large file  → Zero-copy (copy_file_range, sendfile)
Cross-filesystem      → Streaming with buffer pool
Slow network link     → Compression (zstd/lz4)
Cloud storage (S3)    → Multipart with parallel chunks
Unreliable network    → Smart resume (detect corruption, revalidate modified files)
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
| 🌐 `protocols` | Network protocol implementations | ✅ S3, 🟡 SMB |
| 🕵️ `core-watcher` | Monitoring beacon | 🚧 Planned |
| 🧪 `wormhole` | Forward-error correction | 🚧 Dev |

This structure ensures isolation, testability, and reusability.

---

## 🕵️ Watcher / Beacon

**Status:** 🚧 Planned for v0.6.0+

A companion service that will monitor Orbit runtime health:

**Planned Features:**
- Detect stalled transfers
- Track telemetry and throughput
- Trigger recovery actions
- Prometheus-compatible metrics export

This feature is currently in the design phase. See the [roadmap](#-roadmap) for details.

---

## ⚙️ Configuration

Persistent defaults via `orbit.toml`:
```toml
# ~/.orbit/orbit.toml or ./orbit.toml

# Copy mode: "copy", "sync", "update", or "mirror"
copy_mode = "copy"

# Enable recursive directory copying
recursive = true

# Preserve file metadata (timestamps, permissions)
preserve_metadata = true

# Enable resume capability for interrupted transfers
resume_enabled = true

# Enable checksum verification
verify_checksum = true

# Compression: "none", "lz4", or { zstd = { level = 5 } }
compression = { zstd = { level = 5 } }

# Show progress bar
show_progress = true

# Chunk size in bytes for buffered I/O
chunk_size = 1048576  # 1 MB

# Number of retry attempts on failure
retry_attempts = 3

# Retry delay in seconds
retry_delay_secs = 2

# Use exponential backoff for retries
exponential_backoff = true

# Maximum bandwidth in bytes per second (0 = unlimited)
max_bandwidth = 0

# Number of parallel operations (0 = sequential)
parallel = 4

# Symbolic link handling: "skip", "follow", or "preserve"
symlink_mode = "skip"

# Exclude patterns (glob patterns)
exclude_patterns = [
    "*.tmp",
    "*.log",
    ".git/*",
    "node_modules/*",
]

# Dry run mode (don't actually copy)
dry_run = false

# Use zero-copy system calls when available
use_zero_copy = true

# Generate manifests for transfers
generate_manifest = false

# Audit log format: "json" or "csv"
audit_format = "json"

# Path to audit log file
audit_log_path = "/var/log/orbit_audit.log"
```

**Note:** S3-specific options (region, storage class) and SMB options (timeout, encryption) are configured via environment variables or protocol-specific configuration files in future releases.

**Configuration Priority:**
1. CLI arguments (highest)
2. `./orbit.toml` (project)
3. `~/.orbit/orbit.toml` (user)
4. Built-in defaults (lowest)

---

## 🔐 Security

- **Safe Path Handling** — Prevents traversal attacks
- **Checksum Verification** — SHA-256, BLAKE3 for integrity
- **Credential Protection** — Memory scrubbing on drop, no credential logging
- **S3 Encryption** — Server-side encryption (AES-256, AWS KMS)
- **No Telemetry Phone-Home** — All data stays local
- **AWS Credential Chain** — Secure credential sourcing (IAM roles, env vars, credential files)
- **Future FIPS Support** — Compliance-ready crypto modules

---

## 📈 Use Cases

### Cloud Data Lake Ingestion
```bash
# Upload analytics data to S3
orbit --source /data/analytics --dest s3://data-lake/raw/2025/ \
  --recursive \
  --parallel 16 \
  --compress zstd:3
```

Benefits: Parallel uploads, compression, checksums (storage class via config file)

### Enterprise Backup
```bash
# Use manifest system for complex backup jobs
orbit manifest plan --source /data --dest /backup --output ./manifests
orbit manifest verify --manifest-dir ./manifests
```

Benefits: Resume, checksums, parallel jobs, full audit trail

### Hybrid Cloud Migration
```bash
# Migrate local storage to S3
orbit --source /on-prem/data --dest s3://migration-bucket/data \
  --mode sync \
  --recursive \
  --resume \
  --parallel 12
```

Benefits: Resumable, parallel transfers (storage class via config file)

### Data Migration
```bash
orbit --source /old-storage --dest /new-storage \
  --recursive \
  --parallel 16 \
  --show-progress
```

Benefits: Parallel streams, verification enabled by default, progress tracking

### Network Shares
```bash
orbit --source /local/files --dest smb://nas/backup \
  --mode sync \
  --recursive \
  --resume \
  --retry-attempts 10
```

Benefits: Native SMB, automatic resume, exponential backoff

---

## 🧪 Roadmap

### ✅ Completed (v0.4.1)

- Zero-copy and compression engines
- Manifest + Starmap + Audit integration
- Structured telemetry with JSON Lines
- Modular crate architecture
- Resume and retry improvements with chunk-level verification
- **Native S3 support with multipart transfers** ⭐
- S3-compatible storage (MinIO, LocalStack)
- **S3 object versioning support** ⭐ NEW!
- **S3 batch operations with rate limiting** ⭐ NEW!
- **Enhanced error recovery (circuit breaker, exponential backoff)** ⭐ NEW!
- **Progress callbacks for UI integration** ⭐ NEW!
- SMB2/3 native implementation (awaiting upstream fix)

### 🚧 Planned (v0.6.0+)

#### CLI Improvements
- Friendly subcommands (`orbit cp`, `orbit sync`, `orbit run`) as aliases
- Protocol-specific flags (`--smb-user`, `--region`, `--storage-class`)
- File watching mode (`--watch`)
- Watcher component for monitoring transfer health

#### New Protocols
- Azure Blob Storage connector
- Google Cloud Storage (GCS)
- WebDAV protocol support

#### Advanced Features
- Wormhole FEC module for lossy networks
- REST orchestration API
- Job scheduler with cron-like syntax
- Plugin framework for custom protocols
- Disk-space pre-check
- Delta sync algorithm
- S3 Transfer Acceleration
- CloudWatch metrics integration

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

# Run with S3 support
cargo build --features s3-native
cargo test --features s3-native

# Run with SMB (when available)
cargo build --features smb-native

# Format and lint
cargo fmt
cargo clippy
```

### Areas We Need Help

- ☁️ Azure Blob and GCS implementations
- 🌐 Resolving SMB upstream dependencies
- 🧪 Testing on various platforms
- 📚 Documentation improvements
- 🐛 Bug reports and fixes

---

## 📚 Documentation

- **Quick Start:** This README
- **S3 Guide:** [`docs/S3_USER_GUIDE.md`](docs/S3_USER_GUIDE.md) ⭐ NEW!
- **Resume System:** [`docs/RESUME_SYSTEM.md`](docs/RESUME_SYSTEM.md) ⭐ NEW!
- **Protocol Guide:** [`PROTOCOL_GUIDE.md`](PROTOCOL_GUIDE.md)
- **SMB Status:** [`docs/SMB_NATIVE_STATUS.md`](docs/SMB_NATIVE_STATUS.md)
- **Manifest System:** [`docs/MANIFEST_SYSTEM.md`](docs/MANIFEST_SYSTEM.md)
- **Zero-Copy Guide:** [`docs/ZERO_COPY.md`](docs/ZERO_COPY.md)
- **API Reference:** Run `cargo doc --open`
- **Examples:** [`examples/`](examples/) directory

---

## 📜 License

**Apache License 2.0**

Orbit is licensed under the Apache License, Version 2.0 - a permissive open source license that allows you to:

- ✅ **Use** commercially and privately
- ✅ **Modify** and distribute
- ✅ **Patent use** - grants patent rights
- ✅ **Sublicense** to third parties

**Requirements:**
- **License and copyright notice** - Include a copy of the license and copyright notice with the software
- **State changes** - Document significant changes made to the code

**Limitations:**
- ❌ **Liability** - The license includes a limitation of liability
- ❌ **Warranty** - The software is provided "as is" without warranty
- ❌ **Trademark use** - Does not grant rights to use trade names or trademarks

📄 **Full license text:** See [LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0

```
Copyright 2024 Shane Wall

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

---

## 🙏 Acknowledgments

- Built with ❤️ in Rust
- Inspired by rsync, rclone, and modern transfer tools
- Thanks to the Rust community for excellent crates
- AWS SDK for Rust team for the excellent S3 client
- Special thanks to contributors and testers

---

<div align="center">

### Made with ❤️ and 🦀 by [Shane Wall](https://github.com/saworbit)

**Orbit — because your data deserves to travel in style.** ✨  

[⬆ Back to Top](#-orbit)

</div>