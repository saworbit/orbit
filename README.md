# 🚀 Orbit

> **O**pen **R**esilient **B**ulk **I**nformation **T**ransfer

**The intelligent file transfer tool that never gives up** 💪

[![CI](https://github.com/saworbit/orbit/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/saworbit/orbit/actions/workflows/ci.yml)
[![Security Audit](https://github.com/saworbit/orbit/actions/workflows/compliance.yml/badge.svg)](https://github.com/saworbit/orbit/actions/workflows/compliance.yml)
[![codecov](https://codecov.io/gh/saworbit/orbit/branch/main/graph/badge.svg)](https://codecov.io/gh/saworbit/orbit)
[![Release](https://img.shields.io/github/v/release/saworbit/orbit)](https://github.com/saworbit/orbit/releases)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue)](https://ghcr.io/saworbit/orbit)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![GitHub](https://img.shields.io/github/stars/saworbit/orbit?style=social)](https://github.com/saworbit/orbit)

---

## ⚠️ Project Status: Alpha

**Orbit is currently in active development and should be considered alpha-quality software.**

- ✅ **Safe for**: Experimentation, evaluation, non-critical workloads, development environments
- ⚠️ **Use with caution for**: Important data transfers (test thoroughly first, maintain backups)
- ❌ **Not recommended for**: Mission-critical production systems without extensive testing

**What this means:**
- APIs may change between versions
- Some features are experimental and marked as such
- Extensive testing in your specific environment is recommended before production use

See the [Feature Maturity Matrix](#-feature-maturity-matrix) below for per-feature stability status.

---

## 📑 Table of Contents

- [Project Status](#️-project-status-alpha-v050)
- [What is Orbit?](#-what-is-orbit)
- [Why Orbit?](#-why-orbit)
- [Feature Maturity Matrix](#-feature-maturity-matrix)
- [Key Features](#-key-features)
  - [Error Handling & Retries](#-error-handling--retries-never-give-up)
  - [Disk Guardian](#️-disk-guardian-pre-flight-safety)
  - [Config Optimizer](#️-config-optimizer)
  - [Manifest System + Starmap](#️-manifest-system--starmap-planner)
  - [Metadata Preservation](#️-metadata-preservation--transformation)
  - [Delta Detection](#-delta-detection-efficient-transfers)
  - [Progress Reporting & Operational Controls](#-progress-reporting--operational-controls)
  - [Inclusion/Exclusion Filters](#-inclusionexclusion-filters-selective-transfers)
  - [Protocol Support](#-protocol-support)
  - [Audit & Telemetry](#-audit-and-telemetry)
  - [Data Flow Patterns](#-data-flow-patterns)
- [Quick Start](#-quick-start)
- [Performance Benchmarks](#-performance-benchmarks)
- [Smart Strategy Selection](#-smart-strategy-selection)
- [Use Cases](#-use-cases)
- [Configuration](#️-configuration)
- [Modular Architecture](#-modular-architecture)
- [Security](#-security)
- [Documentation](#-documentation)
- [Roadmap](#-roadmap)
- [Contributing](#-contributing)
- [License](#-license)

---

## 🌟 What is Orbit?

Orbit is a file transfer tool built in Rust that aims to combine reliability with performance. Whether you're backing up data, syncing files across locations, transferring to network shares, or moving data to the cloud, Orbit provides features designed to help.

**Key Philosophy:** Intelligence, resilience, and speed. Currently in active development (alpha).

---

## ✨ Why Orbit?

| Feature | Benefit |
|---------|---------|
| 🚄 **Performance** | Zero-copy system calls for faster transfers (instant APFS cloning on macOS) |
| 🛡️ **Resilient** | Smart resume with chunk verification, checksums, corruption detection |
| 🧠 **Adaptive** | Adapts strategy based on environment (zero-copy, compression, buffered) |
| 🛡️ **Safe** | Disk Guardian prevents mid-transfer failures with pre-flight checks |
| 🌐 **Protocol Support** | Local, **SSH/SFTP**, SMB/CIFS (experimental), **S3**, **Azure Blob**, **GCS**, with unified backend API |
| 📊 **Auditable** | Structured JSON telemetry for operations |
| 🧩 **Modular** | Clean architecture with reusable crates |
| 🌍 **Cross-Platform** | Linux, macOS, Windows with native optimizations |

---

## 🎯 Feature Maturity Matrix

Understanding feature stability helps you make informed decisions about what to use in production.

| Feature | Maturity | Notes |
|---------|----------|-------|
| **Core File Copy (Buffered)** | 🟢 Stable | Well-tested, safe for production use |
| **Zero-Copy Optimization** | 🟢 Stable | Platform-specific (Linux, macOS, Windows) |
| **OrbitSystem Abstraction** | 🟢 Stable | I/O abstraction layer |
| **Resume/Checkpoint** | 🟡 Beta | Works well, needs more edge-case testing |
| **Compression (LZ4, Zstd)** | 🟢 Stable | Reliable for most workloads |
| **Checksum Verification** | 🟢 Stable | BLAKE3 (default), SHA-256 well-tested |
| **Local Filesystem** | 🟢 Stable | Primary use case, thoroughly tested |
| **SSH/SFTP Backend** | 🟡 Beta | Functional, needs more real-world testing |
| **S3 Backend** | 🟡 Beta | Works well, multipart upload is newer |
| **SMB Backend** | 🟡 Beta | v0.11.0 upgrade complete, ready for integration testing |
| **Azure Blob Backend** | 🟡 Beta | Production-ready using object_store crate, newly added in v0.6.0 |
| **GCS Backend** | 🟡 Beta | Production-ready using object_store crate, newly added in v0.6.0 |
| **Delta Detection (V1)** | 🟡 Beta | rsync-style algorithm, tested but newer |
| **V2 Architecture (CDC)** | 🔴 Alpha | Content-defined chunking, introduced in v0.5.0 |
| **Semantic Replication** | 🔴 Alpha | Priority-based transfers, introduced in v0.5.0 |
| **Global Deduplication (V3)** | 🟡 Beta | High-cardinality Universe index, v2.1 scalability upgrade |
| **Disk Guardian** | 🟡 Beta | Pre-flight checks, works well but newer |
| **Filter System** | 🟡 Beta | Glob/regex filters, functional but newer |
| **Metadata Preservation** | 🟡 Beta | Works well, extended attributes are platform-specific |
| **Config Optimizer** | 🟡 Beta | Config validation with active probing |
| **Init Wizard** | 🟡 Beta | Interactive setup with `orbit init` (v0.7.0) |
| **Active Environment Probing** | 🟡 Beta | Auto-tuning based on hardware/destination (v0.7.0) |
| **Manifest System** | 🟡 Beta | File tracking and verification |
| **Progress/Bandwidth Limiting** | 🟡 Beta | Recently integrated across all modes |
| **Audit Logging** | 🟡 Beta | Structured telemetry, needs more use |
| **Sparse File Handling** | 🟡 Beta | Zero-chunk detection during CDC, hole-aware writes |
| **Hardlink Preservation** | 🟡 Beta | Inode tracking on Unix/Windows, `--preserve-hardlinks` flag |
| **In-Place Updates** | 🟡 Beta | Reflink/journaled/unsafe safety tiers, `--inplace` |
| **Link-Dest++ (Incremental Backup)** | 🔴 Alpha | Chunk-level reference hardlinking, `--link-dest` |
| **Transfer Journal (Batch Mode)** | 🔴 Alpha | Content-addressed operation journal, `--write-batch` / `--read-batch` |
| **Backpressure** | 🔴 Alpha | Dual-threshold flow control (object count + byte size) |
| **Penalization** | 🔴 Alpha | Exponential backoff deprioritization of failed items |
| **Dead-Letter Queue** | 🔴 Alpha | Bounded quarantine for permanently failed items |
| **Health Monitor** | 🔴 Alpha | Continuous mid-transfer health checks with advisories |
| **Ref-Counted GC** | 🔴 Alpha | WAL-gated garbage collection for shared chunks |
| **Container Packing** | 🔴 Alpha | Chunk packing into `.orbitpak` files to reduce inode pressure |
| **Typed Provenance** | 🔴 Alpha | Structured event taxonomy for audit lineage queries |
| **Composable Prioritizers** | 🔴 Alpha | Chainable sort criteria for transfer scheduling |

**Legend:**
- 🟢 **Stable**: Production-ready with extensive testing
- 🟡 **Beta**: Functional and tested, but needs more real-world validation
- 🔴 **Alpha**: Experimental, expect changes and potential issues

---

## 🔑 Key Features

### 🔄 Error Handling & Retries: Never Give Up

**NEW in v0.4.1!** Intelligent error handling with retry logic and comprehensive diagnostics.

**Features:**
- **Smart Retry Logic** — Exponential backoff with jitter to avoid thundering herd
- **Error Classification** — Distinguishes transient (retry-worthy) from fatal errors
- **Flexible Error Modes** — Abort, Skip, or Partial (keep incomplete files for resume)
- **Default Statistics Tracking** — Retry metrics (attempts, successes, failures) are collected and emitted automatically during copy operations
- **Structured Logging** — Tracing integration for detailed diagnostics
- **Resilient Sync Verification** — Detects source changes during copy and retries or fails safely

**Default Retry Metrics:**

Retry metrics are now collected and emitted by default for all `copy_file` operations, enhancing observability for data migration, transport, and storage workflows. When retries or failures occur, you'll see output like:

```
[orbit] Retry metrics: 2 retries, 1 successful, 0 failed, 0 skipped
```

Control emission with the `ORBIT_STATS` environment variable:
- `ORBIT_STATS=off` — Disable default emission (for high-volume transfers)
- `ORBIT_STATS=verbose` — Always emit, even for successful operations with no retries

**Error Modes:**
- **Abort** (default) — Stop on first error for maximum safety
- **Skip** — Skip failed files, continue with remaining files
- **Partial** — Keep partial files and retry, perfect for unstable networks

**Smart Retry Logic:**
- ⚡ **Permanent errors fail fast** — `PermissionDenied`, `AlreadyExists`, `Compression`, `Decompression` skip retries entirely
- 🔄 **Transient errors retry** — `TimedOut`, `ConnectionRefused`, `Protocol` use full exponential backoff
- 🎯 **Intelligent classification** — Allow-list approach ensures only truly transient errors are retried

```bash
# Resilient transfer with retries and logging
orbit --source /data --dest /backup --recursive \
      --retry-attempts 5 \
      --exponential-backoff \
      --error-mode partial \
      --log-level debug \
      --log /var/log/orbit.log

# Quick skip mode for batch operations
orbit -s /source -d /dest -R \
      --error-mode skip \
      --verbose

# Disable stats emission for high-volume batch transfers
ORBIT_STATS=off orbit --source /data --dest /backup --recursive
```

**Programmatic Statistics Tracking:**

For aggregated metrics across batch operations, pass a custom `OperationStats` instance:

```rust
use orbit::{CopyConfig, OperationStats, copy_file_with_stats};

// For aggregated stats across multiple files:
let stats = OperationStats::new();
for file in &files {
    copy_file_with_stats(&file.src, &file.dest, &config, Some(&stats))?;
}
stats.emit(); // Emit once after all operations

// Get detailed snapshot for programmatic access
let snapshot = stats.snapshot();
println!("Success rate: {:.1}%", snapshot.success_rate());
println!("Total retries: {}", snapshot.total_retries);
```

**Error Categories Tracked:**
- Validation (path errors)
- I/O operations
- Network/protocol issues
- Resource constraints (disk, memory)
- Data integrity (checksums)
- And 11 more categories for comprehensive diagnostics

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

📖 **Full Documentation:** See [`docs/DISK_GUARDIAN.md`](docs/DISK_GUARDIAN.md)

---

### 🛰️ Config Optimizer

Automatic configuration validation and optimization that ensures safe, performant transfers, with active hardware and destination detection.

**What It Does:**
The Config Optimizer acts as an intelligent pre-processor, analyzing your configuration for logical conflicts and automatically resolving them before execution begins. It actively probes your system environment (CPU, RAM, I/O speed, destination type) and auto-tunes settings for optimal performance.

**Key Benefits:**
- 🔒 **Safety First** — Prevents data corruption from incompatible flag combinations
- ⚡ **Performance Optimization** — Automatically selects the fastest valid strategy
- 🧠 **Active Probing** — Detects hardware, I/O speed, and destination type (v0.7.0)
- 🎯 **Auto-Tuning** — Optimizes for SMB, cloud storage, slow I/O, low memory (v0.7.0)
- 🎓 **Educational** — Explains why configurations were changed
- 🤖 **Automatic** — No manual debugging of conflicting flags

**Example Output:**
```
┌── 🛰️  Orbit Config Optimizer ──────────────────────┐
│ 🚀 Strategy: Disabling zero-copy to allow streaming checksum verification
│ 🛡️  Safety: Disabling resume capability to prevent compressed stream corruption
│ 🔧 Network: Detected SMB destination. Enabling resume for reliability.
│ 🔧 Performance: Detected slow I/O (45.2 MB/s) with 16 cores. Enabling Zstd:3.
└────────────────────────────────────────────────────┘
```

**Implemented Rules:**

| Rule | Conflict | Resolution | Icon |
|------|----------|------------|------|
| **Hardware** | Zero-copy on unsupported OS | Disable zero-copy | ⚠️ |
| **Strategy** | Zero-copy + Checksum | Disable zero-copy (streaming is faster) | 🚀 |
| **Integrity** | Resume + Checksum | Disable checksum (can't verify partial file) | 🛡️ |
| **Safety** | Resume + Compression | Disable resume (can't append to streams) | 🛡️ |
| **Precision** | Zero-copy + Resume | Disable zero-copy (need byte-level seeking) | 🚀 |
| **Visibility** | Manifest + Zero-copy | Disable zero-copy (need content inspection) | 🚀 |
| **Logic** | Delta + Zero-copy | Disable zero-copy (need patch logic) | 🚀 |
| **Control** | macOS + Bandwidth + Zero-copy | Disable zero-copy (can't throttle fcopyfile) | ⚠️ |
| **UX** | Parallel + Progress bars | Info notice (visual artifacts possible) | ℹ️ |
| **Performance** | Sync + Checksum mode | Info notice (forces dual reads) | ℹ️ |
| **Physics** | Compression + Encryption | Placeholder (encrypted data won't compress) | 🚀 |
| **🆕 Network Auto-Tune** | SMB/NFS destination | Enable resume + increase retries | 🔧 |
| **🆕 CPU/IO Optimization** | ≥8 cores + <50 MB/s I/O | Enable Zstd:3 compression | 🔧 |
| **🆕 Low Memory** | <1GB RAM + >4 parallel | Reduce to 2 parallel operations | 🔧 |
| **🆕 Cloud Storage** | S3/Azure/GCS destination | Enable compression + backoff | 🔧 |
| **🆕 Local Worker Opt.** | >8 cores + parallel=0 | Set workers to cores/2 | 🔧 |
| **🆕 Fast I/O Chunks** | >500 MB/s + chunk≤1MB | Increase chunk size to 4MB | 🔧 |

**Philosophy:**
> Users express **intent**. Orbit ensures **technical correctness**.

Rather than failing with cryptic errors, Orbit understands what you're trying to achieve and automatically adjusts settings to make it work safely and efficiently.

**Programmatic API:**
```rust
use orbit::core::guidance::ConfigOptimizer;

let mut config = CopyConfig::default();
config.use_zero_copy = true;
config.verify_checksum = true;

// Run optimization pass
let optimized = ConfigOptimizer::optimize(config)?;

// Display notices
for notice in &optimized.notices {
    println!("{}", notice);
}

// Use optimized config
copy_file(&source, &dest, &optimized.final_config)?;
```

📖 **Full Documentation:** See [`docs/architecture/GUIDANCE_SYSTEM.md`](docs/architecture/GUIDANCE_SYSTEM.md)

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

---

### 🔄 Data Flow Patterns

Production-grade modules for reliable, observable data transfer pipelines.

```text
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  Ingest      │────>│  Transfer    │────>│  Delivery   │
│              │     │  Pipeline    │     │             │
│ Prioritizer  │     │ Backpressure │     │ Container   │
│ Provenance   │     │ Penalization │     │ Packing     │
└─────────────┘     │ Health Mon.  │     └─────────────┘
                    │ Dead-Letter  │
                    │ Ref-Count GC │
                    └──────────────┘
```

**Flow Control & Resilience**:
- **Backpressure** — Dual-threshold guards (object count + byte size) with apply/release semantics
- **Penalization** — Exponential backoff deprioritization with configurable cap and decay
- **Dead-Letter Queue** — Bounded quarantine with reason tracking, retry, and drain support
- **Health Monitor** — Continuous mid-transfer health checks emitting typed advisories
- **Ref-Counted GC** — WAL-gated garbage collection preventing premature deletion of shared chunks

**Scheduling & Intelligence** (`core-semantic`):
- **Composable Prioritizers** — Chainable sort criteria (size, age, priority, name) with `then()` composition

**Observability & Lifecycle**:
- **Typed Provenance** (`core-audit`) — Structured event taxonomy for lineage queries
- **Container Packing** (`core-starmap`) — Chunk packing into `.orbitpak` files to reduce inode pressure

```rust
use orbit::manifests::{BeaconBuilder};

// Record structured audit beacon for a completed job
let beacon = BeaconBuilder::new("job-123", "sha256:abc123")
    .with_total_bytes(1_048_576)
    .with_file_count(42)
    .build();
```

---

### 🏷️ Metadata Preservation & Transformation

**NEW in v0.4.1!** Comprehensive file metadata preservation with transformation capabilities for cross-platform transfers and reproducible builds.

**Default Metadata Support:**
- **Timestamps** — Access time (atime), modification time (mtime), creation time (ctime)
- **Permissions** — Unix mode bits, Windows file attributes

**Extended Metadata Support** (requires `extended-metadata` feature):
- **Ownership** — User ID (UID) and Group ID (GID) on Unix systems
- **Extended Attributes (xattrs)** — User-defined metadata on supported filesystems

To enable extended metadata preservation:
```toml
[dependencies]
orbit = { version = "0.6.0", features = ["extended-metadata"] }
```

> **Note:** Extended attributes have platform limitations (e.g., partial or no support on Windows, requires compatible filesystem on Unix). Ownership preservation typically requires root/administrator privileges.

**Features:**
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
- `times` — Access and modification timestamps (default)
- `perms` — Unix permissions (mode bits) (default)
- `owners` — User and group ownership (UID/GID) (requires privileges)
- `xattrs` — Extended attributes (requires `extended-metadata` feature, Unix-like systems only)
- `all` — Preserve everything (full support requires `extended-metadata` feature)

**Transformation Options:**
- `rename:pattern=replacement` — Regex-based path renaming
- `case:lower|upper|title` — Filename case conversion
- `strip:xattrs|ownership|permissions` — Remove metadata
- `normalize:timestamps` — Set all timestamps to epoch (reproducible builds)

📖 **API Documentation:** See `src/core/file_metadata.rs`, `src/core/transform.rs`, and `src/core/metadata_ops.rs`

---

### 🔄 Delta Detection: Efficient Transfers

**NEW in v0.4.1!** rsync-inspired delta algorithm that minimizes bandwidth by transferring only changed blocks.

**Orbit V2 Architecture** 🚀

**UPGRADED in v2.1: Universe Scalability** 🌌
- **High-Cardinality Performance** — Eliminated O(N²) write amplification bottleneck in Universe index
  - **Multimap Architecture**: Uses `redb::MultimapTableDefinition` for discrete location entries
  - **O(log N) Inserts**: Constant-time performance regardless of duplicate count (was O(N) in V2)
  - **Streaming Iteration**: O(1) memory usage via `scan_chunk()` callback API
  - **Production Scale**: Handles billions of chunks with millions of duplicates per chunk
  - **Benchmark**: 20,000 duplicates - last batch 0.55x faster than first (V2 would be ~200x slower)
  - **See:** [SCALABILITY_SPEC.md](docs/architecture/SCALABILITY_SPEC.md) for technical details

- **Content-Defined Chunking (CDC)** — Gear Hash CDC solves the "shift problem" with 99.1% chunk preservation
- **Semantic Prioritization** — Intelligent file classification with 4-tier priority system for optimized disaster recovery
  - **Critical(0)**: Configs (.toml, .json, .yaml, .lock) → AtomicReplace strategy
  - **High(10)**: WAL files (pg_wal/*, *.wal, *.binlog) → AppendOnly strategy
  - **Normal(50)**: Source code, documents → ContentDefined strategy
  - **Low(100)**: Media, archives, disk images (.iso, .zip, .mp4) → ContentDefined strategy
  - **Extensible**: Custom adapters via `SemanticAdapter` trait
- **Global Deduplication** — Identical chunks stored once, regardless of file location
- **Universe Map** — Repository-wide content-addressed index for cross-file deduplication
- **100% Rename Detection** — Renaming a file results in 0 bytes transferred
- **Smart Sync Mode** — Priority-ordered transfers using BinaryHeap for semantic-aware replication
  - Automatically detects when `check_mode_str = "smart"` is configured
  - 3-phase algorithm: Scan → Analyze → Queue → Execute in priority order
  - Ensures critical files (configs) are transferred before low-priority files (backups, media)
  - ~60% faster disaster recovery via semantic prioritization
- **Persistent Universe** — ACID-compliant embedded database for chunk index persistence (Stage 4)
  - Uses redb for zero-copy, memory-mapped storage with full ACID guarantees
  - Data survives application restarts (verified with drop & re-open tests)
  - ChunkLocation tracking: Full path + offset + length for precise deduplication
  - Atomic chunk claims (`try_claim_chunk`) avoid duplicate transfers under concurrency
  - 4/4 persistence tests passing
- **See:** [ORBIT_V2_ARCHITECTURE.md](ORBIT_V2_ARCHITECTURE.md) for complete details

**V2 CDC Features:**
- **Gear Hash Rolling Hash** — 256-entry lookup table for fast boundary detection (~2GB/s per core)
- **Shift-Resilient** — Inserting 1 byte preserves 99.1% of chunks (vs 0% with fixed-size blocks)
- **Variable Chunks** — 8KB min, 64KB avg, 256KB max (configurable)
- **BLAKE3 Hashing** — Cryptographically secure content identification
- **Iterator-Based API** — Memory-efficient streaming with `ChunkStream<R: Read>`
- **Threshold-Based Cuts** — Robust chunking across different data patterns

**Features:**
- **4 Detection Modes** — ModTime (fast), Size, Checksum (BLAKE3), Delta (block-based)
- **Rolling Checksum** — Gear64 (default, 64-bit) or Adler-32 (legacy, 32-bit)
- **Slice & Emit Buffering** — Non-matching spans flush as slices (no per-byte allocations) for much faster 0% similarity workloads
- **Parallel Hashing** — Rayon-based concurrent block processing
- **Smart Fallback** — Automatic full copy for incompatible files
- **80-99% Savings** — For files with minor changes
- **Configurable Blocks** — 64KB to 4MB block sizes
- **Resume Handling** — Partial manifest support for interrupted transfers (NEW!)

**Use Cases:**
- ✅ Daily database backups (90-95% savings)
- ✅ VM image updates (85-95% savings)
- ✅ Large file synchronization over slow links
- ✅ Log file rotation (95-99% savings for append-only)
- ✅ Fault-tolerant transfers over unreliable networks (NEW!)

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

**Delta Resume Handling (NEW!):**

Delta transfers now support resume capability via partial manifests for fault-tolerant operations. On failure, a `{dest}.delta.partial.json` manifest is saved; subsequent calls will resume if possible.

```rust
use orbit::{CopyConfig, copy_file};
use orbit::core::delta::CheckMode;

let mut config = CopyConfig::default();
config.check_mode = CheckMode::Delta;
config.delta_resume_enabled = true;  // Enabled by default
config.delta_chunk_size = 1024 * 1024;  // 1MB chunks

// Attempts delta with resume; falls back on non-resumable errors
copy_file(&src, &dest, &config)?;
```

For large data migrations, enable retries at higher levels to leverage resumes. Disable resume with `config.delta_resume_enabled = false` if not needed.

**Manifest Generation (NEW!):**

When `update_manifest` is enabled and a `manifest_path` is provided, Orbit will emit or update a manifest database post-transfer, tracking file metadata and checksums. Use `ignore_existing` to skip updates if the manifest already exists.

```rust
use orbit::core::delta::{DeltaConfig, copy_with_delta_fallback, ManifestDb};
use std::path::PathBuf;

let mut config = DeltaConfig::default();
config.update_manifest = true;
config.manifest_path = Some(PathBuf::from("transfer_manifest.json"));
config.ignore_existing = false;  // Update existing manifest (default)

// Delta transfer with automatic manifest update
let (stats, checksum) = copy_with_delta_fallback(&src, &dest, &config)?;

if stats.manifest_updated {
    println!("Manifest updated with checksum: {:?}", checksum);
}

// Load manifest for custom analytics or auditing
let manifest = ManifestDb::load(&PathBuf::from("transfer_manifest.json"))?;
for (path, entry) in manifest.iter() {
    println!("{}: {} bytes, delta_used={}", path.display(), entry.size, entry.delta_used);
}
```

**Manifest Features:**
- **Automatic Updates** — Manifests are updated after successful delta or fallback transfers
- **Entry Tracking** — Each file entry includes source path, destination path, checksum, size, modification time, and delta statistics
- **JSON Format** — Human-readable and machine-parseable manifest format
- **Validation** — `config.validate_manifest()` ensures proper configuration before transfer

**Performance:**
- 1GB file with 5% changes: **10x faster** (3s vs 30s), **95% less data** (50MB vs 1GB)
- Identical files: **99% savings** with minimal CPU overhead

📖 **Full Documentation:** See [`docs/DELTA_DETECTION_GUIDE.md`](docs/DELTA_DETECTION_GUIDE.md) and [`docs/DELTA_QUICKSTART.md`](docs/DELTA_QUICKSTART.md)

---

### 📊 Progress Reporting & Operational Controls

**NEW in v0.4.1!** Production-grade progress tracking, simulation mode, bandwidth management, and concurrency control for enterprise workflows.

**Features:**
- **Enhanced Progress Bars** — Multi-transfer tracking with `indicatif`, real-time ETA and speed
- **Dry-Run Mode** — Safe simulation and planning before actual transfers
- **Bandwidth Limiting** — Token bucket rate limiting (`governor`) **fully integrated** across all copy modes ⭐
- **Concurrency Control** — Semaphore-based parallel operation management **fully integrated** ⭐
- **Verbosity Levels** — Detailed logging with structured tracing
- **Multi-Transfer Support** — Concurrent progress bars for parallel operations
- **Zero New Dependencies** — Leveraged existing infrastructure

**What's New:**
- ✅ **BandwidthLimiter** now integrated into buffered, LZ4, Zstd, and zero-copy operations
- ✅ **ConcurrencyLimiter** now integrated into directory copy with RAII permits
- ✅ **Zero-copy** now supports bandwidth limiting (Linux/macOS with 1MB chunks)
- ✅ **Throttle logging** for monitoring rate limit events (debug level)
- ✅ **Load tests** verify accuracy of rate limiting and concurrency control

**Use Cases:**
- ✅ Preview large migrations before executing (dry-run)
- ✅ **Limit bandwidth to avoid network saturation or cloud costs**
- ✅ **Control resource usage with fine-grained concurrency limits**
- ✅ Monitor complex parallel transfers with real-time progress
- ✅ Test filter rules and transformations safely

```bash
# Preview transfer with dry-run
orbit -s /data -d /backup -R --dry-run --verbose
# Output:
# [DRY-RUN] Would copy: /data/file1.txt -> /backup/file1.txt (1024 bytes)
# [DRY-RUN] Would skip: /data/file2.txt - already exists
#
# Dry-Run Summary:
#   Files to copy:    5
#   Files to skip:    2
#   Total data size:  10.5 MB

# Limit bandwidth to 10 MB/s with 4 concurrent transfers
orbit -s /large/dataset -d /backup \
  --recursive \
  --max-bandwidth 10 \
  --parallel 4 \
  --show-progress

# Bandwidth limiting now works with zero-copy!
orbit -s /large/file.bin -d /backup/file.bin --max-bandwidth 10

# Auto-detect optimal concurrency (2x CPU cores, capped at 16)
# Note: If CPU detection fails (restricted containers/cgroups), defaults to 1 thread with warning
orbit -s /data -d /backup -R --parallel 0

# Full-featured production transfer
orbit -s /production/data -d /backup/location \
  --recursive \
  --max-bandwidth 10 \
  --parallel 8 \
  --show-progress \
  --resume \
  --retry-attempts 5 \
  --exponential-backoff \
  --verbose
```

**Progress Features:**
- Real-time transfer speed (MB/s)
- Accurate ETA calculations
- Per-file progress tracking
- Support for concurrent transfers
- Terminal-friendly progress bars

**Bandwidth Limiting:**
- Token bucket algorithm for smooth throttling (`governor` crate)
- Configurable MB/s limits via `--max-bandwidth`
- Zero overhead when disabled (0 = unlimited)
- **Integrated across ALL copy modes**: buffered, LZ4, Zstd, zero-copy (Linux/macOS)
- Thread-safe and cloneable
- Throttle event logging (debug level)
- 1MB chunks for precise control in zero-copy mode

**Concurrency Control:**
- Auto-detection based on CPU cores (2× CPU count, max 16)
- Safe fallback: Defaults to 1 thread if CPU detection fails (restricted environments)
- Configurable maximum parallel operations via `--parallel`
- **Integrated into directory copy** with per-file permit acquisition
- RAII-based permit management (automatic cleanup via Drop)
- Optimal for I/O-bound operations
- See [Performance Guide](docs/guides/PERFORMANCE.md) for detailed concurrency tuning
- Works seamlessly with rayon thread pools

**Dry-Run Capabilities:**
- Simulate all operations (copy, update, skip, delete, mkdir)
- Detailed logging via tracing framework
- Summary statistics with total data size
- Works with all other features (filters, transformations, etc.)

**Technical Details:**
- **Implementation**: Integrated existing `BandwidthLimiter` and `ConcurrencyLimiter` classes
- **Testing**: 177 tests passed, 3 timing-sensitive load tests available with `--ignored`
- **Monitoring**: Structured logging via `tracing` with debug-level throttle events
- **Compatibility**: Zero impact on existing functionality, all tests passing

📖 **Full Documentation:** See [`docs/PROGRESS_AND_CONCURRENCY.md`](docs/PROGRESS_AND_CONCURRENCY.md) ⭐ **NEW!**

---

### 🎯 Inclusion/Exclusion Filters: Selective Transfers

**NEW in v0.4.1!** Powerful rsync/rclone-inspired filter system for selective file processing with glob patterns, regex, and exact path matching.

**Features:**
- **Multiple Pattern Types** — Glob (`*.txt`, `target/**`), Regex (`^src/.*\.rs$`), Exact paths
- **Include/Exclude Rules** — Both supported with first-match-wins semantics
- **Filter Files** — Load reusable filter rules from `.orbitfilter` files
- **Early Directory Pruning** — Skip entire directory trees efficiently
- **Cross-Platform** — Consistent path matching across Windows, macOS, Linux
- **Dry-Run Visibility** — See what would be filtered before actual transfer
- **Negation Support** — Invert filter actions with `!` prefix

**Use Cases:**
- ✅ Selective backups (exclude build artifacts, logs, temp files)
- ✅ Source code transfers (include only source files, exclude dependencies)
- ✅ Clean migrations (exclude platform-specific files)
- ✅ Compliance-aware transfers (exclude sensitive files by pattern)

```bash
# Basic exclude patterns
orbit -s /project -d /backup -R \
  --exclude="*.tmp" \
  --exclude="target/**" \
  --exclude="node_modules/**"

# Include overrides exclude (higher priority)
orbit -s /logs -d /archive -R \
  --include="important.log" \
  --exclude="*.log"

# Use regex for complex patterns
orbit -s /code -d /backup -R \
  --exclude="regex:^tests/.*_test\.rs$" \
  --include="**/*.rs"

# Load filters from file
orbit -s /data -d /backup -R --filter-from=backup.orbitfilter

# Combine with other features
orbit -s /source -d /dest -R \
  --include="*.rs" \
  --exclude="target/**" \
  --check delta \
  --compress zstd:3 \
  --dry-run
```

**Filter File Example (`backup.orbitfilter`):**
```text
# Include source files (higher priority - checked first)
+ **/*.rs
+ **/*.toml
+ **/*.md

# Exclude build artifacts
- target/**
- build/**
- *.o

# Exclude logs and temp files
- *.log
- *.tmp

# Regex for test files
- regex: ^tests/.*_test\.rs$

# Exact path inclusion
include path: Cargo.lock
```

**Pattern Priority:**
1. Include patterns from `--include` (highest)
2. Exclude patterns from `--exclude`
3. Rules from filter file (in file order)
4. Default: Include (if no rules match)

**Example Filter File:** [`examples/filters/example.orbitfilter`](examples/filters/example.orbitfilter)

📖 **Full Documentation:** See [`docs/FILTER_SYSTEM.md`](docs/FILTER_SYSTEM.md)

---

### 🌐 Protocol Support

Orbit supports multiple storage backends through a **unified backend abstraction layer** that provides a consistent async API across all storage types.

| Protocol | Status | Feature Flag | Description |
|----------|--------|--------------|-------------|
| 🗂️ **Local** | 🟢 Stable | Built-in | Local filesystem with zero-copy optimization |
| 🔐 **SSH/SFTP** | 🟡 Beta | `ssh-backend` | Remote filesystem access via SSH/SFTP with async I/O |
| ☁️ **S3** | 🟡 Beta | `s3-native` | Amazon S3 and compatible object storage (MinIO, LocalStack) |
| 🌐 **SMB/CIFS** | 🟡 Beta | `smb-native` | Native SMB2/3 client (pure Rust, v0.11.1, ready for testing) |
| ☁️ **Azure Blob** | 🟡 Beta | `azure-native` | Microsoft Azure Blob Storage (using object_store crate) |
| ☁️ **GCS** | 🟡 Beta | `gcs-native` | Google Cloud Storage (using object_store crate) |
| 🌐 **WebDAV** | 🚧 Planned | - | WebDAV protocol support |

#### 🆕 Unified Backend Abstraction (v0.5.0+ - Streaming API)

**NEW!** Write once, run on any storage backend. The backend abstraction provides a consistent async API with **streaming I/O** for memory-efficient large file transfers:

```rust
use orbit::backend::{Backend, LocalBackend, SshBackend, S3Backend, SmbBackend, AzureBackend, GcsBackend, SmbConfig, AzureConfig, GcsConfig};
use tokio::fs::File;
use tokio::io::AsyncRead;
use futures::StreamExt;

// All backends implement the same trait with streaming support
async fn copy_file<B: Backend>(backend: &B, src: &Path, dest: &Path) -> Result<()> {
    // Stream file directly from disk - no memory buffering!
    let file = File::open(src).await?;
    let metadata = file.metadata().await?;
    let reader: Box<dyn AsyncRead + Unpin + Send> = Box::new(file);

    backend.write(dest, reader, Some(metadata.len()), Default::default()).await?;
    Ok(())
}

// List directories with streaming (constant memory for millions of entries)
async fn list_large_directory<B: Backend>(backend: &B, path: &Path) -> Result<()> {
    let mut stream = backend.list(path, ListOptions::recursive()).await?;

    while let Some(entry) = stream.next().await {
        let entry = entry?;
        println!("{}: {} bytes", entry.path.display(), entry.metadata.size);
    }
    Ok(())
}

// Works with any backend
let local = LocalBackend::new();
let ssh = SshBackend::connect(config).await?;
let s3 = S3Backend::new(s3_config).await?;
let smb = SmbBackend::new(SmbConfig::new("server", "share")
    .with_username("user")
    .with_password("pass")).await?;
let azure = AzureBackend::new("my-container").await?;
let gcs = GcsBackend::new("my-bucket").await?;
```

**Features:**
- ✅ **URI-based configuration**: `ssh://user@host/path`, `s3://bucket/key`, `smb://user@server/share/path`, `azblob://container/path`, `gs://bucket/path`, etc.
- ✅ **Streaming I/O**: Upload files up to **5TB** to S3 with ~200MB RAM
- ✅ **Constant Memory Listing**: List millions of S3 objects with ~10MB RAM
- ✅ **Automatic Multipart Upload**: S3 files ≥5MB use efficient chunked transfers
- ✅ **Optimized Download**: Sliding window concurrency for 30-50% faster S3 downloads
- ✅ **Metadata operations**: Set permissions, timestamps, xattrs, ownership
- ✅ **Extensibility**: Plugin system for custom backends
- ✅ **Type-safe**: Strong typing with comprehensive error handling
- ✅ **Security**: Built-in secure credential handling

📖 **Full Guide:** [docs/guides/BACKEND_GUIDE.md](docs/guides/BACKEND_GUIDE.md)
📖 **Migration Guide:** [docs/guides/BACKEND_STREAMING_GUIDE.md](docs/guides/BACKEND_STREAMING_GUIDE.md) ⭐ **NEW!**

#### SSH/SFTP Remote Access

Transfer files securely over SSH/SFTP with async implementation:

```bash
# Download from SSH server using agent authentication
orbit --source ssh://user@example.com/remote/file.txt --dest ./file.txt

# Upload to SFTP server (SSH and SFTP URIs are equivalent)
orbit --source ./local-file.txt --dest sftp://example.com/upload/file.txt

# Recursive directory sync with compression
orbit --source /local/photos --dest ssh://backup.server.com/photos/ \
  --mode sync --compress zstd:3 --recursive

# Download with resume support for unreliable connections
orbit --source ssh://server.com/large-file.iso --dest ./large-file.iso \
  --resume --retry-attempts 10
```

**SSH/SFTP Features:**
- ✅ Pure Rust using libssh2 (battle-tested SSH library)
- ✅ Async I/O with tokio::task::spawn_blocking (non-blocking operations)
- ✅ Three authentication methods (SSH Agent, Private Key, Password)
- ✅ Secure credential handling with `secrecy` crate
- ✅ Connection timeout configuration
- ✅ Automatic SSH handshake and session management
- ✅ Full Backend trait implementation (stat, list, read, write, delete, mkdir, rename)
- ✅ Recursive directory operations
- ✅ Optional SSH compression for text files
- ✅ Compatible with all SFTP servers (OpenSSH, etc.)
- ✅ Resume support with checkpoint recovery
- ✅ Integration with manifest system

**Authentication Priority:**
1. **SSH Agent** (Default) — Most secure, no credentials in command history
2. **Private Key File** — Supports passphrase-protected keys
3. **Password** — Use only when key-based auth unavailable

📖 **Full Documentation:** See [`docs/guides/PROTOCOL_GUIDE.md`](docs/guides/PROTOCOL_GUIDE.md#-ssh--sftp-production-ready)

#### S3 Cloud Storage (Streaming Optimized)

Transfer files seamlessly to AWS S3 and S3-compatible storage services with **streaming I/O** and advanced features:

```bash
# Upload to S3 (streams directly from disk, no memory buffering!)
orbit --source /local/dataset.tar.gz --dest s3://my-bucket/backups/dataset.tar.gz

# Download from S3 (optimized sliding window concurrency)
orbit --source s3://my-bucket/data/report.pdf --dest ./report.pdf

# Sync directory to S3 with compression
orbit --source /local/photos --dest s3://my-bucket/photos/ \
  --mode sync --compress zstd:3 --recursive

# Use with MinIO
export S3_ENDPOINT=http://localhost:9000
orbit --source file.txt --dest s3://my-bucket/file.txt
```

**S3 Features:**
- ✅ Pure Rust (no AWS CLI dependency)
- ✅ **Streaming multipart upload** - Files ≥5MB automatically use multipart with **5TB max file size**
- ✅ **Constant memory usage** - ~200MB RAM for any file size upload/download
- ✅ **Optimized downloads** - Sliding window concurrency for 30-50% faster transfers
- ✅ **Lazy S3 pagination** - List millions of objects with ~10MB RAM
- ✅ Resumable transfers with checkpoint support
- ✅ Parallel chunk transfers (configurable)
- ✅ All storage classes (Standard, IA, Glacier, etc.)
- ✅ Server-side encryption (AES-256, AWS KMS)
- ✅ S3-compatible services (MinIO, LocalStack, DigitalOcean Spaces)
- ✅ Flexible authentication (env vars, credentials file, IAM roles)
- ✅ Full integration with manifest system
- ✅ Object versioning and lifecycle management
- ✅ Batch operations with rate limiting
- ✅ **Resilience patterns** — Circuit breaker and rate limiting

📖 **Full Documentation:** See [`docs/guides/S3_USER_GUIDE.md`](docs/guides/S3_USER_GUIDE.md)
📖 **Streaming Guide:** See [`docs/guides/BACKEND_STREAMING_GUIDE.md`](docs/guides/BACKEND_STREAMING_GUIDE.md) ⭐ **NEW!**

#### Azure Blob Storage

**NEW in v0.6.0**: Production-ready Azure Blob Storage backend using the industry-standard `object_store` crate.

Transfer files seamlessly to Microsoft Azure Blob Storage with streaming I/O:

```bash
# Upload to Azure Blob Storage
orbit --source /local/dataset.tar.gz --dest azblob://mycontainer/backups/dataset.tar.gz

# Download from Azure
orbit --source azure://mycontainer/data/report.pdf --dest ./report.pdf

# Sync directory to Azure with compression
orbit --source /local/photos --dest azblob://photos-container/backup/ \
  --mode sync --compress zstd:3 --recursive

# Test with Azurite (Azure Storage Emulator)
export AZURE_STORAGE_ACCOUNT="devstoreaccount1"
export AZURE_STORAGE_KEY="Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw=="
orbit --source file.txt --dest azblob://testcontainer/file.txt
```

**Azure Features:**
- ✅ Pure Rust using `object_store` crate (used by Apache Arrow DataFusion)
- ✅ **Unified cloud API** - Same crate powers S3, Azure, and GCS backends
- ✅ **Streaming I/O** - Memory-efficient transfers for large files
- ✅ **Environment variable authentication** - Works with AZURE_STORAGE_ACCOUNT + AZURE_STORAGE_KEY
- ✅ **Connection string support** - Compatible with AZURE_STORAGE_CONNECTION_STRING
- ✅ **Azurite compatible** - Test locally with Azure Storage Emulator
- ✅ **URI schemes** - Both `azblob://` and `azure://` supported
- ✅ **Full Backend trait** - stat, list, read, write, delete, mkdir, rename, exists
- ✅ **Prefix support** - Virtual directory isolation within containers
- ✅ **Strong consistency** - Azure Blob Storage guarantees
- ✅ **Production-ready** - 33% less code than Azure SDK implementation

📖 **Implementation Status:** See [`docs/project-status/AZURE_IMPLEMENTATION_STATUS.md`](docs/project-status/AZURE_IMPLEMENTATION_STATUS.md)

#### Google Cloud Storage

**NEW in v0.6.0**: Production-ready Google Cloud Storage backend using the industry-standard `object_store` crate.

Transfer files seamlessly to Google Cloud Storage with streaming I/O:

```bash
# Upload to Google Cloud Storage
orbit --source /local/dataset.tar.gz --dest gs://mybucket/backups/dataset.tar.gz

# Download from GCS
orbit --source gcs://mybucket/data/report.pdf --dest ./report.pdf

# Sync directory to GCS with prefix
orbit --source /local/photos --dest gs://mybucket/archives/photos \
  --mode sync --resume --parallel 8 --recursive
```

**Authentication:**
```bash
# Service account JSON file (recommended)
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json

# Or use service account credentials directly
export GOOGLE_SERVICE_ACCOUNT=myaccount@myproject.iam.gserviceaccount.com
export GOOGLE_SERVICE_ACCOUNT_KEY="-----BEGIN PRIVATE KEY-----\n..."
```

**Features:**
- ✅ **Service account support** - GOOGLE_APPLICATION_CREDENTIALS or direct credentials
- ✅ **Streaming I/O** - Memory-efficient large file transfers
- ✅ **URI schemes** - Both `gs://` and `gcs://` supported
- ✅ **Full Backend trait** - stat, list, read, write, delete, mkdir, rename, exists
- ✅ **Prefix support** - Virtual directory isolation within buckets
- ✅ **Strong consistency** - Google Cloud Storage guarantees
- ✅ **Production-ready** - Using battle-tested object_store crate (same as Azure and S3)

📖 **Full Documentation:** See [`docs/guides/GCS_USER_GUIDE.md`](docs/guides/GCS_USER_GUIDE.md)

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
- **Enforced security policies** (RequireEncryption, SignOnly, Opportunistic)
- Encryption support (AES-128/256-GCM, AES-128/256-CCM)
- Packet signing (HMAC-SHA256, AES-GMAC, AES-CMAC)
- Async/await with Tokio
- Custom port support for non-standard deployments
- Adaptive chunking (256KB-2MB blocks)
- Integration with manifest system

---

### 📊 Audit and Telemetry (V3 Unified Observability)

**NEW in v0.6.0**: Enterprise-grade observability with **cryptographic integrity** and distributed tracing.

Every operation emits structured events for compliance auditing, troubleshooting, and operational monitoring.

#### 🔒 Cryptographic Audit Chaining

Orbit V3 provides **tamper-evident audit logs** using HMAC-SHA256 cryptographic chaining. Any modification, deletion, or reordering of audit events is immediately detectable.

**Enable Secure Audit Logging:**
```bash
# Set HMAC secret key (required for cryptographic chaining)
export ORBIT_AUDIT_SECRET=$(openssl rand -hex 32)

# Enable audit logging with integrity protection
orbit --source /source --dest /dest --audit-log ./audit.jsonl

# Verify log integrity (detects any tampering)
python3 scripts/verify_audit.py audit.jsonl
```

**V3 Event Format (JSONL with HMAC chain):**
```json
{
  "trace_id": "335f8464197139ab59c4494274e55749",
  "span_id": "4a63b017626d3de5",
  "timestamp": "2025-12-18T11:56:41.722338400Z",
  "sequence": 0,
  "integrity_hash": "a70ee3ca57a26eb650d19b4d7ed66d28d3fc187137b8edb182c5ea2d7a8eeee9",
  "payload": {
    "type": "file_start",
    "source": "/data/file.bin",
    "dest": "s3://bucket/backup/file.bin",
    "bytes": 1048576
  }
}
```

#### 🌍 Distributed Tracing (W3C Trace Context)

Orbit supports **W3C Trace Context** for distributed tracing across microservices and remote transfers.

**Enable OpenTelemetry Export:**
```bash
# Export traces to Jaeger/Honeycomb/Datadog
orbit --source /source --dest /dest \
  --audit-log ./audit.jsonl \
  --otel-endpoint http://jaeger:4317
```

**Trace correlation features:**
- **W3C-compliant** trace IDs (32-char hex) and span IDs (16-char hex)
- **Hierarchical correlation** — trace_id → job_id → file_id → span_id
- **Backend instrumentation** — All 45 backend methods emit trace spans (S3, SMB, SSH, local)

#### LLM-Native Debug Logging (Developer Mode)

When you need clean, LLM-friendly logs without audit/HMAC or OTel layers, enable the JSON-only debug mode:

```bash
TEST_LOG=llm-debug RUST_LOG=debug \
  cargo test --test integration_tests -- --nocapture

# Or for normal runs
ORBIT_LOG_MODE=llm-debug RUST_LOG=debug \
  orbit --source /source --dest /dest
```

#### 📊 Prometheus Metrics

**Expose metrics for monitoring:**
```bash
orbit --source /source --dest /dest \
  --audit-log ./audit.jsonl \
  --metrics-port 9090

# Scrape metrics at http://localhost:9090/metrics
curl http://localhost:9090/metrics | grep orbit_
```

**Available metrics:**
- `orbit_transfer_retries_total` — Retry attempts by protocol
- `orbit_backend_latency_seconds` — Backend operation latency (histogram)
- `orbit_audit_integrity_failures_total` — Audit chain breaks (CRITICAL alert)
- `orbit_files_transferred_total` — Successful transfers
- `orbit_bytes_transferred_total` — Total bytes transferred

#### 🛡️ Security & Compliance Features

- **Tamper detection** — Any modification, deletion, insertion, or reordering detected
- **Forensic validation** — Verify chain integrity with `verify_audit.py`
- **Secret management** — HMAC keys via `ORBIT_AUDIT_SECRET` environment variable
- **Monotonic sequencing** — Events are strictly ordered
- **Compliance-ready** — SOC 2, HIPAA, GDPR audit trail support

#### 📖 Documentation

See [docs/observability-v3.md](docs/observability-v3.md) for complete documentation including:
- Configuration guide (environment variables, CLI flags, TOML config)
- Integration with Jaeger, Honeycomb, Datadog, Grafana
- Forensic validation procedures
- Security best practices
- Troubleshooting guide

### 🔧 Advanced Transfer Optimizations

Six rsync-inspired features, reimplemented to leverage Orbit's CDC + Star Map architecture:

```bash
# Sparse files — hole-aware writes for zero-heavy files (VMs, databases)
orbit -s /data/vm.qcow2 -d /backup/ --sparse auto

# Hardlink preservation — detect and recreate hardlink groups
orbit -s /backups/daily/ -d /backups/offsite/ -R --preserve-hardlinks

# In-place updates — modify destination directly (3 safety tiers)
orbit -s /data/large.img -d /backup/large.img --inplace
orbit -s /data/db.mdf -d /backup/db.mdf --inplace --inplace-safety journaled

# Incremental backups — hardlink unchanged files to reference
orbit -s /data/ -d /backups/today/ -R --link-dest /backups/yesterday/

# Batch mode — record once, replay to many destinations
orbit -s /release/ -d /server1/ -R --write-batch update.batch
orbit --read-batch update.batch -d /server2/
```

| Feature | rsync Equivalent | Orbit Improvement |
|---------|-----------------|-------------------|
| `--sparse` | `--sparse` | Zero-cost detection during CDC; works with `--inplace` (rsync can't) |
| `--preserve-hardlinks` | `-H` | Cross-platform (Unix + Windows FFI) |
| `--inplace` | `--inplace` | Reflink/journaled/unsafe safety tiers (rsync has none) |
| `--link-dest` | `--link-dest` | Chunk-level partial reuse vs all-or-nothing per file |
| `--write-batch` | `--write-batch` | Content-addressed journal, portable across different destinations |

**Current limitations:** `--sparse` and `--inplace` are mutually exclusive. Compression is incompatible with `--sparse`/`--inplace` (use `--sparse never` and avoid `--inplace`). Delta transfer (`--check delta`) is disabled when `--sparse` or `--inplace` is enabled (Orbit falls back to a full buffered copy). `--link-dest` currently hardlinks **exact matches** only; partial-chunk delta basis is planned. `--write-batch` records full-file create entries (no delta ops yet) and requires `--mode copy`.

See [Advanced Transfer Features](docs/architecture/ADVANCED_TRANSFER.md) for design details.

---

## 🚀 Quick Start

> **⚠️ Alpha Software:** Remember that Orbit is in active development (v0.6.0). Test thoroughly in non-production environments first, and always maintain backups when working with important data.

### Install

```bash
# From source
git clone https://github.com/saworbit/orbit.git
cd orbit

# Minimal build (local copy only, ~10MB binary) - DEFAULT
cargo build --release

# With network protocols (S3, SMB, SSH)
cargo build --release --features network

# Install to system
sudo cp target/release/orbit /usr/local/bin/

# Or with cargo install
cargo install --path .                    # Minimal
cargo install --path . --features network  # With network
```

> **v0.5+:** Orbit defaults to a minimal build (just local copy with zero-copy optimizations) for fastest compile times and smallest binaries. Network protocols are opt-in via feature flags.

### Feature Flags & Binary Sizes

**v0.5-0.6 Performance Improvements:**
- 🎯 **60% smaller default binary** — Minimal build is ~10MB (was ~50MB)
- ⚡ **50% faster compilation** — Default build in ~60s (was ~120s)
- 🔒 **Reduced attack surface** — Minimal default build
- 🚀 **2x Delta throughput** — Gear64 hash replaces Adler-32 for better collision resistance

| Feature | Description | Binary Size | Default |
|---------|-------------|-------------|---------|
| `zero-copy` | OS-level zero-copy syscalls for maximum speed | +1MB | ✅ Yes |
| `network` | All network protocols (S3, SMB, SSH, Azure, GCS) | +31MB | ❌ No |
| `s3-native` | Amazon S3 and compatible storage | +15MB | ❌ No |
| `smb-native` | Native SMB2/3 network shares | +8MB | ❌ No |
| `ssh-backend` | SSH/SFTP remote access | +5MB | ❌ No |
| `azure-native` | Microsoft Azure Blob Storage | +3MB | ❌ No |
| `gcs-native` | Google Cloud Storage | +3MB | ❌ No |
| `delta-manifest` | SQLite-backed delta persistence | +3MB | ❌ No |
| `extended-metadata` | xattr + ownership (Unix/Linux/macOS only) | +500KB | ❌ No |

```bash
# Minimal: Fast local copies only (~10MB)
cargo build --release
cargo install orbit

# Network: Add S3, SMB, SSH, Azure support (~38MB)
cargo build --release --features network
cargo install orbit --features network

# Size-optimized: Maximum compression
cargo build --profile release-min
```

### First-Time Setup (NEW in v0.7.0!)

**Interactive Configuration Wizard** - Get started in seconds with the new `orbit init` command:

```bash
# Run the interactive setup wizard
orbit init
```

**What it does:**
1. 🔍 **Scans your system** — Detects CPU cores, RAM, and I/O speed
2. 💬 **Asks about your use case** — Backup, Sync, Cloud, or Network
3. ⚙️ **Generates optimal config** — Auto-tuned for your hardware
4. 💾 **Saves to `~/.orbit/orbit.toml`** — Ready to use immediately

**Example session:**
```
🪐 Welcome to Orbit Setup
   We will scan your system and create an optimized configuration.

Scanning system environment...
  16 CPU cores detected
  32 GB RAM available
  I/O throughput: ~450 MB/s

Configuration Setup
? What is your primary use case?
  > Backup (Reliability First)
    Sync (Speed First)
    Cloud Upload (Compression First)
    Network Transfer (Resume + Compression)

✅ Configuration saved to: /home/user/.orbit/orbit.toml
```

**Pre-configured profiles (also available via `--profile` flag):**
- **Fast** → Zero-copy, no checksums, no resume — maximum speed for local SSD/NVMe
- **Safe** → Checksums, resume, retries — maximum reliability for critical data
- **Backup** → Checksums + Zstd compression + resume + metadata preservation — reliable backups
- **Network** → Zstd compression, resume, 10 retries — optimized for remote/slow networks

After running `orbit init`, your config is ready! All transfers will use your optimized settings automatically.

📖 **Full Guide:** See [`docs/guides/INIT_WIZARD_GUIDE.md`](docs/guides/INIT_WIZARD_GUIDE.md)

---

### Basic Usage

```bash
# Simple copy (positional arguments)
orbit source.txt destination.txt

# Or with named flags
orbit --source source.txt --dest destination.txt

# Use a preset profile for common scenarios
orbit /data /backup -R --profile backup     # Reliable backup: checksums + compression + resume
orbit /data /backup -R --profile fast       # Maximum speed: zero-copy, no checksums
orbit /data /backup -R --profile network    # Network-optimized: compression + resume + retries

# Copy with resume and checksum verification
orbit --source large-file.iso --dest /backup/large-file.iso --resume

# Recursive directory copy with compression
orbit --source /data/photos --dest /backup/photos --recursive --compress zstd:3

# Sync with parallel workers
orbit --source /source --dest /destination --mode sync --workers 8 --recursive

# High-concurrency S3 upload (256 workers, 8 parts per file)
orbit --source dataset.tar.gz --dest s3://my-bucket/backups/dataset.tar.gz \
  --workers 256 --concurrency 8

# Upload to S3 with execution statistics
orbit --source dataset.tar.gz --dest s3://my-bucket/backups/dataset.tar.gz --stat -H

# Preserve metadata with transformations
orbit --source /data --dest /backup --recursive \
  --preserve=times,perms,owners \
  --transform="case:lower"

# Selective transfer with filters
orbit --source /project --dest /backup --recursive \
  --exclude="target/**" \
  --exclude="*.log" \
  --include="important.log"

# Use filter file for complex rules
orbit --source /data --dest /backup --recursive \
  --filter-from=backup.orbitfilter

# Resilient transfer with retries and logging
orbit --source /data --dest /backup --recursive \
  --retry-attempts 5 \
  --exponential-backoff \
  --error-mode partial \
  --log-level debug \
  --log /var/log/orbit.log

# Skip failed files for batch operations
orbit --source /archive --dest /backup --recursive \
  --error-mode skip \
  --verbose

# Preview transfer with dry-run before executing
orbit --source /data --dest /backup --recursive --dry-run --verbose

# Bandwidth-limited transfer with progress tracking
orbit --source /large/dataset --dest /backup --recursive \
  --max-bandwidth 10 \
  --workers 4 \
  --show-progress

# Create flight plan manifest
orbit manifest plan --source /data --dest /backup --output ./manifests

# Batch execution: run multiple operations in parallel
orbit run --file commands.txt --workers 256

# Stream S3 object to stdout (requires s3-native feature)
orbit cat s3://bucket/data/report.csv | head -100

# Upload stdin to S3
tar czf - /data | orbit pipe s3://bucket/backups/data.tar.gz

# Generate a pre-signed URL (expires in 1 hour)
orbit presign s3://bucket/data/report.csv --expires 3600

# S3 wildcard listing (optimized prefix scan)
orbit --source "s3://bucket/data/2024-*.parquet" --dest /local --recursive
```

---

## ⚡ Performance Benchmarks

### Local Transfer Performance

| File Size | Traditional cp | Orbit (Zero-Copy) | Speedup | CPU Usage |
|-----------|----------------|-------------------|---------|-----------|
| 10 MB | 12 ms | 8 ms | 1.5× | ↓ 65% |
| 1 GB | 980 ms | 340 ms | 2.9× | ↓ 78% |
| 10 GB | 9.8 s | 3.4 s | 2.9× | ↓ 80% |

**macOS APFS Optimization**: On APFS filesystems (macOS 10.13+), file copies complete **instantly** via Copy-On-Write cloning — regardless of file size! Data is only duplicated when modified, providing near-zero latency for large files.

### S3 Transfer Performance

- **Multipart Upload:** 500+ MB/s on high-bandwidth links
- **Parallel Workers:** Up to 256 concurrent file operations (configurable via `--workers`)
- **Per-File Concurrency:** 5 concurrent parts per multipart upload (configurable via `--concurrency`)
- **Adaptive Chunking:** 5MB-2GB chunks based on file size
- **Wildcard Optimization:** Prefix-scoped listing with in-memory glob filtering
- **Resume Efficiency:** Chunk-level verification with intelligent restart decisions

### Compression Performance

- Zstd level 3 → 2.3× faster over networks
- LZ4 → near-realtime local copies
- Adaptive selection based on link speed

---

## 🧠 Smart Strategy Selection

Orbit automatically selects the optimal transfer strategy:

```
Same-disk large file  → Zero-copy (copy_file_range on Linux, APFS cloning on macOS)
macOS APFS            → Instant Copy-On-Write cloning (fclonefileat)
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
# Resume persistence is atomic (temp + rename); set ORBIT_RESUME_SLEEP_BEFORE_RENAME_MS for crash simulations

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

# Number of parallel file workers (0 = auto: 256 for network, CPU count for local)
# Alias: --parallel on CLI
parallel = 0

# Per-operation concurrency for multipart transfers (parts per file)
concurrency = 5

# Show execution statistics at end of run
show_stats = false

# Human-readable output (e.g., "1.5 GiB" instead of raw bytes)
human_readable = false

# Symbolic link handling: "skip", "follow", or "preserve"
symlink_mode = "skip"

# Error handling mode: "abort" (stop on error), "skip" (skip failed files), or "partial" (keep partial files for resume)
error_mode = "abort"

# Log level: "error", "warn", "info", "debug", or "trace"
log_level = "info"

# Path to log file (omit for stdout)
# log_file = "/var/log/orbit.log"

# Enable verbose logging (shorthand for log_level = "debug")
verbose = false

# Include patterns (glob, regex, or path - can be specified multiple times)
# Examples: "*.rs", "regex:^src/.*", "path:Cargo.toml"
include_patterns = [
    "**/*.rs",
    "**/*.toml",
]

# Exclude patterns (glob, regex, or path - can be specified multiple times)
# Examples: "*.tmp", "target/**", "regex:^build/.*"
exclude_patterns = [
    "*.tmp",
    "*.log",
    ".git/*",
    "node_modules/*",
    "target/**",
]

# Load filter rules from a file (optional)
# filter_from = "backup.orbitfilter"

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
2. `--profile` preset (applied as base, then CLI flags override)
3. `./orbit.toml` (project)
4. `~/.orbit/orbit.toml` (user)
5. Built-in defaults (lowest)

---

## 🧩 Modular Architecture

### OrbitSystem I/O Abstraction

Orbit features a universal I/O abstraction layer that decouples core logic from filesystem operations.

**Key Components:**

- **`orbit-core-interface`**: Defines the `OrbitSystem` trait
  - Discovery: `exists()`, `metadata()`, `read_dir()`
  - Data Access: `reader()`, `writer()`
  - Compute Offloading: `read_header()`, `calculate_hash()`

- **`LocalSystem`**: Default provider for standalone mode (wraps `tokio::fs`)
- **`MockSystem`**: In-memory implementation for testing (no disk I/O)

**Benefits:**

- ✅ **Testability**: Unit tests without filesystem via `MockSystem`
- ✅ **Flexibility**: Runtime switching between Local/Remote providers
- ✅ **Performance**: Compute offloading enables efficient distributed CDC

```rust
use orbit::system::LocalSystem;
use orbit_core_interface::OrbitSystem;

async fn example() -> orbit::error::Result<()> {
    let system = LocalSystem::new();
    let header = system.read_header(path, 512).await?;
    // Same code works for future RemoteSystem!
    Ok(())
}
```

📖 **See:** [`docs/specs/PHASE_1_ABSTRACTION_SPEC.md`](docs/specs/PHASE_1_ABSTRACTION_SPEC.md)

---

### Crate Structure

Orbit is built from clean, reusable crates:

| Crate | Purpose | Status |
|-------|---------|--------|
| 🔌 `orbit-core-interface` | OrbitSystem I/O abstraction | 🟢 Stable |
| 🧩 `core-manifest` | Manifest parsing and job orchestration | 🟡 Beta |
| 🌌 `core-starmap` | Job planner, dependency graph, container packing | 🟡 Beta |
| 🧬 `core-cdc` | Content-defined chunking (Gear Hash CDC) | 🔴 Alpha |
| 🧠 `core-semantic` | Intent-based replication, composable prioritizers | 🔴 Alpha |
| 📊 `core-audit` | Structured logging, telemetry, typed provenance | 🟡 Beta |
| 📡 `orbit-observability` | Observability and monitoring | 🟡 Beta |
| 🚀 `orbit` (root) | CLI, core engine, and transfer orchestration | 🟡 Beta |

This structure ensures isolation, testability, and reusability.

---

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

### 🛡️ Dependency Security & Build Features

**Default Build Security:** The default `cargo build` configuration includes **zero runtime security vulnerabilities**. Our minimal feature set (`zero-copy` only) ensures the smallest possible attack surface.

| Build Configuration | Security Status | Use Case |
|---------------------|----------------|----------|
| `cargo build` (default) | ✅ **Zero vulnerabilities** | Production deployments |
| `cargo build --features network` | ✅ **Zero vulnerabilities** | All network protocols |
| `cargo build --features smb-native` | ⚠️ **Optional advisory** | SMB protocol (see note below) |

**Optional Feature Advisory:** When building with `--features smb-native`, a medium-severity timing side-channel advisory (RUSTSEC-2023-0071) is present in the SMB authentication stack. This requires active exploitation during SMB connections and does not affect other protocols or default builds.

**Security Verification:**
```bash
# Verify default build has no active vulnerabilities
cargo tree -p rsa           # Expected: "nothing to print"
cargo tree -p sqlx-mysql    # Expected: "package ID not found"
```

For complete security audit results, dependency chain analysis, and mitigation details, see **[SECURITY.md](SECURITY.md#dependency-security-audit)**.

---

## 📖 CLI Quick Reference

**Transfer operations:**
```bash
orbit <SOURCE> <DEST> [FLAGS]               # Positional arguments
orbit --source <PATH> --dest <PATH> [FLAGS]  # Named flags
```

**Profile presets:**
| Flag | Description |
|------|-------------|
| `--profile fast` | Maximum speed: zero-copy, no checksums, no resume |
| `--profile safe` | Maximum reliability: checksums, resume, retries |
| `--profile backup` | Reliable backups: checksums + Zstd compression + resume + metadata |
| `--profile network` | Network-optimized: Zstd compression, resume, 10 retries |

**Parallelism flags:**
| Flag | Description | Default |
|------|-------------|---------|
| `--workers N` | Parallel file workers (0 = auto) | 0 (auto: 256 network, CPU count local) |
| `--parallel N` | Alias for `--workers` | 0 |
| `--concurrency N` | Multipart parts per file | 5 |

**Output flags:**
| Flag | Description |
|------|-------------|
| `--stat` | Print execution statistics at end of run |
| `-H` / `--human-readable` | Human-readable byte values (e.g., "1.5 GiB") |
| `--json` | JSON Lines output for all operations (machine-parseable) |

**S3 flags:**
| Flag | Description |
|------|-------------|
| `--no-sign-request` | Anonymous access for public S3 buckets |
| `--use-acceleration` | Enable S3 Transfer Acceleration |
| `--request-payer` | Access requester-pays buckets |
| `--aws-profile NAME` | Use a specific AWS credential profile |
| `--credentials-file PATH` | Custom AWS credentials file path |
| `--part-size N` | Multipart part size in MiB (default: 50) |
| `--no-verify-ssl` | Skip SSL certificate verification |
| `--use-list-objects-v1` | Use ListObjects v1 API for compatibility |

**Conditional copy flags:**
| Flag | Description |
|------|-------------|
| `-n` / `--no-clobber` | Do not overwrite existing files |
| `--if-size-differ` | Only copy if source and destination sizes differ |
| `--if-source-newer` | Only copy if source is newer than destination |

**Subcommands:**
```bash
orbit run [--file FILE] [--workers N]     # Batch execution from file/stdin
orbit ls s3://bucket/prefix [-e] [-s]     # List S3 objects
orbit head s3://bucket/key                # Show S3 object metadata
orbit du s3://bucket/prefix [-g]          # Show storage usage
orbit rm s3://bucket/pattern [--dry-run]  # Delete S3 objects
orbit mv s3://src s3://dst                # Move/rename S3 objects
orbit mb s3://bucket-name                 # Create S3 bucket
orbit rb s3://bucket-name                 # Remove S3 bucket
orbit cat s3://bucket/key                 # Stream S3 object to stdout
orbit pipe s3://bucket/key                # Upload stdin to S3
orbit presign s3://bucket/key [--expires] # Generate pre-signed URL
orbit manifest <plan|verify|diff|info>    # Manifest operations
orbit <init|stats|presets|capabilities>   # Configuration & info
```

> **Note:** `cat`, `pipe`, and `presign` require the `s3-native` feature flag.

---

## 🧪 Roadmap

### ✅ Core Features Implemented

**Stable/Well-Tested:**
- Zero-copy system calls (Linux, macOS, Windows)
- Compression engines (LZ4, Zstd)
- Checksum verification (SHA-256, BLAKE3)
- Modular crate architecture

**Beta/Recently Added (needs more real-world testing):**
- Resume and retry with chunk-level verification
- Native S3 support with multipart transfers
- SSH/SFTP backend
- S3-compatible storage (MinIO, LocalStack)
- Disk Guardian: Pre-flight space & integrity checks
- Delta Detection: rsync-inspired efficient transfers with block-based diffing
- Metadata Preservation & Transformation
- Inclusion/Exclusion Filters: Glob, regex, and path patterns
- Progress Reporting & Operational Controls: Bandwidth limiting, concurrency control
- Manifest + Starmap + Audit integration
- Structured telemetry with JSON Lines
- Config Optimizer with active environment probing

**Alpha/Experimental:**
- V2 Architecture (CDC, semantic replication, global dedup)
- SMB2/3 native implementation (awaiting upstream fix)

### ✅ Recently Completed

- **Simplified CLI**: Positional arguments (`orbit /src /dest`), `--profile` presets, actionable error messages
- **Codebase restructuring**: main.rs reduced by 45%, subcommand handlers extracted to dedicated modules
- **Config Optimizer enhancements**: Hardware probe caching, auto-tune summary display, new worker/chunk rules
- **Dependency cleanup**: Removed `anyhow`, made Tokio optional (only for network backends)

### 🚧 In Progress

- Stabilizing V2 architecture components (CDC, semantic replication)
- Expanding test coverage for newer features
- Real-world validation of S3 and SSH backends

### 🔮 Planned

#### CLI Improvements
- Friendly subcommands (`orbit cp`, `orbit sync`, `orbit run`) as aliases
- File watching mode (`--watch`)
- Interactive mode with prompts

#### New Protocols
- WebDAV protocol support

#### Advanced Features
- Wormhole FEC module for lossy networks
- Plugin framework for custom protocols
- Disk quota integration

---

## 🦀 Contributing

Pull requests welcome! See `CONTRIBUTING.md` for code style and guidelines.

### Development

```bash
# Clone and build (includes S3, SMB, SSH by default)
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build

# Run tests (includes S3 backend tests)
cargo test

# Run with network features
cargo build --features network
cargo test --features network

# Minimal build (no network backends)
cargo build --no-default-features --features zero-copy

# Format and lint
cargo fmt
cargo clippy
```

### Areas We Need Help

- 🌐 Resolving SMB upstream dependencies
- 🧪 Testing on various platforms
- 📚 Documentation improvements
- 🐛 Bug reports and fixes

---

## 📚 Documentation

### User Guides
- **Quick Start:** This README
- **Testing & Validation Scripts:** [`docs/guides/TESTING_SCRIPTS_GUIDE.md`](docs/guides/TESTING_SCRIPTS_GUIDE.md)
- **S3 Guide:** [`docs/guides/S3_USER_GUIDE.md`](docs/guides/S3_USER_GUIDE.md)
- **GCS Guide:** [`docs/guides/GCS_USER_GUIDE.md`](docs/guides/GCS_USER_GUIDE.md)
- **Disk Guardian:** [`docs/architecture/DISK_GUARDIAN.md`](docs/architecture/DISK_GUARDIAN.md)
- **Delta Detection:** [`docs/guides/DELTA_DETECTION_GUIDE.md`](docs/guides/DELTA_DETECTION_GUIDE.md) and [`docs/guides/DELTA_QUICKSTART.md`](docs/guides/DELTA_QUICKSTART.md)
- **Filter System:** [`docs/guides/FILTER_SYSTEM.md`](docs/guides/FILTER_SYSTEM.md)
- **Progress & Concurrency:** [`docs/architecture/PROGRESS_AND_CONCURRENCY.md`](docs/architecture/PROGRESS_AND_CONCURRENCY.md)
- **Resume System:** [`docs/architecture/RESUME_SYSTEM.md`](docs/architecture/RESUME_SYSTEM.md)
- **Protocol Guide:** [`docs/guides/PROTOCOL_GUIDE.md`](docs/guides/PROTOCOL_GUIDE.md)

### Technical Documentation
- **SMB Status:** [`docs/SMB_NATIVE_STATUS.md`](docs/SMB_NATIVE_STATUS.md)
- **Manifest System:** [`docs/MANIFEST_SYSTEM.md`](docs/MANIFEST_SYSTEM.md)
- **Zero-Copy Guide:** [`docs/ZERO_COPY.md`](docs/ZERO_COPY.md)
- **API Reference:** Run `cargo doc --open`

### Examples
- **Basic Examples:** [`examples/`](examples/) directory
- **S3 Examples:** [`examples/s3_*.rs`](examples/)
- **Disk Guardian Demo:** [`examples/disk_guardian_demo.rs`](examples/disk_guardian_demo.rs)
- **Filter Example:** [`examples/filters/example.orbitfilter`](examples/filters/example.orbitfilter)
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
