use orbit::{
    config::{CopyConfig, CopyMode, CompressionType},
    copy_file, copy_directory,
    error::OrbitError,
    core::validation::matches_exclude_pattern,
    get_zero_copy_capabilities, is_zero_copy_available,
};
use std::path::Path;
use tempfile::tempdir;
use std::io::Write;

#[test]
fn test_copy_simple_file() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    std::fs::write(&source, b"test data").unwrap();
    
    let config = CopyConfig::default();
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(stats.bytes_copied, 9);
    assert_eq!(std::fs::read(&dest).unwrap(), b"test data");
}

#[test]
fn test_copy_nonexistent_source() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("nonexistent.txt");
    let dest = dir.path().join("dest.txt");
    
    let config = CopyConfig::default();
    let result = copy_file(&source, &dest, &config);
    
    assert!(matches!(result, Err(OrbitError::SourceNotFound(_))));
}

#[test]
fn test_compression_lz4() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    let test_data = b"This is test data that will be compressed with LZ4. ".repeat(100);
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.compression = CompressionType::Lz4;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(stats.bytes_copied, test_data.len() as u64);
    assert_eq!(std::fs::read(&dest).unwrap(), test_data);
    assert!(stats.compression_ratio.is_some());
}

#[test]
fn test_resume_capability() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    // Create a 10KB file
    let test_data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.resume_enabled = true;
    config.verify_checksum = false;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(stats.bytes_copied, 10_000);
    
    // Resume file should be cleaned up
    let resume_file = dest.with_extension("orbit_resume");
    assert!(!resume_file.exists());
}

#[test]
fn test_exclude_patterns() {
    let path = Path::new("/tmp/test.tmp");
    let patterns = vec!["*.tmp".to_string(), "*.log".to_string()];
    
    assert!(matches_exclude_pattern(path, &patterns));
    
    let path2 = Path::new("/tmp/test.txt");
    assert!(!matches_exclude_pattern(path2, &patterns));
}

#[test]
fn test_dry_run_mode() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    std::fs::write(&source, b"Test data").unwrap();
    
    let mut config = CopyConfig::default();
    config.dry_run = true;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    // File should not actually be copied in dry run
    assert!(!dest.exists());
    assert_eq!(stats.files_copied, 1); // But stats show it would have been
}

#[test]
fn test_preserve_metadata() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    std::fs::write(&source, b"Test").unwrap();
    
    let mut config = CopyConfig::default();
    config.preserve_metadata = true;
    
    copy_file(&source, &dest, &config).unwrap();
    
    let source_meta = std::fs::metadata(&source).unwrap();
    let dest_meta = std::fs::metadata(&dest).unwrap();
    
    // Timestamps should be preserved (within a small margin)
    let source_modified = source_meta.modified().unwrap();
    let dest_modified = dest_meta.modified().unwrap();
    
    // Allow 1 second difference due to filesystem precision
    let diff = source_modified.duration_since(dest_modified)
        .or_else(|_| dest_modified.duration_since(source_modified))
        .unwrap();
    
    assert!(diff.as_secs() <= 1);
}

#[test]
fn test_checksum_verification() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    std::fs::write(&source, b"Checksum test data").unwrap();
    
    let mut config = CopyConfig::default();
    config.verify_checksum = true;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert!(stats.checksum.is_some());
    assert_eq!(stats.checksum.unwrap().len(), 64); // SHA256 is 64 hex chars
}

#[test]
fn test_parallel_directory_copy() {
    let temp = tempdir().unwrap();
    let source_dir = temp.path().join("source");
    let dest_dir = temp.path().join("dest");
    
    std::fs::create_dir(&source_dir).unwrap();
    
    // Create multiple files
    for i in 0..10 {
        std::fs::write(
            source_dir.join(format!("file{}.txt", i)),
            format!("Content {}", i),
        ).unwrap();
    }
    
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.parallel = 4;
    
    let stats = copy_directory(&source_dir, &dest_dir, &config).unwrap();
    
    assert_eq!(stats.files_copied, 10);
    assert_eq!(stats.files_failed, 0);
    
    // Verify all files were copied
    for i in 0..10 {
        let dest_file = dest_dir.join(format!("file{}.txt", i));
        assert!(dest_file.exists());
        let content = std::fs::read_to_string(&dest_file).unwrap();
        assert_eq!(content, format!("Content {}", i));
    }
}

// ============================================================================
// Zero-Copy Tests
// ============================================================================

#[test]
fn test_zero_copy_capabilities_detection() {
    let caps = get_zero_copy_capabilities();
    
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        assert!(caps.available, "Zero-copy should be available on this platform");
        assert!(!caps.method.is_empty(), "Zero-copy method should be specified");
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        assert!(!caps.available, "Zero-copy should not be available on this platform");
    }
}

#[test]
fn test_zero_copy_basic_functionality() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    // Create a file large enough for zero-copy (> 64KB)
    let test_data: Vec<u8> = (0..128_000).map(|i| (i % 256) as u8).collect();
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.verify_checksum = false; // Disable to allow zero-copy path
    config.show_progress = false;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(stats.bytes_copied, test_data.len() as u64);
    assert_eq!(std::fs::read(&dest).unwrap(), test_data);
}

#[test]
fn test_zero_copy_with_post_verification() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    // Create a file large enough for zero-copy
    let test_data: Vec<u8> = (0..128_000).map(|i| (i % 256) as u8).collect();
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.verify_checksum = true; // Should do post-copy verification
    config.show_progress = false;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(stats.bytes_copied, test_data.len() as u64);
    assert!(stats.checksum.is_some(), "Checksum should be calculated");
    assert_eq!(std::fs::read(&dest).unwrap(), test_data);
}

#[test]
fn test_zero_copy_disabled_by_resume() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    let test_data: Vec<u8> = (0..128_000).map(|i| (i % 256) as u8).collect();
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.resume_enabled = true; // Should disable zero-copy
    config.show_progress = false;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    // Should still work, just with buffered copy
    assert_eq!(stats.bytes_copied, test_data.len() as u64);
    assert_eq!(std::fs::read(&dest).unwrap(), test_data);
}

#[test]
fn test_zero_copy_disabled_by_compression() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    let test_data = b"Compressible data ".repeat(1000);
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.compression = CompressionType::Lz4; // Should use compression path
    config.show_progress = false;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    // Should use compression, not zero-copy
    assert!(stats.compression_ratio.is_some());
    assert_eq!(std::fs::read(&dest).unwrap(), test_data.as_slice());
}

#[test]
fn test_zero_copy_disabled_by_bandwidth_limit() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    let test_data: Vec<u8> = (0..128_000).map(|i| (i % 256) as u8).collect();
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.max_bandwidth = 1024 * 1024; // 1 MB/s limit should disable zero-copy
    config.show_progress = false;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    // Should still work, just with buffered copy for bandwidth control
    assert_eq!(stats.bytes_copied, test_data.len() as u64);
    assert_eq!(std::fs::read(&dest).unwrap(), test_data);
}

#[test]
fn test_zero_copy_skipped_for_small_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("small.txt");
    let dest = dir.path().join("dest.txt");
    
    // Small file (< 64KB) should not use zero-copy due to syscall overhead
    let test_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.verify_checksum = false;
    config.show_progress = false;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    // Should still copy successfully using buffered path
    assert_eq!(stats.bytes_copied, test_data.len() as u64);
    assert_eq!(std::fs::read(&dest).unwrap(), test_data);
}

#[test]
fn test_zero_copy_explicit_disable() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    let test_data: Vec<u8> = (0..128_000).map(|i| (i % 256) as u8).collect();
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.use_zero_copy = false; // Explicitly disabled
    config.verify_checksum = false;
    config.show_progress = false;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    // Should use buffered copy
    assert_eq!(stats.bytes_copied, test_data.len() as u64);
    assert_eq!(std::fs::read(&dest).unwrap(), test_data);
}

#[test]
fn test_zero_copy_data_integrity() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.bin");
    let dest = dir.path().join("dest.bin");
    
    // Create binary data with patterns to detect corruption
    let mut test_data = Vec::new();
    for i in 0..256 {
        for j in 0..256 {
            test_data.push(i as u8);
            test_data.push(j as u8);
        }
    }
    std::fs::write(&source, &test_data).unwrap();
    
    let mut config = CopyConfig::default();
    config.use_zero_copy = true;
    config.verify_checksum = true; // Enable verification
    config.show_progress = false;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(stats.bytes_copied, test_data.len() as u64);
    
    // Byte-by-byte verification
    let dest_data = std::fs::read(&dest).unwrap();
    assert_eq!(dest_data.len(), test_data.len());
    assert_eq!(dest_data, test_data);
}

#[test]
fn test_config_presets() {
    // Test fast preset
    let fast = CopyConfig::fast_preset();
    assert!(fast.use_zero_copy);
    assert!(!fast.verify_checksum);
    assert!(fast.parallel > 0);
    
    // Test safe preset
    let safe = CopyConfig::safe_preset();
    assert!(!safe.use_zero_copy); // Safe preset prefers buffered for control
    assert!(safe.verify_checksum);
    assert!(safe.resume_enabled);
    
    // Test network preset
    let network = CopyConfig::network_preset();
    assert!(!network.use_zero_copy); // Network uses compression
    assert!(matches!(network.compression, CompressionType::Zstd { .. }));
}