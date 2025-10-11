/*!
 * Validation logic for copy operations
 */

use std::path::Path;
use sysinfo::Disks;
use crate::config::CopyMode;
use crate::error::{OrbitError, Result};

/// Validate that sufficient disk space is available
pub fn validate_disk_space(destination_path: &Path, required_size: u64) -> Result<()> {
    let disks = Disks::new_with_refreshed_list();
    
    let destination_disk = disks.iter().find(|disk| {
        destination_path.starts_with(disk.mount_point())
    });
    
    if let Some(disk) = destination_disk {
        if disk.available_space() < required_size {
            return Err(OrbitError::InsufficientDiskSpace {
                required: required_size,
                available: disk.available_space(),
            });
        }
    } else {
        eprintln!("Warning: Could not determine available disk space");
    }
    
    Ok(())
}

/// Determine if a file should be copied based on the copy mode
pub fn should_copy_file(source_path: &Path, dest_path: &Path, mode: CopyMode) -> Result<bool> {
    // Always copy if destination doesn't exist
    if !dest_path.exists() {
        return Ok(true);
    }
    
    match mode {
        CopyMode::Copy => Ok(true),
        CopyMode::Sync | CopyMode::Update => {
            let source_meta = std::fs::metadata(source_path)?;
            let dest_meta = std::fs::metadata(dest_path)?;
            
            // Copy if source is newer or different size
            Ok(source_meta.modified()? > dest_meta.modified()? 
               || source_meta.len() != dest_meta.len())
        }
        CopyMode::Mirror => Ok(true),
    }
}

/// Check if a path matches any exclude patterns
pub fn matches_exclude_pattern(path: &Path, patterns: &[String]) -> bool {
    use glob::Pattern;
    
    let path_str = path.to_string_lossy();
    
    patterns.iter().any(|pattern| {
        Pattern::new(pattern)
            .ok()
            .map(|p| p.matches(&path_str))
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_should_copy_copy_mode() {
        let source = NamedTempFile::new().unwrap();
        let dest = NamedTempFile::new().unwrap();
        
        assert!(should_copy_file(source.path(), dest.path(), CopyMode::Copy).unwrap());
    }
    
    #[test]
    fn test_should_copy_sync_mode_newer() {
        let mut source = NamedTempFile::new().unwrap();
        let mut dest = NamedTempFile::new().unwrap();
        
        dest.write_all(b"old").unwrap();
        dest.flush().unwrap();
        
        thread::sleep(Duration::from_millis(100));
        
        source.write_all(b"new").unwrap();
        source.flush().unwrap();
        
        assert!(should_copy_file(source.path(), dest.path(), CopyMode::Sync).unwrap());
    }
    
    #[test]
    fn test_matches_exclude_pattern() {
        let path = Path::new("/tmp/test.tmp");
        let patterns = vec!["*.tmp".to_string(), "*.log".to_string()];
        
        assert!(matches_exclude_pattern(path, &patterns));
        
        let path2 = Path::new("/tmp/test.txt");
        assert!(!matches_exclude_pattern(path2, &patterns));
    }
}