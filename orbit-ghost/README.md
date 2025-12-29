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

1. **Metadata Oracle**: Database-backed metadata layer (Magnetar integration)
2. **Inode Translator**: Bidirectional mapping between FUSE inodes and artifact IDs
3. **Ghost Driver**: FUSE interface intercepting POSIX syscalls
4. **Entangler**: Maps OS read requests to block IDs and manages priority queue
5. **Wormhole Client**: Handles both sequential background fill and priority fetching

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

### Failure Modes
- **Network Loss**: Read syscalls will hang unless timeout implemented
- **Recommendation**: Add timeout in Entangler loop to return EIO gracefully

## Integration Points

This module is designed to integrate with:
- **Magnetar Catalog**: Manifest metadata source
- **Orbit Transfer Protocol**: Backend download mechanism
- **Core Starmap**: Block ID mapping and routing

## Development Status

**Current Implementation (v0.2.0 - Phase 2):**
- ✅ Core FUSE filesystem
- ✅ Block-level entanglement logic
- ✅ Priority queue signaling
- ✅ Simulated wormhole transport
- ✅ **Database-backed metadata** (Magnetar integration)
- ✅ **CLI argument parsing** (clap)
- ✅ **Inode translation layer** (lazy allocation)
- ✅ **Error handling** (errno mapping)
- ✅ **Async/sync bridge** (tokio runtime handle)

**Next Steps (v0.3.0):**
- ⚠️ Replace polling with Condvar::wait()
- ⚠️ Real Orbit backend integration
- ⚠️ Timeout/retry for database queries
- ⚠️ Prefetching heuristics
- ⚠️ Cache eviction policy (LRU)
- ⚠️ Windows support (WinFSP)

## Technical Details

### FUSE Operations Implemented

- `lookup()`: Instant file/directory resolution from manifest
- `getattr()`: Metadata retrieval without network I/O
- `readdir()`: Directory listing from in-memory manifest
- `read()`: Triggers quantum entanglement for block fetching

### Block Request Flow

```
Application read() syscall
    ↓
FUSE read() handler
    ↓
Calculate block range
    ↓
Entangler.ensure_block_available()
    ↓
Check local cache
    ↓
[If missing] Send priority signal to Wormhole
    ↓
Block until block appears in cache
    ↓
Read from cache and return data
```

### Concurrency Model

- **FUSE threads**: Multiple read operations can occur concurrently
- **Entangler**: Thread-safe via Arc<Mutex<HashMap>>
- **Wormhole**: Single background thread with channel-based priority queue
- **Production**: Scale to thread pool for parallel downloads

## Configuration

### Block Size

Configured in [src/fs.rs](src/fs.rs:11):

```rust
const BLOCK_SIZE: u64 = 1024 * 1024; // 1MB
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
