# 🚀 Orbit

> **O**pen **R**esilient **B**ulk **I**nformation **T**ransfer

**The intelligent file transfer tool that never gives up** 💪

[![CI](https://github.com/saworbit/orbit/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/saworbit/orbit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![GitHub](https://img.shields.io/github/stars/saworbit/orbit?style=social)](https://github.com/saworbit/orbit)

---

## ⚠️ Project Status: Alpha (v0.6.0 Core / v2.2.0-rc.1 Control Plane)

**Orbit is currently in active development and should be considered alpha-quality software.**

- ✅ **Safe for**: Experimentation, evaluation, non-critical workloads, development environments
- ⚠️ **Use with caution for**: Important data transfers (test thoroughly first, maintain backups)
- ❌ **Not recommended for**: Mission-critical production systems without extensive testing

**What this means:**
- APIs may change between versions
- Some features are experimental and marked as such
- The V2 architecture (content-defined chunking, semantic replication) is newly introduced
- **NEW v0.6.0-alpha.5**: Phase 5 - Sentinel (Autonomous Resilience Engine) - OODA loop monitors Universe V3 and autonomously heals under-replicated chunks via Phase 4 P2P transfers
- **v0.6.0-alpha.4**: Phase 4 - Data Plane (P2P Transfer) - Direct Star-to-Star data transfer eliminates Nucleus bandwidth bottleneck, enabling infinite horizontal scaling
- **v0.6.0-alpha.3**: Phase 3 - Nucleus Client & RemoteSystem (Client-side connectivity for Nucleus-to-Star orchestration, 99.997% network reduction via compute offloading)
- **v0.6.0-alpha.2**: Phase 2 - Star Protocol & Agent (gRPC remote execution server for distributed Orbit Grid)
- **v0.6.0-alpha.1**: Phase 1 I/O Abstraction Layer - OrbitSystem trait enables future distributed topologies
- **NEW v2.2.0-rc.1**: Full-stack CI/CD pipeline with dashboard-quality checks, professional file browser, and enhanced developer experience
- **v2.2.0-beta.1**: Enterprise platform features - Intelligence API (Estimations), Administration (User Management), System Health monitoring
- **v2.2.0-alpha.2**: React Dashboard implementation with Visual Pipeline Editor, File Browser, and Job Management UI
- **v2.2.0-alpha.1**: Control Plane architecture with decoupled React dashboard ("The Separation")
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
  - [Guidance System](#️-guidance-system-the-flight-computer)
  - [Manifest System + Starmap](#️-manifest-system--starmap-planner)
  - [Magnetar State Machine](#-magnetar-persistent-job-state-machine)
  - [Metadata Preservation](#️-metadata-preservation--transformation)
  - [Delta Detection](#-delta-detection-efficient-transfers)
  - [Progress Reporting & Operational Controls](#-progress-reporting--operational-controls)
  - [Inclusion/Exclusion Filters](#-inclusionexclusion-filters-selective-transfers)
  - [Protocol Support](#-protocol-support)
  - [Audit & Telemetry](#-audit-and-telemetry)
- [Quick Start](#-quick-start)
- [Web GUI](#️-web-gui-new-in-v050)
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

**Key Philosophy:** Intelligence, resilience, and speed. Currently in active development (v0.6.0 alpha).

---

## ✨ Why Orbit?

| Feature | Benefit |
|---------|---------|
| 🚄 **Performance** | Zero-copy system calls for faster transfers (instant APFS cloning on macOS) |
| 🛡️ **Resilient** | Smart resume with chunk verification, checksums, corruption detection |
| 🧠 **Adaptive** | Adapts strategy based on environment (zero-copy, compression, buffered) |
| 🛡️ **Safe** | Disk Guardian prevents mid-transfer failures with pre-flight checks |
| 🌐 **Protocol Support** | Local, **SSH/SFTP**, SMB/CIFS (experimental), **S3**, **Azure Blob**, **GCS**, with unified backend API |
| 🌐 **Web Dashboard** | Modern React dashboard with OpenAPI-documented Control Plane (v2.2.0-alpha) |
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
| **OrbitSystem Abstraction (Phase 1)** | 🟢 Stable | I/O abstraction layer, foundation for Grid topology |
| **Resume/Checkpoint** | 🟡 Beta | Works well, needs more edge-case testing |
| **Compression (LZ4, Zstd)** | 🟢 Stable | Reliable for most workloads |
| **Checksum Verification** | 🟢 Stable | SHA-256, BLAKE3 well-tested |
| **Local Filesystem** | 🟢 Stable | Primary use case, thoroughly tested |
| **SSH/SFTP Backend** | 🟡 Beta | Functional, needs more real-world testing |
| **S3 Backend** | 🟡 Beta | Works well, multipart upload is newer |
| **SMB Backend** | 🟡 Beta | v0.11.0 upgrade complete, ready for integration testing |
| **Azure Blob Backend** | 🟡 Beta | Production-ready using object_store crate, newly added in v0.6.0 |
| **GCS Backend** | 🟡 Beta | Production-ready using object_store crate, newly added in v0.6.0 |
| **Delta Detection (V1)** | 🟡 Beta | rsync-style algorithm, tested but newer |
| **V2 Architecture (CDC)** | 🔴 Alpha | Content-defined chunking, introduced in v0.5.0 |
| **Semantic Replication** | 🔴 Alpha | Priority-based transfers, introduced in v0.5.0 |
| **Neutrino Fast Lane** | 🔴 Alpha | Small file optimization (<8KB), introduced in v0.5.0 |
| **Global Deduplication (V3)** | 🟡 Beta | High-cardinality Universe index, v2.1 scalability upgrade |
| **Disk Guardian** | 🟡 Beta | Pre-flight checks, works well but newer |
| **Magnetar State Machine** | 🟡 Beta | Job persistence, recently added |
| **Resilience Patterns** | 🟡 Beta | Circuit breaker, rate limiting - new features |
| **Sentinel Resilience Engine (Phase 5)** | 🔴 Alpha | Autonomous OODA loop for chunk redundancy healing |
| **Filter System** | 🟡 Beta | Glob/regex filters, functional but newer |
| **Metadata Preservation** | 🟡 Beta | Works well, extended attributes are platform-specific |
| **Guidance System** | 🟡 Beta | Config validation, recently added |
| **Control Plane API** | 🔴 Alpha | v2.2.0-alpha - OpenAPI/Swagger documented REST API |
| **React Dashboard** | 🔴 Alpha | v2.2.0-alpha - Modern SPA with React Flow pipelines |
| **Manifest System** | 🟡 Beta | File tracking and verification |
| **Progress/Bandwidth Limiting** | 🟡 Beta | Recently integrated across all modes |
| **Audit Logging** | 🟡 Beta | Structured telemetry, needs more use |

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

**Smart Retry Logic (NEW):**
- ⚡ **Permanent errors fail fast** — `PermissionDenied`, `AlreadyExists` skip retries (saves 35+ seconds per error)
- 🔄 **Transient errors retry** — `TimedOut`, `ConnectionRefused` use full exponential backoff
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

### 🛰️ Guidance System: The "Flight Computer"

Automatic configuration validation and optimization that ensures safe, performant transfers.

**What It Does:**
The Guidance System acts as an intelligent pre-processor, analyzing your configuration for logical conflicts and automatically resolving them before execution begins. Think of it as a co-pilot that prevents common mistakes and optimizes settings based on hardware capabilities and use-case logic.

**Key Benefits:**
- 🔒 **Safety First** — Prevents data corruption from incompatible flag combinations
- ⚡ **Performance Optimization** — Automatically selects the fastest valid strategy
- 🎓 **Educational** — Explains why configurations were changed
- 🤖 **Automatic** — No manual debugging of conflicting flags

**Example Output:**
```
┌── 🛰️  Orbit Guidance System ───────────────────────┐
│ 🚀 Strategy: Disabling zero-copy to allow streaming checksum verification
│ 🛡️  Safety: Disabling resume capability to prevent compressed stream corruption
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

**Philosophy:**
> Users express **intent**. Orbit ensures **technical correctness**.

Rather than failing with cryptic errors, Orbit understands what you're trying to achieve and automatically adjusts settings to make it work safely and efficiently.

**Programmatic API:**
```rust
use orbit::core::guidance::Guidance;

let mut config = CopyConfig::default();
config.use_zero_copy = true;
config.verify_checksum = true;

// Run guidance pass
let flight_plan = Guidance::plan(config)?;

// Display notices
for notice in &flight_plan.notices {
    println!("{}", notice);
}

// Use optimized config
copy_file(&source, &dest, &flight_plan.config)?;
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
  - 4/4 persistence tests passing
- **See:** [ORBIT_V2_ARCHITECTURE.md](ORBIT_V2_ARCHITECTURE.md) for complete details

**Neutrino Fast Lane** ⚡

The **Neutrino Fast Lane** provides ~3x performance improvement for small-file workloads by bypassing CDC/deduplication overhead:

- **Smart Routing** — Files <8KB automatically routed to high-concurrency direct transfer
- **High Concurrency** — 100-500 concurrent async tasks (vs standard 16)
- **Zero Overhead** — Bypasses BLAKE3 hashing, CDC chunking, and starmap indexing
- **Reduced CPU Load** — Direct I/O without rolling hash computation
- **Configurable Threshold** — Adjustable size threshold (default: 8KB)
- **Seamless Integration** — Works with Smart Sync priority-based transfers

**Performance:**
- 10,000 files (1-4KB): ~15s vs ~45s (standard) = **3x faster**
- 60% lower CPU usage for small-file workloads
- Minimal database bloat (no index entries for small files)

**Usage:**
```bash
# Enable Neutrino fast lane
orbit copy --profile neutrino --recursive /source /dest

# Custom threshold (16KB)
orbit copy --profile neutrino --neutrino-threshold 16 --recursive /source /dest

# Combined with Smart Sync
orbit copy --check smart --profile neutrino --recursive /source /dest
```

**Best For:**
- Source code repositories (`node_modules`, `.git` directories)
- Configuration directories (`/etc`, `.config`)
- Log files and small assets
- npm/pip package directories

**Requirements:** Requires `backend-abstraction` feature (included with network backends)

**See:** [PERFORMANCE.md](docs/guides/PERFORMANCE.md#neutrino-fast-lane-v05) for detailed documentation

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
| 🌐 **SMB/CIFS** | 🟡 Beta | `smb-native` | Native SMB2/3 client (pure Rust, v0.11.0, ready for testing) |
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
  --mode sync --compress zstd:5 --recursive

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
  --mode sync --compress zstd:5 --recursive

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
- ✅ **Resilience patterns** — Circuit breaker, connection pooling, and rate limiting via Magnetar ⭐

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
  --mode sync --compress zstd:5 --recursive

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

📖 **Implementation Status:** See [`AZURE_IMPLEMENTATION_STATUS.md`](AZURE_IMPLEMENTATION_STATUS.md)

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
orbit copy /source /dest --audit-log ./audit.jsonl

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
orbit copy /source /dest \
  --audit-log ./audit.jsonl \
  --otel-endpoint http://jaeger:4317
```

**Trace correlation features:**
- **W3C-compliant** trace IDs (32-char hex) and span IDs (16-char hex)
- **Hierarchical correlation** — trace_id → job_id → file_id → span_id
- **Cross-service tracing** — Trace transfers across Nucleus, Star, and Sentinel components
- **Backend instrumentation** — All 45 backend methods emit trace spans (S3, SMB, SSH, local)

#### 📊 Prometheus Metrics

**Expose metrics for monitoring:**
```bash
orbit copy /source /dest \
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

# With Control Plane API
cargo build --release --features api

# Full build (everything)
cargo build --release --features full

# Install to system
sudo cp target/release/orbit /usr/local/bin/

# Or with cargo install
cargo install --path .                    # Minimal
cargo install --path . --features network  # With network
cargo install --path . --features full    # Everything
```

> **v0.5+:** Orbit defaults to a minimal build (just local copy with zero-copy optimizations) for fastest compile times and smallest binaries. Network protocols and GUI are opt-in via feature flags.

### Feature Flags & Binary Sizes

**v0.5-0.6 Performance Improvements:**
- 🎯 **60% smaller default binary** — Minimal build is ~10MB (was ~50MB)
- ⚡ **50% faster compilation** — Default build in ~60s (was ~120s)
- 🔒 **Reduced attack surface** — No web server code in default CLI build
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
| `api` | Control Plane REST API (v2.2.0+) | +15MB | ❌ No |
| `delta-manifest` | SQLite-backed delta persistence | +3MB | ❌ No |
| `extended-metadata` | xattr + ownership (Unix/Linux/macOS only) | +500KB | ❌ No |
| `full` | All features enabled | +50MB | ❌ No |

```bash
# Minimal: Fast local copies only (~10MB)
cargo build --release
cargo install orbit

# Network: Add S3, SMB, SSH, Azure support (~38MB)
cargo build --release --features network
cargo install orbit --features network

# GUI: Add web dashboard (~25MB)
cargo build --release --features gui
cargo install orbit --features gui

# Full: Everything including network + GUI (~50MB+)
cargo build --release --features full
cargo install orbit --features full

# Size-optimized: Maximum compression
cargo build --profile release-min
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
  --parallel 4 \
  --show-progress

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

**macOS APFS Optimization**: On APFS filesystems (macOS 10.13+), file copies complete **instantly** via Copy-On-Write cloning — regardless of file size! Data is only duplicated when modified, providing near-zero latency for large files.

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

# Number of parallel operations (0 = sequential)
parallel = 4

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
2. `./orbit.toml` (project)
3. `~/.orbit/orbit.toml` (user)
4. Built-in defaults (lowest)

---

## 🧩 Modular Architecture

### Phase 1: OrbitSystem I/O Abstraction (v0.6.0-alpha.1)

**NEW!** Orbit now features a universal I/O abstraction layer that decouples core logic from filesystem operations.

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
- ✅ **Future-Ready**: Foundation for distributed Grid/Star topology
- ✅ **Performance**: Compute offloading enables efficient distributed CDC

```rust
use orbit::system::LocalSystem;
use orbit_core_interface::OrbitSystem;

async fn example() -> anyhow::Result<()> {
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
| 🔌 `orbit-core-interface` | OrbitSystem I/O abstraction (Phase 1) | 🟢 Stable |
| 🧩 `core-manifest` | Manifest parsing and job orchestration | 🟡 Beta |
| 🌌 `core-starmap` | Job planner and dependency graph | 🟡 Beta |
| 🌌 `core-starmap::universe` | Global deduplication index (V2) | 🔴 Alpha |
| 🌌 `core-starmap::migrate` | V1→V2 migration utilities | 🔴 Alpha |
| 🧬 `core-cdc` | FastCDC content-defined chunking (V2) | 🔴 Alpha |
| 🧠 `core-semantic` | Intent-based replication (V2) | 🔴 Alpha |
| 📊 `core-audit` | Structured logging and telemetry | 🟡 Beta |
| ⚡ `core-zero-copy` | OS-level optimized I/O | 🟢 Stable |
| 🗜️ `core-compress` | Compression and decompression | 🟢 Stable |
| 🛡️ `disk-guardian` | Pre-flight space & integrity checks | 🟡 Beta |
| 🧲 `magnetar` | Idempotent job state machine (SQLite + redb) | 🟡 Beta |
| 🛡️ `magnetar::resilience` | Circuit breaker, connection pool, rate limiter | 🟡 Beta |
| 🛡️ `orbit-sentinel` | Autonomous resilience engine (Phase 5 OODA loop) | 🔴 Alpha |
| 🌐 `protocols` | Network protocol implementations | 🟡 S3/SSH Beta, 🔴 SMB Alpha |
| 🌐 `orbit-server` | Headless Control Plane API (v2.2.0-alpha) | 🔴 Alpha |
| 🎨 `orbit-dashboard` | React dashboard (v2.2.0-alpha) | 🔴 Alpha |
| 🕵️ `core-watcher` | Monitoring beacon | 🚧 Planned |
| 🧪 `wormhole` | Forward-error correction | 🚧 Planned |

This structure ensures isolation, testability, and reusability.

---

## 🖥️ Orbit Control Plane v2.2.0-alpha - "The Separation"

**Breaking architectural change:** Orbit v2.2.0 separates the monolithic web application into a **headless Control Plane (Rust)** and a **modern Dashboard (React/TypeScript)**, enabling independent deployment, faster iteration, and better scalability.

### Architecture Overview

```
┌─────────────────────┐
│  Orbit Dashboard    │  React 18 + Vite + TypeScript
│  (Port 5173)        │  TanStack Query + React Flow
└──────────┬──────────┘
           │ HTTP/WebSocket
           │ (CORS enabled)
           ▼
┌─────────────────────┐
│  Control Plane API  │  Axum + OpenAPI/Swagger
│  (Port 8080)        │  JWT Auth + WebSocket
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  Magnetar Database  │  SQLite + redb
└─────────────────────┘
```

### Quick Start

**Option 1: Use the launcher scripts** (Easiest)

```bash
# Unix/Linux/macOS
./launch-orbit.sh

# Windows
launch-orbit.bat
```

**Option 2: Manual startup**

```bash
# Terminal 1: Start Control Plane
cd crates/orbit-web
cargo run --bin orbit-server

# Terminal 2: Start Dashboard
cd dashboard
npm install  # First time only
npm run dev
```

**Access Points:**
- 🎨 **Dashboard**: http://localhost:5173
- 🔌 **API**: http://localhost:8080/api
- 📚 **Swagger UI**: http://localhost:8080/swagger-ui
- 🔒 **Default credentials**: `admin` / `orbit2025` (⚠️ Change in production!)

**What's Running:**
- ☢️ **Reactor Engine**: Background job executor (starts automatically with orbit-server)
- 🎨 **Dashboard Dev Server**: React app with hot reload (port 5173)
- 🔌 **API Server**: RESTful API with WebSockets (port 8080)

**Browser Safety**: Launch scripts open the dashboard in a **new browser tab** and will **NOT kill your other tabs** when you close the script. Safe to use with your existing browser session!

### 🛰️ E2E Demo Harness - "Deep Space Telemetry Scenario"

**NEW in v2.2.0!** Experience Orbit's full capabilities with an automated end-to-end demonstration that showcases real-time job management, visual chunk maps, and live telemetry tracking.

**🛡️ Safety First (Recommended for First-Time Users):**

Before running the demo, use the safety validator to verify your system is ready **without making any changes**:

```bash
# Unix/Linux/macOS
./scripts/validate-demo-safety.sh

# Windows (Git Bash)
bash scripts/validate-demo-safety.sh
```

The validator checks system requirements, port availability, disk space, and shows exactly what the demo will do. See [SAFETY_FIRST.md](SAFETY_FIRST.md) for complete safety documentation.

**Option 3: Run the E2E Demo** (Best for first-time users and demonstrations)

```bash
# Unix/Linux/macOS
./demo-orbit.sh

# Windows
demo-orbit.bat
```

**Requirements:**
- 💾 **Disk Space:** 4GB free (or 400MB if binaries already built) - [See details](DISK_SPACE_GUIDE.md)
- ⏱️ **Duration:** ~5-10 minutes (includes build time)
- 🌐 **Ports:** 8080 (API) and 5173 (Dashboard) must be available

**What the demo does:**
1. ✅ **Environment Validation** - Verifies Rust, Node.js, and port availability
2. 📊 **Data Fabrication** - Generates ~170MB of synthetic telescope telemetry data
3. 🚀 **System Ignition** - Launches both Control Plane and Dashboard
4. 🎯 **Job Injection** - Programmatically creates and starts a transfer job via REST API
5. 👁️ **Observation Phase** - Interactive pause to explore the dashboard's Visual Chunk Map and live telemetry graphs
6. 🧹 **Cleanup** - Gracefully terminates services and removes temporary data

**Features demonstrated:**
- **Magnetar State Machine** - Job lifecycle management (`pending` → `running` → `completed`)
- **Real-Time Dashboard** - Visual Chunk Map showing chunk-level transfer progress
- **Live Telemetry** - Transfer speed graphs and statistics
- **REST API** - Programmatic job creation and control
- **Resilient Transfer** - Compression, verification, parallel workers (4 concurrent)

**Perfect for:**
- 🎬 **Sales Demonstrations** - Show Orbit's capabilities to stakeholders
- 🧪 **Development Testing** - Validate full-stack functionality quickly
- 📚 **Training** - Onboard new developers to the architecture
- 🤖 **CI/CD Integration** - Automated E2E testing in pipelines

📖 **Full Documentation:** See [`DEMO_GUIDE.md`](DEMO_GUIDE.md) for detailed usage, troubleshooting, and customization options.

### Compilation Modes: Headless vs Full UI

**NEW in v2.2.0!** The Control Plane now supports **compile-time modularity** via feature flags, allowing you to build either a lightweight headless API server or a full-featured server with embedded dashboard.

#### Scenario A: Headless Mode (Default)
Build a smaller, API-only binary without UI dependencies. Perfect for automation, CI/CD pipelines, or custom frontend integrations.

```bash
# Minimal binary - no UI, smaller attack surface
cargo build --release -p orbit-server

# Binary size: ~15MB (vs ~25MB with UI)
# No static file serving, no dashboard embedded
```

**Use cases:**
- Kubernetes/Docker deployments with separate UI CDN
- API-only microservices
- Custom dashboard integration
- Embedded systems with limited storage

#### Scenario B: Full UI Mode
Build with embedded dashboard for all-in-one deployment.

```bash
# Full binary with embedded React dashboard
cargo build --release -p orbit-server --features ui

# Binary serves dashboard from dashboard/dist
# Requires: npm run build in dashboard/ first
```

**Use cases:**
- Single-binary desktop applications
- Quick demos and development
- End-user installations
- Local workstation deployment

#### Runtime Behavior

**Headless Mode:**
```
⚙️ Headless Mode: Dashboard not included, API-only server
Orbit Control Plane (Headless Mode) - API available at /api/*
```

**UI Mode:**
```
🎨 UI Feature Enabled: Serving embedded dashboard from dashboard/dist
Dashboard available at http://localhost:8080/
```

### Control Plane Features (v2.2.0-alpha)

#### ✅ OpenAPI-Documented REST API
- **Swagger UI** at `/swagger-ui` for interactive API testing
- **Type-safe endpoints** with utoipa schema generation
- **Job Management**: Create, list, monitor, cancel, delete jobs
- **Backend Configuration**: Manage S3, SMB, SSH, Local backends
- **Authentication**: JWT-based auth with httpOnly cookies
- **Real-time Updates**: WebSocket streams at `/ws/:job_id`

#### ✅ Intelligent Scheduling (Planned)
- **Duration Estimation**: Predict transfer times based on historical data
- **Bottleneck Detection**: Proactive warnings for performance issues
- **Confidence Scoring**: Reliability metrics for time estimates
- **Priority Queues**: Smart job ordering for critical transfers

#### ✅ Production Security
- **JWT Authentication** with 24-hour expiration
- **Argon2 Password Hashing** (OWASP recommended)
- **Role-Based Access Control** (Admin/Operator/Viewer)
- **CORS Configuration** for dashboard integration
- **Environment-based secrets** via `ORBIT_JWT_SECRET`

### Dashboard Features (v2.2.0-rc.1)

#### ✅ Modern React Stack
- **React 19** with TypeScript for type safety and strict ESLint compliance
- **Vite 7** for instant hot module replacement (HMR)
- **TanStack Query** for intelligent data fetching and caching
- **Tailwind CSS 4** with tailwindcss-animate plugin for professional design and smooth animations
- **Lucide Icons** for consistent iconography
- **@xyflow/react 12** for visual pipeline editing
- **Full-Screen Layout**: Edge-to-edge dashboard design with removed Vite scaffolding constraints

#### ✅ Cockpit-Style App Shell (NEW in Unreleased)
- **Sidebar Navigation**: Professional persistent sidebar replacing top navigation bar
- **Live Status Indicator**: Animated pulsing green dot for "System Online" confirmation
- **Pre-Alpha Warning**: Prominent warning banner across all views
- **Mobile Drawer**: Smooth slide-in menu with backdrop overlay for mobile devices
- **Responsive Design**: Fully optimized from 320px to 4K displays
- **Theme Integration**: Dark/light mode toggle with consistent styling
- **Operator Profile**: Gradient avatar with system status in sidebar

#### ✅ Mission Control Dashboard (NEW in Unreleased - Embedded Visibility)
- **Live Telemetry**: Real-time network throughput with SVG area charts
- **Client-Side Buffering**: 30-point rolling history for smooth "live" feel
- **Metric Cards**: Active Jobs, Throughput, System Load, Storage Health with trend indicators
- **Animated Status**: Pulsing green dot for "Live Stream Active" confirmation
- **Capacity Planning**: Donut chart visualization with used/available space breakdown
- **Traffic Statistics**: Peak, Average, and Total Transferred metrics

#### ✅ Deep-Dive Job Details (NEW in Unreleased - Embedded Visibility)
- **Visual Chunk Map**: 100-cell grid showing completion progress with color coding
- **Glowing Effects**: Green (completed) and red (failed) chunks with shadow effects
- **Proportional Sampling**: Intelligent downsampling for jobs with >100 chunks
- **Event Stream**: Real-time lifecycle events with timestamps and status icons
- **Configuration Display**: Detailed source/destination, mode, compression, verification
- **Performance Metrics**: Throughput, chunk statistics, and timing data
- **Breadcrumb Navigation**: "Job List → Job #N" with back button

#### ✅ Enhanced Job Management (NEW in Unreleased)
- **Click-to-Expand**: Select any job to view detailed inspection view
- **Real-time Search**: Filter jobs by ID, source path, or destination path
- **Status Filtering**: Dropdown to filter by All/Running/Pending/Completed/Failed
- **Manual Refresh**: Button for on-demand data refresh
- **Compact Mode**: Shows 5 most recent jobs for dashboard integration
- **Enhanced Empty States**: Helpful messaging with icons for better user guidance

#### ✅ Professional File Browser (rc.1)
- **Click-to-Select** files and folders with visual feedback
- **Up Navigation** button to traverse parent directories
- **Folder Selection** button for directory transfers
- **Visual Indicators**: Selected items highlighted in blue with dark mode support
- **Loading States**: Spinner and error handling for API calls
- **RESTful API**: GET `/api/files/list?path={path}` endpoint

#### ✅ Improved Quick Transfer (NEW in Unreleased)
- **Visual Flow**: Source → destination with animated connector
- **Color Coding**: Blue borders for source, orange for destination
- **State Management**: Success/error feedback (no more browser alerts)
- **Auto-reset**: Form clears automatically after successful transfer
- **Validation**: Better input validation and loading states

#### ✅ Visual Pipeline Builder
- **React Flow v12** DAG editor for intuitive job configuration
- **Drag-and-drop** source and destination nodes
- **Theme-aware**: Uses design system colors for consistent styling
- **Icon Toolbar**: Enhanced buttons with Database/Zap/Cloud icons
- **Node Counter**: Displays current number of nodes and connections

#### ✅ User Management (NEW in Unreleased)
- **Statistics Dashboard**: Cards showing Total Users, Admins, and Operators
- **Delete Functionality**: Remove users with confirmation dialogs
- **Gradient Avatars**: Auto-generated avatars with user initials
- **Role Badges**: Theme-aware badges for Admin/Operator/Viewer roles
- **Enhanced Forms**: Better layout with clear field labeling

#### ✅ Smart Data Fetching
- **Adaptive Polling**: 2s for jobs and health, optimized for responsiveness
- **Optimistic Updates**: Instant UI feedback on mutations
- **Automatic Cache Invalidation**: Always shows fresh data
- **Request Deduplication**: Efficient network usage

#### ✅ Real-time Monitoring
- **Live Job Status** with progress bars and percentages
- **Transfer Speed Tracking** with chunk completion metrics
- **Sparkline Trends**: Visual representation of metric history
- **Auto-refresh**: Continuous updates for active monitoring

#### ✅ CI/CD Pipeline (rc.1)
- **Dashboard Quality Control**: Dedicated GitHub Actions job
  - Prettier formatting checks
  - ESLint linting (zero warnings)
  - TypeScript strict type checking
  - npm security audit (high severity)
  - Vitest unit tests
  - Production build verification
- **Rust Security**: cargo-audit integrated into backend CI
- **Local Validation**: `npm run ci:check` for pre-push checks

### API Examples

**Create a Job**
```bash
curl -X POST http://localhost:8080/api/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "source": "/data/backup",
    "destination": "s3://bucket/backup",
    "compress": true,
    "verify": true,
    "parallel_workers": 4
  }'
```

**Get Job Status**
```bash
curl http://localhost:8080/api/jobs/1
```

**WebSocket Monitoring**
```javascript
const ws = new WebSocket('ws://localhost:8080/ws/1');
ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log('Progress:', update.progress);
};
```

### Development

**Backend (Control Plane)**
```bash
cd crates/orbit-web
cargo watch -x 'run --bin orbit-server'  # Auto-reload on changes
cargo check  # Quick compilation check
cargo audit  # Security vulnerability scan
```

**Frontend (Dashboard)**
```bash
cd dashboard
npm install              # Install dependencies (first time)
npm run dev              # Vite HMR enabled
npm run ci:check         # Run all checks before pushing
npm run format:fix       # Auto-fix code formatting
npm run typecheck        # TypeScript validation
npm test                 # Run unit tests
```

**API Documentation**
```bash
# Generate and open API docs
cd crates/orbit-web
cargo doc --open -p orbit-server
```

**Pre-Push Checklist**
```bash
# Backend
cargo fmt --all --check
cargo clippy --all
cargo test
cargo audit

# Frontend
cd dashboard
npm run ci:check  # Runs: typecheck + lint + format:check + test
```

### Configuration

**Environment Variables:**
```bash
# Control Plane
export ORBIT_SERVER_HOST=0.0.0.0       # Bind address (default: 127.0.0.1)
export ORBIT_SERVER_PORT=8080          # API port (default: 8080)
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)  # REQUIRED for production
export ORBIT_MAGNETAR_DB=magnetar.db   # Job database path
export ORBIT_USER_DB=users.db          # Auth database path

# Dashboard
# Edit dashboard/.env if needed
VITE_API_URL=http://localhost:8080
```

### Migration from v1.0 (Nebula)

The v2.2.0 architecture is a complete rewrite. Key changes:

| v1.0 (Nebula) | v2.2.0 (Control Plane) |
|---------------|------------------------|
| Leptos SSR | Axum REST API + React SPA |
| `orbit-web` binary | `orbit-server` + separate dashboard |
| Monolithic | Decoupled microservices |
| Server-side rendering | Client-side rendering |
| `cargo leptos watch` | `cargo run` + `npm run dev` |
| `/pkg` WASM assets | Static JSON API |

**Breaking Changes:**
- `orbit serve` now **only** starts the API (no UI bundled)
- Dashboard must be hosted separately or via CDN
- API endpoints remain compatible but are now OpenAPI-documented
- Authentication flow unchanged (JWT cookies)

### Deployment

**Production Checklist:**
- [ ] Set `ORBIT_JWT_SECRET` (minimum 32 characters)
- [ ] Change default admin password
- [ ] Configure CORS for your dashboard domain
- [ ] Use HTTPS (reverse proxy recommended: nginx/Caddy)
- [ ] Set up persistent volumes for databases
- [ ] Configure firewall rules (allow 8080 for API, 5173 for dev dashboard)
- [ ] Enable request logging (`RUST_LOG=info`)

**Docker Compose Example** (Coming soon)

### Roadmap

- ✅ v2.2.0-alpha.1 - Basic separation, API refactoring, React scaffolding
- 🚧 v2.2.0-alpha.2 - Interactive job creation UI, pipeline visual editor
- 🚧 v2.2.0-beta.1 - Complete dashboard features, duration estimation API
- 🚧 v2.2.0-rc.1 - Production hardening, performance optimization
- 🚧 v2.2.0 - Stable release with full documentation

### Troubleshooting

**Control Plane won't start**
```bash
# Check if port is in use
lsof -i :8080  # Unix
netstat -ano | findstr :8080  # Windows

# Check logs
RUST_LOG=debug cargo run --bin orbit-server
```

**Dashboard can't connect to API**
```bash
# Verify Control Plane is running
curl http://localhost:8080/api/health

# Check CORS configuration in server.rs
# Ensure dashboard origin is allowed
```

**JWT Authentication fails**
```bash
# Ensure JWT_SECRET is set
echo $ORBIT_JWT_SECRET

# Generate a new secret
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)
```

### Support & Documentation

- 📖 **API Docs**: http://localhost:8080/swagger-ui (when running)
- 📁 **Source**: [crates/orbit-web/](crates/orbit-web/) (Control Plane), [dashboard/](dashboard/) (React app)
- 📝 **CHANGELOG**: [CHANGELOG.md](CHANGELOG.md#architecture-shift---orbit-control-plane-v220-alpha-breaking)
- 🐛 **Issues**: [GitHub Issues](https://github.com/saworbit/orbit/issues)

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
| `cargo build --features api` | ✅ **Zero vulnerabilities** | Web dashboard (SQLite only) |
| `cargo build --features smb-native` | ⚠️ **Optional advisory** | SMB protocol (see note below) |
| `cargo build --features full` | ⚠️ **Optional advisory** | Testing & development only |

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

### ✅ Core Features Implemented (v0.4.1 - v0.6.0)

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
- Magnetar: Idempotent job state machine with SQLite + redb backends
- Magnetar Resilience Module: Circuit breaker, connection pooling, rate limiting
- Delta Detection: rsync-inspired efficient transfers with block-based diffing
- Metadata Preservation & Transformation
- Inclusion/Exclusion Filters: Glob, regex, and path patterns
- Progress Reporting & Operational Controls: Bandwidth limiting, concurrency control
- Manifest + Starmap + Audit integration
- Structured telemetry with JSON Lines

**Alpha/Experimental:**
- V2 Architecture (CDC, semantic replication, global dedup)
- SMB2/3 native implementation (awaiting upstream fix)
- **Orbit Control Plane v2.2.0-alpha.2** with React Dashboard
  - Visual Pipeline Editor (React Flow)
  - Interactive File Browser with filesystem navigation
  - Job Management UI with real-time progress tracking
  - REST API with OpenAPI documentation

### 🚧 In Progress (v0.6.0)

- Stabilizing V2 architecture components (CDC, semantic replication)
- Expanding test coverage for newer features
- Real-world validation of S3 and SSH backends
- Enhanced CLI with subcommands
- Web GUI interactive dashboard (Nebula beta)

### 🔮 Planned (v0.6.0+)

#### CLI Improvements
- Friendly subcommands (`orbit cp`, `orbit sync`, `orbit run`) as aliases
- Protocol-specific flags (`--smb-user`, `--region`, `--storage-class`)
- File watching mode (`--watch`)
- Interactive mode with prompts

#### New Protocols
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
# Clone and build (includes S3, SMB, SSH by default)
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build

# Run tests (includes S3 backend tests)
cargo test

# Run with all features (adds extended-metadata, delta-manifest)
cargo build --features full
cargo test --features full

# Minimal build (no network backends or GUI)
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
- **🎨 Control Plane v2.2.0-alpha.2 Deployment:** [`DEPLOYMENT_GUIDE_V2.2.0-alpha.2.md`](DEPLOYMENT_GUIDE_V2.2.0-alpha.2.md) ⭐ **NEW!**
- **Nebula MVP Summary:** [`crates/orbit-web/NEBULA_MVP_SUMMARY.md`](crates/orbit-web/NEBULA_MVP_SUMMARY.md) ⭐ **v1.0.0-alpha.2**
- **Nebula Changelog:** [`crates/orbit-web/CHANGELOG.md`](crates/orbit-web/CHANGELOG.md) ⭐ **NEW!**
- **Nebula README:** [`crates/orbit-web/README.md`](crates/orbit-web/README.md) ⭐ **v1.0.0-alpha.2**
- **Web Dashboard (v2.2.0):** See Control Plane documentation
- **GUI Integration:** [`docs/GUI_INTEGRATION.md`](docs/GUI_INTEGRATION.md)
- **Testing & Validation Scripts:** [`docs/guides/TESTING_SCRIPTS_GUIDE.md`](docs/guides/TESTING_SCRIPTS_GUIDE.md) ⭐ **NEW!**
- **S3 Guide:** [`docs/guides/S3_USER_GUIDE.md`](docs/guides/S3_USER_GUIDE.md)
- **GCS Guide:** [`docs/guides/GCS_USER_GUIDE.md`](docs/guides/GCS_USER_GUIDE.md)
- **Disk Guardian:** [`docs/architecture/DISK_GUARDIAN.md`](docs/architecture/DISK_GUARDIAN.md)
- **Magnetar:** [`crates/magnetar/README.md`](crates/magnetar/README.md) ⭐ **NEW!**
- **Resilience Module:** [`crates/magnetar/src/resilience/README.md`](crates/magnetar/src/resilience/README.md) ⭐ **NEW!**
- **Delta Detection:** [`docs/guides/DELTA_DETECTION_GUIDE.md`](docs/guides/DELTA_DETECTION_GUIDE.md) and [`docs/guides/DELTA_QUICKSTART.md`](docs/guides/DELTA_QUICKSTART.md) ⭐ **NEW!**
- **Filter System:** [`docs/guides/FILTER_SYSTEM.md`](docs/guides/FILTER_SYSTEM.md) ⭐ **NEW!**
- **Progress & Concurrency:** [`docs/architecture/PROGRESS_AND_CONCURRENCY.md`](docs/architecture/PROGRESS_AND_CONCURRENCY.md) ⭐ **NEW!**
- **Resume System:** [`docs/architecture/RESUME_SYSTEM.md`](docs/architecture/RESUME_SYSTEM.md)
- **Protocol Guide:** [`docs/guides/PROTOCOL_GUIDE.md`](docs/guides/PROTOCOL_GUIDE.md)

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
- **Filter Example:** [`examples/filters/example.orbitfilter`](examples/filters/example.orbitfilter) ⭐ **NEW!**
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
