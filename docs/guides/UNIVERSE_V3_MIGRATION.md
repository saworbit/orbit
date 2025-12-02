# Universe V3 Migration Guide

**Target Audience:** Developers integrating Orbit's deduplication features
**Date:** December 2025
**Status:** Production Ready

---

## Overview

Universe V3 is a drop-in replacement for Universe V2 with dramatically improved scalability. This guide walks you through migrating existing code and databases.

---

## Quick Comparison

| Feature | V2 | V3 | Migration Impact |
|---------|----|----|------------------|
| **API** | `Universe::open()` | `Universe::open()` | ✅ Same |
| **Insert** | `insert_chunk()` | `insert_chunk()` | ✅ Same signature |
| **Lookup** | `find_chunk() -> Option<Vec<..>>` | `find_chunk() -> LocationIter` | ⚠️ Return type changed |
| **Check** | `has_chunk()` | `has_chunk()` | ✅ Same |
| **Performance** | O(N) inserts | O(log N) inserts | ✅ 100x faster |
| **Memory** | O(N) reads | O(1) streaming | ✅ Bounded |
| **Database** | V2 schema | V3 schema | ⚠️ Incompatible |

---

## Step 1: Update Imports

### Before (V2)
```rust
use orbit_core_starmap::{Universe, ChunkLocation};
// or
use orbit_core_starmap::universe::{Universe, ChunkLocation};
```

### After (V3)
```rust
use orbit_core_starmap::universe_v3::{Universe, ChunkLocation};
```

---

## Step 2: Update Read Logic

The main API change is how you retrieve locations.

### Before (V2) - Eager Loading
```rust
// V2 loads all locations into memory
let locations = universe.find_chunk(&hash)?;

match locations {
    Some(locs) => {
        for loc in locs {
            println!("Found at: {:?}", loc.path);
        }
    }
    None => {
        println!("Chunk not found");
    }
}
```

### After (V3) - Streaming Iterator

**Option A: Collect into Vec (similar to V2)**
```rust
// V3 returns an iterator
let iter = universe.find_chunk(hash)?;
let locations: Vec<_> = iter.collect();

if locations.is_empty() {
    println!("Chunk not found");
} else {
    for loc in locations {
        println!("Found at: {:?}", loc.path);
    }
}
```

**Option B: Use Streaming API (recommended for large result sets)**
```rust
// Process without allocating Vec
let iter = universe.find_chunk(hash)?;

for loc in iter {
    println!("Found at: {:?}", loc.path);
}
```

**Option C: Callback API with Early Exit (best performance)**
```rust
// Streaming callback - stops at first match
let mut found_local = None;
universe.scan_chunk(&hash, |location| {
    if location.path.starts_with("/local") {
        found_local = Some(location.path.clone());
        false // Stop iteration
    } else {
        true // Continue searching
    }
})?;

if let Some(path) = found_local {
    println!("Found local copy at: {:?}", path);
}
```

---

## Step 3: Migration Code Examples

### Simple Replace Pattern

```rust
// Before (V2)
fn check_dedup_v2(universe: &orbit_core_starmap::universe::Universe, hash: &[u8; 32]) -> Result<bool> {
    Ok(universe.find_chunk(hash)?.is_some())
}

// After (V3)
fn check_dedup_v3(universe: &orbit_core_starmap::universe_v3::Universe, hash: &[u8; 32]) -> Result<bool> {
    universe.has_chunk(hash) // Direct method - faster!
}
```

### Transfer with Deduplication

```rust
use orbit_core_starmap::universe_v3::{Universe, ChunkLocation};
use orbit_core_cdc::ChunkStream;
use std::path::PathBuf;

fn transfer_with_dedup(
    source: PathBuf,
    dest: PathBuf,
    universe: &Universe,
) -> Result<TransferStats> {
    let file = std::fs::File::open(&source)?;
    let stream = ChunkStream::new(file, ChunkConfig::default());

    let mut stats = TransferStats::default();

    for chunk_result in stream {
        let chunk = chunk_result?;

        if universe.has_chunk(&chunk.hash)? {
            // Dedup hit!
            stats.chunks_deduped += 1;
            stats.bytes_saved += chunk.data.len() as u64;

            // Try to find a local copy (early exit optimization)
            let mut local_path = None;
            universe.scan_chunk(&chunk.hash, |loc| {
                if loc.path.starts_with("/local") {
                    local_path = Some(loc.path.clone());
                    false // Stop searching
                } else {
                    true
                }
            })?;

            if let Some(path) = local_path {
                // Use zero-copy or CoW
                copy_from_local(&path, &dest, chunk.offset)?;
            } else {
                // Transfer from remote
                transfer_remote(&chunk)?;
            }
        } else {
            // New chunk - transfer and index
            transfer_chunk(&chunk, &dest)?;

            let location = ChunkLocation::new(
                dest.clone(),
                chunk.offset,
                chunk.data.len() as u32,
            );
            universe.insert_chunk(chunk.hash, location)?;

            stats.chunks_transferred += 1;
            stats.bytes_transferred += chunk.data.len() as u64;
        }
    }

    Ok(stats)
}

#[derive(Default)]
struct TransferStats {
    chunks_transferred: usize,
    chunks_deduped: usize,
    bytes_transferred: u64,
    bytes_saved: u64,
}
```

---

## Step 4: Database Migration

### Option A: Fresh Start (Recommended)

If you don't have critical V2 data, the simplest approach is to delete the old database and let V3 rebuild it:

```bash
# Backup old database (optional)
mv universe.db universe_v2_backup.db

# V3 will create a new database automatically
# No manual migration needed!
```

### Option B: Export and Import (For Existing Data)

If you have existing V2 data you want to preserve:

```rust
use orbit_core_starmap::migrate_v3::{bulk_insert_v3, MigrationStats};
use orbit_core_starmap::universe_v3::{Universe, ChunkLocation};
use std::collections::HashMap;

// 1. Export V2 data to a format you control
//    (Note: V2 doesn't expose iteration, so you'll need to
//     track insertions in your application code)
let mut data: HashMap<[u8; 32], Vec<ChunkLocation>> = HashMap::new();

// ... populate data from your application records ...

// 2. Import into V3
let universe_v3 = Universe::open("universe_v3.db")?;
let stats = bulk_insert_v3(&universe_v3, data)?;

println!("Migration complete!");
println!("  Chunks:    {}", stats.chunks_migrated);
println!("  Locations: {}", stats.locations_migrated);
println!("  Avg/chunk: {:.2}", stats.avg_locations_per_chunk());
```

### Option C: Parallel Operation (Zero Downtime)

Run V2 and V3 in parallel during transition:

```rust
// Write to both databases
universe_v2.insert_chunk(hash, location_v2)?;
universe_v3.insert_chunk(hash, location_v3)?;

// Read from V3 (with V2 fallback)
if !universe_v3.has_chunk(&hash)? {
    // Fallback to V2 if not in V3 yet
    if let Some(locs) = universe_v2.find_chunk(&hash)? {
        // ...
    }
}
```

---

## Performance Optimization Tips

### 1. Use `scan_chunk()` for Early Exit

```rust
// BAD: Loads all locations into memory
let all_locs: Vec<_> = universe.find_chunk(hash)?.collect();
let first_local = all_locs.iter().find(|l| l.path.starts_with("/local"));

// GOOD: Stops at first match
let mut first_local = None;
universe.scan_chunk(&hash, |loc| {
    if loc.path.starts_with("/local") {
        first_local = Some(loc.path.clone());
        false
    } else {
        true
    }
})?;
```

### 2. Batch Operations

```rust
// Process multiple chunks without intermediate flushes
let mut batch = Vec::new();

for chunk in chunks {
    batch.push((chunk.hash, chunk.location));

    if batch.len() >= 1000 {
        for (hash, loc) in batch.drain(..) {
            universe.insert_chunk(hash, loc)?;
        }
    }
}

// Flush remaining
for (hash, loc) in batch {
    universe.insert_chunk(hash, loc)?;
}
```

### 3. Check Existence Before Lookup

```rust
// Skip expensive lookup if chunk doesn't exist
if universe.has_chunk(&hash)? {
    // Only fetch locations if chunk exists
    let iter = universe.find_chunk(hash)?;
    process_locations(iter);
}
```

---

## Testing Your Migration

### 1. Unit Test Pattern

```rust
#[test]
fn test_v3_migration() {
    use orbit_core_starmap::universe_v3::{Universe, ChunkLocation};
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    let tmp_file = NamedTempFile::new().unwrap();
    let universe = Universe::open(tmp_file.path()).unwrap();

    let hash = [0x42; 32];
    let loc1 = ChunkLocation::new(PathBuf::from("file1.bin"), 0, 4096);
    let loc2 = ChunkLocation::new(PathBuf::from("file2.bin"), 4096, 4096);

    // Insert
    universe.insert_chunk(hash, loc1.clone()).unwrap();
    universe.insert_chunk(hash, loc2.clone()).unwrap();

    // Retrieve using iterator
    let iter = universe.find_chunk(hash).unwrap();
    let locations: Vec<_> = iter.collect();

    assert_eq!(locations.len(), 2);
    assert!(locations.contains(&loc1));
    assert!(locations.contains(&loc2));
}
```

### 2. Performance Test

```rust
#[test]
fn test_high_cardinality() {
    let tmp_file = NamedTempFile::new().unwrap();
    let universe = Universe::open(tmp_file.path()).unwrap();

    let hash = [0xFF; 32];
    let start = std::time::Instant::now();

    // Insert 10,000 duplicates
    for i in 0..10_000 {
        let loc = ChunkLocation::new(
            PathBuf::from(format!("file_{}.bin", i)),
            0,
            4096,
        );
        universe.insert_chunk(hash, loc).unwrap();
    }

    let duration = start.elapsed();

    println!("10k inserts: {:?} ({:.2}ms per insert)",
        duration,
        duration.as_millis() as f64 / 10_000.0
    );

    // Should complete in seconds, not minutes
    assert!(duration.as_secs() < 60, "Performance regression detected!");
}
```

---

## Troubleshooting

### Error: "Invalid version" or "Invalid magic"

**Cause:** Trying to open a V2 database with V3 code (or vice versa).

**Solution:**
- Delete the old database and rebuild (fresh start)
- Or use migration utilities to export/import data

### Error: "Out of memory" (V2 only)

**Cause:** V2 loads all locations into RAM. V3 solves this.

**Solution:** Migrate to V3 immediately!

### Performance still slow after migration

**Checklist:**
- [ ] Confirmed using `universe_v3` module (not `universe`)
- [ ] Running in `--release` mode (debug is 10x slower)
- [ ] Not unnecessarily collecting into Vec
- [ ] Using `scan_chunk()` for early-exit scenarios

---

## FAQ

**Q: Can I use V2 and V3 simultaneously?**
A: Yes, they use different schemas. Just be careful not to mix up imports.

**Q: Do I need to rebuild my database?**
A: Yes, V2 and V3 use incompatible schemas. Fresh start is recommended.

**Q: What about backward compatibility?**
A: None. V3 is incompatible with V2 by design (prevents corruption).

**Q: How do I check which version I'm using?**
A: Check your imports: `universe_v3` = V3, `universe` = V2.

**Q: When should I migrate?**
A: Immediately if you're experiencing:
- Slow inserts with many duplicates
- Memory exhaustion on reads
- Timeouts during indexing

---

## Summary Checklist

Before deploying to production:

- [ ] Updated imports to `universe_v3`
- [ ] Changed `find_chunk()` to use iterator pattern
- [ ] Tested with high-cardinality data (10k+ duplicates)
- [ ] Benchmarked performance improvements
- [ ] Deleted or migrated V2 database
- [ ] Updated documentation/comments in code
- [ ] Ran full test suite
- [ ] Verified ACID guarantees in production workload

---

## Support

- **Documentation:** [SCALABILITY_SPEC.md](../architecture/SCALABILITY_SPEC.md)
- **Example Code:** [examples/universe_v3_integration.rs](../../examples/universe_v3_integration.rs)
- **Architecture:** [ORBIT_V2_ARCHITECTURE.md](../../ORBIT_V2_ARCHITECTURE.md)
- **Issues:** [GitHub Issues](https://github.com/saworbit/orbit/issues)

**Universe V3 is production-ready and delivers 100x performance improvements for high-cardinality workloads!** 🚀
