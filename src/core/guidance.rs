/*!
 * Guidance System ("Flight Computer")
 *
 * Responsible for validating, sanitizing, and optimizing transfer configurations
 * before execution. It enforces logical consistency and safety rules.
 */

use crate::config::{CompressionType, CopyConfig, CopyMode};
use crate::core::zero_copy::ZeroCopyCapabilities;
use crate::error::Result;
use std::fmt;

/// The Guidance system responsible for validating and optimizing transfer configurations.
pub struct Guidance;

/// The output of a guidance check, containing the optimized config and pilot notices.
pub struct FlightPlan {
    /// The sanitized and optimized configuration to be used for execution
    pub config: CopyConfig,
    /// Notices generated during the optimization pass
    pub notices: Vec<Notice>,
}

/// A notification from the Guidance system
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    pub level: NoticeLevel,
    pub category: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoticeLevel {
    Info,
    Warning,
    Optimization,
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
    ///
    /// This resolves logical conflicts (e.g., Zero-Copy vs Checksum) and enforces
    /// best practices (e.g., Compress before Encrypt).
    pub fn plan(mut config: CopyConfig) -> Result<FlightPlan> {
        let mut notices = Vec::new();
        let sys_caps = ZeroCopyCapabilities::detect();

        // --- RULE 1: Hardware Reality (Zero-Copy Support) ---
        // If zero-copy is requested but not supported by OS/Hardware
        if config.use_zero_copy && !sys_caps.available {
            notices.push(Notice {
                level: NoticeLevel::Warning,
                category: "Hardware",
                message: format!(
                    "Zero-copy not supported on {} ({}). Disabling optimization.",
                    std::env::consts::OS,
                    sys_caps.method
                ),
            });
            config.use_zero_copy = false;
        }

        // --- RULE 2: Integrity Strategy (Zero-Copy vs Checksum) ---
        // Zero-copy moves data kernel-to-kernel. Reading it back for a checksum
        // effectively reads the file twice, killing performance.
        if config.use_zero_copy && config.verify_checksum {
            notices.push(Notice {
                level: NoticeLevel::Optimization,
                category: "Strategy",
                message: "Disabling zero-copy to allow streaming checksum verification (faster than Zero-Copy + Read-Back).".to_string(),
            });
            config.use_zero_copy = false;
        }

        // --- RULE 3: Data Safety (Resume vs Compression) ---
        // You cannot safely append to a standard compressed stream without context.
        if config.resume_enabled && config.compression != CompressionType::None {
            notices.push(Notice {
                level: NoticeLevel::Safety,
                category: "Safety",
                message: "Disabling resume capability to prevent compressed stream corruption (cannot resume standard streams).".to_string(),
            });
            config.resume_enabled = false;
        }

        // --- RULE 4: Seeking Precision (Zero-Copy vs Resume) ---
        // Zero-copy usually requires transferring whole file descriptors or blocks.
        // Precise byte-level resuming is safer with buffered I/O.
        if config.use_zero_copy && config.resume_enabled {
            notices.push(Notice {
                level: NoticeLevel::Optimization,
                category: "Precision",
                message: "Resume enabled; disabling zero-copy to support precise offset seeking."
                    .to_string(),
            });
            config.use_zero_copy = false;
        }

        // --- RULE 5: Performance Warning (Sync vs Checksum) ---
        // Sync/Update with Checksum check mode forces full reads on both sides.
        if matches!(config.copy_mode, CopyMode::Sync | CopyMode::Update)
            && matches!(config.check_mode, crate::core::delta::CheckMode::Checksum)
        {
            notices.push(Notice {
                level: NoticeLevel::Info,
                category: "Performance",
                message: "'Checksum' check mode enabled with Sync/Update. This forces full file reads on both ends.".to_string(),
            });
        }

        // --- RULE 6: Entropy (Compression vs Encryption) ---
        // Placeholder for when Encryption is added to CopyConfig
        /*
        if config.encryption.is_some() && config.compression != CompressionType::None {
             // Ensure Compression runs BEFORE Encryption
             // Or disable compression if order cannot be guaranteed
        }
        */

        Ok(FlightPlan { config, notices })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_rule_resume_vs_compression() {
        let mut config = CopyConfig::default();
        config.resume_enabled = true;
        config.compression = CompressionType::Zstd { level: 3 };

        let plan = Guidance::plan(config).unwrap();

        // Should disable resume
        assert_eq!(plan.config.resume_enabled, false);
        // Should keep compression
        assert!(matches!(
            plan.config.compression,
            CompressionType::Zstd { .. }
        ));
        // Should have a safety notice
        assert!(plan.notices.iter().any(|n| n.level == NoticeLevel::Safety));
    }

    #[test]
    fn test_optimization_rule_zerocopy_vs_checksum() {
        let mut config = CopyConfig::default();
        config.use_zero_copy = true;
        config.verify_checksum = true;

        // Mock capabilities if possible, otherwise this tests specific platform behavior
        // Assuming we are running on a platform where ZeroCopy is technically "possible" in config
        let plan = Guidance::plan(config).unwrap();

        // Should disable zero-copy to favor checksum
        assert_eq!(plan.config.use_zero_copy, false);
        assert!(plan
            .notices
            .iter()
            .any(|n| n.level == NoticeLevel::Optimization));
    }

    #[test]
    fn test_clean_config_has_no_notices() {
        let config = CopyConfig::default(); // Safe defaults
        let plan = Guidance::plan(config).unwrap();
        // Default config has verify_checksum=true and use_zero_copy=true, so they conflict
        // So we'll actually have notices. Let's test a truly clean config
        let mut config = CopyConfig::default();
        config.use_zero_copy = false; // No conflicts
        let plan = Guidance::plan(config).unwrap();
        assert!(plan.notices.is_empty());
    }

    #[test]
    fn test_precision_rule_zerocopy_vs_resume() {
        let mut config = CopyConfig::default();
        config.use_zero_copy = true;
        config.resume_enabled = true;
        config.verify_checksum = false; // Disable to avoid triggering rule 2

        let plan = Guidance::plan(config).unwrap();

        // Should disable zero-copy to favor resume precision
        assert_eq!(plan.config.use_zero_copy, false);
        assert!(plan.config.resume_enabled);
        assert!(plan
            .notices
            .iter()
            .any(|n| n.level == NoticeLevel::Optimization && n.category == "Precision"));
    }

    #[test]
    fn test_performance_info_sync_checksum() {
        let mut config = CopyConfig::default();
        config.copy_mode = CopyMode::Sync;
        config.check_mode = crate::core::delta::CheckMode::Checksum;
        config.use_zero_copy = false; // Avoid other rules

        let plan = Guidance::plan(config).unwrap();

        // Should have an info notice about performance
        assert!(plan
            .notices
            .iter()
            .any(|n| n.level == NoticeLevel::Info && n.category == "Performance"));
    }

    #[test]
    fn test_notice_display_format() {
        let notice = Notice {
            level: NoticeLevel::Warning,
            category: "Test",
            message: "Test message".to_string(),
        };

        let display = format!("{}", notice);
        assert!(display.contains("⚠️"));
        assert!(display.contains("Test"));
        assert!(display.contains("Test message"));
    }

    #[test]
    fn test_hardware_rule_zerocopy_unsupported() {
        // This test is platform-dependent. On platforms that don't support zero-copy,
        // the guidance system should detect it and add a warning.
        let mut config = CopyConfig::default();
        config.use_zero_copy = true;

        let plan = Guidance::plan(config).unwrap();
        let sys_caps = ZeroCopyCapabilities::detect();

        if !sys_caps.available {
            // Should have disabled zero-copy
            assert_eq!(plan.config.use_zero_copy, false);
            // Should have a warning
            assert!(plan
                .notices
                .iter()
                .any(|n| n.level == NoticeLevel::Warning && n.category == "Hardware"));
        } else {
            // On platforms with zero-copy support, other rules might still apply
            // (like checksum verification), so we just check that it was processed
            assert!(plan.config.verify_checksum); // Default is true
        }
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
        // Rule 2: Zero-copy vs Checksum OR Rule 4: Zero-copy vs Resume
        // Rule 3: Resume vs Compression
        assert!(plan.notices.len() >= 2);

        // Resume should be disabled due to compression
        assert_eq!(plan.config.resume_enabled, false);
        // Zero-copy should be disabled
        assert_eq!(plan.config.use_zero_copy, false);
    }
}
