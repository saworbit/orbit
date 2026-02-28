//! Enhanced Error Recovery for S3 Operations
//!
//! This module provides sophisticated error recovery mechanisms including:
//! - Exponential backoff with jitter
//! - Circuit breaker pattern
//! - Retry policies with configurable strategies
//! - Automatic detection of retryable vs. fatal errors
//! - Metrics and observability
//!
//! # Overview
//!
//! Network operations are inherently unreliable. This module helps make S3
//! operations resilient by automatically retrying failed requests with
//! intelligent backoff strategies.
//!
//! # Features
//!
//! - **Exponential Backoff** - Progressively longer delays between retries
//! - **Jitter** - Randomized delays to prevent thundering herd
//! - **Circuit Breaker** - Stop retrying if service is consistently down
//! - **Configurable Policies** - Fine-tune retry behavior per operation
//! - **Error Classification** - Distinguish transient vs. permanent failures
//!
//! # Examples
//!
//! ## Basic Retry
//!
//! ```ignore
//! use orbit::protocol::s3::recovery::{RetryPolicy, with_retry};
//! use orbit::protocol::s3::S3Error;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let policy = RetryPolicy::default();
//!
//!     let result = with_retry(policy, || async {
//!         // Your S3 operation here
//!         Ok::<_, S3Error>(42)
//!     }).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Custom Policy
//!
//! ```no_run
//! # use orbit::protocol::s3::recovery::{RetryPolicy, BackoffStrategy};
//! use std::time::Duration;
//!
//! let policy = RetryPolicy {
//!     max_attempts: 5,
//!     initial_delay: Duration::from_millis(100),
//!     max_delay: Duration::from_secs(30),
//!     backoff: BackoffStrategy::ExponentialWithJitter,
//!     ..Default::default()
//! };
//! ```

use super::error::{S3Error, S3Result};
use rand::Rng;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial delay before first retry
    pub initial_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Backoff strategy to use
    pub backoff: BackoffStrategy,

    /// Jitter factor (0.0-1.0)
    pub jitter_factor: f64,

    /// Whether to use circuit breaker
    pub use_circuit_breaker: bool,

    /// Circuit breaker threshold (consecutive failures)
    pub circuit_breaker_threshold: u32,

    /// Circuit breaker timeout (how long to wait before half-open)
    pub circuit_breaker_timeout: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(60),
            backoff: BackoffStrategy::ExponentialWithJitter,
            jitter_factor: 0.3,
            use_circuit_breaker: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// Create a policy for fast retries (good for rate limiting)
    pub fn fast() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            backoff: BackoffStrategy::Linear,
            ..Default::default()
        }
    }

    /// Create a policy for slow/expensive operations
    pub fn slow() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(120),
            backoff: BackoffStrategy::Exponential,
            ..Default::default()
        }
    }

    /// Create a policy for network-flaky scenarios
    pub fn network() -> Self {
        Self {
            max_attempts: 10,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff: BackoffStrategy::ExponentialWithJitter,
            jitter_factor: 0.5,
            ..Default::default()
        }
    }

    /// Calculate delay for a given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = match self.backoff {
            BackoffStrategy::Linear => self.initial_delay * attempt,
            BackoffStrategy::Exponential | BackoffStrategy::ExponentialWithJitter => {
                let multiplier = 2_u32.pow(attempt - 1);
                self.initial_delay * multiplier
            }
            BackoffStrategy::Fixed => self.initial_delay,
        };

        // Cap at max_delay
        let capped_delay = base_delay.min(self.max_delay);

        // Add jitter if enabled
        if matches!(self.backoff, BackoffStrategy::ExponentialWithJitter) {
            let jitter = rand::rng().random_range(0.0..self.jitter_factor);
            let jitter_amount = capped_delay.as_secs_f64() * jitter;
            capped_delay + Duration::from_secs_f64(jitter_amount)
        } else {
            capped_delay
        }
    }
}

/// Backoff strategy for retries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,

    /// Linear increase in delay
    Linear,

    /// Exponential increase in delay (2^n)
    Exponential,

    /// Exponential with random jitter to prevent thundering herd
    ExponentialWithJitter,
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,

    /// Circuit is open, requests fail fast
    Open,

    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    last_failure: Arc<RwLock<Option<Instant>>>,
    threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            last_failure: Arc::new(RwLock::new(None)),
            threshold,
            timeout,
        }
    }

    /// Check if request should be allowed
    pub async fn allow_request(&self) -> bool {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout elapsed
                if let Some(last) = *self.last_failure.read().await {
                    if last.elapsed() >= self.timeout {
                        // Transition to half-open
                        *self.state.write().await = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful request
    pub async fn record_success(&self) {
        *self.failure_count.write().await = 0;
        *self.state.write().await = CircuitState::Closed;
    }

    /// Record a failed request
    pub async fn record_failure(&self) {
        let mut count = self.failure_count.write().await;
        *count += 1;
        *self.last_failure.write().await = Some(Instant::now());

        if *count >= self.threshold {
            *self.state.write().await = CircuitState::Open;
        }
    }

    /// Get current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }
}

/// Classify errors as retryable or fatal
pub fn is_retryable_error(error: &S3Error) -> bool {
    match error {
        // Network/transient errors - retryable
        S3Error::Sdk(e) => {
            let err_str = e.to_lowercase();
            err_str.contains("timeout")
                || err_str.contains("connection")
                || err_str.contains("throttl")
                || err_str.contains("slow down")
                || err_str.contains("503")
                || err_str.contains("500")
        }

        // IO errors - potentially retryable
        S3Error::Io(_) => true,

        // These are typically not retryable
        S3Error::InvalidConfig(_) => false,
        S3Error::NotFound { .. } => false,
        S3Error::AccessDenied(_) => false,
        S3Error::InvalidKey(_) => false,
        S3Error::BucketNotFound(_) => false,

        // Service errors - check if retryable
        S3Error::Service { code, .. } => {
            matches!(
                code.as_str(),
                "RequestTimeout"
                    | "ServiceUnavailable"
                    | "InternalError"
                    | "SlowDown"
                    | "RequestTimeTooSkewed"
            )
        }

        // Network, timeout, rate limit - retryable
        S3Error::Network(_) | S3Error::Timeout(_) | S3Error::RateLimitExceeded(_) => true,

        // Other errors - generally not retryable
        _ => false,
    }
}

/// Execute an operation with retry logic
pub async fn with_retry<F, Fut, T>(policy: RetryPolicy, mut operation: F) -> S3Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = S3Result<T>>,
{
    let circuit_breaker = if policy.use_circuit_breaker {
        Some(CircuitBreaker::new(
            policy.circuit_breaker_threshold,
            policy.circuit_breaker_timeout,
        ))
    } else {
        None
    };

    let mut attempt = 0;

    loop {
        attempt += 1;

        // Check circuit breaker
        if let Some(cb) = &circuit_breaker {
            if !cb.allow_request().await {
                return Err(S3Error::Service {
                    code: "CircuitBreakerOpen".to_string(),
                    message: "Circuit breaker open - service unavailable".to_string(),
                });
            }
        }

        // Attempt the operation
        match operation().await {
            Ok(result) => {
                // Record success
                if let Some(cb) = &circuit_breaker {
                    cb.record_success().await;
                }
                return Ok(result);
            }
            Err(e) => {
                // Record failure
                if let Some(cb) = &circuit_breaker {
                    cb.record_failure().await;
                }

                // Check if we should retry
                if attempt >= policy.max_attempts {
                    return Err(e);
                }

                if !is_retryable_error(&e) {
                    return Err(e);
                }

                // Calculate and apply delay
                let delay = policy.calculate_delay(attempt);
                sleep(delay).await;
            }
        }
    }
}

/// Retry context with metrics
pub struct RetryContext {
    /// Total attempts made
    pub attempts: u32,

    /// Total delay accumulated
    pub total_delay: Duration,

    /// Whether operation succeeded
    pub succeeded: bool,

    /// Final error if failed
    pub error: Option<S3Error>,

    /// Start time
    start_time: Instant,
}

impl RetryContext {
    /// Create a new retry context
    pub fn new() -> Self {
        Self {
            attempts: 0,
            total_delay: Duration::ZERO,
            succeeded: false,
            error: None,
            start_time: Instant::now(),
        }
    }

    /// Record an attempt
    pub fn record_attempt(&mut self, delay: Duration) {
        self.attempts += 1;
        self.total_delay += delay;
    }

    /// Record success
    pub fn record_success(&mut self) {
        self.succeeded = true;
    }

    /// Record failure
    pub fn record_failure(&mut self, error: S3Error) {
        self.error = Some(error);
    }

    /// Get total elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get average delay per attempt
    pub fn average_delay(&self) -> Duration {
        if self.attempts == 0 {
            Duration::ZERO
        } else {
            self.total_delay / self.attempts
        }
    }
}

impl Default for RetryContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_delay_calculation() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_millis(100),
            backoff: BackoffStrategy::Exponential,
            max_delay: Duration::from_secs(10),
            jitter_factor: 0.0,
            ..Default::default()
        };

        assert_eq!(policy.calculate_delay(1), Duration::from_millis(100)); // 100 * 2^0
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(200)); // 100 * 2^1
        assert_eq!(policy.calculate_delay(3), Duration::from_millis(400)); // 100 * 2^2
    }

    #[test]
    fn test_retry_policy_max_delay_cap() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_secs(1),
            backoff: BackoffStrategy::Exponential,
            max_delay: Duration::from_secs(5),
            jitter_factor: 0.0,
            ..Default::default()
        };

        // 1 * 2^9 = 512 seconds, but should be capped at 5
        let delay = policy.calculate_delay(10);
        assert!(delay <= Duration::from_secs(5));
    }

    #[test]
    fn test_linear_backoff() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_millis(100),
            backoff: BackoffStrategy::Linear,
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.0,
            ..Default::default()
        };

        assert_eq!(policy.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(policy.calculate_delay(3), Duration::from_millis(300));
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1));

        // Should allow requests initially
        assert!(cb.allow_request().await);
        assert_eq!(cb.state().await, CircuitState::Closed);

        // Record failures
        for _ in 0..3 {
            cb.record_failure().await;
        }

        // Circuit should be open
        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(!cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(50));

        // Trigger circuit breaker
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for timeout
        sleep(Duration::from_millis(60)).await;

        // Should transition to half-open
        assert!(cb.allow_request().await);
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Success should close it
        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_retry_with_success() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let policy = RetryPolicy::default();
        let attempts = AtomicU32::new(0);

        let result = with_retry(policy, || {
            let current = attempts.fetch_add(1, Ordering::SeqCst) + 1;
            async move {
                if current < 3 {
                    Err(S3Error::Network("Transient error".to_string()))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_retry_context() {
        let mut ctx = RetryContext::new();
        assert_eq!(ctx.attempts, 0);

        ctx.record_attempt(Duration::from_millis(100));
        ctx.record_attempt(Duration::from_millis(200));

        assert_eq!(ctx.attempts, 2);
        assert_eq!(ctx.total_delay, Duration::from_millis(300));
        assert_eq!(ctx.average_delay(), Duration::from_millis(150));
    }

    #[test]
    fn test_retry_policy_network_defaults() {
        let policy = RetryPolicy::network();
        assert_eq!(policy.max_attempts, 10);
        assert_eq!(policy.initial_delay, Duration::from_millis(100));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert_eq!(policy.backoff, BackoffStrategy::ExponentialWithJitter);
        assert_eq!(policy.jitter_factor, 0.5);
    }

    #[test]
    fn test_retry_policy_fast_defaults() {
        let policy = RetryPolicy::fast();
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(50));
        assert_eq!(policy.max_delay, Duration::from_secs(5));
        assert_eq!(policy.backoff, BackoffStrategy::Linear);
    }

    #[test]
    fn test_retry_policy_slow_defaults() {
        let policy = RetryPolicy::slow();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(120));
        assert_eq!(policy.backoff, BackoffStrategy::Exponential);
    }

    #[test]
    fn test_retry_policy_default_values() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_delay, Duration::from_millis(200));
        assert_eq!(policy.max_delay, Duration::from_secs(60));
        assert_eq!(policy.backoff, BackoffStrategy::ExponentialWithJitter);
    }

    #[test]
    fn test_fixed_backoff() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_millis(500),
            backoff: BackoffStrategy::Fixed,
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.0,
            ..Default::default()
        };
        assert_eq!(policy.calculate_delay(1), Duration::from_millis(500));
        assert_eq!(policy.calculate_delay(5), Duration::from_millis(500));
        assert_eq!(policy.calculate_delay(10), Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_with_retry_non_retryable_fails_immediately() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let policy = RetryPolicy {
            max_attempts: 5,
            use_circuit_breaker: false,
            ..Default::default()
        };
        let attempts = AtomicU32::new(0);

        let result = with_retry(policy, || {
            attempts.fetch_add(1, Ordering::SeqCst);
            async { Err::<(), _>(S3Error::InvalidKey("bad".to_string())) }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_exhausts_attempts() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            use_circuit_breaker: false,
            ..Default::default()
        };
        let attempts = AtomicU32::new(0);

        let result = with_retry(policy, || {
            attempts.fetch_add(1, Ordering::SeqCst);
            async { Err::<(), _>(S3Error::Network("transient".to_string())) }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_is_retryable_error_function() {
        // Retryable errors
        assert!(is_retryable_error(&S3Error::Network("net".to_string())));
        assert!(is_retryable_error(&S3Error::Timeout("timeout".to_string())));
        assert!(is_retryable_error(&S3Error::RateLimitExceeded(
            "rate".to_string()
        )));
        assert!(is_retryable_error(&S3Error::Io("io".to_string())));

        // Non-retryable errors
        assert!(!is_retryable_error(&S3Error::InvalidConfig(
            "bad".to_string()
        )));
        assert!(!is_retryable_error(&S3Error::NotFound {
            bucket: "b".to_string(),
            key: "k".to_string(),
        }));
        assert!(!is_retryable_error(&S3Error::AccessDenied(
            "denied".to_string()
        )));
        assert!(!is_retryable_error(&S3Error::InvalidKey(
            "bad".to_string()
        )));
        assert!(!is_retryable_error(&S3Error::BucketNotFound(
            "gone".to_string()
        )));

        // Service with retryable code
        assert!(is_retryable_error(&S3Error::Service {
            code: "RequestTimeout".to_string(),
            message: "timeout".to_string(),
        }));

        // Service with non-retryable code
        assert!(!is_retryable_error(&S3Error::Service {
            code: "AccessDenied".to_string(),
            message: "denied".to_string(),
        }));

        // Sdk with retryable keyword
        assert!(is_retryable_error(&S3Error::Sdk("timeout".to_string())));

        // Sdk without retryable keyword
        assert!(!is_retryable_error(&S3Error::Sdk("invalid".to_string())));
    }

    #[test]
    fn test_retry_context_default() {
        let ctx = RetryContext::default();
        assert_eq!(ctx.attempts, 0);
        assert_eq!(ctx.total_delay, Duration::ZERO);
        assert!(!ctx.succeeded);
        assert!(ctx.error.is_none());
        assert_eq!(ctx.average_delay(), Duration::ZERO);
    }
}
