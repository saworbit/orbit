# Delta Detection - Quick Start

## 5-Minute Guide to Efficient Transfers

### Basic Usage

```bash
# Simple delta transfer
orbit --source bigfile.iso --dest bigfile.iso --check delta

# Recursive delta sync
orbit --source /data --dest /backup --recursive --check delta
```

### Detection Modes

| Mode | Speed | Use Case | Command |
|------|-------|----------|---------|
| **modtime** | ⚡⚡⚡ | Quick sync, reliable timestamps | `--check modtime` (default) |
| **size** | ⚡⚡⚡ | Size-based comparison | `--check size` |
| **checksum** | ⚡ | Full integrity verification | `--check checksum` |
| **delta** | ⚡⚡ | Minimize bandwidth, large files | `--check delta` |

### Common Scenarios

#### 1. Daily Backup (90-95% savings)
```bash
orbit \
  --source /database/prod.sql \
  --dest /backups/prod.sql \
  --check delta
```

#### 2. Remote Sync Over Slow Link
```bash
orbit \
  --source local_vm.img \
  --dest remote:/vms/vm.img \
  --check delta \
  --resume
```

#### 3. Large File Update (e.g., Software ISO)
```bash
orbit \
  --source ubuntu-new.iso \
  --dest ubuntu-old.iso \
  --check delta \
  --block-size 1024
```

### Block Size Tuning

```bash
# Small files / scattered changes
--block-size 64    # 64KB blocks

# General purpose (default)
--block-size 1024  # 1MB blocks

# Large sequential changes
--block-size 2048  # 2MB blocks
```

**Rule of thumb**: `block_size = file_size / 1000` (cap at 4MB)

### Key Options

```bash
# Essential
--check delta              # Enable delta transfer
--block-size <KB>         # Block size in KB (default: 1024)
--whole-file              # Force full copy (disable delta)

# Advanced
--update-manifest         # Track in database
--ignore-existing         # Skip existing files
--delta-manifest <path>   # Manifest DB location
```

### Expected Savings

| File Changes | Savings | Transfer Time |
|--------------|---------|---------------|
| Identical    | 95-100% | < 1% |
| Minor edits  | 90-95%  | 5-10% |
| Moderate     | 70-90%  | 10-30% |
| Major        | 30-70%  | 30-70% |
| Different    | 0-10%   | 100% |

### Quick Tips

✅ **Do:**
- Use delta for files > 64KB
- Use for similar files with < 50% changes
- Combine with `--resume` for large transfers
- Tune block size for your workload

❌ **Don't:**
- Use delta for small files (< 64KB)
- Use delta for completely different files
- Use delta + compression together
- Use block size > file_size / 100

### Real-World Examples

```bash
# Example 1: 1GB database, daily changes
$ orbit --source db.sql --dest backup/db.sql --check delta
✓ Delta: 950/1000 blocks matched (95% savings, 50MB/1000MB transferred)
Time: 2.1s (instead of 30s for full copy)

# Example 2: 10GB VM image, minor updates
$ orbit --source vm-new.qcow2 --dest vm-old.qcow2 --check delta --block-size 2048
✓ Delta: 4800/5000 blocks matched (96% savings, 400MB/10GB transferred)
Time: 15s (instead of 5 minutes for full copy)

# Example 3: Source code directory
$ orbit --source ./code --dest remote:/code --recursive --check delta --block-size 64
✓ Delta: 95% file similarity
Time: 5s (instead of 45s for full copy)
```

### Performance Comparison

| Method | 1GB file with 5% changes |
|--------|-------------------------|
| **Standard copy** | 30s, 1GB transferred |
| **Delta transfer** | 3s, 50MB transferred |
| **Savings** | 10x faster, 95% less data |

### Troubleshooting

**Issue**: No savings with delta
- Files might be completely different (use `--check modtime` first)
- Block size might be wrong (try 512 or 2048)

**Issue**: Slower than expected
- File too small (< 64KB) - delta auto-skips, uses regular copy
- Block size too small - increase to 1024 or 2048

**Issue**: High CPU usage
- Normal for delta (checksumming overhead)
- Consider `--check modtime` for faster transfers

### Next Steps

1. **Read full guide**: [DELTA_DETECTION_GUIDE.md](DELTA_DETECTION_GUIDE.md)
2. **Run tests**: `cargo test delta`
3. **Try it**: Start with a large file backup scenario
4. **Tune**: Experiment with block sizes for your workload

### Need Help?

```bash
# Show all options
orbit --help

# Test delta on sample files
dd if=/dev/urandom of=test.dat bs=1M count=100
cp test.dat test_old.dat
dd if=/dev/urandom of=test.dat bs=1M count=5 seek=50 conv=notrunc
orbit --source test.dat --dest test_old.dat --check delta
```

---

**Pro Tip**: Start with `--check delta` default settings. Only tune if needed!
