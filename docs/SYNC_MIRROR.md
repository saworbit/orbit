# ORBIT Sync and Mirror Features

This document describes ORBIT's directory synchronization and mirroring capabilities for efficient, resilient file transfers.

## Overview

ORBIT provides two primary modes for directory-level operations:

- **Sync Mode**: Copies new or changed files from source to destination, skipping unchanged ones. Does not delete extra files at the destination. Ideal for backups or incremental data transport.

- **Mirror Mode**: Like sync, but also deletes files at the destination that no longer exist in the source. Use for exact replicas, e.g., mirroring a data lake for transformation workflows.

## Key Features

### Delta Detection

Instead of full file copies, ORBIT can detect and transfer only changed portions:

- **ModTime**: Fastest check - compares modification timestamps (default)
- **Size**: Compares file sizes only
- **Checksum**: Full content hashing using BLAKE3
- **Delta**: Block-level comparison using rolling checksums (rsync-like)

### Filter System

Control what gets synced/mirrored using include/exclude rules:

- **Glob patterns**: `*.rs`, `target/**`
- **Regex patterns**: `regex: ^test_\d+\.txt$`
- **Path matching**: `path: src/main.rs`
- **File-based filters**: Load rules from `.orbitfilter` files

### Resilience

Built-in reliability features:

- **Automatic retries** with exponential backoff
- **Error modes**: Abort, Skip, or Partial
- **State tracking** via Magnetar state machine
- **Resume capability** after interruptions
- **Pre-flight disk space validation**

### Dry-Run Mode

Preview operations without making changes:

```bash
orbit --source /data --dest /backup --mode sync --dry-run
```

## Usage Examples

### Basic Sync

```bash
# Sync directories (copy new/changed files only)
orbit --source /project --dest /backup --recursive --mode sync
```

### Mirror with Deletions

```bash
# Mirror directory (exact replica with deletions)
orbit --source /data --dest /replica --recursive --mode mirror
```

### With Delta Detection

```bash
# Use checksums for change detection
orbit --source /src --dest /dst --recursive --mode sync --check checksum

# Use block-level delta for large files
orbit --source /src --dest /dst --recursive --mode sync --check delta
```

### With Filters

```bash
# Exclude build artifacts
orbit --source /project --dest /backup --recursive \
    --mode sync \
    --exclude "target/**" \
    --exclude "*.log"

# Include only specific files
orbit --source /project --dest /backup --recursive \
    --mode sync \
    --include "**/*.rs" \
    --include "Cargo.toml"

# Use filter file
orbit --source /project --dest /backup --recursive \
    --mode sync \
    --filter-from .orbitfilter
```

### Error Handling

```bash
# Skip failed files and continue
orbit --source /src --dest /dst --recursive --mode sync --error-mode skip

# Keep partial files for resume
orbit --source /src --dest /dst --recursive --mode sync --error-mode partial
```

## Filter File Format

Create a `.orbitfilter` file with rules:

```text
# Comments start with #

# Include patterns (+ prefix)
+ **/*.rs
+ Cargo.toml

# Exclude patterns (- prefix)
- target/**
- **/*.log

# Include/exclude keywords
include **/*.md
exclude **/temp/**

# Explicit pattern types
+ glob: src/**/*.rs
- regex: ^test_\d+\.txt$
+ path: README.md

# Negation (inverts action)
! + *.secret
```

Rules are evaluated in order (first match wins).

## Check Modes

### ModTime (Default)

```bash
orbit --source /src --dest /dst --mode sync --check modtime
```

- Compares modification timestamps
- Fastest check mode
- May miss content changes if timestamps are unreliable

### Size

```bash
orbit --source /src --dest /dst --mode sync --check size
```

- Compares file sizes
- Very fast
- Won't detect same-size changes

### Checksum

```bash
orbit --source /src --dest /dst --mode sync --check checksum
```

- Computes BLAKE3 hash of entire file
- Most accurate
- Slower for large files

### Delta

```bash
orbit --source /src --dest /dst --mode sync --check delta --delta-block-size 1024
```

- Block-level comparison using rolling checksums
- Best for large files with small changes
- Minimum file size: 64KB
- Configurable block size

## Parallel Processing

```bash
# Use 8 parallel workers
orbit --source /src --dest /dst --recursive --mode sync --parallel 8

# Auto-detect based on CPU cores (default)
orbit --source /src --dest /dst --recursive --mode sync --parallel 0
```

## API Usage

For programmatic access:

```rust
use orbit::config::{CopyConfig, CopyMode, CheckMode};
use orbit::core::resilient_sync::resilient_sync;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        check_mode: CheckMode::Checksum,
        exclude_patterns: vec!["target/**".to_string()],
        dry_run: false,
        ..Default::default()
    };

    let stats = resilient_sync(
        Path::new("/source"),
        Path::new("/destination"),
        config,
    )?;

    println!("Copied {} files ({} bytes)",
        stats.files_copied,
        stats.bytes_copied);
    println!("Deleted {} files", stats.files_deleted);

    Ok(())
}
```

## Statistics

After a sync/mirror operation, you get detailed statistics:

- `files_copied`: Number of files transferred
- `files_deleted`: Number of files removed (mirror mode)
- `files_skipped`: Number of unchanged files
- `files_failed`: Number of failed operations
- `bytes_copied`: Total bytes transferred
- `bytes_saved_by_delta`: Bytes saved through delta detection
- `duration`: Total operation time

## Performance Tips

1. **Use ModTime check for daily backups** - fastest and usually sufficient
2. **Use Checksum check for critical data** - ensures content integrity
3. **Use Delta check for large files** - minimizes bandwidth
4. **Increase parallel workers** for many small files
5. **Use filters** to exclude unnecessary files (build artifacts, logs)
6. **Enable compression** for network transfers

## Comparison with Other Tools

| Feature | ORBIT | rsync | rclone |
|---------|-------|-------|--------|
| Delta transfers | ✅ | ✅ | ✅ |
| Rolling checksum | ✅ | ✅ | ❌ |
| Filter files | ✅ | ✅ | ✅ |
| Parallel transfers | ✅ | ❌ | ✅ |
| State persistence | ✅ | ❌ | ❌ |
| Compression | ✅ | ✅ | ✅ |
| S3 support | ✅ | ❌ | ✅ |
| Cross-platform | ✅ | ⚠️ | ✅ |

## Troubleshooting

### Sync is slow

- Check if Delta mode is appropriate for your files
- Increase `--parallel` for many small files
- Use `--check modtime` for faster checks
- Exclude unnecessary directories

### Files not being deleted in Mirror mode

- Check your exclude patterns
- Excluded files are protected from deletion
- Use `--dry-run` to preview deletions

### High memory usage

- ORBIT uses streaming iteration for directories
- Large delta operations may use more memory
- Consider smaller `--delta-block-size`

## See Also

- [examples/backup.orbitfilter](../examples/backup.orbitfilter) - Example filter for backups
- [examples/mirror.orbitfilter](../examples/mirror.orbitfilter) - Example filter for mirroring
- [scripts/benchmark_sync.ps1](../scripts/benchmark_sync.ps1) - Performance benchmarking
