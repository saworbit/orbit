# Orbit GhostFS User Guide

Complete guide to using Orbit GhostFS for on-demand remote data access.

## Table of Contents

- [Introduction](#introduction)
- [Quick Start](#quick-start)
- [Basic Usage](#basic-usage)
- [Advanced Usage](#advanced-usage)
- [Configuration](#configuration)
- [Performance Tuning](#performance-tuning)
- [Use Cases](#use-cases)
- [Best Practices](#best-practices)
- [Monitoring](#monitoring)

## Introduction

### What is Orbit GhostFS?

Orbit GhostFS is a filesystem that makes remote data appear local, fetching it on-demand as applications access it. Instead of downloading entire datasets before use, GhostFS transfers only the blocks your applications actually read.

### Key Concepts

**Ghost Files**: Virtual files that exist in the filesystem but whose data is fetched on-demand from remote storage.

**Quantum Entanglement**: The mechanism where reading a file triggers immediate, priority fetching of the required data blocks.

**Block-Level JIT**: Just-In-Time fetching at the block level (default: 1MB blocks), ensuring minimal data transfer.

**Priority Queue**: User-initiated reads preempt background downloads, ensuring responsive performance.

### When to Use GhostFS

✅ **Good Use Cases:**
- Large datasets where only portions are accessed
- Video/media files (seeking, preview thumbnails)
- Log files (reading recent entries)
- Database files (indexed queries)
- Machine learning datasets (sampling, exploration)

❌ **Poor Use Cases:**
- Small files (< 10 MB) - overhead not worth it
- Full sequential reads - traditional transfer is faster
- Write-intensive workloads - currently read-only
- Applications requiring guaranteed offline access

## Quick Start

### 1. Start the Ghost Filesystem

```bash
cd orbit-ghost
./target/release/orbit-ghost
```

**Output:**
```
[Orbit] 🌌 Projecting Holographic Filesystem at /tmp/orbit_ghost_mount
[Wormhole] Transport Layer Active.
```

Leave this running in the background or press Ctrl+Z then `bg`.

### 2. Access Ghost Files

In another terminal:

```bash
# List files (instant - from manifest)
ls -lh /tmp/orbit_ghost_mount

# Check file metadata (instant - no download)
stat /tmp/orbit_ghost_mount/visionary_demo.mp4

# Read data (triggers on-demand fetch)
head -c 1000 /tmp/orbit_ghost_mount/visionary_demo.mp4
```

### 3. Stop the Filesystem

```bash
# Find the process
ps aux | grep orbit-ghost

# Unmount
fusermount -u /tmp/orbit_ghost_mount  # Linux
umount /tmp/orbit_ghost_mount         # macOS

# Or kill the process (auto-unmounts)
killall orbit-ghost
```

## Basic Usage

### Mounting

#### Default Mount

```bash
./target/release/orbit-ghost
# Mounts to /tmp/orbit_ghost_mount
```

#### Background Mode

```bash
# Run in background
./target/release/orbit-ghost &

# Or with nohup (survives terminal close)
nohup ./target/release/orbit-ghost > orbit-ghost.log 2>&1 &
```

#### With Logging

```bash
# Enable debug logs
RUST_LOG=debug ./target/release/orbit-ghost

# Specific module logs
RUST_LOG=orbit_ghost::entangler=trace ./target/release/orbit-ghost

# Log to file
RUST_LOG=info ./target/release/orbit-ghost 2>&1 | tee orbit-ghost.log
```

### Unmounting

#### Clean Unmount

```bash
# Linux
fusermount -u /tmp/orbit_ghost_mount

# macOS
umount /tmp/orbit_ghost_mount

# Alternative: Ctrl+C in the terminal running orbit-ghost
```

#### Force Unmount (if hanging)

```bash
# Linux
fusermount -uz /tmp/orbit_ghost_mount

# macOS/Linux (requires sudo)
sudo umount -f /tmp/orbit_ghost_mount
```

### Reading Files

#### Sequential Read

```bash
# Read entire file (fetches all blocks sequentially)
cat /tmp/orbit_ghost_mount/visionary_demo.mp4 > local_copy.mp4
```

#### Random Access

```bash
# Read last 1000 bytes (fetches only final block)
tail -c 1000 /tmp/orbit_ghost_mount/visionary_demo.mp4

# Read specific byte range with dd
dd if=/tmp/orbit_ghost_mount/visionary_demo.mp4 \
   of=chunk.bin \
   bs=1M skip=25 count=1
# Skips 25 MB, reads 1 MB (fetches only block 25)
```

#### Streaming

```bash
# Stream video with mpv (fetches blocks as played)
mpv /tmp/orbit_ghost_mount/visionary_demo.mp4

# Stream with VLC
vlc /tmp/orbit_ghost_mount/visionary_demo.mp4

# Play audio
ffplay /tmp/orbit_ghost_mount/audio_file.mp3
```

### Inspecting Files

#### Metadata (No Download)

```bash
# File size, permissions, timestamps
stat /tmp/orbit_ghost_mount/visionary_demo.mp4

# Just the size
ls -lh /tmp/orbit_ghost_mount/visionary_demo.mp4

# File type
file /tmp/orbit_ghost_mount/visionary_demo.mp4
```

#### Directory Listing (Instant)

```bash
# Simple list
ls /tmp/orbit_ghost_mount

# Detailed listing
ls -lah /tmp/orbit_ghost_mount

# Tree view (if tree is installed)
tree /tmp/orbit_ghost_mount
```

## Advanced Usage

### Cache Management

#### View Cache

```bash
# See downloaded blocks
ls -lh /tmp/orbit_cache/

# Check cache size
du -sh /tmp/orbit_cache/
```

#### Clear Cache

```bash
# Remove all cached blocks
rm -rf /tmp/orbit_cache/*

# Or remove cache for specific file
rm /tmp/orbit_cache/file_123_*.bin
```

#### Prewarm Cache

```bash
# Download specific blocks in advance
# (Currently requires manual implementation)

# Full file download to cache
cat /tmp/orbit_ghost_mount/visionary_demo.mp4 > /dev/null
# Now subsequent reads are instant (served from cache)
```

### Performance Monitoring

#### Real-time Activity

```bash
# Watch cache growth
watch -n 1 'du -sh /tmp/orbit_cache/'

# Monitor block fetches
tail -f orbit-ghost.log | grep "Quantum Request"

# System resource usage
top -p $(pgrep orbit-ghost)
```

#### Benchmarking

```bash
# Measure cold read (no cache)
rm -rf /tmp/orbit_cache/*
time cat /tmp/orbit_ghost_mount/visionary_demo.mp4 > /dev/null

# Measure warm read (cached)
time cat /tmp/orbit_ghost_mount/visionary_demo.mp4 > /dev/null

# Compare with traditional transfer
time wget https://example.com/file.mp4 -O /dev/null
```

### Integration with Applications

#### Jupyter Notebooks

```python
# Access large datasets without full download
import pandas as pd

# This reads only the first 1MB to get CSV header/preview
df = pd.read_csv('/tmp/orbit_ghost_mount/huge_dataset.csv', nrows=1000)

# Full read fetches all blocks
df_full = pd.read_csv('/tmp/orbit_ghost_mount/huge_dataset.csv')
```

#### Video Editing

```bash
# Load in video editor (fetches on seek)
shotcut /tmp/orbit_ghost_mount/raw_footage.mp4

# Or use ffmpeg for quick preview
ffmpeg -i /tmp/orbit_ghost_mount/raw_footage.mp4 \
       -ss 00:01:00 -t 00:00:10 \
       -c copy preview.mp4
# Only fetches blocks around the 1-minute mark
```

#### Data Science

```python
# PyTorch DataLoader with random access
from torch.utils.data import Dataset
import numpy as np

class GhostDataset(Dataset):
    def __init__(self, ghost_file):
        self.file = ghost_file
        # File size read from metadata (instant)
        self.size = os.path.getsize(ghost_file)

    def __getitem__(self, idx):
        # Each access fetches only required block
        with open(self.file, 'rb') as f:
            f.seek(idx * 1024)
            data = f.read(1024)
        return np.frombuffer(data, dtype=np.float32)

    def __len__(self):
        return self.size // 1024

# Only accessed samples are downloaded
dataset = GhostDataset('/tmp/orbit_ghost_mount/features.bin')
```

#### Database Access

```bash
# SQLite can work with ghost files (random access patterns)
sqlite3 /tmp/orbit_ghost_mount/database.db "SELECT * FROM users LIMIT 10;"
# Only fetches blocks containing queried data

# Note: Write operations will fail (read-only FS)
```

## Configuration

### Environment Variables

```bash
# Cache directory (default: /tmp/orbit_cache)
export ORBIT_CACHE_DIR=/var/cache/orbit

# Mount point (default: /tmp/orbit_ghost_mount)
export ORBIT_MOUNT_POINT=/mnt/ghostfs

# Block size in bytes (default: 1048576 = 1MB)
export ORBIT_BLOCK_SIZE=5242880  # 5MB

# Enable debug logging
export RUST_LOG=debug

# Manifest source (future)
export ORBIT_MANIFEST_URL=https://magnetar.example.com/manifest.json
```

### Runtime Flags (Future)

```bash
# Current version has no CLI flags, but planned:
orbit-ghost \
  --mount-point /mnt/data \
  --cache-dir /var/cache/orbit \
  --block-size 5M \
  --prefetch 3 \
  --cache-limit 50G \
  --read-only \
  --allow-other
```

## Performance Tuning

### Block Size Optimization

**Small Blocks (64 KB - 256 KB):**
- ✅ Low latency for random access
- ❌ High HTTP request overhead
- **Use case:** Database files, random seeks

**Medium Blocks (1 MB - 4 MB):**
- ✅ Balanced performance
- ✅ Good for most workloads
- **Use case:** General purpose (default)

**Large Blocks (16 MB - 64 MB):**
- ✅ Maximum throughput for sequential reads
- ✅ Aligns with cloud storage chunk sizes
- ❌ Wastes bandwidth on random access
- **Use case:** Video streaming, large file transfers

**Changing block size** (requires code modification currently):

Edit [src/fs.rs:8](src/fs.rs):
```rust
const BLOCK_SIZE: u64 = 5 * 1024 * 1024; // Change to 5MB
```

### Cache Strategy

#### Unlimited Cache (Default)

```bash
# Cache grows indefinitely in /tmp/orbit_cache
# Pro: Maximum hit rate after initial access
# Con: Can fill disk
```

#### Manual LRU (Script)

```bash
#!/bin/bash
# Limit cache to 10 GB

CACHE_DIR="/tmp/orbit_cache"
MAX_SIZE_GB=10

while true; do
  CURRENT_SIZE=$(du -sb $CACHE_DIR | cut -f1)
  MAX_SIZE=$((MAX_SIZE_GB * 1024 * 1024 * 1024))

  if [ $CURRENT_SIZE -gt $MAX_SIZE ]; then
    # Delete oldest files
    find $CACHE_DIR -type f -printf '%T+ %p\n' | \
      sort | head -n 100 | cut -d' ' -f2- | xargs rm -f
  fi

  sleep 60
done
```

#### Session-based Cache

```bash
# Clear cache on each mount
rm -rf /tmp/orbit_cache/*
./target/release/orbit-ghost
```

### Network Optimization

#### Increase Download Parallelism

Currently single-threaded. Future enhancement:

```rust
// In main.rs, replace single thread with thread pool
let pool = ThreadPoolBuilder::new()
    .num_threads(8)
    .build()
    .unwrap();

pool.install(|| {
    // Handle block downloads in parallel
});
```

#### Prefetch Next Blocks

Modify [src/entangler.rs](src/entangler.rs):

```rust
pub fn ensure_block_available(&self, file_id: &str, block_index: u64) {
    // Fetch requested block
    self.fetch_block(file_id, block_index);

    // Prefetch next 3 blocks (heuristic)
    for i in 1..=3 {
        self.fetch_block(file_id, block_index + i);
    }
}
```

## Use Cases

### 1. Video Streaming

**Scenario:** Preview a 50GB video file stored remotely.

```bash
mpv /tmp/orbit_ghost_mount/raw_footage.mp4
```

**Behavior:**
- Metadata loads instantly (file size, duration, codec info)
- First few MB downloaded to start playback
- Subsequent blocks fetched as you watch
- Seeking to minute 30 fetches only blocks around that timestamp

**Bandwidth Saved:** If you watch 5 minutes of a 50GB file, you download ~500MB instead of 50GB.

### 2. Log Analysis

**Scenario:** Analyze recent entries in a 100GB log file.

```bash
tail -n 10000 /tmp/orbit_ghost_mount/application.log | grep ERROR
```

**Behavior:**
- Only the last ~100MB is downloaded
- 99.9GB remains untouched

### 3. Machine Learning Dataset Sampling

**Scenario:** Explore a 1TB dataset to understand structure.

```python
import h5py

# Open dataset (metadata only, instant)
f = h5py.File('/tmp/orbit_ghost_mount/dataset.h5', 'r')

# Read 1000 random samples (fetches ~1GB of blocks)
indices = np.random.randint(0, 1000000, size=1000)
samples = [f['data'][i] for i in indices]
```

**Bandwidth Saved:** 1GB transferred instead of 1TB.

### 4. Database Query on Remote DB

**Scenario:** Run indexed query on large SQLite database.

```bash
sqlite3 /tmp/orbit_ghost_mount/analytics.db \
  "SELECT * FROM events WHERE timestamp > '2024-01-01' LIMIT 100;"
```

**Behavior:**
- SQLite seeks to index blocks
- Only index + matching data pages are fetched
- Bulk of database untouched

### 5. Docker Image Layer Exploration

**Scenario:** Inspect Docker image without full pull.

```bash
# Mount image as ghost filesystem
ls /tmp/orbit_ghost_mount/layers/

# Read specific config
cat /tmp/orbit_ghost_mount/layers/sha256:abc123/config.json
```

**Behavior:**
- Directory listing is instant (from manifest)
- Only accessed layers are downloaded

## Best Practices

### 1. Organize Data by Access Patterns

**Good:** Separate hot and cold data
```
/cold-archive/  ← Large files, rarely accessed
/hot-data/      ← Frequently accessed, keep local
```

**Bad:** Mixed hot/cold in same ghost filesystem
```
/mixed/  ← Causes frequent cache misses
```

### 2. Use Appropriate Block Sizes

- **Random access (databases):** 256 KB - 1 MB
- **Sequential (video):** 4 MB - 16 MB
- **General purpose:** 1 MB (default)

### 3. Prewarm Cache for Predictable Access

```bash
# If you know you'll need specific files, prefetch them
cat /tmp/orbit_ghost_mount/frequently_used.dat > /dev/null &

# Then access is instant
your-application /tmp/orbit_ghost_mount/frequently_used.dat
```

### 4. Monitor Cache Size

```bash
# Set up monitoring
watch -n 5 'du -sh /tmp/orbit_cache/'

# Or add to crontab
*/5 * * * * du -sh /tmp/orbit_cache/ >> /var/log/orbit-cache-size.log
```

### 5. Handle Errors Gracefully

Applications should handle I/O errors (network issues):

```python
import errno

try:
    data = open('/tmp/orbit_ghost_mount/file.dat').read()
except IOError as e:
    if e.errno == errno.EIO:
        print("Network error fetching data, retrying...")
        # Implement retry logic
```

## Monitoring

### Filesystem Statistics

```bash
# Mount status
mount | grep orbit_ghost

# FUSE stats (Linux)
cat /proc/self/mountstats | grep -A 20 orbit_ghost
```

### Block Fetch Activity

```bash
# Enable detailed logging
RUST_LOG=info ./target/release/orbit-ghost 2>&1 | tee orbit.log

# In another terminal, watch fetches
tail -f orbit.log | grep "Quantum Request"
```

**Example output:**
```
[Orbit-Ghost] 🚀 Quantum Request: file_123 Block 0
[Wormhole] ⚡ Intercepted PRIORITY request for Block 0
[Wormhole] ✅ Block 0 Downloaded & Cached.
```

### Performance Metrics

```bash
# Application access latency
time cat /tmp/orbit_ghost_mount/file.dat > /dev/null

# Cache hit rate (manual calculation)
TOTAL_READS=$(grep "Quantum Request" orbit.log | wc -l)
CACHE_HITS=$(grep "Block exists, returning" orbit.log | wc -l)
HIT_RATE=$(echo "scale=2; $CACHE_HITS / $TOTAL_READS * 100" | bc)
echo "Cache hit rate: ${HIT_RATE}%"
```

### System Resource Usage

```bash
# CPU and memory
top -p $(pgrep orbit-ghost)

# I/O statistics
iotop -p $(pgrep orbit-ghost)

# Network bandwidth (if using real backend)
iftop -f "host backend.example.com"
```

## Troubleshooting

See [FAQ.md](FAQ.md) for common issues and solutions.

## Next Steps

- Learn about the architecture in [ARCHITECTURE.md](ARCHITECTURE.md)
- Contribute improvements via [CONTRIBUTING.md](CONTRIBUTING.md)
- Check the roadmap in [ROADMAP.md](ROADMAP.md)

## Support

- **Issues:** [GitHub Issues](https://github.com/saworbit/orbit/issues)
- **Discussions:** [GitHub Discussions](https://github.com/saworbit/orbit/discussions)
- **Email:** shaneawall@gmail.com
