# Orbit Performance Guide

This guide provides information on optimizing Orbit performance and understanding how Orbit manages system resources.

## Concurrency Detection

Orbit attempts to automatically detect the number of available CPU cores to optimize parallel transfer threads.

### Auto-detection Behavior

- **Auto-detection:** Uses `std::thread::available_parallelism` to detect available CPU cores
- **Fallback:** If detection fails (e.g., in strict `cgroup` environments or restricted containers), Orbit defaults to **1 core** and logs a warning to stderr
- **Override:** You can manually set concurrency using the `--concurrency` (or `-j`) flag to bypass detection

### Why Single-Threaded Fallback?

When CPU detection fails, it typically indicates a restricted or hostile environment (e.g., containers with limited syscall access, strict cgroup configurations). In such cases:

- **Safety First:** Defaulting to 1 thread prevents resource exhaustion and OOM kills
- **Predictable Behavior:** Single-threaded mode is the safest fallback, guaranteeing minimal system pressure
- **User Awareness:** A warning is logged to stderr so operators know detection failed

### Manual Concurrency Override

If you need to override the detected concurrency level:

```bash
# Use 8 concurrent operations
orbit sync --concurrency 8 /source s3://bucket/destination

# Single-threaded mode
orbit sync --concurrency 1 /source s3://bucket/destination
```

### Optimal Concurrency

For I/O-bound operations like file transfers, Orbit uses `2 × CPU_count` concurrent operations, capped at 16. This allows better utilization during network I/O operations where threads spend time waiting.

## Performance Tips

1. **Network Transfers:** Higher concurrency helps saturate network bandwidth
2. **Local Operations:** Lower concurrency may be better for disk-bound operations
3. **Resource-Constrained Environments:** Use `--concurrency 1` or `--concurrency 2` to minimize resource usage
4. **High-Performance Servers:** Manually set higher concurrency if auto-detection caps too low

## Monitoring Performance

Watch for the CPU detection warning:

```
WARN: Orbit failed to detect available parallelism: <error>.
Defaulting to 1 concurrent operation to prevent resource exhaustion.
```

If you see this warning:
- Check your container/cgroup configuration
- Verify syscall permissions
- Consider manually setting `--concurrency` to an appropriate value for your environment

## Neutrino Fast Lane (v0.5+)

Orbit v0.5 introduces the **Neutrino Fast Lane**, an optimized pipeline for workloads with many small files (<8KB). This feature provides significant performance improvements for scenarios like:

- Migrating source code repositories (`node_modules`, `.git` directories)
- Backing up configuration directories (`/etc`, `.config`)
- Transferring log files and small assets
- Syncing documentation and web content

### How It Works

For small files, CDC chunking and deduplication overhead exceeds potential savings. The Neutrino Fast Lane:

1. **Routes by size**: Files <8KB bypass CDC/starmap indexing
2. **High concurrency**: Uses 100-500 concurrent async tasks (vs standard 16)
3. **Direct I/O**: Simple buffered copy without hashing overhead
4. **Reduced CPU load**: No BLAKE3 hashing or Adler-32 rolling hashes
5. **Reduced DB bloat**: Avoids starmap index entries for non-deduplicable files

### Usage

Enable Neutrino fast lane with the `--profile neutrino` flag:

```bash
# Basic usage
orbit copy --profile neutrino --recursive /source /dest

# With custom threshold (16KB)
orbit copy --profile neutrino --neutrino-threshold 16 --recursive /source /dest

# Combined with smart sync for priority-based transfers
orbit copy --check smart --profile neutrino --recursive /source /dest
```

### Performance Characteristics

**Benchmark Results** (10,000 files of 1-4KB each):

| Mode | Time | CPU Usage | DB Size |
|------|------|-----------|---------|
| Standard | ~45s | High (BLAKE3 hashing) | 100MB index |
| Neutrino | ~15s | Low (direct I/O) | Minimal index |

**Performance Gain**: ~3x faster for small-file workloads

### Configuration Options

```bash
# Transfer profile (standard, neutrino, adaptive)
--profile <PROFILE>

# Neutrino threshold in KB (default: 8)
--neutrino-threshold <KB>

# Concurrency override (default: 200 for Neutrino)
--parallel <N>
```

### When to Use Neutrino

✅ **Best For:**
- Source code trees (`.js`, `.py`, `.rs` files)
- Configuration directories (`/etc`, `/usr/local/etc`)
- Log files and small assets
- npm/pip package directories

❌ **Not Recommended For:**
- Large media files (videos, disk images)
- Database backups (deduplication beneficial)
- Mixed workloads (use `adaptive` profile instead)

### Adaptive Mode (Coming Soon)

The `--profile adaptive` mode will automatically route files based on:
- File size distribution analysis
- Workload profiling
- Historical deduplication effectiveness

### Technical Details

**Requirements:**
- Requires `backend-abstraction` feature (included with network backends)
- Uses tokio async runtime for I/O-bound concurrency
- Compatible with all Smart Sync priority modes

**Limitations:**
- No deduplication for small files (by design)
- No delta detection (full file transfer)
- Metadata preservation still applies

**Architecture:**
- Router: Size-based routing ("The Sieve")
- Executor: tokio::JoinSet with semaphore-based concurrency control
- Integration: Phase 2a in Smart Sync pipeline (before standard lane)

For more details, see [ORBIT_V2_ARCHITECTURE.md](../architecture/ORBIT_V2_ARCHITECTURE.md).
