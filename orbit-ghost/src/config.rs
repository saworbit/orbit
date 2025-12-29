use std::time::Duration;

#[derive(Debug, Clone)]
pub struct GhostConfig {
    /// Database query timeout (default: 5 seconds)
    pub db_timeout: Duration,

    /// Max retry attempts for transient errors (default: 3)
    pub max_retries: usize,

    /// Backoff multiplier for retries (default: 2x)
    pub backoff_multiplier: u32,

    /// Initial backoff delay (default: 100ms)
    pub initial_backoff: Duration,
}

impl Default for GhostConfig {
    fn default() -> Self {
        Self {
            db_timeout: Duration::from_secs(5),
            max_retries: 3,
            backoff_multiplier: 2,
            initial_backoff: Duration::from_millis(100),
        }
    }
}
