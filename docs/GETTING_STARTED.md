# Getting Started with Orbit

> **Open Resilient Bulk Information Transfer**

This guide gets you from zero to your first transfer in under 5 minutes.

> **Alpha Software:** Orbit is in active development (v0.6.0). Test in non-production environments first and maintain backups when working with important data.

---

## Installation

### From Source (Recommended)

```bash
git clone https://github.com/saworbit/orbit.git
cd orbit

# Minimal build - local copy only (~10MB binary)
cargo build --release

# With network protocols (S3, SMB, SSH, Azure, GCS)
cargo build --release --features network

# Install to PATH
cargo install --path .                    # Minimal
cargo install --path . --features network  # With network
```

### Feature Flags & Binary Sizes

Orbit defaults to a minimal build for fast compiles and small binaries. Network protocols are opt-in.

| Feature | Description | Size Impact | Default |
|---------|-------------|-------------|---------|
| `zero-copy` | OS-level zero-copy syscalls | +1MB | Yes |
| `network` | All network protocols | +31MB | No |
| `s3-native` | Amazon S3 and compatible storage | +15MB | No |
| `smb-native` | Native SMB2/3 network shares | +8MB | No |
| `ssh-backend` | SSH/SFTP remote access | +5MB | No |
| `azure-native` | Microsoft Azure Blob Storage | +3MB | No |
| `gcs-native` | Google Cloud Storage | +3MB | No |

---

## First-Time Setup

Run the interactive setup wizard to auto-detect your hardware and generate an optimal config:

```bash
orbit init
```

The wizard will:
1. Scan your system (CPU cores, RAM, I/O speed)
2. Ask about your primary use case (Backup, Sync, Cloud, Network)
3. Set up common file exclusion patterns
4. Offer shell completion installation
5. Save an optimized config to `~/.orbit/orbit.toml`

---

## Basic Usage

### Copying Files

```bash
# Simple file copy (positional arguments)
orbit source.txt destination.txt

# Recursive directory copy
orbit /data /backup -R

# With named flags
orbit --source /data --dest /backup --recursive
```

### Shorthand Subcommands

These set mode and profile automatically, and support all global flags:

```bash
orbit cp /data /backup                       # Copy (reads naturally)
orbit sync /data /backup                     # Sync mode, recursive, metadata preserved
orbit backup /data /backup                   # Backup profile (checksums + Zstd + resume)
orbit mirror /data /backup                   # Mirror mode (exact replica)
```

### Profiles

Pre-configured presets for common scenarios:

```bash
orbit /data /backup -R --profile fast        # Zero-copy, no checksums (max speed)
orbit /data /backup -R --profile safe        # Checksums + resume + retries
orbit /data /backup -R --profile backup      # Checksums + Zstd + resume + metadata
orbit /data /backup -R --profile network     # Zstd + resume + 10 retries
```

### Preview Before You Run

```bash
# Dry run - see what would happen
orbit /data /backup -R --dry-run --verbose

# Plain-English transfer plan (no files touched)
orbit explain /data /backup -R --zstd
```

---

## Compression

```bash
# Smart auto-selection (Zstd for remote, LZ4 for cross-device, off for same-device)
orbit /data /backup --compress auto

# Explicit compression
orbit /data /backup -R --zstd               # Zstd compression
orbit /data /backup -R --lz4                # LZ4 (faster, lower ratio)
orbit /data /backup -R --compress zstd:5    # Zstd with custom level
```

---

## Cloud & Network Transfers

### S3

```bash
# Upload to S3
orbit /local/data s3://my-bucket/path/ -R --profile network

# High-concurrency upload
orbit /data s3://bucket/backups/ -R --workers 256 --concurrency 8

# Stream to stdout
orbit cat s3://bucket/data/report.csv | head -100

# Pre-signed URL
orbit presign s3://bucket/data/report.csv --expires 3600
```

See the full [S3 User Guide](guides/S3_USER_GUIDE.md) for multipart uploads, wildcard listing, and more.

### SSH/SFTP

```bash
orbit /local/data ssh://user@host:/remote/path -R --resume
```

### SMB/CIFS

```bash
orbit /local/files smb://nas/backup -R --resume --retry-attempts 10
```

### Azure Blob / GCS

```bash
orbit /data az://container/path -R
orbit /data gs://bucket/path -R
```

See the [GCS User Guide](guides/GCS_USER_GUIDE.md) and [Backend Guide](guides/BACKEND_GUIDE.md) for full details.

---

## Filters

Control which files get transferred:

```bash
# Exclude patterns
orbit /project /backup -R --exclude="target/**" --exclude="*.log"

# Include override
orbit /project /backup -R --exclude="*.log" --include="important.log"

# Filter file for complex rules
orbit /data /backup -R --filter-from=backup.orbitfilter
```

See the [Filter System Guide](guides/FILTER_SYSTEM.md) for glob, regex, and path pattern syntax.

---

## Resume & Retries

```bash
# Resume an interrupted transfer
orbit /data /backup -R --resume

# Resilient transfer with retries
orbit /data /backup -R --retry-attempts 5 --exponential-backoff --error-mode partial

# Skip failed files and continue
orbit /data /backup -R --error-mode skip
```

---

## Diagnostics

```bash
# Validate config and check environment
orbit doctor

# View transfer history
orbit history
orbit history --limit 50 --json

# Bandwidth-limited transfer with progress
orbit /data /backup -R --max-bandwidth 10 --workers 4 --show-progress
```

---

## Configuration File

Create `~/.orbit/orbit.toml` (or run `orbit init` to generate one):

```toml
recursive = true
preserve_metadata = true
resume_enabled = true
verify_checksum = true
compression = { zstd = { level = 5 } }
show_progress = true
retry_attempts = 3
exponential_backoff = true
error_mode = "abort"
```

### Configuration Priority

1. **CLI arguments** (highest)
2. **`--profile` preset** or auto-network detection
3. **`~/.orbit/orbit.toml`** (user config)
4. **Built-in defaults** (lowest)

When the destination is a remote URI (`s3://`, `smb://`, `ssh://`), Orbit automatically overlays network-friendly defaults (resume, compression, retries) without overriding your customized values.

---

## Next Steps

- [Quickstart Guide](guides/quickstart_guide.md) - More CLI examples
- [Init Wizard Guide](guides/INIT_WIZARD_GUIDE.md) - Deep dive into `orbit init`
- [Backend Guide](guides/BACKEND_GUIDE.md) - All storage backends
- [Architecture Overview](../ARCHITECTURE.md) - How Orbit works under the hood
- [Feature Maturity Matrix](../README.md#feature-maturity-matrix) - What's production-ready
