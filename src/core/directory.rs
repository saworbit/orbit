/*!
 * Directory copy operations with parallel processing support
 */

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::bounded;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::config::{CopyConfig, SymlinkMode};
use crate::error::{OrbitError, Result};
use super::CopyStats;
use super::metadata::preserve_metadata;

/// Work item for parallel processing
#[derive(Clone)]
struct WorkItem {
    source_path: PathBuf,
    dest_path: PathBuf,
    entry_type: EntryType,
}

#[derive(Clone)]
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
pub fn copy_directory(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
) -> Result<CopyStats> {
    if !config.recursive {
        return Err(OrbitError::Config(
            "Recursive flag not set for directory copy".to_string()
        ));
    }

    let start_time = Instant::now();

    // Create destination directory
    if !dest_dir.exists() {
        std::fs::create_dir_all(dest_dir)?;
    }

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

    println!("Scanning and copying directory tree...");

    // Producer thread: walks directory tree and sends work items
    let producer_handle = {
        let source_dir = source_dir.clone();
        let dest_dir = dest_dir.clone();
        let config = config.clone();

        thread::spawn(move || -> Result<()> {
            produce_work_items(&source_dir, &dest_dir, &config, tx)
        })
    };

    // Consumer: process work items (files) in parallel or sequentially
    consume_work_items(
        &source_dir,
        &dest_dir,
        config,
        rx,
        total_stats.clone(),
    )?;

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

    final_stats.duration = start_time.elapsed();

    println!("\nDirectory copy completed:");
    println!("  Files copied: {}", final_stats.files_copied);
    println!("  Files skipped: {}", final_stats.files_skipped);
    println!("  Files failed: {}", final_stats.files_failed);
    println!("  Total bytes: {}", final_stats.bytes_copied);
    println!("  Duration: {:?}", final_stats.duration);

    if final_stats.files_failed > 0 {
        return Err(OrbitError::Parallel(
            format!("{} files failed to copy", final_stats.files_failed)
        ));
    }

    Ok(final_stats)
}

/// Producer: walks directory tree and sends work items via bounded channel
fn produce_work_items(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
    tx: crossbeam_channel::Sender<WorkItem>,
) -> Result<()> {
    let walker = WalkDir::new(source_dir)
        .follow_links(false)
        .same_file_system(true);

    // Process in batches for better cache locality and reduced syscalls
    let mut dir_batch = Vec::with_capacity(100);
    let mut file_batch = Vec::with_capacity(100);

    for entry in walker {
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
                eprintln!("Warning: Failed to compute relative path for {:?}", entry.path());
                continue;
            }
        };

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
                eprintln!("Warning: Failed to preserve directory metadata for {:?}: {}",
                    item.dest_path, e);
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
                if let Err(e) = process_work_item(&item, source_dir, dest_dir, config, &total_stats) {
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
            handle_symlink(&item.source_path, &item.dest_path, config.symlink_mode, config)?;
            CopyStats {
                bytes_copied: 0,
                duration: Duration::ZERO,
                checksum: None,
                compression_ratio: None,
                files_copied: 1,
                files_skipped: 0,
                files_failed: 0,
            }
        }
        EntryType::File => {
            super::copy_file(&item.source_path, &item.dest_path, config)?
        }
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
fn handle_symlink(source_path: &Path, dest_path: &Path, mode: SymlinkMode, config: &CopyConfig) -> Result<()> {
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
                source_path.parent()
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
    std::os::unix::fs::symlink(target, link_path)
        .map_err(|e| OrbitError::Symlink(e.to_string()))
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
