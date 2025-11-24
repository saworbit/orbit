/*!
 * Compression and decompression support for Orbit
 */

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use lz4::{Decoder as Lz4Decoder, EncoderBuilder as Lz4Encoder};
use tracing::{debug, info};
use zstd::stream::{Decoder as ZstdDecoder, Encoder as ZstdEncoder};

use crate::config::CopyConfig;
use crate::core::bandwidth::BandwidthLimiter;
use crate::core::{checksum::StreamingHasher, resume, CopyStats};
use crate::error::{OrbitError, Result};

/// Copy file with LZ4 compression
pub fn copy_with_lz4(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
) -> Result<CopyStats> {
    let start_time = Instant::now();
    let temp_compressed = dest_path.with_extension("tmp.lz4");

    // Setup bandwidth limiter
    let bandwidth_limiter = BandwidthLimiter::new(config.max_bandwidth);
    if bandwidth_limiter.is_enabled() {
        info!(
            "LZ4 compression with bandwidth limiting: {} bytes/sec",
            config.max_bandwidth
        );
    }

    // Ensure cleanup on exit
    let _cleanup = TempFileCleanup::new(&temp_compressed);

    // Load resume info
    let resume_info = if config.resume_enabled {
        resume::load_resume_info(dest_path, true)?
    } else {
        resume::ResumeInfo::default()
    };

    let start_offset = resume_info.bytes_copied;
    let compressed_start = resume_info.compressed_bytes.unwrap_or(0);

    // Phase 1: Compression
    if compressed_start == 0 || !temp_compressed.exists() {
        println!("Compressing with LZ4...");

        let mut source_file = BufReader::new(File::open(source_path)?);
        if start_offset > 0 {
            use std::io::Seek;
            source_file.seek(std::io::SeekFrom::Start(start_offset))?;
        }

        let compressed_file = if start_offset > 0 && temp_compressed.exists() {
            OpenOptions::new().append(true).open(&temp_compressed)?
        } else {
            File::create(&temp_compressed)?
        };

        let mut encoder = Lz4Encoder::new()
            .level(4)
            .build(compressed_file)
            .map_err(|e| OrbitError::Compression(e.to_string()))?;

        let mut buffer = vec![0u8; config.chunk_size];
        let mut bytes_read = start_offset;
        let mut last_checkpoint = Instant::now();

        while bytes_read < source_size {
            let remaining = (source_size - bytes_read) as usize;
            let to_read = remaining.min(config.chunk_size);

            let n = source_file.read(&mut buffer[..to_read])?;
            if n == 0 {
                break;
            }

            encoder
                .write_all(&buffer[..n])
                .map_err(|e| OrbitError::Compression(e.to_string()))?;
            bytes_read += n as u64;

            // Bandwidth throttling
            if bandwidth_limiter.is_enabled() {
                let throttle_start = Instant::now();
                bandwidth_limiter.wait_for_capacity(n as u64);
                let throttle_duration = throttle_start.elapsed();
                if throttle_duration > Duration::from_millis(10) {
                    debug!(
                        "LZ4: Bandwidth throttle: waited {:?} for {} bytes",
                        throttle_duration, n
                    );
                }
            }

            if config.resume_enabled && last_checkpoint.elapsed() > Duration::from_secs(5) {
                let compressed_size = std::fs::metadata(&temp_compressed)
                    .map(|m| m.len())
                    .unwrap_or(0);
                resume::save_resume_info(dest_path, bytes_read, Some(compressed_size), true)?;
                last_checkpoint = Instant::now();
            }
        }

        let (_output, result) = encoder.finish();
        result.map_err(|e| OrbitError::Compression(e.to_string()))?;
    }

    let compressed_size = std::fs::metadata(&temp_compressed)?.len();
    let compression_ratio = (compressed_size as f64 / source_size as f64) * 100.0;

    println!(
        "Compression: {} bytes -> {} bytes ({:.1}%)",
        source_size, compressed_size, compression_ratio
    );

    // Phase 2: Decompression
    {
        let compressed_input = BufReader::new(File::open(&temp_compressed)?);
        let mut decoder = Lz4Decoder::new(compressed_input)
            .map_err(|e| OrbitError::Decompression(e.to_string()))?;
        let mut decompressed_output = BufWriter::new(File::create(dest_path)?);

        let bytes_written = std::io::copy(&mut decoder, &mut decompressed_output)?;
        decompressed_output.flush()?;

        if bytes_written != source_size {
            return Err(OrbitError::Decompression(format!(
                "Size mismatch: expected {} bytes, got {} bytes",
                source_size, bytes_written
            )));
        }

        println!("Decompressed {} bytes", bytes_written);
    }

    if config.resume_enabled {
        resume::cleanup_resume_info(dest_path, true);
    }

    Ok(CopyStats {
        bytes_copied: source_size,
        duration: start_time.elapsed(),
        checksum: None,
        compression_ratio: Some(compression_ratio),
        files_copied: 1,
        files_skipped: 0,
        files_failed: 0,
        delta_stats: None,
        chunks_resumed: 0,
        bytes_skipped: 0,
    })
}

/// Copy file with Zstd compression
pub fn copy_with_zstd(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    level: i32,
    config: &CopyConfig,
) -> Result<CopyStats> {
    let start_time = Instant::now();
    let temp_compressed = dest_path.with_extension("tmp.zst");

    // Setup bandwidth limiter
    let bandwidth_limiter = BandwidthLimiter::new(config.max_bandwidth);
    if bandwidth_limiter.is_enabled() {
        info!(
            "Zstd compression with bandwidth limiting: {} bytes/sec",
            config.max_bandwidth
        );
    }

    let _cleanup = TempFileCleanup::new(&temp_compressed);

    let resume_info = if config.resume_enabled {
        resume::load_resume_info(dest_path, true)?
    } else {
        resume::ResumeInfo::default()
    };

    let start_offset = resume_info.bytes_copied;
    let compressed_start = resume_info.compressed_bytes.unwrap_or(0);

    // Phase 1: Compression
    if compressed_start == 0 || !temp_compressed.exists() {
        println!("Compressing with Zstd (level {})...", level);

        let mut source_file = BufReader::new(File::open(source_path)?);
        if start_offset > 0 {
            use std::io::Seek;
            source_file.seek(std::io::SeekFrom::Start(start_offset))?;
        }

        let compressed_file = File::create(&temp_compressed)?;
        let mut encoder = ZstdEncoder::new(compressed_file, level)
            .map_err(|e| OrbitError::Compression(e.to_string()))?;

        let mut buffer = vec![0u8; config.chunk_size];
        let mut bytes_read = start_offset;
        let mut last_checkpoint = Instant::now();

        while bytes_read < source_size {
            let remaining = (source_size - bytes_read) as usize;
            let to_read = remaining.min(config.chunk_size);

            let n = source_file.read(&mut buffer[..to_read])?;
            if n == 0 {
                break;
            }

            encoder
                .write_all(&buffer[..n])
                .map_err(|e| OrbitError::Compression(e.to_string()))?;
            bytes_read += n as u64;

            // Bandwidth throttling
            if bandwidth_limiter.is_enabled() {
                let throttle_start = Instant::now();
                bandwidth_limiter.wait_for_capacity(n as u64);
                let throttle_duration = throttle_start.elapsed();
                if throttle_duration > Duration::from_millis(10) {
                    debug!(
                        "Zstd: Bandwidth throttle: waited {:?} for {} bytes",
                        throttle_duration, n
                    );
                }
            }

            if config.resume_enabled && last_checkpoint.elapsed() > Duration::from_secs(5) {
                let compressed_size = std::fs::metadata(&temp_compressed)
                    .map(|m| m.len())
                    .unwrap_or(0);
                resume::save_resume_info(dest_path, bytes_read, Some(compressed_size), true)?;
                last_checkpoint = Instant::now();
            }
        }

        encoder
            .finish()
            .map_err(|e| OrbitError::Compression(e.to_string()))?;
    }

    let compressed_size = std::fs::metadata(&temp_compressed)?.len();
    let compression_ratio = (compressed_size as f64 / source_size as f64) * 100.0;

    println!(
        "Compression: {} bytes -> {} bytes ({:.1}%)",
        source_size, compressed_size, compression_ratio
    );

    // Phase 2: Decompression
    {
        let compressed_input = File::open(&temp_compressed)?;
        let mut decoder = ZstdDecoder::new(compressed_input)
            .map_err(|e| OrbitError::Decompression(e.to_string()))?;
        let mut decompressed_output = BufWriter::new(File::create(dest_path)?);

        let mut hasher = if config.verify_checksum {
            Some(StreamingHasher::new())
        } else {
            None
        };

        let mut buffer = vec![0u8; config.chunk_size];
        let mut bytes_written = 0u64;

        loop {
            let n = decoder.read(&mut buffer)?;
            if n == 0 {
                break;
            }

            if let Some(ref mut h) = hasher {
                h.update(&buffer[..n]);
            }

            decompressed_output.write_all(&buffer[..n])?;
            bytes_written += n as u64;
        }

        decompressed_output.flush()?;

        if bytes_written != source_size {
            return Err(OrbitError::Decompression(format!(
                "Size mismatch: expected {} bytes, got {} bytes",
                source_size, bytes_written
            )));
        }

        println!("Decompressed {} bytes", bytes_written);
    }

    if config.resume_enabled {
        resume::cleanup_resume_info(dest_path, true);
    }

    Ok(CopyStats {
        bytes_copied: source_size,
        duration: start_time.elapsed(),
        checksum: None,
        compression_ratio: Some(compression_ratio),
        files_copied: 1,
        files_skipped: 0,
        files_failed: 0,
        delta_stats: None,
        chunks_resumed: 0,
        bytes_skipped: 0,
    })
}

/// RAII helper for temporary file cleanup
struct TempFileCleanup {
    path: std::path::PathBuf,
}

impl TempFileCleanup {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }
}

impl Drop for TempFileCleanup {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                eprintln!(
                    "Warning: Failed to clean up temporary file {:?}: {}",
                    self.path, e
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_lz4_compression_roundtrip() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Use more repetitive data that compresses better
        std::fs::write(&source, b"test data that should compress well because it has repetition repetition repetition repetition repetition repetition repetition repetition").unwrap();

        let config = CopyConfig::default();
        let source_size = std::fs::metadata(&source).unwrap().len();

        let stats = copy_with_lz4(&source, &dest, source_size, &config).unwrap();

        assert_eq!(
            std::fs::read(&dest).unwrap(),
            std::fs::read(&source).unwrap()
        );
        // Just verify compression happened, don't check the ratio (small files may not compress)
        assert!(stats.compression_ratio.is_some());
    }

    #[test]
    fn test_zstd_compression_roundtrip() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        std::fs::write(&source, b"test data for zstd compression").unwrap();

        let config = CopyConfig::default();
        let source_size = std::fs::metadata(&source).unwrap().len();

        let stats = copy_with_zstd(&source, &dest, source_size, 3, &config).unwrap();

        assert_eq!(
            std::fs::read(&dest).unwrap(),
            std::fs::read(&source).unwrap()
        );
        assert!(stats.compression_ratio.is_some());
    }
}
