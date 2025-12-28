# Orbit GhostFS Architecture

## Executive Summary

Orbit GhostFS transforms remote data access from a "store-then-process" to a "process-while-moving" paradigm through FUSE-based virtualization and just-in-time block fetching.

## Core Architectural Principles

### 1. Illusion of Locality
The filesystem projects remote data as if it exists locally. Applications interact via standard POSIX calls, unaware that data is being fetched on-demand.

### 2. Lazy Materialization
Data blocks are only transferred when accessed. A 1TB dataset with 1GB of actual usage transfers only 1GB.

### 3. Priority Inversion
User-initiated reads preempt background sequential transfers, ensuring responsive interactive performance.

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
│                Ghost Driver (FUSE Handler)                   │
│  - lookup()    : Manifest-based file resolution             │
│  - getattr()   : Metadata without network I/O               │
│  - readdir()   : Instant directory listing                  │
│  - read()      : Triggers block entanglement                │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                     Entangler Core                           │
│  ┌──────────────────────────────────────────────┐           │
│  │  Block Request Manager                       │           │
│  │  - Calculate block range from offset         │           │
│  │  - Check local cache (bitmap lookup)         │           │
│  │  - Queue priority fetch if missing           │           │
│  │  - Block thread until data arrives           │           │
│  └──────────────────┬───────────────────────────┘           │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────┐           │
│  │  Waiting Rooms (Condvar synchronization)     │           │
│  │  Key: (FileID, BlockIndex)                   │           │
│  │  Value: Arc<Condvar> for thread wake-up      │           │
│  └──────────────────────────────────────────────┘           │
└─────────────────────┬───────────────────────────────────────┘
                      │ Priority Signal (BlockRequest)
                      │ via crossbeam-channel
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                 Wormhole Transport Layer                     │
│                                                              │
│  Background Thread Loop:                                    │
│  1. Check priority queue (non-blocking recv)                │
│  2. If priority request exists:                             │
│     a. Pause sequential fill                                │
│     b. Fetch requested block immediately                    │
│     c. Write to cache directory                             │
│     d. Resume sequential fill                               │
│  3. Else: Continue sequential background download           │
│                                                              │
│  Future Enhancement:                                        │
│  - Thread pool for parallel block downloads                 │
│  - Adaptive prefetching based on access patterns            │
│  - Integration with Orbit backend protocol                  │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    Block Cache Layer                         │
│  /tmp/orbit_cache/{file_id}_{block_index}.bin               │
│                                                              │
│  Production TODO:                                           │
│  - LRU eviction policy                                      │
│  - Configurable cache size limits                           │
│  - Persistent cache across mounts                           │
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
   - Check cache: /tmp/orbit_cache/file_123_50.bin exists?
   - NO → Send BlockRequest{file_id: "file_123", block_index: 50}
   - Create Condvar for this block
   - Enter blocking wait loop
   ↓
5. Wormhole receives priority signal:
   - Interrupt sequential download (if running)
   - Fetch block 50 from remote storage
   - Simulate 500ms network latency
   - Write 1MB to /tmp/orbit_cache/file_123_50.bin
   ↓
6. Entangler detects file exists:
   - Exit blocking loop
   - Return control to read() handler
   ↓
7. OrbitGhostFS.read() continues:
   - Open /tmp/orbit_cache/file_123_50.bin
   - Seek to relative offset: 52428600 % 1048576 = 824
   - Read 200 bytes
   - Return data to FUSE
   ↓
8. Kernel VFS returns data to application
   ↓
9. Application receives data instantly (500ms total latency)
   vs traditional full download (50MB @ 10MB/s = 5 seconds)
```

## Module Breakdown

### inode.rs - Virtual File Representation

**Purpose:** Bridges Orbit manifest metadata to FUSE inode attributes.

**Key Structures:**
```rust
pub struct GhostFile {
    pub name: String,
    pub size: u64,              // Total file size (from manifest)
    pub orbit_id: String,       // Unique identifier for backend lookup
    pub is_dir: bool,
    pub blocks_present: Vec<bool>, // Bitmap of cached blocks
}
```

**Design Notes:**
- `to_attr()` generates FUSE FileAttr on-demand
- Production should use `RoaringBitmap` for block tracking (memory efficient)
- UID/GID hardcoded to 501/20 (demo values)

### entangler.rs - Quantum Coordination Logic

**Purpose:** Synchronizes FUSE threads with the download pipeline.

**Key Structures:**
```rust
pub struct BlockRequest {
    pub file_id: String,
    pub block_index: u64,
}

pub struct Entangler {
    priority_tx: Sender<BlockRequest>,
    waiting_rooms: Arc<Mutex<HashMap<BlockRequest, Arc<Condvar>>>>,
}
```

**Critical Method:**
```rust
pub fn ensure_block_available(&self, file_id: &str, block_index: u64)
```

**Behavior:**
1. Fast-path: If block exists in cache, return immediately
2. Slow-path:
   - Create Condvar for this block
   - Send priority signal to Wormhole
   - Block current thread via polling loop
   - Wake up when block materializes in cache

**Production Improvements:**
- Replace polling loop with true Condvar::wait()
- Add timeout (return EIO after N seconds)
- Handle duplicate requests (multiple threads reading same block)
- Implement backpressure if queue is full

### fs.rs - FUSE Interface Implementation

**Purpose:** Translates POSIX filesystem operations to manifest lookups and block fetches.

**Implemented Operations:**

1. **lookup(parent, name):**
   - Linear scan of inodes DashMap
   - Production: Use BTreeMap with parent_ino → children hierarchy

2. **getattr(inode):**
   - O(1) lookup in DashMap
   - Returns size, permissions, timestamps from manifest

3. **readdir(inode):**
   - Iterates DashMap entries
   - Returns "." and ".." synthetic entries
   - Users see full directory instantly (no network I/O)

4. **read(inode, offset, size):**
   - **Most critical operation**
   - Calculates affected blocks
   - For each block:
     - Call entangler.ensure_block_available()
     - Read from local cache
   - Slices exact byte range
   - Returns data to kernel

**Performance Characteristics:**
- `readdir`: O(n) in file count, but instant (no network)
- `read`: O(blocks) × network_latency, but parallelizable

### main.rs - System Initialization

**Purpose:** Bootstrap the entire ghost filesystem.

**Execution Flow:**

1. **Setup:**
   - Create mount point directory
   - Create cache directory
   - Initialize logging

2. **Manifest Loading:**
   - Currently: Hardcoded demo file
   - Production: Load from Magnetar catalog
   - Insert into DashMap<u64, GhostFile>

3. **Channel Setup:**
   - Create unbounded priority channel
   - Production: Bounded channel with backpressure

4. **Wormhole Thread:**
   - Spawns background thread
   - Infinite loop: recv() → simulate_download() → write_cache()
   - Production: Thread pool with parallel downloads

5. **FUSE Mount:**
   - Assemble OrbitGhostFS struct
   - Call fuser::mount2() (blocking call)
   - Kernel now routes syscalls to our handlers

## Concurrency Model

### Thread Safety Guarantees

1. **DashMap<u64, GhostFile>:**
   - Lock-free concurrent HashMap
   - Multiple threads can lookup/read simultaneously
   - No write operations after mount (read-only FS)

2. **Entangler.waiting_rooms:**
   - Mutex-protected HashMap
   - Lock held only during insert/remove (minimal contention)
   - Condvar wakeups are lock-free

3. **crossbeam-channel:**
   - MPMC (multi-producer, multi-consumer) channel
   - Lock-free internally
   - Used for priority signaling

### Deadlock Prevention

- No nested locks
- Entangler holds mutex only briefly (insert Condvar, then release)
- FUSE threads block on cache file existence, not on mutexes

### Scalability Analysis

**Current Implementation:**
- Single Wormhole thread (bottleneck for parallel requests)
- Polling loop in Entangler (CPU inefficient)

**Production Scaling:**
- Thread pool: 4-16 download threads
- True Condvar::wait() with notify_one() from Wormhole
- Batch block requests to reduce channel overhead

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

**Current:** Unlimited cache in /tmp (grows forever)

**Production:**
- LRU eviction when cache exceeds size limit
- Priority: Keep blocks near current read position
- Persist cache across mounts (survive reboot)

## Error Handling

### Network Failures

**Current Behavior:** Infinite blocking in `ensure_block_available()`

**Production Fix:**
```rust
// Add timeout
let timeout = Duration::from_secs(30);
let start = Instant::now();

loop {
    if self.is_block_on_disk(file_id, block_index) {
        return Ok(());
    }
    if start.elapsed() > timeout {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "Block fetch timeout"));
    }
    thread::sleep(Duration::from_millis(100));
}
```

**FUSE Handler:**
```rust
match self.entangler.ensure_block_available(&file_info.orbit_id, block_idx) {
    Ok(_) => { /* continue */ },
    Err(_) => {
        reply.error(libc::EIO); // Return I/O error to application
        return;
    }
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

### Phase 1: Standalone Demo (Current)
- Hardcoded manifest
- Simulated backend (generates dummy data)
- Fixed mount point

### Phase 2: Magnetar Integration
- Load manifest from Magnetar catalog
- Map Orbit Block IDs to cache files
- Respect access control policies

### Phase 3: Backend Protocol
- Replace simulated download with real Orbit transfer
- Support resumable block fetches
- Handle backend connection pooling

### Phase 4: Production Hardening
- Implement all error handling
- Add metrics/observability (Prometheus)
- Support configuration files
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

### Unit Tests
- Inode mapping correctness
- Block range calculation
- Cache hit/miss logic

### Integration Tests
- Full mount/unmount cycle
- Read operations with simulated backend
- Concurrent access from multiple threads

### Performance Tests
- Benchmark: Random vs sequential reads
- Measure: Latency distribution for block fetch
- Profile: CPU usage of Entangler

### Chaos Testing
- Network interruptions during block fetch
- Corrupted cache files
- Out-of-space conditions

## Deployment Checklist

Before production deployment:

- [ ] Replace polling loop with Condvar::wait()
- [ ] Add timeout to ensure_block_available()
- [ ] Implement LRU cache eviction
- [ ] Add configuration file support
- [ ] Integrate with Magnetar manifest loading
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

Orbit GhostFS demonstrates that remote data can be accessed with local-like latency through intelligent block-level virtualization. The current implementation provides a solid foundation, with clear paths to production hardening and advanced optimizations.

**Key Takeaway:** By inverting the traditional transfer model (fetch-all → process) to (process → fetch-on-demand), we enable instant interaction with arbitrarily large remote datasets.
