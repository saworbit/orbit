/*!
 * Integration tests for Orbit
 */

use std::path::Path;
use tempfile::tempdir;

use orbit::config::{CopyConfig, CompressionType, CopyMode};
use orbit::core::{copy_file, copy_directory};

#[test]
fn test_basic_file_copy() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    std::fs::write(&source, b"Hello, Orbit!").unwrap();
    
    let config = CopyConfig::default();
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(stats.bytes_copied, 13);
    assert_eq!(std::fs::read(&dest).unwrap(), b"Hello, Orbit!");
    assert!(stats.checksum.is_some());
}

#[test]
fn test_copy_with_lz4_compression() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    // Create a file with repetitive content that compresses well
    let content = b"AAAA".repeat(1000);
    std::fs::write(&source, &content).unwrap();
    
    let mut config = CopyConfig::default();
    config.compression = CompressionType::Lz4;
    config.verify_checksum = true;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(std::fs::read(&dest).unwrap(), content);
    assert!(stats.compression_ratio.is_some());
    assert!(stats.compression_ratio.unwrap() < 100.0); // Should compress
}

#[test]
fn test_copy_with_zstd_compression() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    let content = b"Test data for Zstd compression. ".repeat(100);
    std::fs::write(&source, &content).unwrap();
    
    let mut config = CopyConfig::default();
    config.compression = CompressionType::Zstd { level: 3 };
    config.verify_checksum = true;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(std::fs::read(&dest).unwrap(), content);
    assert!(stats.compression_ratio.is_some());
}

#[test]
fn test_sync_mode_skips_unchanged() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    let content = b"original content";
    std::fs::write(&source, content).unwrap();
    std::fs::write(&dest, content).unwrap();
    
    // Record the destination's original modification time
    let dest_metadata_before = std::fs::metadata(&dest).unwrap();
    let mtime_before = dest_metadata_before.modified().unwrap();
    
    // Small delay to ensure modification time would change if file were rewritten
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    let mut config = CopyConfig::default();
    config.copy_mode = CopyMode::Sync;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    // Strong assertions: verify skip behavior
    assert_eq!(stats.files_skipped, 1, "File should have been skipped");
    assert_eq!(stats.files_copied, 0, "File should NOT have been copied");
    
    // Verify destination wasn't modified
    let dest_metadata_after = std::fs::metadata(&dest).unwrap();
    let mtime_after = dest_metadata_after.modified().unwrap();
    assert_eq!(mtime_before, mtime_after, "Destination file should not have been modified");
    
    // Verify content is still correct
    assert_eq!(std::fs::read(&dest).unwrap(), content, "File content should remain unchanged");
}

#[test]
fn test_directory_copy() {
    let temp = tempdir().unwrap();
    let source_dir = temp.path().join("source");
    let dest_dir = temp.path().join("dest");
    
    // Create source directory structure
    std::fs::create_dir(&source_dir).unwrap();
    std::fs::write(source_dir.join("file1.txt"), b"File 1").unwrap();
    std::fs::write(source_dir.join("file2.txt"), b"File 2").unwrap();
    
    let sub_dir = source_dir.join("subdir");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("file3.txt"), b"File 3").unwrap();
    
    let mut config = CopyConfig::default();
    config.recursive = true;
    
    let stats = copy_directory(&source_dir, &dest_dir, &config).unwrap();
    
    assert_eq!(stats.files_copied, 3);
    assert!(dest_dir.join("file1.txt").exists());
    assert!(dest_dir.join("file2.txt").exists());
    assert!(dest_dir.join("subdir").join("file3.txt").exists());
    
    assert_eq!(std::fs::read(dest_dir.join("file1.txt")).unwrap(), b"File 1");
}

#[test]
fn test_resume_capability() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    
    // Create a larger file
    let content = b"X".repeat(10_000);
    std::fs::write(&source, &content).unwrap();
    
    let mut config = CopyConfig::default();
    config.resume_enabled = true;
    
    // First copy should succeed
    let stats = copy_file(&source, &dest, &config).unwrap();
    assert_eq!(stats.bytes_copied, 10_000);
    
    // Resume file should be cleaned up
    let resume_file = dest.with_extension("orbit_resume");
    assert!(!resume_file.exists());
}

#[test]
fn test_exclude_patterns() {
    use orbit::core::validation::matches_exclude_pattern;
    
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
            format!("Content {}", i)
        ).unwrap();
    }
    
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.parallel = 4; // Use 4 threads
    
    let stats = copy_directory(&source_dir, &dest_dir, &config).unwrap();
    
    assert_eq!(stats.files_copied, 10);
    
    // Verify all files copied correctly
    for i in 0..10 {
        let content = std::fs::read_to_string(dest_dir.join(format!("file{}.txt", i))).unwrap();
        assert_eq!(content, format!("Content {}", i));
    }
}

#[test]
fn test_large_file_chunked_copy() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("large.bin");
    let dest = dir.path().join("large_copy.bin");
    
    // Create a 5MB file
    let content = vec![0xAB; 5 * 1024 * 1024];
    std::fs::write(&source, &content).unwrap();
    
    let mut config = CopyConfig::default();
    config.chunk_size = 64 * 1024; // 64KB chunks
    config.verify_checksum = true;
    
    let stats = copy_file(&source, &dest, &config).unwrap();
    
    assert_eq!(stats.bytes_copied, content.len() as u64);
    assert_eq!(std::fs::read(&dest).unwrap(), content);
}

#[test]
fn test_error_handling_nonexistent_source() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("nonexistent.txt");
    let dest = dir.path().join("dest.txt");
    
    let config = CopyConfig::default();
    let result = copy_file(&source, &dest, &config);
    
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), orbit::error::OrbitError::SourceNotFound(_)));
}

#[test]
fn test_compression_type_parsing() {
    use orbit::config::CompressionType;
    
    assert!(matches!(
        CompressionType::from_str("none").unwrap(),
        CompressionType::None
    ));
    
    assert!(matches!(
        CompressionType::from_str("lz4").unwrap(),
        CompressionType::Lz4
    ));
    
    assert!(matches!(
        CompressionType::from_str("zstd:5").unwrap(),
        CompressionType::Zstd { level: 5 }
    ));
    
    assert!(CompressionType::from_str("invalid").is_err());
    assert!(CompressionType::from_str("zstd:99").is_err());
}