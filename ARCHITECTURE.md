# Orbit Architecture

> A high-performance file transfer and data management system designed for reliability and scalability.

**Version:** 0.6.0
**Status:** Alpha -- stable core with experimental advanced features

---

## Executive Summary

Orbit is a Rust-based file transfer system that combines:

- **High-performance transfers** via zero-copy syscalls, compression, and parallel I/O
- **Content-aware synchronization** using content-defined chunking (CDC) with 99.1% shift resilience
- **Global deduplication** across all files and backups via the Universe index

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              APPLICATION LAYER                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│  CLI (orbit)                                                                    │
│  - copy/sync                                                                    │
│  - backup/restore                                                               │
│  - verify                                                                       │
└─────────────────────────────────────────────────────────────────────────────────┘
                                      │
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              INTELLIGENCE LAYER                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│  Semantic Registry                │  Universe V3 Index                          │
│  - File type classification       │  - Global deduplication (redb)              │
│  - Priority assignment            │  - O(log N) inserts                         │
│  - Composable prioritizers        │  - O(1) memory via streaming                │
│  - Sync strategy selection        │  - Container packing (.orbitpak)            │
│                                   │                                             │
│  CDC Engine (Gear Hash)           │  Config Optimizer                           │
│  - Variable-size chunks           │  - Config validation                        │
│  - 99.1% shift resilience         │  - Auto-tuning                              │
│  - BLAKE3 content hashing         │  - Safety constraints                       │
└─────────────────────────────────────────────────────────────────────────────────┘
                                      │
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              TRANSPORT LAYER                                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│  OrbitSystem Abstraction          │  Backend Registry                           │
│  - LocalSystem (std::fs)          │  - Local filesystem                         │
│  - RemoteSystem (future)          │  - S3 / Azure / GCS                        │
│                                   │  - SSH/SFTP                                 │
│                                   │  - SMB2/3 (native)                          │
└─────────────────────────────────────────────────────────────────────────────────┘
                                      │
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              I/O OPTIMIZATION LAYER                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│  Zero-Copy Engine                 │  Compression                │  Checksums    │
│  - Linux: copy_file_range         │  - LZ4 (fast)               │  - SHA-256    │
│  - macOS: fcopyfile               │  - Zstd (balanced)          │  - BLAKE3     │
│  - Windows: CopyFileEx            │  - None (pre-compressed)    │               │
│                                   │                             │               │
│  Disk Guardian                    │  Progress & Telemetry                       │
│  - Pre-flight space check         │  - Real-time progress bars                  │
│  - Permission verification        │  - JSON audit trail                         │
│  - Path validation                │  - OpenTelemetry integration                │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Crate Architecture

Orbit is organized as a Rust workspace with 8 member crates:

### Core Transfer Engine

| Crate | Purpose |
|-------|---------|
| **orbit** | Main CLI binary and library - file copy, sync, verify operations |
| **orbit-core-manifest** | Cargo manifest data structures |
| **orbit-core-audit** | Audit logging, structured JSON telemetry, typed provenance events |
| **orbit-core-starmap** | Binary indexing engine (Universe V1/V2/V3), container packing |

### Content-Aware System

| Crate | Purpose |
|-------|---------|
| **orbit-core-cdc** | Content-Defined Chunking with Gear Hash rolling hash |
| **orbit-core-semantic** | Intent-based replication: file priority, sync strategy, composable prioritizers |
| **orbit-core-interface** | OrbitSystem trait - universal I/O abstraction for local/remote |

### Observability

| Crate | Purpose |
|-------|---------|
| **orbit-observability** | Unified telemetry, audit chaining, OpenTelemetry integration |

### Dependency Graph

```
orbit (CLI/Library)
├── orbit-core-manifest
├── orbit-core-audit
├── orbit-core-starmap
├── orbit-core-cdc
├── orbit-core-semantic
│   └── orbit-core-interface
└── orbit-observability
```

---

## Data Flow

### Simple File Copy

```
orbit /data /backup --recursive --profile backup

User CLI
    │
    ▼
┌─────────────────────┐
│   Parse Arguments   │
│   Apply Profile     │
│   Load Config       │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  Config Optimizer   │◄── Validates config, auto-tunes settings
│  - compression?     │
│  - checksum?        │
│  - bandwidth limit? │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│   Disk Guardian     │◄── Pre-flight checks: space, permissions, paths
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  Directory Walker   │◄── Enumerate source files
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│   Transfer Loop     │
│   ┌─────────────┐   │
│   │ Read Source │   │
│   └──────┬──────┘   │
│          ▼          │
│   ┌─────────────┐   │
│   │ Compress?   │   │
│   └──────┬──────┘   │
│          ▼          │
│   ┌─────────────┐   │
│   │ Zero-Copy   │◄──┼── Platform-specific optimization
│   │ or Buffered │   │
│   └──────┬──────┘   │
│          ▼          │
│   ┌─────────────┐   │
│   │ Checksum?   │   │
│   └──────┬──────┘   │
│          ▼          │
│   ┌─────────────┐   │
│   │Write Dest   │   │
│   └─────────────┘   │
└─────────────────────┘
          │
          ▼
┌─────────────────────┐
│   Emit Telemetry    │◄── JSON audit trail + progress
└─────────────────────┘
```

### Smart Sync with Deduplication (V2)

```
orbit sync --source /project --dest /backup --smart

Source Files                              Destination
    │                                         │
    ▼                                         │
┌─────────────────────┐                       │
│  Semantic Registry  │                       │
│  ┌───────────────┐  │                       │
│  │ Analyze Type  │  │                       │
│  │ .toml → Critical │                       │
│  │ .wal  → High    │                        │
│  │ .rs   → Normal  │                        │
│  │ .mp4  → Low     │                        │
│  └───────┬───────┘  │                       │
└──────────┼──────────┘                       │
           │                                  │
           ▼                                  │
┌─────────────────────┐                       │
│  Priority Queue     │◄── Critical files transferred first
│  (BinaryHeap)       │                       │
└──────────┬──────────┘                       │
           │                                  │
           ▼                                  │
┌─────────────────────┐                       │
│   CDC Engine        │                       │
│   ┌───────────────┐ │                       │
│   │ Gear Hash     │ │◄── Rolling hash for boundary detection
│   │ 8KB-256KB     │ │                       │
│   │ Variable      │ │                       │
│   └───────┬───────┘ │                       │
│           │         │                       │
│   ┌───────▼───────┐ │                       │
│   │ BLAKE3 Hash   │ │◄── Content-addressable chunk IDs
│   └───────────────┘ │                       │
└──────────┬──────────┘                       │
           │                                  │
           ▼                                  │
┌─────────────────────┐                       │
│   Universe V3       │                       │
│   ┌───────────────┐ │                       │
│   │ Lookup Hash   │ │                       │
│   │ Already       │ │                       │
│   │ Exists?       │ │                       │
│   └───────┬───────┘ │                       │
│       YES │ NO      │                       │
│       ▼   │         │                       │
│    [SKIP] │         │                       │
│           ▼         │                       │
│   ┌───────────────┐ │                       │
│   │Transfer Chunk │─┼───────────────────────┼─► Write
│   │Insert Index   │ │                       │
│   └───────────────┘ │                       │
└─────────────────────┘                       │
                                              │
           Global Deduplication Achieved ◄────┘
```

---

## Feature Matrix

### Stable Core (Production-Ready)

These features form the reliable foundation of Orbit. They are well-tested and safe for production use.

| Feature | Description |
|---------|-------------|
| **Buffered Copy** | Safe, cross-platform default transfer mode |
| **Zero-Copy** | Platform syscalls: copy_file_range, fcopyfile, CopyFileEx |
| **Streaming** | Low-memory mode for large files |
| **Parallel Files** | Concurrent file transfers via rayon |
| **Resume** | Checkpoint-based recovery with chunk verification |
| **Bandwidth Limit** | Token bucket rate limiting |
| **LZ4 / Zstd** | Fast and balanced compression options |
| **SHA-256 / BLAKE3** | Cryptographic checksum verification |
| **OrbitSystem Trait** | Unified I/O abstraction for local/remote |
| **Local Filesystem** | Primary use case, thoroughly tested |

### Beta (Functional, Needs Real-World Validation)

These features work well but need broader real-world testing before being considered stable.

| Feature | Notes |
|---------|-------|
| **SSH/SFTP Backend** | Functional via ssh2 crate |
| **S3 Backend** | Multipart upload, presign, wildcard listing |
| **Azure Blob Backend** | Via object_store crate (v0.6.0) |
| **GCS Backend** | Via object_store crate (v0.6.0) |
| **SMB2/3 Backend** | Native pure-Rust (v0.11.0 upgrade) |
| **Delta Detection (V1)** | rsync-style block-based diffing |
| **Disk Guardian** | Pre-flight space/permission/path checks |
| **Config Optimizer** | Config validation with active hardware probing |
| **Init Wizard** | Interactive setup (`orbit init`) |
| **Filter System** | Glob/regex include/exclude patterns |
| **Metadata Preservation** | Permissions, timestamps, xattrs (platform-specific) |
| **Manifest System** | File tracking and verification |
| **Audit Logging** | JSONL telemetry with HMAC chaining |
| **Global Dedup (V3)** | Universe index, O(log N) inserts, O(1) memory |

### Alpha / Experimental (Expect Changes)

These features are under active development. They are feature-gated and should not be relied upon for production workloads. Expect API changes.

| Feature | Description |
|---------|-------------|
| **V2 CDC Engine** | Content-defined chunking with Gear Hash |
| **Semantic Replication** | Priority-based file classification and ordering |
| **Container Packing** | Chunk packing into .orbitpak files |
| **Typed Provenance** | Structured event taxonomy (20 event types) |
| **Composable Prioritizers** | Chainable sort criteria (semantic, size, age, retry) |
| **Backpressure / Dead-Letter** | Flow control and quarantine for failed items |
| **Link-Dest++** | Chunk-level incremental backup hardlinking |
| **Transfer Journal** | Content-addressed batch operation journal |

> **Guidance:** Use stable core features for production. Beta features are safe for non-critical use with testing. Alpha features are preview-only -- enable with `--smart` or explicit flags.

---

## Deployment Modes

### 1. Standalone CLI

Single-machine file operations with no external dependencies.

```bash
# Simple copy (positional arguments)
orbit /data /backup --recursive

# With a preset profile
orbit /data /backup --recursive --profile backup

# With compression and verification
orbit --source /data --dest /backup \
      --compression zstd \
      --checksum blake3 \
      --recursive

# Smart sync with deduplication
orbit sync --source /project --dest /backup --smart
```

---

## Configuration

### CLI Arguments

```bash
orbit <SOURCE> <DEST> [FLAGS]   # Positional arguments
orbit --source <PATH>           # Source path (named flag)
      --dest <PATH>             # Destination path (named flag)
      --profile <PRESET>        # Apply preset: fast|safe|backup|network
      --recursive               # Copy directories recursively
      --compression <TYPE>      # none|lz4|zstd
      --checksum <TYPE>         # none|sha256|blake3
      --bandwidth-limit <BPS>   # Rate limit in bytes/second
      --resume                  # Resume interrupted transfer
      --parallel <N>            # Concurrent file transfers
      --filter <PATTERN>        # Include/exclude patterns
      --verbose                 # Detailed output
      --json                    # JSON output format
```

### Environment Variables

```bash
ORBIT_CONFIG=/path/to/orbit.toml    # Config file location
ORBIT_CACHE=/var/cache/orbit        # Cache directory
ORBIT_LOG=debug                     # Log level
ORBIT_TELEMETRY=json                # Telemetry format
```

### Feature Flags (Cargo)

```toml
[dependencies]
orbit = { version = "0.6", features = ["full"] }  # Everything

# Or selective features:
orbit = { version = "0.6", features = [
    "zero-copy",        # Platform optimizations (default)
    "smb-native",       # SMB2/3 support (includes Tokio)
    "s3-native",        # AWS S3 support (includes Tokio)
    "opentelemetry",    # Distributed tracing
] }
```

> **Note:** The default build includes only `zero-copy`. The root binary's heavy
> Tokio runtime features (multi-thread, networking, timers) are gated behind
> network backend features (`s3-native`, `ssh-backend`, `azure-native`,
> `gcs-native`, `smb-native`). A minimal `tokio` is still pulled in transitively
> via the `orbit-core-interface` and `orbit-observability` workspace crates.

---

## Observability

### Telemetry

Orbit emits structured JSON events for all operations:

```json
{
  "timestamp": "2025-01-19T10:30:00Z",
  "event": "file_transferred",
  "source": "/data/file.txt",
  "destination": "/backup/file.txt",
  "bytes": 1048576,
  "duration_ms": 150,
  "checksum": "blake3:abc123...",
  "compression": "zstd"
}
```

### OpenTelemetry Integration

```bash
# Enable distributed tracing
OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317 \
orbit --source /data --dest /backup
```

### Audit Trail

All operations are recorded with cryptographic chaining:

```
Event N-1 (HMAC: abc123)
    │
    ▼
Event N (prev_hmac: abc123, HMAC: def456)
    │
    ▼
Event N+1 (prev_hmac: def456, HMAC: ...)
```

---

## Security Considerations

### Data Integrity

- **Checksums**: SHA-256 or BLAKE3 verification of all transferred data
- **CDC Hashing**: BLAKE3 content-addressable chunks
- **Audit Chaining**: HMAC-SHA256 linked event log

### Network Security

- **Rate Limiting**: Token bucket algorithm prevents abuse

### Access Control

- **Path Validation**: Disk Guardian prevents path traversal
- **Permission Preservation**: Transfers maintain source permissions
- **Sandboxed Cache**: 0700 permissions on cache directories

---

## Performance Characteristics

### Transfer Speed

| Scenario | Typical Performance |
|----------|---------------------|
| Local SSD → SSD | Near hardware limit (zero-copy) |
| Local HDD → HDD | ~100-200 MB/s |
| Network (1Gbps) | ~100 MB/s |
| Network (10Gbps) | ~1 GB/s (with tuning) |

### Memory Usage

| Operation | Memory Footprint |
|-----------|------------------|
| Buffered copy | ~64KB per file |
| CDC chunking | ~1MB window |
| Universe V3 lookup | O(1) via streaming |

### Deduplication Efficiency

| Workload | Typical Savings |
|----------|-----------------|
| Code repositories | 60-80% |
| VM images | 70-90% |
| Backup sets | 80-95% |
| Media files | 10-30% |

---

## Roadmap

### Phase 1: Stabilize & Simplify (Current)

Focus: Make the core rock-solid and the project approachable.

- Stabilize core local + S3/SSH transfers with 80%+ test coverage on core paths
- Feature-gate experimental V2/V3 features behind `--smart` / alpha flags
- Continue workspace refactoring: push logic from `src/` into dedicated crates
- Expand integration tests (MinIO for S3, container-based SSH/SMB)
- ✅ Workspace dependency inheritance in root Cargo.toml

### Phase 2: Polish & Differentiate

Focus: Performance proof-points and user experience.

- Performance tuning with realistic benchmarks vs rsync/rclone
- Stabilize one advanced feature (CDC resume + basic dedup)
- Explore `io_uring` on Linux for async I/O
- Enhanced UX: init wizard evolution, progress, telemetry
- Profile-guided optimization (PGO) investigation
- Grow documentation and examples

### Phase 3: Expand & Community

Focus: Ecosystem and production readiness.

- Full backend parity (streaming Backend trait for all protocols)
- Publish reusable crates (`orbit-core-cdc`, `orbit-core-interface`) to crates.io
- Plugin framework exploration
- Encryption at rest
- `--watch` mode / daemon capabilities
- WebDAV protocol support
- v1.0 when core + 1-2 advanced features are production-grade

### Completed Milestones

- Core transfer engine (buffered, zero-copy, streaming)
- All backends (local, S3, Azure, GCS, SMB, SSH)
- CDC + Semantic + Universe V3 (alpha)
- Enhanced init wizard with active probing
- Configuration file support (TOML)
- CLI simplification: positional args, `--profile` presets
- Hardware probe caching and auto-tune summary
- Removed `anyhow` from all first-party crates (unified on `thiserror`/`OrbitError`); transitive presence via third-party deps (`jsonschema`, `opentelemetry-otlp`) remains
- UX overhaul: shorthands, `orbit doctor`, output control
- Workspace dependency inheritance (single source of truth for shared deps)

### Risks to Watch

- **Scope creep**: New alpha features before core is solid
- **Dependency bloat**: Large transitive deps from Tokio/aws-sdk
- **Platform parity gaps**: Windows/macOS metadata and zero-copy edge cases
- **Dedup at scale**: V2/V3 CDC not yet battle-tested with large real-world datasets

---

## Getting Started

### Installation

```bash
# From source
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build --release

# With all features
cargo build --release --features full
```

### Quick Start

```bash
# Basic copy
./target/release/orbit \
    --source /path/to/source \
    --dest /path/to/destination \
    --recursive

# With compression and verification
./target/release/orbit \
    --source /data \
    --dest /backup \
    --compression zstd \
    --checksum blake3 \
    --recursive \
    --verbose
```

### Documentation

- [Config Optimizer](docs/architecture/GUIDANCE_SYSTEM.md) - Configuration validation
- [Disk Guardian](docs/architecture/DISK_GUARDIAN.md) - Pre-flight safety
- [V2 Architecture](docs/architecture/ORBIT_V2_ARCHITECTURE.md) - CDC + Semantic

---

## License

Apache 2.0 - See [LICENSE](LICENSE) for details.

---

*Built with Rust for performance, reliability, and safety.*
