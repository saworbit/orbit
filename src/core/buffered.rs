/*!
 * Buffered file copy with streaming checksum, resume, and progress tracking
 */

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};

use crate::config::CopyConfig;
use crate::error::Result;
use super::CopyStats;
use super::checksum::StreamingHasher;
use super::resume::{ResumeInfo, load_resume_info, save_resume_info, cleanup_resume_info};
use super::bandwidth;

/// Buffered copy with streaming checksum (original implementation)
///
/// This is the most compatible copy method that works across all filesystems
/// and supports resume, checksum verification, and bandwidth throttling.
pub fn copy_buffered(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
) -> Result<CopyStats> {
    let start_time = Instant::now();

    // Load resume info if enabled
    let resume_info = if config.resume_enabled {
        load_resume_info(dest_path, false)?
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
            save_resume_info(dest_path, bytes_copied, None, false)?;
            last_checkpoint = Instant::now();
        }

        // Bandwidth throttling
        if config.max_bandwidth > 0 {
            bandwidth::apply_limit(n as u64, config.max_bandwidth, &mut last_checkpoint);
        }
    }

    dest_file.flush()?;

    if let Some(pb) = progress {
        pb.finish_with_message("Complete");
    }

    // Clean up resume info
    if config.resume_enabled {
        cleanup_resume_info(dest_path, false);
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
