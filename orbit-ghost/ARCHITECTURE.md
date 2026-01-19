# Orbit GhostFS Architecture

## Executive Summary

Orbit GhostFS transforms remote data access from a "store-then-process" to a "process-while-moving" paradigm through FUSE-based virtualization and just-in-time block fetching.

**Current Status:** Phase 2 (Materialization Layer) - Database-backed metadata via Magnetar integration

**Version:** 0.1.0
**Platform Support:** Linux (libfuse3), macOS (macFUSE)

## Core Architectural Principles

### 1. Illusion of Locality
The filesystem projects remote data as if it exists locally. Applications interact via standard POSIX calls, unaware that data is being fetched on-demand.

### 2. Lazy Materialization
Data blocks are only transferred when accessed. A 1TB dataset with 1GB of actual usage transfers only 1GB.

### 3. Priority Inversion
User-initiated reads preempt background sequential transfers, ensuring responsive interactive performance.

## Project Structure

```
orbit-ghost/
├── src/
│   ├── main.rs        # CLI entry point & initialization
│   ├── fs.rs          # FUSE filesystem implementation (OrbitGhostFS)
│   ├── entangler.rs   # Block request coordination (Entangler)
│   ├── translator.rs  # Inode ↔ Artifact ID mapping (InodeTranslator)
│   ├── oracle.rs      # Metadata abstraction trait (MetadataOracle)
│   ├── adapter.rs     # SQLite/Magnetar implementation (MagnetarAdapter)
│   ├── inode.rs       # File metadata structures (GhostEntry, GhostFile)
│   ├── config.rs      # Configuration types (GhostConfig)
│   └── error.rs       # Error types & errno mapping (GhostError)
├── Cargo.toml
└── [documentation files]
```

## Dependencies

| Category | Crates | Purpose |
|----------|--------|---------|
| **FUSE** | `fuser 0.16` | Safe FUSE bindings |
| **Async** | `tokio (full)` | Async runtime |
| **Database** | `sqlx 0.8.1 (SQLite)` | Magnetar DB queries |
| **Concurrency** | `dashmap`, `parking_lot`, `crossbeam-channel` | Lock-free structures |
| **CLI** | `clap 4.0 (derive)` | Argument parsing |
| **Error Handling** | `thiserror`, `anyhow` | Error types |
| **Async Traits** | `async-trait` | Async trait support |

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Application Layer                       │
│  (Video Player, Data Science Tools, IDEs, etc.)             │
└─────────────────────┬───────────────────────────────────────┘
                      │ Standard POSIX syscalls
                      │ (open, read, stat, readdir)
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    Kernel VFS Layer                          │
└─────────────────────┬───────────────────────────────────────┘
                      │ FUSE Protocol
                      ▼
┌─────────────────────────────────────────────────────────────┐
│             Ghost Driver [fs.rs: OrbitGhostFS]               │
│  - lookup()    : Database-backed file resolution            │
│  - getattr()   : Metadata from Magnetar DB                  │
│  - readdir()   : Database query with lazy inode allocation  │
│  - read()      : Triggers block entanglement                │
└─────────────────────┬───────────────────────────────────────┘
                      │
          ┌───────────┴───────────┐
          ▼                       ▼
┌─────────────────────┐  ┌───────────────────────────────────┐
│ Inode Translation   │  │      Materialization Layer        │
│ [translator.rs]     │  │                                   │
│                     │  │  ┌───────────────────────────┐    │
│ InodeTranslator     │  │  │ MetadataOracle [oracle.rs]│    │
│ - inode_to_id       │  │  │ (trait abstraction)       │    │
│ - id_to_inode       │  │  │ - get_root_id()           │    │
│ - next_inode        │  │  │ - lookup()                │    │
│   (AtomicU64)       │  │  │ - readdir()               │    │
│                     │  │  │ - getattr()               │    │
│ Lazy allocation:    │  │  └───────────┬───────────────┘    │
│ - DashMap for O(1)  │  │              │                    │
│ - Root inode = 1    │  │  ┌───────────▼───────────────┐    │
└─────────────────────┘  │  │ MagnetarAdapter           │    │
                         │  │ [adapter.rs]              │    │
                         │  │ - SqlitePool connection   │    │
                         │  │ - Job-based filtering     │    │
                         │  │ - Async/sync bridge       │    │
                         │  └───────────────────────────┘    │
                         └───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│               Entangler Core [entangler.rs]                  │
│  ┌──────────────────────────────────────────────┐           │
│  │  Block Request Manager                       │           │
│  │  - Calculate block range from offset         │           │
│  │  - Check local cache (file existence)        │           │
│  │  - Queue priority fetch if missing           │           │
│  │  - Block thread until data arrives           │           │
│  └──────────────────┬───────────────────────────┘           │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │  Waiting Rooms (polling synchronization)     │           │
│  │  Key: BlockRequest{file_id, block_index}     │           │
│  │  Value: Arc<Condvar> (future: true wait)     │           │
│  └──────────────────────────────────────────────┘           │
└─────────────────────┬───────────────────────────────────────┘
                      │ Priority Signal (BlockRequest)
                      │ via crossbeam-channel (unbounded)
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            Wormhole Transport [main.rs thread]               │
│                                                              │
│  Background Thread Loop:                                    │
│  1. Block on priority_rx.recv()                             │
│  2. Simulate network latency (500ms)                        │
│  3. Generate dummy block data (1MB)                         │
│  4. Write to cache: {cache_dir}/{file_id}_{block}.bin       │
│  5. Loop back to step 1                                     │
│                                                              │
│  Production TODO:                                           │
│  - Thread pool for parallel block downloads                 │
│  - Real Orbit backend protocol integration                  │
│  - Adaptive prefetching based on access patterns            │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    Block Cache Layer                         │
│  {cache_dir}/{file_id}_{block_index}.bin                    │
│  Default: /tmp/orbit_cache/                                 │
│                                                              │
│  Production TODO:                                           │
│  - LRU eviction policy                                      │
│  - Configurable cache size limits                           │
│  - Block verification (checksums)                           │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow: Read Operation

### Scenario: Application reads bytes 52,428,800 - 52,429,000 from a 50MB file

**Step-by-step execution:**

```
1. Application: tail -c 200 /tmp/orbit_ghost_mount/file.dat
   ↓
2. Kernel VFS: Issues FUSE read(offset=52428600, size=200)
   ↓
3. OrbitGhostFS.read():
   - Calculate blocks: start_block = 52428600 / 1048576 = 50
                       end_block   = 52428799 / 1048576 = 50
   - Only block 50 is needed
   ↓
4. Entangler.ensure_block_available("file_123", 50):
   - Check cache: {cache_dir}/file_123_50.bin exists?
   - NO → Send BlockRequest{file_id: "file_123", block_index: 50}
   - Create Condvar for this block
   - Enter polling loop (10ms sleep)
   ↓
5. Wormhole receives priority signal:
   - Receive BlockRequest from priority_rx channel
   - Simulate 500ms network latency
   - Generate 1MB dummy block data
   - Write to {cache_dir}/file_123_50.bin
   ↓
6. Entangler polling loop detects file exists:
   - Exit polling loop
   - Return control to read() handler
   ↓
7. OrbitGhostFS.read() continues:
   - Open {cache_dir}/file_123_50.bin
   - Read entire block into memory
   - Slice to relative offset: 52428600 % 1048576 = 824
   - Return 200 bytes to FUSE
   ↓
8. Kernel VFS returns data to application
   ↓
9. Application receives data instantly (500ms total latency)
   vs traditional full download (50MB @ 10MB/s = 5 seconds)
```

## Module Breakdown

### inode.rs - Virtual File Representation

**Purpose:** Data structures bridging manifest metadata to FUSE attributes.

**Key Structures:**
```rust
// Primary structure (database-backed)
pub struct GhostEntry {
    pub id: String,       // Artifact ID
    pub name: String,     // Filename
    pub size: u64,        // File size in bytes
    pub is_dir: bool,     // Directory flag
    pub mtime: u64,       // Unix timestamp
}

// Legacy structure (in-memory manifest)
pub struct GhostFile {
    pub name: String,
    pub size: u64,
    pub orbit_id: String,
    pub is_dir: bool,
    pub blocks_present: Vec<bool>,
}
```

**Key Method:**
```rust
impl GhostEntry {
    pub fn to_attr(&self, inode: u64) -> FileAttr
    // Converts to FUSE FileAttr with:
    // - kind: RegularFile or Directory
    // - perm: 0o755 (dirs), 0o644 (files)
    // - uid/gid: 501/20 (hardcoded demo values)
}
```

### translator.rs - Inode Translation Layer

**Purpose:** Bidirectional mapping between FUSE inodes (u64) and Orbit artifact IDs (String).

**Key Structure:**
```rust
pub struct InodeTranslator {
    inode_to_id: DashMap<u64, String>,  // Forward lookup
    id_to_inode: DashMap<String, u64>,  // Reverse lookup
    next_inode: AtomicU64,              // Allocator counter
}
```

**Key Methods:**
```rust
pub fn new() -> Self
// Pre-allocates root inode: 1 → "root"

pub fn get_or_allocate(&self, artifact_id: &str) -> u64
// Fast-path: Check reverse map
// Slow-path: Allocate atomically via fetch_add

pub fn to_artifact_id(&self, inode: u64) -> Result<String, GhostError>
// Reverse lookup, returns InvalidInode error if not found
```

**Design Notes:**
- Lock-free (DashMap + AtomicU64)
- Enables lazy allocation without full database scan at mount
- Stable inodes per session (same artifact ID = same inode)

### oracle.rs - Metadata Abstraction Trait

**Purpose:** Pluggable interface for metadata storage backends.

**Trait Definition:**
```rust
#[async_trait]
pub trait MetadataOracle: Send + Sync {
    async fn get_root_id(&self) -> Result<String, GhostError>;
    async fn lookup(&self, parent_id: &str, name: &str)
        -> Result<Option<GhostEntry>, GhostError>;
    async fn readdir(&self, parent_id: &str)
        -> Result<Vec<GhostEntry>, GhostError>;
    async fn getattr(&self, id: &str)
        -> Result<GhostEntry, GhostError>;
}
```

**Design Notes:**
- Enables swapping backends (SQLite, remote catalog, mock)
- All methods are async for database query compatibility
- Used by OrbitGhostFS via Arc<dyn MetadataOracle>

### adapter.rs - Magnetar Database Adapter

**Purpose:** Implements MetadataOracle for Magnetar SQLite database.

**Key Structure:**
```rust
pub struct MagnetarAdapter {
    pool: SqlitePool,        // Connection pool
    job_id: i64,             // Filter scope (multi-tenancy)
    config: GhostConfig,     // Timeouts & retries
}
```

**Key Methods:**
```rust
pub async fn new(db_path: &str, job_id: i64) -> Result<Self>
// Creates SqlitePool, validates connection

// MetadataOracle implementation:
async fn get_root_id(&self) -> Result<String>
// Query: SELECT id FROM artifacts WHERE parent_id IS NULL

async fn lookup(&self, parent_id: &str, name: &str) -> Result<Option<GhostEntry>>
// Query: WHERE parent_id = ? AND name = ?

async fn readdir(&self, parent_id: &str) -> Result<Vec<GhostEntry>>
// Query: WHERE parent_id = ? ORDER BY name ASC

async fn getattr(&self, id: &str) -> Result<GhostEntry>
// Query: WHERE id = ?
```

**Expected Database Schema:**
```sql
artifacts (
    id TEXT PRIMARY KEY,
    job_id INTEGER,
    parent_id TEXT,      -- NULL for root
    name TEXT,
    size INTEGER,
    is_dir INTEGER,
    mtime INTEGER        -- Unix timestamp
)
```

### error.rs - Error Handling

**Purpose:** Custom error types with errno mapping for FUSE.

**Error Enum:**
```rust
pub enum GhostError {
    Database(sqlx::Error),
    NotFound(String),
    Timeout(Duration),
    InvalidInode(u64),
    Io(std::io::Error),
}
```

**Errno Mapping:**
```rust
impl GhostError {
    pub fn to_errno(&self) -> i32 {
        match self {
            NotFound(_) | InvalidInode(_) => libc::ENOENT,
            Timeout(_) => libc::ETIMEDOUT,
            Database(_) | Io(_) => libc::EIO,
        }
    }
}
```

### config.rs - Configuration Types

**Purpose:** Runtime configuration parameters.

**Structure:**
```rust
pub struct GhostConfig {
    pub db_timeout: Duration,        // Default: 5s
    pub max_retries: usize,          // Default: 3
    pub backoff_multiplier: u32,     // Default: 2x
    pub initial_backoff: Duration,   // Default: 100ms
}
```

**Status:** Defined but not yet actively used. Future: TOML file support.

### entangler.rs - Block Coordination Logic

**Purpose:** Synchronizes FUSE threads with the download pipeline.

**Key Structures:**
```rust
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BlockRequest {
    pub file_id: String,
    pub block_index: u64,
}

pub struct Entangler {
    priority_tx: Sender<BlockRequest>,
    waiting_rooms: Arc<Mutex<HashMap<BlockRequest, Arc<Condvar>>>>,
    cache_path: String,
}
```

**Critical Method:**
```rust
pub fn ensure_block_available(&self, file_id: &str, block_index: u64)
```

**Behavior:**
1. Fast-path: If block exists in cache (`{cache_path}/{file_id}_{block_index}.bin`), return immediately
2. Slow-path:
   - Create Condvar for this block
   - Send BlockRequest via priority channel to Wormhole
   - Poll with 10ms sleep until block appears on disk
   - Return when block materializes

**Current Limitations:**
- Uses polling loop (10ms sleep) instead of true Condvar::wait()
- No timeout handling (can block indefinitely)
- No duplicate request deduplication

**Production Improvements:**
- Replace polling loop with true Condvar::wait() + notify_one()
- Add timeout (return EIO after N seconds)
- Handle duplicate requests (multiple threads reading same block)
- Implement backpressure if queue is full

### fs.rs - FUSE Interface Implementation

**Purpose:** Translates POSIX filesystem operations to database queries and block fetches.

**Key Structure:**
```rust
pub struct OrbitGhostFS {
    oracle: Arc<dyn MetadataOracle>,        // Database abstraction
    translator: Arc<InodeTranslator>,       // Inode mapping
    entangler: Arc<Entangler>,              // Block coordination
    runtime_handle: tokio::runtime::Handle, // Async/sync bridge
    cache_path: String,                     // Block cache location
}
```

**Constants:**
```rust
const BLOCK_SIZE: u64 = 1024 * 1024;  // 1MB blocks
const TTL: Duration = Duration::from_secs(1);  // Attribute cache TTL
```

**Implemented Operations:**

1. **lookup(parent, name):**
   - Translate parent inode → artifact_id via InodeTranslator
   - Query database via MetadataOracle::lookup()
   - Allocate inode for result via get_or_allocate()
   - Return entry attributes

2. **getattr(inode):**
   - Special case: Root inode (1) returns hardcoded attributes
   - Otherwise: Translate inode → artifact_id, query database
   - O(1) inode translation + async database query

3. **readdir(inode):**
   - Add synthetic "." and ".." entries
   - Query database for children via MetadataOracle::readdir()
   - Allocate inodes lazily for each child
   - Users see full directory instantly (metadata only, no block I/O)

4. **read(inode, offset, size):** *(Most critical operation)*
   - Translate inode → artifact_id
   - Calculate affected blocks: `start_block..=end_block`
   - For each block:
     - Call `entangler.ensure_block_available()` (may block)
     - Read block from cache file
   - Slice exact byte range from block data
   - Return data to kernel

**Helper Method:**
```rust
fn block_on<F, T>(&self, future: F) -> Result<T, i32>
// Runs async code synchronously via tokio runtime handle
// Converts GhostError → errno
```

**Performance Characteristics:**
- `lookup/getattr`: O(1) translation + async DB query (~1ms)
- `readdir`: O(n) in file count, but instant (no block I/O)
- `read`: O(blocks) × network_latency, parallelizable per block

### main.rs - System Initialization

**Purpose:** Bootstrap the entire ghost filesystem with CLI argument parsing.

**CLI Interface:**
```rust
#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "magnetar.db")]
    database: String,

    #[arg(short, long)]
    job_id: i64,  // Required

    #[arg(short, long, default_value = "/tmp/orbit_ghost_mount")]
    mount_point: String,

    #[arg(short, long, default_value = "/tmp/orbit_cache")]
    cache_dir: String,
}
```

**Usage:**
```bash
orbit-ghost --job-id <ID> [OPTIONS]

Options:
  -d, --database <PATH>    [default: magnetar.db]
  -j, --job-id <ID>        (required)
  -m, --mount-point <PATH> [default: /tmp/orbit_ghost_mount]
  -c, --cache-dir <PATH>   [default: /tmp/orbit_cache]
```

**Execution Flow:**

1. **Setup:**
   - Parse CLI arguments via clap
   - Initialize logging via env_logger (reads RUST_LOG)
   - Create mount point and cache directories

2. **Database Connection:**
   - Create MagnetarAdapter with SqlitePool
   - Query root artifact to verify connection
   - Filter by job_id for multi-tenancy

3. **Component Initialization:**
   - Create InodeTranslator (pre-allocates root inode)
   - Create unbounded crossbeam channel for priority requests
   - Create Entangler with channel sender

4. **Wormhole Thread:**
   - Spawns background thread
   - Infinite loop: `recv()` → simulate 500ms latency → write block to cache
   - Production: Replace with thread pool + real backend

5. **FUSE Mount:**
   - Assemble OrbitGhostFS with all components
   - Call `fuser::mount2()` (blocking call)
   - Kernel routes syscalls to handlers until unmount

## Concurrency Model

### Thread Safety Guarantees

| Component | Strategy | Safety |
|-----------|----------|--------|
| **InodeTranslator.inode_to_id** | DashMap (lock-free) | Multiple concurrent lookups ✅ |
| **InodeTranslator.id_to_inode** | DashMap (lock-free) | Multiple concurrent allocations ✅ |
| **InodeTranslator.next_inode** | AtomicU64 | fetch_add(SeqCst) prevents races ✅ |
| **Entangler.waiting_rooms** | Mutex\<HashMap\> | Brief lock for insert/remove only ✅ |
| **priority_tx channel** | crossbeam (MPMC) | Multiple FUSE threads → 1 Wormhole ✅ |
| **Cache files** | Filesystem-based | read() race-free, no concurrent writes ✅ |

### Deadlock Prevention

- No nested locks
- Entangler holds mutex only briefly (insert Condvar, then release)
- FUSE threads block on cache file existence, not on mutexes
- AtomicU64 for inode allocation avoids contention

### Scalability Analysis

**Current Implementation:**
- Single Wormhole thread (bottleneck for parallel requests)
- Polling loop in Entangler (10ms sleep, CPU inefficient)
- No duplicate request deduplication

**Production Scaling:**
- Thread pool: 4-16 download threads
- True Condvar::wait() with notify_one() from Wormhole
- Batch block requests to reduce channel overhead
- Deduplication: Check waiting_rooms before sending request

## Performance Tuning

### Block Size Selection

**Trade-offs:**

| Block Size | Pros | Cons |
|------------|------|------|
| 64 KB | Low latency for random access | High HTTP request overhead |
| 1 MB | Balanced (current default) | Good for most workloads |
| 16 MB | Aligns with S3 multipart | Wastes bandwidth for small reads |

**Recommendation:**
- 1 MB for general use
- 16 MB for cloud object storage backends
- 64 KB for latency-sensitive applications

### Prefetch Strategies

**Heuristic 1: Linear Sequential**
```
If read(block N) follows read(block N-1):
    Prefetch blocks N+1, N+2, N+3
```

**Heuristic 2: Stride Detection**
```
If pattern: 0, 10, 20, 30 (stride=10):
    Prefetch 40, 50, 60
```

**Heuristic 3: ML-Based**
```
Train model on access patterns
Predict next K blocks with confidence score
Prefetch if confidence > threshold
```

### Cache Management

**Current:** Unlimited cache in configurable directory (default: /tmp/orbit_cache), grows forever

**Production:**
- LRU eviction when cache exceeds size limit
- Priority: Keep blocks near current read position
- Persist cache across mounts (survive reboot)

## Error Handling

### GhostError Types (error.rs)

The crate defines a custom error enum with errno mapping:

```rust
pub enum GhostError {
    Database(sqlx::Error),   // → libc::EIO
    NotFound(String),        // → libc::ENOENT
    Timeout(Duration),       // → libc::ETIMEDOUT
    InvalidInode(u64),       // → libc::ENOENT
    Io(std::io::Error),      // → libc::EIO
}
```

### Network Failures

**Current Behavior:** Infinite blocking in `ensure_block_available()`

**Production Fix:**
```rust
// Add timeout using GhostError
let timeout = Duration::from_secs(30);
let start = Instant::now();

loop {
    if self.is_block_on_disk(file_id, block_index) {
        return Ok(());
    }
    if start.elapsed() > timeout {
        return Err(GhostError::Timeout(timeout));
    }
    thread::sleep(Duration::from_millis(10));
}
```

**FUSE Handler (block_on helper):**
```rust
fn block_on<F, T>(&self, future: F) -> Result<T, i32>
where F: Future<Output = Result<T, GhostError>>
{
    self.runtime_handle.block_on(future)
        .map_err(|e| e.to_errno())
}
```

### Corrupted Blocks

**Detection:** Verify block checksum after download

**Recovery:**
1. Delete corrupted block from cache
2. Re-fetch from backend
3. If re-fetch fails, return EIO

### Mount Failures

**Common Issues:**
- FUSE not installed → Clear error message
- Mount point doesn't exist → Auto-create
- Mount point not empty → Fail with diagnostic
- Insufficient permissions → Suggest `sudo` or user_allow_other

## Platform-Specific Implementation

### Linux
- **FUSE Version:** libfuse3 (3.x API)
- **System Call:** `mount.fuse3`
- **Unmount:** `fusermount3 -u` (or `fusermount` for older systems)
- **Permissions:** Requires `/dev/fuse` access

### macOS
- **FUSE Version:** macFUSE (OSXFUSE successor)
- **System Call:** `mount_macfuse`
- **Unmount:** `umount`
- **Security:** May require TCC permissions, Kernel Extension approval

### Windows (Future)
- **Technology:** WinFSP (Windows File System Proxy)
- **Rust Crate:** `winfsp-rs` (experimental)
- **Alternative:** NFS server emulation layer

**Implementation Paths:**

1. **Option A: WinFSP Bindings**
   - Pros: Native filesystem integration
   - Cons: Requires kernel driver installation, complex API

2. **Option B: NFS Emulation**
   - Pros: No kernel drivers needed, works with Windows built-in NFS client
   - Cons: Network stack overhead, authentication complexity

3. **Option C: Windows Filter Driver**
   - Pros: Deepest integration, highest performance
   - Cons: Kernel-mode development, driver signing requirements

## Integration with Orbit Ecosystem

### Phase 1: Standalone Demo ✅
- ~~Hardcoded manifest~~ → Database-backed
- Simulated backend (generates dummy data)
- ~~Fixed mount point~~ → CLI configurable

### Phase 2: Magnetar Integration ✅ (Current)
- Load manifest from Magnetar SQLite catalog
- Map Orbit artifact IDs to cache files
- Job-based filtering for multi-tenancy
- Async/sync bridge via tokio runtime

### Phase 3: Backend Protocol (Planned)
- Replace simulated download with real Orbit transfer
- Support resumable block fetches
- Handle backend connection pooling

### Phase 4: Production Hardening (Planned)
- Replace polling with Condvar::wait()
- Add timeout to ensure_block_available()
- Thread pool for parallel downloads
- LRU cache eviction
- Add metrics/observability (Prometheus)
- Support configuration files (TOML)
- Add systemd service unit

## Security Considerations

### Threat Model

1. **Data Integrity:**
   - Verify block checksums against manifest
   - Detect MITM attacks during transfer

2. **Access Control:**
   - Respect manifest permissions
   - Integrate with host OS user/group system

3. **Cache Poisoning:**
   - Isolate cache per user
   - Permissions: 0600 on cache files

4. **Resource Exhaustion:**
   - Limit cache size
   - Implement request rate limiting

### Mitigation Strategies

- TLS for all backend communication
- HMAC verification of blocks
- Sandboxed cache directory (chmod 0700)
- Memory limits on block buffers

## Testing Strategy

**Dev Dependencies:** `tempfile`, `criterion` (benchmarking), `assert_fs`, `predicates`

### Unit Tests
- Inode mapping correctness (InodeTranslator)
- Block range calculation
- Cache hit/miss logic
- GhostError → errno mapping

### Integration Tests
- Full mount/unmount cycle
- Read operations with simulated backend
- Concurrent access from multiple threads
- Database query correctness

### Performance Tests
- Benchmark: Random vs sequential reads
- Measure: Latency distribution for block fetch
- Profile: CPU usage of Entangler polling

### Chaos Testing
- Network interruptions during block fetch
- Corrupted cache files
- Out-of-space conditions

## Deployment Checklist

Before production deployment:

- [x] Integrate with Magnetar manifest loading
- [x] CLI argument parsing (database, job_id, mount_point, cache_dir)
- [x] Inode translation layer (lazy allocation)
- [x] Error handling with errno mapping
- [x] Async/sync bridge via tokio runtime
- [ ] Replace polling loop with Condvar::wait()
- [ ] Add timeout to ensure_block_available()
- [ ] Implement LRU cache eviction
- [ ] Add configuration file support (TOML)
- [ ] Connect to real Orbit backend protocol
- [ ] Add Prometheus metrics
- [ ] Write systemd service unit
- [ ] Document installation for target platforms
- [ ] Security audit of FUSE handlers
- [ ] Fuzz testing of read edge cases
- [ ] Load testing with realistic workloads

## Future Enhancements

### Adaptive Block Sizing
Dynamically adjust block size based on access patterns:
- Large sequential reads → Increase block size
- Random access → Decrease block size

### Write Support (Read-Write Mode)
Currently read-only. Adding write support requires:
1. Write-back cache with dirty block tracking
2. Conflict resolution for concurrent writes
3. Integration with Orbit versioning system

### Multi-Source Redundancy
Fetch blocks from multiple replicas in parallel:
- Use fastest responding server
- Fallback if primary fails
- Implement erasure coding for partial data

### ML-Powered Prefetching
Train models on access patterns:
- Predict next N blocks
- Prefetch proactively
- Adapt to user behavior

## Conclusion

Orbit GhostFS demonstrates that remote data can be accessed with local-like latency through intelligent block-level virtualization. The architecture separates concerns across 9 focused modules:

- **fs.rs**: FUSE interface translation (OrbitGhostFS)
- **entangler.rs**: Block coordination (Entangler)
- **translator.rs**: Bidirectional inode mapping (InodeTranslator)
- **oracle.rs**: Pluggable metadata abstraction (MetadataOracle trait)
- **adapter.rs**: SQLite implementation (MagnetarAdapter)
- **inode.rs**, **error.rs**, **config.rs**: Supporting data structures

**Current Status (Phase 2):**
- Database-backed metadata via Magnetar integration
- CLI with configurable paths
- Lazy inode allocation
- Error handling with errno mapping

**Next Steps:**
- Replace polling with Condvar::wait()
- Thread pool for parallel downloads
- LRU cache eviction
- Real Orbit backend protocol

**Key Takeaway:** By inverting the traditional transfer model (fetch-all → process) to (process → fetch-on-demand), we enable instant interaction with arbitrarily large remote datasets.
