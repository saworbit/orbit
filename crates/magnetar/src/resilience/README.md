# Resilience Module

A comprehensive resilience toolkit for building fault-tolerant data access patterns in Rust.

## Overview

The resilience module provides three core components for handling transient failures in distributed systems:

1. **Circuit Breaker** - Prevents cascading failures by failing fast when a service is unhealthy
2. **Connection Pool** - Efficient connection reuse with health checking and lifecycle management
3. **Rate Limiter** - Token-based rate limiting to prevent overwhelming external services

## Features

- **Async/await support** - Built on tokio for high-performance async operations
- **Thread-safe** - All components are `Send + Sync` and can be safely shared across threads
- **Configurable** - Extensive configuration options for all components
- **Type-safe** - Generic implementations that work with any connection type
- **Well-tested** - Comprehensive unit and integration tests
- **Production-ready** - Used for resilient S3, SMB, and database access

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
magnetar = { version = "0.1", features = ["resilience"] }
```

### Basic Circuit Breaker

```rust
use magnetar::resilience::{CircuitBreaker, CircuitBreakerConfig, ResilienceError};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ResilienceError> {
    let config = CircuitBreakerConfig {
        failure_threshold: 5,
        success_threshold: 2,
        cooldown: Duration::from_secs(60),
        ..Default::default()
    };

    let breaker = CircuitBreaker::new(config);

    // Execute with automatic retry and circuit breaker protection
    let result = breaker.execute(|| async {
        // Your potentially failing operation
        make_api_call().await
    }).await?;

    Ok(())
}
```

### Connection Pool

```rust
use magnetar::resilience::{ConnectionPool, PoolConfig, ConnectionFactory, ResilienceError};
use std::sync::Arc;

struct HttpClientFactory;

#[async_trait::async_trait]
impl ConnectionFactory<HttpClient> for HttpClientFactory {
    async fn create(&self) -> Result<HttpClient, ResilienceError> {
        Ok(HttpClient::new())
    }

    async fn is_healthy(&self, client: &HttpClient) -> bool {
        client.is_connected()
    }
}

#[tokio::main]
async fn main() -> Result<(), ResilienceError> {
    let factory = Arc::new(HttpClientFactory);
    let config = PoolConfig {
        max_size: 10,
        min_idle: 2,
        ..Default::default()
    };

    let pool = ConnectionPool::new(factory, config);

    // Acquire connection
    let client = pool.acquire().await?;

    // Use connection
    let response = client.get("https://api.example.com").await?;

    // Return to pool
    pool.release(client).await;

    Ok(())
}
```

### Combined Usage

```rust
use magnetar::resilience::prelude::*;
use std::sync::Arc;

async fn resilient_api_call(
    breaker: &CircuitBreaker,
    pool: &Arc<ConnectionPool<HttpClient>>,
    limiter: &RateLimiter,
) -> Result<String, ResilienceError> {
    breaker.execute(|| {
        let pool = pool.clone();
        let limiter = limiter.clone();
        async move {
            limiter.execute(|| async {
                let client = pool.acquire().await?;
                let response = client.get("https://api.example.com").await
                    .map_err(|e| ResilienceError::Transient(e.to_string()))?;
                pool.release(client).await;
                Ok(response)
            }).await
        }
    }).await
}
```

## Circuit Breaker States

The circuit breaker has three states:

1. **Closed** (normal operation)
   - Requests pass through normally
   - Failures are counted
   - Transitions to Open when failure threshold is reached

2. **Open** (rejecting requests)
   - All requests fail immediately with `CircuitOpen` error
   - No actual operations are executed
   - Transitions to HalfOpen after cooldown period

3. **HalfOpen** (testing recovery)
   - Limited requests are allowed through
   - Success transitions back to Closed
   - Failure transitions back to Open

## Configuration

### Circuit Breaker Config

```rust
CircuitBreakerConfig {
    failure_threshold: 5,      // Failures before opening circuit
    success_threshold: 2,      // Successes in half-open to close
    cooldown: Duration::from_secs(60),  // Time before half-open
    initial_backoff: Duration::from_millis(100),
    max_backoff: Duration::from_secs(30),
    backoff_multiplier: 2.0,   // Exponential backoff factor
    max_retries: 3,            // Maximum retry attempts
}
```

### Connection Pool Config

```rust
PoolConfig {
    max_size: 10,              // Maximum connections
    min_idle: 2,               // Minimum idle connections to maintain
    idle_timeout: Some(Duration::from_secs(300)),   // Idle connection timeout
    max_lifetime: Some(Duration::from_secs(1800)),  // Max connection lifetime
    acquire_timeout: Duration::from_secs(30),       // Timeout for acquiring
}
```

### Rate Limiter

```rust
// 100 requests per second
let limiter = RateLimiter::per_second(100);

// 60 requests per minute
let limiter = RateLimiter::per_minute(60);

// Custom configuration
let limiter = RateLimiter::new(1000, Duration::from_secs(60));
```

## Error Handling

The module uses a custom `ResilienceError` type:

```rust
pub enum ResilienceError {
    CircuitOpen,              // Circuit breaker is open
    PoolExhausted,            // No connections available
    RateLimitExceeded,        // Rate limit hit
    Transient(String),        // Temporary error (will be retried)
    Permanent(String),        // Permanent error (no retry)
    ConnectionCreation(String),
    UnhealthyConnection(String),
    Timeout(Duration),
    MaxRetriesExceeded(usize),
}
```

Errors are classified as:
- **Transient** - Will be retried automatically
- **Permanent** - Will not be retried
- **Circuit-related** - Affect circuit breaker state

## Advanced Features

### Health Checking

The connection pool automatically checks connection health:

```rust
impl ConnectionFactory<MyConnection> for MyFactory {
    async fn is_healthy(&self, conn: &MyConnection) -> bool {
        // Perform health check
        conn.ping().await.is_ok()
    }
}
```

### Connection Lifecycle

```rust
impl ConnectionFactory<MyConnection> for MyFactory {
    async fn close(&self, conn: MyConnection) {
        // Custom cleanup before closing
        conn.shutdown().await;
    }
}
```

### Pool Statistics

```rust
let stats = pool.stats().await;
println!("Idle: {}, Active: {}, Utilization: {:.2}%",
    stats.idle,
    stats.active,
    stats.utilization()
);
```

## Integration Examples

### S3 Client with Resilience

```rust
use aws_sdk_s3::Client;
use magnetar::resilience::prelude::*;

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

async fn upload_with_resilience(
    breaker: &CircuitBreaker,
    pool: &ConnectionPool<Client>,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
) -> Result<(), ResilienceError> {
    breaker.execute(|| async {
        let client = pool.acquire().await?;

        client.put_object()
            .bucket(bucket)
            .key(key)
            .body(data.into())
            .send()
            .await
            .map_err(|e| ResilienceError::Transient(e.to_string()))?;

        pool.release(client).await;
        Ok(())
    }).await
}
```

### Database with Connection Pool

```rust
use sqlx::PgPool;

struct DbPoolFactory {
    database_url: String,
}

#[async_trait::async_trait]
impl ConnectionFactory<PgPool> for DbPoolFactory {
    async fn create(&self) -> Result<PgPool, ResilienceError> {
        PgPool::connect(&self.database_url)
            .await
            .map_err(|e| ResilienceError::ConnectionCreation(e.to_string()))
    }

    async fn is_healthy(&self, pool: &PgPool) -> bool {
        sqlx::query("SELECT 1")
            .execute(pool)
            .await
            .is_ok()
    }
}
```

## Performance Considerations

- **Pool overhead**: Minimal - uses `Arc` and `Mutex` for thread-safety
- **Circuit breaker**: O(1) state checks with atomic operations
- **Rate limiting**: Simple token bucket with minimal overhead
- **Zero-copy**: Connection reuse eliminates setup overhead

## Testing

Run tests:

```bash
# Unit tests
cargo test --features resilience --lib

# Integration tests
cargo test --features resilience --test resilience_integration_tests

# Run examples
cargo run --example resilience_demo --features resilience
```

## License

Apache-2.0

## Contributing

Contributions welcome! Please see CONTRIBUTING.md for details.
