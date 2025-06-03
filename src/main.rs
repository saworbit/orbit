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

#[derive(Debug)]
struct ResumeInfo {
    bytes_copied: u64,
    partial_checksum: Option<Sha256>,
    compressed_bytes: Option<u64>,
}

fn main() -> Result<()> {
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

    let source_path = PathBuf::from(matches.get_one::<String>("source").unwrap());
    let destination_path = PathBuf::from(matches.get_one::<String>("destination").unwrap());
    let compress = *matches.get_one::<bool>("compress").unwrap_or(&false);
    let resume_enabled = *matches.get_one::<bool>("resume").unwrap_or(&false);
    let retry_attempts: u32 = matches.get_one::<String>("retry-attempts").unwrap().parse()?;
    let retry_delay_secs: u64 = matches.get_one::<String>("retry-delay").unwrap().parse()?;
    let timeout_secs: u64 = matches.get_one::<String>("timeout").unwrap().parse()?;
    let chunk_size_kb: usize = matches.get_one::<String>("chunk-size").unwrap().parse()?;
    
    let chunk_size = chunk_size_kb * 1024;
    let retry_delay = Duration::from_secs(retry_delay_secs);
    let _timeout = Duration::from_secs(timeout_secs); // For future I/O timeout implementation

    let source_metadata = std::fs::metadata(&source_path)
        .with_context(|| format!("Failed to get metadata for source file: {:?}", source_path))?;
    let source_size = source_metadata.len();

    // --- Disk space check ---
    let disks = Disks::new_with_refreshed_list();
    let destination_disk = disks.iter().find(|disk| destination_path.starts_with(disk.mount_point()));

    if let Some(disk) = destination_disk {
        if disk.available_space() < source_size {
            return Err(anyhow::anyhow!("Not enough space on destination disk"));
        }
    } else {
        println!("Warning: Could not determine available space on destination disk");
    }

    // Calculate source checksum first
    let checksum_source = calculate_checksum(&source_path)?;
    println!("Source file checksum: {:x}", checksum_source);

    let mut attempt = 0;
    let mut last_error: Option<anyhow::Error> = None;
    
    while attempt <= retry_attempts {
        if attempt > 0 {
            println!("Retry attempt {} of {}...", attempt, retry_attempts);
            thread::sleep(retry_delay);
        }
        
        let start_time = Instant::now();
        
        match perform_copy(
            &source_path,
            &destination_path,
            source_size,
            compress,
            resume_enabled,
            chunk_size,
        ) {
            Ok(()) => {
                // --- Final checksum validation ---
                let checksum_dest = calculate_checksum(&destination_path)?;
                let duration = start_time.elapsed();
                
                let status = if checksum_source == checksum_dest {
                    println!("✓ Copy verified. Destination SHA-256: {:x}", checksum_dest);
                    if compress { "Success (Compressed)" } else { "Success" }
                } else {
                    eprintln!("✗ Error: Checksum mismatch detected!");
                    eprintln!("  Source:      {:x}", checksum_source);
                    eprintln!("  Destination: {:x}", checksum_dest);
                    if compress { "Checksum Mismatch (Compressed)" } else { "Checksum Mismatch" }
                };

                write_audit_log(&source_path, &destination_path, source_size, duration, checksum_dest, status, attempt)?;
                println!("Operation completed in {:?} after {} attempts", duration, attempt + 1);
                
                return Ok(());
            }
            Err(e) => {
                last_error = Some(e);
                attempt += 1;
                if attempt <= retry_attempts {
                    eprintln!("Transfer failed, will retry in {:?}...", retry_delay);
                }
            }
        }
    }
    
    // If we get here, all attempts failed
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts exhausted")))
}

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

fn perform_direct_copy(
    source_path: &PathBuf,
    destination_path: &PathBuf,
    source_size: u64,
    resume_enabled: bool,
    chunk_size: usize,
) -> Result<()> {
    let resume_info = if resume_enabled {
        load_resume_info(destination_path, false)?
    } else {
        ResumeInfo { bytes_copied: 0, partial_checksum: None, compressed_bytes: None }
    };

    let start_offset = resume_info.bytes_copied;
    if start_offset > 0 {
        println!("Resuming direct copy from byte {} of {}", start_offset, source_size);
    }

    let mut source_file = BufReader::new(File::open(source_path)?);
    source_file.seek(SeekFrom::Start(start_offset))?;

    let mut dest_file = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(start_offset > 0)
            .truncate(start_offset == 0)
            .open(destination_path)?
    );

    let mut buffer = vec![0u8; chunk_size];
    let mut bytes_copied = start_offset;
    let mut last_checkpoint = Instant::now();

    while bytes_copied < source_size {
        let remaining = (source_size - bytes_copied) as usize;
        let to_read = remaining.min(chunk_size);
        
        let n = source_file.read(&mut buffer[..to_read])?;
        if n == 0 {
            break;
        }
        
        dest_file.write_all(&buffer[..n])?;
        bytes_copied += n as u64;
        
        // Checkpoint every 5 seconds for resume capability
        if resume_enabled && last_checkpoint.elapsed() > Duration::from_secs(5) {
            dest_file.flush()?;
            save_resume_info(destination_path, bytes_copied, None, false)?;
            print!("\rProgress: {:.1}%", (bytes_copied as f64 / source_size as f64) * 100.0);
            std::io::stdout().flush()?;
            last_checkpoint = Instant::now();
        }
    }
    
    dest_file.flush()?;
    
    if resume_enabled {
        cleanup_resume_info(destination_path, false);
    }
    
    println!("\nDirect copy completed, {} bytes written.", bytes_copied);
    Ok(())
}

fn perform_compressed_copy(
    source_path: &PathBuf,
    destination_path: &PathBuf,
    source_size: u64,
    resume_enabled: bool,
    chunk_size: usize,
) -> Result<()> {
    let temp_compressed_path = destination_path.with_extension("tmp.lz4");
    let _cleanup_guard = TempFileCleanup::new(&temp_compressed_path);

    let resume_info = if resume_enabled {
        load_resume_info(destination_path, true)?
    } else {
        ResumeInfo { bytes_copied: 0, partial_checksum: None, compressed_bytes: None }
    };

    let start_offset = resume_info.bytes_copied;
    let compressed_start = resume_info.compressed_bytes.unwrap_or(0);

    // Compression phase (with resume support)
    if compressed_start == 0 || !temp_compressed_path.exists() {
        println!("Starting compression phase...");
        
        let mut source_file = BufReader::new(File::open(source_path)?);
        if start_offset > 0 {
            source_file.seek(SeekFrom::Start(start_offset))?;
            println!("Resuming compression from byte {}", start_offset);
        }

        let compressed_file = if start_offset > 0 && temp_compressed_path.exists() {
            OpenOptions::new().append(true).open(&temp_compressed_path)?
        } else {
            File::create(&temp_compressed_path)?
        };

        let mut encoder = EncoderBuilder::new().level(4).build(compressed_file)?;
        let mut buffer = vec![0u8; chunk_size];
        let mut bytes_read = start_offset;
        let mut last_checkpoint = Instant::now();

        while bytes_read < source_size {
            let remaining = (source_size - bytes_read) as usize;
            let to_read = remaining.min(chunk_size);
            
            let n = source_file.read(&mut buffer[..to_read])?;
            if n == 0 {
                break;
            }
            
            encoder.write_all(&buffer[..n])?;
            bytes_read += n as u64;
            
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
        
        let (_output, result) = encoder.finish();
        result?;
        
        let compressed_size = std::fs::metadata(&temp_compressed_path)?.len();
        let compression_ratio = (compressed_size as f64 / source_size as f64) * 100.0;
        println!("\nCompression completed: {} bytes -> {} bytes ({:.1}% of original)", 
                 source_size, compressed_size, compression_ratio);
    } else {
        println!("Using existing compressed file from previous attempt");
    }

    // Decompression phase
    {
        let compressed_input = BufReader::new(File::open(&temp_compressed_path)?);
        let mut decoder = Decoder::new(compressed_input)?;
        let mut decompressed_output = BufWriter::new(File::create(destination_path)?);
        
        let bytes_written = std::io::copy(&mut decoder, &mut decompressed_output)?;
        decompressed_output.flush()?;
        println!("Decompression completed, {} bytes written.", bytes_written);
        
        if bytes_written != source_size {
            return Err(anyhow::anyhow!(
                "Decompressed size mismatch: expected {} bytes, got {} bytes", 
                source_size, bytes_written
            ));
        }
    }

    if resume_enabled {
        cleanup_resume_info(destination_path, true);
    }

    Ok(())
}

fn load_resume_info(destination_path: &PathBuf, compressed: bool) -> Result<ResumeInfo> {
    let resume_file_path = get_resume_file_path(destination_path, compressed);
    
    if !resume_file_path.exists() {
        return Ok(ResumeInfo { 
            bytes_copied: 0, 
            partial_checksum: None, 
            compressed_bytes: None 
        });
    }

    let resume_data = std::fs::read_to_string(&resume_file_path)?;
    let lines: Vec<&str> = resume_data.lines().collect();
    
    if lines.is_empty() {
        return Ok(ResumeInfo { 
            bytes_copied: 0, 
            partial_checksum: None, 
            compressed_bytes: None 
        });
    }

    let bytes_copied: u64 = lines[0].parse().unwrap_or(0);
    let compressed_bytes = if lines.len() > 1 {
        lines[1].parse().ok()
    } else {
        None
    };

    println!("Found resume info: {} bytes copied", bytes_copied);
    
    Ok(ResumeInfo {
        bytes_copied,
        partial_checksum: None, // Could implement partial checksum resume in future
        compressed_bytes,
    })
}

fn save_resume_info(destination_path: &PathBuf, bytes_copied: u64, compressed_bytes: Option<u64>, compressed: bool) -> Result<()> {
    let resume_file_path = get_resume_file_path(destination_path, compressed);
    
    let mut content = bytes_copied.to_string();
    if let Some(cb) = compressed_bytes {
        content.push('\n');
        content.push_str(&cb.to_string());
    }
    
    std::fs::write(&resume_file_path, content)?;
    Ok(())
}

fn cleanup_resume_info(destination_path: &PathBuf, compressed: bool) {
    let resume_file_path = get_resume_file_path(destination_path, compressed);
    if resume_file_path.exists() {
        let _ = std::fs::remove_file(&resume_file_path);
    }
}

fn get_resume_file_path(destination_path: &PathBuf, compressed: bool) -> PathBuf {
    if compressed {
        destination_path.with_extension("orbit_resume_compressed")
    } else {
        destination_path.with_extension("orbit_resume")
    }
}

fn calculate_checksum(path: &PathBuf) -> Result<sha2::digest::Output<Sha256>> {
    let mut file = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize())
}

fn write_audit_log(
    source_path: &PathBuf,
    destination_path: &PathBuf,
    source_size: u64,
    duration: std::time::Duration,
    checksum: sha2::digest::Output<Sha256>,
    status: &str,
    attempts: u32,
) -> Result<()> {
    let mut log_file = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open("orbit_audit.log")
            .with_context(|| "Failed to open audit log")?
    );
    
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

// RAII cleanup helper to ensure temp files are always deleted
struct TempFileCleanup {
    path: PathBuf,
}

impl TempFileCleanup {
    fn new(path: &PathBuf) -> Self {
        Self { path: path.clone() }
    }
}

impl Drop for TempFileCleanup {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                eprintln!("Warning: Failed to clean up temporary file {:?}: {}", self.path, e);
            }
        }
    }
}