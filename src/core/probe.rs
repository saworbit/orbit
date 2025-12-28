/*!
 * System Probe - Hardware Environment Detection
 *
 * This module provides active probing of the hardware environment to enable
 * intelligent auto-tuning of configuration parameters by the Guidance System.
 *
 * Version: 0.7.0
 * Phase: 4 - Active Guidance System
 */

use crate::error::Result;
use std::path::Path;
use std::time::Instant;
use sysinfo::{Disks, System};

/// System profile containing hardware and environment metrics
#[derive(Debug, Clone)]
pub struct SystemProfile {
    /// Number of logical CPU cores
    pub logical_cores: usize,

    /// Available RAM in GB
    pub available_ram_gb: u64,

    /// Running on battery power (always false on desktop systems)
    pub is_battery_power: bool,

    /// Detected filesystem type for destination
    pub dest_filesystem_type: FileSystemType,

    /// Estimated I/O throughput in MB/s
    pub estimated_io_throughput: f64,

    /// Total system memory in GB
    pub total_memory_gb: u64,
}

/// Filesystem type detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemType {
    /// Local filesystem (ext4, NTFS, APFS, etc.)
    Local,

    /// SMB/CIFS network share
    SMB,

    /// NFS network filesystem
    NFS,

    /// S3-compatible object storage
    S3,

    /// Azure Blob Storage
    Azure,

    /// Google Cloud Storage
    GCS,

    /// Unknown or undetectable filesystem
    Unknown,
}

impl FileSystemType {
    /// Returns true if this is a network-based filesystem
    pub fn is_network(&self) -> bool {
        matches!(
            self,
            Self::SMB | Self::NFS | Self::S3 | Self::Azure | Self::GCS
        )
    }

    /// Returns true if this is cloud object storage
    pub fn is_cloud_storage(&self) -> bool {
        matches!(self, Self::S3 | Self::Azure | Self::GCS)
    }
}

/// System probing utility
pub struct Probe;

impl Probe {
    /// Scan the system and destination environment to build a profile
    pub fn scan(dest_path: &Path) -> Result<SystemProfile> {
        let mut sys = System::new_all();
        sys.refresh_all();

        // 1. Detect CPU cores
        let logical_cores = sys.cpus().len().max(1); // Ensure at least 1

        // 2. Detect RAM (convert from bytes to GB)
        let available_ram_gb = sys.available_memory() / 1024 / 1024 / 1024;
        let total_memory_gb = sys.total_memory() / 1024 / 1024 / 1024;

        // 3. Create Disks instance for filesystem detection
        let disks = Disks::new_with_refreshed_list();

        // 4. Detect filesystem type
        let dest_filesystem_type = Self::detect_fs_type(dest_path, &disks);

        // 4. IO Micro-benchmark (write 10MB test)
        let estimated_io_throughput = Self::benchmark_io(dest_path);

        // 5. Battery status (future enhancement - placeholder)
        let is_battery_power = false;

        Ok(SystemProfile {
            logical_cores,
            available_ram_gb,
            is_battery_power,
            dest_filesystem_type,
            estimated_io_throughput,
            total_memory_gb,
        })
    }

    /// Detect filesystem type based on path and disk information
    fn detect_fs_type(path: &Path, disks: &Disks) -> FileSystemType {
        let path_str = path.to_string_lossy();

        // Check for URI-based protocols
        if path_str.starts_with("smb://") || path_str.starts_with("\\\\") {
            return FileSystemType::SMB;
        }
        if path_str.starts_with("nfs://") {
            return FileSystemType::NFS;
        }
        if path_str.starts_with("s3://") {
            return FileSystemType::S3;
        }
        if path_str.starts_with("az://") || path_str.starts_with("azure://") {
            return FileSystemType::Azure;
        }
        if path_str.starts_with("gs://") || path_str.starts_with("gcs://") {
            return FileSystemType::GCS;
        }

        // For local paths, check if it's on a network mount
        // Try to find the disk that contains this path
        for disk in disks.list() {
            let mount_point = disk.mount_point().to_string_lossy();
            if path_str.starts_with(mount_point.as_ref()) {
                let fs_type = disk.file_system().to_str().unwrap_or("");

                // Common network filesystem types
                if fs_type.contains("cifs") || fs_type.contains("smb") {
                    return FileSystemType::SMB;
                }
                if fs_type.contains("nfs") {
                    return FileSystemType::NFS;
                }

                // If we found a local filesystem
                return FileSystemType::Local;
            }
        }

        // Default to local if we can't determine
        FileSystemType::Local
    }

    /// Benchmark I/O throughput by writing a test file
    /// Returns throughput in MB/s
    fn benchmark_io(path: &Path) -> f64 {
        use std::fs;
        use std::io::Write;

        // Try to find a writable directory for the benchmark
        let test_dir = if path.is_dir() {
            path
        } else if let Some(parent) = path.parent() {
            parent
        } else {
            // Can't benchmark, return conservative estimate
            return 50.0;
        };

        // Create a unique test file
        let test_file = test_dir.join(format!(".orbit_bench_{}", std::process::id()));

        // Test data: 10MB of zeros
        let test_data = vec![0u8; 10 * 1024 * 1024];

        let start = Instant::now();

        // Attempt to write test data
        let throughput = match fs::File::create(&test_file) {
            Ok(mut file) => {
                match file.write_all(&test_data) {
                    Ok(_) => {
                        // Force sync to disk for accurate measurement
                        let _ = file.sync_all();
                        let elapsed = start.elapsed();

                        // Calculate MB/s
                        let mb_written = test_data.len() as f64 / (1024.0 * 1024.0);
                        let throughput = mb_written / elapsed.as_secs_f64();

                        // Clean up test file
                        let _ = fs::remove_file(&test_file);

                        throughput
                    }
                    Err(_) => {
                        // Write failed, return conservative estimate
                        let _ = fs::remove_file(&test_file);
                        50.0
                    }
                }
            }
            Err(_) => {
                // Can't create test file (read-only filesystem?), return conservative estimate
                50.0
            }
        };

        // Ensure we return a reasonable value
        throughput.clamp(1.0, 10000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_filesystem_type_detection() {
        let disks = Disks::new_with_refreshed_list();

        // Test URI-based detection
        assert_eq!(
            Probe::detect_fs_type(Path::new("smb://server/share"), &disks),
            FileSystemType::SMB
        );
        assert_eq!(
            Probe::detect_fs_type(Path::new("s3://bucket/key"), &disks),
            FileSystemType::S3
        );
        assert_eq!(
            Probe::detect_fs_type(Path::new("gs://bucket/key"), &disks),
            FileSystemType::GCS
        );
    }

    #[test]
    fn test_filesystem_type_is_network() {
        assert!(FileSystemType::SMB.is_network());
        assert!(FileSystemType::NFS.is_network());
        assert!(FileSystemType::S3.is_network());
        assert!(!FileSystemType::Local.is_network());
    }

    #[test]
    fn test_filesystem_type_is_cloud() {
        assert!(FileSystemType::S3.is_cloud_storage());
        assert!(FileSystemType::Azure.is_cloud_storage());
        assert!(FileSystemType::GCS.is_cloud_storage());
        assert!(!FileSystemType::SMB.is_cloud_storage());
        assert!(!FileSystemType::Local.is_cloud_storage());
    }

    #[test]
    fn test_system_scan() {
        let dir = tempdir().unwrap();
        let result = Probe::scan(dir.path());

        assert!(result.is_ok());
        let profile = result.unwrap();

        // Basic sanity checks
        assert!(profile.logical_cores > 0);
        assert!(profile.logical_cores <= 1024); // Reasonable upper bound
        assert!(profile.estimated_io_throughput > 0.0);
    }

    #[test]
    fn test_io_benchmark() {
        let dir = tempdir().unwrap();
        let throughput = Probe::benchmark_io(dir.path());

        // Should return a positive value
        assert!(throughput > 0.0);

        // Should be within reasonable bounds (1 MB/s to 10 GB/s)
        assert!(throughput >= 1.0);
        assert!(throughput <= 10000.0);
    }

    #[test]
    fn test_io_benchmark_readonly() {
        // Test with a path that likely doesn't exist or isn't writable
        let throughput = Probe::benchmark_io(Path::new("/nonexistent/path/that/should/not/exist"));

        // Should return the conservative fallback value
        assert_eq!(throughput, 50.0);
    }

    #[test]
    fn test_system_profile_values() {
        let dir = tempdir().unwrap();
        let profile = Probe::scan(dir.path()).unwrap();

        // Verify all fields are populated with reasonable values
        assert!(profile.logical_cores >= 1);
        assert!(profile.total_memory_gb > 0);
        assert!(profile.estimated_io_throughput >= 1.0);
    }
}
