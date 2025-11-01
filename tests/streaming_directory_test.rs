use orbit::config::{CopyConfig, CopyMode, SymlinkMode};
use orbit::core::copy_directory;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_streaming_directory_copy_memory_efficiency() {
    // Create a temporary directory with many files
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let dest_dir = temp.path().join("dest");

    fs::create_dir(&source_dir).unwrap();

    // Create 1000 small files to test streaming
    println!("Creating test files...");
    for i in 0..1000 {
        let subdir = source_dir.join(format!("subdir_{}", i / 100));
        fs::create_dir_all(&subdir).unwrap();

        let file_path = subdir.join(format!("file_{}.txt", i));
        fs::write(&file_path, format!("Test content {}", i)).unwrap();
    }

    // Configure for parallel copying
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.parallel = 4;
    config.show_progress = false;

    println!("Starting streaming copy...");
    let stats = copy_directory(&source_dir, &dest_dir, &config).unwrap();

    // Verify results
    assert_eq!(stats.files_copied, 1000, "Should copy all 1000 files");
    assert_eq!(stats.files_failed, 0, "Should have no failures");

    // Verify all files exist and have correct content
    for i in 0..1000 {
        let subdir = dest_dir.join(format!("subdir_{}", i / 100));
        let file_path = subdir.join(format!("file_{}.txt", i));

        assert!(file_path.exists(), "File {} should exist", i);
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, format!("Test content {}", i));
    }

    println!("✓ Streaming directory copy test passed!");
    println!("  Files copied: {}", stats.files_copied);
    println!("  Duration: {:?}", stats.duration);
}

#[test]
fn test_streaming_handles_large_directory_tree() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("large_tree");
    let dest_dir = temp.path().join("dest");

    // Create a deeper directory structure
    fs::create_dir(&source_dir).unwrap();

    for i in 0..10 {
        let level1 = source_dir.join(format!("level1_{}", i));
        fs::create_dir(&level1).unwrap();

        for j in 0..10 {
            let level2 = level1.join(format!("level2_{}", j));
            fs::create_dir(&level2).unwrap();

            // Create a few files in each directory
            for k in 0..5 {
                let file = level2.join(format!("file_{}.txt", k));
                fs::write(&file, format!("Content {}_{}", j, k)).unwrap();
            }
        }
    }

    let mut config = CopyConfig::default();
    config.recursive = true;
    config.parallel = 2;
    config.show_progress = false;

    println!("Copying large directory tree...");
    let stats = copy_directory(&source_dir, &dest_dir, &config).unwrap();

    // 10 level1 dirs × 10 level2 dirs × 5 files = 500 files
    assert_eq!(stats.files_copied, 500);
    assert_eq!(stats.files_failed, 0);

    println!("✓ Large tree test passed!");
}

#[test]
fn test_streaming_sequential_mode() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let dest_dir = temp.path().join("dest");

    fs::create_dir(&source_dir).unwrap();

    // Create 100 files for sequential test
    for i in 0..100 {
        fs::write(
            source_dir.join(format!("file_{}.txt", i)),
            format!("Content {}", i),
        )
        .unwrap();
    }

    // Test with parallel = 0 (sequential mode)
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.parallel = 0;
    config.show_progress = false;

    let stats = copy_directory(&source_dir, &dest_dir, &config).unwrap();

    assert_eq!(stats.files_copied, 100);
    assert_eq!(stats.files_failed, 0);

    println!("✓ Sequential mode test passed!");
}

#[test]
fn test_mirror_deletes_extra_files() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let dest_dir = temp.path().join("dest");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();

    fs::write(source_dir.join("keep.txt"), "keep").unwrap();
    fs::write(dest_dir.join("keep.txt"), "old").unwrap();
    fs::write(dest_dir.join("extra.txt"), "remove").unwrap();

    let mut config = CopyConfig::default();
    config.recursive = true;
    config.copy_mode = CopyMode::Mirror;
    config.show_progress = false;

    let stats = copy_directory(&source_dir, &dest_dir, &config).unwrap();

    assert_eq!(stats.files_failed, 0);
    assert!(dest_dir.join("keep.txt").exists());
    assert!(!dest_dir.join("extra.txt").exists());
}

#[test]
fn test_mirror_respects_exclude_patterns() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let dest_dir = temp.path().join("dest");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();

    fs::write(source_dir.join("keep.txt"), "keep").unwrap();
    fs::write(dest_dir.join("keep.txt"), "old").unwrap();
    fs::write(dest_dir.join("keep.log"), "preserve").unwrap();

    let mut config = CopyConfig::default();
    config.recursive = true;
    config.copy_mode = CopyMode::Mirror;
    config.show_progress = false;
    config.exclude_patterns = vec!["*.log".to_string()];

    let stats = copy_directory(&source_dir, &dest_dir, &config).unwrap();

    assert_eq!(stats.files_failed, 0);
    assert!(dest_dir.join("keep.txt").exists());
    assert!(dest_dir.join("keep.log").exists());
}

#[cfg(unix)]
#[test]
fn test_mirror_skips_symlink_when_configured() {
    use std::os::unix::fs as unix_fs;

    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let dest_dir = temp.path().join("dest");

    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();

    fs::write(source_dir.join("keep.txt"), "keep").unwrap();
    fs::write(dest_dir.join("keep.txt"), "old").unwrap();
    fs::write(dest_dir.join("extra.txt"), "remove").unwrap();
    unix_fs::symlink("target", dest_dir.join("link"))
        .expect("failed to create symlink in destination");

    let mut config = CopyConfig::default();
    config.recursive = true;
    config.copy_mode = CopyMode::Mirror;
    config.show_progress = false;
    config.symlink_mode = SymlinkMode::Skip;

    let stats = copy_directory(&source_dir, &dest_dir, &config).unwrap();

    assert_eq!(stats.files_failed, 0);
    assert!(std::fs::symlink_metadata(dest_dir.join("link")).is_ok());
    assert!(!dest_dir.join("extra.txt").exists());
}
