# Orbit - Quick Start Guide

Get up and running with Orbit in 5 minutes!

---

## 📦 Installation

```bash
# Clone the repository
git clone https://github.com/saworbit/orbit.git
cd orbit

# Build and install
cargo build --release
cargo install --path .

# Verify installation
orbit --version
```

---

## 🔧 First-Time Setup (Recommended)

```bash
# Run the interactive setup wizard
orbit init

# Or check your system and config health
orbit doctor
```

`orbit init` scans your hardware, asks about your use case, sets up exclusion patterns, offers shell completion installation, and generates an optimized `~/.orbit/orbit.toml`. All subsequent commands use these settings automatically.

---

## 🚀 Basic Usage

### Copy a Single File
```bash
orbit input.txt output.txt
```

### Copy with Compression
```bash
# Auto-detect best compression for your destination
orbit large.dat backup.dat --compress auto

# Quick Zstd shorthand (level 3, balanced)
orbit large.dat backup.dat --zstd

# Quick LZ4 shorthand (fastest)
orbit large.dat backup.dat --lz4

# Explicit level control
orbit large.dat backup.dat --compress zstd:19
```

### Copy a Directory
```bash
# Auto-detects directories and enables recursive mode
orbit ./source_dir ./backup_dir
```

### Shorthand Subcommands
```bash
# Copy (alias for bare orbit — reads naturally in scripts)
orbit cp /data /backup

# Sync (only copy new/changed files)
orbit sync /project /backup

# Backup (checksums + Zstd + resume + metadata)
orbit backup /data /backup

# Mirror (exact replica, deletes extras at destination)
orbit mirror /source /replica
```

All global flags work with shorthands:
```bash
orbit sync /data /backup --quiet --zstd --workers 8
orbit backup /data s3://bucket/backup --retry-attempts 10
```

### Preview Before Running (Explain)
```bash
# See exactly what Orbit would do, in plain English — no files touched
orbit explain /data /backup -R --zstd --resume
```

This shows your transfer plan: mode, compression, checksums, parallelism, filters, and more.

### View Transfer History
```bash
# Show recent transfers from the audit log
orbit history

# Show last 50 entries in JSON format
orbit history --limit 50 --json

# Use a specific audit log file
orbit history --audit-file /var/log/orbit/audit.jsonl
```

### Resume Interrupted Transfer
```bash
orbit bigfile.iso /mnt/network/bigfile.iso --resume
# If interrupted, just run the same command again!
```

---

## 💡 Common Scenarios

### 1. Network Backup (Slow/Unreliable Connection)
```bash
# Orbit auto-detects remote destinations and enables resume + retries!
orbit backup ./important_data smb://server/backup --max-bandwidth 5

# Or with explicit control:
orbit ./important_data /mnt/backup -R \
  --compress zstd:9 \
  --resume \
  --retry-attempts 10 \
  --exponential-backoff \
  --max-bandwidth 5
```

**What this does:**
- `orbit backup` automatically enables checksums, Zstd compression, resume, and metadata preservation
- Auto-network detection adds retries and backoff for remote destinations
- Limits bandwidth to 5 MB/s

### 2. Fast Local Sync (Many Small Files)
```bash
orbit sync ./project /backup/project \
  --workers 8 \
  --exclude "node_modules/*" \
  --exclude "*.tmp"
```

**What this does:**
- `orbit sync` sets sync mode and enables recursive + metadata preservation
- Uses 8 parallel threads
- Excludes node_modules and temp files

### 3. Large File Transfer with Verification
```bash
orbit database_dump.sql /backup/database_dump.sql --zstd --resume
```

**What this does:**
- Compresses with Zstd (level 3)
- Verifies with BLAKE3 checksum (enabled by default)
- Preserves timestamps and permissions (enabled by default)
- Can resume if interrupted

### 4. Preview Before Copying (Dry Run)
```bash
orbit ./source ./dest -R --dry-run --verbose
```

**What this does:**
- Shows what would be copied without actually copying
- Displays detailed operation log ([DRY-RUN] messages)
- Shows summary statistics (files to copy/skip, total size)
- Useful for testing exclude patterns and transformations

**Example Output:**
```
[DRY-RUN] Would copy: /source/file1.txt -> /dest/file1.txt (1024 bytes) - new file
[DRY-RUN] Would skip: /source/file2.txt - already exists
[DRY-RUN] Would create directory: /dest/subdir

Dry-Run Summary:
  Files to copy:    5
  Files to skip:    2
  Total data size:  10.5 MB

No changes were made (dry-run mode).
```

### 5. Bandwidth-Limited Transfer with Progress
```bash
orbit -s /large/dataset -d /backup \
  -R \
  --max-bandwidth 10 \
  --parallel 4 \
  --show-progress \
  --verbose
```

**What this does:**
- Limits bandwidth to 10 MB/s (prevents network saturation)
- Uses 4 concurrent transfers (auto-detects optimal with `--parallel 0`)
- Shows real-time progress bars with ETA and transfer speed
- Detailed logging with `--verbose`

**Example Output:**
```
📁 Transferring: /large/dataset/file1.dat
   [████████████████████████████░░░░] 75.2%  45.2 MB/s  ETA: 5s
✓ Complete - 500.00 MB in 11.05s (45.25 MB/s)
```

---

## ⚙️ Configuration File

Run `orbit init` to generate one automatically, or create `~/.orbit/orbit.toml` manually:

```toml
# Compression
compression = { zstd = { level = 3 } }

# Chunk size in bytes
chunk_size = 2048

# Retry attempts
retry_attempts = 5

# Preserve metadata [default: true]
preserve_metadata = true

# Show execution statistics [default: true]
show_stats = true

# Human-readable output [default: true]
human_readable = true

# JSON Lines output (suppresses human output, stats, and progress)
json_output = false

# Parallel operations
parallel = 4

# Exclude patterns
exclude_patterns = [
    "*.tmp",
    "*.log",
    ".git/*",
    "node_modules/*",
    "__pycache__/*",
]

# Audit log settings
audit_format = "json"
audit_log_path = "~/.orbit/audit.log"
```

All commands use these defaults automatically. CLI flags override individual settings. Setting `json_output = true` in the config file suppresses all human-readable output, progress bars, and statistics for machine consumption.

---

## 📊 Viewing Audit Logs

### Built-in History Viewer (Recommended)
```bash
# Human-friendly table of recent transfers
orbit history

# Last 50 entries
orbit history --limit 50

# Machine-readable JSON output
orbit history --json

# Use a specific audit log file
orbit history --audit-file /var/log/orbit/audit.jsonl
```

### Manual Inspection (JSON Format)
```bash
# View all logs
cat orbit_audit.log | jq

# View only successful operations
cat orbit_audit.log | jq 'select(.status == "success")'

# View failed operations
cat orbit_audit.log | jq 'select(.status == "failed")'

# Total bytes transferred
cat orbit_audit.log | jq '.bytes_copied' | paste -sd+ | bc
```

### Text/CSV Format
```bash
orbit -s file.txt -d backup.txt --audit-format text

# Open in Excel or
cat orbit_audit.log | column -t -s,
```

---

## LLM-Native Debug Logging (Developer Mode)

When you need LLM-friendly JSON logs without audit/HMAC or OTel layers, set a log mode environment variable:

```bash
ORBIT_LOG_MODE=llm-debug RUST_LOG=debug \
  orbit copy /source /dest
```

For integration tests:

```bash
TEST_LOG=llm-debug RUST_LOG=debug \
  cargo test --test integration_tests -- --nocapture
```

---

## 🎯 CLI Flags Quick Reference

| Flag | Short | Description | Example |
|------|-------|-------------|---------|
| `--source` | `-s` | Source path (or use positional) | `orbit src dst` |
| `--dest` | `-d` | Destination path (or use positional) | `orbit src dst` |
| `--recursive` | `-R` | Copy directories (auto-detected) | `-R` |
| `--compress` | `-c` | Compression (`auto`, `zstd:N`, `lz4`, `none`) | `--compress auto` |
| `--zstd` | | Shorthand for `--compress zstd:3` | `--zstd` |
| `--lz4` | | Shorthand for `--compress lz4` | `--lz4` |
| `--resume` | `-r` | Enable resume | `--resume` |
| `--mode` | `-m` | Copy mode | `--mode sync` |
| `--profile` | | Configuration preset | `--profile backup` |
| `--workers` | | Parallel threads | `--workers 8` |
| `--exclude` | | Exclude pattern | `--exclude "*.tmp"` |
| `--dry-run` | | Preview only | `--dry-run` |
| `--preserve-metadata` | `-p` | Keep timestamps (default: on) | `-p` |
| `--no-preserve-metadata` | | Disable metadata preservation | |
| `--max-bandwidth` | | Limit speed (MB/s) | `--max-bandwidth 10` |
| `--retry-attempts` | | Retry count | `--retry-attempts 5` |
| `--quiet` | `-q` | Suppress non-essential output | `-q` |
| `--json` | | Machine-readable JSON output | `--json` |
| `--raw` | | Raw byte output (no formatting) | `--raw` |
| `--no-stat` | | Disable end-of-run statistics | `--no-stat` |

### Subcommands

| Command | Description | Example |
|---------|-------------|---------|
| `orbit sync <SRC> <DST>` | Sync mode (recursive, metadata) | `orbit sync /data /backup` |
| `orbit backup <SRC> <DST>` | Backup profile (checksums + Zstd) | `orbit backup /data /backup` |
| `orbit mirror <SRC> <DST>` | Mirror mode (exact replica) | `orbit mirror /data /replica` |
| `orbit doctor` | Validate config and probe system | `orbit doctor` |
| `orbit init` | Interactive setup wizard | `orbit init` |
| `orbit cp <SRC> <DST>` | Copy alias (same as bare `orbit`) | `orbit cp /data /backup` |
| `orbit explain <SRC> <DST>` | Preview transfer plan (no files touched) | `orbit explain /data /backup -R` |
| `orbit history` | View recent transfer history | `orbit history --limit 50` |
| `orbit presets` | Show available profile presets | `orbit presets` |
| `orbit run` | Batch execution from file | `orbit run --file cmds.txt` |

---

## 🆘 Troubleshooting

### "Source not found"
```bash
# Check the path exists
ls -la /path/to/source

# Use absolute paths if relative paths don't work
orbit -s /absolute/path/to/source -d /absolute/path/to/dest
```

### "Insufficient disk space"
```bash
# Check available space
df -h /destination/path

# Use compression to reduce size
orbit -s large.iso -d backup.iso --compress zstd:19
```

### Transfer is too slow
```bash
# Reduce compression
orbit -s file -d backup --compress lz4

# Or disable compression
orbit -s file -d backup --compress none

# For directories, increase parallelism
orbit -s ./dir -d ./backup -R --parallel 16
```

### Need to cancel and resume
```bash
# Just Ctrl+C to stop
^C

# Run the same command with --resume to continue
orbit -s file -d backup --resume
```

---

## 📚 Learn More

- **Full documentation**: Run `orbit --help`
- **Migration from v0.2.0**: See `MIGRATION_GUIDE.md`
- **Implementation details**: See `IMPLEMENTATION_SUMMARY.md`
- **Configuration examples**: See `orbit.toml`
- **Tests**: Run `cargo test`

---

## 🎓 Examples by Use Case

### Developer Workflow
```bash
# Sync project excluding build artifacts
orbit sync ~/projects/myapp /backup/myapp \
  --exclude "target/*" \
  --exclude "node_modules/*" \
  --exclude ".git/*"
```

### System Administrator
```bash
# Nightly database backup with compression
orbit backup /var/lib/postgresql/backup.sql /mnt/backup/db/backup.sql \
  --audit-format json \
  --audit-log /var/log/orbit/backup.log
```

### Data Migration
```bash
# Transfer large dataset to new server (auto-network enables resume + retries)
orbit /data/warehouse smb://newserver/warehouse -R \
  --zstd \
  --workers 8 \
  --max-bandwidth 50
```

### Personal Backup
```bash
# Mirror documents to external drive
orbit mirror ~/Documents /mnt/external/Documents \
  --exclude "*.tmp"
```

### CI/CD Pipeline (Machine-Readable Output)
```bash
# JSON output for automation — no progress bars, no stats, no human text
orbit backup /artifacts s3://bucket/builds --json
```

---

## ✅ Quick Checklist

Before your first real copy:

- [ ] Install: `cargo install --path .`
- [ ] Test: `orbit --version`
- [ ] Setup: `orbit init` (interactive wizard — sets up config, exclusions, and shell completions)
- [ ] Diagnose: `orbit doctor` (verify config + hardware)
- [ ] Preview: `orbit explain /test /backup -R` (see what Orbit would do)
- [ ] Try dry run: `orbit /test /backup --dry-run`
- [ ] Read help: `orbit --help` (essential flags) or `orbit --help-all` (every flag)
- [ ] Run tests: `cargo test`

---

**You're ready to use Orbit!** 🚀

For questions or issues, open a ticket on GitHub.
