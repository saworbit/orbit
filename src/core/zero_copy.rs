/*!
 * Zero-copy file transfer using platform-specific system calls
 * 
 * This module provides optimized file copying that bypasses userspace buffers
 * by using kernel-level operations like copy_file_range (Linux), CopyFileExW (Windows),
 * and sendfile (Unix-like systems).
 */

use std::fs::File;
use std::io;
use std::path::Path;

/// Result of attempting a zero-copy operation
pub enum ZeroCopyResult {
    /// Successfully copied the specified number of bytes
    Success(u64),
    /// Zero-copy not supported (fallback to buffered copy needed)
    Unsupported,
    /// Zero-copy failed with a retriable error
    Failed(io::Error),
}

/// Capabilities for zero-copy operations on this platform
#[derive(Debug, Clone)]
pub struct ZeroCopyCapabilities {
    pub available: bool,
    pub cross_filesystem: bool,
    pub method: &'static str,
}

impl ZeroCopyCapabilities {
    /// Detect available zero-copy capabilities at runtime
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            Self {
                available: true,
                cross_filesystem: false, // copy_file_range requires same FS
                method: "copy_file_range",
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            Self {
                available: true,
                cross_filesystem: true,
                method: "CopyFileExW",
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            Self {
                available: true,
                cross_filesystem: false,
                method: "copyfile",
            }
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            Self {
                available: false,
                cross_filesystem: false,
                method: "none",
            }
        }
    }
}

/// Attempt to copy a file using zero-copy system calls
/// 
/// # Arguments
/// * `source` - Source file (must be opened for reading)
/// * `dest` - Destination file (must be opened for writing)
/// * `offset` - Starting offset in bytes (for resume support)
/// * `len` - Number of bytes to copy
/// 
/// # Returns
/// * `ZeroCopyResult::Success(n)` - Successfully copied n bytes
/// * `ZeroCopyResult::Unsupported` - Zero-copy not available, use buffered copy
/// * `ZeroCopyResult::Failed(err)` - Operation failed with error
pub fn try_zero_copy(
    source: &File,
    dest: &File,
    offset: u64,
    len: u64,
) -> ZeroCopyResult {
    if len == 0 {
        return ZeroCopyResult::Success(0);
    }
    
    #[cfg(target_os = "linux")]
    {
        linux::copy_file_range_loop(source, dest, offset, len)
    }
    
    #[cfg(target_os = "windows")]
    {
        windows::copy_file_ex(source, dest, offset, len)
    }
    
    #[cfg(target_os = "macos")]
    {
        macos::copyfile_wrapper(source, dest, offset, len)
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        ZeroCopyResult::Unsupported
    }
}

/// Check if two paths are on the same filesystem
/// This is important because copy_file_range requires same FS on Linux
pub fn same_filesystem(path1: &Path, path2: &Path) -> io::Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta1 = std::fs::metadata(path1)?;
        let meta2 = match std::fs::metadata(path2) {
            Ok(meta) => meta,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if let Some(parent) = path2.parent() {
                    std::fs::metadata(parent)?
                } else {
                    return Ok(false);
                }
            }
            Err(e) => return Err(e),
        };
        Ok(meta1.dev() == meta2.dev())
    }
    
    #[cfg(windows)]
    {
        // On Windows, check if drive letters match
        let vol1 = get_volume_path(path1)?;
        let vol2 = get_volume_path(path2)?;
        Ok(vol1 == vol2)
    }
    
    #[cfg(not(any(unix, windows)))]
    {
        // Conservative: assume different filesystems
        Ok(false)
    }
}

#[cfg(windows)]
fn get_volume_path(path: &Path) -> io::Result<String> {
    // Extract drive letter or UNC path prefix
    let path_str = path.to_string_lossy();
    if let Some(prefix) = path_str.split(':').next() {
        Ok(prefix.to_string())
    } else {
        Ok(String::new())
    }
}

// ============================================================================
// Linux implementation using copy_file_range
// ============================================================================

#[cfg(target_os = "linux")]
mod linux {
    use super::ZeroCopyResult;
    use std::fs::File;
    use std::io;
    use std::os::unix::io::AsRawFd;
    
    pub fn copy_file_range_loop(
        source: &File,
        dest: &File,
        offset: u64,
        len: u64,
    ) -> ZeroCopyResult {
        let mut total_copied = 0u64;
        let mut src_offset = offset as i64;
        let mut dst_offset = offset as i64;
        
        while total_copied < len {
            let remaining = len - total_copied;
            
            // copy_file_range can copy at most isize::MAX bytes
            let to_copy = remaining.min(isize::MAX as u64) as usize;
            
            match unsafe {
                libc::syscall(
                    libc::SYS_copy_file_range,
                    source.as_raw_fd(),
                    &mut src_offset as *mut i64,
                    dest.as_raw_fd(),
                    &mut dst_offset as *mut i64,
                    to_copy,
                    0u32, // flags
                )
            } {
                -1 => {
                    let err = io::Error::last_os_error();
                    
                    // Check for specific error codes that indicate unsupported
                    match err.raw_os_error() {
                        Some(libc::ENOSYS) | Some(libc::EXDEV) | Some(libc::EOPNOTSUPP) => {
                            return ZeroCopyResult::Unsupported;
                        }
                        _ => {
                            return ZeroCopyResult::Failed(err);
                        }
                    }
                }
                0 => {
                    // EOF reached or no bytes copied
                    break;
                }
                n => {
                    total_copied += n as u64;
                }
            }
        }
        
        ZeroCopyResult::Success(total_copied)
    }
}

// ============================================================================
// Windows implementation using CopyFileExW
// ============================================================================

#[cfg(target_os = "windows")]
mod windows {
    use super::ZeroCopyResult;
    use std::fs::File;
    
    pub fn copy_file_ex(
        _source: &File,
        _dest: &File,
        _offset: u64,
        _len: u64,
    ) -> ZeroCopyResult {
        // Windows CopyFileExW works at the path level, not file descriptor level
        // For now, we'll return Unsupported and implement path-based copying
        // in a separate function that can be called before files are opened
        // This is a limitation we'll need to document
        ZeroCopyResult::Unsupported
    }
}

// ============================================================================
// macOS implementation using copyfile
// ============================================================================

#[cfg(target_os = "macos")]
mod macos {
    use super::ZeroCopyResult;
    use std::fs::File;
    use std::io;
    use std::os::unix::io::AsRawFd;
    
    pub fn copyfile_wrapper(
        source: &File,
        dest: &File,
        offset: u64,
        len: u64,
    ) -> ZeroCopyResult {
        // macOS has fcopyfile which works on file descriptors
        // For simplicity, we'll use sendfile which is more portable
        sendfile_loop(source, dest, offset, len)
    }
    
    fn sendfile_loop(
        source: &File,
        dest: &File,
        offset: u64,
        len: u64,
    ) -> ZeroCopyResult {
        let mut total_copied = 0u64;
        let mut current_offset = offset as i64;
        
        while total_copied < len {
            let remaining = (len - total_copied) as i64;
            
            let result = unsafe {
                let mut bytes_written: libc::off_t = remaining;
                libc::sendfile(
                    dest.as_raw_fd(),
                    source.as_raw_fd(),
                    current_offset,
                    &mut bytes_written as *mut libc::off_t,
                    std::ptr::null_mut(),
                    0,
                )
            };
            
            if result == -1 {
                let err = io::Error::last_os_error();
                match err.raw_os_error() {
                    Some(libc::ENOSYS) | Some(libc::EOPNOTSUPP) => {
                        return ZeroCopyResult::Unsupported;
                    }
                    _ => {
                        return ZeroCopyResult::Failed(err);
                    }
                }
            }
            
            // sendfile updates current_offset automatically
            // bytes_written contains the actual bytes transferred
            if result == 0 {
                break; // EOF
            }
            
            total_copied += result as u64;
        }
        
        ZeroCopyResult::Success(total_copied)
    }
}

// ============================================================================
// Zero-copy heuristics and orchestration
// ============================================================================

use std::fs::OpenOptions;
use std::time::Instant;
use indicatif::{ProgressBar, ProgressStyle};
use crate::config::CopyConfig;
use crate::error::{OrbitError, Result};
use super::CopyStats;
use super::checksum;
use super::progress::ProgressPublisher;

/// Determine if zero-copy should be attempted based on heuristics
///
/// Returns true if conditions are favorable for zero-copy:
/// - Zero-copy is available on this platform
/// - No resume needed (complex offset handling works better with buffered)
/// - No bandwidth throttling (need granular control)
/// - Files on same filesystem (if required by platform)
/// - File size >= 64KB (small files have syscall overhead)
pub fn should_use_zero_copy(
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
        let same_fs = same_filesystem(source_path, dest_path)?;
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

/// Attempt zero-copy transfer with progress tracking and checksum verification
///
/// This function attempts a zero-copy transfer and handles:
/// - Progress bar updates
/// - Progress event emission
/// - Post-copy checksum verification
/// - File descriptor management
///
/// Returns `Err(OrbitError::ZeroCopyUnsupported)` if zero-copy is not available,
/// allowing the caller to fall back to buffered copy.
pub fn try_zero_copy_direct(
    source_path: &Path,
    dest_path: &Path,
    source_size: u64,
    config: &CopyConfig,
    publisher: &ProgressPublisher,
) -> Result<CopyStats> {
    let start_time = Instant::now();

    // Emit transfer start event
    let file_id = publisher.start_transfer(
        source_path.to_path_buf(),
        dest_path.to_path_buf(),
        source_size,
    );

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
    let result = try_zero_copy(&source_file, &dest_file, 0, source_size);

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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_capabilities_detection() {
        let caps = ZeroCopyCapabilities::detect();
        
        #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
        assert!(caps.available);
        
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        assert!(!caps.available);
    }
    
    #[test]
    fn test_same_filesystem() {
        let temp1 = NamedTempFile::new().unwrap();
        let temp2 = NamedTempFile::new().unwrap();
        
        // Both temp files should be on the same filesystem
        let result = same_filesystem(temp1.path(), temp2.path());
        assert!(result.is_ok());
        // Note: We can't assert true because temp dirs might be on different mounts
    }
    
    #[cfg(target_os = "linux")]
    #[test]
    fn test_zero_copy_basic() {
        use std::io::{Read, Seek, SeekFrom, Write};

        let mut source = NamedTempFile::new().unwrap();
        let mut dest = NamedTempFile::new().unwrap();

        // Write test data
        let test_data = b"Hello, zero-copy world!";
        source.write_all(test_data).unwrap();
        source.flush().unwrap();
        source.seek(SeekFrom::Start(0)).unwrap();
        
        // Attempt zero-copy
        let result = try_zero_copy(
            source.as_file(),
            dest.as_file(),
            0,
            test_data.len() as u64,
        );
        
        match result {
            ZeroCopyResult::Success(n) => {
                assert_eq!(n, test_data.len() as u64);
                
                // Verify data
                dest.seek(SeekFrom::Start(0)).unwrap();
                let mut buffer = Vec::new();
                dest.read_to_end(&mut buffer).unwrap();
                assert_eq!(&buffer[..], test_data);
            }
            ZeroCopyResult::Unsupported => {
                // Acceptable on older kernels
                println!("Zero-copy not supported on this system");
            }
            ZeroCopyResult::Failed(e) => {
                panic!("Zero-copy failed: {}", e);
            }
        }
    }
}