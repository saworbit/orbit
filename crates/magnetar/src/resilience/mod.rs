//! Resilience module for fault-tolerant data access
//!
//! This module provides building blocks for creating resilient systems that can handle
//! transient failures in external services like S3, SMB, or databases. It includes:
//!
//! - **Circuit Breaker**: Prevents cascading failures by failing fast when a service is unhealthy
//! - **Connection Pool**: Efficient connection reuse with health checking and lifecycle management
//! - **Rate Limiter**: Token-based rate limiting to prevent overwhelming external services
//!
//! # Architecture
//!
//! These components are designed to work together but can also be used independently:
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
//! use magnetar::resilience::{CircuitBreaker, CircuitBreakerConfig, ResilienceError};
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
//! use magnetar::resilience::{ConnectionPool, PoolConfig, ConnectionFactory, ResilienceError};
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
//!
//! ## Combined Usage
//!
//! ```no_run
//! use magnetar::resilience::{
//!     CircuitBreaker, CircuitBreakerConfig, ConnectionPool, PoolConfig,
//!     ConnectionFactory, RateLimiter, ResilienceError,
//! };
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! # #[derive(Clone)]
//! # struct MyConnection;
//! # struct MyConnectionFactory;
//! # #[async_trait::async_trait]
//! # impl ConnectionFactory<MyConnection> for MyConnectionFactory {
//! #     async fn create(&self) -> Result<MyConnection, ResilienceError> {
//! #         Ok(MyConnection)
//! #     }
//! #     async fn is_healthy(&self, _conn: &MyConnection) -> bool {
//! #         true
//! #     }
//! # }
//! #
//! # async fn perform_operation(_conn: &MyConnection) -> Result<String, ResilienceError> {
//! #     Ok("result".to_string())
//! # }
//!
//! # async fn example() -> Result<(), ResilienceError> {
//! // Setup components
//! let breaker = CircuitBreaker::new_default();
//! let factory = Arc::new(MyConnectionFactory);
//! let pool = Arc::new(ConnectionPool::new_default(factory));
//! let limiter = RateLimiter::per_second(100);
//!
//! // Execute with full protection
//! let result = breaker.execute(|| {
//!     let pool = pool.clone();
//!     async move {
//!         limiter.execute(|| async {
//!             let conn = pool.acquire().await?;
//!             let result = perform_operation(&conn).await;
//!             pool.release(conn).await;
//!             result
//!         }).await
//!     }
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Integration with S3/SMB
//!
//! For S3 operations using AWS SDK:
//!
//! ```no_run
//! # #[cfg(feature = "s3-integration")]
//! # mod s3_example {
//! use magnetar::resilience::{CircuitBreaker, ConnectionPool, ConnectionFactory, ResilienceError};
//! use std::sync::Arc;
//!
//! // Connection factory for S3 clients
//! struct S3ClientFactory {
//!     config: aws_config::SdkConfig,
//! }
//!
//! #[async_trait::async_trait]
//! impl ConnectionFactory<aws_sdk_s3::Client> for S3ClientFactory {
//!     async fn create(&self) -> Result<aws_sdk_s3::Client, ResilienceError> {
//!         Ok(aws_sdk_s3::Client::new(&self.config))
//!     }
//!
//!     async fn is_healthy(&self, _client: &aws_sdk_s3::Client) -> bool {
//!         // Could implement health check by listing buckets or similar
//!         true
//!     }
//! }
//!
//! async fn upload_with_resilience(
//!     breaker: &CircuitBreaker,
//!     pool: &ConnectionPool<aws_sdk_s3::Client>,
//!     bucket: &str,
//!     key: &str,
//!     data: Vec<u8>,
//! ) -> Result<(), ResilienceError> {
//!     breaker.execute(|| async {
//!         let client = pool.acquire().await?;
//!
//!         let result = client
//!             .put_object()
//!             .bucket(bucket)
//!             .key(key)
//!             .body(data.into())
//!             .send()
//!             .await
//!             .map_err(|e| ResilienceError::Transient(e.to_string()))?;
//!
//!         pool.release(client).await;
//!         Ok(())
//!     }).await
//! }
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

#[cfg(feature = "resilience-governor")]
pub use rate_limiter::governor_impl::GovernorRateLimiter;

/// Prelude module for convenient imports
///
/// # Example
/// ```
/// use magnetar::resilience::prelude::*;
/// ```
pub mod prelude {
    pub use super::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    pub use super::connection_pool::{ConnectionFactory, ConnectionPool, PoolConfig};
    pub use super::error::ResilienceError;
    pub use super::rate_limiter::RateLimiter;
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    struct TestConnection {
        id: usize,
    }

    struct TestFactory {
        counter: Arc<tokio::sync::Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl ConnectionFactory<TestConnection> for TestFactory {
        async fn create(&self) -> Result<TestConnection, ResilienceError> {
            let mut counter = self.counter.lock().await;
            *counter += 1;
            Ok(TestConnection { id: *counter })
        }

        async fn is_healthy(&self, _conn: &TestConnection) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_integrated_resilience() {
        // Setup all components
        let breaker = CircuitBreaker::new_default();
        let factory = Arc::new(TestFactory {
            counter: Arc::new(tokio::sync::Mutex::new(0)),
        });
        let pool = Arc::new(ConnectionPool::new_default(factory));
        let limiter = RateLimiter::per_second(100);

        // Execute operation with full resilience stack
        let result = breaker
            .execute(|| {
                let pool = pool.clone();
                let limiter = limiter.clone();
                async move {
                    limiter
                        .execute(|| async {
                            let conn = pool.acquire().await?;
                            // Simulate operation
                            let result = Ok::<_, ResilienceError>(conn.id);
                            pool.release(conn).await;
                            result
                        })
                        .await
                }
            })
            .await;

        assert!(result.is_ok());
    }
}
