# Changelog

All notable changes to Orbit GhostFS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for v0.2.0
- Replace polling loop with Condvar::wait() for efficient synchronization
- Add timeout to ensure_block_available() (prevent indefinite hangs)
- Implement thread pool for parallel block downloads
- Add LRU cache eviction policy
- Configuration file support (orbit-ghost.toml)
- Command-line argument parsing
- Heuristic prefetching (sequential access detection)

### Planned for v0.3.0
- Integration with Magnetar manifest catalog
- Real Orbit backend protocol support
- TLS encryption for backend communication
- HMAC block verification
- Resumable block fetches

## [0.1.0] - 2024-12-29

Initial proof-of-concept release demonstrating quantum entanglement filesystem.

### Added

#### Core Functionality
- FUSE filesystem implementation using `fuser` crate
- Block-level just-in-time data fetching (1 MB blocks)
- Priority queue for user-initiated reads
- Simulated wormhole transport layer
- In-memory manifest projection (instant directory listings)
- Local block caching in `/tmp/orbit_cache/`

#### Modules
- `inode.rs` - Virtual file representation and FUSE attribute mapping
- `entangler.rs` - Quantum coordination logic, priority signaling
- `fs.rs` - FUSE operations (lookup, getattr, readdir, read)
- `main.rs` - System bootstrap, mount handling, wormhole thread

#### Developer Tools
- Demo script (`demo_quantum.sh`) showcasing on-demand fetching
- Comprehensive logging via `env_logger`
- DashMap for concurrent inode storage
- Crossbeam channels for priority communication

#### Documentation
- README.md - Project overview
- ARCHITECTURE.md - Technical deep dive (architecture, data flow, design decisions)
- INSTALL.md - Platform-specific installation instructions
- GUIDE.md - User manual with usage examples
- DEVELOPMENT.md - Developer setup and contribution guide
- ROADMAP.md - Long-term vision and milestones
- FAQ.md - Common questions and troubleshooting
- CONTRIBUTING.md - Contribution guidelines
- WINDOWS.md - Windows platform notes and future plans

#### Platform Support
- ✅ Linux (Ubuntu, Debian, Fedora, Arch, RHEL) via libfuse3
- ⚠️ macOS (experimental) via macFUSE
- ❌ Windows (not yet supported)

### Implementation Details

#### FUSE Operations
- `lookup()` - Instant file resolution from in-memory manifest
- `getattr()` - Metadata retrieval without network I/O
- `readdir()` - Directory listing from manifest (no network latency)
- `read()` - Triggers quantum entanglement for block fetching

#### Synchronization
- Polling-based block availability check (10ms intervals)
- Mutex-protected waiting rooms for concurrent requests
- Crossbeam unbounded channel for priority signaling

#### Caching
- Unlimited cache growth in `/tmp/orbit_cache/`
- Block naming: `{file_id}_{block_index}.bin`
- No eviction policy (manual cleanup required)

### Known Limitations

#### Stability
- No timeout handling - read operations can hang indefinitely on network issues
- No error recovery for corrupted blocks
- Single-threaded download (bottleneck for concurrent requests)
- Polling loop consumes CPU even when idle

#### Configuration
- Hardcoded mount point (`/tmp/orbit_ghost_mount`)
- Hardcoded cache directory (`/tmp/orbit_cache`)
- Hardcoded block size (1 MB)
- No CLI arguments or config file support

#### Platform
- Windows not supported (FUSE not available)
- macOS requires kernel extension approval (user friction)

#### Features
- Read-only filesystem (no write support)
- No prefetching (every block fetch incurs latency)
- No cache size limits (can fill disk)
- Simulated backend (not connected to real storage)

### Performance Characteristics

**Benchmarks (simulated 500ms network latency):**
- Directory listing: < 10ms (instant from manifest)
- First block read: ~520ms (network fetch + cache write)
- Cached block read: < 5ms (disk I/O)
- Sequential 50 MB read: ~26 seconds (50 blocks × 520ms)

**Compared to traditional transfer:**
- Random access (1% of file): **50-100x faster**
- Full sequential read: **20-30% slower**

### Dependencies

```toml
fuser = "0.14"             # FUSE bindings
libc = "0.2"               # syscall constants
tokio = "1"                # Async runtime (not yet used)
serde = "1"                # Serialization
serde_json = "1"           # JSON handling
anyhow = "1.0"             # Error handling
log = "0.4"                # Logging facade
env_logger = "0.10"        # Logger implementation
dashmap = "5.5"            # Concurrent HashMap
crossbeam-channel = "0.5"  # MPMC channels
parking_lot = "0.12"       # Fast mutexes
```

### Breaking Changes

None (initial release).

### Migration Guide

N/A (initial release).

### Contributors

- Shane Wall (@saworbit) - Initial implementation

---

## Version History

| Version | Release Date | Status | Notes |
|---------|--------------|--------|-------|
| 0.1.0 | 2024-01-15 | ✅ Released | Proof of concept |
| 0.2.0 | Q2 2024 | 🚧 Planned | Production hardening |
| 0.3.0 | Q3 2024 | 📋 Planned | Orbit integration |
| 0.4.0 | Q4 2024 | 📋 Planned | Advanced features |
| 0.5.0 | Q1 2025 | 📋 Planned | Platform expansion |
| 1.0.0 | Q2 2025 | 📋 Planned | Enterprise launch |

---

## Upgrade Instructions

### From Source (Development)

Since this is the initial release, there are no upgrade instructions yet.

**Future upgrades:**
```bash
cd orbit-ghost
git pull origin main
cargo build --release
# Follow version-specific migration notes below
```

### From Binary (Future)

```bash
# Download latest release
wget https://github.com/saworbit/orbit/releases/download/v0.2.0/orbit-ghost-linux-x86_64.tar.gz

# Extract
tar -xzf orbit-ghost-linux-x86_64.tar.gz

# Replace binary
sudo cp orbit-ghost /usr/local/bin/

# Verify
orbit-ghost --version
```

---

## Deprecation Notices

### v0.1.0

None.

### Planned Deprecations

**v0.2.0:**
- Hardcoded configuration constants (replaced by config file)
- Polling-based synchronization (replaced by Condvar)

**v0.3.0:**
- Simulated wormhole transport (replaced by real backend)

---

## Security Advisories

### v0.1.0

No security vulnerabilities known. However:

**⚠️ Not production-ready:**
- No authentication/authorization
- No encryption (simulated backend only)
- Cache files have 0600 permissions (user-readable only)

**⚠️ Caution:**
- Do not expose mount point to untrusted users
- Do not use for sensitive data without additional encryption

### Future Security Enhancements (v0.3.0+)

- TLS 1.3 for all backend communication
- HMAC verification of blocks
- Optional cache encryption
- Integration with Orbit access control

---

## Compatibility Matrix

### Rust Version

| Orbit GhostFS | Minimum Rust | Recommended |
|---------------|--------------|-------------|
| 0.1.0 | 1.70.0 | 1.75.0+ |

### FUSE Version

| Platform | Minimum | Recommended |
|----------|---------|-------------|
| Linux | libfuse3 3.0.0 | 3.10.0+ |
| macOS | macFUSE 4.0.0 | 4.4.0+ |

### Operating Systems

| OS | Version | Status |
|----|---------|--------|
| Ubuntu | 20.04+ | ✅ Supported |
| Debian | 11+ | ✅ Supported |
| Fedora | 35+ | ✅ Supported |
| RHEL/CentOS | 8+ | ✅ Supported |
| Arch Linux | Rolling | ✅ Supported |
| macOS | 11+ (Big Sur) | ⚠️ Experimental |
| Windows 10/11 | N/A | ❌ Not supported |

---

## Acknowledgments

### v0.1.0

- **FUSE:** The kernel interface that makes this possible
- **fuser crate:** Safe Rust bindings by @cberner
- **DashMap:** Lock-free concurrent HashMap by @xacrimon
- **Crossbeam:** Concurrency tools by @crossbeam-rs
- **Orbit community:** Feedback and testing

---

## Links

- **Repository:** https://github.com/saworbit/orbit
- **Issues:** https://github.com/saworbit/orbit/issues
- **Discussions:** https://github.com/saworbit/orbit/discussions
- **Releases:** https://github.com/saworbit/orbit/releases
- **Documentation:** https://github.com/saworbit/orbit/tree/main/orbit-ghost

---

## Notes

### Version Numbering

We follow [Semantic Versioning](https://semver.org/):
- **MAJOR:** Incompatible API changes
- **MINOR:** New functionality (backward compatible)
- **PATCH:** Bug fixes (backward compatible)

**Pre-1.0 versions:** API may change between minor versions.

### Release Cadence

- **v0.x:** Quarterly releases (feature-driven)
- **v1.0+:** Scheduled releases with LTS support

### Support Policy

- **Latest version:** Full support (bug fixes, features)
- **Previous minor version:** Security fixes only (6 months)
- **Older versions:** Community support (best effort)

---

*Last updated: 2024-01-15*
