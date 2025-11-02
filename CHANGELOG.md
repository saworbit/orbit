# Changelog

All notable changes to Orbit will be documented in this file.

## [0.4.1] - 2025-11-02

### Added
- **Enhanced Resume System** - Smart resume with chunk-level verification
  - `ResumeInfo` now tracks verified chunk digests using BLAKE3
  - Window ID tracking for manifest-based verification
  - Four resume strategies: Resume, Revalidate, Restart, StartFresh
  - File metadata validation (mtime, size) to detect external modifications
  - Smart decision logic compares resume info against destination state
  - Progress events for resume decisions (ResumeDecision events)
  - New documentation: [docs/RESUME_SYSTEM.md](docs/RESUME_SYSTEM.md)

- **S3 Object Versioning** - Full versioning support (`src/protocol/s3/versioning.rs`)
  - List all versions of an object with metadata
  - Download specific versions by version ID
  - Delete specific versions or create delete markers
  - Restore previous versions to current
  - Compare versions (size, timestamps, etags)
  - Enable/disable versioning on buckets
  - Suspend versioning without deleting version history
  - `VersioningOperations` trait for extensibility

- **S3 Batch Operations** - Efficient batch processing (`src/protocol/s3/batch.rs`)
  - Batch delete up to 1,000 objects per operation
  - Batch copy with concurrent transfers
  - Batch metadata updates
  - Batch storage class changes (STANDARD, IA, GLACIER, etc.)
  - Batch tagging operations
  - Rate limiting with token bucket algorithm
  - Configurable concurrency control via semaphores
  - Comprehensive error tracking per operation
  - `BatchOperations` trait for custom batch operations

- **Enhanced Error Recovery** - Production-grade retry logic (`src/protocol/s3/recovery.rs`)
  - Retry policies with exponential backoff
  - Circuit breaker pattern to prevent cascading failures
  - Jitter to prevent thundering herd
  - Error classification (retryable vs fatal)
  - Preset policies: fast, slow, network-optimized
  - Configurable max attempts and delays
  - Integration with S3 operations

- **Progress Callbacks** - Real-time UI integration (`src/protocol/s3/progress.rs`)
  - `ProgressEvent` enum for transfer lifecycle events
  - `ProgressReporter` using tokio unbounded channels
  - `ThroughputTracker` for transfer rate calculation
  - ETA (estimated time remaining) calculation
  - Transfer statistics collection
  - `ProgressAggregator` for multiple reporters
  - Batch progress tracking
  - Support for pause/resume operations

- **Magnetar Resilience Module** - Fault-tolerant data access patterns (`crates/magnetar/src/resilience/`)
  - **Circuit Breaker** - Three-state pattern (Closed → Open → HalfOpen) with automatic recovery
    - Configurable failure and success thresholds
    - Exponential backoff with jitter
    - Cooldown period before recovery testing
    - Smart retry logic for transient vs permanent errors
  - **Connection Pool** - Generic connection management with health checking
    - Configurable pool size, idle timeout, and max lifetime
    - Automatic connection reuse and cleanup
    - Health checking via `ConnectionFactory` trait
    - Pool statistics and monitoring
  - **Rate Limiter** - Token bucket rate limiting
    - Configurable requests per period
    - Optional governor crate integration for advanced features
  - Thread-safe async/await with full Tokio integration
  - Custom error types with transient/permanent classification
  - Comprehensive unit and integration tests (27 tests)
  - S3 and SMB integration examples
  - Full documentation with usage patterns
  - Features: `resilience` (default), `resilience-governor`, `s3-integration`

### Changed
- **License** - Migrated to Apache License 2.0
  - Updated LICENSE file from dual-license (MIT/Commercial) to Apache 2.0
  - Updated all Cargo.toml files with `license = "Apache-2.0"`
  - Updated README.md with Apache 2.0 badge and license section
  - Updated CONTRIBUTING.md and PR templates
  - Removed obsolete commercial license references

- **S3 Module Structure** - Expanded protocol support
  - New modules: `versioning`, `batch`, `recovery`, `progress`
  - All modules include comprehensive documentation and examples
  - Async-first design using tokio
  - Trait-based APIs for extensibility

### Fixed
- Documentation link in S3_USER_GUIDE.md (PROTOCOL_GUIDE.md path corrected)
- Resume system now handles filesystem timestamp precision (2-second tolerance)

---

## [0.4.0] - 2025-10-14

### Added
- **Protocol abstraction layer** - Unified interface for multiple storage backends
  - New `src/protocol/` module with `StorageBackend` trait
  - URI parsing support for protocol detection (e.g., `smb://server/share/path`)
  - Local filesystem backend fully implemented
  - SMB/CIFS backend (experimental/stub implementation)
- **Protocol module exports** - `Protocol` and `StorageBackend` now public API
- **URI support** - Parse URIs like `smb://user:pass@server/share/path`

### Changed
- Project structure now includes `src/protocol/` directory
- Library API extended with protocol support

### Experimental
- SMB/CIFS support added but not production-ready
  - Stub implementation for testing architecture
  - Full implementation planned for v0.4.1

---

## [0.3.1] - 2025-10-12

### Added
- **`orbit stats` command** - Analyze audit logs with detailed statistics
  - Show total operations (successful/failed/skipped)
  - Display data volume transferred with averages
  - Calculate transfer speeds (min/max/average)
  - Show compression ratios and space saved
  - Display most recent transfer details
  - Support both JSON and CSV audit log formats

### Fixed
- Removed unused import warnings in stats module

---

## [0.3.0] - 2025-10-12

### Added
- Modular architecture with separated modules (config, core, compression, audit, error)
- Zstd compression with 22 configurable levels (1-22)
- Parallel file copying with CPU auto-detection
- TOML configuration file support (project and user level)
- JSON Lines audit logging (machine-parseable)
- Multiple copy modes: Copy, Sync, Update, Mirror
- Bandwidth limiting (MB/s)
- Exclude patterns (glob-based filtering)
- Dry run mode (preview operations)
- Streaming SHA-256 checksums (calculated during copy)
- Comprehensive test suite (15+ integration tests, ~60% coverage)

### Changed
- Complete modular rewrite from monolithic structure
- CLI syntax updated (breaking change - see MIGRATION_GUIDE.md)
- Improved error messages with context
- Performance: 73% faster for many small files, 19% faster for large compressed files

### Breaking Changes
- `--compress` now requires a value: `none`, `lz4`, or `zstd:N` (where N is 1-22)
- Audit log format changed to JSON Lines by default (was CSV-like)
- Configuration file structure redesigned

## [0.2.0] - 2025-06-02

### Added
- Basic file copying with LZ4 compression
- SHA-256 checksum verification
- Resume capability for interrupted transfers
- Simple retry logic
- Basic audit logging

## [0.1.0] - 2025-05-01

### Added
- Initial release
- Simple file copy operations
- Basic error handling