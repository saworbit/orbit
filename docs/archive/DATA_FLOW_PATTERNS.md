# Data Flow Patterns

> Enterprise-grade flow control, error isolation, and observability for the Orbit transfer pipeline.

**Status**: 🔴 Alpha (all modules newly introduced)
**Test Coverage**: 50+ dedicated tests across 3 modules

---

## Overview

Data transfer is a **flow control problem**, not just a copy problem. Orbit implements data flow patterns designed for content-addressed bulk transfers:

- **Storage Efficiency** — Pack chunks into container files to reduce inode pressure
- **Observability** — Typed provenance events replace unstructured logs
- **Prioritization** — Composable prioritizers provide fine-grained sort control

### Design Principles

All modules follow a founding principle: **pure logic with zero knowledge of storage, network, or application-specific concerns**. They accept and return plain Rust types -- the caller decides where to persist, how to serialize, and when to act on advisories.

---

## Module Summary

| Module | Crate | Purpose | Key Types |
|--------|-------|---------|-----------|
| Container Packing | `core-starmap` | Chunk packing into `.orbitpak` files | `ContainerWriter`, `ContainerPool` |
| Typed Provenance | `core-audit` | Structured event taxonomy (20 types) | `ProvenanceEvent`, `ProvenanceType` |
| Composable Prioritizers | `core-semantic` | Chainable sort criteria | `ComposablePrioritizer`, `Prioritizer` trait |

---

## Architecture Diagram

```
Source Files
    │
    ▼
┌─────────────────────────────────────────────────────────┐
│  Semantic Registry + Composable Prioritizers            │
│  (core-semantic)                                        │
│  Classify files → Chain sort criteria → Ordered queue   │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Transfer Execution                                      │
│  ┌───────────────┐    ┌──────────────────┐              │
│  │ On success:   │    │ On failure:      │              │
│  │ Provenance    │    │ Retry with       │              │
│  │ event logged  │    │ backoff          │              │
│  └───────────────┘    └──────────────────┘              │
└────────────────────────┬────────────────────────────────┘
                         │
              ┌──────────┴──────────┐
              ▼                     ▼
        Provenance            Container
        Events                Packing
        (audit)               (.orbitpak)
```

---

## Container Packing

**Crate**: `orbit_core_starmap::container`

Instead of one file per chunk (inode pressure at scale), chunks are packed into `.orbitpak` container files. The Universe index stores `(container_id, offset, length)` tuples for random access.

### File Format

```
┌─────────────────────────────────────┐
│ Magic: "ORBITPAK\0" (9 bytes)       │
│ Version: u32 (4 bytes)              │
│ Reserved (3 bytes)                   │
├─────────────────────────────────────┤
│ Chunk A data (raw bytes)             │
│ Chunk B data (raw bytes)             │
│ ...                                  │
└─────────────────────────────────────┘
```

### Pool Rotation

`ContainerPool` manages multiple container files. When a container reaches the configured maximum size (default 4 GiB), a new container is created automatically.

```rust
use orbit_core_starmap::container::{ContainerPool, PackedChunkRef};

let mut pool = ContainerPool::new("/data/containers", 4 * 1024 * 1024 * 1024)?;

let chunk_ref: PackedChunkRef = pool.write_chunk(&chunk_data)?;
// chunk_ref = { container_id: "abc123", offset: 16, length: 65536 }
```

---

## Typed Provenance Events

**Crate**: `orbit_core_audit::provenance`

A 20-type event taxonomy replacing unstructured log messages with queryable structured events.

### Event Types

| Category | Events |
|----------|--------|
| **Chunk** | ChunkCreated, ChunkDeduplicated, ChunkTransferred, ChunkVerified, ChunkHealed, ChunkPenalized, ChunkDeadLettered, ChunkPacked |
| **File** | FileRenamed, FileSkipped, FileStarted, FileCompleted |
| **Job** | JobCreated, JobResumed, JobCompleted, JobFailed |
| **Grid** | StarRegistered, StarScheduled, StarDraining, StarDeregistered |

### Usage

```rust
use orbit_core_audit::provenance::{ProvenanceEvent, ProvenanceType};

let event = ProvenanceEvent::new(ProvenanceType::ChunkTransferred, 42)
    .with_chunk_hash("abc123def456")
    .with_source("/data/file.bin")
    .with_destination("star-2:/backup/file.bin")
    .with_bytes(65536)
    .with_duration_ms(150);

// Write to JSON Lines log
logger.log(&event)?;
```

---

## Composable Prioritizers

**Crate**: `orbit_core_semantic::prioritizer`

While `SemanticRegistry` classifies files into priority tiers, composable prioritizers provide fine-grained sort control within those tiers by chaining multiple criteria.

### Built-in Prioritizers

| Prioritizer | Sort Key |
|-------------|----------|
| `SemanticPrioritizer` | Critical before Low |
| `SmallestFirstPrioritizer` | Smaller bytes first |
| `LargestFirstPrioritizer` | Larger bytes first |
| `OldestFirstPrioritizer` | Earlier timestamp first |
| `NewestFirstPrioritizer` | Later timestamp first |
| `FewestRetriesPrioritizer` | Fewer retries first |

### Chain Composition

First non-`Equal` result wins:

```rust
use orbit_core_semantic::prioritizer::*;

// Default: semantic → smallest → oldest
let chain = ComposablePrioritizer::default_chain();

// Custom: semantic → largest (throughput-optimized) → fewest retries
let custom = ComposablePrioritizer::new(vec![
    Box::new(SemanticPrioritizer),
    Box::new(LargestFirstPrioritizer),
    Box::new(FewestRetriesPrioritizer),
]);

items.sort_by(|a, b| custom.compare(a, b));
```

---

## Test Coverage

| Module | Tests | Key Areas |
|--------|-------|-----------|
| Container Packing | 16 | Empty chunks, invalid magic, pool rotation, multiple reads |
| Typed Provenance | 17 | Logger write/read, parser edge cases, event classification |
| Composable Prioritizers | 17 | Chain composition, sort stability, tiebreaker, empty chain |
| **Total** | **50** | |
