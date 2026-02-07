# orbit-core-cdc

Content-Defined Chunking (CDC) engine for Orbit V2.

## Overview

This crate implements a fast, shift-resilient chunking algorithm using the Gear Hash rolling hash. It solves the "shift problem" that plagues fixed-size chunking - when data is inserted or deleted, only the affected chunk changes, not all subsequent chunks.

## The Shift Problem

**Fixed-Size Chunking (Orbit V1):**
```
Original: [Chunk1: 0-64KB] [Chunk2: 64-128KB] [Chunk3: 128-192KB]
Insert 1 byte at offset 0:
Modified: [Chunk1: 0-64KB] [Chunk2: 64-128KB] [Chunk3: 128-192KB]
          └─ Different!    └─ Different!      └─ Different!
Result: 0% chunk preservation, 100% data retransfer
```

**Content-Defined Chunking (Orbit V2):**
```
Original: [Chunk1: 0-45KB] [Chunk2: 45-112KB] [Chunk3: 112-189KB]
Insert 1 byte at offset 0:
Modified: [Chunk1: 0-46KB] [Chunk2: 46-113KB] [Chunk3: 113-190KB]
          └─ Different!    └─ SAME CONTENT!  └─ SAME CONTENT!
Result: 99.1% chunk preservation, only 1 chunk retransferred
```

## Features

- **Gear Hash Rolling Hash**: Fast 64-bit rolling hash using a BLAKE3-derived 256-entry lookup table
- **Variable-Sized Chunks**: Configurable min/avg/max sizes (default: 8KB/64KB/256KB)
- **BLAKE3 Content Hashing**: Cryptographically secure chunk identification
- **Iterator-Based API**: Memory-efficient streaming with `ChunkStream<R: Read>`
- **Robust Boundary Detection**: Threshold-based cut detection works across different data patterns
- **Efficient Buffering**: 2× max_size buffer with smart refilling

## Usage

```rust
use orbit_core_cdc::{ChunkConfig, ChunkStream};
use std::fs::File;

// Configure chunking parameters
let config = ChunkConfig::new(
    8 * 1024,    // 8 KB min
    64 * 1024,   // 64 KB avg
    256 * 1024,  // 256 KB max
)?;

// Create chunk stream from any Read implementor
let file = File::open("data.bin")?;
let stream = ChunkStream::new(file, config);

// Process chunks
for result in stream {
    let chunk = result?;
    println!(
        "Chunk at offset {}: {} bytes, hash: {}",
        chunk.offset,
        chunk.length,
        hex::encode(&chunk.hash)
    );
    
    // chunk.data contains the actual bytes
}
```

## Algorithm

1. **Sliding Window**: Scan through data byte-by-byte
2. **Hash Update**: Update Gear Hash with each byte: `hash = (hash << 1) + GEAR_TABLE[byte]`
3. **Boundary Check**: After min_size, check if `(hash & MASK) < THRESHOLD`
4. **Cut Point**: When condition is met, emit chunk and start new one
5. **Force Cut**: Always cut at max_size to prevent unbounded chunks

## Performance

- **Throughput**: Designed for >2GB/s per core (limited by I/O in practice)
- **Memory**: Uses 2× max_size buffer (default: 512KB)
- **Resilience**: 99.1% chunk preservation after single-byte insertion
- **Determinism**: Same data always produces same chunks

## Testing

```bash
# Run unit tests
cargo test -p orbit-core-cdc

# Run resilience verification
cargo test --test v2_resilience_check -- --nocapture
```

### Test Results

```
✓ test_config_validation ............. PASSED
✓ test_empty_input ................... PASSED
✓ test_small_input ................... PASSED
✓ test_basic_chunking ................ PASSED
✓ test_deterministic_chunking ........ PASSED
✓ test_cdc_resilience_to_insertion ... PASSED (99.1% preservation!)
✓ test_chunk_size_distribution ....... PASSED
```

## Architecture

```
┌─────────────────────┐
│   ChunkStream<R>    │  Iterator-based API
├─────────────────────┤
│   Buffer Manager    │  Efficient 2× buffering
├─────────────────────┤
│   Gear Hash Engine  │  Fast rolling hash
├─────────────────────┤
│   BLAKE3 Hasher     │  Content addressing
└─────────────────────┘
```

## Integration

The CDC engine integrates with Orbit V2 architecture:

- **Universe Map** (`core-starmap`): Stores chunk locations for global deduplication
- **Semantic Layer** (`core-semantic`): Prioritizes file processing order
- **V2 Integration** (`v2_integration.rs`): Coordinates CDC + Universe Map + Semantic

## References

- [FastCDC Paper](https://www.usenix.org/system/files/conference/atc16/atc16-paper-xia.pdf)
- [Gear Hash](https://github.com/restic/chunker) - Similar implementation in Go
- BLAKE3: https://github.com/BLAKE3-team/BLAKE3

## License

Licensed under Apache-2.0
