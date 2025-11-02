# Magnetar

**Idempotent State Machine for Crash-Proof, Persistent Jobs in Rust**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

## Overview

Magnetar is a lightweight, embeddable state machine for managing idempotent jobs with persistent storage. It transforms ephemeral manifests (e.g., TOML/JSON) into durable database schemas, enabling crash-proof resumption, atomic chunk claims, and DAG-based dependency graphs.

**Tagline:** *"Grip. Flare. Persist. Forever."*

## Features

- **Idempotent Claims**: Atomic "pending → processing" transitions; retries without duplicates
- **Job Resumption**: Query pending chunks post-crash, ordered by chunk ID
- **DAG Dependencies**: Support for dependency graphs with topological sorting
- **Manifest Migration**: TOML/JSON → DB bulk upsert with parallel processing
- **Multiple Backends**: SQLite (default) and redb (pure Rust, WASM-ready)
- **Zero-Downtime Swaps**: Dual-write mode for live backend migration
- **Analytics Export**: Export to Parquet/CSV for integration with Polars/Arrow

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
magnetar = { path = "../magnetar" }  # Or version from crates.io when published
```

### Basic Usage

```rust
use magnetar::{JobStore, JobStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Open a job store (SQLite by default)
    let mut store = magnetar::open("jobs.db").await?;

    // Initialize from TOML manifest
    let manifest = toml::from_str(r#"
        [[chunks]]
        id = 1
        checksum = "abc123"

        [[chunks]]
        id = 2
        checksum = "def456"
    "#)?;

    store.init_from_manifest(42, &manifest).await?;

    // Process chunks
    while let Some(chunk) = store.claim_pending(42).await? {
        // Do work...
        println!("Processing chunk {} with checksum {}", chunk.chunk, chunk.checksum);

        // Mark complete
        store.mark_status(42, chunk.chunk, JobStatus::Done, None).await?;
    }

    Ok(())
}
```

### DAG Dependencies

```rust
// Task 3 depends on tasks 1 and 2
store.add_dependency(job_id, 3, vec![1, 2]).await?;

// Get tasks ready for processing (all dependencies completed)
let ready = store.topo_sort_ready(job_id).await?;
```

### Crash Recovery

```rust
// Resume pending chunks after a crash
let pending = store.resume_pending(job_id).await?;
for chunk in pending {
    println!("Resuming chunk {}", chunk.chunk);
}
```

## Architecture

### Core Abstraction

The `JobStore` trait provides a unified API across backends:

```rust
#[async_trait]
pub trait JobStore: Send + Sync {
    async fn init_from_manifest(&mut self, job_id: i64, manifest: &toml::Value) -> Result<()>;
    async fn claim_pending(&mut self, job_id: i64) -> Result<Option<JobState>>;
    async fn mark_status(&mut self, job_id: i64, chunk: u64, status: JobStatus, checksum: Option<String>) -> Result<()>;
    async fn resume_pending(&self, job_id: i64) -> Result<Vec<JobState>>;
    async fn topo_sort_ready(&self, job_id: i64) -> Result<Vec<u64>>;
    // ... more methods
}
```

### Backends

#### SQLite (default)
- SQL-based with WAL mode for concurrency
- Automatic migrations via `sqlx::migrate!`
- Ideal for analytics queries

#### redb (optional)
- Pure Rust embedded database
- MMAP for zero-copy reads
- WASM/embedded-friendly (no FFI)

## Examples

Run the included examples:

```bash
# Basic usage
cargo run --example basic_usage --features sqlite

# DAG dependencies
cargo run --example dag_dependencies --features sqlite

# Crash recovery simulation
cargo run --example crash_recovery --features sqlite
```

## Features

```toml
[features]
default = ["sqlite"]
sqlite = ["dep:sqlx"]
redb = ["dep:redb", "dep:bincode"]
analytics = ["dep:polars", "dep:arrow"]
all = ["sqlite", "redb", "analytics"]
```

## Testing

```bash
# Run all tests
cargo test --all-features

# Test specific backend
cargo test --features sqlite
cargo test --features redb
```

## Performance

Benchmarks on 10,000 chunks (local SQLite/redb):

| Operation | SQLite | redb |
|-----------|--------|------|
| Claim (single) | <1ms | <1ms |
| Resume scan | 5-10ms | 3-5ms |
| DAG topo-sort (100 nodes) | 20ms | 15ms |
| Bulk import | 100ms | 50ms |

## Use Cases

- **Data Pipelines**: Chunked file processing with resume capability
- **ETL Jobs**: Transform large datasets with failure recovery
- **Distributed Computation**: Track work units across workers
- **Build Systems**: Incremental builds with dependency tracking

## License

Licensed under Apache 2.0. See [LICENSE](../../LICENSE) for details.

## Contributing

Contributions welcome! This is part of the [Orbit](https://github.com/saworbit/orbit) project workspace.
