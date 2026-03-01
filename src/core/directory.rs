/*!
 * Directory copy operations with parallel processing support
 */

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use crossbeam_channel::bounded;
use rayon::prelude::*;
use tracing::info;
use walkdir::WalkDir;

use super::batch::{record_create_file, JournalEntry, TransferJournal};
use super::concurrency::ConcurrencyLimiter;
use super::disk_guardian::{self, GuardianConfig};
use super::filter::FilterList;
use super::hardlink::{create_hardlink, HardlinkTracker};
use super::metadata::preserve_metadata;
use super::progress::ProgressPublisher;
use super::validation::matches_exclude_pattern;
use super::CopyStats;
use crate::audit::AuditLogger;
use crate::config::{CopyConfig, CopyMode, SymlinkMode};
use crate::core::checksum::calculate_checksum;
use crate::error::{OrbitError, Result};

/// Work item for parallel processing
#[derive(Clone)]
struct WorkItem {
    source_path: PathBuf,
    dest_path: PathBuf,
    relative_path: PathBuf,
    entry_type: EntryType,
}

#[derive(Clone, Debug)]
enum EntryType {
    Directory,
    File,
    Symlink,
    Hardlink { target: PathBuf },
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

/// Generate a unique job ID for audit event correlation
fn generate_job_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("orbit-dir-{:x}-{:04x}", timestamp, rand::random::<u16>())
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
            "Recursive flag not set for directory copy".to_string(),
        ));
    }

    if config.read_batch.is_some() {
        return Err(OrbitError::Config(
            "Batch replay is not supported in directory copy; use --read-batch at the CLI"
                .to_string(),
        ));
    }

    if config.write_batch.is_some() && config.copy_mode != CopyMode::Copy {
        return Err(OrbitError::Config(
            "--write-batch is only supported with --mode copy".to_string(),
        ));
    }

    let start_time = Instant::now();
    let job_id = generate_job_id();

    // Initialize audit logger if audit log path is configured
    let audit_logger: Option<Arc<Mutex<AuditLogger>>> =
        if config.audit_log_path.is_some() || config.verbose {
            match AuditLogger::new(config.audit_log_path.as_deref(), config.audit_format) {
                Ok(logger) => Some(Arc::new(Mutex::new(logger))),
                Err(e) => {
                    tracing::warn!("Failed to initialize audit logger: {}", e);
                    None
                }
            }
        } else {
            None
        };

    // Setup concurrency limiter for controlling parallel transfers
    let concurrency_limiter = if config.parallel > 0 {
        let limiter = ConcurrencyLimiter::new(config.parallel);
        info!(
            "Concurrency control enabled: max {} parallel transfers",
            limiter.max_concurrent()
        );
        Some(Arc::new(limiter))
    } else {
        None
    };

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

    // Emit start audit event
    if let Some(ref logger) = audit_logger {
        if let Ok(mut log) = logger.lock() {
            if let Err(e) = log.emit_start(&job_id, source_dir, dest_dir, "local", estimated_size) {
                tracing::warn!("Failed to emit audit start event: {}", e);
            }
        }
    }

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

    let rename_index = if config.detect_renames {
        Some(Arc::new(build_hash_index(dest_dir)?))
    } else {
        None
    };

    let batch_journal = if config.write_batch.is_some() {
        Some(Arc::new(Mutex::new(TransferJournal::new(
            source_dir.to_path_buf(),
            dest_dir.to_path_buf(),
        ))))
    } else {
        None
    };

    // Build filter list from config
    let filter_list = match FilterList::from_config(
        &config.include_patterns,
        &config.exclude_patterns,
        config.filter_from.as_deref(),
    ) {
        Ok(filters) => Arc::new(filters),
        Err(e) => {
            return Err(OrbitError::Config(format!(
                "Invalid filter configuration: {}",
                e
            )));
        }
    };

    // Bounded channel prevents scanner from overwhelming copiers
    // Buffer size: use parallel threads as baseline, bounded between 16-1000
    let buffer_size = if config.parallel > 0 {
        config.parallel.clamp(16, 1000)
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
            produce_work_items(
                &source_dir,
                &dest_dir,
                &config,
                tx,
                expected_entries,
                &filter_list,
            )
        })
    };

    // Consumer: process work items (files) in parallel or sequentially
    consume_work_items(
        &source_dir,
        &dest_dir,
        config,
        rx,
        total_stats.clone(),
        concurrency_limiter.as_ref(),
        rename_index.as_ref(),
        batch_journal.as_ref(),
    )?;

    // Wait for producer to finish and check for errors
    match producer_handle.join() {
        Ok(Ok(())) => {
            // Producer finished successfully
        }
        Ok(Err(e)) => {
            tracing::error!("Producer thread error: {}", e);
            return Err(e);
        }
        Err(e) => {
            return Err(OrbitError::Parallel(format!(
                "Producer thread panicked: {:?}",
                e
            )));
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
                tracing::error!("Failed to scan destination for deletions: {}", e);
                final_stats.files_failed += 1;
            }
        }
    }

    if let (Some(batch_path), Some(journal)) = (config.write_batch.as_ref(), batch_journal.as_ref())
    {
        let journal = journal.lock().unwrap();
        journal.save(batch_path).map_err(OrbitError::Io)?;
    }

    final_stats.duration = start_time.elapsed();

    // Emit directory scan complete event
    let total_files =
        final_stats.files_copied + final_stats.files_skipped + final_stats.files_failed;
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

    // Emit completion audit event
    if let Some(ref logger) = audit_logger {
        if let Ok(mut log) = logger.lock() {
            let error_msg = if final_stats.files_failed > 0 {
                Some(format!("{} files failed to copy", final_stats.files_failed))
            } else {
                None
            };
            let _ = log.emit_from_stats(
                &job_id,
                &source_dir,
                &dest_dir,
                "local",
                &final_stats,
                config.compression,
                0,
                error_msg.as_deref(),
            );
        }
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
#[allow(clippy::while_let_on_iterator)]
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
    let mut hardlink_tracker = if config.preserve_hardlinks {
        Some(HardlinkTracker::new())
    } else {
        None
    };

    while let Some(entry) = walker.next() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read entry: {}", e);
                continue;
            }
        };

        let relative_path = match entry.path().strip_prefix(source_dir) {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!("Failed to compute relative path for {:?}", entry.path());
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

        let mut entry_type = if entry.file_type().is_dir() {
            EntryType::Directory
        } else if entry.file_type().is_symlink() {
            EntryType::Symlink
        } else if entry.file_type().is_file() {
            EntryType::File
        } else {
            continue; // Skip special files
        };

        if matches!(entry_type, EntryType::File) {
            if let Some(ref mut tracker) = hardlink_tracker {
                if let Ok(metadata) = entry.metadata() {
                    if let Some(original_path) = tracker.check(entry.path(), &metadata) {
                        if let Ok(original_rel) = original_path.strip_prefix(source_dir) {
                            let original_dest = dest_dir.join(original_rel);
                            entry_type = EntryType::Hardlink {
                                target: original_dest,
                            };
                        }
                    }
                }
            }
        }

        let work_item = WorkItem {
            source_path,
            dest_path,
            relative_path: relative_path.to_path_buf(),
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
            EntryType::File | EntryType::Symlink | EntryType::Hardlink { .. } => {
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

#[allow(clippy::while_let_on_iterator)]
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
                tracing::warn!("Failed to read destination entry: {}", e);
                continue;
            }
        };

        if entry.depth() == 0 {
            continue;
        }

        let relative_path = match entry.path().strip_prefix(dest_dir) {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(
                    "Failed to compute relative destination path for {:?}",
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
            EntryType::File | EntryType::Symlink | EntryType::Hardlink { .. } => {
                std::fs::remove_file(&item.path)
            }
        };

        match result {
            Ok(_) => {
                summary.deleted += 1;
            }
            Err(e) => {
                tracing::error!("Failed to delete {:?}: {}", item.path, e);
                summary.failed += 1;
            }
        }
    }

    summary
}

fn build_hash_index(dest_dir: &Path) -> Result<HashMap<String, PathBuf>> {
    let mut index = HashMap::new();
    let walker = WalkDir::new(dest_dir)
        .follow_links(false)
        .same_file_system(true)
        .into_iter();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read destination entry: {}", e);
                continue;
            }
        };

        if entry.depth() == 0 {
            continue;
        }

        if entry.file_type().is_symlink() {
            continue;
        }

        if entry.file_type().is_dir() {
            continue;
        }

        let path = entry.path().to_path_buf();
        match calculate_checksum(&path) {
            Ok(hash) => {
                index.entry(hash).or_insert(path);
            }
            Err(e) => {
                tracing::warn!("Failed to hash {:?}: {}", path, e);
            }
        }
    }

    Ok(index)
}

/// Flush directory batch - create directories sequentially before files
fn flush_directory_batch(batch: &mut Vec<WorkItem>, config: &CopyConfig) -> Result<()> {
    for item in batch.drain(..) {
        if !item.dest_path.exists() {
            std::fs::create_dir_all(&item.dest_path)?;
        }

        if config.preserve_metadata {
            if let Err(e) = preserve_metadata(&item.source_path, &item.dest_path) {
                tracing::warn!(
                    "Failed to preserve directory metadata for {:?}: {}",
                    item.dest_path,
                    e
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
#[allow(clippy::too_many_arguments)]
fn consume_work_items(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
    rx: crossbeam_channel::Receiver<WorkItem>,
    total_stats: Arc<Mutex<CopyStats>>,
    concurrency_limiter: Option<&Arc<ConcurrencyLimiter>>,
    rename_index: Option<&Arc<HashMap<String, PathBuf>>>,
    batch_journal: Option<&Arc<Mutex<TransferJournal>>>,
) -> Result<()> {
    if config.parallel > 0 {
        // Parallel processing with thread pool and concurrency control
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.parallel)
            .build()
            .map_err(|e| OrbitError::Parallel(e.to_string()))?;

        pool.install(|| {
            rx.into_iter().par_bridge().for_each(|item| {
                if let Err(e) = process_work_item(
                    &item,
                    source_dir,
                    dest_dir,
                    config,
                    &total_stats,
                    concurrency_limiter,
                    rename_index,
                    batch_journal,
                ) {
                    tracing::error!("Error copying {:?}: {}", item.source_path, e);
                    if let Ok(mut stats) = total_stats.lock() {
                        stats.files_failed += 1;
                    }
                }
            });
        });
    } else {
        // Sequential processing (no concurrency limiter needed)
        for item in rx {
            if let Err(e) = process_work_item(
                &item,
                source_dir,
                dest_dir,
                config,
                &total_stats,
                None,
                rename_index,
                batch_journal,
            ) {
                tracing::error!("Error copying {:?}: {}", item.source_path, e);
                if let Ok(mut stats) = total_stats.lock() {
                    stats.files_failed += 1;
                }
            }
        }
    }

    Ok(())
}

/// Process a single work item (file or symlink)
#[allow(clippy::too_many_arguments)]
fn process_work_item(
    item: &WorkItem,
    _source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
    stats_mutex: &Arc<Mutex<CopyStats>>,
    concurrency_limiter: Option<&Arc<ConcurrencyLimiter>>,
    rename_index: Option<&Arc<HashMap<String, PathBuf>>>,
    batch_journal: Option<&Arc<Mutex<TransferJournal>>>,
) -> Result<()> {
    // Acquire concurrency permit if limiter is provided
    // Permit is automatically released when dropped (RAII pattern)
    let _permit = concurrency_limiter.map(|limiter| limiter.acquire());
    let mut file_config = config.clone();
    file_config.detect_renames = false;
    file_config.link_dest.clear();
    file_config.write_batch = None;
    file_config.read_batch = None;

    let stats = match &item.entry_type {
        EntryType::Directory => {
            // Directories already handled in producer
            return Ok(());
        }
        EntryType::Symlink => {
            handle_symlink(
                &item.source_path,
                &item.dest_path,
                config.symlink_mode,
                &file_config,
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
                chunks_resumed: 0,
                bytes_skipped: 0,
            }
        }
        EntryType::Hardlink { target } => {
            if let Some(parent) = item.dest_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            let mut linked = false;
            if target.exists() {
                match create_hardlink(target, &item.dest_path) {
                    Ok(()) => {
                        linked = true;
                        if let Some(journal) = batch_journal {
                            let target_rel = target.strip_prefix(dest_dir).unwrap_or(target);
                            let link_rel = item
                                .dest_path
                                .strip_prefix(dest_dir)
                                .unwrap_or(&item.dest_path);
                            journal
                                .lock()
                                .unwrap()
                                .record(JournalEntry::CreateHardlink {
                                    target: target_rel.to_path_buf(),
                                    link: link_rel.to_path_buf(),
                                });
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to create hardlink {:?} -> {:?}: {}. Falling back to copy.",
                            target,
                            item.dest_path,
                            e
                        );
                    }
                }
            }

            if linked {
                CopyStats {
                    bytes_copied: 0,
                    duration: Duration::ZERO,
                    checksum: None,
                    compression_ratio: None,
                    files_copied: 1,
                    files_skipped: 0,
                    files_failed: 0,
                    delta_stats: None,
                    chunks_resumed: 0,
                    bytes_skipped: 0,
                }
            } else {
                let stats = super::copy_file(&item.source_path, &item.dest_path, &file_config)?;
                if let Some(journal) = batch_journal {
                    let mut journal = journal.lock().unwrap();
                    record_create_file(&mut journal, &item.source_path, &item.relative_path)?;
                }
                stats
            }
        }
        EntryType::File => {
            // Ensure parent directory exists before copying file
            if let Some(parent) = item.dest_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            let mut source_hash: Option<String> = None;
            let mut hardlink_stats: Option<CopyStats> = None;

            if !config.link_dest.is_empty() {
                if let Ok(source_meta) = std::fs::metadata(&item.source_path) {
                    for ref_dir in &config.link_dest {
                        let ref_file = ref_dir.join(&item.relative_path);
                        if let Ok(ref_meta) = std::fs::metadata(&ref_file) {
                            if ref_meta.len() != source_meta.len() {
                                continue;
                            }
                            if source_hash.is_none() {
                                source_hash = Some(calculate_checksum(&item.source_path)?);
                            }
                            let ref_hash = calculate_checksum(&ref_file)?;
                            if ref_hash == source_hash.as_deref().unwrap()
                                && create_hardlink(&ref_file, &item.dest_path).is_ok()
                            {
                                if let Some(journal) = batch_journal {
                                    let mut journal = journal.lock().unwrap();
                                    record_create_file(
                                        &mut journal,
                                        &item.source_path,
                                        &item.relative_path,
                                    )?;
                                }
                                hardlink_stats = Some(CopyStats {
                                    bytes_copied: 0,
                                    duration: Duration::ZERO,
                                    checksum: None,
                                    compression_ratio: None,
                                    files_copied: 1,
                                    files_skipped: 0,
                                    files_failed: 0,
                                    delta_stats: None,
                                    chunks_resumed: 0,
                                    bytes_skipped: 0,
                                });
                                break;
                            }
                        }
                        if hardlink_stats.is_some() {
                            break;
                        }
                    }
                }
            }

            if hardlink_stats.is_none() {
                if let Some(index) = rename_index {
                    if !item.dest_path.exists() {
                        if source_hash.is_none() {
                            source_hash = Some(calculate_checksum(&item.source_path)?);
                        }
                        if let Some(basis_path) = index.get(source_hash.as_ref().unwrap()) {
                            if basis_path != &item.dest_path
                                && basis_path.exists()
                                && create_hardlink(basis_path, &item.dest_path).is_ok()
                            {
                                if let Some(journal) = batch_journal {
                                    let mut journal = journal.lock().unwrap();
                                    record_create_file(
                                        &mut journal,
                                        &item.source_path,
                                        &item.relative_path,
                                    )?;
                                }
                                hardlink_stats = Some(CopyStats {
                                    bytes_copied: 0,
                                    duration: Duration::ZERO,
                                    checksum: None,
                                    compression_ratio: None,
                                    files_copied: 1,
                                    files_skipped: 0,
                                    files_failed: 0,
                                    delta_stats: None,
                                    chunks_resumed: 0,
                                    bytes_skipped: 0,
                                });
                            }
                        }
                    }
                }
            }

            let stats = if let Some(stats) = hardlink_stats {
                stats
            } else {
                let stats = super::copy_file(&item.source_path, &item.dest_path, &file_config)?;
                if let Some(journal) = batch_journal {
                    let mut journal = journal.lock().unwrap();
                    record_create_file(&mut journal, &item.source_path, &item.relative_path)?;
                }
                stats
            };
            stats
        }
    };

    // Update total stats atomically
    if let Ok(mut total_stats) = stats_mutex.lock() {
        total_stats.bytes_copied += stats.bytes_copied;
        total_stats.files_copied += stats.files_copied;
        total_stats.files_skipped += stats.files_skipped;
        total_stats.files_failed += stats.files_failed;
        total_stats.bytes_skipped += stats.bytes_skipped;
        total_stats.chunks_resumed += stats.chunks_resumed;
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
        let deletions =
            collect_deletion_candidates(dest_dir, &expected, &config, &filter_list).unwrap();

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
        let deletions =
            collect_deletion_candidates(dest_dir, &expected, &config, &filter_list).unwrap();

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
        let deletions =
            collect_deletion_candidates(dest_dir, &expected, &config, &filter_list).unwrap();

        assert!(deletions.is_empty());
    }
}
