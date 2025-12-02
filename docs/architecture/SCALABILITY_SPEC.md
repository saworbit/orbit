# Orbit V2.1: Universe Scalability Upgrade

**Date:** December 2025
**Status:** Approved
**Component:** core-starmap (Universe Index)

## 1. Problem Statement

### 1.1 The Write Amplification Bottleneck

In the initial V2 architecture, chunk locations were stored as a `Vec<ChunkLocation>` serialized into a single `Vec<u8>` blob under a single key (the chunk hash).

**The Flow:**

1. **Read:** Fetch 50MB blob (1M duplicates).
2. **Deserialize:** Parse into 1M structs.
3. **Modify:** Append 1 new struct.
4. **Serialize:** Encode 1M+1 structs back to 50MB+.
5. **Write:** Write 50MB+ back to disk (Write-Ahead-Log + Data file).

**Complexity:** $O(N^2)$ write IOPS for highly duplicated content (e.g., empty blocks, template headers).

**Impact:** A file with 100k duplicates of a zero-block could take minutes to index.

### 1.2 The Memory Exhaustion Risk

Retrieving locations for a chunk required loading the entire vector into RAM. For a massive dataset with "hot" chunks (millions of references), this causes:

- Large allocations
- Potential OOM (Out of Memory) crashes
- GC pressure (if applicable) or allocator fragmentation

## 2. Solution: The Multimap Architecture

We transition from a **Blob Storage** model to a **Discrete Entry** model using redb's `MultimapTable`.

### 2.1 Data Structure

**Old (V2):**
```rust
TableDefinition<&[u8; 32], Vec<u8>> // Hash -> Serialized List
```

**New (V3):**
```rust
MultimapTableDefinition<&[u8; 32], Vec<u8>> // Hash -> { Serialized Loc A, Serialized Loc B, ... }
```

### 2.2 Complexity Analysis

| Operation        | V2 (Old)                | V3 (New)              | Improvement         |
|------------------|-------------------------|-----------------------|---------------------|
| Insert Duplicate | $O(N)$ (Data Size)      | $O(\log N)$ (B-Tree)  | Exponential         |
| Read All         | $O(1)$ (One Fetch)      | $O(N)$ (Iterate)      | Neutral (Streaming) |
| Memory Usage     | $O(N)$ (All Loaded)     | $O(1)$ (Streamed)     | Safety Critical     |

### 2.3 Lazy Iteration

The `find_chunk` API is refactored to return an `Iterator` rather than a `Vec`. This allows the consumer (Magnetar/Transfer Engine) to process locations one by one, enabling:

- Constant memory usage regardless of duplicate count.
- Early exit (stop after finding the first local copy).

## 3. Implementation Details

### 3.1 Serialization

To maintain efficiency, individual `ChunkLocation` structs are serialized using `bincode` before insertion. This keeps the values small (~30-50 bytes).

### 3.2 ACID Properties

The `MultimapTable` in redb fully supports ACID transactions. We wrap inserts in write transactions and reads in read transactions, maintaining the robustness of the system.

### 3.3 API Design

Two read patterns are provided:

1. **`find_chunk(hash) -> LocationIter`**: Returns an iterator wrapper (currently backed by Vec for simplicity, but interface allows future optimization)
2. **`scan_chunk(hash, callback)`**: Streaming callback pattern for maximum memory efficiency with large result sets

## 4. Migration Strategy

Since V2 has not yet been widely released (0.5.0 target), we will perform an **In-Place Upgrade**.

- The `universe.rs` file will be updated to use the new schema.
- Existing V2 databases (if any alpha users exist) would need to be rebuilt or migrated.
- **Action:** Increment `UNIVERSE_VERSION` to 3 to prevent opening incompatible DBs.

## 5. Verification Plan

A new test suite `tests/scalability_test.rs` will be added to:

1. Insert 100,000 duplicates of a single hash.
2. Measure time per batch.
3. Assert that the time per batch does not grow linearly (verifying $O(1)$ insert complexity).
4. Verify data integrity by reading back all 100,000 entries.

## 6. Performance Expectations

### Before (V2)
- **Insert 1,000th duplicate:** ~10ms
- **Insert 10,000th duplicate:** ~100ms
- **Insert 100,000th duplicate:** ~1000ms (quadratic growth)

### After (V3)
- **Insert 1,000th duplicate:** ~1ms
- **Insert 10,000th duplicate:** ~1ms
- **Insert 100,000th duplicate:** ~1ms (constant time)

## 7. Backward Compatibility

V3 databases are **not** compatible with V2 code. The version number change ensures graceful failure with a clear error message rather than data corruption.

Users upgrading from V2 (if any exist) will need to:
1. Export their V2 Universe to a file list
2. Rebuild the index using V3

A migration tool may be provided if needed.

## 8. Future Optimizations

The current V3 implementation provides the foundation for further improvements:

1. **True Zero-Copy Iteration:** Implement a cursor-based API that holds the read transaction alive
2. **Compression:** Large location lists could be compressed at rest
3. **Sharding:** Multi-file database for parallel access patterns
4. **Bloom Filters:** Add per-chunk bloom filters for existence checks

## 9. Testing Matrix

| Test Case                      | V2 Behavior           | V3 Behavior          |
|--------------------------------|-----------------------|----------------------|
| Insert 100k duplicates         | Minutes (timeout)     | Seconds              |
| Read 1M locations              | OOM crash             | Streaming (O(1) mem) |
| Mixed workload (read/write)    | Lock contention       | Improved concurrency |
| Database restart               | ACID preserved        | ACID preserved       |

## 10. Conclusion

The Universe V3 architecture solves the critical scalability bottlenecks identified in V2:

✅ **Write Amplification:** Eliminated via discrete entries
✅ **Memory Exhaustion:** Solved via streaming iteration
✅ **Performance:** O(N²) → O(log N) for inserts
✅ **Reliability:** Full ACID guarantees maintained

This upgrade enables Orbit to scale to enterprise workloads with billions of chunks and millions of duplicates per chunk.
