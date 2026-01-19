# Orbit GhostFS - Quantum Entanglement Filesystem

[![CI](https://github.com/saworbit/orbit/actions/workflows/ci.yml/badge.svg)](https://github.com/saworbit/orbit/actions)
[![codecov](https://codecov.io/gh/saworbit/orbit/branch/main/graph/badge.svg)](https://codecov.io/gh/saworbit/orbit)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos-lightgrey.svg)](INSTALL.md)

A FUSE-based on-demand filesystem that projects remote datasets onto local mount points with just-in-time block-level fetching.

## Overview

Orbit GhostFS implements a "Process-while-Moving" paradigm, moving beyond traditional "Store-then-Process" models. Data appears instantly in the filesystem, with blocks fetched on-demand when accessed by applications.

### Key Features

- **Instant Projection**: Files appear in the mount point immediately based on manifest metadata
- **Block-Level JIT**: Data fetched only when accessed, at the exact block needed
- **Priority Queue**: User requests jump ahead of sequential background downloads
- **Quantum Entanglement**: Read operations trigger immediate fetch of required blocks

### Architecture Components

| Module | Component | Purpose |
|--------|-----------|---------|
| `oracle.rs` | MetadataOracle | Trait abstraction for metadata backends |
| `adapter.rs` | MagnetarAdapter | SQLite implementation of MetadataOracle |
| `translator.rs` | InodeTranslator | Bidirectional u64 ↔ artifact_id mapping |
| `fs.rs` | OrbitGhostFS | FUSE interface intercepting POSIX syscalls |
| `entangler.rs` | Entangler | Block coordination and priority queue |
| `main.rs` | Wormhole thread | Background block fetching (simulated) |

Supporting modules: `inode.rs` (GhostEntry), `error.rs` (GhostError), `config.rs` (GhostConfig)

## Platform Support

### Linux (Recommended)
Native support via libfuse3. Best performance.

**Requirements:**
```bash
# Ubuntu/Debian
sudo apt-get install fuse3 libfuse3-dev pkg-config

# Fedora/RHEL
sudo dnf install fuse3 fuse3-devel pkgconfig

# Arch
sudo pacman -S fuse3 pkgconf
```

**Build:**
```bash
cd orbit-ghost
cargo build --release
```

### macOS
Requires macFUSE installation.

**Requirements:**
```bash
# Install macFUSE from https://osxfuse.github.io/
# Or via Homebrew:
brew install --cask macfuse
```

**Note:** macOS security policies may require additional permissions or kernel extension approval.

### Windows
FUSE is not natively supported. Windows implementation requires WinFSP (Windows File System Proxy).

**Current Status:** Not yet implemented.

**Future Implementation:**
- Use WinFSP bindings
- Alternative: NFS-based emulation layer
- Consider native Windows filter driver

## Usage

### Quick Start

**Prerequisites:**
- Magnetar database with migrations applied
- Job ID to mount

**Step 1: Seed test database**
```bash
# Create test database
sqlite3 test.db < ../crates/magnetar/migrations/*.sql

# Insert test job and artifacts
sqlite3 test.db << EOF
INSERT INTO jobs (source, destination) VALUES ('/test/source', '/test/dest');
INSERT INTO artifacts (id, job_id, parent_id, name, size, is_dir) VALUES
  ('root', 1, NULL, '', 0, 1),
  ('file1', 1, 'root', 'demo.txt', 1024, 0),
  ('dir1', 1, 'root', 'documents', 0, 1),
  ('file2', 1, 'dir1', 'readme.md', 2048, 0);
EOF
```

**Step 2: Run GhostFS**
```bash
# Basic usage (required argument: --job-id)
RUST_LOG=info cargo run -- --job-id 1 --database test.db

# Custom mount point and cache directory
cargo run -- --job-id 1 \
             --database test.db \
             --mount-point /mnt/orbit \
             --cache-dir /var/cache/orbit

# See all options
cargo run -- --help
```

**Step 3: Access filesystem**
```bash
# In another terminal:
ls -la /tmp/orbit_ghost_mount
# Files from database appear instantly

# Read file (triggers on-demand block fetch)
cat /tmp/orbit_ghost_mount/demo.txt

# Navigate directories
cd /tmp/orbit_ghost_mount/documents
cat readme.md

# Unmount when done
fusermount -u /tmp/orbit_ghost_mount  # Linux
umount /tmp/orbit_ghost_mount         # macOS
```

### CLI Options

```
orbit-ghost --job-id <ID> [OPTIONS]

Options:
  -d, --database <PATH>      Path to Magnetar database [default: magnetar.db]
  -j, --job-id <ID>          Job ID to mount (required)
  -m, --mount-point <PATH>   Mount point directory [default: /tmp/orbit_ghost_mount]
  -c, --cache-dir <PATH>     Cache directory for blocks [default: /tmp/orbit_cache]
  -h, --help                 Print help
  -V, --version              Print version
```

## Performance Characteristics

### Block Size Tuning
- Default: 1MB blocks
- Production recommendation: 5-16MB (align with cloud object storage chunk sizes)

### Optimizations
- **Prefetching**: Entangler can heuristically queue N+1, N+2 for sequential access
- **Caching**: Downloaded blocks persist in `/tmp/orbit_cache/`
- **Concurrency**: Multiple threads can read different blocks simultaneously

### Current Limitations
- **Network Loss**: Read syscalls will hang indefinitely (no timeout in polling loop)
- **Polling Overhead**: 10ms sleep loop wastes CPU vs Condvar::wait()
- **Single Download Thread**: Sequential block fetching (no parallel downloads)
- **Recommendation**: Add timeout in Entangler to return ETIMEDOUT gracefully

## Integration Points

This module is designed to integrate with:
- **Magnetar Catalog**: Manifest metadata source
- **Orbit Transfer Protocol**: Backend download mechanism
- **Core Starmap**: Block ID mapping and routing

## Development Status

**Current Implementation (v0.1.0 - Phase 2: Materialization Layer):**
- ✅ Core FUSE filesystem (OrbitGhostFS)
- ✅ Block-level entanglement logic (Entangler)
- ✅ Priority queue signaling (crossbeam-channel)
- ✅ Simulated wormhole transport
- ✅ Database-backed metadata (MagnetarAdapter)
- ✅ CLI argument parsing (clap)
- ✅ Inode translation layer (InodeTranslator with DashMap)
- ✅ Error handling with errno mapping (GhostError)
- ✅ Async/sync bridge (tokio runtime handle)
- ✅ Pluggable metadata abstraction (MetadataOracle trait)

**Next Steps (v0.2.0 - Phase 3: Production Hardening):**
- ⚠️ Replace polling with Condvar::wait()
- ⚠️ Add timeout to ensure_block_available()
- ⚠️ Real Orbit backend integration
- ⚠️ Thread pool for parallel downloads
- ⚠️ Prefetching heuristics
- ⚠️ Cache eviction policy (LRU)
- ⚠️ Configuration file support (TOML)
- ⚠️ Windows support (WinFSP)

## Technical Details

### FUSE Operations Implemented

- `lookup()`: Database-backed file/directory resolution via MetadataOracle
- `getattr()`: Metadata retrieval from Magnetar DB (no block I/O)
- `readdir()`: Directory listing from database with lazy inode allocation
- `read()`: Triggers block entanglement for on-demand fetching

### Block Request Flow

```
Application read() syscall
    ↓
FUSE read() handler (OrbitGhostFS)
    ↓
Translate inode → artifact_id (InodeTranslator)
    ↓
Calculate block range (offset / BLOCK_SIZE)
    ↓
Entangler.ensure_block_available()
    ├─→ [Cache hit] Return immediately
    └─→ [Cache miss] Send BlockRequest via channel
                        ↓
                    Wormhole thread receives request
                        ↓
                    Simulate 500ms network latency
                        ↓
                    Write block to {cache_dir}/{file_id}_{block}.bin
                        ↓
                    Entangler polling detects file exists
    ↓
Read block from cache, slice to exact byte range
    ↓
Return data to kernel
```

### Concurrency Model

- **FUSE threads**: Multiple read operations can occur concurrently
- **InodeTranslator**: Lock-free via DashMap + AtomicU64
- **Entangler**: Thread-safe via Arc<Mutex<HashMap>> for waiting rooms
- **Wormhole**: Single background thread with crossbeam-channel priority queue
- **Current limitation**: Polling loop (10ms sleep) instead of Condvar::wait()
- **Production**: Scale to thread pool for parallel downloads

## Configuration

### Block Size

Configured in [fs.rs:11](src/fs.rs#L11):

```rust
const BLOCK_SIZE: u64 = 1024 * 1024; // 1MB
const TTL: Duration = Duration::from_secs(1); // Attribute cache TTL
```

**Recommendation:** Increase to 5-16MB for production to align with cloud object storage chunk sizes.

### Runtime Configuration

All configuration is now via CLI arguments (no hardcoded constants):

```bash
orbit-ghost \
  --job-id 1 \
  --database /path/to/magnetar.db \
  --mount-point /custom/mount \
  --cache-dir /custom/cache
```

### Future Configuration (v0.3.0)

Configuration file support planned:

```toml
# orbit-ghost.toml
[database]
path = "magnetar.db"

[mount]
point = "/mnt/orbit"
cache_dir = "/var/cache/orbit"

[performance]
block_size = 5242880  # 5MB
prefetch_count = 3
cache_limit = 107374182400  # 100GB

[timeouts]
db_query = 5000  # 5s
block_fetch = 30000  # 30s
```

## Troubleshooting

### FUSE mount fails
- Ensure FUSE is installed (see Platform Support)
- Check permissions: `ls -l /dev/fuse`
- Verify mount point exists and is empty

### "Bus error" or crashes
- Check FUSE version compatibility
- Ensure libfuse3-dev is installed (not just libfuse3)

### Slow performance
- Increase BLOCK_SIZE for larger files
- Enable prefetching (implement heuristic lookahead)
- Check network latency to backend

### Build fails on Windows
- Expected: FUSE not available on Windows
- Use Linux/macOS for development
- Windows support requires WinFSP integration (future work)

## License

Part of the Orbit project. See root [LICENSE](../LICENSE) for details.

## Strategic Vision

This module represents a paradigm shift:

**Traditional Model:**
```
Download 100% → Process
```

**Quantum Entanglement Model:**
```
Download 0.1% → Process immediately → Download on-demand
```

Applications see instant availability. Bandwidth consumed only for accessed data. Perfect for large datasets where only subsets are needed.
