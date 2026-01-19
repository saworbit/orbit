/*!
 * Integration tests for delta resume handling with partial manifests
 */

use orbit::{
    config::CopyConfig,
    copy_file,
    core::delta::{CheckMode, DeltaConfig, PartialManifest},
};
use std::fs;
use std::time::SystemTime;
use tempfile::tempdir;

#[test]
fn test_partial_manifest_creation_and_load() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    // Create test files
    let data = vec![0xAB; 100 * 1024]; // 100KB
    fs::write(&source, &data).unwrap();
    fs::write(&dest, &data[..50 * 1024]).unwrap();

    let source_metadata = fs::metadata(&source).unwrap();
    let source_mtime = source_metadata.modified().unwrap();

    // Create partial manifest
    let manifest = PartialManifest::new(
        &source,
        &dest,
        64 * 1024, // 64KB chunks
        64 * 1024, // 64KB blocks
        source_metadata.len(),
        source_mtime,
        50 * 1024,
    );

    // Save and reload
    let manifest_path = PartialManifest::manifest_path_for(&dest);
    manifest.save(&manifest_path).unwrap();

    let loaded = PartialManifest::load(&manifest_path).unwrap();

    assert_eq!(loaded.source_path, source);
    assert_eq!(loaded.dest_path, dest);
    assert_eq!(loaded.chunk_size, 64 * 1024);
    assert_eq!(loaded.source_size, 100 * 1024);
    assert!(loaded.is_valid_for(&source, &dest));

    // Cleanup
    fs::remove_file(&manifest_path).unwrap();
}

#[test]
fn test_partial_manifest_validation() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");
    let other = dir.path().join("other.txt");

    let data = vec![0xAB; 100 * 1024];
    fs::write(&source, &data).unwrap();
    fs::write(&dest, &data[..50 * 1024]).unwrap();
    fs::write(&other, b"other content").unwrap();

    let source_metadata = fs::metadata(&source).unwrap();
    let source_mtime = source_metadata.modified().unwrap();

    let manifest = PartialManifest::new(
        &source,
        &dest,
        64 * 1024,
        64 * 1024,
        source_metadata.len(),
        source_mtime,
        50 * 1024,
    );

    // Valid for original paths
    assert!(manifest.is_valid_for(&source, &dest));

    // Invalid for wrong source
    assert!(!manifest.is_valid_for(&other, &dest));

    // Invalid for wrong dest
    assert!(!manifest.is_valid_for(&source, &other));
}

#[test]
fn test_partial_manifest_invalidated_on_source_change() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    let data = vec![0xAB; 100 * 1024];
    fs::write(&source, &data).unwrap();
    fs::write(&dest, &data[..50 * 1024]).unwrap();

    let source_metadata = fs::metadata(&source).unwrap();
    let source_mtime = source_metadata.modified().unwrap();

    let manifest = PartialManifest::new(
        &source,
        &dest,
        64 * 1024,
        64 * 1024,
        source_metadata.len(),
        source_mtime,
        50 * 1024,
    );

    // Valid initially
    assert!(manifest.is_valid_for(&source, &dest));

    // Modify source file size
    let new_data = vec![0xAB; 150 * 1024];
    fs::write(&source, &new_data).unwrap();

    // Should now be invalid due to size change
    assert!(!manifest.is_valid_for(&source, &dest));
}

#[test]
fn test_manifest_path_generation() {
    use std::path::Path;

    let dest = Path::new("/path/to/file.txt");
    let manifest_path = PartialManifest::manifest_path_for(dest);

    assert!(manifest_path
        .to_string_lossy()
        .contains("file.txt.delta.partial.json"));
}

#[test]
fn test_chunk_tracking() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    fs::write(&source, b"test").unwrap();
    fs::write(&dest, b"test").unwrap();

    let mut manifest = PartialManifest::new(
        &source,
        &dest,
        64 * 1024,
        64 * 1024,
        4,
        SystemTime::now(),
        4,
    );

    assert!(!manifest.is_chunk_processed(0));
    assert!(!manifest.is_chunk_processed(1));

    manifest.record_chunk(0, "abc123".to_string());

    assert!(manifest.is_chunk_processed(0));
    assert!(!manifest.is_chunk_processed(1));
    assert_eq!(manifest.processed_count(), 1);

    manifest.record_chunk(1, "def456".to_string());

    assert!(manifest.is_chunk_processed(1));
    assert_eq!(manifest.processed_count(), 2);
}

#[test]
fn test_progress_tracking() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    fs::write(&source, b"test").unwrap();
    fs::write(&dest, b"test").unwrap();

    let mut manifest = PartialManifest::new(
        &source,
        &dest,
        64 * 1024,
        64 * 1024,
        100 * 1024,
        SystemTime::now(),
        50 * 1024,
    );

    assert_eq!(manifest.diff_applied_up_to, 0);

    manifest.update_progress(50 * 1024);
    assert_eq!(manifest.diff_applied_up_to, 50 * 1024);

    manifest.update_progress(100 * 1024);
    assert_eq!(manifest.diff_applied_up_to, 100 * 1024);
}

#[test]
fn test_delta_config_resume_settings() {
    let config = DeltaConfig::default();

    // Default should have resume enabled
    assert!(config.resume_enabled);
    assert_eq!(config.chunk_size, 1024 * 1024); // 1MB default

    // Test builder pattern
    let config = DeltaConfig::default()
        .with_resume_enabled(false)
        .with_chunk_size(512 * 1024);

    assert!(!config.resume_enabled);
    assert_eq!(config.chunk_size, 512 * 1024);
}

#[test]
fn test_delta_stats_resume_fields() {
    use orbit::core::delta::DeltaStats;

    let mut stats = DeltaStats::new();

    assert_eq!(stats.chunks_resumed, 0);
    assert_eq!(stats.bytes_skipped, 0);
    assert!(!stats.was_resumed);

    stats.chunks_resumed = 10;
    stats.bytes_skipped = 1024 * 1024;
    stats.was_resumed = true;

    assert_eq!(stats.chunks_resumed, 10);
    assert_eq!(stats.bytes_skipped, 1024 * 1024);
    assert!(stats.was_resumed);
}

#[test]
fn test_copy_stats_resume_fields() {
    use orbit::CopyStats;

    let stats = CopyStats::new();

    assert_eq!(stats.chunks_resumed, 0);
    assert_eq!(stats.bytes_skipped, 0);
}

#[test]
fn test_delta_transfer_with_resume_enabled() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    // Create files > 64KB with similar content for delta
    let mut data = vec![0u8; 200 * 1024];
    for (i, value) in data.iter_mut().enumerate() {
        *value = (i % 256) as u8;
    }
    fs::write(&source, &data).unwrap();
    fs::write(&dest, &data[..180 * 1024]).unwrap(); // Slightly different

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        delta_block_size: 64 * 1024,
        delta_resume_enabled: true,
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    // Should complete successfully
    assert_eq!(stats.files_copied, 1);

    // Verify destination matches source
    assert_eq!(fs::read(&dest).unwrap(), data);

    // Manifest should be cleaned up on success
    let manifest_path = PartialManifest::manifest_path_for(&dest);
    assert!(!manifest_path.exists());
}

#[test]
fn test_delta_transfer_with_resume_disabled() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    let data = vec![0xAB; 100 * 1024];
    fs::write(&source, &data).unwrap();
    fs::write(&dest, &data[..50 * 1024]).unwrap();

    let config = CopyConfig {
        check_mode: CheckMode::Delta,
        delta_block_size: 32 * 1024,
        delta_resume_enabled: false,
        ..Default::default()
    };

    let stats = copy_file(&source, &dest, &config).unwrap();

    assert_eq!(stats.files_copied, 1);
    assert_eq!(fs::read(&dest).unwrap(), data);

    // No manifest should exist
    let manifest_path = PartialManifest::manifest_path_for(&dest);
    assert!(!manifest_path.exists());
}

#[test]
fn test_manifest_serialization_roundtrip() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let dest = dir.path().join("dest.txt");

    fs::write(&source, b"source content").unwrap();
    fs::write(&dest, b"dest content").unwrap();

    let source_metadata = fs::metadata(&source).unwrap();
    let source_mtime = source_metadata.modified().unwrap();

    let mut manifest = PartialManifest::new(
        &source,
        &dest,
        1024 * 1024,
        1024 * 1024,
        14,
        source_mtime,
        12,
    );

    manifest.record_chunk(0, "hash0".to_string());
    manifest.record_chunk(1, "hash1".to_string());
    manifest.update_progress(2048);
    manifest.checksum = Some("final_hash".to_string());

    // Save
    let manifest_path = dir.path().join("test.manifest.json");
    manifest.save(&manifest_path).unwrap();

    // Load
    let loaded = PartialManifest::load(&manifest_path).unwrap();

    assert_eq!(loaded.source_path, manifest.source_path);
    assert_eq!(loaded.dest_path, manifest.dest_path);
    assert_eq!(loaded.chunk_size, manifest.chunk_size);
    assert_eq!(loaded.processed_chunks.len(), 2);
    assert_eq!(loaded.diff_applied_up_to, 2048);
    assert_eq!(loaded.checksum, Some("final_hash".to_string()));
    assert_eq!(loaded.version, PartialManifest::VERSION);
}
