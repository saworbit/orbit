# 🚀 Orbit

> **O**pen **R**esilient **B**ulk **I**nformation **T**ransfer

**The intelligent file transfer tool that never gives up** 💪

[![CI](https://github.com/saworbit/orbit/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/saworbit/orbit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![GitHub](https://img.shields.io/github/stars/saworbit/orbit?style=social)](https://github.com/saworbit/orbit)

---

## 📑 Table of Contents

- [What is Orbit?](#-what-is-orbit)
- [Why Orbit?](#-why-orbit)
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

Orbit is a **blazingly fast** 🔥 file transfer tool built in Rust that combines enterprise-grade reliability with cutting-edge performance. Whether you're backing up terabytes of data, syncing files across continents, transferring to network shares, or moving data to the cloud, Orbit has you covered.

**Key Philosophy:** Intelligence, resilience, and speed without compromise.

---

## ✨ Why Orbit?

| Feature | Benefit |
|---------|---------|
| 🚄 **3× Faster** | Zero-copy system calls transfer at device speed (instant APFS cloning on macOS!) |
| 🛡️ **Bulletproof** | Smart resume with chunk verification, checksums, corruption detection |
| 🧠 **Smart** | Adapts strategy based on environment (zero-copy, compression, buffered) |
| 🛡️ **Safe** | Disk Guardian prevents mid-transfer failures with pre-flight checks |
| 🌐 **Protocol Ready** | Local, **SSH/SFTP**, SMB/CIFS, **S3**, with unified backend API |
| 🌐 **GUI Ready** | Launch the web dashboard with `orbit serve` (enabled by default) |
| 📊 **Fully Auditable** | Structured JSON telemetry for every operation |
| 🧩 **Modular** | Clean architecture with reusable crates |
| 🌍 **Cross-Platform** | Linux, macOS, Windows with native optimizations |

---

## 🔑 Key Features

### 🔄 Error Handling & Retries: Never Give Up

**NEW in v0.4.1!** Enterprise-grade error handling with intelligent retry logic and comprehensive diagnostics.

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

**NEW in v0.5.0!** Automatic configuration validation and optimization that ensures safe, performant transfers.

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
orbit = { version = "0.5.0", features = ["extended-metadata"] }
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

**NEW in v0.5.0: Orbit V2 Architecture** 🚀
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
| 🗂️ **Local** | ✅ Stable | Built-in | Local filesystem with zero-copy optimization |
| 🔐 **SSH/SFTP** | ✅ **Stable** | `ssh-backend` | Remote filesystem access via SSH/SFTP with async I/O |
| 🌐 **SMB/CIFS** | ✅ **Stable** | `smb-native` | Native SMB2/3 client (pure Rust, no dependencies) |
| ☁️ **S3** | ✅ **Stable** | `s3-native` | Amazon S3 and compatible object storage (MinIO, LocalStack) |
| ☁️ **Azure Blob** | 🚧 Planned | - | Microsoft Azure Blob Storage |
| ☁️ **GCS** | 🚧 Planned | - | Google Cloud Storage |
| 🌐 **WebDAV** | 🚧 Planned | - | WebDAV protocol support |

#### 🆕 Unified Backend Abstraction (v0.5.0 - Streaming API)

**NEW!** Write once, run on any storage backend. The backend abstraction provides a consistent async API with **streaming I/O** for memory-efficient large file transfers:

```rust
use orbit::backend::{Backend, LocalBackend, SshBackend, S3Backend, SmbBackend, SmbConfig};
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
```

**Features:**
- ✅ **URI-based configuration**: `ssh://user@host/path`, `s3://bucket/key`, `smb://user@server/share/path`, etc.
- ✅ **Streaming I/O**: Upload files up to **5TB** to S3 with ~200MB RAM (v0.5.0 ⭐)
- ✅ **Constant Memory Listing**: List millions of S3 objects with ~10MB RAM (v0.5.0 ⭐)
- ✅ **Automatic Multipart Upload**: S3 files ≥5MB use efficient chunked transfers (v0.5.0 ⭐)
- ✅ **Optimized Download**: Sliding window concurrency for 30-50% faster S3 downloads (v0.5.0 ⭐)
- ✅ **Metadata operations**: Set permissions, timestamps, xattrs, ownership
- ✅ **Extensibility**: Plugin system for custom backends
- ✅ **Type-safe**: Strong typing with comprehensive error handling
- ✅ **Security**: Built-in secure credential handling

📖 **Full Guide:** [docs/guides/BACKEND_GUIDE.md](docs/guides/BACKEND_GUIDE.md)
📖 **Migration Guide:** [BACKEND_STREAMING_GUIDE.md](BACKEND_STREAMING_GUIDE.md) ⭐ **NEW!**

#### 🆕 SSH/SFTP Remote Access (v0.5.0)

Transfer files securely over SSH/SFTP with production-ready async implementation:

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

#### 🆕 S3 Cloud Storage (v0.5.0 - Streaming Optimized)

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
- ✅ **Streaming multipart upload** - Files ≥5MB automatically use multipart with **5TB max file size** (v0.5.0 ⭐)
- ✅ **Constant memory usage** - ~200MB RAM for any file size upload/download (v0.5.0 ⭐)
- ✅ **Optimized downloads** - Sliding window concurrency for 30-50% faster transfers (v0.5.0 ⭐)
- ✅ **Lazy S3 pagination** - List millions of objects with ~10MB RAM (v0.5.0 ⭐)
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
📖 **Streaming Guide:** See [`BACKEND_STREAMING_GUIDE.md`](BACKEND_STREAMING_GUIDE.md) ⭐ **NEW!**

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

### 📊 Audit and Telemetry

Every copy operation automatically emits structured audit events for full observability and compliance tracking.

**Enable Audit Logging:**
```bash
# Via CLI flag
orbit copy /source /dest --audit-log ./audit.log

# Via configuration file
audit_format = "json"
audit_log_path = "/var/log/orbit_audit.log"
```

**Example Audit Log (JSON Lines):**
```json
{"timestamp":"2025-10-25T16:42:19Z","job":"orbit-1a2b3c4d-5e6f","source":"/local/data/","destination":"s3://my-bucket/backups/","protocol":"s3","bytes_transferred":104857600,"duration_ms":2341,"compression":"zstd","compression_ratio":2.3,"checksum_algorithm":"blake3","checksum_match":true,"storage_class":"INTELLIGENT_TIERING","multipart_parts":20,"status":"success","retries":0,"starmap_node":"orbit.node.cloud-backup"}
```

**Audit Event Lifecycle:**
1. **started** — Emitted when operation begins (with expected bytes)
2. **progress** — Optional periodic updates during long transfers
3. **success/failure** — Final status with complete metrics

**Audit Features:**
- **JSON Lines format** — One event per line, machine-parseable
- **CSV format** — Alternative format for spreadsheet analysis
- **ISO 8601 timestamps** — With timezone for global deployments
- **Job correlation** — Unique job IDs link related events
- **Full metrics** — Bytes, duration, compression ratio, checksum status
- **Protocol-specific fields** — Storage class, multipart parts (S3)
- **Graceful degradation** — Audit failures don't abort copy operations
- **Ready for ingestion** — ELK, Loki, Datadog, Splunk compatible
- **Starmap node correlation** — For distributed transfer tracking

---

## 🚀 Quick Start

### Install

```bash
# From source
git clone https://github.com/saworbit/orbit.git
cd orbit

# Minimal build (local copy only, ~10MB binary) - DEFAULT
cargo build --release

# With network protocols (S3, SMB, SSH)
cargo build --release --features network

# With Web GUI
cargo build --release --features gui

# Full build (everything)
cargo build --release --features full

# Install to system
sudo cp target/release/orbit /usr/local/bin/

# Or with cargo install
cargo install --path .                    # Minimal
cargo install --path . --features network  # With network
cargo install --path . --features full    # Everything
```

> **NEW in v0.5/v0.6:** Orbit now defaults to a minimal build (just local copy with zero-copy optimizations) for fastest compile times and smallest binaries. Network protocols and GUI are opt-in via feature flags.

### Feature Flags & Binary Sizes

**v0.5+ Performance Improvements:**
- 🎯 **60% smaller default binary** — Minimal build is ~10MB (was ~50MB)
- ⚡ **50% faster compilation** — Default build in ~60s (was ~120s)
- 🔒 **Reduced attack surface** — No web server code in default CLI build
- 🚀 **2x Delta throughput** — Gear64 hash replaces Adler-32 for better collision resistance

| Feature | Description | Binary Size | Default |
|---------|-------------|-------------|---------|
| `zero-copy` | OS-level zero-copy syscalls for maximum speed | +1MB | ✅ Yes |
| `network` | All network protocols (S3, SMB, SSH) | +25MB | ❌ No |
| `s3-native` | Amazon S3 and compatible storage | +15MB | ❌ No |
| `smb-native` | Native SMB2/3 network shares | +8MB | ❌ No |
| `ssh-backend` | SSH/SFTP remote access | +5MB | ❌ No |
| `gui` | Web-based dashboard (`orbit serve`) | +15MB | ❌ No |
| `delta-manifest` | SQLite-backed delta persistence | +3MB | ❌ No |
| `extended-metadata` | xattr + ownership (Unix/Linux/macOS only) | +500KB | ❌ No |
| `full` | All features enabled | +50MB | ❌ No |

```bash
# Minimal: Fast local copies only (~10MB)
cargo build --release
cargo install orbit

# Network: Add S3, SMB, SSH support (~35MB)
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

Orbit is built from clean, reusable crates:

| Crate | Purpose | Status |
|-------|---------|--------|
| 🧩 `core-manifest` | Manifest parsing and job orchestration | ✅ Stable |
| 🌌 `core-starmap` | Job planner and dependency graph | ✅ Stable |
| 🌌 `core-starmap::universe` | Global deduplication index (V2) | ✅ **v0.5.0** |
| 🌌 `core-starmap::migrate` | V1→V2 migration utilities | ✅ **v0.5.0** |
| 🧬 `core-cdc` | FastCDC content-defined chunking (V2) | ✅ **v0.5.0** |
| 🧠 `core-semantic` | Intent-based replication (V2) | ✅ **v0.5.0** |
| 📊 `core-audit` | Structured logging and telemetry | ✅ Stable |
| ⚡ `core-zero-copy` | OS-level optimized I/O | ✅ Stable |
| 🗜️ `core-compress` | Compression and decompression | ✅ Stable |
| 🛡️ `disk-guardian` | Pre-flight space & integrity checks | ✅ Stable |
| 🧲 `magnetar` | Idempotent job state machine (SQLite + redb) | ✅ Stable |
| 🛡️ `magnetar::resilience` | Circuit breaker, connection pool, rate limiter | ✅ Stable |
| 🌐 `protocols` | Network protocol implementations | ✅ S3, 🟡 SMB |
| 🌐 `orbit-web` | Enterprise web control center (Nebula) | ✅ **v1.0.0-alpha.2** |
| 🕵️ `core-watcher` | Monitoring beacon | 🚧 Planned |
| 🧪 `wormhole` | Forward-error correction | 🚧 Dev |

This structure ensures isolation, testability, and reusability.

---

## 🖥️ Web GUI - Nebula (v1.0.0-alpha.2)

**Orbit Nebula** is a complete ground-up rewrite of the web interface, transforming it from a basic polling dashboard into an enterprise-grade, real-time data orchestration control center. Built with production-ready authentication, WebSocket streaming, and a comprehensive security stack.

### Status: v1.0.0-alpha.2 (100% Backend Complete - Fully Compiling)

**Codename:** Nebula
**What's New:** Complete rewrite with ~2,000 lines of production Rust implementing JWT auth, real-time events, RESTful APIs, and comprehensive security. **Alpha.2 achieves clean compilation with 0 errors and 0 warnings.**

### How to launch the GUI from the CLI

⚠️ **Note:** v1.0.0-alpha.2 is API-focused with production-ready backend. Full interactive UI coming in beta.1.

1) Build with defaults (GUI enabled): `cargo build --release`
2) Start the server: `./target/release/orbit serve --addr 127.0.0.1:8080`
3) Open `http://127.0.0.1:8080` in your browser.
4) Default credentials: `admin` / `orbit2025` (⚠️ Change in production!)

Tips:
- CLI-only build: `cargo build --release --no-default-features --features zero-copy`
- Set JWT secret: `export ORBIT_JWT_SECRET=your-secret-key`

### Why Use Nebula?

- **Enterprise Authentication** — JWT + Argon2 password hashing with RBAC (Admin/Operator/Viewer)
- **Real-Time Updates** — WebSocket streaming with <500ms latency for live job events
- **Multi-User Support** — Role-based access control with secure session management
- **Production Security** — httpOnly cookies, encrypted passwords, automatic token expiration
- **RESTful API** — Complete backend API ready for custom frontends
- **Crash Recovery** — Resume monitoring after disconnects via Magnetar persistence

### What's Implemented (v1.0.0-alpha.2)

#### ✅ 1. Authentication & Security (100% Complete)
- **JWT Authentication** — 24-hour token expiration with automatic validation
- **Argon2 Password Hashing** — OWASP-recommended with salt
- **RBAC** — Three roles: Admin, Operator, Viewer with permission checking
- **httpOnly Cookies** — Secure token storage preventing XSS attacks
- **Default Admin Account** — Auto-created on first run (`admin` / `orbit2025`)
- **SQLite User Database** — Separate from job state for security isolation

#### ✅ 2. Real-Time Event System (100% Complete)
- **WebSocket Handler** — JWT-validated connections for live updates
- **Broadcast Channels** — Sub-500ms latency event distribution
- **6 Event Types** — JobUpdated, TransferSpeed, JobCompleted, JobFailed, AnomalyDetected, ChunkCompleted
- **Role-Based Filtering** — Events filtered by user permissions
- **Job-Specific Streams** — Subscribe to individual job updates via `/ws/:job_id`

#### ✅ 3. RESTful API (100% Complete)
- **Auth Endpoints** — POST `/api/auth/login`, `/api/auth/logout`, GET `/api/auth/me`
- **Job CRUD** — List, create, get stats, delete, run, cancel jobs
- **Backend Management** — List configured backends (S3, SMB, Local)
- **Health Check** — GET `/api/health` for monitoring
- **Leptos Server Functions** — Type-safe RPC-style endpoints

#### ✅ 4. State Management (100% Complete)
- **Magnetar Integration** — SQLite-backed persistent job state
- **User Database Pool** — Async connection pooling with sqlx 0.8
- **Event Broadcasting** — 1,000-message channel buffer
- **Backend Configuration** — Thread-safe storage for S3/SMB credentials
- **Crash Recovery** — Automatic state restoration on server restart

### Quick Start (v1.0.0-alpha.2)

⚠️ **Alpha Status:** Backend fully compiling and production-ready. Interactive UI coming in beta.1.

#### Automated Startup Scripts (Easiest Way)

We provide automated startup scripts that handle all setup for you:

**Unix/Linux/macOS:**
```bash
cd crates/orbit-web
chmod +x start-nebula.sh
./start-nebula.sh
```

**Windows:**
```cmd
cd crates\orbit-web
start-nebula.bat
```

**What the scripts do:**
- ✅ Check for Rust/Cargo installation
- ✅ Install wasm32-unknown-unknown target if missing
- ✅ Generate JWT secret automatically (or use your `ORBIT_JWT_SECRET`)
- ✅ Create data directories
- ✅ Build the project (only if needed)
- ✅ Display all API endpoints and default credentials
- ✅ Start the server

**Environment Variables (Optional):**
```bash
# Customize before running the script
export ORBIT_JWT_SECRET=your-secret-key-minimum-32-chars
export ORBIT_MAGNETAR_DB=/path/to/magnetar.db
export ORBIT_USER_DB=/path/to/users.db
export ORBIT_HOST=0.0.0.0
export ORBIT_PORT=3000
```

#### API Testing with curl

```bash
# Start the server
cd crates/orbit-web
cargo run --release

# Login (returns JWT cookie)
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"orbit2025"}' \
  -c cookies.txt

# Get current user info
curl http://localhost:8080/api/auth/me -b cookies.txt

# Health check
curl http://localhost:8080/api/health
```

#### WebSocket Testing

```javascript
// Connect to WebSocket (requires JWT cookie from login)
const ws = new WebSocket('ws://localhost:8080/ws/job-123');

ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log('Job update:', update);
  // Receives: JobUpdated, TransferSpeed, JobCompleted, etc.
};
```

#### Environment Variables

```bash
# Set JWT secret (REQUIRED for production)
export ORBIT_JWT_SECRET=your-secret-key-minimum-32-chars

# Set database paths (optional)
export ORBIT_MAGNETAR_DB=/var/lib/orbit/magnetar.db
export ORBIT_USER_DB=/var/lib/orbit/users.db

# Run server
cargo run --release
```

#### Development Mode

```bash
# Install prerequisites
cargo install cargo-leptos
rustup target add wasm32-unknown-unknown

# Run with hot reload
cd crates/orbit-web
cargo leptos watch
```

### Architecture (v1.0.0-alpha.2)

Built with enterprise-grade Rust technologies:

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Authentication** | JWT + Argon2 | Token-based auth with secure password hashing |
| **Backend** | Axum 0.7 | High-performance async HTTP server |
| **Real-time** | WebSockets | Sub-500ms latency event streaming |
| **State** | Magnetar (SQLite) | Persistent job state with crash recovery |
| **User DB** | SQLx 0.8 + SQLite | Async user authentication database |
| **Frontend** | Leptos 0.6 | Full-stack Rust framework (simplified for MVP) |
| **Runtime** | Tokio | Efficient async task execution |

**Key Design Decisions:**
- **Separate User DB** — Authentication isolated from job state for security
- **Runtime SQL Queries** — Flexibility without compile-time DATABASE_URL requirement
- **Backend-First MVP** — Solid API foundation before UI polish
- **API-Driven** — Backend APIs can be consumed by any frontend
- **Production Security** — JWT, Argon2, RBAC, httpOnly cookies from day one

### API Reference (v1.0.0-alpha.2)

#### Authentication Endpoints

**Login**
```bash
POST /api/auth/login
Content-Type: application/json
Body: {"username":"admin","password":"orbit2025"}
# Returns: JWT cookie (httpOnly, 24h expiration) + user info
```

**Logout**
```bash
POST /api/auth/logout
# Clears authentication cookie
```

**Get Current User**
```bash
GET /api/auth/me
Cookie: orbit_token=<jwt>
# Returns: {"id":"...","username":"admin","role":"Admin"}
```

#### Health Check

```bash
GET /api/health
# Returns: {"status":"ok","service":"orbit-web","version":"1.0.0"}
```

#### WebSocket Events (Requires JWT Cookie)

**Real-Time Job Updates**
```bash
WS /ws/:job_id
Cookie: orbit_token=<jwt>
# Streams JSON events:
# - JobUpdated {job_id, status, progress, timestamp}
# - TransferSpeed {job_id, bytes_per_sec, timestamp}
# - JobCompleted {job_id, total_bytes, duration_ms, timestamp}
# - JobFailed {job_id, error, timestamp}
# - AnomalyDetected {job_id, message, severity, timestamp}
# - ChunkCompleted {job_id, chunk_id, bytes, timestamp}
```

**Subscribe to All Jobs**
```bash
WS /ws
Cookie: orbit_token=<jwt>
# Receives events for all jobs (filtered by role permissions)
```

### Configuration (v1.0.0-alpha.2)

**Required Environment Variables:**
```bash
# JWT Secret (REQUIRED for production)
export ORBIT_JWT_SECRET=your-secret-key-minimum-32-characters

# Database paths (optional, defaults shown)
export ORBIT_MAGNETAR_DB=/path/to/magnetar.db  # Job state
export ORBIT_USER_DB=/path/to/users.db         # User auth

# Server configuration (optional)
export ORBIT_HOST=127.0.0.1
export ORBIT_PORT=8080

# Logging (optional)
export RUST_LOG=info,orbit_web=debug
```

**Security Checklist:**
- ✅ Set `ORBIT_JWT_SECRET` to a strong random value (min 32 chars)
- ✅ Change default admin password after first login
- ✅ Enable HTTPS/TLS in production (use reverse proxy)
- ✅ Configure CORS for production domain
- ✅ Restrict network access to trusted IPs

### Nebula Roadmap

**✅ v1.0.0-alpha.2** (COMPLETED) - Compilation Fixes
- ✅ Fixed Leptos server function type annotations
- ✅ Cleaned up unused imports
- ✅ Tested compilation and basic server startup
- ✅ 0 errors, 0 warnings

**v1.0.0-beta.1** (4-6 hours) - Interactive UI
- Complete interactive dashboard with live updates
- Job creation form with validation
- WebSocket-powered real-time progress bars
- Job control buttons (run, pause, cancel, delete)

**v1.0.0-beta.2** (8-12 hours) - Advanced Features
- File explorer with directory navigation
- Drag-and-drop file upload
- Backend credential management UI
- User management panel (Admin only)

**v1.0.0** (12-16 hours) - Production Release
- Telemetry dashboard with charts and graphs
- Visual pipeline builder with DAG visualization
- Dark mode theme
- PWA support for offline monitoring
- Comprehensive end-to-end testing

**v1.1.0+** - Future Enhancements
- SSO integration (SAML, OAuth2)
- Audit log viewer
- Multi-language support
- Mobile-optimized views

📖 **Complete Documentation:**
- **MVP Summary:** [`crates/orbit-web/NEBULA_MVP_SUMMARY.md`](crates/orbit-web/NEBULA_MVP_SUMMARY.md) ⭐ **v1.0.0-alpha.2**
- **Changelog:** [`crates/orbit-web/CHANGELOG.md`](crates/orbit-web/CHANGELOG.md) ⭐ **NEW!**
- **Full README:** [`crates/orbit-web/README.md`](crates/orbit-web/README.md) ⭐ **UPDATED!**
- **API Docs:** Run `cargo doc --open -p orbit-web`

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
- **Inclusion/Exclusion Filters: Selective file processing with glob, regex, and path patterns** ⭐ **NEW!**
- **Progress Reporting & Operational Controls: Enhanced progress bars, dry-run, bandwidth limiting, concurrency control** ⭐ **NEW!**
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

- ☁️ Azure Blob and GCS implementations
- 🌐 Resolving SMB upstream dependencies
- 🧪 Testing on various platforms
- 📚 Documentation improvements
- 🐛 Bug reports and fixes

---

## 📚 Documentation

### User Guides
- **Quick Start:** This README
- **Nebula MVP Summary:** [`crates/orbit-web/NEBULA_MVP_SUMMARY.md`](crates/orbit-web/NEBULA_MVP_SUMMARY.md) ⭐ **v1.0.0-alpha.2**
- **Nebula Changelog:** [`crates/orbit-web/CHANGELOG.md`](crates/orbit-web/CHANGELOG.md) ⭐ **NEW!**
- **Nebula README:** [`crates/orbit-web/README.md`](crates/orbit-web/README.md) ⭐ **v1.0.0-alpha.2**
- **Web GUI (v0.5.0):** [`docs/WEB_GUI.md`](docs/WEB_GUI.md) (deprecated, see Nebula docs)
- **GUI Integration:** [`docs/GUI_INTEGRATION.md`](docs/GUI_INTEGRATION.md)
- **S3 Guide:** [`docs/S3_USER_GUIDE.md`](docs/S3_USER_GUIDE.md)
- **Disk Guardian:** [`docs/DISK_GUARDIAN.md`](docs/DISK_GUARDIAN.md)
- **Magnetar:** [`crates/magnetar/README.md`](crates/magnetar/README.md) ⭐ **NEW!**
- **Resilience Module:** [`crates/magnetar/src/resilience/README.md`](crates/magnetar/src/resilience/README.md) ⭐ **NEW!**
- **Delta Detection:** [`docs/DELTA_DETECTION_GUIDE.md`](docs/DELTA_DETECTION_GUIDE.md) and [`docs/DELTA_QUICKSTART.md`](docs/DELTA_QUICKSTART.md) ⭐ **NEW!**
- **Filter System:** [`docs/FILTER_SYSTEM.md`](docs/FILTER_SYSTEM.md) ⭐ **NEW!**
- **Progress & Concurrency:** [`docs/PROGRESS_AND_CONCURRENCY.md`](docs/PROGRESS_AND_CONCURRENCY.md) ⭐ **NEW!**
- **Resume System:** [`docs/RESUME_SYSTEM.md`](docs/RESUME_SYSTEM.md)
- **Protocol Guide:** [`docs/PROTOCOL_GUIDE.md`](docs/PROTOCOL_GUIDE.md)

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
