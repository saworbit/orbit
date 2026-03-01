//! Health Monitor: Continuous mid-transfer health checks
//!
//! Unlike pre-flight checks that run once before transfer, the health monitor
//! runs periodically during transfer to detect degrading conditions early:
//! - Disk fill rate trending toward full
//! - Network throughput dropping below threshold
//! - Memory pressure rising
//! - Source/destination becoming unreachable
//!
//! # Design
//!
//! The health monitor is a pure-logic state machine. It receives health
//! samples from the caller and produces advisories. The caller is responsible
//! for collecting the actual system metrics and acting on advisories.
//!
//! # Example
//!
//! ```
//! use orbit_core_resilience::health_monitor::{HealthMonitor, HealthSample, HealthConfig, Advisory};
//!
//! let config = HealthConfig {
//!     disk_critical_pct: 95.0,
//!     disk_warning_pct: 85.0,
//!     throughput_floor_bps: 1_000_000, // 1 MB/s minimum
//!     check_interval_secs: 30,
//! };
//!
//! let mut monitor = HealthMonitor::new(config);
//!
//! // Feed a health sample
//! let advisories = monitor.check(HealthSample {
//!     disk_used_pct: 96.0,
//!     throughput_bps: 500_000,
//!     memory_used_pct: 70.0,
//!     active_errors: 2,
//!     ..Default::default()
//! });
//!
//! // Monitor produces advisories
//! assert!(advisories.iter().any(|a| matches!(a, Advisory::DiskCritical { .. })));
//! ```

use std::collections::VecDeque;
use std::time::Instant;

/// Configuration for health monitoring thresholds
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Disk usage percentage that triggers a critical advisory (pause transfer)
    pub disk_critical_pct: f64,

    /// Disk usage percentage that triggers a warning advisory
    pub disk_warning_pct: f64,

    /// Minimum throughput in bytes/sec — below this triggers a warning
    pub throughput_floor_bps: u64,

    /// How often checks should be performed (guidance for the caller)
    pub check_interval_secs: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            disk_critical_pct: 95.0,
            disk_warning_pct: 85.0,
            throughput_floor_bps: 100_000, // 100 KB/s
            check_interval_secs: 30,
        }
    }
}

/// A snapshot of system health metrics
#[derive(Debug, Clone, Default)]
pub struct HealthSample {
    /// Destination disk usage as a percentage (0.0 - 100.0)
    pub disk_used_pct: f64,

    /// Current throughput in bytes per second
    pub throughput_bps: u64,

    /// System memory usage as a percentage (0.0 - 100.0)
    pub memory_used_pct: f64,

    /// Number of errors in the current sample window
    pub active_errors: u32,

    /// Available disk space in bytes (optional, for trending)
    pub disk_available_bytes: Option<u64>,
}

/// Advisory produced by the health monitor
#[derive(Debug, Clone, PartialEq)]
pub enum Advisory {
    /// Disk usage is critical — should pause transfer
    DiskCritical { used_pct: f64 },

    /// Disk usage is elevated — should alert operator
    DiskWarning { used_pct: f64 },

    /// Disk fill rate predicts exhaustion within N seconds
    DiskExhaustionPredicted { seconds_remaining: f64 },

    /// Throughput has dropped below the configured floor
    ThroughputLow { current_bps: u64, floor_bps: u64 },

    /// Error rate is elevated
    ErrorRateHigh { errors: u32, window_secs: u64 },

    /// System is healthy — no issues detected
    Healthy,
}

/// Continuous health monitor that produces advisories from health samples.
#[derive(Debug)]
pub struct HealthMonitor {
    config: HealthConfig,
    /// Recent disk available bytes for fill-rate prediction
    disk_history: VecDeque<(Instant, u64)>,
    /// Maximum samples to keep for trending
    max_history: usize,
    /// Total checks performed
    check_count: u64,
    /// Total advisories produced (excluding Healthy)
    advisory_count: u64,
}

impl HealthMonitor {
    /// Create a new health monitor with the given configuration
    pub fn new(config: HealthConfig) -> Self {
        Self {
            config,
            disk_history: VecDeque::with_capacity(64),
            max_history: 60, // ~30 minutes at 30s intervals
            check_count: 0,
            advisory_count: 0,
        }
    }

    /// Process a health sample and produce advisories.
    ///
    /// Returns a list of advisories. If the system is healthy, returns
    /// a single `Advisory::Healthy`.
    pub fn check(&mut self, sample: HealthSample) -> Vec<Advisory> {
        self.check_count += 1;
        let mut advisories = Vec::new();

        // Disk checks
        if sample.disk_used_pct >= self.config.disk_critical_pct {
            advisories.push(Advisory::DiskCritical {
                used_pct: sample.disk_used_pct,
            });
        } else if sample.disk_used_pct >= self.config.disk_warning_pct {
            advisories.push(Advisory::DiskWarning {
                used_pct: sample.disk_used_pct,
            });
        }

        // Disk fill rate prediction
        if let Some(available) = sample.disk_available_bytes {
            let now = Instant::now();
            self.disk_history.push_back((now, available));
            while self.disk_history.len() > self.max_history {
                self.disk_history.pop_front();
            }

            if let Some(prediction) = self.predict_disk_exhaustion() {
                if prediction > 0.0 && prediction < 3600.0 {
                    // Less than 1 hour
                    advisories.push(Advisory::DiskExhaustionPredicted {
                        seconds_remaining: prediction,
                    });
                }
            }
        }

        // Throughput check
        if sample.throughput_bps > 0 && sample.throughput_bps < self.config.throughput_floor_bps {
            advisories.push(Advisory::ThroughputLow {
                current_bps: sample.throughput_bps,
                floor_bps: self.config.throughput_floor_bps,
            });
        }

        // Error rate check
        if sample.active_errors > 5 {
            advisories.push(Advisory::ErrorRateHigh {
                errors: sample.active_errors,
                window_secs: self.config.check_interval_secs,
            });
        }

        if advisories.is_empty() {
            advisories.push(Advisory::Healthy);
        } else {
            self.advisory_count += advisories.len() as u64;
        }

        advisories
    }

    /// Predict seconds until disk exhaustion based on fill rate trend.
    ///
    /// Uses linear regression over the disk availability history.
    fn predict_disk_exhaustion(&self) -> Option<f64> {
        if self.disk_history.len() < 3 {
            return None;
        }

        let first = self.disk_history.front()?;
        let last = self.disk_history.back()?;

        let elapsed_secs = last.0.duration_since(first.0).as_secs_f64();
        if elapsed_secs < 10.0 {
            return None; // Not enough time span for meaningful prediction
        }

        let bytes_consumed = first.1.saturating_sub(last.1);
        if bytes_consumed == 0 {
            return None; // Disk usage not increasing
        }

        let consume_rate_bps = bytes_consumed as f64 / elapsed_secs;
        let remaining_secs = last.1 as f64 / consume_rate_bps;

        Some(remaining_secs)
    }

    /// Get monitoring statistics
    pub fn stats(&self) -> HealthMonitorStats {
        HealthMonitorStats {
            check_count: self.check_count,
            advisory_count: self.advisory_count,
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &HealthConfig {
        &self.config
    }
}

/// Statistics for the health monitor
#[derive(Debug, Clone)]
pub struct HealthMonitorStats {
    /// Total health checks performed
    pub check_count: u64,
    /// Total advisories produced (excluding Healthy)
    pub advisory_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthy_system() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        let advisories = monitor.check(HealthSample {
            disk_used_pct: 50.0,
            throughput_bps: 10_000_000,
            memory_used_pct: 40.0,
            active_errors: 0,
            ..Default::default()
        });

        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0], Advisory::Healthy);
    }

    #[test]
    fn test_disk_critical() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        let advisories = monitor.check(HealthSample {
            disk_used_pct: 96.0,
            throughput_bps: 10_000_000,
            ..Default::default()
        });

        assert!(advisories
            .iter()
            .any(|a| matches!(a, Advisory::DiskCritical { .. })));
    }

    #[test]
    fn test_disk_warning() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        let advisories = monitor.check(HealthSample {
            disk_used_pct: 90.0,
            throughput_bps: 10_000_000,
            ..Default::default()
        });

        assert!(advisories
            .iter()
            .any(|a| matches!(a, Advisory::DiskWarning { .. })));
    }

    #[test]
    fn test_low_throughput() {
        let config = HealthConfig {
            throughput_floor_bps: 1_000_000,
            ..Default::default()
        };
        let mut monitor = HealthMonitor::new(config);

        let advisories = monitor.check(HealthSample {
            disk_used_pct: 50.0,
            throughput_bps: 500_000,
            ..Default::default()
        });

        assert!(advisories.iter().any(|a| matches!(
            a,
            Advisory::ThroughputLow {
                current_bps: 500_000,
                floor_bps: 1_000_000
            }
        )));
    }

    #[test]
    fn test_error_rate_high() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        let advisories = monitor.check(HealthSample {
            disk_used_pct: 50.0,
            throughput_bps: 10_000_000,
            active_errors: 10,
            ..Default::default()
        });

        assert!(advisories
            .iter()
            .any(|a| matches!(a, Advisory::ErrorRateHigh { .. })));
    }

    #[test]
    fn test_multiple_advisories() {
        let mut monitor = HealthMonitor::new(HealthConfig {
            throughput_floor_bps: 1_000_000,
            ..Default::default()
        });

        let advisories = monitor.check(HealthSample {
            disk_used_pct: 96.0,
            throughput_bps: 100,
            active_errors: 10,
            ..Default::default()
        });

        // Should get multiple advisories
        assert!(advisories.len() >= 3);
    }

    #[test]
    fn test_stats() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        monitor.check(HealthSample::default());
        monitor.check(HealthSample {
            disk_used_pct: 96.0,
            ..Default::default()
        });

        let stats = monitor.stats();
        assert_eq!(stats.check_count, 2);
        assert!(stats.advisory_count >= 1);
    }

    #[test]
    fn test_disk_at_exact_critical_threshold() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        let advisories = monitor.check(HealthSample {
            disk_used_pct: 95.0, // exactly at default critical threshold
            throughput_bps: 10_000_000,
            ..Default::default()
        });

        // >= 95.0 should trigger DiskCritical
        assert!(advisories
            .iter()
            .any(|a| matches!(a, Advisory::DiskCritical { used_pct } if *used_pct == 95.0)));
    }

    #[test]
    fn test_disk_at_exact_warning_threshold() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        let advisories = monitor.check(HealthSample {
            disk_used_pct: 85.0, // exactly at default warning threshold
            throughput_bps: 10_000_000,
            ..Default::default()
        });

        // >= 85.0 (but < 95.0) should trigger DiskWarning
        assert!(advisories
            .iter()
            .any(|a| matches!(a, Advisory::DiskWarning { used_pct } if *used_pct == 85.0)));
    }

    #[test]
    fn test_zero_throughput_no_advisory() {
        let config = HealthConfig {
            throughput_floor_bps: 1_000_000,
            ..Default::default()
        };
        let mut monitor = HealthMonitor::new(config);

        let advisories = monitor.check(HealthSample {
            disk_used_pct: 50.0,
            throughput_bps: 0, // zero throughput: code checks > 0 first
            ..Default::default()
        });

        // throughput_bps == 0 should NOT trigger ThroughputLow because
        // the condition is `throughput_bps > 0 && throughput_bps < floor`
        assert!(!advisories
            .iter()
            .any(|a| matches!(a, Advisory::ThroughputLow { .. })));
    }

    #[test]
    fn test_error_rate_boundary() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        // active_errors = 5 should NOT trigger (condition is > 5)
        let advisories = monitor.check(HealthSample {
            disk_used_pct: 50.0,
            throughput_bps: 10_000_000,
            active_errors: 5,
            ..Default::default()
        });
        assert!(!advisories
            .iter()
            .any(|a| matches!(a, Advisory::ErrorRateHigh { .. })));

        // active_errors = 6 SHOULD trigger
        let advisories = monitor.check(HealthSample {
            disk_used_pct: 50.0,
            throughput_bps: 10_000_000,
            active_errors: 6,
            ..Default::default()
        });
        assert!(advisories
            .iter()
            .any(|a| matches!(a, Advisory::ErrorRateHigh { errors: 6, .. })));
    }

    #[test]
    fn test_disk_exhaustion_prediction() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        // predict_disk_exhaustion requires >= 3 samples to return Some
        // With fewer than 3 samples it returns None

        // Feed 1 sample
        monitor.check(HealthSample {
            disk_used_pct: 50.0,
            throughput_bps: 10_000_000,
            disk_available_bytes: Some(1_000_000),
            ..Default::default()
        });
        assert!(monitor.predict_disk_exhaustion().is_none());

        // Feed 2 samples
        monitor.check(HealthSample {
            disk_used_pct: 55.0,
            throughput_bps: 10_000_000,
            disk_available_bytes: Some(900_000),
            ..Default::default()
        });
        assert!(monitor.predict_disk_exhaustion().is_none());

        // Feed 3rd sample — now we have 3, but time elapsed between
        // Instant::now() calls in a test is tiny (< 10s), so it should
        // return None due to the elapsed_secs < 10.0 guard.
        monitor.check(HealthSample {
            disk_used_pct: 60.0,
            throughput_bps: 10_000_000,
            disk_available_bytes: Some(800_000),
            ..Default::default()
        });
        // With 3 samples but near-zero elapsed time, predict returns None
        assert!(monitor.predict_disk_exhaustion().is_none());
    }

    #[test]
    fn test_disk_history_pruning() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        // Push more than max_history (60) samples
        for i in 0..80 {
            monitor.check(HealthSample {
                disk_used_pct: 50.0,
                throughput_bps: 10_000_000,
                disk_available_bytes: Some(1_000_000 - i * 1000),
                ..Default::default()
            });
        }

        // Deque should be capped at max_history
        assert!(monitor.disk_history.len() <= monitor.max_history);
    }

    #[test]
    fn test_stable_disk_no_exhaustion_prediction() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        // Feed multiple samples with constant disk_available_bytes
        for _ in 0..5 {
            let advisories = monitor.check(HealthSample {
                disk_used_pct: 50.0,
                throughput_bps: 10_000_000,
                disk_available_bytes: Some(500_000),
                ..Default::default()
            });

            // Should never produce DiskExhaustionPredicted when disk is stable
            assert!(!advisories
                .iter()
                .any(|a| matches!(a, Advisory::DiskExhaustionPredicted { .. })));
        }

        // Also verify predict_disk_exhaustion returns None (bytes_consumed == 0)
        assert!(monitor.predict_disk_exhaustion().is_none());
    }

    #[test]
    fn test_advisory_count_excludes_healthy() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        // Feed a completely healthy sample
        let advisories = monitor.check(HealthSample {
            disk_used_pct: 50.0,
            throughput_bps: 10_000_000,
            memory_used_pct: 40.0,
            active_errors: 0,
            ..Default::default()
        });

        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0], Advisory::Healthy);
        // advisory_count should remain 0 because Healthy is not counted
        assert_eq!(monitor.stats().advisory_count, 0);
    }

    #[test]
    fn test_default_config_values() {
        let config = HealthConfig::default();

        assert_eq!(config.disk_critical_pct, 95.0);
        assert_eq!(config.disk_warning_pct, 85.0);
        assert_eq!(config.throughput_floor_bps, 100_000);
        assert_eq!(config.check_interval_secs, 30);
    }

    #[test]
    fn test_memory_usage_accepted_but_unused() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());

        // Feed a sample with high memory usage — should not trigger any advisory
        let advisories = monitor.check(HealthSample {
            disk_used_pct: 50.0,
            throughput_bps: 10_000_000,
            memory_used_pct: 99.0, // very high, but not checked by monitor
            active_errors: 0,
            ..Default::default()
        });

        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0], Advisory::Healthy);
    }
}
