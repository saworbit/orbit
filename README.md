# 🚀 Orbit

> **O**pen **R**esilient **B**ulk **I**nformation **T**ransfer

**The intelligent file transfer tool that never gives up** 💪

[![Crates.io](https://img.shields.io/crates/v/orbit.svg)](https://crates.io/crates/orbit)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/saworbit/orbit/workflows/CI/badge.svg)](https://github.com/saworbit/orbit/actions)

---

## 📑 Table of Contents

- [What is Orbit?](#-what-is-orbit)
- [Why Orbit?](#-why-orbit)
- [Key Features](#-key-features)
  - [Disk Guardian](#-disk-guardian-pre-flight-safety)
  - [Manifest System + Starmap](#-manifest-system--starmap-planner)
  - [Magnetar State Machine](#-magnetar-persistent-job-state-machine)
  - [Metadata Preservation](#-metadata-preservation--transformation)
  - [Delta Detection](#-delta-detection-efficient-transfers)
  - [Protocol Support](#-protocol-support)
  - [Audit & Telemetry](#-audit-and-telemetry)
- [Quick Start](#-quick-start)
- [Performance](#-performance-benchmarks)
- [Use Cases](#-use-cases)
- [Configuration](#-configuration)
- [Documentation](#-documentation)
- [Roadmap](#-roadmap)
- [Contributing](#-contributing)

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
| 🛡️ **Safe** | Disk Guardian prevents mid-transfer failures with pre-flight checks |
| 🌐 **Protocol Ready** | Local, **SSH/SFTP**, SMB/CIFS, **S3**, with unified backend API |
| 📊 **Fully Auditable** | Structured JSON telemetry for every operation |
| 🧩 **Modular** | Clean architecture with reusable crates |
| 🌍 **Cross-Platform** | Linux, macOS, Windows with native optimizations |

---

## 🔑 Key Features

### 🛡️ Disk Guardian: Pre-Flight Safety

**NEW in v0.4.1!** Comprehensive disk space and filesystem validation to prevent mid-transfer failures.

**Prevents:**
- ❌ Mid-transfer disk-full errors
- ❌ OOM conditions from insufficient space
- ❌ Transfers to read-only filesystems
- ❌ Permission errors (detected early)

**Features:**
- **Safety Margins** — 10% extra space by default, fully configurable
- **Minimum Free Space** — Always leaves 100 MB free (configurable)
- **Filesystem Integrity** — Write permissions, read-only detection
- **Staging Areas** — Atomic transfers with temporary staging
- **Live Monitoring** — Optional filesystem watching (via `notify` crate)
- **Directory Estimation** — Pre-calculate space needed for directory transfers

```bash
# Automatic pre-flight checks for directory transfers
orbit --source /data --dest /backup --recursive
# Output:
# Performing pre-flight checks...
# Estimated transfer size: 5368709120 bytes
# ✓ Sufficient disk space (with safety margin)
```

**Manual API:**
```rust
use orbit::core::disk_guardian::{ensure_transfer_safety, GuardianConfig};

let config = GuardianConfig {
    safety_margin_percent: 0.10,      // 10% extra
    min_free_space: 100 * 1024 * 1024, // 100 MB
    check_integrity: true,
    enable_watching: false,
};

ensure_transfer_safety(dest_path, required_bytes, &config)?;
```

**Try it:**
```bash
cargo run --example disk_guardian_demo
```

📖 **Full Documentation:** See [`DISK_GUARDIAN.md`](DISK_GUARDIAN.md)

---

### 🗂️ Manifest System + Starmap Planner

Orbit v0.4 introduces a **manifest-based transfer framework** with flight plans, cargo manifests, and verification tools.

#### Current Workflow (v0.4.1)
```bash
# 1. Create flight plan (transfer metadata)
orbit manifest plan --source /data --dest /backup --output ./manifests

# 2. Execute transfer with manifest generation
orbit --source /data --dest /backup --recursive \
  --generate-manifest --manifest-dir ./manifests

# 3. Verify transfer integrity
orbit manifest verify --manifest-dir ./manifests
```

#### 🔭 Current Starmap Features

- **Flight Plans** — JSON-based transfer metadata and file tracking
- **Cargo Manifests** — Per-file chunk-level verification
- **Verification Tools** — Post-transfer integrity checking
- **Diff Support** — Compare manifests with target directories
- **Audit Integration** — Full traceability for every operation

#### 🚧 Planned: Declarative Manifests (v0.6.0+)

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

---

### 🧲 Magnetar: Persistent Job State Machine

**NEW in v0.4.1!** A crash-proof, idempotent state machine for managing persistent jobs with dual backend support.

**Prevents:**
- ❌ Duplicate work after crashes
- ❌ Lost progress on interruptions
- ❌ Dependency conflicts in DAG-based workflows
- ❌ Cascading failures from flaky external services

**Features:**
- **Atomic Claims** — Idempotent "pending → processing" transitions
- **Crash Recovery** — Resume from any point with chunk-level verification
- **DAG Dependencies** — Topological sorting for complex job graphs
- **Dual Backends** — SQLite (default) or redb (pure Rust, WASM-ready)
- **Zero-Downtime Migration** — Swap backends without stopping jobs
- **Analytics Ready** — Export to Parquet for analysis
- **Resilience Module** — Circuit breaker, connection pooling, and rate limiting for fault-tolerant data access ⭐ **NEW!**

```rust
use magnetar::JobStatus;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut store = magnetar::open("jobs.db").await?;

    // Load chunks from manifest
    let manifest = toml::from_str(r#"
        [[chunks]]
        id = 1
        checksum = "abc123"
    "#)?;

    store.init_from_manifest(42, &manifest).await?;

    // Process with automatic deduplication
    while let Some(chunk) = store.claim_pending(42).await? {
        // Do work... (if crash happens, chunk auto-reverts to pending)
        store.mark_status(42, chunk.chunk, JobStatus::Done, None).await?;
    }

    Ok(())
}
```

**Try it:**
```bash
cd crates/magnetar
cargo run --example basic_usage
cargo run --example crash_recovery  # Simulates crash and resume
cargo run --example resilience_demo --features resilience  # Circuit breaker demo
```

#### 🛡️ Resilience Module

**NEW in v0.4.1!** Built-in resilience patterns for fault-tolerant access to flaky external services like S3, SMB, and databases.

**Components:**
- **Circuit Breaker** — Fail-fast protection with automatic recovery
- **Connection Pool** — Efficient connection reuse with health checking
- **Rate Limiter** — Token bucket rate limiting to prevent service overload

```rust
use magnetar::resilience::prelude::*;
use std::sync::Arc;

// Setup resilience stack
let breaker = CircuitBreaker::new_default();
let pool = Arc::new(ConnectionPool::new_default(factory));
let limiter = RateLimiter::per_second(100);

// Execute with full protection
breaker.execute(|| {
    let pool = pool.clone();
    let limiter = limiter.clone();
    async move {
        limiter.execute(|| async {
            let conn = pool.acquire().await?;
            let result = perform_s3_operation(&conn).await;
            pool.release(conn).await;
            result
        }).await
    }
}).await?;
```

**Resilience Features:**
- ✅ Three-state circuit breaker (Closed → Open → HalfOpen)
- ✅ Exponential backoff with configurable retries
- ✅ Generic connection pool with health checks
- ✅ Pool statistics and monitoring
- ✅ Idle timeout and max lifetime management
- ✅ Rate limiting with token bucket algorithm
- ✅ Optional governor crate integration
- ✅ Thread-safe async/await support
- ✅ Transient vs permanent error classification
- ✅ S3 and SMB integration examples

📖 **Full Documentation:** See [`crates/magnetar/README.md`](crates/magnetar/README.md) and [`crates/magnetar/src/resilience/README.md`](crates/magnetar/src/resilience/README.md)

---

### 🏷️ Metadata Preservation & Transformation

**NEW in v0.4.1!** Comprehensive file metadata preservation with transformation capabilities for cross-platform transfers and reproducible builds.

**Features:**
- **Complete Attribute Support** — Timestamps (atime, mtime, ctime), permissions, ownership (UID/GID), extended attributes (xattrs)
- **Selective Preservation** — Choose exactly what to preserve: `times,perms,owners,xattrs`
- **Path Transformations** — Regex-based renaming with sed-like syntax: `s/old/new/`
- **Case Conversion** — Lowercase, uppercase, or titlecase filename normalization
- **Metadata Filtering** — Strip ownership, permissions, or xattrs for privacy/portability
- **Cross-Platform** — Graceful fallbacks on unsupported platforms
- **Backend Integration** — Works with local, SSH, S3 (extensible)
- **Strict Mode** — Configurable error handling (warn vs. fail)
- **Verification** — Post-transfer metadata validation

**Use Cases:**
- ✅ Cross-platform migrations (Unix → Windows, macOS → Linux)
- ✅ Reproducible builds (normalize timestamps, strip metadata)
- ✅ Privacy-aware backups (strip ownership information)
- ✅ Cloud storage with metadata (preserve via manifest integration)
- ✅ Archival compliance (preserve extended attributes, ACLs)

```bash
# Basic metadata preservation
orbit --source /data --dest /backup --recursive --preserve-metadata

# Selective preservation with detailed flags
orbit --source /data --dest /backup \
  --preserve=times,perms,owners,xattrs \
  --verify-metadata

# With path transformations
orbit --source /photos --dest /archive \
  --preserve=all \
  --transform="rename:s/IMG_/photo_/,case:lower"

# Strip sensitive metadata for cloud
orbit --source /data --dest s3://bucket/data \
  --preserve=times,perms \
  --transform="strip:ownership,strip:xattrs"

# Strict mode (fail on any metadata error)
orbit --source /critical --dest /backup \
  --preserve=all \
  --strict-metadata
```

**Preservation Flags:**
- `times` — Access and modification timestamps
- `perms` — Unix permissions (mode bits)
- `owners` — User and group ownership (UID/GID)
- `xattrs` — Extended attributes (requires `extended-metadata` feature)
- `all` — Preserve everything

**Transformation Options:**
- `rename:pattern=replacement` — Regex-based path renaming
- `case:lower|upper|title` — Filename case conversion
- `strip:xattrs|ownership|permissions` — Remove metadata
- `normalize:timestamps` — Set all timestamps to epoch (reproducible builds)

📖 **API Documentation:** See `src/core/file_metadata.rs`, `src/core/transform.rs`, and `src/core/metadata_ops.rs`

---

### 🔄 Delta Detection: Efficient Transfers

**NEW in v0.4.1!** rsync-inspired delta algorithm that minimizes bandwidth by transferring only changed blocks.

**Features:**
- **4 Detection Modes** — ModTime (fast), Size, Checksum (BLAKE3), Delta (block-based)
- **Rolling Checksum** — Adler-32 for O(1) per-byte block matching
- **Parallel Hashing** — Rayon-based concurrent block processing
- **Smart Fallback** — Automatic full copy for incompatible files
- **80-99% Savings** — For files with minor changes
- **Configurable Blocks** — 64KB to 4MB block sizes

**Use Cases:**
- ✅ Daily database backups (90-95% savings)
- ✅ VM image updates (85-95% savings)
- ✅ Large file synchronization over slow links
- ✅ Log file rotation (95-99% savings for append-only)

```bash
# Basic delta transfer
orbit --source bigfile.iso --dest bigfile.iso --check delta

# Recursive sync with custom block size
orbit --source /data --dest /backup --recursive \
  --check delta --block-size 512

# With resume for large files
orbit --source vm.qcow2 --dest backup/vm.qcow2 \
  --check delta --resume --block-size 2048
```

**Performance:**
- 1GB file with 5% changes: **10x faster** (3s vs 30s), **95% less data** (50MB vs 1GB)
- Identical files: **99% savings** with minimal CPU overhead

📖 **Full Documentation:** See [`DELTA_DETECTION_GUIDE.md`](DELTA_DETECTION_GUIDE.md) and [`DELTA_QUICKSTART.md`](DELTA_QUICKSTART.md)

---

### 🌐 Protocol Support

Orbit supports multiple storage backends through a **unified backend abstraction layer** that provides a consistent async API across all storage types.

| Protocol | Status | Feature Flag | Description |
|----------|--------|--------------|-------------|
| 🗂️ **Local** | ✅ Stable | Built-in | Local filesystem with zero-copy optimization |
| 🔐 **SSH/SFTP** | 🚧 WIP | `ssh-backend` | Remote filesystem access via SSH/SFTP (implementation in progress) |
| 🌐 **SMB/CIFS** | 🟡 Ready* | `smb-native` | Native SMB2/3 client (pure Rust, no dependencies) |
| ☁️ **S3** | ✅ **Stable** | `s3-native` | Amazon S3 and compatible object storage (MinIO, LocalStack) |
| ☁️ **Azure Blob** | 🚧 Planned | - | Microsoft Azure Blob Storage |
| ☁️ **GCS** | 🚧 Planned | - | Google Cloud Storage |
| 🌐 **WebDAV** | 🚧 Planned | - | WebDAV protocol support |

**\*SMB Status:** Implementation complete (~1,900 lines) but blocked by upstream dependency conflict. See [`docs/SMB_NATIVE_STATUS.md`](docs/SMB_NATIVE_STATUS.md) for details.

#### 🆕 Unified Backend Abstraction (v0.4.1)

**NEW!** Write once, run on any storage backend. The backend abstraction provides a consistent async API across all storage types:

```rust
use orbit::backend::{Backend, LocalBackend, SshBackend, S3Backend};

// All backends implement the same trait
async fn copy_file<B: Backend>(backend: &B, src: &Path, dest: &Path) -> Result<()> {
    let data = backend.read(src).await?;
    backend.write(dest, data, Default::default()).await?;
    Ok(())
}

// Works with any backend
let local = LocalBackend::new();
let ssh = SshBackend::connect(config).await?;
let s3 = S3Backend::new(s3_config).await?;
```

**Features:**
- ✅ **URI-based configuration**: `ssh://user@host/path`, `s3://bucket/key`, etc.
- ✅ **Streaming I/O**: Memory-efficient for large files
- ✅ **Metadata operations**: Set permissions, timestamps, xattrs, ownership
- ✅ **Extensibility**: Plugin system for custom backends
- ✅ **Type-safe**: Strong typing with comprehensive error handling
- ✅ **Security**: Built-in secure credential handling

📖 **Full Guide:** [BACKEND_GUIDE.md](BACKEND_GUIDE.md)

#### 🆕 S3 Cloud Storage (v0.4.1)

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
- ✅ Object versioning and lifecycle management
- ✅ Batch operations with rate limiting
- ✅ **Resilience patterns** — Circuit breaker, connection pooling, and rate limiting via Magnetar ⭐

📖 **Full Documentation:** See [`docs/S3_USER_GUIDE.md`](docs/S3_USER_GUIDE.md)

#### SMB/CIFS Network Shares

```bash
# Copy to SMB share (when available)
orbit --source /local/file.txt --dest smb://user:pass@server/share/file.txt

# Sync directories over SMB
orbit --source /local/data --dest smb://server/backup \
  --mode sync --resume --parallel 4 --recursive
```

**SMB Features:**
- Pure Rust (no libsmbclient dependency)
- SMB2/3 only (SMBv1 disabled for security)
- Encryption support (AES-GCM, AES-CCM)
- Async/await with Tokio
- Adaptive chunking (256KB-2MB blocks)
- Integration with manifest system

---

### 📊 Audit and Telemetry

Every operation emits structured audit events for full observability.

**Example Audit Log:**
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
- Protocol-specific metrics
- Ready for ELK, Loki, Datadog ingestion
- Starmap node correlation

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

# Copy with resume and checksum verification
orbit --source large-file.iso --dest /backup/large-file.iso --resume

# Recursive directory copy with compression
orbit --source /data/photos --dest /backup/photos --recursive --compress zstd:5

# Sync with parallel transfers
orbit --source /source --dest /destination --mode sync --parallel 8 --recursive

# Upload to S3
orbit --source dataset.tar.gz --dest s3://my-bucket/backups/dataset.tar.gz

# Preserve metadata with transformations
orbit --source /data --dest /backup --recursive \
  --preserve=times,perms,owners \
  --transform="case:lower"

# Create flight plan manifest
orbit manifest plan --source /data --dest /backup --output ./manifests
```

---

## ⚡ Performance Benchmarks

### Local Transfer Performance

| File Size | Traditional cp | Orbit (Zero-Copy) | Speedup | CPU Usage |
|-----------|----------------|-------------------|---------|-----------|
| 10 MB | 12 ms | 8 ms | 1.5× | ↓ 65% |
| 1 GB | 980 ms | 340 ms | 2.9× | ↓ 78% |
| 10 GB | 9.8 s | 3.4 s | 2.9× | ↓ 80% |

### S3 Transfer Performance

- **Multipart Upload:** 500+ MB/s on high-bandwidth links
- **Parallel Operations:** 4-16 concurrent chunks (configurable)
- **Adaptive Chunking:** 5MB-2GB chunks based on file size
- **Resume Efficiency:** Chunk-level verification with intelligent restart decisions

### Compression Performance

- Zstd level 3 → 2.3× faster over networks
- LZ4 → near-realtime local copies
- Adaptive selection based on link speed

---

## 🧠 Smart Strategy Selection

Orbit automatically selects the optimal transfer strategy:

```
Same-disk large file  → Zero-copy (copy_file_range, sendfile)
Cross-filesystem      → Streaming with buffer pool
Slow network link     → Compression (zstd/lz4)
Cloud storage (S3)    → Multipart with parallel chunks
Unreliable network    → Smart resume (detect corruption, revalidate)
Critical data         → SHA-256 checksum + audit log
Directory transfers   → Disk Guardian pre-flight checks
```

You can override with explicit flags when needed.

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

**Benefits:** Parallel uploads, compression, checksums, automatic pre-flight checks

### Enterprise Backup

```bash
# Use manifest system for complex backup jobs
orbit manifest plan --source /data --dest /backup --output ./manifests
orbit manifest verify --manifest-dir ./manifests
```

**Benefits:** Resume, checksums, parallel jobs, full audit trail, disk space validation

### Hybrid Cloud Migration

```bash
# Migrate local storage to S3
orbit --source /on-prem/data --dest s3://migration-bucket/data \
  --mode sync \
  --recursive \
  --resume \
  --parallel 12
```

**Benefits:** Resumable, parallel transfers, pre-flight safety checks

### Data Migration

```bash
orbit --source /old-storage --dest /new-storage \
  --recursive \
  --parallel 16 \
  --show-progress
```

**Benefits:** Parallel streams, verification enabled by default, progress tracking, disk space validation

### Network Shares

```bash
orbit --source /local/files --dest smb://nas/backup \
  --mode sync \
  --recursive \
  --resume \
  --retry-attempts 10
```

**Benefits:** Native SMB, automatic resume, exponential backoff

---

## ⚙️ Configuration

### Configuration File

Persistent defaults via `orbit.toml`:

```toml
# ~/.orbit/orbit.toml or ./orbit.toml

# Copy mode: "copy", "sync", "update", or "mirror"
copy_mode = "copy"

# Enable recursive directory copying
recursive = true

# Preserve file metadata (timestamps, permissions)
preserve_metadata = true

# Detailed metadata preservation flags (overrides preserve_metadata if set)
# Options: "times", "perms", "owners", "xattrs", "all"
preserve_flags = "times,perms,owners"

# Metadata transformation configuration
# Format: "rename:pattern=replacement,case:lower,strip:xattrs"
transform = "case:lower"

# Strict metadata preservation (fail on any metadata error)
strict_metadata = false

# Verify metadata after transfer
verify_metadata = false

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

### Configuration Priority

1. CLI arguments (highest)
2. `./orbit.toml` (project)
3. `~/.orbit/orbit.toml` (user)
4. Built-in defaults (lowest)

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
| 🛡️ `disk-guardian` | Pre-flight space & integrity checks | ✅ Stable |
| 🧲 `magnetar` | Idempotent job state machine (SQLite + redb) | ✅ **NEW!** |
| 🛡️ `magnetar::resilience` | Circuit breaker, connection pool, rate limiter | ✅ **NEW!** |
| 🌐 `protocols` | Network protocol implementations | ✅ S3, 🟡 SMB |
| 🕵️ `core-watcher` | Monitoring beacon | 🚧 Planned |
| 🧪 `wormhole` | Forward-error correction | 🚧 Dev |

This structure ensures isolation, testability, and reusability.

---

## 🔐 Security

- **Safe Path Handling** — Prevents traversal attacks
- **Checksum Verification** — SHA-256, BLAKE3 for integrity
- **Credential Protection** — Memory scrubbing on drop, no credential logging
- **S3 Encryption** — Server-side encryption (AES-256, AWS KMS)
- **No Telemetry Phone-Home** — All data stays local
- **AWS Credential Chain** — Secure credential sourcing (IAM roles, env vars, credential files)
- **Pre-Flight Validation** — Disk Guardian prevents dangerous operations
- **Future FIPS Support** — Compliance-ready crypto modules

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

## 🧪 Roadmap

### ✅ Completed (v0.4.1)

- Zero-copy and compression engines
- Manifest + Starmap + Audit integration
- Structured telemetry with JSON Lines
- Modular crate architecture
- Resume and retry improvements with chunk-level verification
- **Native S3 support with multipart transfers** ⭐
- S3-compatible storage (MinIO, LocalStack)
- S3 object versioning support
- S3 batch operations with rate limiting
- Enhanced error recovery (circuit breaker, exponential backoff)
- Progress callbacks for UI integration
- **Disk Guardian: Pre-flight space & integrity checks** ⭐
- **Magnetar: Idempotent job state machine with SQLite + redb backends** ⭐ **NEW!**
- **Magnetar Resilience Module: Circuit breaker, connection pooling, rate limiting** ⭐ **NEW!**
- **Delta Detection: rsync-inspired efficient transfers with block-based diffing** ⭐ **NEW!**
- **Metadata Preservation & Transformation: Comprehensive attribute handling with transformations** ⭐ **NEW!**
- SMB2/3 native implementation (awaiting upstream fix)

### 🚧 In Progress (v0.5.0)

- Watcher component for monitoring transfer health
- Enhanced CLI with subcommands

### 🔮 Planned (v0.6.0+)

#### CLI Improvements
- Friendly subcommands (`orbit cp`, `orbit sync`, `orbit run`) as aliases
- Protocol-specific flags (`--smb-user`, `--region`, `--storage-class`)
- File watching mode (`--watch`)
- Interactive mode with prompts

#### New Protocols
- Azure Blob Storage connector
- Google Cloud Storage (GCS)
- WebDAV protocol support

#### Advanced Features
- Wormhole FEC module for lossy networks
- REST orchestration API
- Job scheduler with cron-like syntax
- Plugin framework for custom protocols
- S3 Transfer Acceleration
- CloudWatch metrics integration
- Disk quota integration

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

### User Guides
- **Quick Start:** This README
- **S3 Guide:** [`docs/S3_USER_GUIDE.md`](docs/S3_USER_GUIDE.md)
- **Disk Guardian:** [`DISK_GUARDIAN.md`](DISK_GUARDIAN.md)
- **Magnetar:** [`crates/magnetar/README.md`](crates/magnetar/README.md) ⭐ **NEW!**
- **Resilience Module:** [`crates/magnetar/src/resilience/README.md`](crates/magnetar/src/resilience/README.md) ⭐ **NEW!**
- **Resume System:** [`docs/RESUME_SYSTEM.md`](docs/RESUME_SYSTEM.md)
- **Protocol Guide:** [`PROTOCOL_GUIDE.md`](PROTOCOL_GUIDE.md)

### Technical Documentation
- **SMB Status:** [`docs/SMB_NATIVE_STATUS.md`](docs/SMB_NATIVE_STATUS.md)
- **Manifest System:** [`docs/MANIFEST_SYSTEM.md`](docs/MANIFEST_SYSTEM.md)
- **Zero-Copy Guide:** [`docs/ZERO_COPY.md`](docs/ZERO_COPY.md)
- **Magnetar Quick Start:** [`crates/magnetar/QUICKSTART.md`](crates/magnetar/QUICKSTART.md) ⭐ **NEW!**
- **Resilience Patterns:** [`crates/magnetar/src/resilience/README.md`](crates/magnetar/src/resilience/README.md) ⭐ **NEW!**
- **API Reference:** Run `cargo doc --open`

### Examples
- **Basic Examples:** [`examples/`](examples/) directory
- **S3 Examples:** [`examples/s3_*.rs`](examples/)
- **Disk Guardian Demo:** [`examples/disk_guardian_demo.rs`](examples/disk_guardian_demo.rs)
- **Magnetar Examples:** [`crates/magnetar/examples/`](crates/magnetar/examples/) ⭐ **NEW!**
- **Resilience Demo:** [`crates/magnetar/examples/resilience_demo.rs`](crates/magnetar/examples/resilience_demo.rs) ⭐ **NEW!**
- **Progress Demo:** [`examples/progress_demo.rs`](examples/progress_demo.rs)

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
