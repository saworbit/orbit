/*!
 * Integration tests for inclusion/exclusion filter functionality
 */

use orbit::{copy_directory, CopyConfig, CopyMode};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_basic_exclude_patterns() {
    let temp_src = TempDir::new().unwrap();
    let temp_dest = TempDir::new().unwrap();

    let src_dir = temp_src.path();
    let dest_dir = temp_dest.path();

    // Create test directory structure
    fs::create_dir_all(src_dir.join("src")).unwrap();
    fs::create_dir_all(src_dir.join("target")).unwrap();
    fs::write(src_dir.join("src/main.rs"), b"fn main() {}").unwrap();
    fs::write(src_dir.join("src/lib.rs"), b"pub fn test() {}").unwrap();
    fs::write(src_dir.join("target/debug.log"), b"logs").unwrap();
    fs::write(src_dir.join("README.md"), b"# README").unwrap();

    // Configure with exclude patterns
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.exclude_patterns = vec!["target/**".to_string(), "*.log".to_string()];
    config.show_progress = false;

    // Copy directory
    let result = copy_directory(src_dir, dest_dir, &config);
    assert!(result.is_ok());

    // Verify results
    assert!(dest_dir.join("src/main.rs").exists());
    assert!(dest_dir.join("src/lib.rs").exists());
    assert!(dest_dir.join("README.md").exists());

    // Files under target/ should be excluded (directory may exist but should be empty)
    assert!(!dest_dir.join("target/debug.log").exists());
}

#[test]
fn test_include_patterns_override_exclude() {
    let temp_src = TempDir::new().unwrap();
    let temp_dest = TempDir::new().unwrap();

    let src_dir = temp_src.path();
    let dest_dir = temp_dest.path();

    // Create test files
    fs::create_dir_all(src_dir.join("docs")).unwrap();
    fs::write(src_dir.join("test.txt"), b"test").unwrap();
    fs::write(src_dir.join("important.txt"), b"important").unwrap();
    fs::write(src_dir.join("data.json"), b"{}").unwrap();
    fs::write(src_dir.join("docs/README.txt"), b"readme").unwrap();

    // Configure with both include and exclude
    let mut config = CopyConfig::default();
    config.recursive = true;
    // Exclude all .txt files
    config.exclude_patterns = vec!["*.txt".to_string()];
    // But include important.txt and docs/**
    config.include_patterns = vec!["important.txt".to_string(), "docs/**".to_string()];
    config.show_progress = false;

    // Copy directory
    let result = copy_directory(src_dir, dest_dir, &config);
    assert!(result.is_ok());

    // Verify results
    // These should be included despite exclude pattern
    assert!(dest_dir.join("important.txt").exists());
    assert!(dest_dir.join("docs/README.txt").exists());

    // This should be excluded
    assert!(!dest_dir.join("test.txt").exists());

    // This should be included (no exclude pattern matched)
    assert!(dest_dir.join("data.json").exists());
}

#[test]
fn test_regex_patterns() {
    let temp_src = TempDir::new().unwrap();
    let temp_dest = TempDir::new().unwrap();

    let src_dir = temp_src.path();
    let dest_dir = temp_dest.path();

    // Create test files
    fs::create_dir_all(src_dir.join("src")).unwrap();
    fs::create_dir_all(src_dir.join("tests")).unwrap();
    fs::write(src_dir.join("src/main.rs"), b"fn main() {}").unwrap();
    fs::write(src_dir.join("src/lib.rs"), b"pub fn test() {}").unwrap();
    fs::write(src_dir.join("tests/test.rs"), b"#[test]").unwrap();
    fs::write(src_dir.join("build.rs"), b"fn main() {}").unwrap();

    // Configure with regex pattern to exclude test files
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.exclude_patterns = vec!["regex:^tests/.*\\.rs$".to_string()];
    config.show_progress = false;

    // Copy directory
    let result = copy_directory(src_dir, dest_dir, &config);
    assert!(result.is_ok());

    // Verify results
    assert!(dest_dir.join("src/main.rs").exists());
    assert!(dest_dir.join("src/lib.rs").exists());
    assert!(dest_dir.join("build.rs").exists());

    // This should be excluded by regex
    assert!(!dest_dir.join("tests/test.rs").exists());
}

#[test]
fn test_nested_directory_filtering() {
    let temp_src = TempDir::new().unwrap();
    let temp_dest = TempDir::new().unwrap();

    let src_dir = temp_src.path();
    let dest_dir = temp_dest.path();

    // Create nested directory structure
    fs::create_dir_all(src_dir.join("a/b/c/d")).unwrap();
    fs::create_dir_all(src_dir.join("x/y/z")).unwrap();
    fs::write(src_dir.join("a/file1.txt"), b"1").unwrap();
    fs::write(src_dir.join("a/b/file2.txt"), b"2").unwrap();
    fs::write(src_dir.join("a/b/c/file3.txt"), b"3").unwrap();
    fs::write(src_dir.join("a/b/c/d/file4.txt"), b"4").unwrap();
    fs::write(src_dir.join("x/y/z/file5.txt"), b"5").unwrap();

    // Exclude everything under a/b/c
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.exclude_patterns = vec!["a/b/c/**".to_string()];
    config.show_progress = false;

    // Copy directory
    let result = copy_directory(src_dir, dest_dir, &config);
    assert!(result.is_ok());

    // Verify results
    assert!(dest_dir.join("a/file1.txt").exists());
    assert!(dest_dir.join("a/b/file2.txt").exists());
    assert!(dest_dir.join("x/y/z/file5.txt").exists());

    // These should be excluded
    assert!(!dest_dir.join("a/b/c/file3.txt").exists());
    assert!(!dest_dir.join("a/b/c/d/file4.txt").exists());
}

#[test]
fn test_filter_from_file() {
    let temp_src = TempDir::new().unwrap();
    let temp_dest = TempDir::new().unwrap();
    let temp_filter = TempDir::new().unwrap();

    let src_dir = temp_src.path();
    let dest_dir = temp_dest.path();
    let filter_file = temp_filter.path().join("filters.txt");

    // Create filter file (order matters: first-match-wins like rsync)
    fs::write(
        &filter_file,
        "# Include important files (before excludes for higher priority)\n\
         + important.log\n\
         \n\
         # Exclude logs\n\
         - *.log\n\
         - *.tmp\n\
         \n\
         # Exclude build directory\n\
         - build/**\n",
    )
    .unwrap();

    // Create test files
    fs::create_dir_all(src_dir.join("build")).unwrap();
    fs::write(src_dir.join("app.log"), b"log").unwrap();
    fs::write(src_dir.join("important.log"), b"important").unwrap();
    fs::write(src_dir.join("data.tmp"), b"temp").unwrap();
    fs::write(src_dir.join("main.rs"), b"fn main() {}").unwrap();
    fs::write(src_dir.join("build/output.txt"), b"output").unwrap();

    // Configure with filter file
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.filter_from = Some(filter_file);
    config.show_progress = false;

    // Copy directory
    let result = copy_directory(src_dir, dest_dir, &config);
    assert!(result.is_ok());

    // Verify results
    assert!(dest_dir.join("important.log").exists()); // Included despite being .log
    assert!(dest_dir.join("main.rs").exists());

    // These should be excluded
    assert!(!dest_dir.join("app.log").exists());
    assert!(!dest_dir.join("data.tmp").exists());
    // Files under build/ should be excluded (directory may exist but should be empty)
    assert!(!dest_dir.join("build/output.txt").exists());
}

#[test]
fn test_dry_run_shows_filtered_items() {
    let temp_src = TempDir::new().unwrap();
    let temp_dest = TempDir::new().unwrap();

    let src_dir = temp_src.path();
    let dest_dir = temp_dest.path();

    // Create test files
    fs::write(src_dir.join("keep.txt"), b"keep").unwrap();
    fs::write(src_dir.join("exclude.log"), b"exclude").unwrap();

    // Configure with dry-run and exclude pattern
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.exclude_patterns = vec!["*.log".to_string()];
    config.dry_run = true;
    config.show_progress = false;

    // Copy directory (dry-run)
    let result = copy_directory(src_dir, dest_dir, &config);
    assert!(result.is_ok());

    // In dry-run, nothing should be copied
    assert!(!dest_dir.join("keep.txt").exists());
    assert!(!dest_dir.join("exclude.log").exists());
}

#[test]
fn test_mirror_mode_respects_filters() {
    let temp_src = TempDir::new().unwrap();
    let temp_dest = TempDir::new().unwrap();

    let src_dir = temp_src.path();
    let dest_dir = temp_dest.path();

    // Create source files
    fs::write(src_dir.join("keep.txt"), b"keep").unwrap();
    fs::write(src_dir.join("exclude.log"), b"exclude").unwrap();

    // Create destination with extra file
    fs::create_dir_all(dest_dir).unwrap();
    fs::write(dest_dir.join("keep.txt"), b"old").unwrap();
    fs::write(dest_dir.join("extra.txt"), b"extra").unwrap();
    fs::write(dest_dir.join("exclude.log"), b"old log").unwrap();

    // Configure mirror mode with exclude pattern
    let mut config = CopyConfig::default();
    config.copy_mode = CopyMode::Mirror;
    config.recursive = true;
    config.exclude_patterns = vec!["*.log".to_string()];
    config.show_progress = false;

    // Mirror directory
    let result = copy_directory(src_dir, dest_dir, &config);
    assert!(result.is_ok());

    // Verify results
    assert!(dest_dir.join("keep.txt").exists());

    // extra.txt should be deleted (not in source and not excluded)
    assert!(!dest_dir.join("extra.txt").exists());

    // exclude.log should be ignored by filter (neither copied nor deleted)
    assert!(dest_dir.join("exclude.log").exists()); // Old file remains
}

#[test]
fn test_path_filter_exact_match() {
    let temp_src = TempDir::new().unwrap();
    let temp_dest = TempDir::new().unwrap();

    let src_dir = temp_src.path();
    let dest_dir = temp_dest.path();

    // Create test files
    fs::create_dir_all(src_dir.join("src")).unwrap();
    fs::write(src_dir.join("Cargo.toml"), b"[package]").unwrap();
    fs::write(src_dir.join("Cargo.lock"), b"lock").unwrap();
    fs::write(src_dir.join("src/Cargo.toml"), b"wrong").unwrap();

    // Include only specific path
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.include_patterns = vec!["path:Cargo.toml".to_string()];
    config.exclude_patterns = vec!["*".to_string()]; // Exclude everything else
    config.show_progress = false;

    // Copy directory
    let result = copy_directory(src_dir, dest_dir, &config);
    assert!(result.is_ok());

    // Only the exact path should be included
    assert!(dest_dir.join("Cargo.toml").exists());

    // These should be excluded
    assert!(!dest_dir.join("Cargo.lock").exists());
    assert!(!dest_dir.join("src/Cargo.toml").exists());
}
