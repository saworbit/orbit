/*!
 * Directory copy operations with parallel processing support
 */

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::bounded;
use rayon::prelude::*;
use walkdir::WalkDir;

use super::metadata::preserve_metadata;
use super::validation::matches_exclude_pattern;
use super::filter::{FilterList, FilterDecision};
use super::disk_guardian::{self, GuardianConfig};
use super::CopyStats;
use super::progress::ProgressPublisher;
use crate::config::{CopyConfig, CopyMode, SymlinkMode};
use crate::error::{OrbitError, Result};

/// Work item for parallel processing
#[derive(Clone)]
struct WorkItem {
    source_path: PathBuf,
    dest_path: PathBuf,
    entry_type: EntryType,
}

#[derive(Clone, Debug)]
enum EntryType {
    Directory,
    File,
    Symlink,
}

/// Copy a directory recursively with streaming iteration to reduce memory usage
///
/// Uses a producer-consumer pattern with bounded channels to:
/// - Prevent scanner from overwhelming copiers
/// - Support parallel file copying
/// - Minimize memory footprint for large directory trees
///
/// If `publisher` is provided via the internal implementation, progress events will be emitted.
pub fn copy_directory(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
) -> Result<CopyStats> {
    copy_directory_impl(source_dir, dest_dir, config, None)
}

/// Internal implementation of copy_directory with optional progress publisher
pub fn copy_directory_impl(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
    publisher: Option<&ProgressPublisher>,
) -> Result<CopyStats> {
    if !config.recursive {
        return Err(OrbitError::Config(
            "Recursive flag not set for directory copy".to_string()
        ));
    }

    let start_time = Instant::now();

    // Pre-flight disk space check for directory transfers
    if config.show_progress {
        println!("Performing pre-flight checks...");
    }

    let guardian_config = GuardianConfig::default();
    let estimated_size = disk_guardian::estimate_directory_size(source_dir)?;

    if config.show_progress {
        println!("Estimated transfer size: {} bytes", estimated_size);
    }

    disk_guardian::ensure_transfer_safety(dest_dir, estimated_size, &guardian_config)?;

    // Use provided publisher or create a no-op one
    let noop_publisher = ProgressPublisher::noop();
    let pub_ref = publisher.unwrap_or(&noop_publisher);

    // Emit directory scan start event
    pub_ref.publish(super::progress::ProgressEvent::DirectoryScanStart {
        path: source_dir.to_path_buf(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    });

    // Create destination directory
    if !dest_dir.exists() {
        std::fs::create_dir_all(dest_dir)?;
    }

    // Build filter list from config
    let filter_list = match FilterList::from_config(
        &config.include_patterns,
        &config.exclude_patterns,
        config.filter_from.as_deref(),
    ) {
        Ok(filters) => Arc::new(filters),
        Err(e) => {
            return Err(OrbitError::Config(format!("Invalid filter configuration: {}", e)));
        }
    };

    // Bounded channel prevents scanner from overwhelming copiers
    // Buffer size: use parallel threads as baseline, bounded between 16-1000
    let buffer_size = if config.parallel > 0 {
        config.parallel.max(16).min(1000)
    } else {
        100
    };

    let (tx, rx) = bounded::<WorkItem>(buffer_size);

    let source_dir = source_dir.to_path_buf();
    let dest_dir = dest_dir.to_path_buf();
    let total_stats = Arc::new(Mutex::new(CopyStats::new()));
    let expected_entries = Arc::new(Mutex::new(HashSet::new()));

    println!("Scanning and copying directory tree...");

    // Producer thread: walks directory tree and sends work items
    let producer_handle = {
        let source_dir = source_dir.clone();
        let dest_dir = dest_dir.clone();
        let config = config.clone();
        let expected_entries = expected_entries.clone();
        let filter_list = filter_list.clone();

        thread::spawn(move || -> Result<()> {
            produce_work_items(&source_dir, &dest_dir, &config, tx, expected_entries, &filter_list)
        })
    };

    // Consumer: process work items (files) in parallel or sequentially
    consume_work_items(&source_dir, &dest_dir, config, rx, total_stats.clone())?;

    // Wait for producer to finish and check for errors
    match producer_handle.join() {
        Ok(Ok(())) => {
            // Producer finished successfully
        }
        Ok(Err(e)) => {
            eprintln!("Producer thread error: {}", e);
        }
        Err(e) => {
            eprintln!("Producer thread panicked: {:?}", e);
        }
    }

    let mut final_stats = match Arc::try_unwrap(total_stats) {
        Ok(mutex) => mutex.into_inner().unwrap(),
        Err(arc) => arc.lock().unwrap().clone(),
    };

    let mut deleted_count = 0;
    if config.copy_mode == CopyMode::Mirror {
        match collect_deletion_candidates(&dest_dir, &expected_entries, config, &filter_list) {
            Ok(deletions) => {
                let summary = apply_deletions(&deletions, config);
                deleted_count = summary.deleted as u64;
                final_stats.files_failed += summary.failed as u64;
            }
            Err(e) => {
                eprintln!("Failed to scan destination for deletions: {}", e);
                final_stats.files_failed += 1;
            }
        }
    }

    final_stats.duration = start_time.elapsed();

    // Emit directory scan complete event
    let total_files = final_stats.files_copied + final_stats.files_skipped + final_stats.files_failed;
    pub_ref.publish(super::progress::ProgressEvent::DirectoryScanComplete {
        total_files,
        total_dirs: 0, // We don't track directories separately in current implementation
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    });

    // Emit batch complete event
    pub_ref.publish(super::progress::ProgressEvent::BatchComplete {
        files_succeeded: final_stats.files_copied,
        files_failed: final_stats.files_failed,
        total_bytes: final_stats.bytes_copied,
        duration_ms: final_stats.duration.as_millis() as u64,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    });

    println!("\nDirectory copy completed:");
    println!("  Files copied: {}", final_stats.files_copied);
    println!("  Files skipped: {}", final_stats.files_skipped);
    println!("  Files failed: {}", final_stats.files_failed);
    println!("  Total bytes: {}", final_stats.bytes_copied);
    println!("  Duration: {:?}", final_stats.duration);
    if config.copy_mode == CopyMode::Mirror {
        println!("  Files deleted: {}", deleted_count);
    }

    if final_stats.files_failed > 0 {
        return Err(OrbitError::Parallel(format!(
            "{} files failed to copy",
            final_stats.files_failed
        )));
    }

    Ok(final_stats)
}

/// Producer: walks directory tree and sends work items via bounded channel
fn produce_work_items(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
    tx: crossbeam_channel::Sender<WorkItem>,
    expected_entries: Arc<Mutex<HashSet<PathBuf>>>,
    filter_list: &FilterList,
) -> Result<()> {
    let mut walker = WalkDir::new(source_dir)
        .follow_links(false)
        .same_file_system(true)
        .into_iter();

    // Process in batches for better cache locality and reduced syscalls
    let mut dir_batch = Vec::with_capacity(100);
    let mut file_batch = Vec::with_capacity(100);

    while let Some(entry) = walker.next() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Failed to read entry: {}", e);
                continue;
            }
        };

        let relative_path = match entry.path().strip_prefix(source_dir) {
            Ok(p) => p,
            Err(_) => {
                eprintln!(
                    "Warning: Failed to compute relative path for {:?}",
                    entry.path()
                );
                continue;
            }
        };

        if relative_path.as_os_str().is_empty() {
            continue;
        }

        // Apply filter rules (first-match-wins with include/exclude)
        let should_process = if !filter_list.is_empty() {
            // If we have filters configured, use them exclusively
            filter_list.should_include(relative_path)
        } else {
            // No filters configured - use old exclude pattern matching for backward compatibility
            !matches_exclude_pattern(relative_path, &config.exclude_patterns)
        };

        if !should_process {
            // Show filtered items in dry-run mode
            if config.dry_run {
                println!("Filtered out: {}", relative_path.display());
            }

            if entry.file_type().is_dir() {
                walker.skip_current_dir();
            }
            continue;
        }

        let dest_path = dest_dir.join(relative_path);
        let source_path = entry.path().to_path_buf();

        let entry_type = if entry.file_type().is_dir() {
            EntryType::Directory
        } else if entry.file_type().is_symlink() {
            EntryType::Symlink
        } else if entry.file_type().is_file() {
            EntryType::File
        } else {
            continue; // Skip special files
        };

        let work_item = WorkItem {
            source_path,
            dest_path,
            entry_type: entry_type.clone(),
        };

        if let Ok(mut expected) = expected_entries.lock() {
            expected.insert(relative_path.to_path_buf());
        }

        // Batch directories and files separately
        match entry_type {
            EntryType::Directory => {
                dir_batch.push(work_item);
                if dir_batch.len() >= 100 {
                    flush_directory_batch(&mut dir_batch, config)?;
                }
            }
            EntryType::File | EntryType::Symlink => {
                file_batch.push(work_item);
                if file_batch.len() >= 100 {
                    flush_file_batch(&mut file_batch, &tx)?;
                }
            }
        }
    }

    // Flush remaining batches
    flush_directory_batch(&mut dir_batch, config)?;
    flush_file_batch(&mut file_batch, &tx)?;

    // Channel will be dropped here, signaling consumers to finish
    Ok(())
}

#[derive(Debug)]
struct DeletionItem {
    path: PathBuf,
    entry_type: EntryType,
}

#[derive(Default)]
struct DeletionSummary {
    deleted: usize,
    failed: usize,
}

fn collect_deletion_candidates(
    dest_dir: &Path,
    expected_entries: &Arc<Mutex<HashSet<PathBuf>>>,
    config: &CopyConfig,
    filter_list: &FilterList,
) -> Result<Vec<DeletionItem>> {
    let expected: HashSet<PathBuf> = expected_entries.lock().unwrap().iter().cloned().collect();
    let mut deletions = Vec::new();
    let mut walker = WalkDir::new(dest_dir)
        .follow_links(false)
        .same_file_system(true)
        .contents_first(true)
        .into_iter();

    while let Some(entry) = walker.next() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Failed to read destination entry: {}", e);
                continue;
            }
        };

        if entry.depth() == 0 {
            continue;
        }

        let relative_path = match entry.path().strip_prefix(dest_dir) {
            Ok(p) => p,
            Err(_) => {
                eprintln!(
                    "Warning: Failed to compute relative destination path for {:?}",
                    entry.path()
                );
                continue;
            }
        };

        // Apply filter rules (same logic as in produce_work_items)
        let should_process = if !filter_list.is_empty() {
            filter_list.should_include(relative_path)
        } else {
            !matches_exclude_pattern(relative_path, &config.exclude_patterns)
        };

        if !should_process {
            if entry.file_type().is_dir() {
                walker.skip_current_dir();
            }
            continue;
        }

        if entry.file_type().is_symlink() && config.symlink_mode == SymlinkMode::Skip {
            continue;
        }

        if expected.contains(relative_path) {
            continue;
        }

        let entry_type = if entry.file_type().is_dir() {
            EntryType::Directory
        } else if entry.file_type().is_symlink() {
            EntryType::Symlink
        } else if entry.file_type().is_file() {
            EntryType::File
        } else {
            continue;
        };

        deletions.push(DeletionItem {
            path: entry.path().to_path_buf(),
            entry_type,
        });
    }

    Ok(deletions)
}

fn apply_deletions(deletions: &[DeletionItem], config: &CopyConfig) -> DeletionSummary {
    let mut summary = DeletionSummary::default();

    for item in deletions {
        if config.dry_run {
            println!("Would delete: {:?}", item.path);
            summary.deleted += 1;
            continue;
        }

        let result = match item.entry_type {
            EntryType::Directory => std::fs::remove_dir_all(&item.path),
            EntryType::File | EntryType::Symlink => std::fs::remove_file(&item.path),
        };

        match result {
            Ok(_) => {
                summary.deleted += 1;
            }
            Err(e) => {
                eprintln!("Failed to delete {:?}: {}", item.path, e);
                summary.failed += 1;
            }
        }
    }

    summary
}

/// Flush directory batch - create directories sequentially before files
fn flush_directory_batch(
    batch: &mut Vec<WorkItem>,
    config: &CopyConfig,
) -> Result<()> {
    for item in batch.drain(..) {
        if !item.dest_path.exists() {
            std::fs::create_dir_all(&item.dest_path)?;
        }

        if config.preserve_metadata {
            if let Err(e) = preserve_metadata(&item.source_path, &item.dest_path) {
                eprintln!(
                    "Warning: Failed to preserve directory metadata for {:?}: {}",
                    item.dest_path, e
                );
            }
        }
    }
    Ok(())
}

/// Flush file batch - send to workers via channel (blocks if channel full = backpressure)
fn flush_file_batch(
    batch: &mut Vec<WorkItem>,
    tx: &crossbeam_channel::Sender<WorkItem>,
) -> Result<()> {
    for item in batch.drain(..) {
        // This will block if channel is full, providing natural backpressure
        if tx.send(item).is_err() {
            // Channel closed, stop sending
            break;
        }
    }
    Ok(())
}

/// Consumer: process work items in parallel or sequentially
fn consume_work_items(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
    rx: crossbeam_channel::Receiver<WorkItem>,
    total_stats: Arc<Mutex<CopyStats>>,
) -> Result<()> {
    if config.parallel > 0 {
        // Parallel processing with thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.parallel)
            .build()
            .map_err(|e| OrbitError::Parallel(e.to_string()))?;

        pool.install(|| {
            rx.into_iter().par_bridge().for_each(|item| {
                if let Err(e) = process_work_item(&item, source_dir, dest_dir, config, &total_stats)
                {
                    eprintln!("Error copying {:?}: {}", item.source_path, e);
                    if let Ok(mut stats) = total_stats.lock() {
                        stats.files_failed += 1;
                    }
                }
            });
        });
    } else {
        // Sequential processing
        for item in rx {
            if let Err(e) = process_work_item(&item, source_dir, dest_dir, config, &total_stats) {
                eprintln!("Error copying {:?}: {}", item.source_path, e);
                if let Ok(mut stats) = total_stats.lock() {
                    stats.files_failed += 1;
                }
            }
        }
    }

    Ok(())
}

/// Process a single work item (file or symlink)
fn process_work_item(
    item: &WorkItem,
    _source_dir: &Path,
    _dest_dir: &Path,
    config: &CopyConfig,
    stats_mutex: &Arc<Mutex<CopyStats>>,
) -> Result<()> {
    let stats = match item.entry_type {
        EntryType::Directory => {
            // Directories already handled in producer
            return Ok(());
        }
        EntryType::Symlink => {
            handle_symlink(
                &item.source_path,
                &item.dest_path,
                config.symlink_mode,
                config,
            )?;
            CopyStats {
                bytes_copied: 0,
                duration: Duration::ZERO,
                checksum: None,
                compression_ratio: None,
                files_copied: 1,
                files_skipped: 0,
                files_failed: 0,
        delta_stats: None,
            }
        }
        EntryType::File => super::copy_file(&item.source_path, &item.dest_path, config)?
    };

    // Update total stats atomically
    if let Ok(mut total_stats) = stats_mutex.lock() {
        total_stats.bytes_copied += stats.bytes_copied;
        total_stats.files_copied += stats.files_copied;
        total_stats.files_skipped += stats.files_skipped;
    }

    Ok(())
}

/// Handle symbolic link based on mode
fn handle_symlink(
    source_path: &Path,
    dest_path: &Path,
    mode: SymlinkMode,
    config: &CopyConfig,
) -> Result<()> {
    match mode {
        SymlinkMode::Skip => {
            println!("Skipping symlink: {:?}", source_path);
            Ok(())
        }
        SymlinkMode::Follow => {
            let target = std::fs::read_link(source_path)?;
            let resolved = if target.is_absolute() {
                target
            } else {
                source_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(target)
            };

            if resolved.is_file() {
                super::copy_file(&resolved, dest_path, config)?;
            }
            Ok(())
        }
        SymlinkMode::Preserve => {
            let target = std::fs::read_link(source_path)?;
            create_symlink(&target, dest_path)
        }
    }
}

/// Create a symbolic link (cross-platform)
#[cfg(unix)]
fn create_symlink(target: &Path, link_path: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link_path).map_err(|e| OrbitError::Symlink(e.to_string()))
}

#[cfg(windows)]
fn create_symlink(target: &Path, link_path: &Path) -> Result<()> {
    if target.is_file() {
        std::os::windows::fs::symlink_file(target, link_path)
            .map_err(|e| OrbitError::Symlink(e.to_string()))
    } else {
        std::os::windows::fs::symlink_dir(target, link_path)
            .map_err(|e| OrbitError::Symlink(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mirror_config() -> CopyConfig {
        CopyConfig {
            copy_mode: CopyMode::Mirror,
            recursive: true,
            show_progress: false,
            ..CopyConfig::default()
        }
    }

    #[test]
    fn collect_deletion_candidates_flags_extras() {
        let temp = TempDir::new().unwrap();
        let dest_dir = temp.path();

        std::fs::create_dir_all(dest_dir).unwrap();
        std::fs::write(dest_dir.join("keep.txt"), b"keep").unwrap();
        std::fs::write(dest_dir.join("extra.txt"), b"extra").unwrap();

        let expected = Arc::new(Mutex::new(HashSet::new()));
        expected.lock().unwrap().insert(PathBuf::from("keep.txt"));

        let config = mirror_config();
        let filter_list = FilterList::new();
        let deletions = collect_deletion_candidates(dest_dir, &expected, &config, &filter_list).unwrap();

        assert_eq!(deletions.len(), 1);
        assert_eq!(deletions[0].path, dest_dir.join("extra.txt"));
        assert!(matches!(deletions[0].entry_type, EntryType::File));
    }

    #[test]
    fn collect_deletion_candidates_respects_excludes() {
        let temp = TempDir::new().unwrap();
        let dest_dir = temp.path();

        std::fs::create_dir_all(dest_dir).unwrap();
        std::fs::write(dest_dir.join("skip.log"), b"log").unwrap();

        let expected = Arc::new(Mutex::new(HashSet::new()));

        let mut config = mirror_config();
        config.exclude_patterns = vec!["*.log".to_string()];

        let filter_list = FilterList::new();
        let deletions = collect_deletion_candidates(dest_dir, &expected, &config, &filter_list).unwrap();

        assert!(deletions.is_empty());
    }

    #[test]
    fn apply_deletions_removes_files() {
        let temp = TempDir::new().unwrap();
        let dest_dir = temp.path();

        std::fs::create_dir_all(dest_dir).unwrap();
        let victim = dest_dir.join("victim.txt");
        std::fs::write(&victim, b"bye").unwrap();

        let deletions = vec![DeletionItem {
            path: victim.clone(),
            entry_type: EntryType::File,
        }];

        let summary = apply_deletions(&deletions, &mirror_config());

        assert_eq!(summary.deleted, 1);
        assert!(summary.failed == 0);
        assert!(!victim.exists());
    }

    #[cfg(unix)]
    #[test]
    fn collect_deletion_candidates_honors_symlink_skip() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().unwrap();
        let dest_dir = temp.path();

        std::fs::create_dir_all(dest_dir).unwrap();
        unix_fs::symlink("target", dest_dir.join("link")).unwrap();

        let expected = Arc::new(Mutex::new(HashSet::new()));

        let mut config = mirror_config();
        config.symlink_mode = SymlinkMode::Skip;

        let filter_list = FilterList::new();
        let deletions = collect_deletion_candidates(dest_dir, &expected, &config, &filter_list).unwrap();

        assert!(deletions.is_empty());
    }
}
