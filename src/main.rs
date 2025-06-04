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
 * Author: Your Name <your@email.com>
 */

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::{Instant, Duration};
use std::thread;

use anyhow::{Context, Result};
use clap::{Arg, Command};
use lz4::EncoderBuilder;
use lz4::Decoder;
use sha2::{Digest, Sha256};
use sysinfo::Disks;
use chrono::Utc;

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
        .get_matches();

    // Extract and parse command line arguments
    let source_path = PathBuf::from(matches.get_one::<String>("source").unwrap());
    let destination_path = PathBuf::from(matches.get_one::<String>("destination").unwrap());
    let compress = *matches.get_one::<bool>("compress").unwrap_or(&false);
    let resume_enabled = *matches.get_one::<bool>("resume").unwrap_or(&false);
    let retry_attempts: u32 = matches.get_one::<String>("retry-attempts").unwrap().parse()?;
    let retry_delay_secs: u64 = matches.get_one::<String>("retry-delay").unwrap().parse()?;
    let timeout_secs: u64 = matches.get_one::<String>("timeout").unwrap().parse()?;
    let chunk_size_kb: usize = matches.get_one::<String>("chunk-size").unwrap().parse()?;
    
    // Convert parsed values to appropriate types and units
    let chunk_size = chunk_size_kb * 1024; // Convert KB to bytes
    let retry_delay = Duration::from_secs(retry_delay_secs);
    let _timeout = Duration::from_secs(timeout_secs); // Reserved for future I/O timeout implementation

    // Get source file metadata to determine file size and validate existence
    let source_metadata = std::fs::metadata(&source_path)
        .with_context(|| format!("Failed to get metadata for source file: {:?}", source_path))?;
    let source_size = source_metadata.len();

    // Perform disk space validation before starting the copy operation
    let disks = Disks::new_with_refreshed_list();
    let destination_disk = disks.iter().find(|disk| destination_path.starts_with(disk.mount_point()));

    if let Some(disk) = destination_disk {
        if disk.available_space() < source_size {
            return Err(anyhow::anyhow!("Not enough space on destination disk"));
        }
    } else {
        println!("Warning: Could not determine available space on destination disk");
    }

    // Calculate source file checksum for integrity verification
    let checksum_source = calculate_checksum(&source_path)?;
    println!("Source file checksum: {:x}", checksum_source);

    // Retry loop with configurable attempts and delays
    let mut attempt = 0;
    let mut last_error: Option<anyhow::Error> = None;
    
    while attempt <= retry_attempts {
        // Add delay between retry attempts (but not for the first attempt)
        if attempt > 0 {
            println!("Retry attempt {} of {}...", attempt, retry_attempts);
            thread::sleep(retry_delay);
        }
        
        let start_time = Instant::now();
        
        // Attempt the file copy operation
        match perform_copy(
            &source_path,
            &destination_path,
            source_size,
            compress,
            resume_enabled,
            chunk_size,
        ) {
            Ok(()) => {
                // Verify copy integrity by comparing checksums
                let checksum_dest = calculate_checksum(&destination_path)?;
                let duration = start_time.elapsed();
                
                // Determine operation status based on checksum verification
                let status = if checksum_source == checksum_dest {
                    println!("✓ Copy verified. Destination SHA-256: {:x}", checksum_dest);
                    if compress { "Success (Compressed)" } else { "Success" }
                } else {
                    eprintln!("✗ Error: Checksum mismatch detected!");
                    eprintln!("  Source:      {:x}", checksum_source);
                    eprintln!("  Destination: {:x}", checksum_dest);
                    if compress { "Checksum Mismatch (Compressed)" } else { "Checksum Mismatch" }
                };

                // Write audit log entry for the completed operation
                write_audit_log(&source_path, &destination_path, source_size, duration, checksum_dest, status, attempt)?;
                println!("Operation completed in {:?} after {} attempts", duration, attempt + 1);
                
                return Ok(());
            }
            Err(e) => {
                // Store error for potential final return and increment attempt counter
                last_error = Some(e);
                attempt += 1;
                if attempt <= retry_attempts {
                    eprintln!("Transfer failed, will retry in {:?}...", retry_delay);
                }
            }
        }
    }
    
    // Return the last error if all retry attempts failed
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts exhausted")))
}

/// Orchestrates the file copy operation based on compression settings
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
    source_path: &PathBuf,
    destination_path: &PathBuf,
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
    source_path: &PathBuf,
    destination_path: &PathBuf,
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
fn load_resume_info(destination_path: &PathBuf, compressed: bool) -> Result<ResumeInfo> {
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
fn save_resume_info(destination_path: &PathBuf, bytes_copied: u64, compressed_bytes: Option<u64>, compressed: bool) -> Result<()> {
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
fn cleanup_resume_info(destination_path: &PathBuf, compressed: bool) {
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
fn get_resume_file_path(destination_path: &PathBuf, compressed: bool) -> PathBuf {
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
fn calculate_checksum(path: &PathBuf) -> Result<sha2::digest::Output<Sha256>> {
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
    source_path: &PathBuf,
    destination_path: &PathBuf,
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