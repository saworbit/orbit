# Orbit Init Wizard - User Guide

**Version**: 0.7.0
**Command**: `orbit init`
**Status**: Beta

## Overview

The `orbit init` command is an interactive setup wizard that helps you create an optimal Orbit configuration in seconds. It eliminates the need to manually write configuration files by:

1. **Probing your system** — Automatically detects CPU cores, RAM, and I/O throughput
2. **Understanding your needs** — Asks about your primary use case
3. **Generating optimal settings** — Creates a configuration tuned for your specific scenario
4. **Setting up security** — Optionally generates secure JWT secrets for the Web Dashboard

## Quick Start

```bash
# Run the setup wizard
orbit init

# The wizard will guide you through the process interactively
```

## What the Wizard Does

### Step 1: System Environment Scan

The wizard automatically probes your system to gather performance metrics:

```
Scanning system environment...
  16 CPU cores detected
  32 GB RAM available
  I/O throughput: ~450 MB/s
```

**What it detects:**
- **CPU Cores**: Used to determine optimal parallelism
- **Available RAM**: Helps set memory-safe defaults
- **I/O Throughput**: Informs compression and buffering decisions
- **Filesystem Type**: Detects local, network, or cloud destinations

### Step 2: Use Case Selection

Choose from four pre-optimized profiles:

```
? What is your primary use case?
  > Backup (Reliability First)
    Sync (Speed First)
    Cloud Upload (Compression First)
    Network Transfer (Resume + Compression)
```

**Profile Descriptions:**

#### 1. Backup (Reliability First)
**Best for**: Critical data backups, archival storage

**Settings:**
- ✅ Resume enabled (survive interruptions)
- ✅ Checksum verification (ensure integrity)
- ✅ Preserve all metadata (times, permissions, ownership)
- ✅ 5 retry attempts with exponential backoff
- ⚙️ Copy mode (never delete from source)

**When to use:**
- Backing up important documents
- Creating disaster recovery copies
- Archiving project data
- Compliance-driven backups

#### 2. Sync (Speed First)
**Best for**: Fast synchronization, development workflows

**Settings:**
- ⚡ Zero-copy enabled (maximum performance)
- ⚙️ Sync mode (mirror source to destination)
- 🔄 Trust modification time (skip checksum for speed)
- 🚀 Auto-detect parallel workers
- ✅ Preserve metadata

**When to use:**
- Syncing code repositories
- Mirroring directories
- Development environment syncs
- Performance-critical transfers

#### 3. Cloud Upload (Compression First)
**Best for**: Uploading to S3, Azure, or GCS

**Settings:**
- 🗜️ Zstd:3 compression (reduce transfer size)
- ✅ Checksum verification
- ✅ 10 retry attempts (cloud reliability)
- ✅ Exponential backoff (API rate limiting)
- ✅ Resume enabled
- ❌ Zero-copy disabled (compression requires userspace)

**When to use:**
- Uploading backups to cloud storage
- Data archival to S3/Azure/GCS
- Minimizing bandwidth costs
- Working with slow/metered connections

#### 4. Network Transfer (Resume + Compression)
**Best for**: SMB/NFS network shares

**Settings:**
- 🗜️ Zstd:3 compression (optimize network usage)
- ✅ Checksum verification
- ✅ Resume enabled (network reliability)
- ✅ 10 retry attempts
- ✅ Exponential backoff
- ⚙️ 4 parallel transfers

**When to use:**
- Copying to/from SMB/CIFS shares
- NFS network filesystems
- Remote file servers
- VPN or WAN transfers

### Step 3: Security Configuration

Optionally generate a secure JWT secret for the Orbit Web Dashboard:

```
🔐 Security Configuration
? Generate secure JWT Secret for Web Dashboard? (Y/n)

  ✓ Generated JWT Secret:
  3K8mNpQr9vXz2WbE7CfGhJkL1MnOpRsT

  Add this to your environment:
  export ORBIT_JWT_SECRET=3K8mNpQr9vXz2WbE7CfGhJkL1MnOpRsT

  (This will NOT be saved to the config file for security)
```

**Important security notes:**
- The JWT secret is **never saved** to the configuration file
- You must set it as an environment variable: `ORBIT_JWT_SECRET`
- Keep this secret secure — it protects your Web Dashboard
- Regenerate if compromised by running `orbit init` again

### Step 4: Configuration Saved

```
╔════════════════════════════════════════╗
║    ✅ Configuration Saved              ║
╚════════════════════════════════════════╝

  Location: /home/user/.orbit/orbit.toml

  Configuration Summary:
  ─────────────────────────
  Copy Mode:        Copy
  Compression:      Zstd { level: 3 }
  Checksum Verify:  true
  Resume:           true
  Parallel:         0
  Retry Attempts:   5

  Next Steps:
  1. Review the configuration: cat /home/user/.orbit/orbit.toml
  2. Set ORBIT_JWT_SECRET environment variable (if you generated one)
  3. Run 'orbit --help' to see available commands
```

## Environment-Based Optimizations

The wizard doesn't just use the selected profile — it also applies intelligent optimizations based on your detected system:

### CPU-Based Adjustments

**High CPU Count (≥8 cores) + Slow I/O (<100 MB/s):**
```
💡 Detected slow I/O with abundant CPU
   → Enabling LZ4 compression (trade spare CPU for throughput)
```

### Memory-Based Adjustments

**Low Memory (<2 GB):**
```
⚙️ Low memory detected
   → Setting chunk size to 512 KB (was 1 MB)
```

**High Memory (≥8 GB):**
```
⚙️ High memory available
   → Setting chunk size to 4 MB for better performance
```

### Parallelism Auto-Tuning

If you select `parallel = 0` (auto-detect):
```
⚙️ Auto-tuning parallelism
   → Setting to 8 workers (half of 16 cores)
```

## Configuration File Location

The wizard saves your configuration to:

**Linux/macOS:**
```
~/.orbit/orbit.toml
```

**Windows:**
```
C:\Users\<username>\.orbit\orbit.toml
```

## Overwriting Existing Configuration

If you already have a configuration file, the wizard will ask before overwriting:

```
Existing configuration found. Overwrite? (y/N) n

Configuration unchanged.
```

## Using the Generated Configuration

After running `orbit init`, all Orbit commands automatically use your configuration:

```bash
# No flags needed — uses your optimized config
orbit -s /data -d /backup --recursive

# Override specific settings if needed
orbit -s /data -d /backup --recursive --parallel 16
```

## Manual Configuration

You can still manually edit `~/.orbit/orbit.toml` after running the wizard:

```toml
# ~/.orbit/orbit.toml
copy_mode = "copy"
compression = { type = "zstd", level = 3 }
verify_checksum = true
resume_enabled = true
retry_attempts = 5
exponential_backoff = true
preserve_metadata = true

# Edit these values as needed
parallel = 8
chunk_size = 1048576  # 1 MB
```

## Advanced: Non-Interactive Mode

For automation and scripting, you can skip the wizard and create configurations directly:

```bash
# Create a default configuration
cat > ~/.orbit/orbit.toml << 'EOF'
copy_mode = "copy"
compression = "none"
verify_checksum = false
resume_enabled = false
parallel = 4
EOF
```

## Troubleshooting

### "Could not determine home directory"

**Problem:** The wizard can't find your home directory.

**Solution:**
```bash
# Set HOME explicitly
export HOME=/home/yourusername
orbit init
```

### "Failed to save configuration: Permission denied"

**Problem:** Can't write to `~/.orbit/` directory.

**Solution:**
```bash
# Create directory with correct permissions
mkdir -p ~/.orbit
chmod 755 ~/.orbit
orbit init
```

### "System probe failed"

**Problem:** Cannot detect system metrics.

**Solution:**
The wizard will continue with safe defaults. The probe failure is logged but doesn't stop configuration generation.

### JWT Secret Not Working

**Problem:** Dashboard says "Invalid token" even with generated secret.

**Solution:**
```bash
# Ensure the environment variable is set
export ORBIT_JWT_SECRET=<your-secret-here>

# Verify it's set
echo $ORBIT_JWT_SECRET

# Restart any running Orbit services
```

## Comparison: Manual vs Init Wizard

| Aspect | Manual Config | Init Wizard |
|--------|--------------|-------------|
| **Time to setup** | 10-15 minutes | 30 seconds |
| **Optimization** | Manual tuning | Auto-tuned |
| **Errors** | Typos, syntax errors | Validated |
| **System awareness** | None | CPU, RAM, I/O detected |
| **Security** | Manual secret generation | Auto-generated |
| **Learning curve** | Steep | Minimal |
| **Flexibility** | Full control | Profile-based + editable |

## Examples

### Example 1: First-Time User

```bash
$ orbit init

🪐 Welcome to Orbit Setup

Scanning system environment...
  8 CPU cores detected
  16 GB RAM available
  I/O throughput: ~250 MB/s

? What is your primary use case?
  > Backup (Reliability First)

? Generate secure JWT Secret for Web Dashboard? Yes

✅ Configuration saved to: /home/alice/.orbit/orbit.toml

# Now ready to use
$ orbit -s ~/documents -d /backup/documents --recursive
```

### Example 2: Cloud Storage User

```bash
$ orbit init

? What is your primary use case?
    Cloud Upload (Compression First)

✅ Configuration saved

# Upload with optimized settings
$ orbit -s dataset.tar.gz -d s3://my-bucket/backups/
# Will automatically use compression and cloud-optimized retries
```

### Example 3: Network Share User

```bash
$ orbit init

? What is your primary use case?
    Network Transfer (Resume + Compression)

✅ Configuration saved

# Copy to SMB share
$ orbit -s /local/data -d smb://fileserver/share/backup --recursive
# Automatic: resume, compression, network-optimized retries
```

## Integration with Active Guidance

The init wizard works seamlessly with Orbit's Active Guidance System (Phase 4). Even after initial setup, every transfer benefits from real-time environment detection:

```bash
$ orbit -s /data -d smb://server/share --recursive

┌── 🛰️  Orbit Guidance System ───────────────────────┐
│ 🔧 Network: Detected SMB destination. Increasing retries to 10.
│ 🔧 Performance: Slow I/O detected. Compression already enabled.
└────────────────────────────────────────────────────┘
```

The wizard creates your **baseline config**, while Active Guidance provides **runtime optimization**.

## Related Documentation

- **Active Guidance:** [`docs/guides/ACTIVE_GUIDANCE_GUIDE.md`](ACTIVE_GUIDANCE_GUIDE.md)
- **Guidance System:** [`docs/architecture/GUIDANCE_SYSTEM.md`](../architecture/GUIDANCE_SYSTEM.md)
- **Configuration Reference:** [`docs/guides/quickstart_guide.md`](quickstart_guide.md)
- **System Probing:** `src/core/probe.rs`

## Feedback

The init wizard is new in v0.7.0 and we're actively improving it. Please report:
- Confusing prompts or unclear options
- System probe failures or inaccurate detection
- Profile recommendations that don't match your use case

Report issues at: https://github.com/saworbit/orbit/issues
