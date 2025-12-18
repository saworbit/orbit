# Changelog

All notable changes to Orbit will be documented in this file.

## [Unreleased]

### Added - V3 Unified Observability & Immutable Audit Plane

**Enterprise-grade observability with cryptographic integrity** - This release implements a unified observability system that combines distributed tracing, immutable audit logs, and operational metrics into a single cohesive platform.

#### Core Observability Implementation

- **`orbit-observability` crate**: Complete observability stack (1,890 lines)
  - **Unified Event Schema**: Single `OrbitEvent` type with 15 payload variants replacing fragmented AuditEvent/TelemetryEvent
  - **Cryptographic Audit Chaining**: HMAC-SHA256 tamper-evident logs with automatic integrity hash linking
  - **W3C Trace Context**: Full distributed tracing support with 32-char hex trace IDs and 16-char hex span IDs
  - **Backend Instrumentation**: All 45 methods across 4 backends (local, S3, SMB, SSH) emit trace spans
  - **Prometheus Metrics**: 5 core metrics derived from event streams (retries, latency, integrity failures)
  - **OpenTelemetry Integration**: Export traces to Jaeger, Honeycomb, Datadog via OTLP protocol

#### Security & Compliance Features

- **Tamper Detection**: Forensic validator detects modification, deletion, insertion, and reordering
- **Secret Management**: HMAC keys via `ORBIT_AUDIT_SECRET` environment variable
- **Monotonic Sequencing**: Strictly ordered events with sequence numbers
- **Forensic Validation**: Python validator (`scripts/verify_audit.py`) for chain integrity verification
- **Compliance Ready**: SOC 2, HIPAA, GDPR audit trail support

#### Integration & Configuration

- **CLI Flags**: `--audit-log`, `--otel-endpoint`, `--metrics-port`
- **Environment Variables**: `ORBIT_AUDIT_SECRET`, `OTEL_EXPORTER_OTLP_ENDPOINT`
- **TOML Configuration**: `audit_log_path`, `otel_endpoint`, `metrics_port` fields
- **Graceful Fallback**: Disabled logger when secret not set (logs warning)

#### Testing & Documentation

- **Automated Tests**: `tests/audit_tampering_test.sh` with 5 tampering detection scenarios
- **Example Demo**: `examples/audit_logging_demo.rs` showing W3C trace context propagation
- **Comprehensive Docs**: 600+ lines in `docs/observability-v3.md` covering:
  - Quick start guide
  - Configuration reference
  - Integration with Jaeger/Honeycomb/Datadog
  - Forensic validation procedures
  - Security best practices
  - Troubleshooting guide
- **Implementation Summary**: `OBSERVABILITY_IMPLEMENTATION.md` documenting all deliverables

#### Performance

- **Low Overhead**: <5% performance impact when enabled (3.2% measured)
- **Zero Cost**: 0% overhead when disabled (opt-in architecture)
- **High Throughput**: 15,000 events/sec write rate, <10µs HMAC computation

#### Files Added/Modified

**New Files (11):**
- `crates/orbit-observability/src/*.rs` (9 modules)
- `scripts/verify_audit.py` (forensic validator)
- `tests/audit_tampering_test.sh` (test suite)
- `examples/audit_logging_demo.rs` (interactive demo)
- `docs/observability-v3.md` (user guide)
- `OBSERVABILITY_IMPLEMENTATION.md` (implementation summary)

**Modified Files (5):**
- `src/logging.rs`: Audit bridge + OpenTelemetry layers integration
- `src/config.rs`: Added `otel_endpoint`, `metrics_port` fields
- `src/main.rs`: CLI arguments for observability flags
- `Cargo.toml`: Added `orbit-observability` dependency
- `src/backend/*.rs`: Instrumented all 4 backends with `#[tracing::instrument]`

**Total Deliverable**: 3,345+ lines (code + tests + documentation)

### Changed

- **Dependency Updates: tonic 0.14 Ecosystem Migration**
  - Updated gRPC stack to tonic 0.14.2 (from 0.12.3)
  - Updated prost to 0.14.1 (from 0.13.5)
  - Migrated from `tonic-build` to `tonic-prost-build` for proto compilation
  - Added `tonic-prost` runtime dependency for generated code
  - Updated `tonic-reflection` to 0.14.2 (from 0.12.3)
  - **Breaking Change**: Requires protoc (Protocol Buffers compiler) to be installed for builds
  - **Files modified**:
    - `orbit-proto/Cargo.toml`: Updated dependencies and build script
    - `orbit-proto/build.rs`: Changed API from `tonic_build::` to `tonic_prost_build::`
    - `orbit-star/Cargo.toml`: Updated tonic and tonic-reflection
    - `orbit-connect/Cargo.toml`: Updated tonic
  - See [Migration Guide](docs/guides/migration_guide.md#tonic-014-migration) for details

- **Other Dependency Updates**
  - Cargo: toml 0.8.23 → 0.9.8, smb 0.11.0 → 0.11.1, criterion 0.5.1 → 0.8.1
  - GitHub Actions: actions/cache 4 → 5, actions/download-artifact 6 → 7, actions/checkout 4 → 6
  - NPM/Dashboard: React 19.2.0 → 19.2.3, Vite 7.2.7 → 7.3.0, and various dev dependencies

### Fixed

- **CLI Integration with orbit-server** (v2.2.0 compatibility)
  - Added `gui` feature alias mapping to `api` for backward compatibility
  - Updated `serve_gui()` function to use correct crate name (`orbit_server` instead of `orbit_web`)
  - Updated configuration struct (`ServerConfig` instead of `WebConfig`)
  - Added `reactor_notify` parameter to match new `start_server()` signature
  - Files modified: `Cargo.toml`, `src/main.rs`

### Added - Phase 5: The Sentinel (Autonomous Resilience Engine)

**OODA Loop for Data Durability** - This release implements the Sentinel, an autonomous resilience engine that continuously monitors the Orbit Grid's Universe V3 database to ensure chunk redundancy. The Sentinel runs an infinite OODA loop (Observe-Orient-Decide-Act), scanning all chunks to identify under-replicated data and autonomously triggering Phase 4 P2P transfers to restore redundancy targets.

#### Core Sentinel Implementation

- **`orbit-sentinel` crate**: Autonomous resilience engine
  - **`Sentinel` daemon**: Main OODA loop executor
    - Observe: Scan Universe V3 for all chunks via `scan_all_chunks()`
    - Orient: Count active copies per chunk
    - Decide: Identify chunks below `min_redundancy` threshold
    - Act: Spawn healing tasks with semaphore-based concurrency control
  - **`Medic`**: Healing orchestration logic
    - Source selection: Pick survivor Star with chunk data
    - Recruit selection: Find target Star without the chunk
    - P2P orchestration: Generate JWT token and trigger `ReplicateFile` RPC
    - Universe update: Record new replica location in Universe V3
  - **`SentinelPolicy`**: Configurable operational parameters
    - `min_redundancy`: Minimum copies required (default: 2)
    - `max_parallel_heals`: Concurrency limit (default: 10)
    - `scan_interval_s`: Sweep frequency (default: 3600s = 1 hour)
    - `healing_bandwidth_limit`: Optional rate limiting
  - **`SweepStats`**: Health metrics tracking
    - Healthy/at-risk/lost chunk counts
    - Healing success/failure rates
    - Grid health percentage reporting

#### Universe V3 Extensions (Breaking Change)

- **`ChunkLocation` schema update**:
  - Added `star_id: String` field to identify chunk ownership
  - Migration: Legacy V2 data defaults to `"local"` star_id
  - Constructor now requires: `ChunkLocation::new(star_id, path, offset, length)`
- **New iteration methods**:
  - `iter_all_hashes()`: Returns all unique chunk hashes
  - `scan_all_chunks(callback)`: Streaming iteration with callback for memory-efficient sweeps
  - Uses `range::<&[u8; 32]>(..)` on redb multimap for efficient traversal

#### orbit-web Integration

- **Optional `sentinel` feature** in `orbit-web/Cargo.toml`
  - Build with Sentinel: `cargo build --features sentinel`
  - Runtime activation: `ORBIT_SENTINEL_ENABLED=true`
- **Environment configuration**:
  - `ORBIT_UNIVERSE_DB`: Path to Universe V3 database (default: `universe_v3.db`)
  - `ORBIT_AUTH_SECRET`: Shared secret for P2P JWT tokens (reuses Phase 4 auth)
  - `ORBIT_SENTINEL_MIN_REDUNDANCY`: Minimum chunk copies (default: 2)
  - `ORBIT_SENTINEL_SCAN_INTERVAL`: Seconds between sweeps (default: 3600)
  - `ORBIT_SENTINEL_MAX_PARALLEL_HEALS`: Concurrent healing limit (default: 10)
  - `ORBIT_SENTINEL_BANDWIDTH_LIMIT`: Optional bytes/sec limit (default: 50 MB/s)
- **Daemon lifecycle**: Spawned as background tokio task after Reactor startup

#### Testing & Documentation

- **Resilience tests** (`sentinel_resilience_test.rs`):
  - **Chaos Monkey test**: Simulates Star failure and validates healing
    - Phase 1: Upload chunk to Star A (1 copy, under-replicated)
    - Phase 2: Sentinel heals to Star B (2 copies, meets threshold)
    - Phase 3: Simulate Star A failure
    - Phase 4: Sentinel heals from B to C (maintains 2 copies)
  - **Basic sweep test**: Validates healthy chunk scanning without healing
- **Comprehensive specification**:
  - Phase 5 spec: [`docs/specs/PHASE_5_SENTINEL_SPEC.md`](docs/specs/PHASE_5_SENTINEL_SPEC.md)

#### Architecture Evolution

```
BEFORE (Phase 4): Manual redundancy management
AFTER (Phase 5):  Autonomous self-healing Grid

[Sentinel OODA Loop]
    ↓ Observe (scan Universe V3)
    ↓ Orient (count copies: 1 < min_redundancy:2)
    ↓ Decide (mark chunk as at-risk)
    ↓ Act (spawn Medic task)
        ↓ [Medic] Pick Source (Star B has chunk)
        ↓ [Medic] Pick Recruit (Star C available)
        ↓ [Medic] Generate JWT token
        ↓ [Medic] Trigger Phase 4 P2P transfer
        ↓ [Medic] Update Universe V3
    ↓ Continue (next chunk...)
```

### Changed

- **Breaking**: `ChunkLocation` now requires `star_id` parameter in constructor
- Updated all tests to use 4-parameter `ChunkLocation::new(star_id, path, offset, length)`

### Maintenance

- Fixed clippy warnings: Removed unused imports (`anyhow::Result`, `ReplicateRequest`, `error` in tracing)
- Fixed clippy warnings: Prefixed unused variables with `_` (`_token`, `_recruit_system`)
- Fixed clippy warning: Removed unnecessary `mut` from `locations_iter` in `universe_v3.rs`
- Fixed clippy warning: Corrected doc comment indentation in `scan_all_chunks()`
- Ran `cargo fmt --all` to format all code
- Ran `cargo audit` with zero vulnerabilities found

## [0.6.0-alpha.4] - 2025-12-11

### Added - Phase 4: The Data Plane (Star-to-Star Transfer)

**Peer-to-Peer Data Movement** - This release implements direct Star-to-Star data transfer, eliminating the Nucleus bandwidth bottleneck. Data now flows directly between Stars without routing through the Nucleus, enabling infinite horizontal bandwidth scaling.

#### Protocol Extensions

- **New RPC Methods in `orbit.proto`**:
  - **`ReplicateFile`**: Command interface (Nucleus → Destination Star)
    - Orchestrates P2P transfer with source URL, paths, and JWT token
    - Returns success status, bytes transferred, and SHA-256 checksum
  - **`ReadStream`**: Data access interface (Destination Star → Source Star)
    - Streams file content in 64KB chunks with backpressure control
    - Stateless JWT token authorization (no session required)
  - **New message types**: `ReplicateRequest`, `ReplicateResponse`, `ReadStreamRequest`, `ReadStreamResponse`

#### Security Infrastructure

- **`orbit-star/src/auth.rs`**: JWT-based authentication service
  - **Stateless authorization**: HMAC-SHA256 signed tokens
  - **Path-specific permissions**: Token authorizes specific file access
  - **Time-bound validity**: 1-hour default expiration (configurable)
  - **Zero-trust model**: No shared database required between Stars
  - **Token claims**: subject, allow_file, expiration, issued_at, issuer
- **Shared secret distribution**: `ORBIT_AUTH_SECRET` environment variable

#### Star Agent Enhancements

- **`read_stream()` implementation** (Source Star):
  - JWT token verification before serving data
  - PathJail security validation (directory traversal protection)
  - Async I/O with tokio::fs for non-blocking reads
  - 64KB streaming chunks with 4-message backpressure buffer
  - Graceful client disconnection handling
- **`replicate_file()` implementation** (Destination Star):
  - Session validation (Nucleus authentication required)
  - Remote Star connection via gRPC
  - Streaming write with SHA-256 checksum computation
  - Expected size verification (detects truncated transfers)
  - Automatic parent directory creation
- **CLI updates**: Added `--auth-secret` parameter and `ORBIT_AUTH_SECRET` env var
- **Dependencies**: Added `jsonwebtoken`, `sha2`, `serde`

#### Testing & Documentation

- **Integration tests** (`p2p_transfer_test.rs`):
  - Triangle Test: End-to-end Star → Star transfer verification
  - Security tests: Invalid token rejection, path authorization enforcement
  - Streaming tests: Multi-chunk transfer validation (200KB file)
- **Comprehensive documentation**:
  - Phase 4 specification: [`docs/specs/PHASE_4_DATA_PLANE_SPEC.md`](docs/specs/PHASE_4_DATA_PLANE_SPEC.md)
  - Implementation summary: [`docs/PHASE_4_IMPLEMENTATION_SUMMARY.md`](docs/PHASE_4_IMPLEMENTATION_SUMMARY.md)

#### Performance Impact

- **Before Phase 4**: 1GB transfer Star A → Star B = 2GB Nucleus traffic (bottleneck)
- **After Phase 4**: 1GB transfer Star A → Star B = <1KB Nucleus traffic (command only)
- **Scalability**: Bandwidth now scales horizontally with number of Stars
- **Chunk size**: 64KB (balance between memory usage and RPC overhead)

#### Architecture Evolution

```
BEFORE: [Star A] ──(1GB)──> [Nucleus] ──(1GB)──> [Star B]  (bottleneck)
AFTER:  [Star A] ──(1GB)────────────────────────> [Star B]  (P2P direct)
        [Nucleus] (sends <1KB command only)
```

### Changed

### Fixed

### Maintenance

## [0.6.0-alpha.3] - 2025-12-11

### Added - Phase 3: The Nucleus Client & RemoteSystem

**Orbit Grid integration layer** - This release implements client-side connectivity, enabling the Nucleus (Hub) to orchestrate operations across remote Stars (Agents). The Nucleus can now execute jobs with transparent local/remote system abstraction.

#### Core Connectivity

- **`orbit-connect` crate**: Client-side gRPC orchestration
  - **`RemoteSystem`**: Full `OrbitSystem` implementation that proxies to remote Stars
    - Discovery operations: `exists()`, `metadata()`, `read_dir()` via `ScanDirectory` RPC
    - Compute offloading: `calculate_hash()` via `CalculateHash` RPC (99.997% network reduction for CDC)
    - Intelligence: `read_header()` via `ReadHeader` RPC (magic number detection)
    - Session management: Automatic `x-orbit-session` metadata attachment
  - **`StarManager`**: Connection pool and lifecycle management
    - Lazy connection establishment (connect on first use)
    - Automatic handshake with session ID caching
    - Support for multiple concurrent Stars
    - Registry with `StarRecord` (ID, address, token, status)
  - **`ConnectError`**: Comprehensive error handling with conversion to `OrbitSystemError`
  - Comprehensive README with architecture diagrams and usage examples
  - Integration tests for registration, connection, and remote operations

#### Database Schema Evolution

- **New migration**: `20250103000000_add_star_columns.sql`
  - Added `source_star_id` and `dest_star_id` columns to `jobs` table
  - NULL = local execution (Nucleus filesystem)
  - Non-NULL = remote execution (Star ID)
  - Indexes for Star-based queries and cross-Star transfers

#### Magnetar Integration

- **Enhanced `JobStore` trait**:
  - Updated `new_job()` signature with `source_star_id` and `dest_star_id` parameters
  - **Backward compatible**: existing jobs (NULL star IDs) continue to work
  - Comprehensive Phase 3 Grid Support documentation
- **Backend implementations**:
  - SQLite backend: Full support for Star ID persistence
  - redb backend: Signature compatibility (job IDs not yet supported)
  - Migration store: Transparent passthrough for Star IDs

#### Documentation

- **New:** `docs/specs/PHASE_3_NUCLEUS_CLIENT_SPEC.md` - Complete architecture specification
  - Executive summary and architecture overview
  - Detailed API design with Liskov Substitution Principle adherence
  - Security model (token storage, session management)
  - Testing strategy and migration path
  - Performance analysis (compute offloading, network reduction)
  - Phase 4 lookahead (Star-to-Star direct transfer)
- **New:** `crates/orbit-connect/README.md` - Client library guide with usage examples

### Performance Benefits

- **Compute Offloading**: Hash calculation on remote Stars
  - Old: Transfer 1GB → compute hash on Nucleus
  - New: Compute hash on Star → transfer 32 bytes
  - **99.997% network reduction** for 1000-chunk CDC operation
- **Header Intelligence**: Magic number detection without full transfer
  - Old: Transfer entire file to determine type
  - New: Transfer first 512 bytes → detect → skip if unwanted
- **Connection Reuse**: gRPC multiplexing
  - Single TCP connection for all operations
  - No per-request handshake overhead

### Architecture Achievement

The Nucleus can now:
- ✅ Maintain a registry of Stars
- ✅ Establish gRPC connections on-demand
- ✅ Offload compute operations (hashing) to remote nodes
- ✅ Use the same magnetar code for local and remote operations (Liskov Substitution)

**Job execution modes now supported:**
- Local-to-local: `source_star_id: None, dest_star_id: None` → LocalSystem
- Remote source: `source_star_id: Some("star-1"), dest_star_id: None` → RemoteSystem + LocalSystem
- Full Grid (Phase 4): `source_star_id: Some("star-1"), dest_star_id: Some("star-2")` → RemoteSystem + RemoteSystem

### Build System

- Updated workspace with `orbit-connect` crate
- All crates compile with zero warnings under `clippy -D warnings`
- Allowed `clippy::too_many_arguments` for `new_job()` API (8 parameters required for Grid support)

### Testing

- ✅ All existing tests passing
- ✅ Integration tests for StarManager (registration, connection, disconnect)
- ✅ Live integration tests for remote operations (requires running orbit-star, marked `#[ignore]`)
- ✅ No security vulnerabilities (cargo audit clean)

## [0.6.0-alpha.2] - 2025-12-11

### Added - Phase 2: The Star Protocol & Agent

**Orbit Grid nervous system** - This release establishes the gRPC protocol for distributed operations and implements the Star agent, a lightweight remote execution server that exposes filesystem and CPU resources to the Grid.

#### Protocol Definition

- **`orbit-proto` crate**: gRPC protocol definitions
  - `orbit.proto` schema with 4 core RPC methods:
    - `Handshake` - Token-based authentication and session establishment
    - `ScanDirectory` - Streaming directory enumeration (handles 1M+ files)
    - `ReadHeader` - File header reading for semantic analysis
    - `CalculateHash` - Remote BLAKE3 hashing with content-defined chunking
  - Generated client and server stubs via tonic/prost
  - Build script with protoc integration (supports bundled protoc)
  - Comprehensive README with usage examples

#### Star Agent Binary

- **`orbit-star` crate**: Remote execution server
  - **Security**: PathJail sandbox for filesystem access control
    - Prevents directory traversal (`../../etc/passwd`)
    - Symlink resolution and canonicalization
    - Whitelist-based path validation
  - **Server**: Full `StarService` implementation
    - Session-based authentication with UUID tokens
    - Streaming directory scans via `tokio_stream`
    - Async file hashing using enhanced `orbit-core-cdc`
  - **CLI**: Production-ready command-line interface
    - Multi-path jail configuration (`--allow /path1 --allow /path2`)
    - Environment variable support (`ORBIT_STAR_TOKEN`)
    - Structured logging with tracing
    - Debug mode for development
  - Comprehensive README with deployment guide

#### Enhanced Core Libraries

- **`orbit-core-cdc` enhancements**:
  - New `hash_file_range()` async function for remote hashing
  - Feature-gated async support (`async` feature)
  - 64KB buffered I/O for efficient network transfers

#### Documentation

- **New:** `docs/specs/PHASE_2_STAR_PROTO_SPEC.md` - Complete architecture specification
  - Protocol contract design
  - Security model (token auth + path jail)
  - Operational guide with grpcurl examples
  - Implementation checklist and Phase 3 lookahead
- **New:** `crates/orbit-proto/README.md` - Protocol usage guide
- **New:** `crates/orbit-star/README.md` - Agent deployment guide
- **Updated:** `README.md` - Added Phase 2 to project status

### Build System

- Updated workspace with `orbit-proto` and `orbit-star` crates
- Resolved protoc dependency (bundled binary for Windows)
- All crates compile with zero warnings under `clippy -D warnings`

### Testing

- ✅ All existing tests passing
- ✅ PathJail unit tests for security validation
- ✅ No security vulnerabilities (cargo audit clean)

### Notes

- Phase 2 establishes the "Hand" (Star agent) for the future "Brain" (Nucleus/Hub)
- Next: Phase 3 will implement RemoteSystem for Grid connectivity

## [0.6.0-alpha.1] - 2025-12-11

### Added - Phase 1: I/O Abstraction Layer

**Foundation for distributed Grid/Star topology** - This release introduces the OrbitSystem abstraction layer, decoupling filesystem operations from core logic while maintaining full backward compatibility.

#### Core Infrastructure

- **`orbit-core-interface` crate**: Universal I/O abstraction trait
  - `OrbitSystem` trait with Discovery (`exists`, `metadata`, `read_dir`), Data Access (`reader`, `writer`), and Compute Offloading (`read_header`, `calculate_hash`) operations
  - `OrbitSystemExt` trait for convenience methods (`read_all`, `write_all`, `calculate_file_hash`)
  - Comprehensive rustdoc documentation with inline examples
  - Zero-overhead abstraction via monomorphization
  - 7 doc tests + 1 unit test, all passing

- **`LocalSystem` implementation**: Default provider for standalone mode
  - Located in `src/system/local.rs`
  - Wraps `tokio::fs` operations with full async/await support
  - Proper error mapping (`NotFound`, `PermissionDenied`, etc.)
  - 6 comprehensive unit tests covering all operations

- **`MockSystem` implementation**: In-memory filesystem for testing
  - Located in `src/system/mock.rs`
  - No real disk I/O required (deterministic tests)
  - Helper methods: `add_file()`, `add_dir()`, `remove()`, `clear()`
  - 6 comprehensive unit tests

#### Library Integration

- **`core-semantic` enhancement**: Added async `determine_intent_async()` method
  - Accepts `OrbitSystem` for filesystem operations
  - Reads first 512 bytes for file type detection
  - Maintains backward compatibility with synchronous `determine_intent()`
  - 8 unit tests + 2 doc tests, all passing

- **`magnetar` preparation**: Added `orbit-core-interface` dependency
  - Foundation for future executor refactoring (Phase 2)
  - Documented in `PHASE_1_ABSTRACTION_SPEC.md`

#### Build System

- New `orbit-system` feature flag (enabled by default)
  - Gates `tokio` and `async-trait` dependencies
  - Integrated into default features: `default = ["zero-copy", "orbit-system"]`
- Updated workspace configuration with new crate

#### Documentation

- **New:** `docs/specs/PHASE_1_ABSTRACTION_SPEC.md` - 14-section comprehensive specification (architecture, implementation, migration, testing)
- **New:** `docs/specs/PHASE_1_IMPLEMENTATION_SUMMARY.md` - Implementation summary with test results
- **Updated:** `CONTRIBUTING.md` - Added "Architecture: OrbitSystem Pattern" section with usage examples and testing guide
- **Updated:** `README.md` - Added Phase 1 to Feature Maturity Matrix and Modular Architecture section
- **Updated:** `src/main.rs` - Version bumped to 0.6.0 with Phase 1 note

### Changed

- Build system: Added `orbit-system` to default features

### Testing

- ✅ 87 total tests passing (21 new + 66 existing)
  - `orbit-core-interface`: 1 + 7 doc tests
  - `orbit-core-semantic`: 8 + 2 doc tests
  - `orbit` (system::local): 6 tests
  - `orbit` (system::mock): 6 tests
- ✅ Zero breaking changes
- ✅ All existing functionality preserved
- ✅ Build: SUCCESS (17.63s)
- ✅ Clippy: Only pre-existing warnings (unrelated)

### Technical Details

**Benefits:**
- **Testability**: MockSystem enables fast, deterministic unit tests without filesystem
- **Flexibility**: Runtime switching between Local/Remote providers with zero code changes
- **Performance**: Compute offloading ready for distributed CDC (hash 32 bytes instead of MB)
- **Maintainability**: Clear separation of concerns

**Architecture Impact:**

```rust
// Before Phase 1
let file = std::fs::File::open(path)?;

// After Phase 1
async fn process<S: OrbitSystem>(system: &S, path: &Path) {
    let header = system.read_header(path, 512).await?;
    // Same code works for LocalSystem AND future RemoteSystem!
}
```

**Files Changed:**
- New: 7 files (~1,200 lines)
  - `crates/orbit-core-interface/` (Cargo.toml, src/lib.rs)
  - `src/system/` (mod.rs, local.rs, mock.rs)
  - `docs/specs/` (PHASE_1_ABSTRACTION_SPEC.md, PHASE_1_IMPLEMENTATION_SUMMARY.md)
- Modified: 7 files (~50 lines)
  - Root Cargo.toml, src/lib.rs, src/main.rs
  - crates/core-semantic/ (Cargo.toml, src/lib.rs)
  - crates/magnetar/Cargo.toml
  - CONTRIBUTING.md, README.md

### Next Steps: Phase 2

- RemoteSystem implementation (gRPC-based)
- Full magnetar executor refactoring
- Main CLI integration with dependency injection
- Performance benchmarking

---

## [0.6.0] - 2025-12-10

> ⚠️ **PRE-ALPHA WARNING**: This version contains highly experimental dashboard features.
> Not suitable for production environments. APIs and UI are subject to breaking changes.

### Added

- **🦕 Gigantor Heavy Lift Lane (SPEC-006)** - Large object optimization for files > 1GB
  - **Automatic Activation**: Routes files > 1GB to specialized pipeline
  - **Tiered Chunking**: Dynamically adjusts chunk size to prevent index explosion
    - 1GB-100GB: 1MB average chunks (16x index reduction vs standard)
    - >100GB: 4MB average chunks (64x index reduction vs standard)
    - Prevents 10TB file from generating 160M index entries (reduces to 2.5M)
  - **Parallel Hashing Pipeline**: Scan-Dispatch-Hash pattern for multi-core CPU saturation
    - Scanner thread: Sequential I/O + Gear Hash (finds CDC boundaries)
    - Hash workers: Parallel BLAKE3 across all cores via Rayon
    - Orchestrator: Batches of 64 chunks between stages
    - Performance: 4-7 GB/s on 8-16 core systems (vs 500 MB/s single-threaded)
  - **Long-Haul Connection Profile**: Extended connection lifetimes for multi-hour transfers
    - 24-hour max lifetime (supports S3 multipart uploads)
    - 10-minute acquire timeout (expected for heavy lane)
    - 4 max connections (prevents bandwidth saturation)
  - **New Components**:
    - `crates/magnetar/src/pipeline/router.rs`: PipelineRouter with strategy selection
    - `crates/magnetar/src/executor/gigantor.rs`: GigantorExecutor with parallel pipeline
    - `crates/core-resilience/src/connection_pool.rs`: PoolConfig::long_haul_profile()
  - **Best For**:
    - Virtual machine images (VMDK, VHD, QCOW2)
    - Database dumps (PostgreSQL, MySQL backups)
    - Video files (raw footage, master copies)
    - Large compressed archives (multi-GB tarballs)
    - Scientific datasets (genomics, satellite imagery)
  - **Performance Characteristics**:
    - Index: 64x fewer entries for >100GB files
    - Throughput: 4-7 GB/s (saturates NVMe drives)
    - Stability: No timeout issues on multi-hour transfers
    - Memory: ~160MB metadata for 10TB file (vs ~10GB with standard)
  - **Documentation**: Complete section added to `docs/guides/PERFORMANCE.md`
  - **Testing**:
    - 5 stress tests covering parallelism, chunk reduction, deduplication, routing
    - Benchmark test available for manual performance validation
    - All tests passing with sparse and realistic data patterns

- **⚖️ Equilibrium Standard Lane (SPEC-005)** - General-purpose deduplication for medium-sized files (8KB-1GB)
  - **Default Operating Mode**: Automatically handles 90% of typical file transfer workloads
  - **CDC Chunking**: Content-Defined Chunking using Gear Hash with 64KB average chunks
    - Min chunk size: 8KB, Average: 64KB, Max: 256KB
    - Optimized for detecting moved/refactored code and document changes
  - **Global Deduplication**: Universe Map integration for cross-file chunk deduplication
    - 30-70% typical bandwidth savings on repeated content
    - 100% deduplication for moved/renamed files
    - ACID-compliant redb storage with O(1) chunk existence lookup
  - **Air Gap Pattern**: CPU-intensive hashing offloaded to blocking threads
    - Prevents async reactor starvation during heavy compute
    - Maintains web dashboard responsiveness during large transfers
    - Uses `tokio::task::spawn_blocking` for BLAKE3 hashing
  - **Auto-Concurrency**: Scales worker threads based on CPU cores
    - Auto-detects via `std::thread::available_parallelism()`
    - Falls back to 1 core if detection fails (per PERFORMANCE.md)
    - Minimum 2 workers for optimal pipeline throughput
  - **New Components**:
    - `crates/magnetar/src/executor/standard.rs`: StandardExecutor with batch processing
    - `crates/magnetar/src/config.rs`: ConcurrencyConfig with auto-detection
    - `src/core/neutrino/router.rs`: Extended to support 3 lanes (Fast/Standard/Large)
  - **Router Updates**: FileRouter now routes files to 3 lanes:
    - Neutrino (<8KB): Direct transfer, no CDC
    - **Equilibrium (8KB-1GB)**: CDC + deduplication ← NEW DEFAULT
    - Gigantor (>1GB): Tiered dedup (future)
  - **Best For**:
    - Source code repositories (dedups moved/refactored code)
    - PDF documents and office files
    - VM images (dedups OS commonality)
    - Media libraries with duplicates
    - Database backups with repeated blocks
  - **Performance Characteristics**:
    - Memory: ~1KB RAM per 64KB of data processed
    - CPU: Moderate (BLAKE3 on blocking threads)
    - Network: Minimal for repeated content (only unique chunks transferred)
  - **Documentation**: Added comprehensive section to `docs/guides/PERFORMANCE.md`
  - **Testing**:
    - 7 integration tests covering deduplication, partial dedup, cross-file dedup, batch processing
    - All tests pass with realistic CDC behavior on uniform vs varied data
  - **Usage**: No special flags required - enabled by default
    ```bash
    # Uses Equilibrium automatically for 8KB-1GB files
    orbit sync /source /destination

    # Adjust concurrency manually
    orbit sync --concurrency 8 /source /dest

    # Enable compression for text-heavy content
    orbit sync --compress /source /destination
    ```

- **⚡ Neutrino Fast Lane (SPEC-004)** - Small file optimization with ~3x performance improvement
  - **Smart Routing**: Files <8KB bypass CDC/deduplication overhead for direct transfer
  - **High Concurrency**: 100-500 concurrent async tasks (vs standard 16)
  - **Zero Overhead**: Direct I/O without BLAKE3 hashing, CDC chunking, or starmap indexing
  - **Performance Gains**:
    - 10,000 files (1-4KB): ~15s vs ~45s (standard) = **3x faster**
    - 60% lower CPU usage for small-file workloads
    - Minimal database bloat (no index entries for small files)
  - **New CLI Flags**:
    - `--profile <standard|neutrino|adaptive>`: Select transfer profile
    - `--neutrino-threshold <KB>`: Custom threshold (default: 8KB)
  - **Architecture**:
    - `src/core/neutrino/router.rs`: Size-based routing ("The Sieve")
    - `src/core/neutrino/executor.rs`: DirectTransferExecutor with tokio::JoinSet
    - `src/core/neutrino/mod.rs`: Module exports and documentation
  - **Integration**: Works seamlessly with Smart Sync priority-based transfers
  - **Best For**: Source code repos, config directories, log files, npm/pip packages
  - **Requirements**: `backend-abstraction` feature (included with network backends)
  - **Connection Pool**: Added `PoolConfig::neutrino_profile()` for optimized settings
  - **Documentation**: Comprehensive guide added to `docs/guides/PERFORMANCE.md`
  - **Testing**: Unit tests for router and executor functionality
  - **Usage**:
    ```bash
    orbit copy --profile neutrino --recursive /source /dest
    orbit copy --profile neutrino --neutrino-threshold 16 --recursive /source /dest
    orbit copy --check smart --profile neutrino --recursive /source /dest
    ```

- **🎨 Full UI Migration to Production Dashboard** - Complete Figma mockup integration with real API
  - **Migration Status**: ✅ Production-ready (12/14 tasks complete, 86%)
  - **Figma Component Integration**: Migrated all UI components from `ui_mockup/` to main dashboard
  - **Screen Implementation**:
    - **Dashboard**: Real-time KPI cards, network topology map, activity feed
    - **Transfers**: Job creation form + JobList + TeraCopy-style chunk map visualization
    - **Files**: Professional placeholder UI with breadcrumb navigation (API integration pending)
    - **Pipelines**: Visual workflow editor placeholder (React Flow integration pending)
    - **Analytics**: Real-time KPI calculations with chart placeholders (Recharts pending)
    - **Settings**: Tabbed interface with theme selector, backend config, user management
  - **Dashboard Components**:
    - `KPICards.tsx`: Live job statistics (active jobs, data transferred, progress, total)
    - `NetworkMap.tsx`: Connection visualization with protocol detection (S3/SMB/SSH/Local)
    - `ActivityFeed.tsx`: Real-time job events with filtering and timestamps
  - **Authentication System**:
    - `AuthContext.tsx`: Complete auth provider with login/logout/session management
    - `Login.tsx`: Professional login screen with Shield icon branding
    - `ProtectedRoute.tsx`: Route protection wrapper for authenticated access
    - Token-based authentication with Bearer token injection via axios interceptor
    - Automatic 401 handling → redirect to login flow
    - User dropdown menu with profile info and sign-out functionality
    - Default credentials: `admin / admin`
  - **Dark Mode System**:
    - `ThemeProvider.tsx`: React Context-based theme management
    - Light/Dark/System theme modes with localStorage persistence
    - Automatic system preference detection via `matchMedia`
    - CSS variable-based theming (pre-existing dark mode support in index.css)
    - Theme selector in Settings → General tab
  - **Dependency Additions**: 125+ new packages including Radix UI component library
    - 40+ @radix-ui/react-* packages (accordion, dialog, dropdown-menu, etc.)
    - shadcn/ui component system with CVA for variants
    - lucide-react icon library
    - TanStack Query for data fetching with 2s refetch intervals
    - recharts (placeholder ready), @xyflow/react (placeholder ready)
  - **Build Performance**:
    - Build time: 3.08s
    - Bundle size: 340.60 KB (102.38 KB gzipped)
    - CSS bundle: 28.57 KB (5.73 KB gzipped)
    - Zero TypeScript errors in strict mode
  - **JobDetail Integration**: Preserved TeraCopy-style chunk map as centerpiece feature
    - 100-cell grid visualization with real-time chunk status updates
    - Color-coded states (green=completed, red=failed, gray=pending)
    - Click-to-detail navigation from JobList
    - Performance metrics and event stream
  - **API Integration**: useJobs hook with real-time data throughout Dashboard, Transfers, Analytics
  - **See**: dashboard/TEST_REPORT.md for comprehensive feature documentation

- **🏗️ Control Plane Compile-Time Modularity** - Build headless or full UI mode with feature flags
  - **New `ui` feature flag** in `crates/orbit-web/Cargo.toml`
  - **Headless Mode (Default)**: API-only server with ~40% smaller binary size
    - No static file serving dependencies
    - Reduced attack surface - no UI code in binary
    - Perfect for Kubernetes, Docker, CI/CD automation
    - Fallback returns helpful 404 message directing users to API endpoints
  - **UI Mode (`--features ui`)**: Full-featured server with embedded dashboard
    - Serves static files from `dashboard/dist`
    - Single-binary deployment with integrated UI
    - Ideal for desktop applications and quick demos
  - **Conditional Compilation**: Uses `#[cfg(feature = "ui")]` for zero runtime overhead
  - **Tower-http `fs` feature**: Only compiled when UI mode is enabled
  - **Informative Logging**: Clear startup messages indicating mode
    - Headless: "⚙️ Headless Mode: Dashboard not included, API-only server"
    - UI: "🎨 UI Feature Enabled: Serving embedded dashboard from dashboard/dist"
  - **Build Commands**:
    - `cargo build --release -p orbit-server` → Headless (~15MB)
    - `cargo build --release -p orbit-server --features ui` → Full UI (~25MB)
  - **See**: README.md section "Compilation Modes: Headless vs Full UI"

- **🎨 Dashboard UI Overhaul** - Complete architectural redesign with professional app shell
  - **STATUS**: 🔴 Pre-Alpha - Experimental features, breaking changes expected
  - **New AppShell Component**: Persistent sidebar navigation replacing top navigation bar
    - Responsive mobile drawer menu with smooth slide-in animations
    - Backdrop overlay with click-to-close functionality
    - Auto-close menu on navigation for better UX
    - Integrated theme toggle and user profile section
  - **Enhanced Job Management**:
    - Real-time search by job ID, source, or destination path
    - Status filter dropdown (All/Running/Pending/Completed/Failed)
    - Manual refresh button for on-demand updates
    - Compact mode showing 5 most recent jobs for dashboard view
    - Improved empty states with icons and helpful messaging
  - **Visual System Health Dashboard**:
    - SVG-based sparkline trend visualizations for metrics
    - Hover effects revealing detailed trend graphs
    - Color-coded health cards (Blue/Green/Orange/Purple)
    - 2-second auto-refresh for live monitoring
  - **Improved Quick Transfer**:
    - Visual source → destination flow with animated connector
    - Color-coded borders (blue for source, orange for destination)
    - Success/error state management (replaced browser alerts)
    - Auto-reset form after successful transfer
    - Better validation and loading states
  - **Redesigned User Management**:
    - User statistics dashboard (Total Users, Admins, Operators)
    - Delete user functionality with confirmation dialogs
    - Gradient avatars with user initials
    - Theme-aware role badges with enhanced visibility
    - Enhanced form layout with better field labeling
  - **Updated Pipeline Editor**:
    - Theme-aware colors throughout (no more hardcoded values)
    - Icon-enhanced toolbar buttons (Database/Zap/Cloud)
    - Node and edge counter in toolbar
    - Improved button styling with hover effects
  - **Embedded Visibility - Mission Control Dashboard**:
    - Live network throughput visualization with SVG area charts
    - Client-side data buffering (30-point history) for smooth "live" feel
    - Real-time metric cards with trend indicators (Active Jobs, Throughput, System Load, Storage Health)
    - Animated status indicators (pulsing green dot for "Live Stream Active")
    - Capacity planning donut chart with used/available space breakdown
    - Peak/Average/Total statistics for comprehensive traffic analysis
  - **Deep-Dive Job Details View**:
    - Visual chunk map with 100-cell grid showing completion progress
    - Color-coded chunk states (green=completed, red=failed, gray=pending)
    - Glowing shadow effects for active chunks
    - Real-time event stream with timestamp and status icons
    - Detailed configuration display (source/destination, mode, compression, verification)
    - Performance metrics (throughput, chunk statistics, timing data)
    - Navigation breadcrumb trail (Job List → Job #N)
  - **Enhanced Job Selection Navigation**:
    - Click-to-expand job details from job list
    - Seamless navigation flow with back button
    - State management for selected job ID
    - Automatic page switching when selecting job
    - Compact mode for dashboard integration
  - **Cockpit-Style App Shell**:
    - Live status indicator (animated pulsing green dot)
    - "System Online" status with visual confirmation
    - Professional operator profile section
    - Prominent pre-alpha warning banner across all views
  - **Mobile-First Responsive Design**: Fully optimized from 320px to 4K displays

### Changed

- **Dependency Updates (December 2024)** - 22 safe dependency updates merged
  - **Cargo Dependencies (12 updates)**:
    - `arrow` 54.x → 57.1.0 - Apache Arrow data structures
    - `aws-sdk-s3` 1.x → 1.113.0 - AWS S3 SDK
    - `governor` 0.6 → 0.10.2 - Rate limiting middleware
    - `notify` 8.x → 8.2.0 - File system notifications
    - `polars` 0.x → 0.52.0 - DataFrame library for analytics
    - `rand` 0.8 → 0.9.2 - Random number generation
    - `sysinfo` 0.35 → 0.37.2 - System information queries
    - `thiserror` 2.0.x → 2.0.17 - Error derive macros
    - `tower` 0.4 → 0.5.2 - Service middleware layer
    - `tower-http` 0.5 → 0.6.7 - HTTP-specific Tower middleware
    - `tracing` 0.1.x → 0.1.43 - Application tracing
    - `utoipa` 5.x → 5.4.0 - OpenAPI documentation
  - **NPM Dependencies (10 updates)**:
    - `react` 18.x → 19.2.1, `react-dom` 18.x → 19.2.1 - UI framework
    - `vite` 6.x → 7.2.7 - Build tool
    - `@vitejs/plugin-react` 4.x → 5.1.2 - React plugin for Vite
    - `@tanstack/react-query` 5.x → 5.90.12 - Data fetching
    - `@xyflow/react` 12.x → 12.10.0 - Flow diagram library
    - `lucide-react` 0.x → 0.556.0 - Icon library
    - `react-resizable-panels` 2.x → 3.0.6 - Resizable panels
    - `jsdom` 25.x → 27.2.0 - DOM testing
    - `vitest` 3.x → 4.0.15 - Testing framework
  - **Deferred (Breaking API Changes)**:
    - `axum-extra` 0.12 - Requires axum 0.8 (breaking web framework upgrade)
    - `bincode` 2.0 - Complete API rewrite requires code refactoring
    - `redb` 3.1 - Transaction API changes need migration
    - `jsonschema` 0.37 - Validation error API redesign
    - `recharts` 3.5.1 - TypeScript conflicts with React 19
  - **Status**: All changes compile cleanly, tests passing, no security vulnerabilities
  - **Cleanup**: 34 dependabot branches merged and deleted
  - See [DEPENDABOT_ISSUES.md](DEPENDABOT_ISSUES.md) for details on deferred updates

- **React 19 Migration & React Flow Upgrade** - Dashboard modernization completed
  - **React**: Already on v19.2.0 (latest stable release)
  - **React Flow**: Migrated from `reactflow` v11.11.4 to `@xyflow/react` v12.9.3
  - **Breaking Change**: The reactflow package has been renamed to @xyflow/react
  - **What Changed**:
    - Updated all imports from `'reactflow'` to `'@xyflow/react'`
    - Updated CSS imports to use new package path
    - Removed Dependabot ignore rules for React major version updates
  - **Verified**: All tests passing, TypeScript compilation clean, ESLint passing
  - **Impact**: Pipeline Editor continues to work with improved React Flow v12 features
  - See [dashboard/README.md](dashboard/README.md) for updated tech stack

- **SMB Protocol v0.11.0 Remediation** - Native SMB client upgraded for long-term stability
  - **Version Bump**: v0.5.0 → v0.5.1
  - **SMB Crate**: Upgraded from v0.10.2 to v0.11.0 for API stability and protocol compliance
  - **API Migration**: Migrated from deprecated `list()` to `Directory::query()` API for directory listings
  - **Retry Logic**: Added 3-attempt connection with exponential backoff (500ms * attempt)
  - **Smart Fallback**: Encryption → Signing fallback for opportunistic security mode
  - **Feature Flags**: Streamlined to essential features (async, encrypt_aesgcm, encrypt_aesccm, std-fs-impls, netbios-transport)
  - **Removed Dependencies**: Eliminated unused bitflags and zeroize dependencies
  - **Type Safety**: Proper Arc<Directory> handling and FileNamesInformation imports
  - **Clippy Clean**: Fixed all linter warnings (match→if-let, redundant guards, io::Error::other)
  - **Security Audit**: No vulnerabilities (cargo audit clean)
  - **Integration Test**: Added tests/smb_v011_check.rs for v0.11.0 query API validation
  - **Documentation**: Updated docs/architecture/SMB_NATIVE_STATUS.md with v0.11.0 status
  - **Build Status**: ✅ Compiles cleanly with `--features smb-native`
  - **What Changed**:
    - Directory listing now uses `Directory::query::<FileNamesInformation>()` instead of deprecated `list()`
    - Connection failures automatically retry up to 3 times with increasing delays
    - SignOnly mode relies on default signing behavior (explicit signing_mode removed)
    - Secret type uses simplified drop implementation without external dependencies
  - **Impact**: More robust SMB connections, future-proof API compatibility, cleaner codebase

### Fixed

- **API Compatibility**: QuickTransfer now uses correct `/create_job` endpoint instead of non-existent `/pipelines/execute`
- **Mobile Navigation**: Hamburger menu now properly opens/closes with state management
- **User Management**: Delete user action now properly wired with confirmation and cache invalidation
- **Theme Consistency**: All components now use theme variables instead of hardcoded colors
- **Full-Screen Layout**: Removed default Vite scaffolding styles (App.css) that constrained dashboard to centered 1280px box
  - Cleared max-width, padding, and text-align constraints blocking AppShell
  - Added tailwindcss-animate plugin for smooth entry animations
  - Added shimmer keyframe for TeraCopy-style progress indicators
  - Dashboard now renders edge-to-edge as designed
- **React 19 Type Safety**: Resolved all ESLint errors for strict React 19 compliance
  - Added proper TypeScript interfaces (SystemHealth, MetricCardProps, HealthCardProps, ActionBtnProps)
  - Fixed setState in useEffect pattern using queueMicrotask() to prevent cascading renders
  - Moved impure Date.now() calls outside render using constants
  - All CI checks passing (ESLint, TypeScript, Prettier)

### Maintenance
- **Dependency Updates** - Merged 15 automated Dependabot updates
  - **GitHub Actions** (5): actions/checkout v4→v6, actions/download-artifact v4→v6, actions/setup-node v4→v6, actions/upload-artifact v4→v5, softprops/action-gh-release v1→v2
  - **Cargo** (8): tracing 0.1.41→0.1.43, aws-sdk-s3 1.112.0→1.113.0, thiserror 1.0.69→2.0.17, rand_core 0.6.4→0.9.3, notify 7.0.0→8.2.0, utoipa 4.2.3→5.4.0, arrow 53.4.1→54.2.1, polars 0.43.1→0.52.0
  - **NPM** (2): vitest 4.0.14→4.0.15, jsdom 26.0.0→27.2.0
  - All updates tested and verified with cargo check and dashboard CI
  - Deferred breaking updates (bincode 2.0, jsonschema 0.37) tracked in separate issues

- **Security Audit** - Comprehensive dependency security review completed
  - **Default build:** Zero runtime vulnerabilities (verified via `cargo tree`)
  - **Optional features:** RSA timing advisory (RUSTSEC-2023-0071) affects `--features smb-native` only
  - Successfully eliminated MySQL dependencies from SQLite-only configurations
  - Reduced unmaintained warnings from 4 to 1 (paste: compile-time only, minimal risk)
  - Updated SECURITY.md with detailed audit results and feature-specific security posture
  - See [SECURITY.md](SECURITY.md#dependency-security-audit) for full analysis

## [2.2.0-rc.1] - 2025-12-03

### Added - Full-Stack CI/CD & Professional File Browser

- **🔄 Full-Stack CI/CD Pipeline**
  - **Dashboard Quality Control**: New `dashboard-quality` job in GitHub Actions
    - Formatting checks with Prettier
    - Linting with ESLint
    - Type checking with TypeScript
    - Security audits with npm audit (high severity threshold)
    - Unit tests with Vitest
    - Production build verification
  - **Rust Security Scanning**: Integrated `cargo-audit` into backend CI pipeline
  - **Local Development Scripts**: Added standardized npm scripts for pre-push validation
    - `npm run ci:check` - Run all checks locally
    - `npm run format:check` / `format:fix` - Code formatting
    - `npm run typecheck` - TypeScript validation
    - `npm run test` - Unit tests with Vitest

- **📁 Professional File Browser Component**
  - **Click-to-Select**: Individual file and folder selection with visual feedback
  - **Folder Selection**: Dedicated "Select Current Folder" button for directory transfers
  - **Up Navigation**: "Go Up" arrow button to navigate parent directories
  - **Visual Enhancements**:
    - Selected items highlighted in blue with dark mode support
    - Hover states for all items
    - CheckCircle icons for selected files/folders
    - Loading spinner with proper error handling
    - Folder icons with blue fill, file icons in gray
    - Item count display in footer
  - **Improved Navigation**: Breadcrumb-style current path display

- **🔌 New Backend API Endpoint**
  - `GET /api/files/list?path={encoded_path}` - RESTful file listing endpoint
  - Query parameter-based path specification (replaces POST with JSON body)
  - Full backwards compatibility maintained with legacy `/api/list_dir` endpoint
  - Proper error handling (404/403/400) for invalid paths
  - Cross-platform support (Windows and Unix filesystems)

### Changed

- **Dashboard Architecture**: FileBrowser now uses GET requests with query parameters
- **Frontend API Integration**: Updated axios calls to match new RESTful endpoints
- **Component Props**: FileBrowser now accepts `selectedPath` prop for visual feedback

### Technical Implementation

- **CI/CD Pipeline** (`.github/workflows/ci.yml`):
  - Added `dashboard-quality` job with Node.js 20 and npm caching
  - Injected `cargo-audit` security scanning into `build-and-test` job
  - Both jobs run in parallel for faster feedback

- **Frontend** (React + TypeScript):
  - New utility: `dashboard/src/lib/utils.ts` (cn helper for Tailwind class merging)
  - Updated `dashboard/src/components/files/FileBrowser.tsx` with professional UI
  - Updated `dashboard/src/components/jobs/QuickTransfer.tsx` to pass selection state
  - Added dev dependencies: `prettier`, `vitest`, `jsdom`
  - Created `vitest.config.ts` for test configuration
  - Created `.prettierrc` for consistent code formatting
  - All code passes TypeScript compilation, ESLint, Prettier, and builds successfully

- **Backend** (Rust):
  - New handler: `list_files_handler` in `crates/orbit-web/src/server.rs`
  - New struct: `ListFilesQuery` for query parameter parsing
  - Route added: `.route("/api/files/list", get(list_files_handler))`
  - Passes `cargo check` and `cargo fmt` validation

### Developer Experience

- **Pre-Push Validation**: Developers can now run `npm run ci:check` locally to catch issues before pushing
- **Consistent Formatting**: Prettier ensures code style consistency across the team
- **Type Safety**: TypeScript strict checks prevent runtime errors
- **Security**: Automated vulnerability scanning for both Rust and Node.js dependencies

### Notes

- Dashboard CI job requires Node.js 20+ and package-lock.json for reproducible builds
- Rust security audit runs on every push to main and all pull requests
- All new code follows existing patterns and conventions
- Full test coverage for file browser navigation and selection

## [2.2.0-beta.1] - 2025-12-03

### Added - Enterprise Platform Features

- **🧠 Intelligence API (Estimations)**
  - `GET /api/estimates/:path` - Predictive job duration estimation endpoint
  - Returns estimated duration (ms), confidence score (0.0-1.0), and historical throughput
  - Foundation for smart scheduling and capacity planning
  - Mock implementation for Beta 1 (production integration planned)

- **👥 Administration API (User Management)**
  - `GET /api/admin/users` - List all users with roles and metadata
  - `POST /api/admin/users` - Create new users with Admin/Operator/Viewer roles
  - Full CRUD operations for multi-user enterprise deployment
  - Role-based access control (RBAC) ready for middleware integration
  - Secure password hashing with Argon2

- **📊 System Health API**
  - `GET /api/stats/health` - Real-time system metrics endpoint
  - Returns active jobs count, bandwidth utilization, system load, and storage health
  - Auto-refreshes every 5 seconds on frontend
  - Foundation for monitoring and alerting systems

- **🎨 Enhanced Dashboard UI**
  - **System Health Widgets**: Four beautiful metric cards on Jobs page
    - Active Jobs counter with live updates
    - Total Bandwidth display (Mbps) with green accent
    - System Load percentage with yellow accent
    - Storage Health status with purple accent
  - **Team Management Interface**: Complete user administration panel
    - User list with role badges and creation timestamps
    - Add user form with username, password, and role selection
    - Delete user functionality (ready for backend integration)
    - SVG icons for visual consistency
  - **Admin Navigation Tab**: New dedicated admin section in dashboard
  - **Version Updated**: Dashboard now displays v2.2.0-beta.1

### Changed

- **Dashboard Architecture**: Jobs page now includes system health overview
- **API Module Structure**: Added `admin`, `estimates`, and `stats` modules
- **Navigation**: Added Admin tab to main dashboard navigation

### Technical Implementation

- **Backend (Rust)**:
  - New modules: `crates/orbit-web/src/api/{admin,estimates,stats}.rs`
  - Updated `crates/orbit-web/src/api/mod.rs` with new exports
  - Routes registered in `crates/orbit-web/src/server.rs`
  - Uses `sqlx` with `user_pool` for database operations
  - Properly formatted with `cargo fmt`
  - Passes `cargo clippy` checks

- **Frontend (React + TypeScript)**:
  - New components:
    - `dashboard/src/components/admin/UserList.tsx` (user management)
    - `dashboard/src/components/dashboard/SystemHealth.tsx` (metrics widgets)
  - Updated `dashboard/src/App.tsx` with admin routing
  - Updated `dashboard/src/components/jobs/JobList.tsx` with health widgets
  - React Query integration for data fetching
  - Tailwind CSS for responsive, beautiful UI

### Notes

- Intelligence API returns mock data for Beta 1 (production integration planned)
- System Health metrics are static for Beta 1 (will query Magnetar in production)
- User management API is fully functional and ready for production use
- All new code follows existing patterns and conventions

## [2.2.0-alpha.2] - 2025-12-03

### Added - Dashboard "Face" Implementation

- **🎨 React Dashboard Components**
  - **Visual Pipeline Editor**: Full React Flow integration with drag-and-drop node creation
    - Support for Source, Transform, and Destination node types
    - Real-time edge connections with visual feedback
    - Background grid and zoom/pan controls
  - **File Browser Component**: Interactive filesystem navigation
    - Supports both Windows and Unix filesystems
    - Directory traversal with parent navigation (..)
    - File size display and icon differentiation
    - Connected to backend `/api/list_dir` endpoint
  - **Job Wizard**: Interactive job creation workflow
    - Dual file browser panels for source/destination selection
    - Real-time validation of job parameters
    - Success/error feedback with TanStack Query mutations
  - **Job List Component**: Real-time job monitoring
    - Auto-refresh every 2 seconds for active jobs
    - Progress bars with chunk completion tracking
    - Status-based color coding (pending/running/completed/failed/cancelled)
    - Run/Cancel/Delete actions with optimistic updates

- **🔧 API Integration Layer**
  - **Custom React Hooks** (`hooks/useJobs.ts`)
    - `useJobs()`: Auto-refreshing job list query
    - `useCreateJob()`: Job creation with cache invalidation
    - `useRunJob()`, `useCancelJob()`, `useDeleteJob()`: Job lifecycle management
  - **Axios Instance** with request/response interceptors
    - JWT token injection from localStorage
    - Automatic 401 redirect to login page
    - Base URL configuration for API endpoint

- **🎯 Frontend Infrastructure**
  - **TanStack Query Setup**: Query client with smart defaults
  - **Tailwind v4 Support**: Updated PostCSS configuration for `@tailwindcss/postcss`
  - **TypeScript Strict Mode**: All components type-safe with proper imports
  - **Simple Navigation**: Tab-based routing between Jobs/Create/Pipelines views
  - **Production Build**: Optimized Vite build (419KB bundle, 136KB gzipped)

### Changed

- **Updated PostCSS Configuration**
  - Migrated from `tailwindcss` to `@tailwindcss/postcss` plugin
  - Resolves Tailwind v4 compatibility issues

### Backend (Already Implemented)

- **File Explorer API** (already existed in [server.rs:419-526](c:\orbit\crates\orbit-web\src\server.rs#L419-L526))
  - `/api/list_dir` (POST): Directory listing with metadata
  - `/api/list_drives` (GET): System drive enumeration
  - Windows and Unix filesystem support
  - Parent directory navigation with ".." entry

### Architecture Shift - Orbit Control Plane (v2.2.0-alpha) [BREAKING]
- **✂️ The Separation**: Split monolithic `orbit-web` into `orbit-server` (Backend) and `orbit-dashboard` (Frontend).
  - **Motivation**: Decouples UI release cycle from Core engine stability.
  - **Performance**: Removed WASM/SSR overhead; Dashboard is now purely static assets served via CDN/Nginx.
  - **Breaking**: `orbit serve` now launches the API only. UI must be hosted separately or embedded via `static` assets.

- **🧠 Orbit Control Plane (`orbit-server`)**
  - **OpenAPI 3.0**: Fully documented REST API available at `/swagger-ui`.
  - **Intelligent Scheduler**: New background service for job prioritization and estimation.
  - **CORS Support**: Enabled by default for development (allow-all) and configurable for production.
  - **Duration Estimation API**: `/api/jobs/:id/estimate` endpoint provides intelligent transfer predictions.
  - **Confidence Scoring**: Estimation API includes confidence scores based on historical Magnetar data.
  - **Bottleneck Detection**: Proactive warnings for potential performance bottlenecks.

- **🎨 Orbit Dashboard (`orbit-dashboard`)**
  - **New Stack**: React 18, Vite, TypeScript, TanStack Query.
  - **Visual Pipeline Builder**: Migrated from custom WASM to **React Flow** for robust DAG editing.
  - **Real-time**: WebSocket integration for sub-50ms progress updates.
  - **Smart Data Fetching**: Adaptive polling (1s for active jobs, 5s for idle state).
  - **Modern UI**: Tailwind CSS + Shadcn/UI (Radix Primitives) for professional design.
  - **Optimistic Updates**: Instant UI feedback with automatic cache invalidation.

- **🔧 Developer Experience**
  - **Unified Dev Script**: `launch-orbit.sh` / `launch-orbit.bat` runs both backend and frontend concurrently.
  - **Hot Module Replacement**: Vite provides instant feedback during UI development.
  - **API Contract**: OpenAPI schema generation ensures frontend/backend type safety.
  - **Zero Downtime Deployments**: Static dashboard can be updated without restarting control plane.

### Added - Retry Logic Optimization

- **⚡ Permanent Error Fast-Fail** - Retry logic now distinguishes permanent vs transient errors
  - **Permanent errors skip retries**: `PermissionDenied`, `AlreadyExists`, `NotFound` (I/O variants) fail immediately
  - **Transient errors still retry**: `TimedOut`, `ConnectionRefused`, `Interrupted` benefit from full retry mechanism
  - **Transient-First Policy**: Decision flow enforces Fatal → Permanence → Budget → Retry checks
  - **Performance impact**: Eliminates wasted retry cycles (saves 35+ seconds per permission error with 3 retries)
  - **User experience**: Faster failure feedback for configuration issues
  - New integration test suite: `tests/retry_efficiency_test.rs` (5/5 tests passing)
  - Updated error classification tests: `test_permanent_io_errors()`, `test_transient_io_errors()`
  - Full specification: `docs/specs/RETRY_OPTIMIZATION_SPEC.md`
  - Implementation: Enhanced permanence check in `src/core/retry.rs:92-112`

### Added - Orbit V2.1: Universe Scalability Upgrade

- **🌌 Universe V3: High-Cardinality Architecture** - New `core-starmap::universe_v3` module
  - **Solves O(N²) Write Amplification**: V2 required reading/deserializing/reserializing entire location lists on every insert
  - **Multimap Table Design**: Uses `redb::MultimapTableDefinition` for discrete location entries
  - **O(log N) Insert Performance**: Constant-time inserts regardless of duplicate count (vs O(N) in V2)
  - **Streaming Iteration**: `scan_chunk()` callback API for O(1) memory usage during reads
  - **Iterator API**: `find_chunk()` returns `LocationIter` for flexible consumption patterns
  - **Production Scalability**: Handles billions of chunks with millions of duplicates per chunk
  - **ACID Guarantees Maintained**: Full transaction support via redb
  - **Version 3 Schema**: Incompatible with V2 (version check prevents corruption)

- **📊 Performance Improvements (V2 → V3)**
  - **Insert Complexity**: O(N) data size → O(log N) B-tree depth
  - **Memory Usage**: O(N) all loaded → O(1) streamed
  - **100k Duplicate Test**: Minutes/timeout → Seconds
  - **Benchmark Results** (20,000 duplicates):
    - First batch: 1.41s
    - Last batch: 0.77s (0.55x ratio - performance *improved* with scale!)
    - V2 expected ratio: ~200x (quadratic degradation)
    - Data integrity: 100% (all 20,000 entries verified)
  - **Per-Insert Latency**: ~1ms consistently across all duplicate counts

- **🧪 Comprehensive Test Suite** - `tests/scalability_test.rs`
  - High-cardinality torture test (20,000 duplicates with performance regression detection)
  - Mixed workload validation (different chunks with varying duplicate counts)
  - Streaming memory efficiency test (early-exit verification)
  - Persistence across database restarts
  - Performance benchmarking across multiple scales
  - 5/5 tests passing with release optimizations

- **📖 Technical Documentation** - `docs/architecture/SCALABILITY_SPEC.md`
  - Problem statement with complexity analysis
  - Multimap architecture design
  - Migration strategy from V2
  - Performance expectations and benchmarks
  - Future optimization roadmap

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
