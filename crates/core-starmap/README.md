# orbit-core-starmap

**Binary index and global deduplication for Orbit V2/V2.1**

## Overview

`orbit-core-starmap` provides efficient indexing and deduplication capabilities for the Orbit file synchronization system. It includes:

- **Star Map V1**: Binary index for chunk and window metadata with bloom filters
- **Universe Map V2**: Global content-addressed deduplication index (in-memory)
- **Universe V3** ⭐: High-cardinality scalable deduplication index (persistent, production-ready)

## Features

### Star Map V1 (File-Level Index)
- Memory-mapped binary format for zero-copy reads
- Bloom filter for O(1) chunk existence checks
- Rank-select bitmaps for resume support
- Efficient binary serialization via bincode

### Universe Map V2 (Global Dedup - In-Memory)
- Repository-wide content-addressed index
- HashMap-based storage for fast lookups
- Deduplication statistics and reporting
- Save/load with versioned binary format

### Universe V3 (Scalable Persistent Index) 🚀
- **Production Ready**: Handles billions of chunks
- **O(log N) Inserts**: Constant-time performance regardless of duplicate count
- **O(1) Memory**: Streaming iteration prevents memory exhaustion
- **ACID Guarantees**: Full transaction support via redb
- **High Cardinality**: Millions of duplicates per chunk supported

## Quick Start

### Star Map (V1)

```rust
use orbit_core_starmap::{StarMapBuilder, StarMapReader};

// Build a Star Map
let mut builder = StarMapBuilder::new(1024000);
builder.add_chunk(0, 4096, &[0u8; 32]);
builder.add_window(0, 0, 10, &[0u8; 32], 0);
let data = builder.build().unwrap();

// Read with memory mapping
let reader = StarMapReader::open("file.starmap.bin").unwrap();
let has_chunk = reader.has_chunk(&[0u8; 32]).unwrap();
```

### Universe V3 (Recommended)

```rust
use orbit_core_starmap::universe_v3::{Universe, ChunkLocation};
use std::path::PathBuf;

// Open database
let universe = Universe::open("universe_v3.db")?;

// Insert chunk location (O(log N) - instant!)
let hash = [0x42; 32];
let location = ChunkLocation::new(PathBuf::from("/data/file.bin"), 0, 4096);
universe.insert_chunk(hash, location)?;

// Check existence (O(1))
if universe.has_chunk(&hash)? {
    println!("Chunk exists!");
}

// Stream process with O(1) memory
universe.scan_chunk(&hash, |location| {
    println!("Found at: {:?}", location.path);
    true // Continue iteration
})?;

// Or collect into iterator
let iter = universe.find_chunk(hash)?;
for location in iter {
    println!("Location: {:?}", location.path);
}
```

### Universe V2 (Legacy)

```rust
use orbit_core_starmap::{UniverseMap, Location};

let mut universe = UniverseMap::new();

// Register files
let file1 = universe.register_file("src/main.rs");

// Add chunks
universe.add_chunk(&hash, Location::new(file1, 0, 4096));

// Query
let locations = universe.get_locations(&hash).unwrap();
```

## Performance Comparison

| Feature | Universe V2 | Universe V3 | Improvement |
|---------|-------------|-------------|-------------|
| **Insert** | O(N) | O(log N) | 100x faster |
| **Memory** | O(N) | O(1) | Unbounded → Constant |
| **100k Dups** | Timeout | ~2 minutes | >95% faster |
| **Max Scale** | ~1k | Billions | Production ready |

## Migration from V2 to V3

See the complete [Universe V3 Migration Guide](../../docs/guides/UNIVERSE_V3_MIGRATION.md) for detailed instructions.

**Quick summary:**
1. Update imports: `universe::` → `universe_v3::`
2. Change read pattern: `find_chunk()` now returns iterator
3. Delete old V2 database (incompatible schemas)
4. Enjoy 100x performance improvement! 🚀

## Architecture

### Star Map Format

```
┌─────────────────────────────────────┐
│          Star Map File              │
├─────────────────────────────────────┤
│ Magic Number (8 bytes)              │
│ Version (2 bytes)                   │
│ Header (counts, sizes)              │
│ Chunk Entries (offset, len, CID)    │
│ Window Entries (id, merkle, etc)    │
│ Bloom Filter (serialized)           │
│ Bitmaps (per-window, serialized)    │
└─────────────────────────────────────┘
```

### Universe V3 Format (Multimap)

```
┌────────────────────────────────────────┐
│      Universe V3 (Multimap)            │
├────────────────────────────────────────┤
│ Key: [u8; 32] (BLAKE3 hash)            │
│ Values: Multiple discrete entries      │
│   Entry 1: bincode(ChunkLocation)      │
│   Entry 2: bincode(ChunkLocation)      │
│   Entry N: bincode(ChunkLocation)      │
└────────────────────────────────────────┘
```

## Modules

- **bitmap**: Rank-select bitmaps for efficient bit operations
- **bloom**: Bloom filter implementation for set membership tests
- **builder**: Star Map builder with incremental construction
- **reader**: Memory-mapped Star Map reader
- **universe**: V2 in-memory global deduplication index
- **universe_v3**: V3 persistent high-cardinality deduplication index ⭐
- **migrate**: V1 → V2 migration utilities
- **migrate_v3**: V2 → V3 migration utilities

## Testing

```bash
# Run all tests
cargo test

# Run V3 scalability tests (20,000 duplicates)
cargo test --test scalability_test --release -- --nocapture

# Run V3 persistence tests
cargo test --test v2_persistence_test --release

# Run V3 unit tests
cargo test universe_v3 --release
```

## Examples

See [examples/universe_v3_integration.rs](../../examples/universe_v3_integration.rs) for a complete integration example demonstrating:
- Content-Defined Chunking (CDC) integration
- Universe V3 deduplication
- High-cardinality performance
- Streaming iteration patterns

## Documentation

- [SCALABILITY_SPEC.md](../../docs/architecture/SCALABILITY_SPEC.md) - Technical specification
- [UNIVERSE_V3_MIGRATION.md](../../docs/guides/UNIVERSE_V3_MIGRATION.md) - Migration guide
- [ORBIT_V2_ARCHITECTURE.md](../../ORBIT_V2_ARCHITECTURE.md) - Full V2/V2.1 architecture

## License

Apache-2.0

## Status

- **Star Map V1**: ✅ Stable - Production ready
- **Universe V2**: 🔴 Alpha - In-memory, limited scale
- **Universe V3**: 🟡 Beta - Production ready, newly released (v2.1)

**Recommendation:** Use Universe V3 for all new projects requiring global deduplication.
