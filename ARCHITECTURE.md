# Orbit Architecture

> A distributed file transfer and data management system designed for performance, reliability, and scalability.

**Version:** 0.6.0 (Core) / 2.2.0-rc.1 (Control Plane)
**Status:** Production-ready core, Grid architecture in active development

---

## Executive Summary

Orbit is a Rust-based file transfer system that scales from a simple CLI tool to a distributed enterprise platform. It combines:

- **High-performance transfers** via zero-copy syscalls, compression, and parallel I/O
- **Content-aware synchronization** using content-defined chunking (CDC) with 99.1% shift resilience
- **Global deduplication** across all files and backups via the Universe index
- **Distributed architecture** with stateless agents (Stars) coordinated by a central Nucleus
- **On-demand data access** through a FUSE filesystem that fetches blocks just-in-time

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              APPLICATION LAYER                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│  CLI (orbit)          │  Control Plane API      │  GhostFS (FUSE)               │
│  - copy/sync          │  - REST endpoints       │  - On-demand block fetch      │
│  - backup/restore     │  - Job management       │  - Priority queue             │
│  - verify             │  - Dashboard (React)    │  - Instant projection         │
└─────────────────────────────────────────────────────────────────────────────────┘
                                      │
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              ORCHESTRATION LAYER                                 │
├─────────────────────────────────────────────────────────────────────────────────┤
│  Magnetar State Machine           │  Sentinel Resilience Engine                 │
│  - Job lifecycle (SQLite/redb)    │  - OODA loop for chunk healing              │
│  - Crash recovery                 │  - Under-replication detection              │
│  - DAG dependencies               │  - Autonomous repair                        │
└─────────────────────────────────────────────────────────────────────────────────┘
                                      │
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              INTELLIGENCE LAYER                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│  Semantic Registry                │  Universe V3 Index                          │
│  - File type classification       │  - Global deduplication (redb)              │
│  - Priority assignment            │  - O(log N) inserts                         │
│  - Sync strategy selection        │  - O(1) memory via streaming                │
│                                   │                                             │
│  CDC Engine (Gear Hash)           │  Guidance System                            │
│  - Variable-size chunks           │  - Config validation                        │
│  - 99.1% shift resilience         │  - Auto-tuning                              │
│  - BLAKE3 content hashing         │  - Safety constraints                       │
└─────────────────────────────────────────────────────────────────────────────────┘
                                      │
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              TRANSPORT LAYER                                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│  OrbitSystem Abstraction          │  Grid Protocol (gRPC)                       │
│  - LocalSystem (std::fs)          │  - Nucleus ↔ Star communication             │
│  - RemoteSystem (future)          │  - Star-to-Star P2P                         │
│                                   │  - mTLS encryption                          │
│  Backend Registry                 │                                             │
│  - Local filesystem               │  Resilience Primitives                      │
│  - S3 / Azure / GCS               │  - Circuit breaker                          │
│  - SSH/SFTP                       │  - Connection pooling                       │
│  - SMB2/3 (native)                │  - Rate limiting                            │
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

Orbit is organized as a Rust workspace with 16 member crates:

### Core Transfer Engine

| Crate | Purpose |
|-------|---------|
| **orbit** | Main CLI binary and library - file copy, sync, verify operations |
| **orbit-core-manifest** | Flight plan and cargo manifest data structures |
| **orbit-core-audit** | Audit logging, structured JSON telemetry |
| **orbit-core-starmap** | Binary indexing engine (Universe V1/V2/V3) |
| **orbit-core-resilience** | Fault tolerance: circuit breaker, rate limiter, connection pool |

### V2 Content-Aware System

| Crate | Purpose |
|-------|---------|
| **orbit-core-cdc** | Content-Defined Chunking with Gear Hash rolling hash |
| **orbit-core-semantic** | Intent-based replication: file priority and sync strategy analysis |
| **orbit-core-interface** | OrbitSystem trait - universal I/O abstraction for local/remote |

### Grid Architecture (Distributed)

| Crate | Purpose |
|-------|---------|
| **orbit-proto** | gRPC protocol definitions (tonic/prost) |
| **orbit-star** | Stateless remote agent for distributed operations |
| **orbit-connect** | Client-side gRPC connectivity (Nucleus → Star) |
| **orbit-sentinel** | Autonomous resilience engine (OODA loop for chunk healing) |

### Control Plane & Observability

| Crate | Purpose |
|-------|---------|
| **orbit-server** | REST API, SQLite job tracking, OpenAPI/Swagger UI, React dashboard |
| **orbit-observability** | Unified telemetry, audit chaining, OpenTelemetry integration |

### Advanced Capabilities

| Crate | Purpose |
|-------|---------|
| **magnetar** | Persistent job state machine with SQLite/redb backends |
| **orbit-ghost** | FUSE-based on-demand filesystem with block-level JIT fetching |

### Dependency Graph

```
orbit (CLI/Library)
├── orbit-core-manifest
├── orbit-core-audit
├── orbit-core-starmap ──────────────────────┐
├── orbit-core-cdc ──────────────────────────┤
├── orbit-core-semantic ─────────────────────┤
│   └── orbit-core-interface                 │
├── orbit-core-resilience                    │
├── orbit-observability                      │
└── magnetar ────────────────────────────────┤
    ├── orbit-core-cdc                       │
    ├── orbit-core-starmap                   │
    └── orbit-core-interface                 │
                                             │
orbit-server (Control Plane)                 │
├── magnetar ────────────────────────────────┤
├── orbit-sentinel ──────────────────────────┤
│   ├── orbit-core-starmap ──────────────────┘
│   ├── orbit-connect
│   │   ├── orbit-core-interface
│   │   └── orbit-proto
│   └── orbit-star
│       └── orbit-proto
└── orbit-observability

orbit-ghost (On-Demand FS)
├── fuser (FUSE bindings)
├── sqlx (SQLite)
└── magnetar (metadata source)
```

---

## Data Flow

### Simple File Copy

```
orbit --source /data --dest /backup --recursive

User CLI
    │
    ▼
┌─────────────────────┐
│   Parse Arguments   │
│   Load Config       │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  Guidance System    │◄── Validates config, auto-tunes settings
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

### Distributed Grid Transfer

```
                    ┌─────────────────────────────┐
                    │     Nucleus (Coordinator)    │
                    │  ┌───────────────────────┐  │
                    │  │ jobs.db (SQLite)      │  │
                    │  │ universe_v3.db (redb) │  │
                    │  │ Star Registry         │  │
                    │  │ REST API + Dashboard  │  │
                    │  └───────────┬───────────┘  │
                    └──────────────┼──────────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
            mTLS/gRPC       mTLS/gRPC      mTLS/gRPC
                    │              │              │
                    ▼              ▼              ▼
          ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
          │  Star A     │ │  Star B     │ │  Star C     │
          │  (NAS-1)    │ │  (NAS-2)    │ │  (Cloud)    │
          │             │ │             │ │             │
          │ LocalSystem │ │ LocalSystem │ │ LocalSystem │
          │ CDC Engine  │ │ CDC Engine  │ │ CDC Engine  │
          └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
                 │               │               │
                 │◄──── P2P Direct Links ───────►│
                 │               │               │
                 ▼               ▼               ▼
          ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
          │ Local NVMe  │ │ Local SSD   │ │ Object Store│
          └─────────────┘ └─────────────┘ └─────────────┘

Key Benefits:
- Compute moves to data (CDC runs locally on Stars)
- P2P reduces Nucleus bandwidth
- Horizontal scaling via Star agents
- Stateless agents, centralized state
```

### On-Demand Filesystem (GhostFS)

```
Application: ffmpeg -i /mnt/ghost/video.mp4 output.mp4

Application
    │ read(offset=52428600, size=1MB)
    ▼
┌─────────────────────┐
│   Kernel VFS        │
└─────────┬───────────┘
          │ FUSE Protocol
          ▼
┌─────────────────────────────────────────────┐
│         OrbitGhostFS (FUSE Handler)         │
│  ┌─────────────┐    ┌───────────────────┐   │
│  │ Inode       │    │ MetadataOracle    │   │
│  │ Translator  │───►│ (MagnetarAdapter) │   │
│  │ u64 ↔ ID    │    │ SQLite queries    │   │
│  └─────────────┘    └───────────────────┘   │
└─────────────────────────┬───────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────┐
│              Entangler                      │
│  ┌─────────────────────────────────────┐    │
│  │ Calculate block: 52428600 / 1MB = 50│    │
│  │                                     │    │
│  │ Cache hit?  ───► YES ───► Return    │    │
│  │      │                              │    │
│  │      ▼ NO                           │    │
│  │ Queue BlockRequest(file_id, 50)     │    │
│  │ Poll for availability               │    │
│  └──────────────────┬──────────────────┘    │
└─────────────────────┼───────────────────────┘
                      │ crossbeam-channel
                      ▼
┌─────────────────────────────────────────────┐
│           Wormhole (Background)             │
│  ┌─────────────────────────────────────┐    │
│  │ Receive BlockRequest                │    │
│  │ Fetch from backend (Orbit protocol) │    │
│  │ Write to: {cache}/file_id_50.bin    │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
                      │
                      ▼
              Block Cache (Disk)
              /tmp/orbit_cache/

Result: Application reads 52MB into a 1TB file,
        only 52MB transferred over network.
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

### Grid Architecture

| Feature | Status | Description |
|---------|--------|-------------|
| **OrbitSystem Trait** | ✅ Stable | Phase 1: Unified I/O abstraction |
| **gRPC Protocol** | 🟡 Beta | Phase 2: Nucleus ↔ Star |
| **Star Agent** | 🟡 Beta | Phase 3: Stateless remote execution |
| **P2P Links** | 🔴 Alpha | Phase 4: Star-to-Star direct |
| **Sentinel Healing** | 🔴 Alpha | Phase 5: Autonomous repair |

### Control Plane

| Feature | Status | Description |
|---------|--------|-------------|
| **REST API** | 🔴 Alpha | Job management endpoints |
| **SQLite Persistence** | 🟡 Beta | Via Magnetar state machine |
| **React Dashboard** | 🔴 Alpha | Web-based job monitoring |
| **OpenAPI/Swagger** | 🔴 Alpha | API documentation |

### Advanced Capabilities

| Feature | Status | Description |
|---------|--------|-------------|
| **Magnetar State Machine** | 🟡 Beta | Crash recovery, DAG dependencies |
| **GhostFS (FUSE)** | 🟡 Beta | On-demand block-level access |
| **Init Wizard** | ✅ Stable | Interactive configuration setup |
| **Active Probing** | ✅ Stable | Auto-detection of hardware/destination |
| **Filter System** | ✅ Stable | Glob/regex include/exclude |
| **Metadata Preservation** | ✅ Stable | Permissions, timestamps, xattrs |

---

## Deployment Modes

### 1. Standalone CLI

Single-machine file operations with no external dependencies.

```bash
# Simple copy
orbit --source /data --dest /backup --recursive

# With compression and verification
orbit --source /data --dest /backup \
      --compression zstd \
      --checksum blake3 \
      --recursive

# Smart sync with deduplication
orbit sync --source /project --dest /backup --smart
```

### 2. Control Plane Server

Centralized job management with REST API and web dashboard.

```bash
# Start the Nucleus server
orbit-server --port 8080 --database jobs.db

# Submit jobs via REST API
curl -X POST http://localhost:8080/jobs \
     -H "Content-Type: application/json" \
     -d '{"source": "/data", "destination": "/backup"}'
```

### 3. Distributed Grid

Horizontal scaling with remote Star agents.

```bash
# On each storage node (Star)
orbit-star --listen 0.0.0.0:9000 --cert star.pem

# On the coordinator (Nucleus)
orbit-server --port 8080 \
             --stars star-a.local:9000,star-b.local:9000
```

### 4. On-Demand Filesystem

Mount remote data locally with just-in-time fetching.

```bash
# Mount a job's data
orbit-ghost --job-id 1 \
            --database magnetar.db \
            --mount-point /mnt/orbit

# Access files (blocks fetched on demand)
ls /mnt/orbit
cat /mnt/orbit/data/file.txt
```

---

## Configuration

### CLI Arguments

```bash
orbit --source <PATH>           # Source path (required)
      --dest <PATH>             # Destination path (required)
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
    "zero-copy",        # Platform optimizations
    "smb-native",       # SMB2/3 support
    "s3-native",        # AWS S3 support
    "api",              # Control Plane
    "opentelemetry",    # Distributed tracing
] }
```

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

- **mTLS**: Mutual TLS for all Grid communication
- **gRPC**: Protocol buffer serialization
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
| GhostFS | ~1MB per active block |

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
- ✅ Magnetar state machine
- ✅ GhostFS on-demand filesystem
- 🔄 Grid architecture (Stars, Nucleus)

### Near-term (v0.7.x)

- Enhanced init wizard with active probing
- Configuration file support (TOML)
- Improved error messages and recovery
- Windows native support for GhostFS (WinFSP)

### Future (v1.0+)

- Production-hardened Grid deployment
- Kubernetes operator for Star agents
- ML-powered prefetching in GhostFS
- Encryption at rest
- Multi-tenant isolation

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

- [Guidance System](docs/architecture/GUIDANCE_SYSTEM.md) - Configuration validation
- [Disk Guardian](docs/architecture/DISK_GUARDIAN.md) - Pre-flight safety
- [V2 Architecture](docs/architecture/ORBIT_V2_ARCHITECTURE.md) - CDC + Semantic
- [Grid Specification](docs/specs/ORBIT_GRID_SPEC.md) - Distributed architecture
- [GhostFS](orbit-ghost/ARCHITECTURE.md) - On-demand filesystem

---

## License

Apache 2.0 - See [LICENSE](LICENSE) for details.

---

*Built with Rust for performance, reliability, and safety.*
