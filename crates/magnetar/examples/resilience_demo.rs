//! Comprehensive demonstration of the resilience module
//!
//! This example shows how to use circuit breakers, connection pools,
//! and rate limiters together for robust data access.
//!
//! Run with: cargo run --example resilience_demo --features resilience

#![cfg(feature = "resilience")]

use magnetar::resilience::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Simulated external service connection
#[derive(Clone, Debug)]
struct ServiceConnection {
    id: usize,
    healthy: Arc<AtomicBool>,
    request_count: Arc<AtomicUsize>,
}

impl ServiceConnection {
    fn new(id: usize) -> Self {
        Self {
            id,
            healthy: Arc::new(AtomicBool::new(true)),
            request_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    async fn make_request(&self, data: &str) -> Result<String, ResilienceError> {
        let count = self.request_count.fetch_add(1, Ordering::SeqCst);

        // Simulate occasional transient failures
        if count % 10 == 7 {
            return Err(ResilienceError::Transient(
                "Simulated network timeout".to_string(),
            ));
        }

        // Simulate processing
        tokio::time::sleep(Duration::from_millis(10)).await;

        Ok(format!(
            "Response from connection {}: processed '{}'",
            self.id, data
        ))
    }

    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::SeqCst)
    }
}

/// Factory for creating service connections
struct ServiceConnectionFactory {
    counter: Arc<tokio::sync::Mutex<usize>>,
}

impl ServiceConnectionFactory {
    fn new() -> Self {
        Self {
            counter: Arc::new(tokio::sync::Mutex::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl ConnectionFactory<ServiceConnection> for ServiceConnectionFactory {
    async fn create(&self) -> Result<ServiceConnection, ResilienceError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        println!("📡 Creating new connection #{}", *counter);

        // Simulate connection establishment delay
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(ServiceConnection::new(*counter))
    }

    async fn is_healthy(&self, conn: &ServiceConnection) -> bool {
        conn.is_healthy()
    }

    async fn close(&self, conn: ServiceConnection) {
        println!("🔌 Closing connection #{}", conn.id);
    }
}

/// Example 1: Basic circuit breaker usage
async fn example_circuit_breaker() -> Result<(), ResilienceError> {
    println!("\n=== Example 1: Circuit Breaker ===\n");

    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        cooldown: Duration::from_secs(2),
        max_retries: 2,
        ..Default::default()
    };

    let breaker = CircuitBreaker::new(config);

    // Successful operations
    println!("✅ Executing successful operations...");
    for i in 0..5 {
        breaker
            .call(|| async move {
                println!("  Operation {} succeeded", i + 1);
                Ok::<_, ResilienceError>(i)
            })
            .await?;
    }

    // Simulate failures
    println!("\n❌ Simulating failures to open circuit...");
    for i in 0..3 {
        let result = breaker
            .call(|| async move {
                Err::<(), _>(ResilienceError::Transient(format!("Failure {}", i + 1)))
            })
            .await;
        println!("  Failure {}: {:?}", i + 1, result);
    }

    println!(
        "\n🔒 Circuit state: {:?}",
        breaker.get_state().await
    );

    // Try operation with open circuit
    println!("\n⚠️  Attempting operation with open circuit...");
    let result = breaker
        .call(|| async { Ok::<_, ResilienceError>(42) })
        .await;
    println!("  Result: {:?}", result);

    Ok(())
}

/// Example 2: Connection pool usage
async fn example_connection_pool() -> Result<(), ResilienceError> {
    println!("\n=== Example 2: Connection Pool ===\n");

    let factory = Arc::new(ServiceConnectionFactory::new());
    let config = PoolConfig {
        max_size: 5,
        min_idle: 2,
        idle_timeout: Some(Duration::from_secs(10)),
        ..Default::default()
    };

    let pool = Arc::new(ConnectionPool::new(factory, config));

    println!("📊 Initial pool stats: {:?}", pool.stats().await);

    // Acquire and use connections
    println!("\n🔄 Acquiring connections...");
    let mut connections = Vec::new();
    for _ in 0..3 {
        let conn = pool.acquire().await?;
        println!("  Acquired connection #{}", conn.id);
        connections.push(conn);
    }

    println!("\n📊 Pool stats with active connections: {:?}", pool.stats().await);

    // Release connections back to pool
    println!("\n🔄 Releasing connections...");
    for conn in connections {
        println!("  Releasing connection #{}", conn.id);
        pool.release(conn).await;
    }

    println!("\n📊 Final pool stats: {:?}", pool.stats().await);

    // Reuse connection
    println!("\n♻️  Reusing connection from pool...");
    let conn = pool.acquire().await?;
    println!("  Reused connection #{}", conn.id);
    pool.release(conn).await;

    Ok(())
}

/// Example 3: Rate limiting
async fn example_rate_limiting() -> Result<(), ResilienceError> {
    println!("\n=== Example 3: Rate Limiting ===\n");

    let limiter = RateLimiter::per_second(5);
    println!("⏱️  Rate limiter: 5 requests per second\n");

    let start = std::time::Instant::now();

    for i in 0..10 {
        limiter
            .execute(|| async move {
                println!("  Request {} at {:?}", i + 1, start.elapsed());
                Ok::<_, ResilienceError>(())
            })
            .await?;
    }

    println!("\n✅ Completed 10 requests in {:?}", start.elapsed());

    Ok(())
}

/// Example 4: Combined resilience stack
async fn example_combined_resilience() -> Result<(), ResilienceError> {
    println!("\n=== Example 4: Combined Resilience Stack ===\n");

    // Setup circuit breaker
    let breaker_config = CircuitBreakerConfig {
        failure_threshold: 5,
        success_threshold: 2,
        cooldown: Duration::from_secs(3),
        max_retries: 2,
        ..Default::default()
    };
    let breaker = CircuitBreaker::new(breaker_config);

    // Setup connection pool
    let factory = Arc::new(ServiceConnectionFactory::new());
    let pool_config = PoolConfig {
        max_size: 3,
        min_idle: 1,
        ..Default::default()
    };
    let pool = Arc::new(ConnectionPool::new(factory, pool_config));

    // Setup rate limiter
    let limiter = RateLimiter::per_second(10);

    // Statistics
    let success_count = Arc::new(AtomicUsize::new(0));
    let failure_count = Arc::new(AtomicUsize::new(0));

    println!("🚀 Executing 20 requests through resilience stack...\n");

    for i in 0..20 {
        let pool = pool.clone();
        let limiter = limiter.clone();
        let success_count = success_count.clone();
        let failure_count = failure_count.clone();

        let result = breaker
            .execute(|| {
                let pool = pool.clone();
                let limiter = limiter.clone();
                let success_count = success_count.clone();
                let failure_count = failure_count.clone();
                async move {
                    limiter
                        .execute(|| async {
                            let conn = pool.acquire().await?;

                            // Make request
                            let result = conn
                                .make_request(&format!("Request {}", i + 1))
                                .await;

                            pool.release(conn).await;

                            match result {
                                Ok(response) => {
                                    success_count.fetch_add(1, Ordering::SeqCst);
                                    println!("  ✅ {}", response);
                                    Ok(())
                                }
                                Err(e) => {
                                    failure_count.fetch_add(1, Ordering::SeqCst);
                                    println!("  ❌ Error: {}", e);
                                    Err(e)
                                }
                            }
                        })
                        .await
                }
            })
            .await;

        if result.is_err() {
            println!("  ⚠️  Request {} failed (circuit may be open)", i + 1);
        }
    }

    println!("\n📊 Final Statistics:");
    println!("  Successes: {}", success_count.load(Ordering::SeqCst));
    println!("  Failures: {}", failure_count.load(Ordering::SeqCst));
    println!("  Pool stats: {:?}", pool.stats().await);
    println!(
        "  Circuit state: {:?}",
        breaker.get_state().await
    );

    Ok(())
}

/// Example 5: Handling permanent failures
async fn example_permanent_failures() -> Result<(), ResilienceError> {
    println!("\n=== Example 5: Permanent vs Transient Failures ===\n");

    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        max_retries: 2,
        ..Default::default()
    };
    let breaker = CircuitBreaker::new(config);

    // Transient error - will be retried
    println!("🔄 Handling transient error (will retry)...");
    let result = breaker
        .execute(|| async {
            Err::<(), _>(ResilienceError::Transient(
                "Network timeout".to_string(),
            ))
        })
        .await;
    println!("  Result: {:?}\n", result);

    // Permanent error - will not be retried
    println!("🛑 Handling permanent error (no retry)...");
    let result = breaker
        .execute(|| async {
            Err::<(), _>(ResilienceError::Permanent(
                "Authentication failed".to_string(),
            ))
        })
        .await;
    println!("  Result: {:?}", result);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ResilienceError> {
    println!("╔═══════════════════════════════════════════╗");
    println!("║  Magnetar Resilience Module Demo         ║");
    println!("╚═══════════════════════════════════════════╝");

    // Run all examples
    if let Err(e) = example_circuit_breaker().await {
        eprintln!("Circuit breaker example failed: {}", e);
    }

    if let Err(e) = example_connection_pool().await {
        eprintln!("Connection pool example failed: {}", e);
    }

    if let Err(e) = example_rate_limiting().await {
        eprintln!("Rate limiting example failed: {}", e);
    }

    if let Err(e) = example_combined_resilience().await {
        eprintln!("Combined resilience example failed: {}", e);
    }

    if let Err(e) = example_permanent_failures().await {
        eprintln!("Permanent failures example failed: {}", e);
    }

    println!("\n✨ All examples completed!");

    Ok(())
}
