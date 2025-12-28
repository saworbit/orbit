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

1. **Manifest Plane**: Loads file structure into memory for instant directory listings
2. **Ghost Driver**: FUSE interface intercepting POSIX syscalls
3. **Entangler**: Maps OS read requests to block IDs and manages priority queue
4. **Wormhole Client**: Handles both sequential background fill and priority fetching

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

### Running the Demo

On Linux/macOS with FUSE installed:

```bash
cd orbit-ghost
chmod +x demo_quantum.sh
./demo_quantum.sh
```

This demonstrates:
1. Mounting a ghost filesystem
2. Projecting a 50MB virtual file
3. Reading the last 10 bytes (triggering only the final block fetch)
4. Measuring latency vs traditional full-file transfer

### Manual Operation

```bash
# Start the ghost filesystem
RUST_LOG=info cargo run

# In another terminal:
ls /tmp/orbit_ghost_mount
# Files appear instantly

# Read data (triggers on-demand fetch)
cat /tmp/orbit_ghost_mount/visionary_demo.mp4

# Unmount
fusermount -u /tmp/orbit_ghost_mount  # Linux
umount /tmp/orbit_ghost_mount         # macOS
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

**Current Implementation:**
- ✅ Core FUSE filesystem
- ✅ Block-level entanglement logic
- ✅ Priority queue signaling
- ✅ Simulated wormhole transport
- ✅ Demo script

**Production Roadiness:**
- ⚠️ Timeout/error handling needed
- ⚠️ Real backend integration pending
- ⚠️ Prefetching heuristics TODO
- ⚠️ Windows support not implemented
- ⚠️ Production block bitmap (RoaringBitmap) not implemented

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

Key constants in [src/fs.rs](src/fs.rs):

```rust
const BLOCK_SIZE: u64 = 1024 * 1024; // 1MB
```

Key constants in [src/main.rs](src/main.rs):

```rust
const MOUNT_POINT: &str = "/tmp/orbit_ghost_mount";
const CACHE_DIR: &str = "/tmp/orbit_cache";
```

Adjust these based on your deployment requirements.

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
