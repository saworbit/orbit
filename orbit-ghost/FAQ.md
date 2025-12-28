# Orbit GhostFS Frequently Asked Questions

Answers to common questions about Orbit GhostFS.

## Table of Contents

- [General Questions](#general-questions)
- [Installation & Setup](#installation--setup)
- [Usage & Operations](#usage--operations)
- [Performance](#performance)
- [Troubleshooting](#troubleshooting)
- [Security & Privacy](#security--privacy)
- [Development](#development)

## General Questions

### What is Orbit GhostFS?

Orbit GhostFS is a FUSE-based filesystem that makes remote data appear local by fetching it on-demand at the block level. Instead of downloading entire files before use, it transfers only the blocks your applications actually access.

**Think of it as:**
- Netflix for files (streaming vs downloading)
- Virtual memory for remote storage (paging on-demand)
- Quantum entanglement for data (instant projection)

### How does it differ from traditional file sync (Dropbox, Google Drive)?

| Feature | Traditional Sync | Orbit GhostFS |
|---------|------------------|---------------|
| **Initial wait** | Hours (full download) | Seconds (manifest only) |
| **Disk usage** | 100% of remote data | Only accessed blocks |
| **Bandwidth** | Downloads everything | Downloads on-demand |
| **Access latency** | Zero (local) | Network latency (first access) |
| **Best for** | Frequent full access | Sparse, random access |

**Example:** With a 100GB dataset where you only use 5GB:
- **Traditional sync:** Downloads 100GB, uses 100GB disk
- **GhostFS:** Downloads 5GB, uses 5GB disk

### What are the main use cases?

1. **Large media libraries:** Preview videos without downloading entire files
2. **Scientific datasets:** Explore large datasets (genomics, astronomy) by sampling
3. **Log analysis:** Read recent entries from massive log files
4. **Machine learning:** Access training data with random sampling patterns
5. **Database queries:** Run indexed queries without full database download

### Is it production-ready?

**Current status (v0.1.0):** Proof of concept, not production-ready.

**Limitations:**
- No timeout handling (can hang on network issues)
- Single-threaded downloads
- No cache management (unlimited growth)
- Linux/macOS only

**Production readiness:** Planned for v0.2.0 (Q2 2024). See [ROADMAP.md](ROADMAP.md).

### How does it compare to NFS/SMB network filesystems?

**Similarities:**
- Both provide remote file access via POSIX interface

**Key differences:**
- **NFS/SMB:** Synchronous remote operations (every read is a network call)
- **GhostFS:** Asynchronous with local caching (first read is network, subsequent reads are local)

**GhostFS advantages:**
- Works over high-latency networks (cloud storage)
- Reduces redundant transfers (cache persists)
- Prioritizes interactive requests

**NFS/SMB advantages:**
- Support writes (GhostFS is currently read-only)
- Lower complexity (well-established protocols)
- Better for low-latency LANs

## Installation & Setup

### What platforms are supported?

- ✅ **Linux:** Fully supported (Ubuntu, Debian, Fedora, Arch, RHEL)
- ⚠️ **macOS:** Experimental (requires macFUSE, kernel extension approval)
- ❌ **Windows:** Not yet supported (planned for v0.5.0 via WinFSP)
- ❓ **FreeBSD:** Untested (may work with modifications)

See [INSTALL.md](INSTALL.md) for details.

### Do I need root/sudo to run it?

**Short answer:** No, for mounting. Yes, for installation.

**Details:**
- **Installation:** Root needed to install FUSE libraries (`sudo apt-get install fuse3`)
- **Mounting:** Normal user can mount if added to `fuse` group or `/etc/fuse.conf` allows it
- **Default behavior:** Uses `/tmp/` directories (writable by all users)

**Recommended setup:**
```bash
# One-time setup (requires sudo)
sudo usermod -a -G fuse $USER
echo "user_allow_other" | sudo tee -a /etc/fuse.conf

# Log out and back in

# Now you can run without sudo
./orbit-ghost
```

### Can I change the mount point?

**Current version (v0.1.0):** Hardcoded to `/tmp/orbit_ghost_mount`.

**Workaround:** Edit [src/main.rs:12](src/main.rs):
```rust
const MOUNT_POINT: &str = "/home/user/my_mount_point";
```

Then rebuild: `cargo build --release`

**Future:** v0.2.0 will support CLI arguments and config files.

### How much disk space does it use?

**Minimum:** ~100 MB for the binary and metadata.

**Cache:** Grows with usage (1:1 with accessed data).

**Example:**
- You mount a 1TB dataset
- You read 50GB of data
- Cache grows to ~50GB (plus small overhead)

**Recommendation:** Provision 10-20% of dataset size for cache, or implement LRU eviction (planned for v0.2.0).

## Usage & Operations

### How do I check if it's working?

```bash
# 1. Verify mount
mount | grep orbit_ghost
# Should show: orbit_ghost on /tmp/orbit_ghost_mount type fuse.orbit_ghost

# 2. List files (instant - from manifest)
ls /tmp/orbit_ghost_mount

# 3. Read data (triggers fetch)
head -c 100 /tmp/orbit_ghost_mount/visionary_demo.mp4

# 4. Check cache
ls /tmp/orbit_cache/
# Should show downloaded blocks: file_123_0.bin, etc.
```

### Can I write to ghost files?

**No.** Current implementation is read-only.

**Workaround:** Copy file locally, modify, then sync back:
```bash
cp /tmp/orbit_ghost_mount/file.dat /tmp/local_copy.dat
# Edit local_copy.dat
# Sync back to remote storage (outside GhostFS)
```

**Future:** Write support planned for v1.1.0+ (challenging due to conflict resolution).

### What happens if I lose network connection?

**Current behavior (v0.1.0):** Read operations **hang indefinitely** until connection restores.

**Applications will:**
- Freeze/hang on I/O
- Eventually timeout (if they implement timeouts)

**Recommended workaround:**
```bash
# Kill the hung process
killall orbit-ghost

# Force unmount
fusermount -uz /tmp/orbit_ghost_mount
```

**Future:** v0.2.0 will add timeout support (return `EIO` error after 30 seconds).

### Can I use it with multiple applications simultaneously?

**Yes.** FUSE supports concurrent access.

**Example:**
```bash
# Terminal 1
mpv /tmp/orbit_ghost_mount/video.mp4

# Terminal 2 (simultaneously)
cat /tmp/orbit_ghost_mount/data.csv | grep "pattern"
```

**Behavior:**
- Both applications read independently
- Blocks are fetched on-demand for each
- Cache is shared (if both read same blocks, second access is instant)

**Limitation (v0.1.0):** Single download thread is a bottleneck. Concurrent requests are serialized. Fixed in v0.2.0 with thread pool.

### How do I unmount cleanly?

```bash
# Method 1: Ctrl+C in the terminal running orbit-ghost
# (Auto-unmounts via fuser::MountOption::AutoUnmount)

# Method 2: Manual unmount
fusermount -u /tmp/orbit_ghost_mount  # Linux
umount /tmp/orbit_ghost_mount         # macOS

# Method 3: Force unmount (if hanging)
fusermount -uz /tmp/orbit_ghost_mount      # Linux
sudo umount -f /tmp/orbit_ghost_mount      # macOS
```

### Can I access the same mount from multiple machines?

**No.** Each GhostFS instance is local to one machine.

**Workaround:** Run GhostFS on each machine, all pointing to the same backend. They share the backend data but have independent local caches.

**Future:** Distributed cache synchronization (planned for v2.0+).

## Performance

### Is it faster than downloading the entire file?

**Depends on access pattern:**

**Faster:**
- Reading last 1 MB of a 100 GB file: **100x faster**
- Random sampling (1% of data): **100x faster**
- Previewing video (first 10 seconds): **10-50x faster**

**Slower:**
- Full sequential read: **20-30% slower** (overhead from block fetching)

**Break-even point:** If you access > 80% of a file, traditional download is often faster.

### What's the latency for the first read?

**Components:**
1. **Manifest load:** < 1 second (one-time, on mount)
2. **Block fetch:** Network latency + download time
   - Example: 1 MB block @ 10 MB/s = ~100ms
   - Over 100ms RTT network: ~200ms total

**Subsequent reads (cached):** < 1ms (disk I/O)

### How can I improve performance?

**1. Increase block size** (for sequential reads):
```rust
// Edit src/fs.rs
const BLOCK_SIZE: u64 = 16 * 1024 * 1024; // 16 MB instead of 1 MB
```

**2. Prewarm cache** (prefetch known files):
```bash
cat /tmp/orbit_ghost_mount/important_file.dat > /dev/null &
# Downloads in background, subsequent access is instant
```

**3. Use SSD for cache** (faster disk I/O):
```rust
// Edit src/main.rs
const CACHE_DIR: &str = "/mnt/ssd/orbit_cache";
```

**4. Wait for v0.2.0:**
- Parallel downloads (10x improvement)
- Heuristic prefetching (50% latency reduction)
- Optimized synchronization (lower CPU usage)

### Does it use compression?

**Current version:** No compression.

**Data transfer:** Raw blocks as stored remotely.

**Future:** v0.3.0 will integrate with Orbit backend, which supports:
- LZ4 compression (fast)
- Zstd compression (high ratio)
- CDC (content-defined chunking) for deduplication

## Troubleshooting

### Error: "Transport endpoint is not connected"

**Cause:** Previous mount was not cleaned up properly.

**Fix:**
```bash
fusermount -u /tmp/orbit_ghost_mount
# Or force unmount:
sudo umount -f /tmp/orbit_ghost_mount

# Then remount
./target/release/orbit-ghost
```

### Error: "FUSE device not found"

**Cause:** FUSE kernel module not loaded.

**Fix:**
```bash
# Load FUSE module
sudo modprobe fuse

# Verify
ls -l /dev/fuse
# Should show: crw-rw-rw- 1 root root 10, 229 ...

# Auto-load on boot (Ubuntu/Debian)
echo "fuse" | sudo tee -a /etc/modules
```

### Error: "Permission denied" when mounting

**Cause:** User not allowed to mount FUSE filesystems.

**Fix:**
```bash
# Option 1: Add user to fuse group
sudo usermod -a -G fuse $USER
# Log out and back in

# Option 2: Edit /etc/fuse.conf
echo "user_allow_other" | sudo tee -a /etc/fuse.conf

# Verify
id | grep fuse  # Should show fuse group
```

### Application hangs when reading files

**Cause:** Network issue preventing block fetch (no timeout in v0.1.0).

**Diagnosis:**
```bash
# Check if orbit-ghost is running
ps aux | grep orbit-ghost

# Check logs
tail -f orbit-ghost.log | grep "Quantum Request"
# If stuck on a block, it's a network issue
```

**Fix:**
```bash
# Kill and restart
killall orbit-ghost
fusermount -u /tmp/orbit_ghost_mount
./target/release/orbit-ghost
```

**Future:** v0.2.0 adds timeout to prevent hangs.

### Cache directory fills up disk

**Cause:** No automatic eviction (unlimited cache in v0.1.0).

**Fix:**
```bash
# Manual cleanup
rm -rf /tmp/orbit_cache/*

# Or delete old blocks
find /tmp/orbit_cache -type f -mtime +7 -delete
```

**Future:** v0.2.0 implements LRU eviction with configurable size limits.

### Build fails on macOS: "ld: library not found for -lfuse"

**Cause:** macFUSE not installed or pkg-config can't find it.

**Fix:**
```bash
# Install macFUSE
brew install --cask macfuse

# Approve kernel extension (System Preferences → Security)
# Restart Mac

# Verify installation
pkg-config --modversion fuse
# Should output version number

# Rebuild
cargo clean
cargo build --release
```

### Build fails on Windows: "could not find fuse3"

**Expected.** Windows doesn't have FUSE.

**Options:**
1. **Use WSL2:** Run Linux inside Windows
2. **Wait for v0.5.0:** WinFSP support planned
3. **Use virtual machine:** Run Linux in VM

See [WINDOWS.md](WINDOWS.md) for details.

## Security & Privacy

### Is my data encrypted in transit?

**Current version (v0.1.0):** No encryption (simulated backend).

**v0.3.0 (Orbit integration):** Yes, all backend communication will use TLS 1.3.

### Is my data encrypted at rest (in cache)?

**No.** Cache files are plain data in `/tmp/orbit_cache/`.

**Security recommendation:**
```bash
# Set restrictive permissions
chmod 700 /tmp/orbit_cache
# Only you can read cached blocks
```

**Future:** v1.0.0 may add optional cache encryption.

### Can other users see my mounted files?

**Depends on mount options.**

**Current default:** Only the user who mounted can access.

**To allow other users:**
```rust
// Edit src/main.rs, add:
fuser::MountOption::AllowOther,
```

**Then edit `/etc/fuse.conf`:**
```bash
echo "user_allow_other" | sudo tee -a /etc/fuse.conf
```

### What data does GhostFS collect?

**None.** GhostFS is fully local, no telemetry or analytics.

**Network communication (v0.3.0+):** Only with your configured backend to fetch blocks.

## Development

### How can I contribute?

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Quick start:**
1. Fork the repository
2. Create a feature branch
3. Make changes, add tests
4. Submit pull request

**Areas needing help:**
- Windows support (WinFSP)
- Prefetching algorithms
- Cache eviction policies
- Documentation improvements

### What's the tech stack?

- **Language:** Rust (edition 2021)
- **FUSE Library:** `fuser` 0.14
- **Concurrency:** `tokio`, `crossbeam-channel`, `dashmap`
- **Logging:** `env_logger`, `log`

### How do I run tests?

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_test -- --ignored

# With logs
RUST_LOG=debug cargo test -- --nocapture
```

### Is there a public roadmap?

Yes: [ROADMAP.md](ROADMAP.md)

**Highlights:**
- **v0.2.0 (Q2 2024):** Production hardening
- **v0.3.0 (Q3 2024):** Orbit integration
- **v0.5.0 (Q1 2025):** Windows support
- **v1.0.0 (Q2 2025):** Enterprise launch

### Where can I get help?

- **Documentation:** Start with [README.md](README.md) and [GUIDE.md](GUIDE.md)
- **Issues:** [GitHub Issues](https://github.com/saworbit/orbit/issues)
- **Discussions:** [GitHub Discussions](https://github.com/saworbit/orbit/discussions)
- **Email:** shaneawall@gmail.com

## Still Have Questions?

If your question isn't answered here:

1. Check the [User Guide](GUIDE.md) for detailed usage instructions
2. Read the [Architecture Document](ARCHITECTURE.md) for technical details
3. Search [existing issues](https://github.com/saworbit/orbit/issues)
4. Open a [new discussion](https://github.com/saworbit/orbit/discussions/new)

**Found a bug?** [Report it](https://github.com/saworbit/orbit/issues/new?template=bug_report.md)

**Want a feature?** [Request it](https://github.com/saworbit/orbit/issues/new?template=feature_request.md)
