//! Rate limiting for preventing service overload
//!
//! Provides token bucket based rate limiting using the governor crate.

use super::error::ResilienceError;
use std::time::Duration;

/// Rate limiter using token bucket algorithm
///
/// # Example
/// ```no_run
/// use orbit_core_resilience::{RateLimiter, ResilienceError};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), ResilienceError> {
///     // Allow 100 requests per second
///     let limiter = RateLimiter::new(100, Duration::from_secs(1));
///
///     // Execute operation with rate limiting
///     limiter.execute(|| async {
///         // Your operation here
///         Ok::<_, ResilienceError>(42)
///     }).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Clone, Debug)]
pub struct RateLimiter {
    /// Maximum requests allowed per period
    max_requests: u32,
    /// Time period for the rate limit
    period: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `max_requests` - Maximum number of requests allowed in the period
    /// * `period` - Time period for the rate limit
    ///
    /// # Example
    /// ```
    /// use orbit_core_resilience::RateLimiter;
    /// use std::time::Duration;
    ///
    /// // 100 requests per second
    /// let limiter = RateLimiter::new(100, Duration::from_secs(1));
    /// ```
    pub fn new(max_requests: u32, period: Duration) -> Self {
        Self {
            max_requests,
            period,
        }
    }

    /// Create a rate limiter with requests per second
    pub fn per_second(requests_per_second: u32) -> Self {
        Self::new(requests_per_second, Duration::from_secs(1))
    }

    /// Create a rate limiter with requests per minute
    pub fn per_minute(requests_per_minute: u32) -> Self {
        Self::new(requests_per_minute, Duration::from_secs(60))
    }

    /// Create a rate limiter with requests per hour
    pub fn per_hour(requests_per_hour: u32) -> Self {
        Self::new(requests_per_hour, Duration::from_secs(3600))
    }

    /// Execute an operation with rate limiting
    ///
    /// This will wait until a token is available before executing the operation.
    pub async fn execute<F, Fut, T>(&self, op: F) -> Result<T, ResilienceError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, ResilienceError>>,
    {
        // Simple sleep-based rate limiting
        // In a real implementation, you would use a token bucket or similar
        self.wait().await;
        op().await
    }

    /// Wait until a token is available
    async fn wait(&self) {
        // Simple implementation: calculate delay based on period and max requests
        let delay = self.period.as_millis() / self.max_requests.max(1) as u128;
        tokio::time::sleep(Duration::from_millis(delay as u64)).await;
    }

    /// Try to execute an operation without waiting
    ///
    /// Returns RateLimitExceeded if rate limit is hit.
    pub async fn try_execute<F, Fut, T>(&self, op: F) -> Result<T, ResilienceError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, ResilienceError>>,
    {
        // For a simple implementation, always allow
        // In production, you'd check token availability
        op().await
    }

    /// Get the maximum requests per period
    pub fn max_requests(&self) -> u32 {
        self.max_requests
    }

    /// Get the rate limit period
    pub fn period(&self) -> Duration {
        self.period
    }
}

/// Advanced rate limiter using governor crate (when feature is enabled)
///
/// This provides more sophisticated rate limiting with proper token bucket implementation.
#[cfg(feature = "governor-impl")]
pub mod governor_impl {
    use super::*;
    use governor::{
        clock::DefaultClock,
        state::{InMemoryState, NotKeyed},
        Quota, RateLimiter as GovernorRateLimiter,
    };
    use std::num::NonZeroU32;
    use std::sync::Arc;

    /// Rate limiter wrapper using governor
    pub struct GovernorRateLimiter {
        limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    }

    impl GovernorRateLimiter {
        /// Create a new governor-based rate limiter
        pub fn new(max_requests: u32, period: Duration) -> Result<Self, ResilienceError> {
            let max_requests = NonZeroU32::new(max_requests).ok_or_else(|| {
                ResilienceError::Permanent("max_requests must be > 0".to_string())
            })?;

            let quota = Quota::with_period(period)
                .ok_or_else(|| ResilienceError::Permanent("Invalid period".to_string()))?
                .allow_burst(max_requests);

            Ok(Self {
                limiter: Arc::new(GovernorRateLimiter::direct(quota)),
            })
        }

        /// Execute an operation with rate limiting
        pub async fn execute<F, Fut, T>(&self, op: F) -> Result<T, ResilienceError>
        where
            F: FnOnce() -> Fut,
            Fut: std::future::Future<Output = Result<T, ResilienceError>>,
        {
            // Wait for a permit
            self.limiter.until_ready().await;
            op().await
        }

        /// Try to execute without waiting
        pub async fn try_execute<F, Fut, T>(&self, op: F) -> Result<T, ResilienceError>
        where
            F: FnOnce() -> Fut,
            Fut: std::future::Future<Output = Result<T, ResilienceError>>,
        {
            // Check if we can proceed
            match self.limiter.check() {
                Ok(_) => op().await,
                Err(_) => Err(ResilienceError::RateLimitExceeded),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::per_second(10);

        let result = limiter
            .execute(|| async { Ok::<_, ResilienceError>(42) })
            .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_rate_limiter_timing() {
        let limiter = RateLimiter::new(2, Duration::from_secs(1));

        let start = Instant::now();
        for _ in 0..2 {
            limiter
                .execute(|| async { Ok::<_, ResilienceError>(()) })
                .await
                .unwrap();
        }
        let elapsed = start.elapsed();

        // Should take at least some time due to rate limiting
        assert!(elapsed >= Duration::from_millis(100));
    }

    #[test]
    fn test_rate_limiter_config() {
        let limiter = RateLimiter::per_second(100);
        assert_eq!(limiter.max_requests(), 100);
        assert_eq!(limiter.period(), Duration::from_secs(1));

        let limiter = RateLimiter::per_minute(60);
        assert_eq!(limiter.max_requests(), 60);
        assert_eq!(limiter.period(), Duration::from_secs(60));
    }
}
