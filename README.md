# Orbit

> **O**pen **R**esilient **B**ulk **I**nformation **T**ransfer

**The intelligent file transfer tool that never gives up.**

[![CI](https://github.com/saworbit/orbit/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/saworbit/orbit/actions/workflows/ci.yml)
[![Security Audit](https://github.com/saworbit/orbit/actions/workflows/compliance.yml/badge.svg)](https://github.com/saworbit/orbit/actions/workflows/compliance.yml)
[![codecov](https://codecov.io/gh/saworbit/orbit/branch/main/graph/badge.svg)](https://codecov.io/gh/saworbit/orbit)
[![Release](https://img.shields.io/github/v/release/saworbit/orbit)](https://github.com/saworbit/orbit/releases)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![GitHub](https://img.shields.io/github/stars/saworbit/orbit?style=social)](https://github.com/saworbit/orbit)

---

## Project Status: Alpha (v0.6.0)

Orbit is in active development and should be considered alpha-quality software.

- **Safe for**: Experimentation, evaluation, non-critical workloads, development environments
- **Use with caution for**: Important data transfers (test thoroughly first, maintain backups)
- **Not recommended for**: Mission-critical production systems without extensive testing

APIs may change between versions. Some features are experimental and marked as such. See the [Feature Maturity Matrix](#feature-maturity-matrix) for per-feature stability.

---

## What is Orbit?

Orbit is a file transfer tool built in Rust that combines reliability with performance. Whether you're backing up data, syncing files, transferring to network shares, or moving data to the cloud, Orbit provides intelligent defaults and powerful features.

**Philosophy:** Intelligence, resilience, and speed.

---

## Why Orbit vs rsync / rclone?

| Capability | rsync | rclone | **Orbit** |
|------------|-------|--------|-----------|
| **Zero-copy transfers** | No | No | Yes (platform-specific syscalls, APFS cloning) |
| **Pre-flight safety** | No | No | Yes (Disk Guardian: space, permissions, path checks) |
| **Smart auto-tuning** | No | No | Yes (Config Optimizer: hardware probing, profile presets) |
| **Content-defined chunking** | No | No | Yes (Gear Hash CDC, 99.1% shift resilience) |
| **Global deduplication** | No | No | Yes (Universe index, cross-file/backup dedup) |
| **Plain-English previews** | `--dry-run` | `--dry-run` | `orbit explain` (human-readable transfer plan) |
| **Interactive setup** | Manual | `rclone config` | `orbit init` (hardware-aware wizard) |
| **Cloud backends** | Limited | Excellent (40+) | S3, Azure, GCS, SSH, SMB (growing) |
| **Resume/checkpoint** | Partial | Partial | Chunk-level verification with smart restart |
| **Compression** | Built-in (zlib) | Limited | LZ4, Zstd (auto-selection by destination) |
| **Structured audit** | No | No | JSONL with HMAC-chained tamper-evident log |
| **Cross-platform** | Linux/macOS | All | Linux, macOS, Windows (native optimizations each) |
| **Maturity** | Battle-tested | Battle-tested | Alpha (early-stage, actively developed) |

Orbit aims to combine rsync/rclone reliability with modern systems programming strengths. rclone remains the best choice for broad cloud provider support; rsync for proven Unix-to-Unix sync. Orbit differentiates on zero-copy performance, safety checks, and intelligent defaults.

---

## Quick Start

```bash
# Install a pre-built binary (Linux x86_64, static musl — see /releases for macOS universal, aarch64, Windows)
curl -L https://github.com/saworbit/orbit/releases/download/v0.6.0/orbit-v0.6.0-x86_64-unknown-linux-musl.tar.gz | tar xz
sudo install -m 755 orbit /usr/local/bin/

# …or build from source
git clone https://github.com/saworbit/orbit.git
cd orbit && cargo build --release

# Run the setup wizard
orbit init

# Copy files
orbit /data /backup -R

# Shorthand subcommands
orbit sync /data /backup              # Sync mode
orbit backup /data /backup            # Checksums + Zstd + resume
orbit mirror /data /backup            # Exact replica

# Preview what Orbit would do
orbit explain /data /backup -R --zstd

# Profiles
orbit /data /backup -R --profile fast     # Max speed
orbit /data /backup -R --profile safe     # Max reliability
orbit /data /backup -R --profile network  # Remote-optimized

# Cloud transfers (requires --features network)
orbit /data s3://bucket/path -R
orbit /data ssh://user@host:/path -R
```

**[Full Getting Started Guide](docs/GETTING_STARTED.md)** | **[CLI Quick Reference](#cli-quick-reference)**

---

## Feature Maturity Matrix

Understanding feature stability helps you make informed decisions about what to use.

### Stable Core

| Feature | Maturity | Notes |
|---------|----------|-------|
| **Core File Copy (Buffered)** | Stable | Well-tested, safe for production use |
| **Zero-Copy Optimization** | Stable | Platform-specific (Linux, macOS, Windows) |
| **Compression (LZ4, Zstd)** | Stable | Reliable for most workloads |
| **Checksum Verification** | Stable | BLAKE3 (default), SHA-256 |
| **Local Filesystem** | Stable | Primary use case, thoroughly tested |
| **OrbitSystem Abstraction** | Stable | I/O abstraction layer |

### Beta (Functional, Needs Real-World Validation)

| Feature | Maturity | Notes |
|---------|----------|-------|
| **Resume/Checkpoint** | Beta | Works well, needs more edge-case testing |
| **SSH/SFTP Backend** | Beta | Functional, needs more real-world testing |
| **S3 Backend** | Beta | Works well, multipart upload is newer |
| **SMB Backend** | Beta | v0.11.0 upgrade, ready for integration testing |
| **Azure Blob Backend** | Beta | Via object_store crate, added in v0.6.0 |
| **GCS Backend** | Beta | Via object_store crate, added in v0.6.0 |
| **Delta Detection (V1)** | Beta | rsync-style algorithm |
| **Disk Guardian** | Beta | Pre-flight checks, works well |
| **Config Optimizer** | Beta | Config validation with active probing |
| **Init Wizard** | Beta | Interactive setup with `orbit init` |
| **Filter System** | Beta | Glob/regex filters |
| **Metadata Preservation** | Beta | Extended attributes are platform-specific |
| **Transfer Shorthands** | Beta | `orbit sync`, `backup`, `mirror`, `cp` |
| **`orbit explain`** | Beta | Plain-English transfer plan preview |
| **`orbit history`** | Beta | Audit log viewer with `--json` output |
| **Auto-Compression** | Beta | `--compress auto` picks by destination |
| **Smart Error Suggestions** | Beta | Fuzzy matching, glob detection, path expansion |
| **Manifest System** | Beta | File tracking and verification |
| **Progress/Bandwidth Limiting** | Beta | Integrated across all modes |
| **Audit Logging** | Beta | Structured JSONL telemetry |
| **Global Deduplication (V3)** | Beta | Universe index, O(log N) scalability |
| **Sparse File Handling** | Beta | Zero-chunk detection, hole-aware writes |
| **Hardlink Preservation** | Beta | Inode tracking, `--preserve-hardlinks` |
| **In-Place Updates** | Beta | Safety tiers, `--inplace` |

### Alpha (Experimental -- Expect Changes)

| Feature | Notes |
|---------|-------|
| **V2 Architecture (CDC)** | Content-defined chunking |
| **Semantic Replication** | Priority-based transfers |
| **Link-Dest++ (Incremental Backup)** | Chunk-level reference hardlinking |
| **Transfer Journal (Batch Mode)** | Content-addressed operation journal |
| **Backpressure** | Dual-threshold flow control |
| **Penalization** | Exponential backoff deprioritization |
| **Dead-Letter Queue** | Bounded quarantine for permanent failures |
| **Health Monitor** | Continuous mid-transfer health checks |
| **Ref-Counted GC** | WAL-gated garbage collection |
| **Container Packing** | `.orbitpak` chunk packing |
| **Typed Provenance** | Structured event taxonomy |
| **Composable Prioritizers** | Chainable sort criteria |

> Alpha features are gated behind feature flags or `--smart` mode. The stable core works without them.

---

## Key Features

### Zero-Copy Performance

Platform-specific syscalls bypass userspace buffers entirely:
- **Linux**: `copy_file_range` (kernel-space copy)
- **macOS**: `fcopyfile` + APFS Copy-on-Write (instant cloning)
- **Windows**: `CopyFileExW`

### Disk Guardian (Pre-Flight Safety)

Automatically runs before every transfer to prevent mid-transfer failures:
- Disk space estimation with configurable safety margins
- Write permission and filesystem integrity checks
- Path validation and read-only filesystem detection

### Config Optimizer

Validates configuration and auto-tunes settings based on your hardware:
- CPU, RAM, and I/O throughput probing
- Conflict detection (e.g., incompatible flag combinations)
- Hardware probe caching for fast subsequent runs

### Smart Error Handling

- **Error classification**: Transient errors retry; permanent errors fail fast
- **Exponential backoff** with jitter to prevent thundering herd
- **Three modes**: Abort (default), Skip (batch-friendly), Partial (resume-friendly)
- **Fuzzy suggestions**: Typo detection, glob pattern hints, path expansion

### Structured Audit & Telemetry

- JSONL audit trail with HMAC-SHA256 chaining (tamper-evident)
- OpenTelemetry integration for distributed tracing
- `orbit history` for human-readable audit browsing
- Machine-readable `--json` output for all operations

### Multi-Protocol Backends

| Backend | Feature Flag | Notes |
|---------|-------------|-------|
| Local filesystem | (default) | Zero-copy, all platforms |
| SSH/SFTP | `ssh-backend` | Via ssh2 crate |
| Amazon S3 | `s3-native` | Via object_store (streaming multipart writes); rich `orbit s3 ...` CLI with presign / versioning / wildcards is opt-in under `s3-cli` |
| Azure Blob | `azure-native` | Via object_store crate (streaming multipart writes) |
| Google Cloud Storage | `gcs-native` | Via object_store crate (streaming multipart writes) |
| SMB2/3 | `smb-native` | Native pure-Rust |

### Content-Defined Chunking (V2 -- Alpha)

- Gear Hash rolling hash with variable-size chunks (8KB-256KB)
- 99.1% shift resilience for efficient delta transfers
- BLAKE3 content-addressable chunk IDs
- Global deduplication via Universe V3 index (redb-backed, O(log N))

> V2/V3 features are under active development. Use `--smart` to opt in.

For detailed feature documentation, see the [docs/](docs/) directory.

---

## Performance

### Local Transfer Benchmarks

| File Size | Traditional cp | Orbit (Zero-Copy) | Speedup |
|-----------|----------------|-------------------|---------|
| 10 MB | 12 ms | 8 ms | 1.5x |
| 1 GB | 980 ms | 340 ms | 2.9x |
| 10 GB | 9.8 s | 3.4 s | 2.9x |

macOS APFS: File copies complete **instantly** via Copy-on-Write cloning regardless of file size.

### S3 Transfer Performance

- **Streaming multipart writes** for objects above 8 MiB, with up to 4 in-flight parts
- **Parallel Workers**: Up to 256 concurrent file operations
- **Adaptive Chunking**: 5MB-2GB chunks based on file size

---

## Use Cases

```bash
# Cloud data lake ingestion
orbit /data/analytics s3://data-lake/raw/2025/ -R --parallel 16 --compress zstd:3

# Enterprise backup with manifest
orbit manifest plan --source /data --dest /backup --output ./manifests
orbit manifest verify --manifest-dir ./manifests

# Hybrid cloud migration
orbit /on-prem/data s3://migration-bucket/data -R --mode sync --resume --parallel 12

# Network share sync
orbit /local/files smb://nas/backup -R --mode sync --resume --retry-attempts 10
```

---

## Configuration

### Configuration File (`~/.orbit/orbit.toml`)

```toml
recursive = true
preserve_metadata = true
resume_enabled = true
verify_checksum = true
compression = { zstd = { level = 5 } }
show_progress = true
retry_attempts = 3
exponential_backoff = true
```

Run `orbit init` to generate an optimal config for your hardware.

### Configuration Priority

1. **CLI arguments** (highest)
2. **`--profile` preset** or auto-network detection
3. **`~/.orbit/orbit.toml`** (user config)
4. **Built-in defaults** (lowest)

### Feature Flags (Cargo)

```bash
cargo build --release                          # Minimal (~10MB) - local only
cargo build --release --features network       # All network backends via object_store (no aws-sdk-s3)
cargo build --release --features s3-cli        # Adds `orbit s3 ...` subcommands (presign, versioning, wildcards)
cargo build --release --features s3-native,ssh-backend  # Selective
```

> Default build includes only `zero-copy`. The root binary's optional Tokio features (multi-thread runtime, `net`, `time`, etc.) are gated behind network backends and only enabled when those features are turned on. Tokio is still pulled in transitively via the `orbit-core-interface` and `orbit-observability` workspace crates regardless of features.

---

## CLI Quick Reference

```bash
# Transfer
orbit <SOURCE> <DEST> [FLAGS]
orbit sync|backup|mirror|cp <SOURCE> <DEST> [FLAGS]

# Profiles
--profile fast|safe|backup|network

# S3 operations (build with --features s3-cli)
orbit ls|head|du|rm|mv|mb|rb|cat|pipe|presign s3://...

# Diagnostics
orbit init|doctor|explain|history|stats|capabilities

# Manifests
orbit manifest plan|verify|diff|info

# Batch execution
orbit run --file commands.txt --workers 256
```

**Key flags:**

| Flag | Description |
|------|-------------|
| `-R` / `--recursive` | Copy directories recursively |
| `--resume` | Resume interrupted transfers |
| `--zstd` / `--lz4` | Compression shorthand |
| `--compress auto` | Auto-select compression |
| `--workers N` | Parallel file workers (0=auto) |
| `--checksum blake3` | Checksum verification |
| `--dry-run` | Preview without copying |
| `--json` | Machine-readable output |
| `--quiet` | Suppress non-essential output |
| `--verbose` | Detailed logging |

---

## Modular Architecture

Orbit is organized as a Rust workspace with 8 member crates. Internal dependencies are shallow: the root `orbit` binary depends on all sub-crates, and `core-semantic` depends on `orbit-core-interface`. Everything else is independent.

| Crate | Path | What's inside | Status |
|-------|------|---------------|--------|
| `orbit` (root) | `src/` | CLI, transfer engine, the `Backend` trait + protocol backend implementations (S3/SSH/SMB/Azure/GCS) in `src/backend/`, Disk Guardian, Config Optimizer | Beta |
| `orbit-core-interface` | `crates/orbit-core-interface/` | The `OrbitSystem` trait (async `exists`/`metadata`/`read_header`/streaming I/O) — the system-level I/O abstraction used by `LocalSystem` and `MockSystem`; distinct from the protocol `Backend` trait in the root crate | Stable |
| `orbit-core-manifest` | `crates/core-manifest/` | `FlightPlan` (job-level) and `CargoManifest` (per-file) structs, JSON Schema validators, UUID job IDs | Beta |
| `orbit-core-starmap` | `crates/core-starmap/` | Memory-mapped binary index for chunks/windows; bloom filter + resume bitmaps via `redb` + `memmap2` | Beta |
| `orbit-core-cdc` | `crates/core-cdc/` | Gear Hash rolling hash + `ChunkConfig` (min/avg/max sizing); no internal deps | Alpha |
| `orbit-core-semantic` | `crates/core-semantic/` | `SemanticRegistry` + `Priority` for intent-based replication (configs first, blobs last); depends on `orbit-core-interface` | Alpha |
| `orbit-core-audit` | `crates/core-audit/` | `TelemetryLogger` — append-only JSONL events (`job_start`, `window_ok`, `job_complete`, etc.) | Beta |
| `orbit-observability` | `crates/orbit-observability/` | HMAC-SHA256 audit chaining, OpenTelemetry bridge, Prometheus metrics (heavier deps — keep separate from `core-audit`) | Beta |

### Where do I make this change?

| If you want to... | Edit |
|---|---|
| Add a new CLI subcommand or flag | `src/` (root crate) |
| Add a new protocol backend (e.g., FTP, WebDAV) | `src/backend/` — implement the `Backend` trait from `src/backend/mod.rs` and register it in `src/backend/registry.rs` |
| Change the manifest schema or add a field | `crates/core-manifest/` |
| Tune chunking thresholds or rolling-hash mask | `crates/core-cdc/` |
| Add a new file-type priority rule | `crates/core-semantic/` |
| Add a new audit event type | `crates/core-audit/` (basic) or `crates/orbit-observability/` (signed/OTel) |
| Change the on-disk resume index format | `crates/core-starmap/` |

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full architecture write-up.

---

## Security

- **Checksum verification**: SHA-256, BLAKE3 for data integrity
- **Credential protection**: Memory scrubbing on drop, no credential logging
- **No telemetry phone-home**: All data stays local
- **Pre-flight validation**: Disk Guardian prevents dangerous operations
- **Dependency auditing**: `deny.toml` + CI-integrated `cargo deny check`
- **S3 encryption**: Server-side encryption (AES-256, AWS KMS)
- **Minimal default build**: Zero runtime vulnerabilities in default configuration

See [SECURITY.md](SECURITY.md) for vulnerability reporting, dependency audit results, and security practices.

---

## Roadmap

### Phase 1: Stabilize & Simplify (Current)

- Stabilize core local + S3/SSH transfers
- Expand integration test coverage (targeting 80%+ on core paths)
- Feature-gate experimental V2/V3 behind `--smart` / alpha flags
- Continue workspace refactoring and crate extraction

### Phase 2: Polish & Differentiate

- Performance tuning with realistic benchmarks (vs rsync/rclone)
- Stabilize CDC resume + basic deduplication
- Enhanced UX (init wizard evolution, progress, telemetry)
- Grow documentation and examples

### Phase 3: Expand & Community

- Full backend parity across all protocols
- Plugin framework exploration
- Encryption at rest
- Publish reusable crates to crates.io
- v1.0 when core + advanced features are production-grade

### Recently Completed

- UX overhaul: shorthand subcommands, `orbit doctor`, output control flags
- Config resolution hardened: unified transfer path, auto-network detection
- Saner defaults: `preserve_metadata`, `show_stats`, `human_readable` default to `true`
- Simplified CLI: positional arguments, `--profile` presets, actionable errors
- Codebase restructuring: main.rs reduced by 45%, subcommands extracted; `Cli` struct split into 9 grouped `Args` sub-structs (`TransferArgs`, `ReliabilityArgs`, `PerformanceArgs`, etc.) for cleaner derive, faster parse, and easier maintenance
- Dependency cleanup: `anyhow` removed from all first-party crates (unified on `thiserror`); root binary's heavy Tokio runtime features are feature-gated to network backends, though a minimal `tokio` is still pulled in transitively via `orbit-core-interface` and `orbit-observability`
- Workspace dependency inheritance: shared dep versions managed centrally

---

## Contributing

Pull requests welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for code style, the OrbitSystem pattern, and guidelines.

```bash
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build && cargo test
cargo fmt && cargo clippy
```

### Areas We Need Help

- Testing on various platforms and real-world scenarios
- Integration tests for S3 (MinIO), SSH, SMB backends
- Documentation improvements
- Bug reports and edge-case discovery
- Stabilizing beta features toward production readiness

---

## Documentation

| Guide | Description |
|-------|-------------|
| **[Getting Started](docs/GETTING_STARTED.md)** | Installation, setup, and first transfers |
| **[S3 User Guide](docs/guides/S3_USER_GUIDE.md)** | Complete S3 operations guide |
| **[GCS User Guide](docs/guides/GCS_USER_GUIDE.md)** | Google Cloud Storage guide |
| **[Backend Guide](docs/guides/BACKEND_GUIDE.md)** | All storage backends |
| **[Filter System](docs/guides/FILTER_SYSTEM.md)** | Glob, regex, and path filters |
| **[Init Wizard Guide](docs/guides/INIT_WIZARD_GUIDE.md)** | Deep dive into `orbit init` |
| **[Delta Detection](docs/guides/DELTA_DETECTION_GUIDE.md)** | Efficient transfer algorithms |
| **[Architecture](ARCHITECTURE.md)** | System design and crate structure |
| **[Protocol Guide](docs/guides/PROTOCOL_GUIDE.md)** | Protocol-specific details |
| **[API Reference](https://docs.rs/orbit)** | `cargo doc --open` |

---

## License

**Apache License 2.0** -- See [LICENSE](LICENSE) for details.

```
Copyright 2024 Shane Wall
Licensed under the Apache License, Version 2.0
```

---

## Acknowledgments

- Built with Rust
- Inspired by rsync, rclone, and modern transfer tools
- Thanks to the Rust community for excellent crates

---

<div align="center">

**[Getting Started](docs/GETTING_STARTED.md)** | **[Architecture](ARCHITECTURE.md)** | **[Contributing](CONTRIBUTING.md)** | **[Security](SECURITY.md)**

Made by [Shane Wall](https://github.com/saworbit)

</div>
