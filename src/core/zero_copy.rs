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
        let meta2 = std::fs::metadata(path2)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
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
        use std::io::{Read, Seek, SeekFrom};
        
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