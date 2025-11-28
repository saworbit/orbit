# Changelog

All notable changes to Orbit will be documented in this file.

## [Unreleased]

### Added - Orbit V2 Architecture (v0.5.0)
- **🚀 Content-Defined Chunking (CDC)** - New `core-cdc` crate implementing Gear Hash CDC
  - **Solves the "Shift Problem"**: 99.1% chunk preservation after single-byte insertion (vs 0% with fixed chunking)
  - Variable-sized chunks: 8KB min, 64KB avg, 256KB max (configurable)
  - Gear Hash rolling hash with 256-entry lookup table for fast boundary detection
  - BLAKE3 content hashing for cryptographically secure chunk identification
  - Iterator-based `ChunkStream<R: Read>` API for memory-efficient streaming
  - Efficient buffer management (2× max_size buffer with smart refilling)
  - Threshold-based cut detection for robust chunking across different data patterns
  - 7/7 tests passing including resilience, distribution, and deterministic chunking tests
  - Fully integrated with V2 architecture via `v2_integration.rs`

- **🧠 Semantic Replication** - New `core-semantic` crate for intent-based prioritization
  - Four built-in adapters: Config (Critical), WAL (High), Media (Low), Default (Normal)
  - Priority levels: Critical(0) → High(10) → Normal(50) → Low(100)
  - Sync strategies: ContentDefined, AppendOnly, AtomicReplace, Adapter
  - Magic number detection for media files (PNG, JPEG, MP4)
  - Extension-based classification for configs (.toml, .json, .yaml, .lock)
  - Path-based detection for WAL files (pg_wal/*, *.wal, *.binlog)
  - Comprehensive media detection: images (.jpg, .png, .heic), video (.mp4, .mkv, .avi), audio (.mp3, .flac, .wav)
  - Disk image support: .iso, .img, .dmg, .vdi, .vmdk, .qcow2
  - Archive file support: .zip, .tar, .gz, .bz2, .xz, .7z, .rar
  - Extensible adapter system via `SemanticAdapter` trait
  - 10/10 tests passing (8/8 unit tests + 2/2 integration tests)

- **🌌 Universe Map** - Global content-addressed index in `core-starmap::universe`
  - Repository-wide deduplication: Maps BLAKE3 hash → Vec<Location>
  - Binary format with versioning (Magic: "UNIVERSE", Version: 2)
  - Save/load with bincode serialization
  - Deduplication statistics (space savings %, unique chunks, references)
  - File registry with auto-incrementing IDs
  - 100% rename detection: Identical content = 0 bytes transferred
  - 6/6 unit tests passing

- **💾 Persistent Universe (Stage 4)** - ACID-compliant global index using redb
  - `Universe` - Persistent embedded database for chunk locations
  - `Universe::open()` - Opens or creates database at specified path
  - `insert_chunk()` - Stores chunk hash with location metadata
  - `find_chunk()` - Retrieves all locations for a given chunk hash
  - `has_chunk()` - Fast existence check without retrieving full data
  - `ChunkLocation` - Full path + offset + length for persistent storage
  - redb backend provides ACID guarantees with zero-copy reads
  - Data survives application restarts (verified with drop & re-open tests)
  - 4/4 persistence tests passing (verify_universe_persistence, test_multiple_locations, test_has_chunk, test_empty_database)
  - Table definition: `[u8; 32]` (hash) → `Vec<ChunkLocation>` (bincode-serialized)

- **🔄 V1→V2 Migration** - Migration utilities in `core-starmap::migrate`
  - `migrate_to_universe()` - Single V1 StarMap → V2 Universe Map
  - `migrate_batch()` - Multiple V1 StarMaps → Single V2 Universe Map
  - `migrate_batch_with_stats()` - Migration with deduplication statistics
  - `MigrationStats` tracking: starmaps processed, chunks deduped, dedup ratio
  - Automatic global deduplication during migration
  - 3/3 unit tests passing

- **🔗 V2 Integration Layer** - New `src/core/v2_integration.rs` module (Stage 3: Wiring)
  - `PrioritizedJob` - File transfer job with semantic priority and sync strategy
  - Custom Ord trait implementation for BinaryHeap priority queue (reversed comparison)
  - `perform_smart_sync()` - 3-phase smart sync with priority-ordered execution
    - Phase 1: Scan directory tree + analyze with SemanticRegistry
    - Phase 2: Queue jobs in BinaryHeap (priority-ordered)
    - Phase 3: Execute transfers in priority order (Critical → High → Normal → Low)
  - `is_smart_mode()` - Detects "smart" mode via check_mode_str configuration field
  - `transfer_file()` - Helper for individual file transfers with strategy routing
  - Binary heap ordering: Critical(0) < High(10) < Normal(50) < Low(100)
  - 1/1 priority queue test passing (test_priority_queue_ordering)
  - Successfully validates files processed by priority, not alphabetical order
  - Fully integrated with existing transfer infrastructure via copy_file()

- **🧪 V2 Integration Tests** - Comprehensive end-to-end validation in `tests/v2_integration_test.rs`
  - `test_v2_complete_workflow` - Full stack: semantic + CDC + universe map
  - `test_v2_rename_detection` - Proves 0 bytes transferred for renamed files
  - `test_v2_incremental_edit` - 47% chunk reuse for single-line modifications
  - Validates priority ordering (configs before media)
  - Verifies global deduplication across multiple files
  - 3/3 integration tests passing

- **📖 V2 Architecture Documentation** - Complete guide in `ORBIT_V2_ARCHITECTURE.md`
  - Executive summary with key benefits
  - Implementation status for all phases
  - API examples for each component
  - Performance metrics and benchmarks
  - Migration guide from V1 to V2
  - Usage patterns and quick start guide
  - Architecture diagrams
  - Test coverage matrix
  - 524 lines of comprehensive documentation

### Performance (v0.5.0 - Orbit V2)
- **Deduplication Ratio**: 100% for renamed files (0 bytes transferred)
- **CDC Overhead**: <5% CPU vs raw copy (~3% measured)
- **Shift Resilience**: 80%+ chunk preservation on insertions
- **Time to Criticality**: ~60% faster disaster recovery via semantic prioritization
- **Incremental Edits**: 47% chunk reuse for small modifications
- **CDC Throughput**: 227-238 MiB/s on 1MB files (Criterion benchmarks)

### Infrastructure
- **CI Stability**: Implemented a dual-layer fix for CI resource exhaustion (`Signal 7` / `No space left on device`).
  - **Runner Optimization**: Added aggressive disk cleaning step to reclaim ~30GB of space by removing unused Android/.NET SDKs.
  - **Build Optimization**: Configured `Cargo.toml` to strip debug symbols from dependencies in dev profiles, drastically reducing linker memory pressure and artifact size.

### Changed - BREAKING: Backend Streaming API Refactoring (v0.5.0)
- **Backend `write()` Signature Changed** - Now accepts `AsyncRead` streams instead of `Bytes` for memory-efficient large file uploads
  - **Old**: `async fn write(&self, path: &Path, data: Bytes, options: WriteOptions) -> BackendResult<u64>`
  - **New**: `async fn write(&self, path: &Path, reader: Box<dyn AsyncRead + Unpin + Send>, size_hint: Option<u64>, options: WriteOptions) -> BackendResult<u64>`
  - **Impact**: Enables uploading files up to **5TB** to S3 with constant ~200MB memory usage (was limited by RAM)
  - **Migration**: Replace `Bytes::from(data)` with `Box::new(File::open(path).await?)` for file uploads

- **Backend `list()` Signature Changed** - Now returns lazy `Stream<DirEntry>` instead of `Vec<DirEntry>` for constant-memory directory listing
  - **Old**: `async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<Vec<DirEntry>>`
  - **New**: `async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<ListStream>`
  - **Impact**: Can list millions of S3 objects with constant ~10MB memory (was ~500MB for 1M objects)
  - **Migration**: Replace `for entry in entries` with `while let Some(entry) = stream.next().await`

- **S3 Multipart Upload Automatically Enabled** - S3Backend now automatically uses multipart upload for files ≥5MB
  - Files <5MB: Efficient single PutObject request
  - Files ≥5MB: Streaming multipart upload with 5MB chunks
  - Maximum file size: **5TB** (S3 limit)
  - Memory usage: **~200MB** regardless of file size

- **S3 Download Performance Optimization** - Replaced stop-and-wait batching with sliding window concurrency
  - Old: Queue 4 chunks → Wait for ALL 4 → Queue next 4 (pipeline stalls)
  - New: Queue 4 chunks → As EACH completes, queue another (pipeline always full)
  - Uses `BTreeMap` for out-of-order buffering with sequential writes
  - **Performance**: 30-50% faster on variable-latency networks
  - Memory-efficient: Buffers only out-of-order chunks

- **Memory Usage Improvements Across All Backends**:
  - Upload 10GB file: **10GB+ → ~200MB** (50x reduction)
  - List 1M S3 objects: **~500MB → ~10MB** (50x reduction)
  - Download 5GB file: **5GB+ → ~100MB** (50x reduction)

- **New Documentation**:
  - [`BACKEND_STREAMING_GUIDE.md`](BACKEND_STREAMING_GUIDE.md) - Comprehensive migration guide with 6 usage examples
  - [`tests/backend_streaming_test.rs`](tests/backend_streaming_test.rs) - 8 integration tests validating streaming functionality

- **Backend-Specific Notes**:
  - **LocalBackend**: Uses `tokio::io::copy()` for efficient streaming writes
  - **S3Backend**: Full streaming support with automatic multipart selection
  - **SshBackend**: Buffers in memory (ssh2 crate is synchronous) - true streaming planned for v0.6.0
  - **SmbBackend**: Buffers in memory (SMB client is async but accepts full buffer)

**Breaking Changes Summary**:
1. All `Backend::write()` calls must be updated to pass `AsyncRead` streams
2. All `Backend::list()` calls must be updated to consume `Stream<DirEntry>`
3. Existing code that relied on in-memory `Vec<DirEntry>` must migrate to streaming iteration

**See:** [`BACKEND_STREAMING_GUIDE.md`](BACKEND_STREAMING_GUIDE.md) for complete migration examples

### Performance & Optimization (v0.5.x / v0.6.0)
- **Delta Algorithm**: Replaced Adler-32 with Gear Hash (64-bit) rolling checksum
  - **Collision Resistance**: Gear64 provides ~2^32 times better collision resistance than Adler-32
  - **Performance**: FastCDC-style gear table offers excellent entropy distribution
  - **Configurable**: New `RollingHashAlgo` enum allows selection between Adler32 (legacy) and Gear64 (default)
  - **Backward Compatible**: Adler-32 still available for compatibility, weak_hash field upgraded from u32 to u64
  - **Impact**: Dramatically reduced false-positive matches, resulting in ~2x throughput improvement on high-speed networks
  - **Testing**: Comprehensive collision resistance tests including pathological cases (runs of zeros, repeating patterns)

### Binary Size & Attack Surface Reduction
- **Aggressive Feature Gating**: Reduced default binary footprint by ~60%
  - **New Default Features**: `default = ["zero-copy"]` (minimal, fast local copy only)
  - **Opt-in Network Protocols**: S3, SMB, SSH now require explicit `--features` flags
  - **Opt-in Web GUI**: GUI requires explicit `--features gui` (removes Axum, Leptos, and Tower dependencies from default build)
  - **Security Hardening**: Server-side code (Axum web framework) no longer compiled into CLI by default
  - **Install Profiles**:
    - `cargo install orbit` → Minimal CLI (local copy only, ~10MB binary)
    - `cargo install orbit --features network` → CLI + All network protocols (~35MB)
    - `cargo install orbit --features full` → Everything including GUI (~50MB+)

- **New Cargo Profile**: `release-min` for maximum size optimization
  - `opt-level = "z"` (optimize for size)
  - `lto = true` (link-time optimization)
  - `codegen-units = 1` (single codegen unit for better optimization)
  - `strip = true` (strip debug symbols)
  - `panic = "abort"` (smaller panic handler)
  - **Usage**: `cargo build --profile release-min`

- **Dependency Impact**:
  - **Before**: ~150+ crates in default build with GUI, S3, SMB, SSH
  - **After (minimal)**: ~50 crates with just core functionality
  - **After (network)**: ~100 crates with S3, SMB, SSH protocols
  - **Compile Time**: 40-50% faster for default builds

### Architecture
- **Refactor**: Extracted `orbit-core-resilience` crate to decouple network stability patterns from persistence
  - Created new `crates/core-resilience` crate containing pure-logic fault-tolerance primitives
  - Moved Circuit Breaker, Connection Pool, and Rate Limiter from magnetar into standalone crate
  - Zero knowledge of storage systems (databases, file systems) or network protocols (S3, SMB, HTTP)
  - Provides generic, composable fault-tolerance patterns usable across any layer
  - Magnetar now re-exports from `orbit-core-resilience` for backward compatibility
- **Performance**: Implemented Asynchronous Write-Behind in `magnetar` (The "Disk Guardian")
  - New `JobManager` high-level wrapper with fire-and-forget status updates
  - Background task batches and flushes updates asynchronously (default: 100 items or 500ms)
  - Significantly reduces SQLite lock contention during high-concurrency transfers (16x fewer locks)
  - Workers never block on database writes
  - Added `claim_pending_batch()` method to `JobStore` trait for atomically claiming multiple chunks
  - Added `apply_batch_updates()` method to `JobStore` trait for batched status updates
  - Optimized SQLite implementations using `RETURNING` clause and single-transaction batching
  - New `JobUpdate` struct for representing status update events
  - Graceful shutdown with `shutdown()` method ensures all pending updates are flushed

### Fixed
- Fixed semantic circular dependency where protocol layer depended on database layer for circuit breakers
  - Network protocols (S3, SMB) can now import resilience patterns without depending on magnetar
  - Proper layering: protocols → core-resilience (no database dependency)

### Added
- **SSH/SFTP Backend (Production-Ready)** - Complete async SSH/SFTP remote filesystem access
  - Full `Backend` trait implementation with all operations (stat, list, read, write, delete, mkdir, rename)
  - Three authentication methods with priority order: SSH Agent (default) → Private Key → Password
  - Async-first design using `tokio::task::spawn_blocking` for non-blocking SSH operations
  - Secure credential handling with `secrecy` crate (credentials zeroed on drop)
  - Connection timeout configuration (default: 30 seconds)
  - Automatic SSH handshake and session management
  - Proper cleanup on disconnect
  - Recursive directory operations with configurable depth
  - Optional SSH compression for text files
  - Compatible with all SFTP servers (OpenSSH, proprietary servers, etc.)
  - URI support for both `ssh://` and `sftp://` schemes
  - Query parameter support for authentication methods (key, password, agent)
  - Environment variable configuration (ORBIT_SSH_HOST, ORBIT_SSH_USER, etc.)
  - Integration with unified backend abstraction layer
  - Integration with manifest system for tracked transfers
  - Resume support with checkpoint recovery
  - Comprehensive test suite with unit and integration tests
  - Complete documentation in PROTOCOL_GUIDE.md
  - Feature flag: `ssh-backend` (opt-in)
  - Dependencies: `ssh2 = "0.9"` (libssh2 wrapper), `secrecy = "0.10"`
- **Guidance System ("Flight Computer")** - Automatic configuration validation and optimization layer
  - New `Guidance` module (`src/core/guidance.rs`) that validates and sanitizes configurations before execution
  - `FlightPlan` struct containing optimized configuration and user-facing notices
  - `Notice` system with four severity levels: Info, Warning, Optimization, Safety
  - Implements 11 configuration rules to prevent conflicts and data corruption:
    - **Rule 1 (Hardware)**: Disables zero-copy when not supported by OS/hardware
    - **Rule 2 (Strategy)**: Disables zero-copy when checksum verification is enabled (streaming checksum is faster than zero-copy + read-back)
    - **Rule 3 (Integrity)**: Disables checksum verification when resume is enabled (cannot verify full file when skipping beginning)
    - **Rule 4 (Safety)**: Disables resume when compression is enabled (cannot safely append to compressed streams)
    - **Rule 5 (Precision)**: Disables zero-copy when resume is enabled (precise byte-level seeking requires buffered I/O)
    - **Rule 6 (Visibility)**: Disables zero-copy when manifest generation is enabled (need content inspection for hashing/chunking)
    - **Rule 7 (Logic)**: Disables zero-copy when delta transfer is active (requires application-level patching logic)
    - **Rule 8 (Control)**: Disables zero-copy on macOS when bandwidth limit is set (fcopyfile cannot be throttled)
    - **Rule 9 (UX)**: Warns when parallel transfers use progress bars (may cause visual artifacts)
    - **Rule 10 (Performance)**: Warns when Sync/Update mode is combined with Checksum check mode (forces full file reads on both ends)
    - **Rule 11 (Physics)**: Placeholder for compression vs encryption (encrypted data cannot be compressed)
  - Transparent user notifications displayed in formatted box with icons (🛡️ Safety, 🚀 Optimization, ⚠️ Warning, ℹ️ Info)
  - Integration in main execution flow between config loading and execution
  - Comprehensive test coverage: 11 unit tests in guidance module, 14 integration tests including CLI output verification
  - Architecture documentation: `docs/architecture/GUIDANCE_SYSTEM.md` with detailed rationale for each rule
  - Philosophy: Users express intent, system ensures technical correctness

### Changed
- **Concurrency Detection Safety** - Improved CPU detection fallback behavior for restricted environments
  - Changed default fallback from 4 threads to 1 thread (single-threaded mode) when CPU detection fails
  - Added stderr warning when `std::thread::available_parallelism()` fails (e.g., in strict cgroup environments or restricted containers)
  - Prevents potential resource exhaustion and OOM kills in hostile environments
  - Updated `ConcurrencyLimiter::new()` documentation to explain auto-detection and fallback behavior
  - New unit tests: `test_shim_behavior_sane()` and `test_optimal_concurrency_calculation()`
  - New performance guide: `docs/guides/PERFORMANCE.md` documenting concurrency detection and manual override options

### Added
- **Delta Rolling Throughput** - Reworked non-matching buffering to slice-and-emit (`pending_start` cursor) instead of byte-by-byte pushes, slashing allocator pressure and improving worst-case (0% similarity) throughput; fixed rolling Adler window updates to keep weak hashes aligned; new Criterion benchmark `delta_throughput` captures no-match performance
- **Resume System Reliability Hardening** - Atomic temp-file + rename persistence for resume metadata, optional crash-simulation hook (`ORBIT_RESUME_SLEEP_BEFORE_RENAME_MS`), and regression tests covering temp-file cleanup and crash behavior
- **Manifest Generation in Delta Transfers** - Integrated manifest emission and updates in delta transfer logic (`src/core/delta/types.rs`, `src/core/delta/transfer.rs`)
  - `ManifestDb` struct for JSON-backed manifest database storage
  - `ManifestEntry` struct for tracking file transfer metadata (source path, dest path, checksum, size, mtime, delta stats)
  - `update_manifest_if_configured()` helper function encapsulates manifest update logic
  - Integration in `copy_with_delta()` and `copy_with_delta_fallback()` — manifests automatically updated on successful transfers
  - `DeltaConfig.validate_manifest()` method ensures proper configuration (update_manifest requires manifest_path)
  - New `DeltaStats.manifest_updated` field for observability
  - Respects `ignore_existing` option to skip manifest updates if file already exists
  - 12 new unit tests covering manifest creation, save/load, validation, and integration
  - README documentation with usage examples for manifest-driven audits and analytics

- **Delta Resume Handling** - Partial manifest support for interrupted delta transfers (`src/core/delta/types.rs`, `src/core/delta/transfer.rs`)
  - `PartialManifest` struct for tracking delta transfer progress with JSON serialization
  - Resume capability via `{dest}.delta.partial.json` manifest files
  - Automatic manifest creation on transfer start (when `resume_enabled = true`)
  - Manifest validation checks source path, size, and modification time
  - Automatic cleanup on successful transfer completion
  - Smart fallback: resumable errors save manifest, non-resumable errors trigger full copy
  - New `DeltaConfig` fields: `resume_enabled` (default: true), `chunk_size` (default: 1MB)
  - New `CopyConfig` fields: `delta_resume_enabled`, `delta_chunk_size`
  - New `DeltaStats` fields: `chunks_resumed`, `bytes_skipped`, `was_resumed`
  - New `CopyStats` fields: `chunks_resumed`, `bytes_skipped`
  - `should_use_delta()` now prioritizes delta when valid partial manifest exists
  - Integration tests in `tests/delta_resume_test.rs`
  - README documentation updates with programmatic usage examples

- **Default Retry Metrics Emission** - Retry statistics are now collected and emitted by default during `copy_file` operations (`src/core/mod.rs`, `src/instrumentation.rs`)
  - `OperationStats::emit()` method for automatic metrics output to stderr
  - `OperationStats::has_activity()` method to check if any operations were recorded
  - `copy_file_with_stats()` function for custom statistics tracking across batch operations
  - `copy_file_impl_with_stats()` internal function for full control over progress and stats
  - Environment variable control: `ORBIT_STATS=off` disables emission, `ORBIT_STATS=verbose` always emits
  - Default behavior: Only emits when noteworthy events occur (retries, failures, skips)
  - Integration with retry module: Stats are passed to `with_retry_and_metadata_stats()` automatically
  - New integration tests for default stats tracking and aggregated batch operations
  - README documentation updates with programmatic usage examples

- **Audit Logging Integration** - Structured audit logging for copy operations (`src/audit.rs`)
  - `AuditLogger` struct for thread-safe, append-only audit log writing
  - `AuditEvent` struct matching README specification with all required fields:
    - `timestamp`, `job`, `source`, `destination`, `protocol`
    - `bytes_transferred`, `duration_ms`, `compression`, `compression_ratio`
    - `checksum_algorithm`, `checksum_match`, `status`, `retries`
    - Optional fields: `storage_class`, `multipart_parts`, `starmap_node`, `files_count`
  - JSON Lines format (default) and CSV format support via `AuditFormat` enum
  - Automatic audit event emission at copy operation lifecycle points:
    - Start event (with expected bytes)
    - Completion event (success with metrics or failure with error details)
  - Integration with `copy_file` and `copy_directory` pipelines
  - Builder pattern for `AuditEvent` construction
  - Graceful error handling (audit failures don't abort copy operations)
  - Configurable via `CopyConfig.audit_log_path` and `CopyConfig.audit_format`
  - Comprehensive unit tests (11) and integration tests (9)

- **SMB/CIFS Backend Abstraction** - Full `Backend` trait implementation for SMB network shares
  - `SmbBackend` implementing unified `Backend` trait (`src/backend/smb.rs`)
  - URI-based configuration: `smb://[user[:pass]@]host[:port]/share/path`
  - Security modes: `RequireEncryption`, `SignOnly`, `Opportunistic` (via `?security=` query param)
  - Environment variable configuration (`ORBIT_SMB_HOST`, `ORBIT_SMB_SHARE`, `ORBIT_SMB_USER`, etc.)
  - Custom port support for non-standard SMB configurations
  - Backend registry integration for dynamic backend creation
  - Full async I/O with tokio integration
  - Streaming directory operations
  - Comprehensive error mapping to unified `BackendError` types

- **Nebula Web Dashboard v1.0.0-alpha.3** - Complete interactive dashboard UI
  - Modern dark theme with gradient accents and professional styling
  - Sidebar navigation (Overview, Jobs, API Explorer, WebSocket Monitor)
  - Stats grid with live indicators (Server Status, Active Transfers, Completed, WebSocket)
  - Interactive login page with loading states and error handling
  - Jobs table with status badges (completed/running/pending/failed) and progress bars
  - Built-in API Explorer for testing endpoints directly from the dashboard
  - WebSocket Monitor for real-time event visualization
  - Demo job scripts (`create-demo-jobs.bat`, `create-demo-jobs-api.bat`)

- **GUI Integration**
  - New `orbit serve` subcommand to launch the web dashboard with GUI enabled by default via the `gui` feature
  - Shared `orbit_web::start_server` entry point reused by the CLI and `orbit-web` binary
  - Documentation updates for GUI integration and a smoke test for the `/api/health` endpoint

- **Magnetar Job Management**
  - New `new_job()` method in JobStore trait for creating jobs with auto-generated IDs
  - Jobs metadata table with source, destination, compress, verify, and parallel settings
  - Database migration `20250102000000_add_jobs_table.sql`
  - Support for SQLite AUTOINCREMENT job IDs (redb backend shows appropriate error)
  - Extended jobs table schema with progress tracking columns (progress, total_chunks, completed_chunks, failed_chunks)

- **Metadata Preservation Documentation Clarity** - Addressed discrepancy between README claims and feature-gated implementation
  - Updated README.md to clearly distinguish default metadata support (timestamps, permissions) from extended metadata (xattrs, ownership)
  - Added explicit documentation that `extended-metadata` feature is required for xattr and full ownership support
  - Added "Optional Features" table in Quick Start section documenting all optional Cargo features
  - Clarified that `extended-metadata` feature is Unix/Linux/macOS only (xattr crate dependency)
  - Updated preservation flags documentation to indicate default vs. feature-gated capabilities
  - New integration test file `tests/metadata_preservation_test.rs` with 9 comprehensive tests:
    - Basic metadata preservation (timestamps, permissions)
    - PreserveFlags parsing and configuration
    - FileMetadata extraction and manifest round-trip
    - Feature-gated xattr tests (conditional compilation)
    - Platform-specific tests (Windows attributes, Unix ownership)

### Fixed
- **macOS Zero-Copy Empty Files Issue** - Fixed empty file creation when using zero-copy on macOS
  - Replaced FD-based `fcopyfile` with path-based `std::fs::copy` for macOS (`src/core/zero_copy.rs`)
  - Fixes file offset mismatch issues between Rust's `File` management and raw libc calls
  - Enables APFS Copy-On-Write cloning via automatic `fclonefileat` attempts (instant copies on APFS!)
  - Fallback chain: `fclonefileat` (instant COW) → `fcopyfile` → `read/write`
  - Re-enabled zero-copy for macOS in `should_use_zero_copy()` (previously disabled)
  - Maintains full checksum verification and progress tracking support
  - Provides significant performance improvements on APFS filesystems (macOS 10.13+)

- **Job ID Alignment** - Fixed inconsistency in Orbit Web API job ID handling
  - `create_job` now returns numeric job IDs (e.g., "1", "2", "3") instead of UUIDs
  - Added `jobs` metadata table to Magnetar with auto-incrementing INTEGER primary key
  - All API endpoints now use consistent numeric ID format throughout the stack
  - Updated `delete_job()` to clean up jobs metadata table
  - Added comprehensive test coverage for job creation lifecycle

- **Login Screen Visibility** - Login page properly hides after authentication
- **Login Response Parsing** - Fixed nested JSON response handling (`data.user || data`)

## [0.5.0] - 2025-11-10

### Added
- **Web GUI (orbit-web crate)** - Full-stack Rust web interface for transfer orchestration
  - Real-time dashboard with auto-refreshing job list (2-second interval)
  - Job creation form with compression, verification, and parallel settings
  - Live progress tracking with visual progress bars and percentage display
  - WebSocket support for real-time updates (/ws/progress/:job_id)
  - Leptos 0.6 SSR framework with Axum 0.7 backend
  - Magnetar SQLite integration for persistent job state
  - Tailwind CSS responsive UI design
  - RESTful API endpoints (/api/jobs, /api/create-job, /api/cancel-job)
  - Comprehensive documentation (docs/WEB_GUI.md - 835 lines)
  - Developer guide (crates/orbit-web/README.md)
  - Architecture diagrams and deployment examples

- **Release Automation System**
  - GitHub Actions workflow for automated releases (.github/workflows/release.yml)
  - Multi-platform binary building (Linux x64/ARM64, macOS x64/ARM64, Windows x64)
  - Automated orbit-web WASM compilation with cargo-leptos
  - Auto-generated GitHub releases with installation instructions
  - Release documentation (RELEASE.md - 579 lines)
  - Quick reference guide (docs/RELEASE_QUICKSTART.md - 136 lines)
  - Semantic versioning guidelines and hotfix procedures

### Changed
- Updated installation instructions to use source-only installation (removed incorrect crates.io references)
- Fixed README.md table of contents links with proper emoji anchors
- Expanded README.md with comprehensive Web GUI section (~260 lines)
- Updated badges (removed incorrect crates.io badges, added GitHub stars)
- Enhanced documentation index with Web GUI and release guides

### Documentation
- docs/WEB_GUI.md - Complete Web GUI guide (835 lines)
- RELEASE.md - Full release process documentation (579 lines)
- docs/RELEASE_QUICKSTART.md - Quick 5-step release guide (136 lines)
- Total new documentation: ~1,550 lines

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

- **Progress Reporting & Operational Controls** - Production-grade progress tracking and resource management
  - **Enhanced Progress Tracking** (`src/core/enhanced_progress.rs`)
    - Multi-progress bars using `indicatif` for concurrent transfers
    - Real-time ETA calculations and transfer speed tracking
    - Per-file progress bars with bytes transferred
    - Event-driven updates integrated with existing progress system
    - Support for multiple simultaneous transfers
  - **Dry-Run Mode** (`src/core/dry_run.rs`)
    - Simulation mode for safe planning and preview
    - Records all planned operations (copy, update, skip, delete, mkdir)
    - Summary statistics with total data size
    - Detailed logging via tracing framework
    - Works with all features (filters, transformations, etc.)
  - **Bandwidth Limiting** (`src/core/bandwidth.rs`)
    - Token bucket rate limiting using `governor` crate
    - Configurable bytes-per-second limits (MB/s via CLI)
    - Zero overhead when disabled (0 = unlimited)
    - Thread-safe and cloneable for concurrent operations
    - Smooth throttling with ~1ms sleep granularity
  - **Concurrency Control** (`src/core/concurrency.rs`)
    - Counting semaphore for parallel operation management
    - Auto-detection based on CPU cores (2x cores, capped at 16)
    - Configurable maximum concurrent operations
    - RAII-based permit release (automatic cleanup)
    - Blocking and non-blocking acquire support
  - CLI integration: `--dry-run`, `--max-bandwidth N`, `--parallel N`, `--show-progress`, `--verbose`
  - Comprehensive unit tests (13 tests total across all modules)
  - Full documentation: [PROGRESS_AND_CONCURRENCY.md](PROGRESS_AND_CONCURRENCY.md)
  - Dependencies added: `indicatif = "0.17"`, `governor = "0.6"`

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
