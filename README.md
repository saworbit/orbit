# Orbit 🚀

**Intelligent file transfer with checksum, compression, resume capability, protocol abstraction, and zero-copy optimization**

[![Crates.io](https://img.shields.io/crates/v/orbit.svg)](https://crates.io/crates/orbit)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/saworbit/orbit/workflows/CI/badge.svg)](https://github.com/saworbit/orbit/actions)

Orbit is a high-performance file copy tool designed for reliability and speed. It provides advanced features like zero-copy system calls, resumable transfers, compression, and checksum verification—perfect for backing up critical data, migrating large datasets, or syncing files across systems.

---

## ✨ Features

### 🚀 Performance
- **⚡ Zero-Copy Transfers**: Kernel-level file copying on Linux (copy_file_range), macOS (copyfile), and Windows (CopyFileExW) for maximum throughput
- **Parallel Operations**: Multi-threaded directory copying
- **Smart Buffering**: Optimized chunk sizes for different scenarios
- **Bandwidth Throttling**: Control transfer speed to avoid network congestion

### 🛡️ Reliability
- **Resume Capability**: Automatically resume interrupted transfers
- **Checksum Verification**: SHA-256 validation with streaming or post-copy verification
- **Retry Logic**: Configurable retries with exponential backoff
- **Dry Run Mode**: Preview operations before executing

### 📦 Compression
- **LZ4**: Fast compression for quick transfers
- **Zstd**: High-ratio compression with configurable levels (1-22)
- **Automatic Detection**: Skip compression for already-compressed files

### 🌐 Protocol Support
- **Local Filesystem**: Full-featured with zero-copy optimization
- **SMB/CIFS**: Network share support (experimental)
- **Cloud Storage**: S3, Azure Blob, GCS (planned)

### 🎯 Smart Features
- **Copy Modes**: Copy, Sync, Update, Mirror
- **Metadata Preservation**: Timestamps, permissions, extended attributes
- **Symlink Handling**: Skip, follow, or preserve symbolic links
- **Exclude Patterns**: Glob-based filtering
- **Progress Tracking**: Real-time progress bars
- **Audit Logging**: JSON or text format for compliance

---

## 📊 Performance

### Zero-Copy Benchmarks

On modern NVMe storage (Linux with copy_file_range):

| File Size | Buffered Copy | Zero-Copy | Speedup |
|-----------|---------------|-----------|---------|
| 10 MB     | 12 ms        | 8 ms      | 1.5x    |
| 100 MB    | 95 ms        | 35 ms     | 2.7x    |
| 1 GB      | 980 ms       | 340 ms    | 2.9x    |
| 10 GB     | 9.8 s        | 3.4 s     | 2.9x    |

**CPU Usage Reduction**: 60-80% lower CPU utilization with zero-copy

### When Zero-Copy Excels
- ✅ Large files (>1MB) on local storage
- ✅ Same filesystem copies (required on Linux)
- ✅ No compression needed
- ✅ NVMe-to-NVMe or SSD-to-SSD transfers

### When to Use Buffered Copy
- ⚠️ Cross-filesystem copies (Linux limitation)
- ⚠️ Network transfers with compression
- ⚠️ Small files (<64KB, syscall overhead)
- ⚠️ Bandwidth throttling needed

---

## 🚀 Installation

### From Crates.io
```bash
cargo install orbit
```

### From Source
```bash
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build --release
sudo cp target/release/orbit /usr/local/bin/
```

### Platform Requirements

- **Linux**: Kernel 4.5+ for copy_file_range (auto-fallback on older kernels)
- **macOS**: 10.15+ for optimal performance
- **Windows**: Windows 10+ recommended

---

## 📖 Usage

### Basic Copy
```bash
# Simple file copy (zero-copy enabled by default)
orbit -s /path/to/source.file -d /path/to/dest.file

# Recursive directory copy
orbit -s /path/to/source/ -d /path/to/dest/ -R

# With progress bar
orbit -s large_file.bin -d /backup/large_file.bin --show-progress
```

### Zero-Copy Control
```bash
# Zero-copy is enabled by default for optimal performance
orbit -s source.dat -d dest.dat

# Explicitly enable zero-copy (redundant but clear)
orbit -s source.dat -d dest.dat --zero-copy

# Disable zero-copy (use buffered copy)
orbit -s source.dat -d dest.dat --no-zero-copy

# Check platform capabilities
orbit capabilities
```

**Output:**
```
Orbit Platform Capabilities
═══════════════════════════════════════════════

Zero-Copy Support:
  Available: ✓ Yes
  Method: copy_file_range
  Cross-filesystem: ✗ No

Platform: linux
Architecture: x86_64
...
```

### Compression
```bash
# LZ4 compression (fast)
orbit -s large.txt -d compressed.txt --compress lz4

# Zstd compression (high ratio)
orbit -s data/ -d backup/ -R --compress zstd:9

# Zero-copy vs Compression trade-off
# For local copies: zero-copy is faster
orbit -s local.dat -d /backup/local.dat

# For network copies: compression wins
orbit -s local.dat -d smb://server/share/remote.dat --compress zstd:3
```

### Resume Capability
```bash
# Enable resume for large transfers
orbit -s huge_file.iso -d /backup/huge_file.iso --resume

# Resume is automatically disabled with zero-copy for simplicity
# Use --no-zero-copy if you need both resume and granular control
orbit -s huge.iso -d /backup/huge.iso --resume --no-zero-copy
```

### Checksum Verification
```bash
# Checksum enabled by default
orbit -s important.db -d /backup/important.db

# With zero-copy, checksum is calculated post-transfer
# (still very fast, just one extra read pass)

# Skip verification for maximum speed
orbit -s /data/ -d /backup/ -R --no-verify
```

### Configuration Presets
```bash
# FAST: Maximum speed (zero-copy, no verification)
orbit -s /data/ -d /nvme-backup/ -R --preset fast

# SAFE: Maximum reliability (resume, checksums, retries)
orbit -s /critical/ -d /backup/ -R --preset safe

# NETWORK: Optimized for remote transfers (compression, resume)
orbit -s /local/ -d smb://nas/backup/ -R --preset network
```

### Parallel Operations
```bash
# Auto-detect CPU cores
orbit -s /photos/ -d /backup/photos/ -R --parallel 0

# Use 8 parallel threads
orbit -s /documents/ -d /nas/documents/ -R --parallel 8

# Note: Zero-copy works with parallel operations
```

### Advanced Examples
```bash
# Bandwidth-limited transfer (disables zero-copy)
orbit -s large.mkv -d /slow-drive/large.mkv --max-bandwidth 10

# Dry run to preview operations
orbit -s /data/ -d /backup/ -R --dry-run

# Exclude patterns
orbit -s /home/user/ -d /backup/user/ -R \
  --exclude "*.tmp" \
  --exclude "node_modules" \
  --exclude ".git"

# With audit logging
orbit -s /critical/ -d /backup/ -R \
  --audit-log /var/log/orbit.json \
  --audit-format json
```

---

## ⚙️ Configuration

### Config File

Create `~/.config/orbit/config.toml`:
```toml
[default]
copy_mode = "copy"
recursive = false
preserve_metadata = true
resume_enabled = false
verify_checksum = true
use_zero_copy = true
compression = "none"
show_progress = true
chunk_size = 1048576  # 1MB
retry_attempts = 3
retry_delay_secs = 5
exponential_backoff = false
parallel = 0  # Auto-detect
symlink_mode = "skip"
exclude_patterns = ["*.tmp", ".DS_Store"]
```

### Environment Variables
```bash
# Disable zero-copy globally
export ORBIT_NO_ZERO_COPY=1

# Set default compression
export ORBIT_COMPRESSION=zstd:3

# Enable verbose logging
export RUST_LOG=orbit=debug
```

---

## 🔧 Library Usage

Orbit can be used as a Rust library:
```rust
use orbit::{
    config::CopyConfig,
    copy_file,
    get_zero_copy_capabilities,
    is_zero_copy_available,
};
use std::path::Path;

fn main() -> orbit::error::Result<()> {
    // Check zero-copy availability
    if is_zero_copy_available() {
        let caps = get_zero_copy_capabilities();
        println!("Zero-copy available: {}", caps.method);
    }
    
    // Copy with zero-copy optimization
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.verify_checksum = true;
    
    let source = Path::new("source.bin");
    let dest = Path::new("dest.bin");
    
    let stats = copy_file(source, dest, &config)?;
    
    println!("Copied {} bytes in {:?}", 
             stats.bytes_copied, 
             stats.duration);
    
    Ok(())
}
```

---

## 🤔 FAQ

### What is zero-copy?

Zero-copy is a technique where data is transferred directly between files by the kernel, without copying it through userspace memory. This reduces CPU usage and improves performance, especially for large files on fast storage.

### When should I disable zero-copy?

Disable zero-copy (`--no-zero-copy`) when:
- Copying across different filesystems on Linux
- You need bandwidth throttling
- You're debugging transfer issues
- Copying very small files where syscall overhead matters

### Does zero-copy work with checksums?

Yes! With zero-copy enabled, checksums are calculated after the transfer completes (post-copy verification). This adds one extra read pass but is still faster than buffered copying with streaming verification for large files.

### Can I use zero-copy with compression?

No. Compression requires reading and processing data in userspace, so zero-copy is automatically disabled when compression is enabled.

### Why is my transfer using buffered copy?

Zero-copy is automatically disabled when:
- Files are on different filesystems (Linux)
- Resume is enabled
- Compression is enabled
- Bandwidth limiting is active
- File size is less than 64KB
- Platform doesn't support zero-copy

Run `orbit capabilities` to check your platform support.

### How do I get maximum performance?

For maximum performance on local storage:
```bash
orbit -s source -d dest \
  --zero-copy \
  --no-verify \
  --parallel 0
```

Or simply:
```bash
orbit -s source -d dest --preset fast
```

### Is zero-copy safe?

Yes. Zero-copy uses well-tested kernel APIs that have been in production for years. The kernel handles all the data integrity guarantees. When combined with checksum verification, you get both speed and safety.

---

## 🏗️ Architecture

### Zero-Copy Implementation
```
┌─────────────────────────────────────────┐
│         Application (Orbit CLI)         │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│      Decision Layer (should_use_zc)     │
│  • Check file size (>64KB?)             │
│  • Check same filesystem?               │
│  • Check for conflicts (resume, etc)    │
└──────────────────┬──────────────────────┘
                   │
        ┌──────────┴──────────┐
        ▼                     ▼
┌──────────────┐      ┌──────────────┐
│  Zero-Copy   │      │   Buffered   │
│    Path      │      │     Copy     │
└──────┬───────┘      └──────┬───────┘
       │                     │
       ▼                     ▼
┌──────────────┐      ┌──────────────┐
│   Kernel     │      │  Userspace   │
│   DMA        │      │   Buffer     │
│  Transfer    │      │   Loop       │
└──────────────┘      └──────────────┘
```

### Platform-Specific Implementations

- **Linux**: `copy_file_range()` syscall (kernel 4.5+)
- **macOS**: `fcopyfile()` with COPYFILE_DATA flag
- **Windows**: `CopyFileExW()` with COPY_FILE_NO_BUFFERING
- **Other**: Automatic fallback to buffered copy

---

## 🧪 Testing

Run the test suite:
```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests

# Benchmark zero-copy performance
cargo bench

# Test on specific platform
cargo test --features zero-copy
```

---

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup
```bash
git clone https://github.com/saworbit/orbit.git
cd orbit
cargo build
cargo test
```

### Reporting Issues

When reporting issues related to zero-copy:
1. Run `orbit capabilities` and include output
2. Specify your platform (OS, kernel version)
3. Include file sizes and filesystem types
4. Provide `--verbose` output if possible

---

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

## 🙏 Acknowledgments

- Built with Rust 🦀
- Uses [rustix](https://github.com/bytecodealliance/rustix) for safe syscall wrappers
- Inspired by rsync, robocopy, and modern file transfer tools

---

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/saworbit/orbit/issues)
- **Discussions**: [GitHub Discussions](https://github.com/saworbit/orbit/discussions)
- **Email**: shaneawall@gmail.com

---

Made with ❤️ by [Shane Wall](https://github.com/saworbit)