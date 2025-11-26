//! Orbit Core Resilience: Pure-logic fault tolerance primitives
//!
//! # Overview
//!
//! This crate provides building blocks for creating resilient systems that can handle
//! transient failures in external services. It includes:
//!
//! - **Circuit Breaker**: Prevents cascading failures by failing fast when a service is unhealthy
//! - **Connection Pool**: Efficient connection reuse with health checking and lifecycle management
//! - **Rate Limiter**: Token-based rate limiting to prevent overwhelming external services
//!
//! # Key Principles
//!
//! This crate is **pure logic** with zero knowledge of:
//! - Storage systems (databases, file systems)
//! - Network protocols (S3, SMB, HTTP)
//! - Application-specific concerns
//!
//! It provides generic, composable fault-tolerance patterns that can be used across any layer.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         Your Application                │
//! └─────────────┬───────────────────────────┘
//!               │
//!               ▼
//! ┌─────────────────────────────────────────┐
//! │       Circuit Breaker                   │  ← Fail-fast protection
//! │  (Tracks failures, opens on threshold)  │
//! └─────────────┬───────────────────────────┘
//!               │
//!               ▼
//! ┌─────────────────────────────────────────┐
//! │       Rate Limiter                      │  ← Prevent overload
//! │  (Token bucket, enforces quotas)        │
//! └─────────────┬───────────────────────────┘
//!               │
//!               ▼
//! ┌─────────────────────────────────────────┐
//! │       Connection Pool                   │  ← Resource efficiency
//! │  (Reuse connections, health checks)     │
//! └─────────────┬───────────────────────────┘
//!               │
//!               ▼
//!         External Service
//!        (S3, SMB, Database)
//! ```
//!
//! # Usage Example
//!
//! ## Basic Circuit Breaker
//!
//! ```no_run
//! use orbit_core_resilience::{CircuitBreaker, CircuitBreakerConfig, ResilienceError};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), ResilienceError> {
//! let config = CircuitBreakerConfig {
//!     failure_threshold: 5,
//!     success_threshold: 2,
//!     cooldown: Duration::from_secs(60),
//!     ..Default::default()
//! };
//!
//! let breaker = CircuitBreaker::new(config);
//!
//! // Execute operation with retry and circuit breaker protection
//! let result = breaker.execute(|| async {
//!     // Your potentially failing operation
//!     Ok::<_, ResilienceError>(42)
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Connection Pool
//!
//! ```no_run
//! use orbit_core_resilience::{ConnectionPool, PoolConfig, ConnectionFactory, ResilienceError};
//! use std::sync::Arc;
//!
//! # #[derive(Clone)]
//! # struct MyConnection;
//! struct MyConnectionFactory;
//!
//! #[async_trait::async_trait]
//! impl ConnectionFactory<MyConnection> for MyConnectionFactory {
//!     async fn create(&self) -> Result<MyConnection, ResilienceError> {
//!         // Create your connection
//! #       Ok(MyConnection)
//!     }
//!
//!     async fn is_healthy(&self, _conn: &MyConnection) -> bool {
//!         // Check connection health
//!         true
//!     }
//! }
//!
//! # async fn example() -> Result<(), ResilienceError> {
//! let factory = Arc::new(MyConnectionFactory);
//! let pool = ConnectionPool::new_default(factory);
//!
//! // Acquire connection
//! let conn = pool.acquire().await?;
//! // Use connection...
//!
//! // Return to pool
//! pool.release(conn).await;
//! # Ok(())
//! # }
//! ```

pub mod circuit_breaker;
pub mod connection_pool;
pub mod error;
pub mod rate_limiter;

// Re-export main types for convenience
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use connection_pool::{ConnectionFactory, ConnectionPool, PoolConfig, PoolStats};
pub use error::ResilienceError;
pub use rate_limiter::RateLimiter;

#[cfg(feature = "governor-impl")]
pub use rate_limiter::governor_impl::GovernorRateLimiter;

/// Prelude module for convenient imports
///
/// # Example
/// ```
/// use orbit_core_resilience::prelude::*;
/// ```
pub mod prelude {
    pub use super::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    pub use super::connection_pool::{ConnectionFactory, ConnectionPool, PoolConfig};
    pub use super::error::ResilienceError;
    pub use super::rate_limiter::RateLimiter;
}
