//! Generic connection pool for reusable connections
//!
//! Provides efficient connection reuse with configurable limits,
//! idle timeouts, and health checking.

use super::error::ResilienceError;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

/// Configuration for connection pool behavior
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    pub max_size: usize,
    /// Minimum number of idle connections to maintain
    pub min_idle: usize,
    /// Maximum time a connection can remain idle before being closed
    pub idle_timeout: Option<Duration>,
    /// Maximum lifetime of a connection
    pub max_lifetime: Option<Duration>,
    /// Timeout for acquiring a connection from the pool
    pub acquire_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_idle: 2,
            idle_timeout: Some(Duration::from_secs(300)), // 5 minutes
            max_lifetime: Some(Duration::from_secs(1800)), // 30 minutes
            acquire_timeout: Duration::from_secs(30),
        }
    }
}

impl PoolConfig {
    /// Neutrino profile: High concurrency for small-file workloads
    ///
    /// Optimized for the Neutrino Fast Lane with:
    /// - Very high max_size (500 concurrent connections)
    /// - High min_idle (50 warm connections)
    /// - Short idle_timeout (10s - connections are cheap)
    /// - Short max_lifetime (60s - frequent refresh)
    /// - Fast acquire_timeout (100ms - fail fast)
    ///
    /// # Use Case
    ///
    /// Designed for high-throughput scenarios where:
    /// - Many small files (<8KB) need to be transferred
    /// - Connection setup overhead exceeds data transfer time
    /// - I/O-bound workload can benefit from high concurrency
    ///
    /// # Example
    ///
    /// ```
    /// use orbit_core_resilience::connection_pool::PoolConfig;
    ///
    /// let config = PoolConfig::neutrino_profile();
    /// assert_eq!(config.max_size, 500);
    /// assert_eq!(config.min_idle, 50);
    /// ```
    pub fn neutrino_profile() -> Self {
        Self {
            max_size: 500,                               // Very high concurrency
            min_idle: 50,                                // Keep many warm
            idle_timeout: Some(Duration::from_secs(10)), // Short timeout
            max_lifetime: Some(Duration::from_secs(60)), // Frequent refresh
            acquire_timeout: Duration::from_millis(100), // Fail fast
        }
    }

    /// Long-haul profile: Configuration optimized for large file workloads (Gigantor)
    ///
    /// This profile is designed for long-running transfers of very large files
    /// (>1GB) where connections need to be maintained for extended periods.
    ///
    /// # Characteristics
    ///
    /// - **Strict Max Size**: Large transfers consume significant bandwidth.
    ///   We don't want 100 parallel 100GB transfers choking the pipe.
    ///   Default: 4 concurrent connections
    ///
    /// - **Min Idle**: Low. We assume these connections stay busy for hours.
    ///   Default: 1 connection
    ///
    /// - **Idle Timeout**: Standard. Connections that go idle for 5 minutes
    ///   are reclaimed.
    ///   Default: 300 seconds (5 minutes)
    ///
    /// - **Max Lifetime**: EXTENDED. A single S3 multipart upload session
    ///   might last hours. Don't kill it in the middle.
    ///   Default: 86400 seconds (24 hours)
    ///
    /// - **Acquire Timeout**: Long. Waiting for a slot on the "Heavy" lane
    ///   is expected when all connections are busy.
    ///   Default: 600 seconds (10 minutes)
    ///
    /// # Use Cases
    ///
    /// - Multi-hour S3 multipart uploads
    /// - Large database backup transfers
    /// - VM image synchronization
    /// - Video file transfers
    ///
    /// # Example
    ///
    /// ```
    /// use orbit_core_resilience::connection_pool::PoolConfig;
    ///
    /// let config = PoolConfig::long_haul_profile();
    /// assert_eq!(config.max_size, 4);
    /// assert_eq!(config.max_lifetime, Some(std::time::Duration::from_secs(24 * 60 * 60)));
    /// ```
    pub fn long_haul_profile() -> Self {
        Self {
            // Strict Max Size: Large transfers consume bandwidth.
            // We don't want 100 parallel 100GB transfers choking the pipe.
            max_size: 4,

            // Min Idle: Low. We assume these connections stay busy for hours.
            min_idle: 1,

            // Idle Timeout: Standard.
            idle_timeout: Some(Duration::from_secs(300)), // 5 minutes

            // Max Lifetime: EXTENDED.
            // A single S3 multipart upload session might last hours.
            // Don't kill it in the middle.
            max_lifetime: Some(Duration::from_secs(24 * 60 * 60)), // 24 Hours

            // Acquire Timeout: Long. Waiting for a slot on the "Heavy" lane is expected.
            acquire_timeout: Duration::from_secs(600), // 10 minutes
        }
    }
}

/// A connection wrapper that tracks metadata
#[derive(Debug)]
struct PooledConnection<T> {
    /// The actual connection
    conn: T,
    /// When this connection was created
    created_at: Instant,
    /// When this connection was last used
    last_used: Instant,
}

impl<T> PooledConnection<T> {
    fn new(conn: T) -> Self {
        let now = Instant::now();
        Self {
            conn,
            created_at: now,
            last_used: now,
        }
    }

    fn is_expired(&self, config: &PoolConfig) -> bool {
        // Check idle timeout
        if let Some(idle_timeout) = config.idle_timeout {
            if self.last_used.elapsed() > idle_timeout {
                return true;
            }
        }

        // Check max lifetime
        if let Some(max_lifetime) = config.max_lifetime {
            if self.created_at.elapsed() > max_lifetime {
                return true;
            }
        }

        false
    }

    fn update_last_used(&mut self) {
        self.last_used = Instant::now();
    }
}

/// Factory trait for creating and validating connections
#[async_trait::async_trait]
pub trait ConnectionFactory<T: Send + 'static>: Send + Sync {
    /// Create a new connection
    async fn create(&self) -> Result<T, ResilienceError>;

    /// Check if a connection is still healthy
    async fn is_healthy(&self, conn: &T) -> bool;

    /// Close a connection (optional cleanup)
    async fn close(&self, conn: T) {
        drop(conn);
    }
}

/// Internal pool state
struct PoolState<T> {
    /// Available connections
    idle: Vec<PooledConnection<T>>,
    /// Number of connections currently in use
    active_count: usize,
}

impl<T> PoolState<T> {
    fn new() -> Self {
        Self {
            idle: Vec::new(),
            active_count: 0,
        }
    }

    fn total_count(&self) -> usize {
        self.idle.len() + self.active_count
    }
}

/// A generic connection pool
///
/// # Example
/// ```no_run
/// use orbit_core_resilience::{ConnectionPool, PoolConfig, ConnectionFactory, ResilienceError};
/// use std::sync::Arc;
///
/// # #[derive(Clone)]
/// # struct MyConnection { id: usize }
/// struct MyConnectionFactory;
///
/// #[async_trait::async_trait]
/// impl ConnectionFactory<MyConnection> for MyConnectionFactory {
///     async fn create(&self) -> Result<MyConnection, ResilienceError> {
///         Ok(MyConnection { id: 1 })
///     }
///
///     async fn is_healthy(&self, _conn: &MyConnection) -> bool {
///         true
///     }
/// }
///
/// # async fn example() -> Result<(), ResilienceError> {
/// let factory = Arc::new(MyConnectionFactory);
/// let config = PoolConfig::default();
/// let pool = ConnectionPool::new(factory, config);
///
/// // Acquire a connection
/// let conn = pool.acquire().await?;
/// // Use connection...
///
/// // Return connection to pool
/// pool.release(conn).await;
/// # Ok(())
/// # }
/// ```
pub struct ConnectionPool<T> {
    config: Arc<PoolConfig>,
    factory: Arc<dyn ConnectionFactory<T>>,
    state: Arc<Mutex<PoolState<T>>>,
    semaphore: Arc<Semaphore>,
}

impl<T: Send + 'static> ConnectionPool<T> {
    /// Create a new connection pool
    pub fn new(factory: Arc<dyn ConnectionFactory<T>>, config: PoolConfig) -> Self {
        let max_size = config.max_size;
        Self {
            config: Arc::new(config),
            factory,
            state: Arc::new(Mutex::new(PoolState::new())),
            semaphore: Arc::new(Semaphore::new(max_size)),
        }
    }

    /// Create a connection pool with default configuration
    pub fn new_default(factory: Arc<dyn ConnectionFactory<T>>) -> Self {
        Self::new(factory, PoolConfig::default())
    }

    /// Acquire a connection from the pool
    ///
    /// If no idle connections are available and the pool is not at max capacity,
    /// a new connection will be created. If the pool is at max capacity, this will
    /// wait until a connection becomes available or timeout.
    pub async fn acquire(&self) -> Result<T, ResilienceError> {
        // Try to acquire permit with timeout
        let permit = tokio::time::timeout(self.config.acquire_timeout, self.semaphore.acquire())
            .await
            .map_err(|_| ResilienceError::Timeout(self.config.acquire_timeout))?
            .map_err(|_| ResilienceError::PoolExhausted)?;

        // Forget permit - we'll manage it manually
        permit.forget();

        // Try to get an idle connection or create a new one
        let mut state = self.state.lock().await;

        // Remove expired connections
        state.idle.retain(|conn| !conn.is_expired(&self.config));

        // Try to reuse an idle connection
        while let Some(mut pooled) = state.idle.pop() {
            // Check if connection is still healthy
            if self.factory.is_healthy(&pooled.conn).await {
                pooled.update_last_used();
                state.active_count += 1;
                drop(state); // Release lock before returning
                return Ok(pooled.conn);
            } else {
                // Connection is unhealthy, close it
                self.factory.close(pooled.conn).await;
            }
        }

        // No idle connections, create a new one
        state.active_count += 1;
        drop(state); // Release lock before creating connection

        match self.factory.create().await {
            Ok(conn) => Ok(conn),
            Err(e) => {
                // Failed to create connection, release permit
                self.semaphore.add_permits(1);
                let mut state = self.state.lock().await;
                state.active_count -= 1;
                Err(e)
            }
        }
    }

    /// Return a connection to the pool
    ///
    /// The connection will be checked for health before being returned to the idle pool.
    /// If unhealthy or the pool is over capacity, the connection will be closed.
    pub async fn release(&self, conn: T) {
        let mut state = self.state.lock().await;
        state.active_count -= 1;

        // Check if we should keep this connection
        let should_keep =
            state.total_count() <= self.config.max_size && self.factory.is_healthy(&conn).await;

        if should_keep {
            state.idle.push(PooledConnection::new(conn));
        } else {
            drop(state); // Release lock before closing
            self.factory.close(conn).await;
        }

        // Release permit
        self.semaphore.add_permits(1);
    }

    /// Get current pool statistics
    pub async fn stats(&self) -> PoolStats {
        let state = self.state.lock().await;
        PoolStats {
            idle: state.idle.len(),
            active: state.active_count,
            total: state.total_count(),
            max_size: self.config.max_size,
        }
    }

    /// Close all idle connections
    pub async fn clear_idle(&self) {
        let mut state = self.state.lock().await;
        let idle = std::mem::take(&mut state.idle);
        drop(state);

        for pooled in idle {
            self.factory.close(pooled.conn).await;
        }
    }

    /// Maintain minimum idle connections
    pub async fn maintain_idle(&self) -> Result<(), ResilienceError> {
        let state = self.state.lock().await;
        let current_idle = state.idle.len();
        let needed = self.config.min_idle.saturating_sub(current_idle);

        if needed == 0 || state.total_count() >= self.config.max_size {
            return Ok(());
        }

        let to_create = std::cmp::min(needed, self.config.max_size - state.total_count());
        drop(state);

        for _ in 0..to_create {
            match self.factory.create().await {
                Ok(conn) => {
                    let mut state = self.state.lock().await;
                    state.idle.push(PooledConnection::new(conn));
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of idle connections
    pub idle: usize,
    /// Number of active connections
    pub active: usize,
    /// Total connections (idle + active)
    pub total: usize,
    /// Maximum pool size
    pub max_size: usize,
}

impl PoolStats {
    /// Get pool utilization as a percentage
    pub fn utilization(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.total as f64 / self.max_size as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestConnection {
        id: usize,
        healthy: Arc<Mutex<bool>>,
    }

    struct TestFactory {
        counter: Arc<Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl ConnectionFactory<TestConnection> for TestFactory {
        async fn create(&self) -> Result<TestConnection, ResilienceError> {
            let mut counter = self.counter.lock().await;
            *counter += 1;
            Ok(TestConnection {
                id: *counter,
                healthy: Arc::new(Mutex::new(true)),
            })
        }

        async fn is_healthy(&self, conn: &TestConnection) -> bool {
            *conn.healthy.lock().await
        }
    }

    #[tokio::test]
    async fn test_pool_acquire_release() {
        let factory = Arc::new(TestFactory {
            counter: Arc::new(Mutex::new(0)),
        });
        let config = PoolConfig {
            max_size: 5,
            ..Default::default()
        };
        let pool = ConnectionPool::new(factory, config);

        // Acquire connection
        let conn1 = pool.acquire().await.unwrap();
        assert_eq!(conn1.id, 1);

        let stats = pool.stats().await;
        assert_eq!(stats.active, 1);
        assert_eq!(stats.idle, 0);

        // Release connection
        pool.release(conn1).await;

        let stats = pool.stats().await;
        assert_eq!(stats.active, 0);
        assert_eq!(stats.idle, 1);
    }

    #[tokio::test]
    async fn test_pool_reuse() {
        let factory = Arc::new(TestFactory {
            counter: Arc::new(Mutex::new(0)),
        });
        let pool = ConnectionPool::new_default(factory);

        // Acquire and release
        let conn1 = pool.acquire().await.unwrap();
        let id1 = conn1.id;
        pool.release(conn1).await;

        // Acquire again - should reuse same connection
        let conn2 = pool.acquire().await.unwrap();
        assert_eq!(conn2.id, id1);
    }

    #[tokio::test]
    async fn test_pool_max_size() {
        let factory = Arc::new(TestFactory {
            counter: Arc::new(Mutex::new(0)),
        });
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
        let _conn3 = pool.acquire().await.unwrap();

        pool.release(conn2).await;
    }
}
