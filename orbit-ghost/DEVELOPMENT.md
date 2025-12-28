# Orbit GhostFS Development Guide

Guide for developers contributing to Orbit GhostFS.

## Table of Contents

- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Building and Testing](#building-and-testing)
- [Debugging](#debugging)
- [Code Style](#code-style)
- [Adding Features](#adding-features)
- [Performance Profiling](#performance-profiling)
- [Testing Strategy](#testing-strategy)
- [Release Process](#release-process)

## Development Setup

### Prerequisites

- Rust 1.70+ (stable)
- FUSE3 development libraries (see [INSTALL.md](INSTALL.md))
- Git
- A code editor with Rust support (VS Code + rust-analyzer recommended)

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/saworbit/orbit.git
cd orbit/orbit-ghost

# Build in debug mode
cargo build

# Run
cargo run
```

### IDE Setup

#### VS Code

Install extensions:
```bash
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb  # For debugging
code --install-extension serayuzgur.crates    # Cargo.toml management
```

**Recommended settings** (`.vscode/settings.json`):
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true
}
```

#### IntelliJ IDEA / CLion

Install the Rust plugin:
- **File** → **Settings** → **Plugins** → Search "Rust" → Install

Enable Clippy:
- **Settings** → **Languages & Frameworks** → **Rust** → **External Linters** → Enable Clippy

## Project Structure

```
orbit-ghost/
├── src/
│   ├── main.rs           # Entry point, FUSE mount, wormhole thread
│   ├── inode.rs          # GhostFile struct, manifest → FUSE mapping
│   ├── entangler.rs      # Priority queue, block fetching logic
│   └── fs.rs             # FUSE operations (lookup, read, readdir, etc.)
├── Cargo.toml            # Dependencies and metadata
├── demo_quantum.sh       # Demo script
├── README.md             # Overview
├── ARCHITECTURE.md       # Technical deep dive
├── GUIDE.md              # User manual
├── INSTALL.md            # Installation instructions
├── FAQ.md                # Common questions
├── ROADMAP.md            # Future plans
└── CHANGELOG.md          # Version history
```

### Module Responsibilities

| Module | Purpose | Key Structs/Functions |
|--------|---------|----------------------|
| `main.rs` | Bootstrap, mount FUSE, spawn wormhole | `main()`, wormhole thread |
| `inode.rs` | Virtual file representation | `GhostFile`, `to_attr()` |
| `entangler.rs` | Coordinate block fetching | `Entangler`, `ensure_block_available()` |
| `fs.rs` | FUSE syscall handlers | `OrbitGhostFS`, `Filesystem` impl |

## Building and Testing

### Build Commands

```bash
# Debug build (fast compile, slow runtime)
cargo build

# Release build (optimized)
cargo build --release

# Check without building (faster)
cargo check

# Build with all features (when added)
cargo build --all-features
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_block_calculation

# Run with output
cargo test -- --nocapture

# Run ignored tests (integration tests)
cargo test -- --ignored
```

### Linting

```bash
# Run Clippy (Rust linter)
cargo clippy

# Clippy with all features
cargo clippy --all-features

# Fix auto-fixable issues
cargo clippy --fix

# Check formatting
cargo fmt -- --check

# Auto-format code
cargo fmt
```

### Documentation

```bash
# Build documentation
cargo doc

# Build and open in browser
cargo doc --open

# Include private items
cargo doc --document-private-items
```

## Debugging

### Using LLDB (macOS/Linux)

```bash
# Build with debug symbols
cargo build

# Run with lldb
rust-lldb ./target/debug/orbit-ghost

# Inside lldb:
(lldb) b main  # Set breakpoint
(lldb) run     # Start
(lldb) c       # Continue
(lldb) bt      # Backtrace
(lldb) p variable_name  # Print variable
```

### Using GDB (Linux)

```bash
# Build with debug symbols
cargo build

# Run with gdb
rust-gdb ./target/debug/orbit-ghost

# Inside gdb:
(gdb) break main
(gdb) run
(gdb) next
(gdb) print variable_name
(gdb) backtrace
```

### VS Code Debugging

Create `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug orbit-ghost",
      "cargo": {
        "args": ["build", "--bin=orbit-ghost"]
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

Set breakpoints, press F5 to debug.

### Logging

```bash
# Enable all logs
RUST_LOG=trace cargo run

# Specific module
RUST_LOG=orbit_ghost::entangler=debug cargo run

# Multiple modules
RUST_LOG=orbit_ghost::fs=trace,orbit_ghost::entangler=debug cargo run

# Log to file
RUST_LOG=info cargo run 2>&1 | tee debug.log
```

### Tracing FUSE Calls

```bash
# Linux: strace
strace -e trace=fuse ./target/debug/orbit-ghost

# macOS: dtruss (requires sudo)
sudo dtruss -t fuse ./target/debug/orbit-ghost

# fusermount debug
fusermount -d /tmp/orbit_ghost_mount
```

## Code Style

### Rust Style Guidelines

Follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/):

- Use `snake_case` for variables and functions
- Use `CamelCase` for types and traits
- Max line length: 100 characters
- Use `rustfmt` for formatting

### Naming Conventions

```rust
// Good
pub struct GhostFile { ... }
fn ensure_block_available(&self, file_id: &str) { ... }
const BLOCK_SIZE: u64 = 1024 * 1024;

// Bad
pub struct ghostfile { ... }  // Wrong case
fn EnsureBlockAvailable(&self, FileID: &str) { ... }  // Wrong case
const blockSize: u64 = 1024 * 1024;  // Wrong case
```

### Error Handling

```rust
// Prefer Result over panics
fn fetch_block(&self, id: &str) -> Result<Vec<u8>, Error> {
    // ...
}

// Use ? operator for propagation
fn process() -> Result<(), Error> {
    let data = fetch_block("123")?;
    Ok(())
}

// Avoid unwrap() in library code
// Good:
let value = map.get(&key).ok_or(Error::NotFound)?;

// Bad:
let value = map.get(&key).unwrap();  // Panics!
```

### Documentation

```rust
/// Ensures a block is available in the local cache.
///
/// This method blocks the calling thread until the requested block
/// has been downloaded and written to the cache directory.
///
/// # Arguments
///
/// * `file_id` - The unique identifier for the file
/// * `block_index` - The zero-indexed block number
///
/// # Behavior
///
/// 1. Checks if block exists in cache (fast path)
/// 2. Sends priority request to wormhole transport
/// 3. Blocks until block appears in cache
///
/// # Panics
///
/// Currently panics if the wormhole channel is closed.
///
/// # Examples
///
/// ```
/// entangler.ensure_block_available("file_123", 42);
/// // Block 42 is now in /tmp/orbit_cache/file_123_42.bin
/// ```
pub fn ensure_block_available(&self, file_id: &str, block_index: u64) {
    // Implementation...
}
```

## Adding Features

### Example: Add Prefetching

**Goal:** Fetch next 3 blocks after a read.

**Step 1:** Modify `entangler.rs`

```rust
// Add configuration
pub struct EntanglerConfig {
    pub prefetch_count: usize,
}

impl Entangler {
    pub fn new(priority_tx: Sender<BlockRequest>, config: EntanglerConfig) -> Self {
        Self {
            priority_tx,
            waiting_rooms: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    pub fn ensure_block_available(&self, file_id: &str, block_index: u64) {
        // Fetch requested block
        self.fetch_block_internal(file_id, block_index, true);

        // Prefetch next N blocks
        for i in 1..=self.config.prefetch_count {
            self.fetch_block_internal(file_id, block_index + i as u64, false);
        }
    }

    fn fetch_block_internal(&self, file_id: &str, block_index: u64, blocking: bool) {
        // Existing logic, with blocking parameter
    }
}
```

**Step 2:** Update `main.rs`

```rust
let config = EntanglerConfig {
    prefetch_count: 3,
};
let entangler = Arc::new(Entangler::new(priority_tx, config));
```

**Step 3:** Add tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefetch() {
        // Setup mock entangler
        // Ensure block 0 is fetched
        // Verify blocks 1, 2, 3 are queued
    }
}
```

**Step 4:** Document in changelog and user guide.

### Example: Add Configuration File Support

**Goal:** Load settings from `orbit-ghost.toml`.

**Step 1:** Add dependency to `Cargo.toml`

```toml
[dependencies]
config = "0.14"
```

**Step 2:** Create config module `src/config.rs`

```rust
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub mount_point: String,
    pub cache_dir: String,
    pub block_size: u64,
    pub prefetch_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mount_point: "/tmp/orbit_ghost_mount".into(),
            cache_dir: "/tmp/orbit_cache".into(),
            block_size: 1024 * 1024,
            prefetch_count: 3,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::File::from(path))
            .build()?
            .try_deserialize()
    }
}
```

**Step 3:** Use in `main.rs`

```rust
mod config;

fn main() {
    let config = match config::Config::load(Path::new("orbit-ghost.toml")) {
        Ok(cfg) => cfg,
        Err(_) => config::Config::default(),
    };

    println!("Mount point: {}", config.mount_point);
    // Use config.block_size, etc.
}
```

## Performance Profiling

### CPU Profiling with Flamegraph

```bash
# Install flamegraph
cargo install flamegraph

# Profile
cargo flamegraph --bin orbit-ghost

# Opens flamegraph.svg in browser
# Red/yellow bars show hot code paths
```

### Memory Profiling with Valgrind

```bash
# Install valgrind
sudo apt-get install valgrind  # Linux only

# Build with debug symbols
cargo build

# Profile
valgrind --tool=massif ./target/debug/orbit-ghost

# Analyze
ms_print massif.out.<pid>
```

### Benchmarking

Create `benches/block_fetch.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_block_calculation(c: &mut Criterion) {
    c.bench_function("block_range_calc", |b| {
        b.iter(|| {
            let offset = black_box(52428800);
            let size = black_box(1024);
            let block_size = black_box(1048576);

            let start_block = offset / block_size;
            let end_block = (offset + size as u64) / block_size;

            (start_block, end_block)
        });
    });
}

criterion_group!(benches, benchmark_block_calculation);
criterion_main!(benches);
```

Run:
```bash
cargo bench
```

## Testing Strategy

### Unit Tests

Test individual functions in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_range_single() {
        let offset = 1024;
        let size = 512;
        let block_size = 1024;

        let start = offset / block_size;
        let end = (offset + size) / block_size;

        assert_eq!(start, 1);
        assert_eq!(end, 1);  // Same block
    }

    #[test]
    fn test_block_range_spanning() {
        let offset = 1024;
        let size = 1536;
        let block_size = 1024;

        let start = offset / block_size;
        let end = (offset + size) / block_size;

        assert_eq!(start, 1);
        assert_eq!(end, 2);  // Spans 2 blocks
    }
}
```

### Integration Tests

Create `tests/integration_test.rs`:

```rust
use std::process::{Command, Child};
use std::thread;
use std::time::Duration;

struct GhostFS {
    process: Child,
}

impl GhostFS {
    fn start() -> Self {
        let process = Command::new("./target/debug/orbit-ghost")
            .spawn()
            .expect("Failed to start orbit-ghost");

        thread::sleep(Duration::from_secs(2));  // Wait for mount

        Self { process }
    }

    fn stop(mut self) {
        self.process.kill().expect("Failed to kill process");
        Command::new("fusermount")
            .args(&["-u", "/tmp/orbit_ghost_mount"])
            .output()
            .ok();
    }
}

#[test]
#[ignore]  // Run with: cargo test -- --ignored
fn test_file_listing() {
    let ghost = GhostFS::start();

    let output = Command::new("ls")
        .arg("/tmp/orbit_ghost_mount")
        .output()
        .expect("Failed to list directory");

    assert!(output.status.success());
    let files = String::from_utf8_lossy(&output.stdout);
    assert!(files.contains("visionary_demo.mp4"));

    ghost.stop();
}
```

### Fuzzing

Create `fuzz/fuzz_targets/fuzz_read.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }

    let offset = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let size = u32::from_le_bytes(data[8..12].try_into().unwrap());

    // Test block calculation doesn't panic
    let block_size = 1048576u64;
    let start_block = offset / block_size;
    let end_block = (offset + size as u64) / block_size;

    // Validate results
    assert!(start_block <= end_block);
});
```

Run:
```bash
cargo install cargo-fuzz
cargo fuzz run fuzz_read
```

## Release Process

### Version Bump

1. Update version in `Cargo.toml`:
   ```toml
   version = "0.2.0"
   ```

2. Update `CHANGELOG.md`:
   ```markdown
   ## [0.2.0] - 2024-02-15
   ### Added
   - Prefetching support
   - Configuration file loading
   ### Fixed
   - Block boundary calculation bug
   ```

3. Commit:
   ```bash
   git commit -am "chore: bump version to 0.2.0"
   ```

### Tag Release

```bash
git tag -a v0.2.0 -m "Release version 0.2.0"
git push origin v0.2.0
```

### Build Release Artifacts

```bash
# Build optimized binary
cargo build --release

# Strip symbols (Linux)
strip target/release/orbit-ghost

# Create tarball
tar -czf orbit-ghost-v0.2.0-linux-x86_64.tar.gz \
  -C target/release orbit-ghost \
  -C ../../ README.md INSTALL.md GUIDE.md LICENSE
```

### Publish to crates.io (Future)

```bash
cargo login
cargo publish
```

## Continuous Integration

### GitHub Actions Workflow

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: clippy, rustfmt

      - name: Install FUSE
        run: sudo apt-get install -y fuse3 libfuse3-dev pkg-config

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Build
        run: cargo build --verbose

      - name: Run tests
        run: cargo test --verbose
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on:
- Code of conduct
- Pull request process
- Issue reporting

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [FUSE Documentation](https://www.kernel.org/doc/html/latest/filesystems/fuse.html)
- [fuser crate docs](https://docs.rs/fuser/)
- [Orbit Architecture](ARCHITECTURE.md)

## Getting Help

- **Questions:** [GitHub Discussions](https://github.com/saworbit/orbit/discussions)
- **Bugs:** [GitHub Issues](https://github.com/saworbit/orbit/issues)
- **Email:** shaneawall@gmail.com
