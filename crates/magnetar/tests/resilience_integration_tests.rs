//! Integration tests for the resilience module
//!
//! These tests verify that all resilience components work together correctly
//! and handle various failure scenarios.

#![cfg(feature = "resilience")]

use magnetar::resilience::prelude::*;
use magnetar::resilience::CircuitState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Test connection for integration tests
#[derive(Clone, Debug)]
struct TestConnection {
    id: usize,
    healthy: Arc<std::sync::atomic::AtomicBool>,
}

impl TestConnection {
    fn new(id: usize) -> Self {
        Self {
            id,
            healthy: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    fn mark_unhealthy(&self) {
        self.healthy.store(false, Ordering::SeqCst);
    }

    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::SeqCst)
    }
}

/// Test connection factory
struct TestConnectionFactory {
    counter: Arc<tokio::sync::Mutex<usize>>,
    should_fail: Arc<std::sync::atomic::AtomicBool>,
}

impl TestConnectionFactory {
    fn new() -> Self {
        Self {
            counter: Arc::new(tokio::sync::Mutex::new(0)),
            should_fail: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn set_should_fail(&self, should_fail: bool) {
        self.should_fail.store(should_fail, Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl ConnectionFactory<TestConnection> for TestConnectionFactory {
    async fn create(&self) -> Result<TestConnection, ResilienceError> {
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(ResilienceError::ConnectionCreation(
                "Simulated connection failure".to_string(),
            ));
        }

        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(TestConnection::new(*counter))
    }

    async fn is_healthy(&self, conn: &TestConnection) -> bool {
        conn.is_healthy()
    }
}

#[tokio::test]
async fn test_circuit_breaker_basic_flow() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        cooldown: Duration::from_millis(100),
        max_retries: 1,
        ..Default::default()
    };
    let breaker = CircuitBreaker::new(config);

    // Successful operations
    for i in 0..5 {
        let result = breaker.call(|| async { Ok::<_, ResilienceError>(i) }).await;
        assert_eq!(result.unwrap(), i);
    }

    // Circuit should still be closed
    assert_eq!(breaker.get_state().await, CircuitState::Closed);
}

#[tokio::test]
async fn test_circuit_breaker_opens_on_failures() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        cooldown: Duration::from_millis(100),
        ..Default::default()
    };
    let breaker = CircuitBreaker::new(config);

    // Cause failures to open circuit
    for _ in 0..3 {
        let result: Result<(), ResilienceError> = breaker
            .call(|| async { Err(ResilienceError::Transient("error".to_string())) })
            .await;
        assert!(result.is_err());
    }

    // Circuit should be open now
    match breaker.get_state().await {
        CircuitState::Open { .. } => (),
        state => panic!("Expected Open state, got {:?}", state),
    }

    // Next call should fail immediately
    let result = breaker
        .call(|| async { Ok::<_, ResilienceError>(42) })
        .await;
    assert!(matches!(result, Err(ResilienceError::CircuitOpen)));
}

#[tokio::test]
async fn test_circuit_breaker_half_open_recovery() {
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        success_threshold: 2,
        cooldown: Duration::from_millis(50),
        ..Default::default()
    };
    let breaker = CircuitBreaker::new(config);

    // Open the circuit
    for _ in 0..2 {
        let _: Result<(), ResilienceError> = breaker
            .call(|| async { Err(ResilienceError::Transient("error".to_string())) })
            .await;
    }

    // Wait for cooldown
    tokio::time::sleep(Duration::from_millis(60)).await;

    // Successful operations should close the circuit
    for i in 0..2 {
        let result = breaker.call(|| async { Ok::<_, ResilienceError>(i) }).await;
        assert!(result.is_ok());
    }

    // Circuit should be closed
    assert_eq!(breaker.get_state().await, CircuitState::Closed);
}

#[tokio::test]
async fn test_connection_pool_basic_operations() {
    let factory = Arc::new(TestConnectionFactory::new());
    let config = PoolConfig {
        max_size: 5,
        min_idle: 1,
        ..Default::default()
    };
    let pool = ConnectionPool::new(factory, config);

    // Acquire and release connections
    let conn1 = pool.acquire().await.unwrap();
    assert_eq!(conn1.id, 1);

    let conn2 = pool.acquire().await.unwrap();
    assert_eq!(conn2.id, 2);

    pool.release(conn1).await;
    pool.release(conn2).await;

    // Pool should have 2 idle connections
    let stats = pool.stats().await;
    assert_eq!(stats.idle, 2);
    assert_eq!(stats.active, 0);

    // Reusing connection
    let conn3 = pool.acquire().await.unwrap();
    assert!(conn3.id == 1 || conn3.id == 2); // Should reuse existing
}

#[tokio::test]
async fn test_connection_pool_max_size() {
    let factory = Arc::new(TestConnectionFactory::new());
    let config = PoolConfig {
        max_size: 2,
        acquire_timeout: Duration::from_millis(100),
        ..Default::default()
    };
    let pool = Arc::new(ConnectionPool::new(factory, config));

    // Acquire max connections
    let conn1 = pool.acquire().await.unwrap();
    let conn2 = pool.acquire().await.unwrap();

    // Try to acquire one more - should timeout
    let result = pool.acquire().await;
    assert!(matches!(result, Err(ResilienceError::Timeout(_))));

    // Release one connection
    pool.release(conn1).await;

    // Now should be able to acquire
    let conn3 = pool.acquire().await.unwrap();
    assert!(conn3.id <= 2); // Should be reused

    pool.release(conn2).await;
    pool.release(conn3).await;
}

#[tokio::test]
async fn test_connection_pool_health_check() {
    let factory = Arc::new(TestConnectionFactory::new());
    let pool = ConnectionPool::new_default(factory);

    // Acquire connection and mark it unhealthy
    let conn = pool.acquire().await.unwrap();
    conn.mark_unhealthy();
    pool.release(conn).await;

    // Next acquire should create a new connection (unhealthy one discarded)
    let conn2 = pool.acquire().await.unwrap();
    assert!(conn2.is_healthy());
    assert_eq!(conn2.id, 2); // New connection
}

#[tokio::test]
async fn test_rate_limiter_execution() {
    let limiter = RateLimiter::per_second(10);
    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..5 {
        let counter = counter.clone();
        limiter
            .execute(|| async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ResilienceError>(())
            })
            .await
            .unwrap();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

#[tokio::test]
async fn test_combined_resilience_stack() {
    // Setup all components
    let breaker_config = CircuitBreakerConfig {
        failure_threshold: 5,
        ..Default::default()
    };
    let breaker = CircuitBreaker::new(breaker_config);

    let factory = Arc::new(TestConnectionFactory::new());
    let pool_config = PoolConfig {
        max_size: 3,
        ..Default::default()
    };
    let pool = Arc::new(ConnectionPool::new(factory, pool_config));

    let limiter = RateLimiter::per_second(100);

    let success_count = Arc::new(AtomicUsize::new(0));

    // Execute multiple operations through the full stack
    for _ in 0..10 {
        let pool = pool.clone();
        let success_count = success_count.clone();
        let limiter = limiter.clone();

        let result = breaker
            .execute(|| {
                let pool = pool.clone();
                let success_count = success_count.clone();
                let limiter = limiter.clone();
                async move {
                    limiter
                        .execute(|| async {
                            let conn = pool.acquire().await?;
                            // Simulate work
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            success_count.fetch_add(1, Ordering::SeqCst);
                            pool.release(conn).await;
                            Ok::<_, ResilienceError>(())
                        })
                        .await
                }
            })
            .await;

        assert!(result.is_ok());
    }

    assert_eq!(success_count.load(Ordering::SeqCst), 10);
}

#[tokio::test]
async fn test_resilience_under_failure() {
    let breaker_config = CircuitBreakerConfig {
        failure_threshold: 3,
        cooldown: Duration::from_millis(100),
        max_retries: 2,
        ..Default::default()
    };
    let breaker = CircuitBreaker::new(breaker_config);

    let factory = Arc::new(TestConnectionFactory::new());
    let pool = Arc::new(ConnectionPool::new_default(factory.clone()));

    // Simulate connection failures
    factory.set_should_fail(true);

    let mut failures = 0;
    for _ in 0..5 {
        let pool = pool.clone();
        let result = breaker
            .execute(|| {
                let pool = pool.clone();
                async move {
                    let _conn = pool.acquire().await?;
                    Ok::<_, ResilienceError>(())
                }
            })
            .await;

        if result.is_err() {
            failures += 1;
        }
    }

    // Should have failures due to circuit breaker opening
    assert!(failures > 0);

    // Restore factory and wait for cooldown
    factory.set_should_fail(false);
    tokio::time::sleep(Duration::from_millis(150)).await;
    breaker.reset().await;

    // Should succeed now
    let pool = pool.clone();
    let result = breaker
        .call(|| {
            let pool = pool.clone();
            async move {
                let conn = pool.acquire().await?;
                pool.release(conn).await;
                Ok::<_, ResilienceError>(())
            }
        })
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_pool_stats() {
    let factory = Arc::new(TestConnectionFactory::new());
    let config = PoolConfig {
        max_size: 10,
        ..Default::default()
    };
    let pool = ConnectionPool::new(factory, config);

    let conn1 = pool.acquire().await.unwrap();
    let conn2 = pool.acquire().await.unwrap();

    let stats = pool.stats().await;
    assert_eq!(stats.active, 2);
    assert_eq!(stats.total, 2);
    assert!(stats.utilization() > 0.0);
    assert!(stats.utilization() <= 100.0);

    pool.release(conn1).await;

    let stats = pool.stats().await;
    assert_eq!(stats.active, 1);
    assert_eq!(stats.idle, 1);

    pool.release(conn2).await;
}
