# Migration Guide

---

## Latest: v0.3.x → v0.4.0

### Overview

Version 0.4.0 introduces the **protocol abstraction layer**, enabling support for multiple storage backends beyond local filesystems. This is primarily an internal architectural change with minimal impact on existing CLI users.

### Breaking Changes

**None for CLI users.** All existing commands and flags work exactly as before.

### New Features for CLI Users

#### Protocol URI Support (Experimental)

You can now use URI syntax for source and destination (experimental):
```bash
# Local filesystem (works as before)
orbit -s /tmp/file.txt -d /backup/file.txt

# SMB share (experimental - stub implementation)
orbit -s smb://server/share/file.txt -d /local/file.txt
orbit -s smb://user:pass@server/share/file.txt -d /local/file.txt

Note: SMB support in v0.4.0 is experimental/stub only. Full implementation coming in v0.4.1.
For Library Users
If you're using Orbit as a Rust library:

New modules available:
use orbit::protocol::{Protocol, StorageBackend};

// Parse a URI
let (protocol, path) = Protocol::from_uri("smb://server/share/file.txt")?;

// Create a backend
let backend = protocol.create_backend()?;

Existing code continues to work:

// This still works exactly as before
use orbit::core::copy_file;
use orbit::config::CopyConfig;

let config = CopyConfig::default();
copy_file(&source, &dest, &config)?;

Upgrade Checklist

 No changes needed for CLI usage
 If using as library, review new protocol module
 If interested in SMB, wait for v0.4.1 for production use
 Run cargo update to get latest version
 Run tests: cargo test

# Migration Guide: Orbit v0.2.0 → v0.3.0

## Overview

Version 0.3.0 is a major refactoring that introduces breaking changes to improve reliability, performance, and extensibility. This guide will help you migrate.

---

## Breaking Changes

### 1. **CLI Arguments Changed**

**Old (v0.2.0):**
```bash
orbit --source ./file.txt --destination /dest/file.txt --compress --retry-attempts 3
```

**New (v0.3.0):**
```bash
orbit -s ./file.txt -d /dest/file.txt --compress zstd:3 --retry-attempts 3
```

**Key differences:**
- `--compress` now requires a value: `none`, `lz4`, or `zstd[:level]`
- Short flags available: `-s` (source), `-d` (destination), `-R` (recursive)
- New flags: `--mode`, `--parallel`, `--max-bandwidth`, `--audit-format`

### 2. **Compression Options**

**Old:** Boolean flag `--compress` (LZ4 only)

**New:** String argument `--compress TYPE`
- `--compress none` - No compression
- `--compress lz4` - LZ4 compression
- `--compress zstd` - Zstd level 3 (default)
- `--compress zstd:19` - Zstd maximum compression

### 3. **Audit Log Format**

**Old:** Custom CSV-like format

**New:** JSON Lines by default (one JSON object per line)

**Migration:**
- Use `--audit-format csv` to get CSV output
- Old audit logs won't be automatically migrated
- JSON format is more parseable: `cat orbit_audit.log | jq`

**Example JSON entry:**
```json
{
  "timestamp": "2025-10-11T10:30:00Z",
  "source": "/tmp/source.txt",
  "destination": "/tmp/dest.txt",
  "bytes_copied": 1024,
  "duration_ms": 523,
  "checksum": "abc123...",
  "status": "success",
  "attempts": 1
}
```

### 4. **Library API (if using as dependency)**

**Old:**
```rust
// Everything in main.rs
```

**New:**
```rust
use orbit::config::CopyConfig;
use orbit::core::copy_file;

let config = CopyConfig::default();
copy_file(&source, &dest, &config)?;
```

---

## New Features

### 1. **Configuration File Support**

Create `orbit.toml` in your project or `~/.orbit/orbit.toml`:

```toml
[defaults]
compress = "zstd:3"
chunk_size = 2048
parallel = 4

[exclude]
patterns = ["*.tmp", "*.log", ".git/*"]
```

### 2. **Multiple Copy Modes**

```bash
# Only copy new files
orbit -s ./src -d ./dest -R --mode sync

# Only copy newer files
orbit -s ./src -d ./dest -R --mode update

# Mirror (delete extra files in destination)
orbit -s ./src -d ./dest -R --mode mirror
```

### 3. **Parallel Copying**

```bash
# Auto-detect based on CPU cores
orbit -s ./src -d ./dest -R --parallel 0

# Use 8 parallel threads
orbit -s ./src -d ./dest -R --parallel 8
```

### 4. **Bandwidth Limiting**

```bash
# Limit to 10 MB/s
orbit -s ./large.iso -d /dest/large.iso --max-bandwidth 10
```

### 5. **Exclude Patterns**

```bash
# Exclude temporary files
orbit -s ./src -d ./dest -R --exclude "*.tmp" --exclude "*.log"
```

### 6. **Dry Run Mode**

```bash
# See what would be copied without actually copying
orbit -s ./src -d ./dest -R --dry-run
```

### 7. **Exponential Backoff**

```bash
# Retry with exponential backoff (5s, 10s, 20s, 40s...)
orbit -s ./file -d ./dest --retry-attempts 5 --exponential-backoff
```

---

## Configuration Precedence

Settings are applied in this order (later overrides earlier):

1. Built-in defaults
2. `~/.orbit/orbit.toml` (user config)
3. `./orbit.toml` (project config)
4. Environment variables (future)
5. CLI arguments (highest priority)

---

## Performance Improvements

### v0.2.0 → v0.3.0 Benchmarks

| Operation | v0.2.0 | v0.3.0 | Improvement |
|-----------|--------|--------|-------------|
| 1000 small files | 45s | 12s | **73% faster** |
| 10GB file (no compression) | 120s | 115s | 4% faster |
| 10GB file (compression) | 180s | 145s | **19% faster** |
| Directory tree (100k files) | N/A | 8m | **New feature** |

*Benchmarks on standard SSD with 8-core CPU*

---

## Code Organization Changes

**v0.2.0:** Single `main.rs` file

**v0.3.0:** Modular structure
```
src/
  lib.rs              # Public library API
  main.rs             # CLI entry point
  config.rs           # Configuration
  error.rs            # Error types
  audit.rs            # Audit logging
  core/
    mod.rs
    checksum.rs       # Streaming checksums
    resume.rs         # Resume logic
    metadata.rs       # Metadata preservation
    validation.rs     # Validation logic
  compression/
    mod.rs            # LZ4 & Zstd
```

---

## Testing Improvements

**v0.2.0:** No tests

**v0.3.0:** Comprehensive test suite
- 15+ integration tests
- Unit tests in each module
- ~60% code coverage

Run tests:
```bash
cargo test
```

---

## Error Messages

Error messages are now more descriptive with context:

**Old:**
```
Error: IO error
```

**New:**
```
Error: Checksum mismatch for file "/tmp/dest.txt": 
  expected abc123..., got def456...
```

---

## Upgrade Checklist

- [ ] Update CLI commands to new syntax
- [ ] Change `--compress` to `--compress zstd:3` (or desired level)
- [ ] Update any scripts that parse audit logs (now JSON by default)
- [ ] Review new configuration options in `orbit.toml`
- [ ] Test with `--dry-run` first
- [ ] Update any code that imports orbit as a library
- [ ] Run `cargo test` to verify functionality
- [ ] Check audit log format migration

---

## Rollback Plan

If you need to rollback to v0.2.0:

```bash
# Install specific version
cargo install --version 0.2.0 orbit

# Or build from tag
git checkout v0.2.0
cargo build --release
```

---

## Getting Help

- GitHub Issues: https://github.com/saworbit/orbit/issues
- Documentation: Run `orbit --help`
- Examples: See `examples/` directory

---

## What's Next (v0.4.0 Roadmap)

- SMB/network share support
- S3 and cloud storage protocols
- Watch mode (auto-sync on file changes)
- REST API for remote control
- GUI/Web dashboard

---

**Welcome to Orbit v0.3.0!** 🚀
