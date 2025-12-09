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

## The "Equilibrium" Standard Lane

The **Equilibrium** profile is Orbit's default operating mode, optimized for the majority of your data: source code repositories, PDF documents, office files, and moderate binary assets (8KB to 1GB). In this zone, the CPU cost of deduplication is outweighed by the bandwidth savings.

### Design Philosophy

Equilibrium represents a balanced trade-off between three competing resources:
- **CPU**: Content-Defined Chunking (CDC) with BLAKE3 hashing
- **Memory**: Universe Map indexing for global deduplication
- **Network**: Delta transfers (only unique chunks are sent)

### The Pipeline Flow

```
File (8KB - 1GB)
    ↓
Chunking: CDC with Gear Hash (64KB avg chunks)
    ↓
Indexing: Lookup chunks in Universe Map
    ↓
Filtering: Identify chunks not present at destination
    ↓
Transfer: Send only unique chunks
```

### Behavior

**Chunking**: Uses Gear Hash CDC with a target size of 64KB:
- Minimum chunk size: 8KB
- Average chunk size: 64KB
- Maximum chunk size: 256KB

**Deduplication**: Full Universe Map lookup ensures that moving a folder of 10,000 PDFs to a new location results in **zero data transfer** (100% deduplication).

**Concurrency**: Automatically scales based on CPU cores (`std::thread::available_parallelism`).

### Resource Usage

This mode represents a balanced trade-off:

- **Memory**: Moderate. Uses ~1KB of RAM per 64KB of data processed during indexing
- **CPU**: Moderate. Uses BLAKE3 hashing on blocking threads to ensure UI/Heartbeat responsiveness
- **Network**: Minimal for repeated content. Only unique chunks are transferred

### Performance Characteristics

**Deduplication Efficiency**: By using 64KB chunks, Equilibrium maximizes deduplication rates for:
- Source code where functions often move between files
- Documents with repeated sections
- Media libraries with similar content
- VM images with common OS files

**Stability**: The `offload_compute` "Air Gap" pattern ensures that hashing a 500MB ISO doesn't freeze the async reactor or web dashboard.

### Usage

No special flags are required. This is the **default behavior** for files 8KB to 1GB.

```bash
# Standard sync (uses Equilibrium automatically for medium files)
orbit sync /source /destination

# With compression
orbit sync --compress /source /destination

# Tune connection pool for unstable networks
orbit sync --idle-timeout 300 /source /dest
```

### When to Use Equilibrium

✅ **Best For:**
- Source code repositories (dedups moved/refactored code)
- PDF documents and office files
- Virtual machine images (dedups OS commonality)
- Media files (photos, music) with duplicates
- Database backups with repeated blocks

✅ **Why It's the Default:**
- Handles 90% of typical file transfer workloads
- Provides significant bandwidth savings (often 30-70% deduplication)
- CPU overhead is acceptable for network-bound transfers
- Memory usage is predictable and bounded

### Deduplication Examples

**Example 1: Repository Refactoring**
```bash
# Before: 1GB repository
# After refactoring: 1GB repository (files moved around)
# Traditional rsync: Transfers ~500MB (50% changed)
# Orbit Equilibrium: Transfers ~0MB (chunks unchanged)
```

**Example 2: Document Versioning**
```bash
# Annual report v1: 50MB PDF
# Annual report v2: 52MB PDF (minor edits)
# Traditional transfer: 52MB
# Orbit Equilibrium: ~2MB (only changed chunks)
```

### Configuration

While Equilibrium requires no special configuration, you can tune related settings:

```bash
# Adjust concurrency (default: auto-detected CPU count)
orbit sync --concurrency 8 /source /dest

# Adjust connection pool idle timeout (default: 300s)
orbit sync --idle-timeout 600 /source /dest

# Enable compression for text-heavy content
orbit sync --compress /source /dest
```

### Technical Details

**CDC Implementation**:
- Algorithm: Gear Hash rolling hash with threshold-based cut detection
- Implementation: `orbit-core-cdc` crate
- Hash function: BLAKE3 (faster than SHA-256, more secure than BLAKE2)

**Universe Map**:
- Storage: redb embedded database (ACID-compliant)
- Index structure: Content-hash → Vec<Location>
- Query performance: O(1) chunk existence lookup
- Implementation: `orbit-core-starmap::universe` module

**Air Gap Pattern**:
- CPU-intensive hashing runs on `tokio::task::spawn_blocking` threads
- Prevents async reactor starvation
- Maintains responsiveness for web dashboard and heartbeats

### Comparing the Lanes

| Feature | Neutrino (<8KB) | Equilibrium (8KB-1GB) | Gigantor (>1GB) |
|---------|-----------------|----------------------|-----------------|
| **Chunking** | None (direct copy) | CDC 64KB avg | Tiered CDC (1-4MB avg) |
| **Deduplication** | None | Full (Universe Map) | Full (Universe Map) |
| **Concurrency** | Very High (200+) | Auto (CPU count) | Parallel Hashing |
| **Best For** | Config files, logs | Code, docs, media | VM images, videos, databases |
| **CPU Usage** | Very Low | Moderate | High (multi-core) |
| **Network Savings** | 0% (no dedup) | 30-70% typical | 20-60% typical |

For more architectural details, see [ORBIT_V2_ARCHITECTURE.md](../architecture/ORBIT_V2_ARCHITECTURE.md).

## The "Gigantor" Heavy Lift Lane (v2.3+)

The **Gigantor** profile is designed for Big Data: files exceeding 1GB, scaling up to terabytes. In this domain, the physics of data movement fundamentally change. Traditional CDC approaches face three critical challenges that Gigantor solves.

### The Problem: Why Large Files Break Standard CDC

When dealing with files larger than 1GB (and scaling to TBs), three specific problems emerge:

1. **Index Explosion**: Storing a 64KB chunk index for a 10TB file results in ~160 million database entries. This overwhelms the Universe Map, causing:
   - Slow lookups (even with indexing)
   - Massive memory consumption
   - Database bloat (gigabytes of metadata for the index alone)

2. **Throughput Bottlenecks**: A single thread calculating BLAKE3 hashes cannot saturate a 10Gbps+ link or modern NVMe drives:
   - Single-threaded BLAKE3: ~500 MB/s
   - Modern NVMe sequential read: 5-7 GB/s
   - The CPU becomes the bottleneck, not the disk

3. **Connection Stability**: Transfers lasting hours are susceptible to:
   - Idle timeouts (connection pools reclaiming "stale" connections)
   - Max lifetime limits (connections killed mid-transfer)
   - Network hiccups causing full restart

### The Solution: Tiered Intelligence + Parallel Velocity

Gigantor addresses these challenges with three key innovations:

#### 1. Tiered Chunking (The Resolution Shift)

Instead of fixed 64KB chunks, Gigantor dynamically adjusts chunk size based on file size:

| File Size | Chunk Size | Index Reduction |
|-----------|------------|-----------------|
| 1GB - 100GB | 1MB avg (256KB min, 4MB max) | 16x fewer entries |
| > 100GB | 4MB avg (1MB min, 16MB max) | 64x fewer entries |

**Example**: A 10TB file with Gigantor produces ~2.5 million chunks instead of 160 million.

#### 2. Pipelined Parallel Hashing (The Scan-Dispatch-Hash Pattern)

Traditional CDC is sequential: read → find boundaries → hash → check index. Gigantor decouples this:

```
┌──────────────────┐  Sequential (I/O + Gear Hash)
│  Scanner Thread  │  Finds CDC boundaries using Gear rolling hash
└────────┬─────────┘  Does NOT compute BLAKE3
         │ Raw chunks (data + offsets)
         ▼
┌─────────────────┐  Async Orchestration
│  Tokio Channel  │  Batches chunks for parallel processing
└────────┬────────┘
         │ Batches of 64 chunks
         ▼
┌─────────────────┐  Parallel (Rayon Pool)
│  Hash Workers   │  BLAKE3 hashing across all CPU cores
│  (Rayon)        │  Saturates multi-core systems
└────────┬────────┘
         │ Hashed chunks
         ▼
┌─────────────────┐  Network/DB Bound
│  Dedup & Xfer   │  Universe lookup + transfer
└─────────────────┘
```

**Performance Impact**:
- Single-threaded: ~500 MB/s (CPU bound)
- Gigantor (8 cores): ~4 GB/s (disk bound)
- Gigantor (16 cores): ~7 GB/s (saturates NVMe)

#### 3. Long-Haul Connection Profile

Gigantor uses a specialized connection pool configuration:

- **Max Connections**: 4 (prevents bandwidth saturation)
- **Max Lifetime**: 24 hours (supports multi-hour S3 multipart uploads)
- **Acquire Timeout**: 10 minutes (waiting for heavy lane is expected)
- **Min Idle**: 1 (connections stay busy for hours)

### Behavior

This mode is **automatic**. When Orbit detects a file > 1GB, it routes to Gigantor.

**Tiered Chunking**:
- Files 1GB - 100GB: 1MB average chunks (256KB min, 4MB max)
- Files > 100GB: 4MB average chunks (1MB min, 16MB max)

**Parallel Hashing**:
- Scanner thread: Sequential I/O + Gear hash (finds boundaries)
- Hash workers: Parallel BLAKE3 across all cores (via Rayon)
- Orchestrator: Batches of 64 chunks between stages

**Long-Haul Transfer**:
- Connections reserved for extended durations
- Aggressive keep-alives
- Suitable for multi-hour S3 multipart upload sessions

### Configuration

This behavior is automatic. To force specific behavior for testing:

```bash
# Force specific chunk size (e.g., 4MB)
orbit sync --chunk-size 4MB /large_dataset s3://bucket

# Monitor Gigantor activation
orbit sync --verbose /large_dataset s3://bucket
# Look for: "Using Gigantor Heavy Lift Lane (file size: 5.2 GB)"
```

### Performance Characteristics

**Index Efficiency**: By dynamically increasing chunk size, we prevent the Universe Map from becoming the bottleneck. A 10TB transfer generates a manageable ~2.5M entry index, not a 160-million-row monstrosity.

**Hardware Saturation**: The decoupling of Scanning (Sequential) and Hashing (Parallel) allows Orbit to utilize 100% of available cores, saturating even the fastest NVMe drives.

**Stability**: The `long_haul_profile` prevents "pool thrashing," where connections are constantly opened and closed, ensuring steady throughput for massive objects.

### When to Use Gigantor

✅ **Automatically Activated For:**
- Virtual machine images (VMDK, VHD, QCOW2)
- Database dumps (PostgreSQL, MySQL backups)
- Video files (raw footage, master copies)
- Large compressed archives (multi-GB tarballs)
- Scientific datasets (genomics, satellite imagery)

✅ **Benefits:**
- Reduced metadata overhead (up to 64x fewer index entries)
- Multi-core CPU utilization (up to 10x hashing throughput)
- Stable multi-hour transfers (no timeout issues)

❌ **Not Recommended For:**
- Collections of small files (use Neutrino)
- Files under 1GB (overhead not justified)

### Technical Details

**Requirements:**
- Requires `tokio` async runtime
- Uses `rayon` for parallel hashing
- Compatible with all Universe backends

**Deduplication Effectiveness:**
While Gigantor uses larger chunks, it still provides significant deduplication for:
- VM images with common OS blocks (20-40% savings)
- Database dumps with repeated schemas (30-50% savings)
- Video files with redundant frames (10-30% savings)

**Architecture:**
- **Router**: `PipelineRouter::select_strategy()` detects files > 1GB
- **Chunk Config**: `PipelineRouter::optimal_chunk_config()` selects 1MB or 4MB chunks
- **Executor**: `GigantorExecutor` implements scan-dispatch-hash pipeline
- **Pool**: `PoolConfig::long_haul_profile()` for extended connections

### Example: Processing a 50GB Database Dump

```bash
orbit sync postgres_backup_50gb.sql s3://backups/

# Gigantor automatically activates:
# ✓ Using Gigantor Heavy Lift Lane (file size: 50.0 GB)
# ✓ Chunk config: 1MB avg (256KB min, 4MB max)
# ✓ Expected chunks: ~50,000 (vs 800,000 with standard CDC)
# ✓ Parallel hashing: 8 cores engaged
# ✓ Connection pool: Long-haul profile (24h lifetime)
#
# Result:
# - Chunks: 51,234
# - Transferred: 31.2 GB (38% deduplication)
# - Duration: 4m 23s
# - Throughput: 121 MB/s (network limited)
```

### Debugging Gigantor

If you suspect Gigantor isn't activating:

```bash
# Check file size
ls -lh large_file.dat

# Force verbose output
orbit sync --verbose large_file.dat s3://bucket/

# Look for these log lines:
# "PipelineRouter: File 5368709120 bytes → DeduplicatedTiered strategy"
# "GigantorExecutor: Using 1MB chunks (256KB-4MB range)"
# "GigantorExecutor: Parallel hasher pool initialized with 8 workers"
```

### Summary of Benefits

| Benefit | Standard CDC (64KB) | Gigantor (1-4MB) |
|---------|---------------------|------------------|
| **10TB File Index** | 160 million entries | 2.5 million entries (64x reduction) |
| **Hashing Throughput** | ~500 MB/s (1 core) | ~4-7 GB/s (all cores) |
| **Connection Stability** | 30-min lifetime | 24-hour lifetime |
| **Memory Usage** | ~10 GB metadata | ~160 MB metadata |
| **Database Size** | Bloats to gigabytes | Stays manageable |

Gigantor ensures that Orbit scales from kilobytes to terabytes without changing behavior or requiring manual tuning.
