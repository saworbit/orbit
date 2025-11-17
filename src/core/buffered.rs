/*!
 * Buffered file copy with streaming checksum, resume, and progress tracking
 */

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};

use super::bandwidth::BandwidthLimiter;
use super::checksum::StreamingHasher;
use super::progress::ProgressPublisher;
use super::resume::{
    cleanup_resume_info, decide_resume_strategy, load_resume_info, record_chunk_digest,
    save_resume_info_full, validate_chunks, ResumeDecision, ResumeInfo,
};
use super::CopyStats;
use crate::config::CopyConfig;
use crate::error::Result;
use tracing::{debug, info};

/// Buffered copy with streaming checksum (original implementation)
///
/// This is the most compatible copy method that works across all filesystems
/// and supports resume, checksum verification, and bandwidth throttling.
///
/// Emits progress events through the provided publisher for real-time monitoring.
pub fn copy_buffered(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
    publisher: &ProgressPublisher,
) -> Result<CopyStats> {
    let start_time = Instant::now();

    // Setup bandwidth limiter with token bucket algorithm
    let bandwidth_limiter = BandwidthLimiter::new(config.max_bandwidth);
    if bandwidth_limiter.is_enabled() {
        info!(
            "Bandwidth limiting enabled: {} bytes/sec",
            config.max_bandwidth
        );
    }

    // Emit transfer start event
    let file_id = publisher.start_transfer(
        source_path.to_path_buf(),
        dest_path.to_path_buf(),
        source_size,
    );

    // Load resume info if enabled
    let mut resume_info = if config.resume_enabled {
        load_resume_info(dest_path, false)?
    } else {
        ResumeInfo::default()
    };

    // Get source file metadata for resume decision
    let source_metadata = std::fs::metadata(source_path)?;
    let source_mtime = source_metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    // Store source metadata in resume info for future validation
    resume_info.file_size = Some(source_size);
    resume_info.file_mtime = source_mtime;

    // Decide resume strategy
    let resume_decision = if config.resume_enabled && resume_info.bytes_copied > 0 {
        decide_resume_strategy(dest_path, &resume_info)
    } else {
        ResumeDecision::StartFresh
    };

    // Emit resume decision event
    match &resume_decision {
        ResumeDecision::Resume {
            from_offset,
            verified_chunks,
        } => {
            publisher.publish_resume_decision(
                &file_id,
                "Resume".to_string(),
                *from_offset,
                *verified_chunks,
                None,
            );
        }
        ResumeDecision::Revalidate { reason } => {
            publisher.publish_resume_decision(
                &file_id,
                "Revalidate".to_string(),
                0,
                0,
                Some(reason.clone()),
            );
        }
        ResumeDecision::Restart { reason } => {
            publisher.publish_resume_decision(
                &file_id,
                "Restart".to_string(),
                0,
                0,
                Some(reason.clone()),
            );
        }
        ResumeDecision::StartFresh => {
            publisher.publish_resume_decision(&file_id, "StartFresh".to_string(), 0, 0, None);
        }
    }

    // Determine start offset based on decision
    let start_offset = match resume_decision {
        ResumeDecision::Resume { from_offset, .. } => {
            println!(
                "Resuming from byte {} ({} chunks verified)",
                from_offset,
                resume_info.verified_chunks.len()
            );
            from_offset
        }
        ResumeDecision::Revalidate { ref reason } => {
            println!("Revalidating file: {}", reason);
            println!("Re-hashing from beginning but preserving partial transfer");
            resume_info.bytes_copied
        }
        ResumeDecision::Restart { ref reason } => {
            println!("Restarting transfer: {}", reason);
            if dest_path.exists() {
                std::fs::remove_file(dest_path)?;
            }
            cleanup_resume_info(dest_path, false);
            resume_info = ResumeInfo::default();
            resume_info.file_size = Some(source_size);
            resume_info.file_mtime = source_mtime;
            0
        }
        ResumeDecision::StartFresh => 0,
    };

    // Open source file
    let mut source_file = BufReader::new(File::open(source_path)?);
    if start_offset > 0 {
        source_file.seek(SeekFrom::Start(start_offset))?;
    }

    // Open destination file
    let mut dest_file = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(start_offset > 0)
            .truncate(start_offset == 0)
            .open(dest_path)?,
    );

    // Setup progress bar
    let progress = if config.show_progress {
        let pb = ProgressBar::new(source_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
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

    // Validate existing chunks if in Revalidate mode
    if matches!(resume_decision, ResumeDecision::Revalidate { .. })
        && resume_info.verified_chunks.len() > 0
    {
        println!(
            "Validating {} existing chunks...",
            resume_info.verified_chunks.len()
        );
        match validate_chunks(
            dest_path,
            &resume_info,
            config.chunk_size,
            publisher,
            &file_id,
        ) {
            Ok(failures) => {
                if failures > 0 {
                    println!(
                        "Warning: {} chunks failed validation, will be re-verified",
                        failures
                    );
                    resume_info.verified_chunks.clear();
                } else {
                    println!("All chunks validated successfully");
                }
            }
            Err(e) => {
                println!("Chunk validation error: {}, clearing verified chunks", e);
                resume_info.verified_chunks.clear();
            }
        }
    }

    // Copy loop
    let mut buffer = vec![0u8; config.chunk_size];
    let mut bytes_copied = start_offset;
    let mut last_checkpoint = Instant::now();
    let mut last_progress_event = Instant::now();
    let progress_interval = Duration::from_millis(500); // Emit progress events every 500ms

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

        // Record chunk digest if this is a complete chunk and resume is enabled
        if config.resume_enabled && n == config.chunk_size {
            let chunk_id = (bytes_copied / config.chunk_size as u64) as u32;
            record_chunk_digest(chunk_id, &buffer[..n], &mut resume_info);
        }

        bytes_copied += n as u64;

        // Update progress bar
        if let Some(ref pb) = progress {
            pb.set_position(bytes_copied);
        }

        // Emit progress event periodically
        if last_progress_event.elapsed() >= progress_interval {
            publisher.update_progress(&file_id, bytes_copied, source_size);
            last_progress_event = Instant::now();
        }

        // Checkpoint for resume
        if config.resume_enabled && last_checkpoint.elapsed() > Duration::from_secs(5) {
            dest_file.flush()?;
            resume_info.bytes_copied = bytes_copied;
            save_resume_info_full(dest_path, &resume_info, false)?;
            last_checkpoint = Instant::now();
        }

        // Bandwidth throttling using token bucket algorithm
        if bandwidth_limiter.is_enabled() {
            let throttle_start = Instant::now();
            bandwidth_limiter.wait_for_capacity(n as u64);
            let throttle_duration = throttle_start.elapsed();

            // Log throttle events for monitoring
            if throttle_duration > Duration::from_millis(10) {
                debug!(
                    "Bandwidth throttle: waited {:?} for {} bytes",
                    throttle_duration, n
                );
            }
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

    let duration = start_time.elapsed();

    // Emit transfer complete event
    publisher.complete_transfer(
        file_id,
        bytes_copied,
        duration.as_millis() as u64,
        checksum.clone(),
    );

    Ok(CopyStats {
        bytes_copied,
        duration,
        checksum,
        compression_ratio: None,
        files_copied: 1,
        files_skipped: 0,
        files_failed: 0,
        delta_stats: None,
    })
}
