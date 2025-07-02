/*!
 * Orbit - Intelligent File Copy Utility
 * 
 * A robust file copy utility with advanced features including:
 * - SHA-256 checksum verification for data integrity
 * - Optional LZ4 compression to reduce I/O bandwidth
 * - Resume capability for interrupted transfers
 * - Retry mechanism with configurable attempts and delays
 * - Disk space validation before copying
 * - Comprehensive audit logging
 * - Progress tracking and reporting
 * 
 * Version: 0.2.0
 * Author: Your Name shaneawall@gmail.com
 */

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Instant, Duration};
use std::thread;

use anyhow::{Context, Result};
use clap::{Arg, Command};
use lz4::EncoderBuilder;
use lz4::Decoder;
use sha2::{Digest, Sha256};
use sysinfo::Disks;
use chrono::Utc;
use walkdir::WalkDir;
use filetime::{FileTime, set_file_times};

#[cfg(unix)]
use std::os::unix::fs::{symlink, PermissionsExt};
#[cfg(windows)]
use std::os::windows::fs::symlink_file;

/// Contains state information for resuming interrupted file transfers
#[derive(Debug)]
struct ResumeInfo {
    /// Number of bytes successfully copied from source
    bytes_copied: u64,
    /// Partial checksum state (reserved for future implementation)
    partial_checksum: Option<Sha256>,
    /// Number of compressed bytes written (for compressed transfers only)
    compressed_bytes: Option<u64>,
}

/// Configuration for copy operations
#[derive(Debug, Clone)]
struct CopyConfig {
    /// Whether to use compression during transfer
    compress: bool,
    /// Whether resume functionality is enabled
    resume_enabled: bool,
    /// Size of chunks for buffered I/O operations
    chunk_size: usize,
    /// Whether to preserve file metadata (timestamps, permissions)
    preserve_metadata: bool,
    /// How to handle symbolic links
    symlink_mode: SymlinkMode,
    /// Whether to copy directories recursively
    recursive: bool,
}

/// Defines how symbolic links should be handled during copying
#[derive(Debug, Clone, Copy)]
enum SymlinkMode {
    /// Copy the symbolic link itself (preserve as symlink)
    Preserve,
    /// Follow the link and copy the target file/directory
    Follow,
    /// Skip symbolic links entirely
    Skip,
}

/// Main entry point for the Orbit file copy utility
/// 
/// Parses command line arguments, validates inputs, and orchestrates the file copy
/// operation with retry logic and comprehensive error handling.
fn main() -> Result<()> {
    // Parse command line arguments using clap
    let matches = Command::new("Orbit")
        .version("0.2.0")
        .author("Your Name <your@email.com>")
        .about("Intelligent file copy with checksum, optional compression, resume capability, and audit logging")
        .arg(
            Arg::new("source")
                .short('s')
                .long("source")
                .value_name("FILE")
                .help("Sets the source file to use")
                .required(true),
        )
        .arg(
            Arg::new("destination")
                .short('d')
                .long("destination")
                .value_name("FILE")
                .help("Sets the destination file to copy to")
                .required(true),
        )
        .arg(
            Arg::new("compress")
                .short('c')
                .long("compress")
                .help("Use compression during transfer to reduce bandwidth/I/O")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("resume")
                .short('r')
                .long("resume")
                .help("Enable resume functionality for interrupted transfers")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("retry-attempts")
                .long("retry-attempts")
                .value_name("COUNT")
                .help("Number of retry attempts on failure (default: 3)")
                .default_value("3"),
        )
        .arg(
            Arg::new("retry-delay")
                .long("retry-delay")
                .value_name("SECONDS")
                .help("Delay between retry attempts in seconds (default: 5)")
                .default_value("5"),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .help("Timeout for I/O operations in seconds (default: 30)")
                .default_value("30"),
        )
        .arg(
            Arg::new("chunk-size")
                .long("chunk-size")
                .value_name("BYTES")
                .help("Chunk size for resumable transfers in KB (default: 1024)")
                .default_value("1024"),
        )
        .arg(
            Arg::new("recursive")
                .short('R')
                .long("recursive")
                .help("Copy directories recursively")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("preserve-metadata")
                .short('p')
                .long("preserve-metadata")
                .help("Preserve file metadata (timestamps, permissions)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("symlinks")
                .short('L')
                .long("symlinks")
                .value_name("MODE")
                .help("How to handle symbolic links: preserve, follow, skip (default: preserve)")
                .default_value("preserve"),
        )
        .get_matches();

    // Extract and parse command line arguments
    let source_path = PathBuf::from(matches.get_one::<String>("source").unwrap());
    let destination_path = PathBuf::from(matches.get_one::<String>("destination").unwrap());
    let compress = *matches.get_one::<bool>("compress").unwrap_or(&false);
    let resume_enabled = *matches.get_one::<bool>("resume").unwrap_or(&false);
    let recursive = *matches.get_one::<bool>("recursive").unwrap_or(&false);
    let preserve_metadata = *matches.get_one::<bool>("preserve-metadata").unwrap_or(&false);
    let retry_attempts: u32 = matches.get_one::<String>("retry-attempts").unwrap().parse()?;
    let retry_delay_secs: u64 = matches.get_one::<String>("retry-delay").unwrap().parse()?;
    let timeout_secs: u64 = matches.get_one::<String>("timeout").unwrap().parse()?;
    let chunk_size_kb: usize = matches.get_one::<String>("chunk-size").unwrap().parse()?;
    
    // Parse symlink handling mode
    let symlink_mode = match matches.get_one::<String>("symlinks").unwrap().as_str() {
        "preserve" => SymlinkMode::Preserve,
        "follow" => SymlinkMode::Follow,
        "skip" => SymlinkMode::Skip,
        _ => return Err(anyhow::anyhow!("Invalid symlink mode. Use: preserve, follow, or skip")),
    };
    
    // Convert parsed values to appropriate types and units
    let chunk_size = chunk_size_kb * 1024; // Convert KB to bytes
    let retry_delay = Duration::from_secs(retry_delay_secs);
    let _timeout = Duration::from_secs(timeout_secs); // Reserved for future I/O timeout implementation

    // Create copy configuration
    let config = CopyConfig {
        compress,
        resume_enabled,
        chunk_size,
        preserve_metadata,
        symlink_mode,
        recursive,
    };

    // Get source metadata and determine if it's a file or directory
    let source_metadata = std::fs::metadata(&source_path)
        .with_context(|| format!("Failed to get metadata for source: {:?}", source_path))?;
    
    // Handle directory vs file copying
    if source_metadata.is_dir() {
        if !recursive {
            return Err(anyhow::anyhow!(
                "Source is a directory but --recursive flag not specified. Use -R to copy directories."
            ));
        }
        
        // For directory copying, we don't calculate total size upfront for disk space check
        // as it would require traversing the entire tree twice
        println!("Copying directory tree from {:?} to {:?}", source_path, destination_path);
        
        // Create destination directory if it doesn't exist
        if !destination_path.exists() {
            std::fs::create_dir_all(&destination_path)
                .with_context(|| format!("Failed to create destination directory: {:?}", destination_path))?;
        }
        
        copy_directory_tree(&source_path, &destination_path, &config, retry_attempts, retry_delay)?;
        
    } else {
        // Single file copy (existing logic)
        let source_size = source_metadata.len();
        
        // Perform disk space validation before starting the copy operation
        validate_disk_space(&destination_path, source_size)?;

        // Calculate source file checksum for integrity verification
        let checksum_source = calculate_checksum(&source_path)?;
        println!("Source file checksum: {:x}", checksum_source);

        // Retry loop with configurable attempts and delays
        perform_file_copy_with_retry(
            &source_path,
            &destination_path,
            source_size,
            &config,
            retry_attempts,
            retry_delay,
            Some(checksum_source),
        )?;
    }
    
    Ok(())
}

/// Validates that sufficient disk space is available for the copy operation
/// 
/// # Arguments
/// * `destination_path` - Path to the destination location
/// * `required_size` - Number of bytes that need to be available
fn validate_disk_space(destination_path: &Path, required_size: u64) -> Result<()> {
    let disks = Disks::new_with_refreshed_list();
    let destination_disk = disks.iter().find(|disk| destination_path.starts_with(disk.mount_point()));

    if let Some(disk) = destination_disk {
        if disk.available_space() < required_size {
            return Err(anyhow::anyhow!(
                "Not enough space on destination disk. Required: {} bytes, Available: {} bytes",
                required_size, disk.available_space()
            ));
        }
    } else {
        println!("Warning: Could not determine available space on destination disk");
    }
    Ok(())
}

/// Copies an entire directory tree recursively with full feature support
/// 
/// This function traverses the source directory tree and copies all files and
/// subdirectories to the destination, handling symbolic links, preserving metadata,
/// and providing progress reporting.
/// 
/// # Arguments
/// * `source_dir` - Path to the source directory
/// * `dest_dir` - Path to the destination directory
/// * `config` - Copy configuration options
/// * `retry_attempts` - Number of retry attempts for failed operations
/// * `retry_delay` - Delay between retry attempts
fn copy_directory_tree(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
    retry_attempts: u32,
    retry_delay: Duration,
) -> Result<()> {
    let start_time = Instant::now();
    let mut total_files = 0u64;
    let mut total_bytes = 0u64;
    let mut failed_files = Vec::new();

    println!("Scanning directory tree...");
    
    // Walk through the source directory tree
    for entry in WalkDir::new(source_dir) {
        let entry = entry.with_context(|| "Failed to read directory entry")?;
        let source_path = entry.path();
        
        // Calculate relative path from source to preserve directory structure
        let relative_path = source_path.strip_prefix(source_dir)
            .with_context(|| format!("Failed to calculate relative path for: {:?}", source_path))?;
        let dest_path = dest_dir.join(relative_path);
        
        if entry.file_type().is_dir() {
            // Create destination directory
            if !dest_path.exists() {
                std::fs::create_dir_all(&dest_path)
                    .with_context(|| format!("Failed to create directory: {:?}", dest_path))?;
                println!("Created directory: {:?}", dest_path);
            }
            
            // Preserve directory metadata if requested
            if config.preserve_metadata {
                if let Err(e) = preserve_metadata(source_path, &dest_path) {
                    eprintln!("Warning: Failed to preserve metadata for directory {:?}: {}", dest_path, e);
                }
            }
            
        } else if entry.file_type().is_symlink() {
            // Handle symbolic links based on configuration
            match handle_symlink(source_path, &dest_path, config.symlink_mode) {
                Ok(()) => {
                    println!("Processed symlink: {:?} -> {:?}", source_path, dest_path);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to handle symlink {:?}: {}", source_path, e);
                    failed_files.push((source_path.to_path_buf(), e.to_string()));
                }
            }
            
        } else if entry.file_type().is_file() {
            // Copy regular file
            let file_size = entry.metadata()
                .map(|m| m.len())
                .unwrap_or(0);
            
            match perform_file_copy_with_retry(
                source_path,
                &dest_path,
                file_size,
                config,
                retry_attempts,
                retry_delay,
                None, // Don't calculate checksum for each file in bulk operations
            ) {
                Ok(()) => {
                    total_files += 1;
                    total_bytes += file_size;
                    if total_files % 100 == 0 {
                        println!("Copied {} files ({} bytes)", total_files, total_bytes);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to copy file {:?}: {}", source_path, e);
                    failed_files.push((source_path.to_path_buf(), e.to_string()));
                }
            }
        }
    }
    
    let duration = start_time.elapsed();
    println!("\nDirectory copy completed:");
    println!("  Files copied: {}", total_files);
    println!("  Total bytes: {}", total_bytes);
    println!("  Duration: {:?}", duration);
    
    if !failed_files.is_empty() {
        println!("  Failed files: {}", failed_files.len());
        for (path, error) in &failed_files {
            eprintln!("    {:?}: {}", path, error);
        }
        return Err(anyhow::anyhow!("{} files failed to copy", failed_files.len()));
    }
    
    Ok(())
}

/// Handles symbolic link copying based on the specified mode
/// 
/// # Arguments
/// * `source_path` - Path to the source symbolic link
/// * `dest_path` - Path where the link should be created or target copied
/// * `mode` - How to handle the symbolic link
fn handle_symlink(source_path: &Path, dest_path: &Path, mode: SymlinkMode) -> Result<()> {
    match mode {
        SymlinkMode::Skip => {
            println!("Skipping symlink: {:?}", source_path);
            Ok(())
        }
        SymlinkMode::Follow => {
            // Follow the link and copy the target
            let target = std::fs::read_link(source_path)
                .with_context(|| format!("Failed to read symlink target: {:?}", source_path))?;
            
            let resolved_target = if target.is_absolute() {
                target
            } else {
                source_path.parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(target)
            };
            
            if resolved_target.is_file() {
                let file_size = std::fs::metadata(&resolved_target)?.len();
                let config = CopyConfig {
                    compress: false,
                    resume_enabled: false,
                    chunk_size: 64 * 1024,
                    preserve_metadata: true,
                    symlink_mode: SymlinkMode::Follow,
                    recursive: false,
                };
                perform_file_copy_with_retry(&resolved_target, dest_path, file_size, &config, 3, Duration::from_secs(5), None)?;
            } else if resolved_target.is_dir() {
                return Err(anyhow::anyhow!("Following directory symlinks not yet implemented"));
            }
            Ok(())
        }
        SymlinkMode::Preserve => {
            // Create a new symlink pointing to the same target
            let target = std::fs::read_link(source_path)
                .with_context(|| format!("Failed to read symlink target: {:?}", source_path))?;
            
            create_symlink(&target, dest_path)
                .with_context(|| format!("Failed to create symlink: {:?} -> {:?}", dest_path, target))?;
            Ok(())
        }
    }
}

/// Creates a symbolic link in a cross-platform manner
/// 
/// # Arguments
/// * `target` - Path that the symlink should point to
/// * `link_path` - Path where the symlink should be created
#[cfg(unix)]
fn create_symlink(target: &Path, link_path: &Path) -> Result<()> {
    symlink(target, link_path).map_err(|e| anyhow::anyhow!("Failed to create symlink: {}", e))
}

#[cfg(windows)]
fn create_symlink(target: &Path, link_path: &Path) -> Result<()> {
    if target.is_file() {
        symlink_file(target, link_path).map_err(|e| anyhow::anyhow!("Failed to create file symlink: {}", e))
    } else {
        std::os::windows::fs::symlink_dir(target, link_path).map_err(|e| anyhow::anyhow!("Failed to create directory symlink: {}", e))
    }
}

/// Preserves file metadata from source to destination
/// 
/// This function copies timestamps, permissions, and other metadata
/// from the source file to the destination file.
/// 
/// # Arguments
/// * `source_path` - Path to the source file
/// * `dest_path` - Path to the destination file
fn preserve_metadata(source_path: &Path, dest_path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(source_path)
        .with_context(|| format!("Failed to read metadata from: {:?}", source_path))?;
    
    // Preserve permissions
    std::fs::set_permissions(dest_path, metadata.permissions())
        .with_context(|| format!("Failed to set permissions on: {:?}", dest_path))?;
    
    // Preserve timestamps
    let accessed = FileTime::from_last_access_time(&metadata);
    let modified = FileTime::from_last_modification_time(&metadata);
    
    set_file_times(dest_path, accessed, modified)
        .with_context(|| format!("Failed to set timestamps on: {:?}", dest_path))?;
    
    Ok(())
}

/// Performs a file copy operation with retry logic and comprehensive error handling
/// 
/// # Arguments
/// * `source_path` - Path to the source file
/// * `dest_path` - Path to the destination file
/// * `source_size` - Size of the source file in bytes
/// * `config` - Copy configuration options
/// * `retry_attempts` - Number of retry attempts on failure
/// * `retry_delay` - Delay between retry attempts
/// * `expected_checksum` - Optional pre-calculated checksum for verification
fn perform_file_copy_with_retry(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
    retry_attempts: u32,
    retry_delay: Duration,
    expected_checksum: Option<sha2::digest::Output<Sha256>>,
) -> Result<()> {
    let mut attempt = 0;
    let mut last_error: Option<anyhow::Error> = None;
    
    while attempt <= retry_attempts {
        if attempt > 0 {
            println!("Retry attempt {} of {} for {:?}...", attempt, retry_attempts, source_path);
            thread::sleep(retry_delay);
        }
        
        let start_time = Instant::now();
        
        match perform_copy(source_path, dest_path, source_size, config) {
            Ok(()) => {
                // Preserve metadata if requested
                if config.preserve_metadata {
                    if let Err(e) = preserve_metadata(source_path, dest_path) {
                        eprintln!("Warning: Failed to preserve metadata for {:?}: {}", dest_path, e);
                    }
                }
                
                // Verify checksum if provided
                if let Some(source_checksum) = expected_checksum {
                    let dest_checksum = calculate_checksum(dest_path)?;
                    if source_checksum != dest_checksum {
                        return Err(anyhow::anyhow!("Checksum mismatch for file: {:?}", dest_path));
                    }
                }
                
                let duration = start_time.elapsed();
                let status = if config.compress { "Success (Compressed)" } else { "Success" };
                
                // Write audit log for single file operations
                if expected_checksum.is_some() {
                    let checksum = calculate_checksum(dest_path)?;
                    write_audit_log(source_path, dest_path, source_size, duration, checksum, status, attempt)?;
                }
                
                return Ok(());
            }
            Err(e) => {
                last_error = Some(e);
                attempt += 1;
                if attempt <= retry_attempts {
                    eprintln!("Transfer failed for {:?}, will retry in {:?}...", source_path, retry_delay);
                }
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts exhausted")))
}
/// 
/// # Arguments
/// * `source_path` - Path to the source file
/// * `destination_path` - Path where the file will be copied
/// * `source_size` - Size of the source file in bytes
/// * `compress` - Whether to use compression during transfer
/// * `resume_enabled` - Whether resume functionality is enabled
/// * `chunk_size` - Size of chunks for buffered I/O operations
/// 
/// # Returns
/// Result indicating success or failure of the copy operation
fn perform_copy(
    source_path: &PathBuf,
    destination_path: &PathBuf,
    source_size: u64,
    compress: bool,
    resume_enabled: bool,
    chunk_size: usize,
) -> Result<()> {
    if compress {
        perform_compressed_copy(source_path, destination_path, source_size, resume_enabled, chunk_size)
    } else {
        perform_direct_copy(source_path, destination_path, source_size, resume_enabled, chunk_size)
    }
}

/// Performs a direct file copy without compression
/// 
/// This function handles resumable transfers by tracking progress and periodically
/// saving resume information. It uses buffered I/O for optimal performance.
/// 
/// # Arguments
/// * `source_path` - Path to the source file
/// * `destination_path` - Path where the file will be copied
/// * `source_size` - Size of the source file in bytes
/// * `resume_enabled` - Whether resume functionality is enabled
/// * `chunk_size` - Size of chunks for buffered I/O operations
fn perform_direct_copy(
    source_path: &Path,
    destination_path: &Path,
    source_size: u64,
    resume_enabled: bool,
    chunk_size: usize,
) -> Result<()> { optimal performance.
/// 
/// # Arguments
/// * `source_path` - Path to the source file
/// * `destination_path` - Path where the file will be copied
/// * `source_size` - Size of the source file in bytes
/// * `resume_enabled` - Whether resume functionality is enabled
/// * `chunk_size` - Size of chunks for buffered I/O operations
fn perform_direct_copy(
    source_path: &Path,
    destination_path: &Path,
    source_size: u64,
    resume_enabled: bool,
    chunk_size: usize,
) -> Result<()> {
    // Load resume information if resume is enabled
    let resume_info = if resume_enabled {
        load_resume_info(destination_path, false)?
    } else {
        ResumeInfo { bytes_copied: 0, partial_checksum: None, compressed_bytes: None }
    };

    let start_offset = resume_info.bytes_copied;
    if start_offset > 0 {
        println!("Resuming direct copy from byte {} of {}", start_offset, source_size);
    }

    // Open source file and seek to resume position
    let mut source_file = BufReader::new(File::open(source_path)?);
    source_file.seek(SeekFrom::Start(start_offset))?;

    // Open destination file in appropriate mode (append for resume, truncate for new)
    let mut dest_file = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(start_offset > 0)  // Append mode if resuming
            .truncate(start_offset == 0)  // Truncate if starting fresh
            .open(destination_path)?
    );

    // Initialize buffer and tracking variables
    let mut buffer = vec![0u8; chunk_size];
    let mut bytes_copied = start_offset;
    let mut last_checkpoint = Instant::now();

    // Main copy loop - process file in chunks
    while bytes_copied < source_size {
        let remaining = (source_size - bytes_copied) as usize;
        let to_read = remaining.min(chunk_size);
        
        // Read chunk from source
        let n = source_file.read(&mut buffer[..to_read])?;
        if n == 0 {
            break; // End of file reached
        }
        
        // Write chunk to destination
        dest_file.write_all(&buffer[..n])?;
        bytes_copied += n as u64;
        
        // Periodic checkpointing for resume capability (every 5 seconds)
        if resume_enabled && last_checkpoint.elapsed() > Duration::from_secs(5) {
            dest_file.flush()?; // Ensure data is written to disk
            save_resume_info(destination_path, bytes_copied, None, false)?;
            print!("\rProgress: {:.1}%", (bytes_copied as f64 / source_size as f64) * 100.0);
            std::io::stdout().flush()?;
            last_checkpoint = Instant::now();
        }
    }
    
    // Ensure all data is written to disk
    dest_file.flush()?;
    
    // Clean up resume information files
    if resume_enabled {
        cleanup_resume_info(destination_path, false);
    }
    
    println!("\nDirect copy completed, {} bytes written.", bytes_copied);
    Ok(())
}

/// Performs a compressed file copy using LZ4 compression
/// 
/// This function implements a two-phase approach:
/// 1. Compression phase: Source -> Temporary compressed file
/// 2. Decompression phase: Temporary compressed file -> Destination
/// 
/// This approach allows for resume capability during the compression phase
/// and ensures data integrity through the decompression verification.
/// 
/// # Arguments
/// * `source_path` - Path to the source file
/// * `destination_path` - Path where the file will be copied
/// * `source_size` - Size of the source file in bytes
/// * `resume_enabled` - Whether resume functionality is enabled
/// * `chunk_size` - Size of chunks for buffered I/O operations
fn perform_compressed_copy(
    source_path: &Path,
    destination_path: &Path,
    source_size: u64,
    resume_enabled: bool,
    chunk_size: usize,
) -> Result<()> {
    // Create temporary file path for compressed data
    let temp_compressed_path = destination_path.with_extension("tmp.lz4");
    let _cleanup_guard = TempFileCleanup::new(&temp_compressed_path);

    // Load resume information for compressed transfers
    let resume_info = if resume_enabled {
        load_resume_info(destination_path, true)?
    } else {
        ResumeInfo { bytes_copied: 0, partial_checksum: None, compressed_bytes: None }
    };

    let start_offset = resume_info.bytes_copied;
    let compressed_start = resume_info.compressed_bytes.unwrap_or(0);

    // PHASE 1: Compression (with resume support)
    if compressed_start == 0 || !temp_compressed_path.exists() {
        println!("Starting compression phase...");
        
        // Open source file and seek to resume position
        let mut source_file = BufReader::new(File::open(source_path)?);
        if start_offset > 0 {
            source_file.seek(SeekFrom::Start(start_offset))?;
            println!("Resuming compression from byte {}", start_offset);
        }

        // Open compressed output file (append if resuming, create if new)
        let compressed_file = if start_offset > 0 && temp_compressed_path.exists() {
            OpenOptions::new().append(true).open(&temp_compressed_path)?
        } else {
            File::create(&temp_compressed_path)?
        };

        // Initialize LZ4 encoder with compression level 4 (balance of speed/compression)
        let mut encoder = EncoderBuilder::new().level(4).build(compressed_file)?;
        let mut buffer = vec![0u8; chunk_size];
        let mut bytes_read = start_offset;
        let mut last_checkpoint = Instant::now();

        // Compression loop
        while bytes_read < source_size {
            let remaining = (source_size - bytes_read) as usize;
            let to_read = remaining.min(chunk_size);
            
            // Read from source
            let n = source_file.read(&mut buffer[..to_read])?;
            if n == 0 {
                break;
            }
            
            // Compress and write
            encoder.write_all(&buffer[..n])?;
            bytes_read += n as u64;
            
            // Periodic checkpointing for resume capability
            if resume_enabled && last_checkpoint.elapsed() > Duration::from_secs(5) {
                let compressed_size = std::fs::metadata(&temp_compressed_path)
                    .map(|m| m.len())
                    .unwrap_or(0);
                save_resume_info(destination_path, bytes_read, Some(compressed_size), true)?;
                print!("\rCompression progress: {:.1}%", (bytes_read as f64 / source_size as f64) * 100.0);
                std::io::stdout().flush()?;
                last_checkpoint = Instant::now();
            }
        }
        
        // Finalize compression
        let (_output, result) = encoder.finish();
        result?;
        
        // Report compression statistics
        let compressed_size = std::fs::metadata(&temp_compressed_path)?.len();
        let compression_ratio = (compressed_size as f64 / source_size as f64) * 100.0;
        println!("\nCompression completed: {} bytes -> {} bytes ({:.1}% of original)", 
                 source_size, compressed_size, compression_ratio);
    } else {
        println!("Using existing compressed file from previous attempt");
    }

    // PHASE 2: Decompression
    {
        // Open compressed file for reading
        let compressed_input = BufReader::new(File::open(&temp_compressed_path)?);
        let mut decoder = Decoder::new(compressed_input)?;
        let mut decompressed_output = BufWriter::new(File::create(destination_path)?);
        
        // Decompress all data in one operation
        let bytes_written = std::io::copy(&mut decoder, &mut decompressed_output)?;
        decompressed_output.flush()?;
        println!("Decompression completed, {} bytes written.", bytes_written);
        
        // Verify decompressed size matches original
        if bytes_written != source_size {
            return Err(anyhow::anyhow!(
                "Decompressed size mismatch: expected {} bytes, got {} bytes", 
                source_size, bytes_written
            ));
        }
    }

    // Clean up resume information
    if resume_enabled {
        cleanup_resume_info(destination_path, true);
    }

    Ok(())
}

/// Loads resume information from disk for interrupted transfers
/// 
/// Resume files store the number of bytes successfully copied and optionally
/// the number of compressed bytes (for compressed transfers).
/// 
/// # Arguments
/// * `destination_path` - Path to the destination file
/// * `compressed` - Whether this is for a compressed transfer
/// 
/// # Returns
/// ResumeInfo structure containing saved progress information
fn load_resume_info(destination_path: &Path, compressed: bool) -> Result<ResumeInfo> {
    let resume_file_path = get_resume_file_path(destination_path, compressed);
    
    // Return default if no resume file exists
    if !resume_file_path.exists() {
        return Ok(ResumeInfo { 
            bytes_copied: 0, 
            partial_checksum: None, 
            compressed_bytes: None 
        });
    }

    // Read and parse resume file content
    let resume_data = std::fs::read_to_string(&resume_file_path)?;
    let lines: Vec<&str> = resume_data.lines().collect();
    
    if lines.is_empty() {
        return Ok(ResumeInfo { 
            bytes_copied: 0, 
            partial_checksum: None, 
            compressed_bytes: None 
        });
    }

    // Parse bytes copied (first line)
    let bytes_copied: u64 = lines[0].parse().unwrap_or(0);
    
    // Parse compressed bytes if available (second line)
    let compressed_bytes = if lines.len() > 1 {
        lines[1].parse().ok()
    } else {
        None
    };

    println!("Found resume info: {} bytes copied", bytes_copied);
    
    Ok(ResumeInfo {
        bytes_copied,
        partial_checksum: None, // Partial checksum resume could be implemented in future
        compressed_bytes,
    })
}

/// Saves current progress to a resume file for interrupted transfer recovery
/// 
/// # Arguments
/// * `destination_path` - Path to the destination file
/// * `bytes_copied` - Number of bytes successfully copied so far
/// * `compressed_bytes` - Number of compressed bytes written (for compressed transfers)
/// * `compressed` - Whether this is for a compressed transfer
fn save_resume_info(destination_path: &Path, bytes_copied: u64, compressed_bytes: Option<u64>, compressed: bool) -> Result<()> {
    let resume_file_path = get_resume_file_path(destination_path, compressed);
    
    // Format resume file content
    let mut content = bytes_copied.to_string();
    if let Some(cb) = compressed_bytes {
        content.push('\n');
        content.push_str(&cb.to_string());
    }
    
    std::fs::write(&resume_file_path, content)?;
    Ok(())
}

/// Removes resume information files after successful completion
/// 
/// # Arguments
/// * `destination_path` - Path to the destination file
/// * `compressed` - Whether this is for a compressed transfer
fn cleanup_resume_info(destination_path: &Path, compressed: bool) {
    let resume_file_path = get_resume_file_path(destination_path, compressed);
    if resume_file_path.exists() {
        let _ = std::fs::remove_file(&resume_file_path);
    }
}

/// Generates the path for resume information files
/// 
/// Resume files are stored alongside the destination file with special extensions
/// to avoid conflicts with the actual file content.
/// 
/// # Arguments
/// * `destination_path` - Path to the destination file
/// * `compressed` - Whether this is for a compressed transfer
/// 
/// # Returns
/// Path to the resume information file
fn get_resume_file_path(destination_path: &Path, compressed: bool) -> PathBuf {
    if compressed {
        destination_path.with_extension("orbit_resume_compressed")
    } else {
        destination_path.with_extension("orbit_resume")
    }
}

/// Calculates SHA-256 checksum of a file for integrity verification
/// 
/// Uses buffered I/O to efficiently process large files without loading
/// the entire file into memory.
/// 
/// # Arguments
/// * `path` - Path to the file to checksum
/// 
/// # Returns
/// SHA-256 digest of the file contents
fn calculate_checksum(path: &Path) -> Result<sha2::digest::Output<Sha256>> {
    let mut file = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024]; // 64KB buffer for optimal I/O performance
    
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break; // End of file reached
        }
        hasher.update(&buffer[..n]);
    }
    
    Ok(hasher.finalize())
}

/// Writes an audit log entry for the completed file operation
/// 
/// The audit log provides a permanent record of all file operations with
/// timestamps, file paths, sizes, checksums, and operation status.
/// 
/// # Arguments
/// * `source_path` - Path to the source file
/// * `destination_path` - Path to the destination file
/// * `source_size` - Size of the source file in bytes
/// * `duration` - Time taken to complete the operation
/// * `checksum` - SHA-256 checksum of the destination file
/// * `status` - Operation status (Success, Checksum Mismatch, etc.)
/// * `attempts` - Number of attempts made (for retry tracking)
fn write_audit_log(
    source_path: &Path,
    destination_path: &Path,
    source_size: u64,
    duration: std::time::Duration,
    checksum: sha2::digest::Output<Sha256>,
    status: &str,
    attempts: u32,
) -> Result<()> {
    // Open audit log file in append mode
    let mut log_file = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open("orbit_audit.log")
            .with_context(|| "Failed to open audit log")?
    );
    
    // Write CSV-formatted log entry with timestamp
    writeln!(
        log_file,
        "{}, Source: {:?}, Destination: {:?}, Size: {} bytes, Duration: {:?}, Checksum: {:x}, Status: {}, Attempts: {}",
        Utc::now(),
        source_path,
        destination_path,
        source_size,
        duration,
        checksum,
        status,
        attempts + 1
    )?;
    log_file.flush()?;
    Ok(())
}

/// RAII helper to ensure temporary files are cleaned up automatically
/// 
/// This struct implements the Drop trait to guarantee that temporary files
/// are removed even if the program exits unexpectedly or panics.
struct TempFileCleanup {
    path: PathBuf,
}

impl TempFileCleanup {
    /// Creates a new cleanup guard for the specified path
    /// 
    /// # Arguments
    /// * `path` - Path to the temporary file that should be cleaned up
    fn new(path: &PathBuf) -> Self {
        Self { path: path.clone() }
    }
}

/// Automatic cleanup implementation
/// 
/// This is called automatically when the TempFileCleanup goes out of scope,
/// ensuring temporary files are always removed.
impl Drop for TempFileCleanup {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                eprintln!("Warning: Failed to clean up temporary file {:?}: {}", self.path, e);
            }
        }
    }
}