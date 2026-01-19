/*!
 * Integration tests for delta detection and efficient transfers
 */

use orbit::{config::CopyConfig, copy_file, core::delta::CheckMode};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_delta_identical_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    // Create identical 200KB files
    let data = vec![0xAB; 200 * 1024];
    fs::write(&source, &data).unwrap();
    fs::write(&dest, &data).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        delta_block_size: 64 * 1024, // 64KB blocks
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    // Should have delta stats
    assert!(stats.delta_stats.is_some());
    let delta = stats.delta_stats.unwrap();

    // Should have very high match rate for identical files (>95%)
    assert!(delta.savings_ratio > 0.95);
    assert!(delta.blocks_matched >= delta.total_blocks - 1); // Allow for edge cases with last block

    // Verify destination is still correct
    assert_eq!(fs::read(&dest).unwrap(), data);
}

#[test]
fn test_delta_completely_different_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    // Create completely different files
    let source_data = vec![0xAA; 150 * 1024];
    let dest_data = vec![0xBB; 150 * 1024];

    fs::write(&source, &source_data).unwrap();
    fs::write(&dest, &dest_data).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        delta_block_size: 32 * 1024, // 32KB blocks
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    assert!(stats.delta_stats.is_some());
    let delta = stats.delta_stats.unwrap();

    // Very few blocks should match for completely different files
    assert!(delta.savings_ratio < 0.2); // Less than 20% savings

    // Destination should now match source
    assert_eq!(fs::read(&dest).unwrap(), source_data);
}

#[test]
fn test_delta_partial_modification() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    // Create base data
    let mut dest_data = vec![0u8; 300 * 1024];
    for (i, value) in dest_data.iter_mut().enumerate() {
        *value = (i % 256) as u8;
    }

    // Modify middle 50KB section for source
    let mut source_data = dest_data.clone();
    for value in source_data.iter_mut().take(175 * 1024).skip(125 * 1024) {
        *value = 0xFF;
    }

    fs::write(&source, &source_data).unwrap();
    fs::write(&dest, &dest_data).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        delta_block_size: 25 * 1024, // 25KB blocks
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    assert!(stats.delta_stats.is_some());
    let delta = stats.delta_stats.unwrap();

    // Should have significant block reuse (>80%)
    assert!(delta.blocks_matched > delta.total_blocks * 8 / 10);
    assert!(delta.savings_ratio > 0.8);

    // Destination should now match modified source
    assert_eq!(fs::read(&dest).unwrap(), source_data);
}

#[test]
fn test_delta_appended_data() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    // Original file
    let original = vec![0xCC; 100 * 1024];
    fs::write(&dest, &original).unwrap();

    // Appended file
    let mut appended = original.clone();
    appended.extend(vec![0xDD; 50 * 1024]);
    fs::write(&source, &appended).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        delta_block_size: 16 * 1024,
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    assert!(stats.delta_stats.is_some());
    let delta = stats.delta_stats.unwrap();

    // Original portion should match completely
    let original_blocks = (100 * 1024) / (16 * 1024);
    assert!(delta.blocks_matched >= original_blocks as u64);
    assert!(delta.savings_ratio > 0.6); // At least 60% savings

    // Verify final file
    assert_eq!(fs::read(&dest).unwrap(), appended);
}

#[test]
fn test_delta_no_destination() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    // Only create source
    let data = b"New file content for delta test";
    fs::write(&source, data).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    // When destination doesn't exist, delta can't be used
    // Falls back to regular copy (no delta stats)
    // File should still be created correctly
    assert_eq!(fs::read(&dest).unwrap(), data);
    assert_eq!(stats.files_copied, 1);
}

#[test]
fn test_check_mode_modtime() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    fs::write(&dest, b"old content").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    fs::write(&source, b"new content").unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::ModTime,
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    // ModTime mode doesn't use delta
    assert!(stats.delta_stats.is_none());
    assert_eq!(fs::read(&dest).unwrap(), b"new content");
}

#[test]
fn test_check_mode_size() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    fs::write(&dest, b"old").unwrap();
    fs::write(&source, b"newer").unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Size,
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    // Size mode doesn't use delta
    assert!(stats.delta_stats.is_none());
    assert_eq!(fs::read(&dest).unwrap(), b"newer");
}

#[test]
fn test_whole_file_flag() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    let data = vec![0xEE; 100 * 1024];
    fs::write(&source, &data).unwrap();
    fs::write(&dest, &data).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        whole_file: true, // Force full copy
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    // Should not use delta when whole_file is true
    assert!(stats.delta_stats.is_none());
}

#[test]
fn test_small_file_threshold() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    // Files < 64KB should not use delta
    let data = vec![0xFF; 32 * 1024];
    fs::write(&source, &data).unwrap();
    fs::write(&dest, &data).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    // Small files skip delta optimization
    assert!(stats.delta_stats.is_none());
}

#[test]
fn test_different_block_sizes() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    let data = vec![0xAA; 1024 * 1024]; // 1MB
    fs::write(&source, &data).unwrap();
    fs::write(&dest, &data).unwrap();

    for block_size_kb in &[64, 256, 512, 1024] {
        let config = CopyConfig {
            check_mode: CheckMode::Delta,
            delta_block_size: block_size_kb * 1024,
            ..Default::default()
        };

        let stats = copy_file(&source, &dest, &config).unwrap();

        assert!(stats.delta_stats.is_some());
        let delta = stats.delta_stats.unwrap();

        // All blocks should still match regardless of size
        assert_eq!(delta.blocks_matched, delta.total_blocks);
        assert_eq!(delta.savings_ratio, 1.0);
    }
}

#[test]
fn test_delta_with_binary_data() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.bin");
    let dest = dir.path().join("dest.bin");

    // Create binary data with patterns
    let mut dest_data = Vec::new();
    for i in 0..(200 * 1024) {
        dest_data.push(((i * 7) % 256) as u8);
    }

    // Modify some portions
    let mut source_data = dest_data.clone();
    for value in source_data.iter_mut().take(75 * 1024).skip(50 * 1024) {
        *value = !*value; // Flip bits
    }

    fs::write(&source, &source_data).unwrap();
    fs::write(&dest, &dest_data).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        delta_block_size: 32 * 1024,
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    assert!(stats.delta_stats.is_some());
    let delta = stats.delta_stats.unwrap();

    // Should have good block reuse (modified ~12.5% of data)
    assert!(delta.savings_ratio > 0.6); // At least 60% savings
    assert_eq!(fs::read(&dest).unwrap(), source_data);
}

#[test]
fn test_delta_stats_reporting() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    // Create test scenario
    let dest_data = vec![0u8; 500 * 1024];
    let mut source_data = dest_data.clone();

    // Modify 20% of the data
    for value in source_data.iter_mut().take(300 * 1024).skip(200 * 1024) {
        *value = 0xFF;
    }

    fs::write(&source, &source_data).unwrap();
    fs::write(&dest, &dest_data).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        delta_block_size: 50 * 1024,
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    assert!(stats.delta_stats.is_some());
    let delta = stats.delta_stats.unwrap();

    // Verify stats make sense
    assert_eq!(delta.total_bytes, 500 * 1024);
    assert_eq!(delta.total_blocks, 10); // 500KB / 50KB
    assert!(delta.blocks_matched >= 8); // At least 80% matched
    assert_eq!(
        delta.bytes_transferred + delta.bytes_saved,
        delta.total_bytes
    );
    assert!(delta.savings_ratio >= 0.8);
}
