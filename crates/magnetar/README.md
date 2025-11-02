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
- **Resilience Module**: Circuit breaker, connection pooling, and rate limiting for fault-tolerant data access ⭐ **NEW in v0.4.1!**

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

## Resilience Module

**NEW in v0.4.1!** Magnetar includes a comprehensive resilience module for fault-tolerant access to flaky external services like S3, SMB, and databases.

### Components

- **Circuit Breaker** — Prevents cascading failures with automatic recovery
- **Connection Pool** — Efficient connection reuse with health checking
- **Rate Limiter** — Token bucket rate limiting to prevent service overload

### Quick Example

```rust
use magnetar::resilience::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), ResilienceError> {
    // Setup resilience stack
    let breaker = CircuitBreaker::new_default();
    let pool = Arc::new(ConnectionPool::new_default(factory));
    let limiter = RateLimiter::per_second(100);

    // Execute with full protection
    breaker.execute(|| {
        let pool = pool.clone();
        let limiter = limiter.clone();
        async move {
            limiter.execute(|| async {
                let conn = pool.acquire().await?;
                let result = perform_operation(&conn).await;
                pool.release(conn).await;
                result
            }).await
        }
    }).await?;

    Ok(())
}
```

### Features

- ✅ Three-state circuit breaker (Closed → Open → HalfOpen)
- ✅ Exponential backoff with configurable retries
- ✅ Generic connection pool with health checks
- ✅ Pool statistics and monitoring
- ✅ Idle timeout and max lifetime management
- ✅ Rate limiting with token bucket algorithm
- ✅ Optional governor crate integration
- ✅ Thread-safe async/await support
- ✅ Transient vs permanent error classification

### S3 Integration Example

```rust
use aws_sdk_s3::Client;
use magnetar::resilience::{ConnectionFactory, ResilienceError};

struct S3ClientFactory {
    config: aws_config::SdkConfig,
}

#[async_trait::async_trait]
impl ConnectionFactory<Client> for S3ClientFactory {
    async fn create(&self) -> Result<Client, ResilienceError> {
        Ok(Client::new(&self.config))
    }

    async fn is_healthy(&self, _client: &Client) -> bool {
        true
    }
}
```

📖 **Full Documentation:** See [`src/resilience/README.md`](src/resilience/README.md)

## Examples

Run the included examples:

```bash
# Basic usage
cargo run --example basic_usage --features sqlite

# DAG dependencies
cargo run --example dag_dependencies --features sqlite

# Crash recovery simulation
cargo run --example crash_recovery --features sqlite

# Resilience demo (circuit breaker, connection pool, rate limiter)
cargo run --example resilience_demo --features resilience
```

## Features

```toml
[features]
default = ["sqlite", "resilience"]
sqlite = ["dep:sqlx"]
redb = ["dep:redb", "dep:bincode"]
analytics = ["dep:polars", "dep:arrow"]
resilience = []
resilience-governor = ["resilience", "dep:governor"]
s3-integration = ["resilience", "dep:aws-config", "dep:aws-sdk-s3", "dep:hyper"]
all = ["sqlite", "redb", "analytics", "resilience-governor", "s3-integration"]
```

## Testing

```bash
# Run all tests
cargo test --all-features

# Test specific backend
cargo test --features sqlite
cargo test --features redb

# Test resilience module
cargo test --features resilience --lib
cargo test --features resilience --test resilience_integration_tests
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

- **Data Pipelines**: Chunked file processing with resume capability and resilient S3/SMB access
- **ETL Jobs**: Transform large datasets with failure recovery and circuit breaker protection
- **Distributed Computation**: Track work units across workers with connection pooling
- **Build Systems**: Incremental builds with dependency tracking
- **Cloud Data Transfer**: S3 uploads/downloads with rate limiting and automatic retry
- **Network File Operations**: SMB transfers with circuit breaker for flaky connections

## License

Licensed under Apache 2.0. See [LICENSE](../../LICENSE) for details.

## Contributing

Contributions welcome! This is part of the [Orbit](https://github.com/saworbit/orbit) project workspace.
