# Data Flow Patterns

> Enterprise-grade flow control, error isolation, and observability for the Orbit transfer pipeline.

**Status**: 🔴 Alpha (all modules newly introduced)
**Test Coverage**: 180+ dedicated tests across 10 modules

---

## Overview

Data transfer is a **flow control problem**, not just a copy problem. Orbit implements 10 data flow patterns designed for content-addressed bulk transfers:

- **Flow Control** — Prevent overwhelming destinations with backpressure signals
- **Error Isolation** — Penalize failed items temporarily; quarantine permanent failures in a dead-letter queue so the rest of the job succeeds
- **Lifecycle Management** — Formalize Star agent states so work is never assigned to a draining node
- **Observability** — Typed provenance events and a centralized bulletin board replace unstructured logs

### Design Principles

All modules follow the `core-resilience` crate's founding principle: **pure logic with zero knowledge of storage, network, or application-specific concerns**. They accept and return plain Rust types — the caller decides where to persist, how to serialize, and when to act on advisories.

---

## Module Summary

| Module | Crate | Purpose | Key Types |
|--------|-------|---------|-----------|
| Penalization | `core-resilience` | Exponential backoff deprioritization | `PenaltyBox`, `PenaltyConfig` |
| Dead-Letter Queue | `core-resilience` | Bounded quarantine for exhausted items | `DeadLetterQueue`, `FailureReason` |
| Backpressure | `core-resilience` | Dual-threshold flow control | `BackpressureGuard`, `BackpressureRegistry` |
| Ref-Counted GC | `core-resilience` | WAL-gated chunk garbage collection | `RefCountMap`, `GarbageCollector` |
| Health Monitor | `core-resilience` | Continuous mid-transfer health checks | `HealthMonitor`, `Advisory` |
| Container Packing | `core-starmap` | Chunk packing into `.orbitpak` files | `ContainerWriter`, `ContainerPool` |
| Typed Provenance | `core-audit` | Structured event taxonomy (20 types) | `ProvenanceEvent`, `ProvenanceType` |
| Bulletin Board | `orbit-connect` | Centralized alert aggregation | `BulletinBoard`, `SharedBulletinBoard` |
| Composable Prioritizers | `core-semantic` | Chainable sort criteria | `ComposablePrioritizer`, `Prioritizer` trait |
| Star Lifecycle | `orbit-star` | Agent state machine | `StarLifecycle`, `LifecycleState` |

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
│  Star Lifecycle Hooks (orbit-star)                       │
│  Only dispatch to Stars in "Scheduled" state            │
│  Registered → Scheduled → Draining → Shutdown           │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Backpressure Guard (core-resilience)                    │
│  Check: object_count < max AND byte_size < max          │
│  If exceeded → signal producer to pause                  │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Transfer Execution                                      │
│  ┌───────────────┐    ┌──────────────────┐              │
│  │ On success:   │    │ On failure:      │              │
│  │ Provenance    │    │ Penalty Box      │              │
│  │ event logged  │    │ (exp. backoff)   │              │
│  └───────────────┘    └────────┬─────────┘              │
│                                │ exhausted?              │
│                       ┌────────▼─────────┐              │
│                       │ Dead-Letter Queue│              │
│                       │ (quarantine)     │              │
│                       └──────────────────┘              │
└────────────────────────┬────────────────────────────────┘
                         │
              ┌──────────┼──────────┐
              ▼          ▼          ▼
        Health      Bulletin    Container
        Monitor     Board       Packing
        (advisories)(alerts)    (.orbitpak)
```

---

## Penalization

**Crate**: `orbit_core_resilience::penalization`

When a chunk transfer fails transiently (timeout, connection refused), the item is **penalized** rather than immediately retried. This prevents a single bad destination from blocking the entire pipeline.

### Configuration

| Field | Default | Description |
|-------|---------|-------------|
| `initial_delay` | 1s | First penalty duration |
| `max_delay` | 60s | Cap on backoff growth |
| `backoff_factor` | 2.0 | Multiplier per consecutive failure |
| `max_penalties` | 5 | Failures before dead-lettering |

### Usage

```rust
use orbit_core_resilience::penalization::{PenaltyBox, PenaltyConfig};

let config = PenaltyConfig {
    max_penalties: 5,
    ..Default::default()
};
let mut box_ = PenaltyBox::new(config);

// On transient failure
let exhausted = box_.penalize("chunk-42", "connection refused");

// Scheduler checks before dispatching
if box_.is_eligible("chunk-42") {
    // Safe to retry
}

if exhausted {
    // Route to dead-letter queue
}
```

---

## Dead-Letter Queue

**Crate**: `orbit_core_resilience::dead_letter`

Items that exhaust their penalty count are routed here instead of failing the entire job. This enables **partial success** — most items transfer, and dead-letter items can be retried manually or by the Sentinel healer.

### Failure Reasons

| Variant | Description |
|---------|-------------|
| `RetriesExhausted` | Max penalty count exceeded |
| `PermanentError` | Non-transient, should not retry |
| `ChecksumMismatch` | Post-transfer verification failed |
| `SourceMissing` | Source file disappeared during transfer |
| `DestinationError` | Write failure (permissions, disk full) |
| `DataCorruption` | Compression/decompression failure |

### Usage

```rust
use orbit_core_resilience::dead_letter::{DeadLetterQueue, DeadLetterEntry, FailureReason};

let mut dlq = DeadLetterQueue::new(1000); // bounded capacity

dlq.push(DeadLetterEntry {
    item_key: "chunk-42".to_string(),
    job_id: 1,
    failure_reason: FailureReason::RetriesExhausted { attempts: 5 },
    last_error: "connection refused".to_string(),
    first_failed_at: std::time::SystemTime::now(),
    last_failed_at: std::time::SystemTime::now(),
    source_path: Some("/data/file.bin".to_string()),
    dest_path: Some("/backup/file.bin".to_string()),
});

// Flush to Magnetar for persistence
let entries = dlq.drain();

// Query by job
let job_failures = dlq.entries_for_job(1);
```

---

## Backpressure

**Crate**: `orbit_core_resilience::backpressure`

Dual-threshold flow control prevents overwhelming a destination Star. Two independent limits — object count and byte size — must both be satisfied before new work is dispatched.

### Design

- **Lock-free**: Uses `AtomicU64` with `Relaxed` ordering for minimal overhead
- **Per-destination**: Each Star gets its own `BackpressureGuard`
- **Registry**: `BackpressureRegistry` manages guards for the entire Grid

```rust
use orbit_core_resilience::backpressure::{BackpressureGuard, BackpressureConfig};

let guard = BackpressureGuard::new("star-1", BackpressureConfig {
    max_object_count: 10_000,
    max_byte_size: 1_073_741_824, // 1 GiB
});

// Producer side
if guard.can_accept() {
    guard.record_enqueue(1, chunk_bytes as u64);
    // dispatch work...
}

// Consumer side (after transfer completes)
guard.record_dequeue(1, chunk_bytes as u64);
```

---

## Reference-Counted Garbage Collection

**Crate**: `orbit_core_resilience::ref_count`

Content-addressed chunks may be referenced by multiple files or jobs. The GC uses a three-phase lifecycle to ensure chunks are only deleted **after** the Magnetar WAL has committed the removal:

```
1. mark_reclaimable(hash)    — Ref count hit zero
2. confirm_wal_synced(hash)  — WAL durably committed
3. collect()                 — Return hashes safe to delete
```

This prevents the crash scenario where a chunk is deleted but the index still references it.

```rust
use orbit_core_resilience::ref_count::{RefCountMap, GarbageCollector};

let mut refs = RefCountMap::new();
refs.increment(&hash);
refs.increment(&hash); // ref_count = 2

refs.decrement(&hash); // ref_count = 1
refs.decrement(&hash); // ref_count = 0 → reclaimable

let mut gc = GarbageCollector::new();
gc.mark_reclaimable(hash);

// After Magnetar WAL sync:
gc.confirm_wal_synced(&hash);

let to_delete = gc.collect(); // Returns [hash]
```

---

## Health Monitor

**Crate**: `orbit_core_resilience::health_monitor`

Continuous mid-transfer health monitoring that produces typed advisories rather than opaque error codes. Includes **linear regression** over disk availability history for exhaustion prediction.

### Advisory Types

| Advisory | Trigger |
|----------|---------|
| `DiskCritical` | Available disk < critical threshold |
| `DiskWarning` | Available disk < warning threshold |
| `DiskExhaustionPredicted` | Linear regression predicts disk full within N seconds |
| `ThroughputLow` | Transfer speed below minimum threshold |
| `ErrorRateHigh` | Error rate exceeds configured percentage |
| `Healthy` | All checks pass |

```rust
use orbit_core_resilience::health_monitor::{HealthMonitor, HealthConfig, HealthSample};

let monitor = HealthMonitor::new(HealthConfig::default());

let sample = HealthSample {
    disk_available_bytes: 500_000_000,
    disk_total_bytes: 1_000_000_000,
    throughput_bytes_per_sec: 50_000_000,
    error_count: 2,
    total_operations: 1000,
};

let advisory = monitor.check(&sample);
// Advisory::Healthy or Advisory::DiskWarning, etc.
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

## Bulletin Board

**Crate**: `orbit_connect::bulletin`

Centralized aggregation of warnings and errors from all Stars into a single queryable feed. Designed to power `GET /api/bulletins` on the Control Plane REST API.

### Design

- **Ring buffer** with bounded capacity (oldest evicted first)
- **Severity**: `Info < Warning < Error` (with `Ord` for threshold filtering)
- **Thread-safe**: `SharedBulletinBoard` uses `Arc<RwLock<BulletinBoard>>`
- **Filterable**: By severity, source Star, category, job ID

```rust
use orbit_connect::bulletin::{SharedBulletinBoard, Severity};

let board = SharedBulletinBoard::new(500);

board.post(Severity::Warning, "star-1", "disk", Some(42), "Disk 85% full");
board.post(Severity::Error, "star-2", "network", Some(42), "Connection refused");

let recent = board.recent(10);
let errors = board.by_severity(Severity::Error);
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

## Star Lifecycle Hooks

**Crate**: `orbit_star::lifecycle`

Formalized state machine for Star agents. As Stars join and leave the Grid, lifecycle hooks ensure clean state transitions and prevent orphan jobs.

### State Machine

```
         register()
              │
     ┌────────▼────────┐
     │   Registered     │  Known but not yet receiving work
     └────────┬─────────┘
              │ schedule()
     ┌────────▼────────┐
     │   Scheduled      │  Actively receiving and processing work
     └────────┬─────────┘
              │ drain()
     ┌────────▼────────┐
     │   Draining       │  Finishing current work, rejecting new
     └────────┬─────────┘
              │ shutdown()
     ┌────────▼────────┐
     │   Shutdown        │  Clean exit
     └──────────────────┘
```

- **`accepts_work()`** — Only true in `Scheduled` state
- **`is_drained()`** — True when `Draining` AND `active_tasks == 0`
- **Force shutdown** — Can jump from any state to `Shutdown`
- **Event history** — All transitions recorded with `SystemTime` timestamps

```rust
use orbit_star::lifecycle::{StarLifecycle, LifecycleState};

let mut lc = StarLifecycle::new("star-1");
assert_eq!(lc.state(), LifecycleState::Registered);

lc.on_scheduled();
assert!(lc.accepts_work());

lc.task_started();
lc.on_draining();
assert!(!lc.accepts_work());
assert!(!lc.is_drained()); // still has active task

lc.task_completed();
assert!(lc.is_drained()); // safe to shut down
```

---

## Integration Patterns

### Penalization → Dead-Letter Flow

```rust
let exhausted = penalty_box.penalize(&item_key, &error_message);
if exhausted {
    dlq.push(DeadLetterEntry {
        item_key,
        failure_reason: FailureReason::RetriesExhausted {
            attempts: penalty_box.penalty_count(&item_key),
        },
        // ...
    });
}
```

### Backpressure → Health Monitor Feedback

```rust
let advisory = health_monitor.check(&sample);
match advisory {
    Advisory::DiskCritical { .. } => {
        // Pause all backpressure guards for this destination
        // until disk space recovers
    }
    Advisory::ThroughputLow { .. } => {
        // Reduce backpressure thresholds temporarily
    }
    _ => {}
}
```

### Provenance → Bulletin Board Forwarding

```rust
// On chunk dead-lettered event, post to bulletin board
if event.event_type == ProvenanceType::ChunkDeadLettered {
    board.post(
        Severity::Error,
        &event.star_id,
        "dead-letter",
        Some(event.job_id),
        &format!("Chunk {} dead-lettered: {}", event.chunk_hash, event.error),
    );
}
```

---

## Test Coverage

| Module | Tests | Key Areas |
|--------|-------|-----------|
| Penalization | 19 | Exponential backoff, max penalties, eligibility, clear/re-penalize |
| Dead-Letter Queue | 16 | Capacity overflow, drain, per-job filtering, field preservation |
| Backpressure | 18 | Threshold boundaries, concurrent access, registry management |
| Ref-Counted GC | 19 | WAL gating, increment/decrement, deduplication, full lifecycle |
| Health Monitor | 17 | Exact thresholds, disk prediction, error rate, history pruning |
| Container Packing | 16 | Empty chunks, invalid magic, pool rotation, multiple reads |
| Typed Provenance | 17 | Logger write/read, parser edge cases, event classification |
| Bulletin Board | 21 | Severity filter, concurrent access, serde roundtrips, category |
| Composable Prioritizers | 17 | Chain composition, sort stability, tiebreaker, empty chain |
| Star Lifecycle | 20 | All state transitions, force shutdown, task tracking, serde |
| **Total** | **180** | |
