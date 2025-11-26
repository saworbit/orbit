/*!
 * Guidance System ("Flight Computer")
 *
 * Responsible for validating, sanitizing, and optimizing transfer configurations
 * before execution. It acts as the "Pre-flight Check" to ensure safety and performance.
 */

use crate::config::{CompressionType, CopyConfig, CopyMode};
use crate::core::zero_copy::ZeroCopyCapabilities;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The Guidance system responsible for validating and optimizing transfer configurations.
pub struct Guidance;

/// The output of a guidance check, containing the optimized config and pilot notices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightPlan {
    /// The sanitized and optimized configuration to be used for execution
    pub final_config: CopyConfig,
    /// Notices generated during the optimization pass
    pub notices: Vec<Notice>,
}

/// A notification from the Guidance system explaining a config change or warning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notice {
    pub level: NoticeLevel,
    pub code: String,
    pub category: String,
    pub message: String,
    pub caused_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoticeLevel {
    /// General information about the plan
    Info,
    /// Potential issues that don't stop execution
    Warning,
    /// Performance adjustments (e.g., disabling zero-copy)
    Optimization,
    /// Critical changes to prevent data corruption
    Safety,
}

impl fmt::Display for Notice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let icon = match self.level {
            NoticeLevel::Info => "ℹ️ ",
            NoticeLevel::Warning => "⚠️ ",
            NoticeLevel::Optimization => "🚀",
            NoticeLevel::Safety => "🛡️ ",
        };
        write!(f, "{} {}: {}", icon, self.category, self.message)
    }
}

impl Guidance {
    /// Runs pre-flight checks to sanitize and optimize the configuration.
    pub fn plan(mut config: CopyConfig) -> Result<FlightPlan> {
        let mut notices = Vec::new();
        let sys_caps = ZeroCopyCapabilities::detect();

        // =================================================================================
        // RULE 1: Hardware Reality (Zero-Copy Support)
        // =================================================================================
        if config.use_zero_copy && !sys_caps.available {
            notices.push(Notice {
                level: NoticeLevel::Warning,
                code: "ZEROCOPY_UNAVAILABLE".to_string(),
                category: "Hardware".to_string(),
                message: format!(
                    "Zero-copy not supported on {} ({}). Disabling optimization.",
                    std::env::consts::OS,
                    sys_caps.method
                ),
                caused_by: vec!["system_capability".to_string()],
            });
            config.use_zero_copy = false;
        }

        // =================================================================================
        // RULE 2: Integrity Strategy (Zero-Copy vs Checksum)
        // =================================================================================
        // Zero-copy moves data kernel-to-kernel. Reading it back for a checksum kills the speed.
        if config.use_zero_copy && config.verify_checksum {
            notices.push(Notice {
                level: NoticeLevel::Optimization,
                code: "ZEROCOPY_DISABLED_CHECKSUM".to_string(),
                category: "Strategy".to_string(),
                message: "Disabling zero-copy to allow streaming checksum verification (faster than Zero-Copy + Read-Back).".to_string(),
                caused_by: vec!["verify_checksum".to_string()],
            });
            config.use_zero_copy = false;
        }

        // =================================================================================
        // RULE 3: The Integrity Paradox (Resume vs Checksum)
        // =================================================================================
        // Streaming verification fails on resume because we miss the start of the file.
        if config.resume_enabled && config.verify_checksum {
            notices.push(Notice {
                level: NoticeLevel::Safety,
                code: "CHECKSUM_DISABLED_RESUME".to_string(),
                category: "Integrity".to_string(),
                message: "Resume enabled; disabling streaming checksum verification (requires full file read).".to_string(),
                caused_by: vec!["resume_enabled".to_string()],
            });
            config.verify_checksum = false;
        }

        // =================================================================================
        // RULE 4: Data Safety (Resume vs Compression)
        // =================================================================================
        // You cannot safely append to a standard compressed stream without context.
        if config.resume_enabled && config.compression != CompressionType::None {
            notices.push(Notice {
                level: NoticeLevel::Safety,
                code: "RESUME_DISABLED_COMPRESSION".to_string(),
                category: "Safety".to_string(),
                message: "Disabling resume capability to prevent compressed stream corruption."
                    .to_string(),
                caused_by: vec!["compression".to_string()],
            });
            config.resume_enabled = false;
        }

        // =================================================================================
        // RULE 5: Seeking Precision (Zero-Copy vs Resume)
        // =================================================================================
        // Zero-copy usually requires transferring whole file descriptors.
        if config.use_zero_copy && config.resume_enabled {
            notices.push(Notice {
                level: NoticeLevel::Optimization,
                code: "ZEROCOPY_DISABLED_RESUME".to_string(),
                category: "Precision".to_string(),
                message: "Resume enabled; disabling zero-copy to support precise offset seeking."
                    .to_string(),
                caused_by: vec!["resume_enabled".to_string()],
            });
            config.use_zero_copy = false;
        }

        // =================================================================================
        // RULE 6: The Observer Effect (Manifest vs Zero-Copy)
        // =================================================================================
        // We cannot chunk/hash data we never see in userspace.
        if config.generate_manifest && config.use_zero_copy {
            notices.push(Notice {
                level: NoticeLevel::Optimization,
                code: "ZEROCOPY_DISABLED_MANIFEST".to_string(),
                category: "Visibility".to_string(),
                message: "Manifest generation requires content inspection. Disabling zero-copy."
                    .to_string(),
                caused_by: vec!["generate_manifest".to_string()],
            });
            config.use_zero_copy = false;
        }

        // =================================================================================
        // RULE 7: The Patchwork Problem (Delta vs Zero-Copy)
        // =================================================================================
        // Delta implies application-level patching logic that zero-copy bypasses.
        if matches!(config.check_mode, crate::core::delta::CheckMode::Delta) && config.use_zero_copy
        {
            notices.push(Notice {
                level: NoticeLevel::Optimization,
                code: "ZEROCOPY_DISABLED_DELTA".to_string(),
                category: "Logic".to_string(),
                message: "Delta transfer active. Disabling zero-copy to handle patch application."
                    .to_string(),
                caused_by: vec!["delta_mode".to_string()],
            });
            config.use_zero_copy = false;
        }

        // =================================================================================
        // RULE 8: The Speed Limit (macOS Bandwidth)
        // =================================================================================
        #[cfg(target_os = "macos")]
        if config.use_zero_copy && config.max_bandwidth > 0 {
            notices.push(Notice {
                level: NoticeLevel::Warning,
                code: "ZEROCOPY_DISABLED_BANDWIDTH".to_string(),
                category: "Control".to_string(),
                message: "macOS zero-copy (fcopyfile) cannot be throttled. Disabling zero-copy to enforce limit.".to_string(),
                caused_by: vec!["max_bandwidth".to_string()],
            });
            config.use_zero_copy = false;
        }

        // =================================================================================
        // RULE 9: Visual Noise (Parallel vs Progress)
        // =================================================================================
        // Warn about console artifacts when running parallel with progress bars
        if config.parallel > 1 && config.show_progress {
            notices.push(Notice {
                level: NoticeLevel::Info,
                code: "PARALLEL_PROGRESS_WARNING".to_string(),
                category: "UX".to_string(),
                message: "Parallel transfer with progress bars may cause visual artifacts."
                    .to_string(),
                caused_by: vec!["parallel".to_string(), "show_progress".to_string()],
            });
        }

        // =================================================================================
        // RULE 10: Performance Warning (Sync vs Checksum)
        // =================================================================================
        if matches!(config.copy_mode, CopyMode::Sync | CopyMode::Update)
            && matches!(config.check_mode, crate::core::delta::CheckMode::Checksum)
        {
            notices.push(Notice {
                level: NoticeLevel::Info,
                code: "SYNC_CHECKSUM_PERFORMANCE".to_string(),
                category: "Performance".to_string(),
                message: "'Checksum' check mode enabled with Sync/Update. This forces full file reads on both ends.".to_string(),
                caused_by: vec!["copy_mode".to_string(), "check_mode".to_string()],
            });
        }

        // =================================================================================
        // RULE 11: The Userspace Boundary (Zero-Copy vs Compression)
        // =================================================================================
        // Zero-copy moves data kernel-to-kernel. Compression requires userspace buffering.
        // This is the CRITICAL RULE from the specification!
        if config.use_zero_copy && config.compression != CompressionType::None {
            notices.push(Notice {
                level: NoticeLevel::Warning,
                code: "ZEROCOPY_DISABLED_COMPRESSION".to_string(),
                category: "Logic".to_string(),
                message: "Zero-copy disabled because compression is enabled. Compression requires userspace buffering.".to_string(),
                caused_by: vec!["compression".to_string()],
            });
            config.use_zero_copy = false;
        }

        // =================================================================================
        // RULE 12: Physics (Compression vs Encryption) - Placeholder
        // =================================================================================
        /*
        if config.encryption.is_some() && config.compression != CompressionType::None {
            notices.push(Notice {
                level: NoticeLevel::Optimization,
                code: "COMPRESSION_DISABLED_ENCRYPTION".to_string(),
                category: "Physics",
                message: "Disabling compression because encryption is active (encrypted data has max entropy).".to_string(),
                caused_by: vec!["encryption".to_string()],
            });
            config.compression = CompressionType::None;
        }
        */

        Ok(FlightPlan {
            final_config: config,
            notices,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_resume_vs_compression() {
        let mut config = CopyConfig::default();
        config.resume_enabled = true;
        config.compression = CompressionType::Zstd { level: 3 };

        let plan = Guidance::plan(config).unwrap();

        // Resume must be disabled to prevent corruption
        assert_eq!(plan.final_config.resume_enabled, false);
        assert!(plan
            .notices
            .iter()
            .any(|n| n.category == "Safety".to_string()));
    }

    #[test]
    fn test_strategy_zerocopy_vs_checksum() {
        let mut config = CopyConfig::default();
        config.use_zero_copy = true;
        config.verify_checksum = true;

        let plan = Guidance::plan(config).unwrap();

        // Zero-copy must be disabled to allow streaming hash
        assert_eq!(plan.final_config.use_zero_copy, false);
        assert!(plan
            .notices
            .iter()
            .any(|n| n.category == "Strategy".to_string()));
    }

    #[test]
    fn test_paradox_resume_vs_checksum() {
        let mut config = CopyConfig::default();
        config.resume_enabled = true;
        config.verify_checksum = true;

        let plan = Guidance::plan(config).unwrap();

        // Checksum verification must be disabled on resume
        assert_eq!(plan.final_config.verify_checksum, false);
        assert!(plan
            .notices
            .iter()
            .any(|n| n.category == "Integrity".to_string()));
    }

    #[test]
    fn test_observer_manifest_vs_zerocopy() {
        let mut config = CopyConfig::default();
        config.generate_manifest = true;
        config.use_zero_copy = true;
        config.verify_checksum = false; // Disable to avoid triggering rule 2

        let plan = Guidance::plan(config).unwrap();

        assert_eq!(plan.final_config.use_zero_copy, false);
        assert!(plan
            .notices
            .iter()
            .any(|n| n.category == "Visibility".to_string()));
    }

    #[test]
    fn test_patchwork_delta_vs_zerocopy() {
        let mut config = CopyConfig::default();
        config.check_mode = crate::core::delta::CheckMode::Delta;
        config.use_zero_copy = true;
        config.verify_checksum = false; // Disable to avoid triggering rule 2

        let plan = Guidance::plan(config).unwrap();

        assert_eq!(plan.final_config.use_zero_copy, false);
        assert!(plan
            .notices
            .iter()
            .any(|n| n.category == "Logic".to_string()));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_speed_limit_macos_bandwidth() {
        let mut config = CopyConfig::default();
        config.use_zero_copy = true;
        config.max_bandwidth = 1_000_000; // 1 MB/s
        config.verify_checksum = false; // Disable to avoid triggering rule 2

        let plan = Guidance::plan(config).unwrap();

        assert_eq!(plan.final_config.use_zero_copy, false);
        assert!(plan
            .notices
            .iter()
            .any(|n| n.category == "Control".to_string()));
    }

    #[test]
    fn test_visual_noise_parallel_progress() {
        let mut config = CopyConfig::default();
        config.parallel = 4;
        config.show_progress = true;
        config.use_zero_copy = false; // Avoid other rules

        let plan = Guidance::plan(config).unwrap();

        assert!(plan.notices.iter().any(|n| n.category == "UX".to_string()));
    }

    #[test]
    fn test_performance_warning_sync_checksum() {
        let mut config = CopyConfig::default();
        config.copy_mode = CopyMode::Sync;
        config.check_mode = crate::core::delta::CheckMode::Checksum;
        config.use_zero_copy = false; // Avoid other rules

        let plan = Guidance::plan(config).unwrap();

        assert!(plan
            .notices
            .iter()
            .any(|n| n.category == "Performance".to_string()));
    }

    #[test]
    fn test_clean_config_minimal_notices() {
        let mut config = CopyConfig::default();
        config.use_zero_copy = false; // Avoid conflicts
        config.verify_checksum = false; // Avoid conflicts

        let plan = Guidance::plan(config).unwrap();

        // Should have no notices for a clean, conflict-free config
        assert!(plan.notices.is_empty());
    }

    #[test]
    fn test_multiple_rules_triggered() {
        let mut config = CopyConfig::default();
        config.use_zero_copy = true;
        config.resume_enabled = true;
        config.compression = CompressionType::Lz4;
        config.verify_checksum = true;

        let plan = Guidance::plan(config).unwrap();

        // Multiple rules should have been triggered
        assert!(plan.notices.len() >= 2);

        // Resume should be disabled due to compression
        assert_eq!(plan.final_config.resume_enabled, false);
        // Zero-copy should be disabled
        assert_eq!(plan.final_config.use_zero_copy, false);
    }

    #[test]
    fn test_notice_display_format() {
        let notice = Notice {
            level: NoticeLevel::Warning,
            code: "TEST_CODE".to_string(),
            category: "Test".to_string(),
            message: "Test message".to_string(),
            caused_by: vec!["test_feature".to_string()],
        };

        let display = format!("{}", notice);
        assert!(display.contains("⚠️"));
        assert!(display.contains("Test"));
        assert!(display.contains("Test message"));
    }

    #[test]
    fn test_zerocopy_vs_compression() {
        let mut config = CopyConfig::default();
        config.use_zero_copy = true;
        config.compression = CompressionType::Lz4;
        config.verify_checksum = false; // Disable to avoid triggering rule 2

        let plan = Guidance::plan(config).unwrap();

        // Zero-copy must be disabled because compression requires userspace buffering
        assert_eq!(plan.final_config.use_zero_copy, false);
        assert_eq!(plan.final_config.compression, CompressionType::Lz4);

        // Verify the notice
        let notice = plan
            .notices
            .iter()
            .find(|n| n.code == "ZEROCOPY_DISABLED_COMPRESSION");
        assert!(
            notice.is_some(),
            "Expected ZEROCOPY_DISABLED_COMPRESSION notice"
        );

        let notice = notice.unwrap();
        assert_eq!(notice.level, NoticeLevel::Warning);
        assert_eq!(notice.category, "Logic".to_string());
        assert!(notice.caused_by.contains(&"compression".to_string()));
    }
}
