//! Error types for the resilience module

use thiserror::Error;

/// Errors that can occur in resilience operations
#[derive(Debug, Error, Clone)]
pub enum ResilienceError {
    /// Circuit breaker is open, rejecting requests
    #[error("Circuit breaker is open, rejecting requests")]
    CircuitOpen,

    /// Connection pool is exhausted
    #[error("Connection pool is exhausted, no available connections")]
    PoolExhausted,

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after cooldown")]
    RateLimitExceeded,

    /// Transient error that may be retried
    #[error("Transient error: {0}")]
    Transient(String),

    /// Permanent error that should not be retried
    #[error("Permanent error: {0}")]
    Permanent(String),

    /// Connection creation failed
    #[error("Failed to create connection: {0}")]
    ConnectionCreation(String),

    /// Connection is unhealthy
    #[error("Connection is unhealthy: {0}")]
    UnhealthyConnection(String),

    /// Timeout occurred
    #[error("Operation timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Maximum retries exceeded
    #[error("Maximum retries ({0}) exceeded")]
    MaxRetriesExceeded(usize),
}

impl ResilienceError {
    /// Check if this error is transient and can be retried
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            ResilienceError::Transient(_)
                | ResilienceError::RateLimitExceeded
                | ResilienceError::Timeout(_)
        )
    }

    /// Check if this error is permanent and should not be retried
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            ResilienceError::Permanent(_) | ResilienceError::CircuitOpen
        )
    }

    /// Check if this error should contribute to circuit breaker failure count
    pub fn should_trip_breaker(&self) -> bool {
        !matches!(
            self,
            ResilienceError::CircuitOpen | ResilienceError::PoolExhausted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let transient = ResilienceError::Transient("network error".to_string());
        assert!(transient.is_transient());
        assert!(!transient.is_permanent());
        assert!(transient.should_trip_breaker());

        let permanent = ResilienceError::Permanent("auth failed".to_string());
        assert!(!permanent.is_transient());
        assert!(permanent.is_permanent());
        assert!(permanent.should_trip_breaker());

        let circuit_open = ResilienceError::CircuitOpen;
        assert!(!circuit_open.is_transient());
        assert!(circuit_open.is_permanent());
        assert!(!circuit_open.should_trip_breaker());
    }
}
