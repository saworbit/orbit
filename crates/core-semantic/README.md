# orbit-core-semantic

Intent-based replication logic for Orbit V2 - the "brain" that decides WHEN and HOW to sync files.

## Overview

Traditional backup tools treat all files equally - every byte is just data to copy. But in disaster recovery scenarios, **Time to Criticality matters more than Time to Completion**.

This crate implements semantic analysis of files to determine:
- **Priority**: When should this file be synced? (Critical configs first, bulk media last)
- **Strategy**: How should this file be synced? (Atomic for configs, CDC for binaries, Append-Only for logs)

## The Problem

Consider a database server recovery:
- **Bad**: Transfer 1TB of user data, then config files → Database offline for hours
- **Good**: Transfer config files first, then critical data, then bulk → Database online in minutes

## Solution: Semantic Adapters

Files are analyzed by a chain of adapters, each identifying specific file types:

1. **ConfigAdapter** → Critical (Priority=0)
   - `.toml`, `.json`, `.yaml`, `.env`, lockfiles
   - Strategy: `AtomicReplace` (all-or-nothing)

2. **WALAdapter** → High (Priority=10)
   - `pg_wal/*`, `.wal`, `.binlog`
   - Strategy: `AppendOnly` (streaming tail)

3. **MediaAdapter** → Low (Priority=100)
   - `.mp4`, `.jpg`, `.iso`, `.zip`
   - Strategy: `ContentDefined` (CDC chunking)

4. **DefaultAdapter** → Normal (Priority=50)
   - Everything else
   - Strategy: `ContentDefined`

### Composable Prioritizers 🆕

While the `SemanticRegistry` classifies files into priority tiers, the composable prioritizer system provides fine-grained sort control within and across those tiers:

- **`Prioritizer` trait**: `compare(a, b) -> Ordering` with `name()` for introspection — requires `Send + Sync`
- **`ComposablePrioritizer`**: Chains multiple criteria; first non-Equal result wins
- **6 Built-in Strategies**:
  - `SemanticPrioritizer` — Critical files before Low priority
  - `SmallestFirstPrioritizer` — Small files transfer faster, unblock dependencies
  - `LargestFirstPrioritizer` — Maximize throughput on high-bandwidth links
  - `OldestFirstPrioritizer` — FIFO fairness tiebreaker
  - `NewestFirstPrioritizer` — Freshest data first
  - `FewestRetriesPrioritizer` — Deprioritize items that keep failing
- **Default chain**: Semantic priority → Smallest first → Oldest first

```rust
use orbit_core_semantic::prioritizer::*;

let prioritizer = ComposablePrioritizer::default_chain();

// Or build a custom chain
let custom = ComposablePrioritizer::new(vec![
    Box::new(SemanticPrioritizer),
    Box::new(LargestFirstPrioritizer),   // High-throughput link
    Box::new(FewestRetriesPrioritizer),  // Avoid repeatedly failing items
]);

items.sort_by(|a, b| custom.compare(a, b));
```

**Status**: 🔴 Alpha — 17 tests passing

## Usage

```rust
use orbit_core_semantic::{SemanticRegistry, Priority, SyncStrategy};
use std::path::Path;

// Create registry with default adapters
let registry = SemanticRegistry::default();

// Analyze a config file
let intent = registry.determine_intent(Path::new("app.toml"), b"");
assert_eq!(intent.priority, Priority::Critical);
assert_eq!(intent.strategy, SyncStrategy::AtomicReplace);

// Analyze a database WAL
let intent = registry.determine_intent(Path::new("pg_wal/000001"), b"");
assert_eq!(intent.priority, Priority::High);
assert_eq!(intent.strategy, SyncStrategy::AppendOnly);

// Analyze a video file
let intent = registry.determine_intent(Path::new("video.mp4"), b"");
assert_eq!(intent.priority, Priority::Low);
assert_eq!(intent.strategy, SyncStrategy::ContentDefined);
```

## File Classification

### Critical (Priority=0)
Files that **must** be transferred first for system boot/recovery:
- Configuration: `.toml`, `.json`, `.yaml`, `.yml`, `.ini`, `.conf`, `.env`
- Lock files: `Cargo.lock`, `package-lock.json`, `poetry.lock`
- Build files: `Dockerfile`, `Makefile`

### High (Priority=10)
Near-realtime streaming data for database consistency:
- PostgreSQL WAL: `pg_wal/*`, `pg_xlog/*`
- MySQL binlog: `*.binlog`
- Generic WAL: `*.wal`, `*.log`

### Normal (Priority=50)
Standard files with no special handling:
- Source code: `.rs`, `.js`, `.py`, `.go`
- Documents: `.md`, `.txt`, `.pdf`
- Default for unknown extensions

### Low (Priority=100)
Large, immutable files that can be deferred:
- Video: `.mp4`, `.mkv`, `.avi`, `.mov`
- Audio: `.mp3`, `.flac`, `.wav`
- Images: `.jpg`, `.png`, `.gif`
- Archives: `.zip`, `.tar`, `.gz`, `.7z`
- Disk images: `.iso`, `.img`, `.dmg`, `.vdi`

## Sync Strategies

### ContentDefined (CDC)
Variable-sized chunks based on content boundaries (from `core-cdc`)
- **Best for**: Large files with localized changes
- **Example**: Source code, binaries, databases

### AppendOnly
Streaming tail mode - only append new data
- **Best for**: Log files, WAL segments
- **Example**: `pg_wal/000001` grows from 16MB → 32MB → Only transfer new 16MB

### AtomicReplace
Transfer complete file or nothing (no partial states)
- **Best for**: Small critical files where consistency is critical
- **Example**: Config files must be valid or system won't boot

### Adapter(name)
Custom logic for complex file types
- **Future**: Git graph traversal, database-specific logic

## Architecture

```
┌─────────────────────────┐
│   SemanticRegistry      │  Ordered chain of adapters
├─────────────────────────┤
│  1. ConfigAdapter       │  .toml → Critical + Atomic
│  2. WalAdapter          │  .wal → High + Append
│  3. MediaAdapter        │  .mp4 → Low + CDC
│  4. DefaultAdapter      │  * → Normal + CDC
└─────────────────────────┘
         ↓
┌─────────────────────────┐
│  ReplicationIntent      │
├─────────────────────────┤
│  priority: Priority     │  When to sync
│  strategy: SyncStrategy │  How to sync
│  description: String    │  For logging
└─────────────────────────┘
```

## Extending with Custom Adapters

```rust
use orbit_core_semantic::{SemanticAdapter, ReplicationIntent, Priority, SyncStrategy};
use std::path::Path;

struct GitAdapter;

impl SemanticAdapter for GitAdapter {
    fn matches(&self, path: &Path, _head: &[u8]) -> bool {
        path.to_string_lossy().contains(".git/")
    }

    fn analyze(&self, path: &Path, _head: &[u8]) -> ReplicationIntent {
        ReplicationIntent::new(
            Priority::High,
            SyncStrategy::Adapter("git-graph".to_string()),
            "Git object",
        )
    }
}

// Register custom adapter
let registry = SemanticRegistry::new(vec![
    Box::new(GitAdapter),
    Box::new(ConfigAdapter),
    // ... other adapters
]);
```

## Testing

```bash
# Run unit tests
cargo test -p orbit-core-semantic

# Run integration tests
cargo test --test v2_semantic_test -- --nocapture
```

### Test Results

```
✓ test_config_adapter .............. PASSED
✓ test_wal_adapter ................. PASSED
✓ test_media_adapter ............... PASSED
✓ test_default_adapter ............. PASSED
✓ test_registry_default ............ PASSED
✓ test_registry_order_matters ...... PASSED
✓ test_priority_ordering ........... PASSED
✓ test_intent_builders ............. PASSED

✓ verify_semantic_classification ... PASSED
  ✅ PASS: All file types correctly classified
  - production.toml → Critical + Atomic
  - database.wal → High + Append
  - source_code.rs → Normal + CDC
  - video.mp4 → Low + CDC

✓ verify_priority_sorting_logic .... PASSED
```

## Performance

- **Classification**: O(1) for extension matching
- **Magic numbers**: O(1) for first 512 bytes read
- **Memory**: Zero allocations for intent determination
- **Thread-safe**: `SemanticAdapter` trait requires `Send + Sync`

## Integration

The semantic layer integrates with Orbit V2 architecture:

- **CDC Engine** (`core-cdc`): Provides ContentDefined strategy
- **Universe Map** (`core-starmap`): Receives prioritized file queue
- **V2 Integration** (`v2_integration.rs`): Coordinates all V2 components

## Disaster Recovery Example

Before V2 (no prioritization):
```
Transfer order: [video.mp4, app.toml, db.wal, source.rs]
Time to Critical: 2 hours (waited for video first!)
```

After V2 (semantic prioritization):
```
Transfer order: [app.toml, db.wal, source.rs, video.mp4]
Time to Critical: 5 minutes (critical files first!)
RTO improvement: ~95%
```

## License

Licensed under Apache-2.0
