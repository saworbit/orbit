# Orbit Architecture

> A high-performance file transfer and data management system designed for reliability and scalability.

**Version:** 0.6.0
**Status:** Production-ready core

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

### Transfer Capabilities

| Feature | Status | Description |
|---------|--------|-------------|
| **Buffered Copy** | ✅ Stable | Safe, cross-platform default |
| **Zero-Copy** | ✅ Stable | Platform syscalls: copy_file_range, fcopyfile, CopyFileEx |
| **Streaming** | ✅ Stable | Low memory for large files |
| **Parallel Files** | ✅ Stable | Concurrent file transfers |
| **Resume** | ✅ Stable | Checkpoint-based recovery |
| **Bandwidth Limit** | ✅ Stable | Token bucket rate limiting |

### Compression & Verification

| Feature | Status | Description |
|---------|--------|-------------|
| **LZ4** | ✅ Stable | Fast compression, lower ratio |
| **Zstd** | ✅ Stable | Balanced speed/ratio, tunable level |
| **SHA-256** | ✅ Stable | Standard cryptographic checksum |
| **BLAKE3** | ✅ Stable | Modern, parallelizable, streaming |

### Storage Backends

| Backend | Status | Notes |
|---------|--------|-------|
| **Local Filesystem** | ✅ Stable | Primary use case |
| **SSH/SFTP** | 🟡 Beta | Functional via ssh2 crate |
| **S3** | 🟡 Beta | Multipart upload support |
| **Azure Blob** | 🟡 Beta | Via object_store crate |
| **GCS** | 🟡 Beta | Via object_store crate |
| **SMB2/3** | 🟡 Beta | Native pure-Rust implementation |

### V2 Content-Aware Features

| Feature | Status | Description |
|---------|--------|-------------|
| **Content-Defined Chunking** | 🟡 Beta | Gear Hash, 99.1% shift resilience |
| **Semantic Prioritization** | 🟡 Beta | Critical → High → Normal → Low |
| **Universe V3 Dedup** | 🟡 Beta | O(log N) inserts, O(1) memory |
| **Global Deduplication** | 🟡 Beta | Across all files and backups |

### Additional Capabilities

| Feature | Status | Description |
|---------|--------|-------------|
| **OrbitSystem Trait** | ✅ Stable | Unified I/O abstraction |
| **Init Wizard** | ✅ Stable | Interactive configuration setup |
| **Active Probing** | ✅ Stable | Auto-detection of hardware/destination |
| **Filter System** | ✅ Stable | Glob/regex include/exclude |
| **Metadata Preservation** | ✅ Stable | Permissions, timestamps, xattrs |

### Data Flow Patterns

| Feature | Status | Description |
|---------|--------|-------------|
| **Container Packing** | 🔴 Alpha | Chunk packing into .orbitpak files, pool rotation |
| **Typed Provenance** | 🔴 Alpha | Structured event taxonomy (20 event types) |
| **Composable Prioritizers** | 🔴 Alpha | Chainable sort criteria (semantic, size, age, retry) |

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

> **Note:** The default build includes only `zero-copy`. Tokio is not included by
> default — it is pulled in automatically by network backend features (`s3-native`,
> `ssh-backend`, `azure-native`, `gcs-native`, `smb-native`).

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

### Current (v0.6.x)

- ✅ Core transfer engine (buffered, zero-copy, streaming)
- ✅ All backends (local, S3, Azure, GCS, SMB, SSH)
- ✅ CDC + Semantic + Universe V3
- ✅ Container packing, composable prioritizers, typed provenance

### Near-term (v0.7.x)

- ✅ Enhanced init wizard with active probing
- ✅ Configuration file support (TOML)
- ✅ Improved error messages with actionable suggestions
- ✅ CLI simplification: positional args, `--profile` presets
- ✅ Hardware probe caching and auto-tune summary display
- ✅ Removed `anyhow` dependency (unified on `thiserror`/`OrbitError`)

### Future (v1.0+)

- Encryption at rest

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
