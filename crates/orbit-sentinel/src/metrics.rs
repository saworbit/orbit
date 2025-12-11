//! Sentinel Metrics and Telemetry
//!
//! Tracks statistics for Sentinel sweep operations and healing activities.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Statistics from a single Universe sweep
///
/// Provides visibility into the health status of the Grid after
/// each Sentinel scan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SweepStats {
    /// Number of chunks with sufficient redundancy (healthy)
    pub healthy: usize,

    /// Number of chunks below minimum redundancy (at risk)
    pub at_risk: usize,

    /// Number of chunks with zero copies (data loss)
    pub lost: usize,

    /// Number of healing operations attempted during this sweep
    pub heals_attempted: usize,

    /// Number of healing operations that completed successfully
    pub heals_succeeded: usize,

    /// Number of healing operations that failed
    pub heals_failed: usize,

    /// Time taken to complete the sweep
    pub duration: Option<Duration>,
}

impl SweepStats {
    /// Create a new empty stats object
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate the health ratio (0.0 - 1.0)
    ///
    /// Returns the percentage of healthy chunks out of total chunks.
    pub fn health_ratio(&self) -> f64 {
        let total = self.total_chunks();
        if total == 0 {
            1.0 // No chunks is technically healthy
        } else {
            self.healthy as f64 / total as f64
        }
    }

    /// Get total number of chunks scanned
    pub fn total_chunks(&self) -> usize {
        self.healthy + self.at_risk + self.lost
    }

    /// Calculate healing success rate (0.0 - 1.0)
    pub fn healing_success_rate(&self) -> f64 {
        if self.heals_attempted == 0 {
            1.0 // No heals attempted means 100% success (vacuous truth)
        } else {
            self.heals_succeeded as f64 / self.heals_attempted as f64
        }
    }

    /// Format a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Sweep: {} total | {} healthy ({:.1}%) | {} at-risk | {} lost | Heals: {}/{} ({:.1}%)",
            self.total_chunks(),
            self.healthy,
            self.health_ratio() * 100.0,
            self.at_risk,
            self.lost,
            self.heals_succeeded,
            self.heals_attempted,
            self.healing_success_rate() * 100.0
        )
    }
}

/// Builder for tracking sweep progress
///
/// This is used internally by the daemon to accumulate stats during a sweep.
#[derive(Debug, Clone)]
pub struct SweepStatsBuilder {
    stats: SweepStats,
    start_time: Instant,
}

impl SweepStatsBuilder {
    /// Start tracking a new sweep
    pub fn new() -> Self {
        Self {
            stats: SweepStats::new(),
            start_time: Instant::now(),
        }
    }

    /// Record a healthy chunk
    pub fn record_healthy(&mut self) {
        self.stats.healthy += 1;
    }

    /// Record an at-risk chunk
    pub fn record_at_risk(&mut self) {
        self.stats.at_risk += 1;
    }

    /// Record a lost chunk (data loss)
    pub fn record_lost(&mut self) {
        self.stats.lost += 1;
    }

    /// Record a healing attempt
    pub fn record_heal_attempt(&mut self) {
        self.stats.heals_attempted += 1;
    }

    /// Record a successful heal
    pub fn record_heal_success(&mut self) {
        self.stats.heals_succeeded += 1;
    }

    /// Record a failed heal
    pub fn record_heal_failure(&mut self) {
        self.stats.heals_failed += 1;
    }

    /// Finalize and return the stats
    pub fn finish(mut self) -> SweepStats {
        self.stats.duration = Some(self.start_time.elapsed());
        self.stats
    }

    /// Get a reference to the current stats (without finishing)
    pub fn stats(&self) -> &SweepStats {
        &self.stats
    }
}

impl Default for SweepStatsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_ratio() {
        let mut stats = SweepStats::new();

        // All healthy
        stats.healthy = 100;
        assert_eq!(stats.health_ratio(), 1.0);

        // 80% healthy
        stats.at_risk = 20;
        assert_eq!(stats.health_ratio(), 100.0 / 120.0);

        // With data loss
        stats.lost = 10;
        assert_eq!(stats.health_ratio(), 100.0 / 130.0);
    }

    #[test]
    fn test_healing_success_rate() {
        let mut stats = SweepStats::new();

        // No heals attempted
        assert_eq!(stats.healing_success_rate(), 1.0);

        // 100% success
        stats.heals_attempted = 10;
        stats.heals_succeeded = 10;
        assert_eq!(stats.healing_success_rate(), 1.0);

        // 50% success
        stats.heals_succeeded = 5;
        stats.heals_failed = 5;
        assert_eq!(stats.healing_success_rate(), 0.5);
    }

    #[test]
    fn test_stats_builder() {
        let mut builder = SweepStatsBuilder::new();

        builder.record_healthy();
        builder.record_healthy();
        builder.record_at_risk();
        builder.record_heal_attempt();
        builder.record_heal_success();

        let stats = builder.finish();

        assert_eq!(stats.healthy, 2);
        assert_eq!(stats.at_risk, 1);
        assert_eq!(stats.lost, 0);
        assert_eq!(stats.heals_attempted, 1);
        assert_eq!(stats.heals_succeeded, 1);
        assert!(stats.duration.is_some());
    }

    #[test]
    fn test_summary() {
        let mut stats = SweepStats::new();
        stats.healthy = 95;
        stats.at_risk = 4;
        stats.lost = 1;
        stats.heals_attempted = 4;
        stats.heals_succeeded = 3;
        stats.heals_failed = 1;

        let summary = stats.summary();
        assert!(summary.contains("100 total"));
        assert!(summary.contains("95 healthy"));
        assert!(summary.contains("4 at-risk"));
        assert!(summary.contains("1 lost"));
        assert!(summary.contains("3/4"));
    }
}
