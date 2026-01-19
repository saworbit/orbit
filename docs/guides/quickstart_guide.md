# Orbit v0.4.1 - Quick Start Guide

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
# Should output: orbit 0.4.0
```

---

## 🚀 Basic Usage

### Copy a Single File
```bash
orbit -s input.txt -d output.txt
```

### Copy with Compression
```bash
# Fast compression (LZ4)
orbit -s large.dat -d backup.dat --compress lz4

# Better compression (Zstd level 3, balanced)
orbit -s large.dat -d backup.dat --compress zstd:3

# Maximum compression (Zstd level 19, slow but best)
orbit -s large.dat -d backup.dat --compress zstd:19
```

### Copy a Directory
```bash
orbit -s ./source_dir -d ./backup_dir -R
```

### Resume Interrupted Transfer
```bash
orbit -s bigfile.iso -d /mnt/network/bigfile.iso --resume
# If interrupted, just run the same command again!
```

---

## 💡 Common Scenarios

### 1. Network Backup (Slow/Unreliable Connection)
```bash
orbit -s ./important_data -d /mnt/backup \
  -R \
  --compress zstd:9 \
  --resume \
  --retry-attempts 10 \
  --exponential-backoff \
  --max-bandwidth 5
```

**What this does:**
- Recursively copies directory
- Compresses with Zstd level 9 (good compression)
- Resumes if interrupted
- Retries up to 10 times with exponential backoff
- Limits bandwidth to 5 MB/s

### 2. Fast Local Sync (Many Small Files)
```bash
orbit -s ./project -d /backup/project \
  -R \
  --mode sync \
  --parallel 8 \
  --exclude "node_modules/*" \
  --exclude "*.tmp"
```

**What this does:**
- Syncs directory (only copies new/changed files)
- Uses 8 parallel threads
- Excludes node_modules and temp files

### 3. Large File Transfer with Verification
```bash
orbit -s database_dump.sql -d /backup/database_dump.sql \
  --compress zstd:3 \
  --resume \
  --preserve-metadata
```

**What this does:**
- Compresses the file
- Verifies with SHA-256 checksum
- Preserves timestamps and permissions
- Can resume if interrupted

### 4. Preview Before Copying (Dry Run)
```bash
orbit -s ./source -d ./dest -R --dry-run --verbose
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

## ⚙️ Configuration File (Optional)

Create `~/.orbit/orbit.toml` for default settings:

```toml
# Compression
compression = { zstd = { level = 3 } }

# Chunk size in bytes
chunk_size = 2048

# Retry attempts
retry_attempts = 5

# Preserve metadata
preserve_metadata = true

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

Now all commands use these defaults automatically! Override with CLI flags.

---

## 📊 Viewing Audit Logs

### JSON Format (Default)
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
| `--source` | `-s` | Source path | `-s ./file.txt` |
| `--destination` | `-d` | Destination path | `-d /backup/file.txt` |
| `--recursive` | `-R` | Copy directories | `-R` |
| `--compress` | `-c` | Compression type | `--compress zstd:9` |
| `--resume` | `-r` | Enable resume | `--resume` |
| `--mode` | `-m` | Copy mode | `--mode sync` |
| `--parallel` | | Parallel threads | `--parallel 8` |
| `--exclude` | | Exclude pattern | `--exclude "*.tmp"` |
| `--dry-run` | | Preview only | `--dry-run` |
| `--preserve-metadata` | `-p` | Keep timestamps | `-p` |
| `--max-bandwidth` | | Limit speed (MB/s) | `--max-bandwidth 10` |
| `--retry-attempts` | | Retry count | `--retry-attempts 5` |

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
# Backup project excluding build artifacts
orbit -s ~/projects/myapp -d /backup/myapp \
  -R \
  --exclude "target/*" \
  --exclude "node_modules/*" \
  --exclude ".git/*" \
  --mode sync
```

### System Administrator
```bash
# Nightly database backup with compression
orbit -s /var/lib/postgresql/backup.sql -d /mnt/backup/db/backup.sql \
  --compress zstd:9 \
  --preserve-metadata \
  --audit-format json \
  --audit-log /var/log/orbit/backup.log
```

### Data Migration
```bash
# Transfer large dataset to new server
orbit -s /data/warehouse -d /mnt/newserver/warehouse \
  -R \
  --compress zstd:3 \
  --parallel 8 \
  --resume \
  --retry-attempts 10 \
  --max-bandwidth 50
```

### Personal Backup
```bash
# Backup documents to external drive
orbit -s ~/Documents -d /mnt/external/Documents \
  -R \
  --mode mirror \
  --exclude "*.tmp" \
  --preserve-metadata
```

---

## ✅ Quick Checklist

Before your first real copy:

- [ ] Install: `cargo install --path .`
- [ ] Test: `orbit --version`
- [ ] Try dry run: `orbit -s test -d backup --dry-run`
- [ ] Create config: Copy `orbit.toml` to `~/.orbit/`
- [ ] Read help: `orbit --help`
- [ ] Run tests: `cargo test`

---

**You're ready to use Orbit!** 🚀

For questions or issues, open a ticket on GitHub.
