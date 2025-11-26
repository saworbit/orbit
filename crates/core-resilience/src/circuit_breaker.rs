//! Circuit Breaker implementation for fault tolerance
//!
//! The circuit breaker prevents cascading failures by failing fast when a service
//! is experiencing issues. It has three states:
//! - Closed: Normal operation, requests pass through
//! - Open: Service is unhealthy, requests fail immediately
//! - HalfOpen: Testing if service has recovered

use super::error::ResilienceError;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// State of the circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests pass through normally
    Closed,
    /// Circuit is open, requests fail immediately
    /// Next probe time indicates when to try half-open
    Open { next_probe: Instant },
    /// Circuit is half-open, testing service recovery
    HalfOpen,
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: usize,
    /// Number of consecutive successes in half-open to close circuit
    pub success_threshold: usize,
    /// Duration to wait before transitioning from open to half-open
    pub cooldown: Duration,
    /// Initial backoff delay for retries
    pub initial_backoff: Duration,
    /// Maximum backoff delay for retries
    pub max_backoff: Duration,
    /// Backoff multiplier (exponential backoff)
    pub backoff_multiplier: f64,
    /// Maximum number of retry attempts
    pub max_retries: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            cooldown: Duration::from_secs(60),
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            max_retries: 3,
        }
    }
}

/// Internal state of the circuit breaker
#[derive(Debug)]
struct CircuitBreakerState {
    /// Current state of the circuit
    state: CircuitState,
    /// Consecutive failure count
    consecutive_failures: usize,
    /// Consecutive success count (used in half-open state)
    consecutive_successes: usize,
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
        }
    }
}

/// Circuit breaker for protecting against cascading failures
///
/// # Example
/// ```no_run
/// use orbit_core_resilience::{CircuitBreaker, CircuitBreakerConfig, ResilienceError};
///
/// #[tokio::main]
/// async fn main() -> Result<(), ResilienceError> {
///     let config = CircuitBreakerConfig::default();
///     let breaker = CircuitBreaker::new(config);
///
///     // Execute operation with circuit breaker protection
///     let result = breaker.execute(|| async {
///         // Your operation here
///         Ok::<_, ResilienceError>(42)
///     }).await?;
///
///     println!("Result: {}", result);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    config: Arc<CircuitBreakerConfig>,
    state: Arc<Mutex<CircuitBreakerState>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config: Arc::new(config),
            state: Arc::new(Mutex::new(CircuitBreakerState::new())),
        }
    }

    /// Create a new circuit breaker with default configuration
    pub fn new_default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Get the current state of the circuit breaker
    pub async fn get_state(&self) -> CircuitState {
        let state = self.state.lock().await;
        state.state
    }

    /// Get current failure count
    pub async fn get_failure_count(&self) -> usize {
        let state = self.state.lock().await;
        state.consecutive_failures
    }

    /// Get current success count
    pub async fn get_success_count(&self) -> usize {
        let state = self.state.lock().await;
        state.consecutive_successes
    }

    /// Reset the circuit breaker to closed state
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.state = CircuitState::Closed;
        state.consecutive_failures = 0;
        state.consecutive_successes = 0;
    }

    /// Execute an operation with circuit breaker protection and retry logic
    ///
    /// The operation will be retried with exponential backoff on transient failures.
    /// If the circuit is open, the operation fails immediately.
    pub async fn execute<F, Fut, T>(&self, op: F) -> Result<T, ResilienceError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, ResilienceError>>,
    {
        // Check circuit state and potentially transition to half-open
        self.check_and_update_state().await?;

        // Execute with retries and exponential backoff
        let mut attempt = 0;
        let mut backoff = self.config.initial_backoff;

        loop {
            attempt += 1;

            // Execute the operation
            match op().await {
                Ok(result) => {
                    self.on_success().await;
                    return Ok(result);
                }
                Err(e) if e.is_transient() && attempt <= self.config.max_retries => {
                    self.on_failure(&e).await;

                    // Sleep with exponential backoff before retrying
                    tokio::time::sleep(backoff).await;
                    backoff = std::cmp::min(
                        Duration::from_secs_f64(
                            backoff.as_secs_f64() * self.config.backoff_multiplier,
                        ),
                        self.config.max_backoff,
                    );
                }
                Err(e) => {
                    // Permanent error or max retries exceeded
                    if !e.is_permanent() {
                        self.on_failure(&e).await;
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Execute an operation without retry logic
    ///
    /// Useful when you want circuit breaker protection but not automatic retries.
    pub async fn call<F, Fut, T>(&self, op: F) -> Result<T, ResilienceError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, ResilienceError>>,
    {
        // Check circuit state
        self.check_and_update_state().await?;

        // Execute the operation
        match op().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                if e.should_trip_breaker() {
                    self.on_failure(&e).await;
                }
                Err(e)
            }
        }
    }

    /// Check circuit state and update if necessary
    async fn check_and_update_state(&self) -> Result<(), ResilienceError> {
        let mut state = self.state.lock().await;

        match state.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open { next_probe } => {
                if Instant::now() >= next_probe {
                    // Transition to half-open for testing
                    state.state = CircuitState::HalfOpen;
                    state.consecutive_successes = 0;
                    Ok(())
                } else {
                    Err(ResilienceError::CircuitOpen)
                }
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    /// Handle successful operation
    async fn on_success(&self) {
        let mut state = self.state.lock().await;

        match state.state {
            CircuitState::Closed => {
                // Reset failure count on success
                state.consecutive_failures = 0;
            }
            CircuitState::HalfOpen => {
                // Increment success count
                state.consecutive_successes += 1;

                // If enough successes, close the circuit
                if state.consecutive_successes >= self.config.success_threshold {
                    state.state = CircuitState::Closed;
                    state.consecutive_failures = 0;
                    state.consecutive_successes = 0;
                }
            }
            CircuitState::Open { .. } => {
                // Should not happen, but reset to closed if it does
                state.state = CircuitState::Closed;
                state.consecutive_failures = 0;
                state.consecutive_successes = 0;
            }
        }
    }

    /// Handle failed operation
    async fn on_failure(&self, _error: &ResilienceError) {
        let mut state = self.state.lock().await;

        match state.state {
            CircuitState::Closed => {
                state.consecutive_failures += 1;

                // If threshold exceeded, open the circuit
                if state.consecutive_failures >= self.config.failure_threshold {
                    state.state = CircuitState::Open {
                        next_probe: Instant::now() + self.config.cooldown,
                    };
                    state.consecutive_failures = 0;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open state reopens the circuit
                state.state = CircuitState::Open {
                    next_probe: Instant::now() + self.config.cooldown,
                };
                state.consecutive_successes = 0;
            }
            CircuitState::Open { .. } => {
                // Already open, nothing to do
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            cooldown: Duration::from_millis(100),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Simulate failures
        for _ in 0..3 {
            let result: Result<(), ResilienceError> = breaker
                .call(|| async { Err(ResilienceError::Transient("test error".to_string())) })
                .await;
            assert!(result.is_err());
        }

        // Circuit should be open
        match breaker.get_state().await {
            CircuitState::Open { .. } => (), // Success
            state => panic!("Expected Open state, got {:?}", state),
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            cooldown: Duration::from_millis(50),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Force circuit to open
        for _ in 0..2 {
            let _: Result<(), ResilienceError> = breaker
                .call(|| async { Err(ResilienceError::Transient("test".to_string())) })
                .await;
        }

        // Wait for cooldown
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Successful calls should close the circuit
        for _ in 0..2 {
            let result = breaker
                .call(|| async { Ok::<_, ResilienceError>(()) })
                .await;
            assert!(result.is_ok());
        }

        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_exponential_backoff() {
        let config = CircuitBreakerConfig {
            initial_backoff: Duration::from_millis(10),
            max_retries: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        let start = Instant::now();
        let result: Result<(), ResilienceError> = breaker
            .execute(|| async { Err(ResilienceError::Transient("test".to_string())) })
            .await;

        assert!(result.is_err());
        // Should have waited at least initial_backoff * (1 + multiplier)
        assert!(start.elapsed() >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Cause failure
        let _: Result<(), ResilienceError> = breaker
            .call(|| async { Err(ResilienceError::Transient("test".to_string())) })
            .await;

        // Reset
        breaker.reset().await;

        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert_eq!(breaker.get_failure_count().await, 0);
    }
}
