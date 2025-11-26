/*!
 * Integration tests for the Guidance System
 *
 * These tests verify that the guidance system correctly sanitizes and optimizes
 * configurations before execution.
 */

use orbit::{
    config::{CompressionType, CopyConfig, CopyMode},
    copy_file,
    core::guidance::Guidance,
};
use tempfile::tempdir;

#[test]
fn test_guidance_resume_vs_compression_safety() {
    // Setup: Resume + Compression should trigger safety rule
    let mut config = CopyConfig::default();
    config.resume_enabled = true;
    config.compression = CompressionType::Zstd { level: 1 };

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Resume should be disabled
    assert_eq!(
        flight_plan.final_config.resume_enabled, false,
        "Resume should be disabled when compression is enabled"
    );

    // Verify: Compression should remain enabled
    assert!(matches!(
        flight_plan.final_config.compression,
        CompressionType::Zstd { .. }
    ));

    // Verify: Should have a safety notice
    assert!(
        flight_plan.notices.iter().any(|n| {
            n.category == "Safety"
                && n.message.contains("resume")
                && n.message.contains("compressed")
        }),
        "Expected safety notice about resume and compression"
    );
}

#[test]
fn test_guidance_zerocopy_vs_checksum_optimization() {
    // Setup: Zero-copy + Checksum should trigger optimization rule
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.verify_checksum = true;

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Zero-copy should be disabled to allow checksum
    assert_eq!(
        flight_plan.final_config.use_zero_copy, false,
        "Zero-copy should be disabled when checksum verification is enabled"
    );

    // Verify: Checksum should remain enabled
    assert!(flight_plan.final_config.verify_checksum);

    // Verify: Should have an optimization notice
    assert!(
        flight_plan.notices.iter().any(|n| {
            n.category == "Strategy"
                && n.message.contains("zero-copy")
                && n.message.contains("checksum")
        }),
        "Expected optimization notice about zero-copy and checksum"
    );
}

#[test]
fn test_guidance_zerocopy_vs_resume_precision() {
    // Setup: Zero-copy + Resume should trigger precision rule
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.resume_enabled = true;
    config.verify_checksum = false; // Disable to avoid triggering other rules

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Zero-copy should be disabled for resume precision
    assert_eq!(
        flight_plan.final_config.use_zero_copy, false,
        "Zero-copy should be disabled when resume is enabled"
    );

    // Verify: Resume should remain enabled
    assert!(flight_plan.final_config.resume_enabled);

    // Verify: Should have a precision notice
    assert!(
        flight_plan.notices.iter().any(|n| {
            n.category == "Precision" && n.message.contains("Resume") && n.message.contains("zero")
        }),
        "Expected precision notice about resume and zero-copy"
    );
}

#[test]
fn test_guidance_sync_checksum_performance_info() {
    // Setup: Sync mode + Checksum check mode should trigger performance info
    let mut config = CopyConfig::default();
    config.copy_mode = CopyMode::Sync;
    config.check_mode = orbit::core::delta::CheckMode::Checksum;
    config.use_zero_copy = false; // Avoid other rules

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Config should remain unchanged (info only, no changes)
    assert_eq!(flight_plan.final_config.copy_mode, CopyMode::Sync);

    // Verify: Should have a performance info notice
    assert!(
        flight_plan
            .notices
            .iter()
            .any(|n| n.category == "Performance" && n.message.contains("Checksum")),
        "Expected performance info notice about Sync + Checksum"
    );
}

#[test]
fn test_guidance_multiple_rules_triggered() {
    // Setup: Configuration that triggers multiple rules
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.verify_checksum = true;
    config.resume_enabled = true;
    config.compression = CompressionType::Lz4;

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Multiple notices should be generated
    assert!(
        flight_plan.notices.len() >= 2,
        "Expected multiple guidance notices"
    );

    // Verify: Resume should be disabled due to compression
    assert_eq!(
        flight_plan.final_config.resume_enabled, false,
        "Resume should be disabled"
    );

    // Verify: Zero-copy should be disabled
    assert_eq!(
        flight_plan.final_config.use_zero_copy, false,
        "Zero-copy should be disabled"
    );
}

#[test]
fn test_guidance_clean_config_no_notices() {
    // Setup: A clean configuration that doesn't trigger any rules
    let mut config = CopyConfig::default();
    config.use_zero_copy = false;
    config.verify_checksum = true;
    config.resume_enabled = false;
    config.compression = CompressionType::None;

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: No notices should be generated
    assert!(
        flight_plan.notices.is_empty(),
        "Expected no guidance notices for clean config"
    );
}

#[test]
fn test_guidance_with_actual_copy_operation() {
    // Integration test: Verify guidance system integrates correctly with actual copy
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    std::fs::write(&source, b"test data for guidance integration").unwrap();

    // Setup: Config that should trigger guidance
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.verify_checksum = true;
    config.show_progress = false; // Disable progress for clean test output

    // Apply guidance
    let flight_plan = Guidance::plan(config).unwrap();
    let optimized_config = flight_plan.final_config;

    // Verify: Guidance should have optimized the config
    assert_eq!(optimized_config.use_zero_copy, false);

    // Perform copy with optimized config
    let stats = copy_file(&source, &dest, &optimized_config).unwrap();

    // Verify: Copy should succeed
    assert!(dest.exists());
    assert_eq!(
        std::fs::read(&dest).unwrap(),
        b"test data for guidance integration"
    );
    assert_eq!(stats.files_copied, 1);
}

#[test]
fn test_guidance_display_format() {
    // Test that notices have proper display format
    let mut config = CopyConfig::default();
    config.resume_enabled = true;
    config.compression = CompressionType::Zstd { level: 3 };
    config.use_zero_copy = false; // Disable to avoid triggering other rules
    config.verify_checksum = false; // Disable to avoid triggering other rules

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: At least one notice should be generated
    assert!(!flight_plan.notices.is_empty());

    // Verify: Notice display contains expected elements
    for notice in &flight_plan.notices {
        let display = format!("{}", notice);
        // Check that the notice category is present (should be "Safety" for resume+compression)
        assert_eq!(notice.category, "Safety");
        // Check that the formatted output is not empty and contains the category
        assert!(!display.is_empty());
        assert!(display.contains("Safety"));
    }
}

#[test]
fn test_guidance_preserves_other_config_options() {
    // Verify that guidance doesn't modify unrelated config options
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.verify_checksum = true;
    config.retry_attempts = 10;
    config.chunk_size = 2 * 1024 * 1024;
    config.max_bandwidth = 1000;
    config.parallel = 4;

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Unrelated options should be preserved
    assert_eq!(flight_plan.final_config.retry_attempts, 10);
    assert_eq!(flight_plan.final_config.chunk_size, 2 * 1024 * 1024);
    assert_eq!(flight_plan.final_config.max_bandwidth, 1000);
    assert_eq!(flight_plan.final_config.parallel, 4);
}

// NOTE: CLI output test disabled - requires assert_cmd crate
// To enable, add `assert_cmd = "2.0"` to [dev-dependencies] in Cargo.toml
/*
#[test]
fn test_guidance_cli_output() {
    // Integration test: Verify the Guidance System displays in the CLI
    let dir = tempdir().unwrap();
    let source = dir.path().join("test_source.txt");
    let dest = dir.path().join("test_dest.txt");

    // Create a test file
    std::fs::write(&source, b"test data").unwrap();

    let mut cmd = Command::cargo_bin("orbit").unwrap();

    // Trigger the Safety Rule: Resume + Compress
    cmd.arg("--source")
        .arg(source.to_str().unwrap())
        .arg("--dest")
        .arg(dest.to_str().unwrap())
        .arg("--resume")
        .arg("--compress=zstd:1");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify the "Box" and the "Message" appear
    assert!(
        stdout.contains("Orbit Guidance System"),
        "Expected guidance system header in output"
    );
    assert!(
        stdout.contains("Safety") || stdout.contains("🛡️"),
        "Expected safety notice in output"
    );
    assert!(
        stdout.contains("resume") || stdout.contains("compressed"),
        "Expected message about resume/compression conflict"
    );
}
*/

#[test]
fn test_guidance_manifest_vs_zerocopy() {
    // Test RULE 6: The Observer Effect (Manifest vs Zero-Copy)
    let mut config = CopyConfig::default();
    config.generate_manifest = true;
    config.use_zero_copy = true;
    config.verify_checksum = false; // Disable to avoid triggering rule 2

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Zero-copy should be disabled
    assert_eq!(
        flight_plan.final_config.use_zero_copy, false,
        "Zero-copy should be disabled when manifest generation is enabled"
    );

    // Verify: Manifest generation should remain enabled
    assert!(flight_plan.final_config.generate_manifest);

    // Verify: Should have a visibility notice
    assert!(
        flight_plan
            .notices
            .iter()
            .any(|n| { n.category == "Visibility" && n.message.contains("Manifest") }),
        "Expected visibility notice about manifest and zero-copy"
    );
}

#[test]
fn test_guidance_delta_vs_zerocopy() {
    // Test RULE 7: The Patchwork Problem (Delta vs Zero-Copy)
    let mut config = CopyConfig::default();
    config.check_mode = orbit::core::delta::CheckMode::Delta;
    config.use_zero_copy = true;
    config.verify_checksum = false; // Disable to avoid triggering rule 2

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Zero-copy should be disabled
    assert_eq!(
        flight_plan.final_config.use_zero_copy, false,
        "Zero-copy should be disabled when delta transfer is active"
    );

    // Verify: Should have a logic notice
    assert!(
        flight_plan
            .notices
            .iter()
            .any(|n| { n.category == "Logic" && n.message.contains("Delta") }),
        "Expected logic notice about delta and zero-copy"
    );
}

#[test]
fn test_guidance_parallel_progress_ux() {
    // Test RULE 9: Visual Noise (Parallel vs Progress)
    let mut config = CopyConfig::default();
    config.parallel = 4;
    config.show_progress = true;
    config.use_zero_copy = false; // Avoid other rules

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Config should remain unchanged (info only)
    assert_eq!(flight_plan.final_config.parallel, 4);
    assert!(flight_plan.final_config.show_progress);

    // Verify: Should have a UX info notice
    assert!(
        flight_plan
            .notices
            .iter()
            .any(|n| { n.category == "UX" && n.message.contains("Parallel") }),
        "Expected UX notice about parallel transfer with progress bars"
    );
}

#[test]
fn test_guidance_resume_vs_checksum_integrity() {
    // Test RULE 3: The Integrity Paradox (Resume vs Checksum)
    let mut config = CopyConfig::default();
    config.resume_enabled = true;
    config.verify_checksum = true;

    let flight_plan = Guidance::plan(config).unwrap();

    // Verify: Checksum should be disabled
    assert_eq!(
        flight_plan.final_config.verify_checksum, false,
        "Checksum should be disabled when resume is enabled"
    );

    // Verify: Resume should remain enabled
    assert!(flight_plan.final_config.resume_enabled);

    // Verify: Should have an integrity notice
    assert!(
        flight_plan
            .notices
            .iter()
            .any(|n| { n.category == "Integrity" && n.message.contains("Resume") }),
        "Expected integrity notice about resume and checksum"
    );
}
