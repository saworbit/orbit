//! Sentinel Policy Engine
//!
//! Defines the rules and thresholds for the Sentinel's resilience operations.

use serde::{Deserialize, Serialize};

/// Sentinel operational policy
///
/// This configures the Sentinel's behavior for redundancy monitoring
/// and automatic healing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelPolicy {
    /// Minimum number of chunk copies required across the Grid
    ///
    /// If a chunk has fewer than this many copies, the Sentinel will
    /// trigger a healing operation to restore redundancy.
    ///
    /// **Default:** 2 (dual redundancy)
    pub min_redundancy: u8,

    /// Maximum number of concurrent healing operations
    ///
    /// This prevents the Sentinel from overwhelming the network with
    /// simultaneous replication requests. Uses a semaphore to enforce.
    ///
    /// **Default:** 10
    pub max_parallel_heals: usize,

    /// Scan interval in seconds
    ///
    /// How frequently the Sentinel performs a full Universe sweep.
    ///
    /// **Default:** 3600 (1 hour)
    pub scan_interval_s: u64,

    /// Optional bandwidth limit for healing operations (bytes/sec)
    ///
    /// Set to `None` for unlimited bandwidth.
    ///
    /// **Default:** 50 MB/s
    pub healing_bandwidth_limit: Option<u64>,
}

impl Default for SentinelPolicy {
    fn default() -> Self {
        Self {
            min_redundancy: 2,
            max_parallel_heals: 10,
            scan_interval_s: 3600,
            healing_bandwidth_limit: Some(50 * 1024 * 1024), // 50 MB/s
        }
    }
}

impl SentinelPolicy {
    /// Create a new policy with custom redundancy requirement
    ///
    /// Other parameters will use defaults.
    pub fn with_redundancy(min_redundancy: u8) -> Self {
        Self {
            min_redundancy,
            ..Default::default()
        }
    }

    /// Validate the policy configuration
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.min_redundancy == 0 {
            return Err("min_redundancy must be at least 1".to_string());
        }

        if self.max_parallel_heals == 0 {
            return Err("max_parallel_heals must be at least 1".to_string());
        }

        if self.scan_interval_s == 0 {
            return Err("scan_interval_s must be greater than 0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let policy = SentinelPolicy::default();

        assert_eq!(policy.min_redundancy, 2);
        assert_eq!(policy.max_parallel_heals, 10);
        assert_eq!(policy.scan_interval_s, 3600);
        assert_eq!(policy.healing_bandwidth_limit, Some(50 * 1024 * 1024));

        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_with_redundancy() {
        let policy = SentinelPolicy::with_redundancy(3);

        assert_eq!(policy.min_redundancy, 3);
        assert_eq!(policy.max_parallel_heals, 10); // Default
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_validation_failures() {
        let mut policy = SentinelPolicy::default();

        // Test min_redundancy = 0
        policy.min_redundancy = 0;
        assert!(policy.validate().is_err());
        policy.min_redundancy = 2; // Reset

        // Test max_parallel_heals = 0
        policy.max_parallel_heals = 0;
        assert!(policy.validate().is_err());
        policy.max_parallel_heals = 10; // Reset

        // Test scan_interval_s = 0
        policy.scan_interval_s = 0;
        assert!(policy.validate().is_err());
    }
}
