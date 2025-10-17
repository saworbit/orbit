/*!
 * Core file copy operations
 */

pub mod checksum;
pub mod resume;
pub mod metadata;
pub mod validation;
pub mod zero_copy;

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::{Arc, Mutex};

use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;

use crate::config::{CopyConfig, SymlinkMode};
use crate::error::{OrbitError, Result};
use crate::compression;

use checksum::StreamingHasher;
use resume::ResumeInfo;
use metadata::preserve_metadata;
use validation::should_copy_file;
use zero_copy::{ZeroCopyResult, ZeroCopyCapabilities};

/// Statistics about a copy operation
#[derive(Debug, Clone)]
pub struct CopyStats {
    pub bytes_copied: u64,
    pub duration: Duration,
    pub checksum: Option<String>,
    pub compression_ratio: Option<f64>,
    pub files_copied: u64,
    pub files_skipped: u64,
    pub files_failed: u64,
}

impl CopyStats {
    pub fn new() -> Self {
        Self {
            bytes_copied: 0,
            duration: Duration::ZERO,
            checksum: None,
            compression_ratio: None,
            files_copied: 0,
            files_skipped: 0,
            files_failed: 0,
        }
    }
}

impl Default for CopyStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Copy a single file with all configured options
pub fn copy_file(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
) -> Result<CopyStats> {
    let start_time = Instant::now();
    
    // Validate source exists
    if !source_path.exists() {
        return Err(OrbitError::SourceNotFound(source_path.to_path_buf()));
    }
    
    let source_metadata = std::fs::metadata(source_path)?;
    let source_size = source_metadata.len();
    
    // Check if we should copy based on mode
    if !should_copy_file(source_path, dest_path, config.copy_mode)? {
        return Ok(CopyStats {
            bytes_copied: 0,
            duration: start_time.elapsed(),
            checksum: None,
            compression_ratio: None,
            files_copied: 0,
            files_skipped: 1,
            files_failed: 0,
        });
    }
    
    // Dry run mode
    if config.dry_run {
        println!("Would copy: {:?} -> {:?} ({} bytes)", source_path, dest_path, source_size);
        return Ok(CopyStats {
            bytes_copied: source_size,
            duration: start_time.elapsed(),
            checksum: None,
            compression_ratio: None,
            files_copied: 1,
            files_skipped: 0,
            files_failed: 0,
        });
    }
    
    // Validate disk space
    validation::validate_disk_space(dest_path, source_size)?;
    
    // Perform copy with retry logic
    let mut attempt = 0;
    let mut last_error: Option<OrbitError> = None;
    
    while attempt <= config.retry_attempts {
        if attempt > 0 {
            let delay = if config.exponential_backoff {
                Duration::from_secs(config.retry_delay_secs * 2_u64.pow(attempt - 1))
            } else {
                Duration::from_secs(config.retry_delay_secs)
            };
            
            println!("Retry attempt {} of {} after {:?}...", attempt, config.retry_attempts, delay);
            thread::sleep(delay);
        }
        
        match perform_copy_internal(source_path, dest_path, source_size, config) {
            Ok(stats) => {
                // Preserve metadata if requested
                if config.preserve_metadata {
                    if let Err(e) = preserve_metadata(source_path, dest_path) {
                        eprintln!("Warning: Failed to preserve metadata: {}", e);
                    }
                }
                
                return Ok(stats);
            }
            Err(e) => {
                if e.is_fatal() {
                    return Err(e);
                }
                last_error = Some(e);
                attempt += 1;
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| OrbitError::RetriesExhausted { 
        attempts: config.retry_attempts 
    }))
}

/// Internal copy implementation (called by retry logic)
fn perform_copy_internal(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
) -> Result<CopyStats> {
    match config.compression {
        crate::config::CompressionType::None => {
            copy_direct(source_path, dest_path, source_size, config)
        }
        crate::config::CompressionType::Lz4 => {
            compression::copy_with_lz4(source_path, dest_path, source_size, config)
        }
        crate::config::CompressionType::Zstd { level } => {
            compression::copy_with_zstd(source_path, dest_path, source_size, level, config)
        }
    }
}

/// Direct copy without compression (with optional zero-copy optimization)
fn copy_direct(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
) -> Result<CopyStats> {
    // Determine if we should attempt zero-copy
    let use_zero_copy = should_use_zero_copy(source_path, dest_path, config)?;
    
    if use_zero_copy {
        // Try zero-copy first
        match try_zero_copy_direct(source_path, dest_path, source_size, config) {
            Ok(stats) => {
                if config.show_progress {
                    println!("✓ Zero-copy transfer completed");
                }
                return Ok(stats);
            }
            Err(OrbitError::ZeroCopyUnsupported) => {
                if config.show_progress {
                    println!("Zero-copy not supported, using buffered copy");
                }
                // Fall through to buffered copy
            }
            Err(e) => {
                // Other errors should be returned
                return Err(e);
            }
        }
    }
    
    // Use buffered copy (either as fallback or by default)
    copy_buffered(source_path, dest_path, source_size, config)
}

/// Determine if zero-copy should be attempted
fn should_use_zero_copy(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
) -> Result<bool> {
    // Check if zero-copy is available on this platform
    let caps = ZeroCopyCapabilities::detect();
    if !caps.available {
        return Ok(false);
    }
    
    // Don't use zero-copy if:
    // 1. Resume is enabled (complex offset handling works better with buffered)
    // 2. Bandwidth throttling is active (need granular control)
    if config.resume_enabled || config.max_bandwidth > 0 {
        return Ok(false);
    }
    
    // Check if files are on the same filesystem (required for Linux copy_file_range)
    if !caps.cross_filesystem {
        let same_fs = zero_copy::same_filesystem(source_path, dest_path)?;
        if !same_fs {
            return Ok(false);
        }
    }
    
    // For very small files (< 64KB), buffered copy is often faster due to syscall overhead
    if source_path.metadata()?.len() < 64 * 1024 {
        return Ok(false);
    }
    
    Ok(true)
}

/// Attempt zero-copy transfer
fn try_zero_copy_direct(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
) -> Result<CopyStats> {
    let start_time = Instant::now();
    
    // Open files
    let source_file = File::open(source_path)?;
    let dest_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dest_path)?;
    
    // Setup progress bar
    let progress = if config.show_progress {
        let pb = ProgressBar::new(source_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );
        Some(pb)
    } else {
        None
    };
    
    // Attempt zero-copy
    let result = zero_copy::try_zero_copy(&source_file, &dest_file, 0, source_size);
    
    let bytes_copied = match result {
        ZeroCopyResult::Success(n) => n,
        ZeroCopyResult::Unsupported => {
            return Err(OrbitError::ZeroCopyUnsupported);
        }
        ZeroCopyResult::Failed(e) => {
            return Err(OrbitError::Io(e));
        }
    };
    
    // Flush to ensure data is written
    dest_file.sync_all()?;
    
    if let Some(pb) = progress {
        pb.set_position(bytes_copied);
        pb.finish_with_message("Complete");
    }
    
    // If checksum verification is enabled, calculate it post-copy
    let checksum = if config.verify_checksum {
        if config.show_progress {
            println!("Calculating checksum...");
        }
        let source_checksum = checksum::calculate_checksum(source_path)?;
        let dest_checksum = checksum::calculate_checksum(dest_path)?;
        
        if source_checksum != dest_checksum {
            return Err(OrbitError::ChecksumMismatch {
                expected: source_checksum,
                actual: dest_checksum,
            });
        }
        Some(source_checksum)
    } else {
        None
    };
    
    Ok(CopyStats {
        bytes_copied,
        duration: start_time.elapsed(),
        checksum,
        compression_ratio: None,
        files_copied: 1,
        files_skipped: 0,
        files_failed: 0,
    })
}

/// Buffered copy with streaming checksum (original implementation)
fn copy_buffered(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
) -> Result<CopyStats> {
    let start_time = Instant::now();
    
    // Load resume info if enabled
    let resume_info = if config.resume_enabled {
        resume::load_resume_info(dest_path, false)?
    } else {
        ResumeInfo::default()
    };
    
    let start_offset = resume_info.bytes_copied;
    
    // Open source file
    let mut source_file = BufReader::new(File::open(source_path)?);
    if start_offset > 0 {
        source_file.seek(SeekFrom::Start(start_offset))?;
        println!("Resuming from byte {}", start_offset);
    }
    
    // Open destination file
    let mut dest_file = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(start_offset > 0)
            .truncate(start_offset == 0)
            .open(dest_path)?
    );
    
    // Setup progress bar
    let progress = if config.show_progress {
        let pb = ProgressBar::new(source_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );
        pb.set_position(start_offset);
        Some(pb)
    } else {
        None
    };
    
    // Streaming hasher for checksum calculation
    let mut hasher = if config.verify_checksum {
        Some(StreamingHasher::new())
    } else {
        None
    };
    
    // Copy loop
    let mut buffer = vec![0u8; config.chunk_size];
    let mut bytes_copied = start_offset;
    let mut last_checkpoint = Instant::now();
    
    while bytes_copied < source_size {
        let remaining = (source_size - bytes_copied) as usize;
        let to_read = remaining.min(config.chunk_size);
        
        let n = source_file.read(&mut buffer[..to_read])?;
        if n == 0 {
            break;
        }
        
        // Update checksum
        if let Some(ref mut h) = hasher {
            h.update(&buffer[..n]);
        }
        
        // Write to destination
        dest_file.write_all(&buffer[..n])?;
        bytes_copied += n as u64;
        
        // Update progress
        if let Some(ref pb) = progress {
            pb.set_position(bytes_copied);
        }
        
        // Checkpoint for resume
        if config.resume_enabled && last_checkpoint.elapsed() > Duration::from_secs(5) {
            dest_file.flush()?;
            resume::save_resume_info(dest_path, bytes_copied, None, false)?;
            last_checkpoint = Instant::now();
        }
        
        // Bandwidth throttling
        if config.max_bandwidth > 0 {
            apply_bandwidth_limit(n as u64, config.max_bandwidth, &mut last_checkpoint);
        }
    }
    
    dest_file.flush()?;
    
    if let Some(pb) = progress {
        pb.finish_with_message("Complete");
    }
    
    // Clean up resume info
    if config.resume_enabled {
        resume::cleanup_resume_info(dest_path, false);
    }
    
    // Verify checksum
    let checksum = if let Some(h) = hasher {
        let hash = h.finalize();
        Some(format!("{:x}", hash))
    } else {
        None
    };
    
    Ok(CopyStats {
        bytes_copied,
        duration: start_time.elapsed(),
        checksum,
        compression_ratio: None,
        files_copied: 1,
        files_skipped: 0,
        files_failed: 0,
    })
}

/// Apply bandwidth limiting
fn apply_bandwidth_limit(bytes_written: u64, max_bandwidth: u64, last_check: &mut Instant) {
    let elapsed = last_check.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    
    if elapsed_secs < 1.0 {
        let bytes_per_sec = bytes_written as f64 / elapsed_secs;
        if bytes_per_sec > max_bandwidth as f64 {
            let sleep_time = Duration::from_secs_f64(
                (bytes_written as f64 / max_bandwidth as f64) - elapsed_secs
            );
            thread::sleep(sleep_time);
        }
    }
    
    if elapsed >= Duration::from_secs(1) {
        *last_check = Instant::now();
    }
}

/// Copy a directory recursively with streaming iteration to reduce memory usage
pub fn copy_directory(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
) -> Result<CopyStats> {
    use crossbeam_channel::bounded;
    
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
    use rayon::prelude::*;
    
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
            copy_file(&item.source_path, &item.dest_path, config)?
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
                copy_file(&resolved, dest_path, config)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use tempfile::tempdir;

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
    fn test_zero_copy_small_file_skipped() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("small.txt");
        let dest = dir.path().join("dest.txt");
        
        // Small file (< 64KB) should not use zero-copy
        std::fs::write(&source, b"small").unwrap();
        
        let config = CopyConfig::default();
        let use_zc = should_use_zero_copy(&source, &dest, &config).unwrap();
        
        // Small files should skip zero-copy
        assert!(!use_zc);
    }
}