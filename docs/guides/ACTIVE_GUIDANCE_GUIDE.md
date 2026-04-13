# Active Config Optimizer - User Guide

**Version**: 0.7.0
**Module**: `src/core/guidance.rs`, `src/core/probe.rs`
**Status**: Beta

## Overview

The Active Config Optimizer (Phase 4 enhancement) transforms Orbit's configuration layer from a passive validator into an intelligent environment-aware optimizer. It actively probes your system hardware and destination to automatically tune settings for optimal performance.

## What's New in v0.7.0

### Before (v0.6.0 and earlier):
```
✅ Validated configuration conflicts
✅ Prevented incompatible flag combinations
❌ No hardware awareness
❌ No destination type detection
❌ Manual optimization required
```

### After (v0.7.0):
```
✅ Validated configuration conflicts
✅ Prevented incompatible flag combinations
✅ Detects CPU, RAM, I/O throughput
✅ Identifies destination type (SMB, S3, Local, etc.)
✅ Auto-tunes settings in real-time
```

## How It Works

Every time you run a transfer, Orbit now:

1. **Probes your system** (CPU cores, RAM, I/O speed)
2. **Detects destination type** (Local, SMB, NFS, S3, Azure, GCS)
3. **Analyzes your configuration** (existing Guidance rules)
4. **Auto-tunes settings** (new Active Rules)
5. **Explains all changes** (transparent notices)

## System Profiling

### What Gets Detected

```rust
pub struct SystemProfile {
    pub logical_cores: usize,        // Number of CPU cores
    pub available_ram_gb: u64,        // Available RAM in GB
    pub total_memory_gb: u64,         // Total system memory
    pub is_battery_power: bool,       // Future: battery detection
    pub dest_filesystem_type: FileSystemType,  // Destination type
    pub estimated_io_throughput: f64, // I/O speed in MB/s
}
```

### Filesystem Type Detection

The system automatically detects:

| Type | Detection Method | Example Paths |
|------|------------------|---------------|
| **Local** | Default for local paths | `/data`, `C:\backup` |
| **SMB** | URI or UNC path | `smb://server/share`, `\\server\share` |
| **NFS** | URI prefix | `nfs://server/export` |
| **S3** | URI prefix | `s3://bucket/key` |
| **Azure** | URI prefix | `azure://container/blob`, `azblob://...` |
| **GCS** | URI prefix | `gs://bucket/object`, `gcs://...` |

### I/O Throughput Benchmarking

For every transfer, Orbit runs a quick 10MB write test to estimate I/O speed:

```
Detected I/O throughput: ~450 MB/s → Fast local SSD
Detected I/O throughput: ~45 MB/s  → Slow HDD or network
```

This measurement informs compression decisions.

## Active Auto-Tuning Rules

### Rule 1: Network Share Auto-Tuning

**Triggers when:** Destination is SMB or NFS

**Actions:**
- ✅ Enables resume capability (network reliability)
- ✅ Increases retry attempts to 5 (minimum)
- ⚙️ May adjust timeout settings

**Example:**
```
┌── 🛰️  Orbit Config Optimizer ───────────────────────┐
│ 🔧 Network: Detected SMB destination. Enabling resume for reliability.
│ 🔧 Network: Increased retry attempts to 5 for network filesystem.
└────────────────────────────────────────────────────┘
```

**When you see this:**
- Transferring to `smb://fileserver/share`
- Copying to `\\server\share` (Windows UNC)
- Working with NFS mounts

### Rule 2: CPU-Rich / IO-Poor Optimization

**Triggers when:**
- System has ≥8 CPU cores AND
- I/O throughput is <50 MB/s AND
- Compression is disabled

**Action:**
- ✅ Enables Zstd:3 compression (trade CPU for I/O)

**Example:**
```
┌── 🛰️  Orbit Config Optimizer ───────────────────────┐
│ 🔧 Performance: Detected slow I/O (42.3 MB/s) with 16 cores.
│                Enabling Zstd:3 to trade CPU for throughput.
└────────────────────────────────────────────────────┘
```

**When you see this:**
- High-end CPU with slow HDD
- Network destination with fast local CPU
- Server with many cores but slow storage

**Why it helps:**
- Compression reduces I/O (the bottleneck)
- You have spare CPU capacity to compress
- Net result: Faster transfers

### Rule 3: Low Memory Protection

**Triggers when:**
- Available RAM <1 GB AND
- Parallel operations >4

**Action:**
- ⚙️ Reduces parallel workers to 2

**Example:**
```
┌── 🛰️  Orbit Config Optimizer ───────────────────────┐
│ 🔧 Memory: Low available memory (0 GB). Reduced parallel
│            operations from 8 to 2.
└────────────────────────────────────────────────────┘
```

**When you see this:**
- Running on constrained systems
- Many other programs using RAM
- Working in containers with memory limits

### Rule 4: Cloud Storage Optimization

**Triggers when:** Destination is S3, Azure Blob, or Google Cloud Storage

**Actions:**
- ✅ Enables Zstd:3 compression (reduce transfer size/cost)
- ✅ Increases retry attempts to 10 (cloud reliability)
- ✅ Enables exponential backoff (API rate limiting)

**Example:**
```
┌── 🛰️  Orbit Config Optimizer ───────────────────────┐
│ 🔧 Cloud: Detected cloud storage destination.
│           Enabling compression to reduce network transfer.
│ 🔧 Cloud: Increased retry attempts to 10 for reliability.
│ 🔧 Cloud: Enabled exponential backoff for API rate limiting.
└────────────────────────────────────────────────────┘
```

**When you see this:**
- `s3://` destinations
- `azure://` or `azblob://` destinations
- `gs://` or `gcs://` destinations

**Why it helps:**
- Compression reduces cloud storage costs
- Higher retries handle transient API failures
- Exponential backoff prevents API throttling

## Examples

### Example 1: Local to SMB Transfer

```bash
$ orbit -s /data -d smb://fileserver/backup --recursive

Scanning system environment...
  8 CPU cores detected
  16 GB RAM available
  I/O throughput: ~120 MB/s

┌── 🛰️  Orbit Config Optimizer ───────────────────────┐
│ 🔧 Network: Detected SMB destination. Enabling resume.
│ 🔧 Network: Increased retry attempts to 5.
└────────────────────────────────────────────────────┘

Starting transfer...
```

**What happened:**
1. System detected 8 cores, 16 GB RAM, ~120 MB/s I/O
2. Destination analyzed: SMB network share
3. Auto-enabled resume for network reliability
4. Increased retries from default 3 to 5

### Example 2: Slow Disk with Fast CPU

```bash
$ orbit -s /source -d /external-hdd/backup --recursive

Scanning system environment...
  16 CPU cores detected
  32 GB RAM available
  I/O throughput: ~35 MB/s

┌── 🛰️  Orbit Config Optimizer ───────────────────────┐
│ 🔧 Performance: Detected slow I/O (35.4 MB/s) with 16 cores.
│                Enabling Zstd:3 compression to trade CPU for throughput.
└────────────────────────────────────────────────────┘

Starting transfer...
```

**What happened:**
1. 16 cores detected → plenty of CPU power
2. I/O benchmark shows slow disk (35 MB/s)
3. Auto-enabled compression to speed up transfer
4. CPU compresses data → less writing → faster overall

### Example 3: Cloud Upload

```bash
$ orbit -s backup.tar.gz -d s3://my-bucket/backups/

Scanning system environment...
  4 CPU cores detected
  8 GB RAM available
  I/O throughput: ~280 MB/s

┌── 🛰️  Orbit Config Optimizer ───────────────────────┐
│ 🔧 Cloud: Detected cloud storage destination.
│           Enabling compression to reduce network transfer.
│ 🔧 Cloud: Increased retry attempts to 10.
│ 🔧 Cloud: Enabled exponential backoff for API rate limiting.
└────────────────────────────────────────────────────┘

Starting upload to S3...
```

**What happened:**
1. S3 destination detected from `s3://` prefix
2. Compression enabled to save bandwidth/cost
3. Retries increased for cloud reliability
4. Exponential backoff enabled for API throttling

## Performance Impact

The Active Config Optimizer adds **minimal overhead**:

| Operation | Time | Impact |
|-----------|------|--------|
| **System probe** | ~50-100ms | One-time per transfer |
| **I/O benchmark** | ~200-300ms | One-time 10MB write |
| **Optimization rules** | <1ms | Negligible |
| **Total overhead** | ~250-400ms | 0.01% for 1GB+ files |

For large transfers (GB-scale), this overhead is negligible compared to actual transfer time.

## Interaction with Static Optimization

Active optimization works **alongside** the original optimization rules:

```
Original Optimization Rules (v0.6.0):
  ✓ Zero-copy vs Checksum conflicts
  ✓ Resume vs Compression safety
  ✓ Hardware capability checks
  (11 rules total)

New Active Optimization Rules (v0.7.0):
  ✓ Network destination tuning
  ✓ CPU/IO optimization
  ✓ Memory protection
  ✓ Cloud storage tuning
  (4 new rules)
```

**Example with both:**
```
┌── 🛰️  Orbit Config Optimizer ───────────────────────┐
│ 🚀 Strategy: Disabling zero-copy for checksum verification
│              (Original static optimization rule)
│ 🔧 Network: Detected SMB destination. Enabling resume.
│             (New active optimization rule)
│ 🔧 Cloud: Increased retry attempts to 10.
│           (New active optimization rule)
└────────────────────────────────────────────────────┘
```

## Disabling Active Optimization

If you prefer manual control, you can bypass active optimization:

**Currently:** Active probing runs by default for all operations. Future versions may add:

```bash
# Future feature (not yet implemented)
orbit -s /data -d /backup --no-probe --recursive
```

For now, active probing always runs but is very fast (<300ms) and transparent.

## Debugging

### Verbose Output

To see detailed probe results:

```bash
$ orbit -s /data -d /backup --recursive --verbose

[DEBUG] System probe starting...
[DEBUG] Detected 8 logical cores
[DEBUG] Available RAM: 16 GB
[DEBUG] Total RAM: 32 GB
[DEBUG] I/O benchmark: 10 MB in 45ms = 222 MB/s
[DEBUG] Filesystem type: Local
[DEBUG] Active Rule 1 (Network): Not triggered (local dest)
[DEBUG] Active Rule 2 (CPU/IO): Not triggered (I/O speed OK)
[DEBUG] Active Rule 3 (Memory): Not triggered (sufficient RAM)
[DEBUG] Active Rule 4 (Cloud): Not triggered (not cloud)
```

### Probe Failures

If system probing fails, Orbit falls back to safe defaults:

```
[WARN] Failed to probe environment: Permission denied
[INFO] Using safe default configuration
```

The transfer continues, just without auto-tuning.

## Comparison: Before vs After

### Before v0.7.0 (Manual Optimization)

```bash
# User had to know and specify everything
$ orbit -s /data -d smb://server/share --recursive \
    --resume \                    # Had to remember this
    --retry-attempts 10 \         # Had to set manually
    --exponential-backoff \       # Had to enable
    --compress zstd:3             # Had to choose
```

### After v0.7.0 (Auto-Tuning)

```bash
# Just specify source and destination
$ orbit -s /data -d smb://server/share --recursive

# Orbit automatically:
# ✅ Detects SMB destination
# ✅ Enables resume
# ✅ Increases retries
# ✅ Enables backoff
# ✅ (May enable compression if CPU/IO suggests it)
```

## Integration with Init Wizard

The Init Wizard (`orbit init`) and the Active Config Optimizer work together perfectly:

1. **Init Wizard** → Creates your **baseline configuration**
2. **Active Guidance** → Provides **runtime optimization** per transfer

Example workflow:
```bash
# 1. Create baseline config (one time)
$ orbit init
? What is your primary use case?
  > Backup (Reliability First)
✅ Configuration saved

# 2. Every transfer gets baseline + active tuning
$ orbit -s /data -d smb://server/share --recursive

# Uses baseline (from init):
#   - Checksum verification
#   - Resume enabled
#   - 5 retry attempts

# Plus active tuning:
#   🔧 Network: Detected SMB, already has resume ✓
#   🔧 Network: Retry attempts increased from 5 to 10
```

## Best Practices

### 1. Trust the Auto-Tuning

The Active Config Optimizer is designed to make optimal choices. Unless you have specific requirements, let it work:

```bash
# ✅ Good: Let the optimizer tune settings
orbit -s /data -d /backup --recursive

# ❌ Over-specified: May conflict with optimizer
orbit -s /data -d /backup --recursive \
  --resume \
  --no-resume \  # Contradictory!
  --parallel 32  # May exceed optimizer limits
```

### 2. Review Optimizer Messages

Always read the optimizer output to understand what's happening:

```
┌── 🛰️  Orbit Config Optimizer ───────────────────────┐
│ 🔧 Performance: Slow I/O detected. Enabling compression.
└────────────────────────────────────────────────────┘
```

This teaches you about your system and optimal settings.

### 3. Use Init Wizard for Baseline

Start with `orbit init` to set up your baseline, then let the Active Config Optimizer handle per-transfer optimizations.

## FAQ

### Q: Does Active Config Optimization slow down transfers?

**A:** No. The probe adds ~250-400ms one-time overhead, negligible for any real transfer. The optimizations it enables (compression, proper retries) often make transfers **faster** overall.

### Q: What if I disagree with an optimization decision?

**A:** You can manually override by explicitly setting flags:

```bash
# Force disable compression even if the optimizer suggests it
orbit -s /data -d /backup --compress none --recursive
```

Explicit flags always take precedence.

### Q: Can I see what would be changed without running the transfer?

**A:** Not yet, but this is planned:

```bash
# Future feature
orbit -s /data -d /backup --guidance-only
```

## Technical Details

### Probe Implementation

Location: `src/core/probe.rs`

```rust
pub struct Probe;

impl Probe {
    pub fn scan(dest_path: &Path) -> Result<SystemProfile> {
        // 1. Detect CPU cores via sysinfo
        // 2. Measure available RAM
        // 3. Benchmark I/O (10MB write test)
        // 4. Analyze destination path for filesystem type
        // Returns SystemProfile
    }
}
```

### ConfigOptimizer Integration

Location: `src/core/guidance.rs`

```rust
impl ConfigOptimizer {
    pub fn optimize_with_probe(
        config: CopyConfig,
        dest_path: Option<&Path>
    ) -> Result<OptimizedConfig> {
        // 1. Run system probe if path provided
        // 2. Apply static optimization rules
        // 3. Apply active optimization rules
        // Returns OptimizedConfig with notices
    }
}
```

## Related Documentation

- **Init Wizard:** [`docs/guides/INIT_WIZARD_GUIDE.md`](INIT_WIZARD_GUIDE.md)
- **Config Optimizer Architecture:** [`docs/architecture/GUIDANCE_SYSTEM.md`](../architecture/GUIDANCE_SYSTEM.md)
- **Implementation:** `src/core/probe.rs`, `src/core/guidance.rs`

## Changelog

### v0.7.0 (Current)
- ✅ Active system probing (CPU, RAM, I/O)
- ✅ Filesystem type detection
- ✅ 4 auto-tuning rules
- ✅ Integration with init wizard

### v0.6.0 and earlier
- ✅ Static optimization rules only
- ❌ No environment awareness
- ❌ Manual optimization required

## Feedback

The Active Config Optimizer is new in v0.7.0. Please report:
- Incorrect auto-tuning decisions
- Performance issues from probing
- Missing optimizations you'd like to see

Report at: https://github.com/saworbit/orbit/issues
