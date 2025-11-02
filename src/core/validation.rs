/*!
 * Validation logic for copy operations
 */

use std::path::Path;
use sysinfo::Disks;
use crate::config::{CopyMode, CopyConfig};
use crate::error::{OrbitError, Result};
use super::disk_guardian::{self, GuardianConfig};
use super::delta::{CheckMode, self};

/// Validate that sufficient disk space is available (basic check)
///
/// This is a backward-compatible wrapper that uses the default guardian configuration.
/// For more advanced checks with safety margins and integrity validation,
/// use `validate_disk_space_enhanced` instead.
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

/// Enhanced disk space validation with safety margins and integrity checks
///
/// This uses the disk_guardian module to provide:
/// - Safety margins (default 10% extra space)
/// - Minimum free space requirements
/// - Filesystem integrity checks (permissions, writability)
pub fn validate_disk_space_enhanced(
    destination_path: &Path,
    required_size: u64,
    config: Option<&GuardianConfig>,
) -> Result<()> {
    let default_config = GuardianConfig::default();
    let guardian_config = config.unwrap_or(&default_config);

    disk_guardian::ensure_transfer_safety(
        destination_path,
        required_size,
        guardian_config
    )
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

/// Determine if files need to be transferred based on check mode
pub fn files_need_transfer(
    source_path: &Path,
    dest_path: &Path,
    check_mode: CheckMode,
) -> Result<bool> {
    if !dest_path.exists() {
        return Ok(true);
    }

    let source_meta = std::fs::metadata(source_path)?;
    let dest_meta = std::fs::metadata(dest_path)?;

    match check_mode {
        CheckMode::ModTime => {
            Ok(source_meta.modified()? > dest_meta.modified()?
               || source_meta.len() != dest_meta.len())
        }
        CheckMode::Size => {
            Ok(source_meta.len() != dest_meta.len())
        }
        CheckMode::Checksum | CheckMode::Delta => {
            if source_meta.len() != dest_meta.len() {
                return Ok(true);
            }
            Ok(true)
        }
    }
}

/// Determine if delta transfer should be used for a file pair
pub fn should_use_delta_transfer(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
) -> Result<bool> {
    let delta_config = super::delta::DeltaConfig {
        check_mode: config.check_mode,
        block_size: config.delta_block_size,
        whole_file: config.whole_file,
        update_manifest: config.update_manifest,
        ignore_existing: config.ignore_existing,
        hash_algorithm: config.delta_hash_algorithm,
        parallel_hashing: config.parallel_hashing,
        manifest_path: config.delta_manifest_path.clone(),
    };

    delta::should_use_delta(source_path, dest_path, &delta_config)
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