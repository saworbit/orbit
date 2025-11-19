/*!
 * Comprehensive integration tests for Sync and Mirror features
 *
 * Tests cover:
 * - Basic sync/mirror operations
 * - Delta detection modes (modtime, size, checksum, delta)
 * - Filter patterns (glob, regex, file-based)
 * - Dry-run simulation
 * - Error handling and recovery
 * - Edge cases and large file handling
 */

use orbit::config::{CopyConfig, CopyMode, ErrorMode};
use orbit::core::delta::CheckMode;
use orbit::core::filter::{FilterAction, FilterList, FilterType};
use orbit::core::resilient_sync::{
    files_need_transfer, resilient_sync, ResilientSyncStats, SyncPlanner,
};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tempfile::tempdir;

/// Helper to create test directory structure
fn create_test_structure(base: &Path) {
    fs::create_dir_all(base.join("src")).unwrap();
    fs::create_dir_all(base.join("tests")).unwrap();
    fs::create_dir_all(base.join("target/debug")).unwrap();

    fs::write(base.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
    fs::write(base.join("README.md"), "# Test Project").unwrap();
    fs::write(base.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(base.join("src/lib.rs"), "pub fn lib() {}").unwrap();
    fs::write(base.join("tests/test.rs"), "#[test] fn test() {}").unwrap();
    fs::write(base.join("target/debug/binary"), "binary data").unwrap();
}

// =============================================================================
// Basic Sync Mode Tests
// =============================================================================

#[test]
fn test_sync_new_files_copied() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    create_test_structure(&src);

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    assert!(stats.files_copied >= 5); // At least 5 files
    assert!(dest.join("src/main.rs").exists());
    assert!(dest.join("Cargo.toml").exists());
}

#[test]
fn test_sync_unchanged_files_skipped() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::write(src.join("file.txt"), "content").unwrap();
    fs::write(dest.join("file.txt"), "content").unwrap();

    // Make destination newer
    let future = SystemTime::now() + Duration::from_secs(3600);
    filetime::set_file_mtime(
        dest.join("file.txt"),
        filetime::FileTime::from_system_time(future),
    )
    .unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        check_mode: CheckMode::ModTime,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    // File should be skipped since dest is newer
    assert_eq!(stats.files_copied, 0);
}

#[test]
fn test_sync_does_not_delete() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::write(src.join("source.txt"), "source").unwrap();
    fs::write(dest.join("extra.txt"), "extra").unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync, // Not Mirror!
        ..Default::default()
    };

    resilient_sync(&src, &dest, config).unwrap();

    // Extra file should NOT be deleted in Sync mode
    assert!(dest.join("extra.txt").exists());
    assert!(dest.join("source.txt").exists());
}

// =============================================================================
// Mirror Mode Tests
// =============================================================================

#[test]
fn test_mirror_deletes_extra_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::write(src.join("keep.txt"), "keep").unwrap();
    fs::write(dest.join("keep.txt"), "keep").unwrap();
    fs::write(dest.join("delete_me.txt"), "delete").unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Mirror,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    assert_eq!(stats.files_deleted, 1);
    assert!(!dest.join("delete_me.txt").exists());
    assert!(dest.join("keep.txt").exists());
}

#[test]
fn test_mirror_deletes_empty_directories() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(dest.join("empty_dir")).unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Mirror,
        ..Default::default()
    };

    resilient_sync(&src, &dest, config).unwrap();

    assert!(!dest.join("empty_dir").exists());
}

#[test]
fn test_mirror_preserves_filtered_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::write(src.join("source.txt"), "source").unwrap();
    fs::write(dest.join("protected.log"), "should stay").unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Mirror,
        exclude_patterns: vec!["*.log".to_string()],
        ..Default::default()
    };

    resilient_sync(&src, &dest, config).unwrap();

    // Excluded file should NOT be deleted
    assert!(dest.join("protected.log").exists());
}

// =============================================================================
// Check Mode Tests
// =============================================================================

#[test]
fn test_check_modtime() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dest = dir.path().join("dest.txt");

    fs::write(&src, "source").unwrap();
    fs::write(&dest, "dest").unwrap();

    // Make source newer
    std::thread::sleep(Duration::from_millis(10));
    fs::write(&src, "updated source").unwrap();

    let needs = files_need_transfer(&src, &dest, CheckMode::ModTime, 512).unwrap();
    assert!(needs, "Source is newer, should need transfer");
}

#[test]
fn test_check_size() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dest = dir.path().join("dest.txt");

    fs::write(&src, "longer content here").unwrap();
    fs::write(&dest, "short").unwrap();

    let needs = files_need_transfer(&src, &dest, CheckMode::Size, 512).unwrap();
    assert!(needs, "Sizes differ, should need transfer");

    // Same size should not need transfer
    fs::write(&dest, "longer content here").unwrap();
    let needs = files_need_transfer(&src, &dest, CheckMode::Size, 512).unwrap();
    assert!(!needs, "Same size, should not need transfer");
}

#[test]
fn test_check_checksum() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dest = dir.path().join("dest.txt");

    fs::write(&src, "content A").unwrap();
    fs::write(&dest, "content B").unwrap();

    let needs = files_need_transfer(&src, &dest, CheckMode::Checksum, 512).unwrap();
    assert!(needs, "Different content, should need transfer");

    // Same content
    fs::write(&dest, "content A").unwrap();
    let needs = files_need_transfer(&src, &dest, CheckMode::Checksum, 512).unwrap();
    assert!(!needs, "Same content, should not need transfer");
}

// =============================================================================
// Filter Tests
// =============================================================================

#[test]
fn test_filter_exclude_glob() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    create_test_structure(&src);

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        exclude_patterns: vec!["**/target/**".to_string(), "target/**".to_string()],
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    // Target directory contents should be excluded
    assert!(!dest.join("target/debug/binary").exists());
    assert!(dest.join("src/main.rs").exists());
    assert!(stats.files_copied > 0);
}

#[test]
fn test_filter_include_only_specific() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    create_test_structure(&src);

    // To achieve "include only" behavior, use include patterns with exclude all others
    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        include_patterns: vec!["**/*.rs".to_string()],
        exclude_patterns: vec!["**/*".to_string()], // Exclude everything else
        ..Default::default()
    };

    let planner = SyncPlanner::new(config).unwrap();
    let (tasks, _) = planner.plan(&src, &dest).unwrap();

    // Should only include .rs files (include patterns have priority)
    for task in &tasks {
        if let orbit::core::resilient_sync::SyncTask::Copy { source, .. } = &task.task {
            assert!(
                source.extension().map(|e| e == "rs").unwrap_or(false),
                "Expected only .rs files, got {:?}",
                source
            );
        }
    }
}

#[test]
fn test_filter_from_file() {
    let dir = tempdir().unwrap();
    let filter_file = dir.path().join(".orbitfilter");

    // Create filter file
    fs::write(
        &filter_file,
        r#"# Include Rust files
+ **/*.rs
# Exclude build artifacts
- target/**
- **/*.o
"#,
    )
    .unwrap();

    let mut filter_list = FilterList::new();
    filter_list.load_from_file(&filter_file).unwrap();

    // Test matches
    assert!(filter_list.should_include(Path::new("src/main.rs")));
    assert!(!filter_list.should_include(Path::new("target/debug/bin")));
    assert!(!filter_list.should_include(Path::new("build/output.o")));
}

#[test]
fn test_filter_regex() {
    let mut filter = FilterList::new();
    filter.add_rule(
        orbit::core::filter::FilterRule::new(
            FilterAction::Exclude,
            FilterType::Regex(r"^test_\d+\.txt$".to_string()),
        )
        .unwrap(),
    );

    assert!(!filter.should_include(Path::new("test_123.txt")));
    assert!(filter.should_include(Path::new("test_abc.txt")));
    assert!(filter.should_include(Path::new("mytest_123.txt")));
}

// =============================================================================
// Dry Run Tests
// =============================================================================

#[test]
fn test_dry_run_no_changes() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), "content").unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        dry_run: true,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    // Stats should show planned operations
    assert_eq!(stats.files_copied, 1);
    // But file should NOT exist
    assert!(!dest.join("file.txt").exists());
}

#[test]
fn test_dry_run_mirror_no_deletes() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::write(dest.join("extra.txt"), "extra").unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Mirror,
        dry_run: true,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    // Should report deletion but not actually delete
    assert_eq!(stats.files_deleted, 1);
    assert!(dest.join("extra.txt").exists());
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_error_mode_skip() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::write(src.join("good.txt"), "good").unwrap();
    fs::write(src.join("bad.txt"), "bad").unwrap();

    // Make destination unwritable (simulate error)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let bad_dest = dest.join("bad.txt");
        fs::write(&bad_dest, "locked").unwrap();
        fs::set_permissions(&bad_dest, std::fs::Permissions::from_mode(0o000)).unwrap();
    }

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        error_mode: ErrorMode::Skip,
        ..Default::default()
    };

    let result = resilient_sync(&src, &dest, config);

    // Should succeed even with errors in Skip mode
    assert!(result.is_ok());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_source() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    assert_eq!(stats.files_copied, 0);
    assert_eq!(stats.total_tasks, 0);
}

#[test]
fn test_nested_directories() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    // Create deeply nested structure
    let deep_path = src.join("a/b/c/d/e/f");
    fs::create_dir_all(&deep_path).unwrap();
    fs::write(deep_path.join("deep.txt"), "deep").unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        ..Default::default()
    };

    resilient_sync(&src, &dest, config).unwrap();

    assert!(dest.join("a/b/c/d/e/f/deep.txt").exists());
}

#[test]
fn test_unicode_filenames() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("日本語.txt"), "Japanese").unwrap();
    fs::write(src.join("émojis 🎉.txt"), "Emojis").unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    assert_eq!(stats.files_copied, 2);
    assert!(dest.join("日本語.txt").exists());
    assert!(dest.join("émojis 🎉.txt").exists());
}

#[test]
fn test_special_characters_in_path() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(src.join("path with spaces")).unwrap();
    fs::write(src.join("path with spaces/file (1).txt"), "content").unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        ..Default::default()
    };

    resilient_sync(&src, &dest, config).unwrap();

    assert!(dest.join("path with spaces/file (1).txt").exists());
}

// =============================================================================
// Statistics Tests
// =============================================================================

#[test]
fn test_stats_accuracy() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();

    // Create files with known sizes
    fs::write(src.join("small.txt"), "1234567890").unwrap(); // 10 bytes
    fs::write(src.join("medium.txt"), "x".repeat(1000)).unwrap(); // 1000 bytes

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    assert_eq!(stats.files_copied, 2);
    assert_eq!(stats.bytes_copied, 1010);
    assert_eq!(stats.completion_percent(), 100.0);
    assert!(stats.is_success());
}

#[test]
fn test_stats_completion_percentage() {
    let stats = ResilientSyncStats {
        total_tasks: 100,
        completed_tasks: 50,
        ..Default::default()
    };

    assert_eq!(stats.completion_percent(), 50.0);
}

// =============================================================================
// Non-Recursive Mode Tests
// =============================================================================

#[test]
fn test_non_recursive_mode() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(src.join("subdir")).unwrap();
    fs::write(src.join("root.txt"), "root").unwrap();
    fs::write(src.join("subdir/nested.txt"), "nested").unwrap();

    let config = CopyConfig {
        recursive: false, // Non-recursive
        copy_mode: CopyMode::Sync,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    assert_eq!(stats.files_copied, 1); // Only root.txt
    assert!(dest.join("root.txt").exists());
    assert!(!dest.join("subdir/nested.txt").exists());
}

// =============================================================================
// Update Mode Tests
// =============================================================================

#[test]
fn test_update_mode() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    // Create files
    fs::write(src.join("newer.txt"), "newer").unwrap();
    fs::write(dest.join("older.txt"), "older").unwrap();

    // Set source as newer
    let past = SystemTime::now() - Duration::from_secs(3600);
    filetime::set_file_mtime(
        dest.join("older.txt"),
        filetime::FileTime::from_system_time(past),
    )
    .unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Update,
        ..Default::default()
    };

    resilient_sync(&src, &dest, config).unwrap();

    // Only newer file should be copied
    assert!(dest.join("newer.txt").exists());
}

// =============================================================================
// Large File Tests (lightweight version)
// =============================================================================

#[test]
fn test_moderate_file_sync() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dest = dir.path().join("dest");

    fs::create_dir_all(&src).unwrap();

    // Create a moderate-sized file (100KB)
    let data = vec![0u8; 100 * 1024];
    fs::write(src.join("moderate.bin"), &data).unwrap();

    let config = CopyConfig {
        recursive: true,
        copy_mode: CopyMode::Sync,
        ..Default::default()
    };

    let stats = resilient_sync(&src, &dest, config).unwrap();

    assert_eq!(stats.files_copied, 1);
    assert_eq!(stats.bytes_copied, 100 * 1024);
    assert_eq!(
        fs::read(dest.join("moderate.bin")).unwrap().len(),
        100 * 1024
    );
}
