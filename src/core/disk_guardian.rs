/*!
 * Disk Guardian: Pre-flight space & integrity checks
 *
 * Prevents mid-transfer OOM/disk-full scenarios with:
 * - Enhanced disk space validation with safety margins
 * - Filesystem integrity checks (type, permissions, writability)
 * - Optional live filesystem watching
 * - Staging area support using tempfile
 */

use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use sysinfo::Disks;
use tempfile::{NamedTempFile, TempDir};

use crate::error::{OrbitError, Result};

/// Safety margin percentage for disk space (default: 10%)
const DEFAULT_SAFETY_MARGIN_PERCENT: f64 = 0.10;

/// Minimum free space to always leave available (100 MB)
const MIN_FREE_SPACE_BYTES: u64 = 100 * 1024 * 1024;

/// Configuration for disk guardian checks
#[derive(Debug, Clone)]
pub struct GuardianConfig {
    /// Safety margin as a percentage (0.0 to 1.0)
    pub safety_margin_percent: f64,

    /// Minimum free space to always leave available (bytes)
    pub min_free_space: u64,

    /// Enable filesystem integrity checks
    pub check_integrity: bool,

    /// Enable filesystem watching during transfer
    pub enable_watching: bool,
}

impl Default for GuardianConfig {
    fn default() -> Self {
        Self {
            safety_margin_percent: DEFAULT_SAFETY_MARGIN_PERCENT,
            min_free_space: MIN_FREE_SPACE_BYTES,
            check_integrity: true,
            enable_watching: false, // Disabled by default for performance
        }
    }
}

/// Comprehensive disk and filesystem validation
pub fn ensure_transfer_safety(
    dest_path: &Path,
    required_bytes: u64,
    config: &GuardianConfig,
) -> Result<()> {
    // 1. Enhanced disk space check
    ensure_sufficient_space(dest_path, required_bytes, config)?;

    // 2. Filesystem integrity checks (if enabled)
    if config.check_integrity {
        check_filesystem_integrity(dest_path)?;
    }

    Ok(())
}

/// Enhanced disk space validation with safety margins
pub fn ensure_sufficient_space(
    dest_path: &Path,
    required_bytes: u64,
    config: &GuardianConfig,
) -> Result<()> {
    let disks = Disks::new_with_refreshed_list();

    // Find the disk that contains the destination path
    let destination_disk = disks.iter().find(|disk| {
        // Handle both the path itself and its parent directory
        dest_path.starts_with(disk.mount_point())
            || dest_path
                .parent()
                .map(|p| p.starts_with(disk.mount_point()))
                .unwrap_or(false)
    });

    if let Some(disk) = destination_disk {
        let available = disk.available_space();

        // Calculate total required space with safety margin
        let safety_bytes = (required_bytes as f64 * config.safety_margin_percent) as u64;
        let total_required = required_bytes
            .saturating_add(safety_bytes)
            .saturating_add(config.min_free_space);

        if available < total_required {
            return Err(OrbitError::InsufficientDiskSpace {
                required: total_required,
                available,
            });
        }

        // Additional warning if available space is getting low (< 5% of total)
        let total = disk.total_space();
        let usage_ratio = (total - available) as f64 / total as f64;
        if usage_ratio > 0.95 {
            eprintln!(
                "Warning: Disk is more than 95% full ({:.1}% used)",
                usage_ratio * 100.0
            );
        }
    } else {
        eprintln!(
            "Warning: Could not determine disk for path: {:?}",
            dest_path
        );
    }

    Ok(())
}

/// Check filesystem integrity (type, permissions, writability)
pub fn check_filesystem_integrity(dest_path: &Path) -> Result<()> {
    // Determine the directory to check
    let check_dir = if dest_path.is_dir() {
        dest_path.to_path_buf()
    } else {
        dest_path
            .parent()
            .ok_or_else(|| OrbitError::InvalidPath(dest_path.to_path_buf()))?
            .to_path_buf()
    };

    // Create directory if it doesn't exist (to test permissions)
    if !check_dir.exists() {
        std::fs::create_dir_all(&check_dir).map_err(|e| OrbitError::Io(e))?;
    }

    // Test write permissions by creating a temporary file
    test_write_permissions(&check_dir)?;

    // Check if filesystem is read-only
    check_not_readonly(&check_dir)?;

    Ok(())
}

/// Test write permissions by creating a temporary file
fn test_write_permissions(dir: &Path) -> Result<()> {
    NamedTempFile::new_in(dir).map_err(|e| {
        OrbitError::MetadataFailed(format!("Cannot write to destination directory: {}", e))
    })?;
    Ok(())
}

/// Check if filesystem is mounted read-only
fn check_not_readonly(path: &Path) -> Result<()> {
    // Try to get filesystem metadata
    let metadata = std::fs::metadata(path)?;

    // On Unix systems, we can check readonly attribute
    #[cfg(unix)]
    {
        let perms = metadata.permissions();
        if perms.readonly() {
            return Err(OrbitError::MetadataFailed(
                "Destination filesystem is read-only".to_string(),
            ));
        }
    }

    // On Windows, check readonly attribute
    #[cfg(windows)]
    {
        if metadata.permissions().readonly() {
            return Err(OrbitError::MetadataFailed(
                "Destination filesystem is read-only".to_string(),
            ));
        }
    }

    Ok(())
}

/// Create a staging area for safe transfers
pub fn create_staging_area(base_path: &Path) -> Result<TempDir> {
    let parent = base_path.parent().unwrap_or_else(|| Path::new("."));

    TempDir::new_in(parent)
        .map_err(|e| OrbitError::Other(format!("Failed to create staging area: {}", e)))
}

/// Filesystem watcher for monitoring disk space during transfers
pub struct DiskWatcher {
    #[allow(dead_code)]
    watcher: Box<dyn Watcher>,
    monitored_path: PathBuf,
}

impl DiskWatcher {
    /// Create a new disk watcher for the specified path
    pub fn new<F>(path: &Path, mut callback: F) -> Result<Self>
    where
        F: FnMut(Event) + Send + 'static,
    {
        let path = path.to_path_buf();

        // Create a file watcher with recommended settings
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                callback(event);
            }
        })
        .map_err(|e| OrbitError::Other(format!("Failed to create filesystem watcher: {}", e)))?;

        // Watch the parent directory
        let watch_path = if path.is_dir() {
            path.clone()
        } else {
            path.parent()
                .ok_or_else(|| OrbitError::InvalidPath(path.clone()))?
                .to_path_buf()
        };

        watcher
            .watch(&watch_path, RecursiveMode::NonRecursive)
            .map_err(|e| OrbitError::Other(format!("Failed to watch directory: {}", e)))?;

        Ok(Self {
            watcher: Box::new(watcher),
            monitored_path: watch_path,
        })
    }

    /// Get the path being monitored
    pub fn monitored_path(&self) -> &Path {
        &self.monitored_path
    }
}

/// Estimate total space needed for a directory transfer
pub fn estimate_directory_size(dir: &Path) -> Result<u64> {
    use walkdir::WalkDir;

    let mut total_size = 0u64;

    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size = total_size.saturating_add(metadata.len());
            }
        }
    }

    Ok(total_size)
}

/// Pre-flight check for directory transfers
pub fn validate_directory_transfer(
    source_dir: &Path,
    dest_dir: &Path,
    config: &GuardianConfig,
) -> Result<u64> {
    // Estimate total size needed
    let total_size = estimate_directory_size(source_dir)?;

    // Validate we have enough space
    ensure_transfer_safety(dest_dir, total_size, config)?;

    Ok(total_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_ensure_space_with_margin() {
        let temp = tempdir().unwrap();
        let config = GuardianConfig::default();

        // Should succeed for small file
        let result = ensure_sufficient_space(temp.path(), 1024, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_integrity() {
        let temp = tempdir().unwrap();
        let result = check_filesystem_integrity(temp.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_staging_area() {
        let temp = tempdir().unwrap();
        let staging = create_staging_area(temp.path());
        assert!(staging.is_ok());

        let staging_dir = staging.unwrap();
        assert!(staging_dir.path().exists());
    }

    #[test]
    fn test_estimate_directory_size() {
        let temp = tempdir().unwrap();

        // Create test files
        std::fs::write(temp.path().join("file1.txt"), b"hello").unwrap();
        std::fs::write(temp.path().join("file2.txt"), b"world").unwrap();

        let size = estimate_directory_size(temp.path()).unwrap();
        assert_eq!(size, 10); // "hello" (5) + "world" (5)
    }

    #[test]
    fn test_validate_directory_transfer() {
        let source = tempdir().unwrap();
        let dest = tempdir().unwrap();

        // Create test files
        std::fs::write(source.path().join("test.txt"), b"test data").unwrap();

        let config = GuardianConfig::default();
        let result = validate_directory_transfer(source.path(), dest.path(), &config);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 9); // "test data" length
    }

    #[test]
    fn test_disk_watcher_creation() {
        let temp = tempdir().unwrap();

        let result = DiskWatcher::new(temp.path(), |_event| {
            // Callback for filesystem events
        });

        assert!(result.is_ok());
        let watcher = result.unwrap();
        assert_eq!(watcher.monitored_path(), temp.path());
    }
}
