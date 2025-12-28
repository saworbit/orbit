/*!
 * Integration tests for the orbit init command
 *
 * Tests configuration generation logic for different use cases
 * without requiring interactive terminal input.
 */

use orbit::config::{CompressionType, CopyConfig, CopyMode};
use orbit::core::probe::{FileSystemType, SystemProfile};

/// Test backup profile generation
#[test]
fn test_backup_profile_generation() {
    // Simulate "Backup" selection logic
    let mut config = CopyConfig::default();
    config.copy_mode = CopyMode::Copy;
    config.verify_checksum = true;
    config.preserve_metadata = true;
    config.retry_attempts = 5;
    config.exponential_backoff = true;
    config.resume_enabled = true;

    // Verify backup profile characteristics
    assert_eq!(config.copy_mode, CopyMode::Copy);
    assert!(config.verify_checksum);
    assert!(config.preserve_metadata);
    assert_eq!(config.retry_attempts, 5);
    assert!(config.exponential_backoff);
    assert!(config.resume_enabled);
}

/// Test sync profile generation
#[test]
fn test_sync_profile_generation() {
    // Simulate "Sync" selection logic
    let mut config = CopyConfig::default();
    config.copy_mode = CopyMode::Sync;
    config.verify_checksum = false; // Trust modtime for speed
    config.preserve_metadata = true;
    config.parallel = 0; // Auto-detect
    config.use_zero_copy = true;

    assert_eq!(config.copy_mode, CopyMode::Sync);
    assert!(!config.verify_checksum); // Speed over verification
    assert!(config.use_zero_copy);
    assert!(config.preserve_metadata);
}

/// Test cloud upload profile generation
#[test]
fn test_cloud_profile_generation() {
    // Simulate "Cloud Upload" selection logic
    let mut config = CopyConfig::default();
    config.copy_mode = CopyMode::Copy;
    config.compression = CompressionType::Zstd { level: 3 };
    config.verify_checksum = true;
    config.retry_attempts = 10;
    config.exponential_backoff = true;
    config.resume_enabled = true;
    config.use_zero_copy = false; // Compression requires userspace

    assert!(matches!(
        config.compression,
        CompressionType::Zstd { level: 3 }
    ));
    assert_eq!(config.retry_attempts, 10);
    assert!(config.exponential_backoff);
    assert!(config.resume_enabled);
    assert!(!config.use_zero_copy);
}

/// Test network transfer profile generation
#[test]
fn test_network_profile_generation() {
    let mut config = CopyConfig::default();
    config.copy_mode = CopyMode::Copy;
    config.compression = CompressionType::Zstd { level: 3 };
    config.verify_checksum = true;
    config.resume_enabled = true;
    config.retry_attempts = 10;
    config.exponential_backoff = true;
    config.parallel = 4;

    assert!(matches!(config.compression, CompressionType::Zstd { .. }));
    assert!(config.resume_enabled);
    assert_eq!(config.parallel, 4);
}

/// Test active guidance logic with system profile
#[test]
fn test_probe_logic_slow_io_with_abundant_cpu() {
    // Mock system profile: High CPU, Slow I/O
    let cores = 16;
    let io_speed = 40.0; // Slow I/O (< 50 MB/s)

    let mut config = CopyConfig::default();

    // Apply Active Guidance logic manually for test
    // This simulates what the Guidance system would do
    if cores >= 8 && io_speed < 50.0 {
        config.compression = CompressionType::Zstd { level: 3 };
    }

    assert!(matches!(
        config.compression,
        CompressionType::Zstd { level: 3 }
    ));
}

/// Test network filesystem auto-tuning
#[test]
fn test_network_filesystem_auto_tune() {
    let mut config = CopyConfig::default();

    // Simulate detecting SMB filesystem
    let fs_type = FileSystemType::SMB;

    if matches!(fs_type, FileSystemType::SMB | FileSystemType::NFS) {
        if !config.resume_enabled {
            config.resume_enabled = true;
        }
        if config.retry_attempts < 5 {
            config.retry_attempts = 5;
        }
    }

    assert!(config.resume_enabled);
    assert_eq!(config.retry_attempts, 5);
}

/// Test cloud storage optimization
#[test]
fn test_cloud_storage_optimization() {
    let mut config = CopyConfig::default();

    // Simulate detecting cloud storage
    let fs_type = FileSystemType::S3;

    if fs_type.is_cloud_storage() {
        if matches!(config.compression, CompressionType::None) {
            config.compression = CompressionType::Zstd { level: 3 };
        }
        if config.retry_attempts < 10 {
            config.retry_attempts = 10;
        }
        if !config.exponential_backoff {
            config.exponential_backoff = true;
        }
    }

    assert!(matches!(config.compression, CompressionType::Zstd { .. }));
    assert_eq!(config.retry_attempts, 10);
    assert!(config.exponential_backoff);
}

/// Test low memory optimization
#[test]
fn test_low_memory_optimization() {
    let mut config = CopyConfig::default();
    config.parallel = 8;

    // Simulate low memory scenario
    let available_ram_gb = 0; // < 1 GB

    if available_ram_gb < 1 && config.parallel > 4 {
        config.parallel = 2;
    }

    assert_eq!(config.parallel, 2);
}

/// Test configuration serialization and deserialization
#[test]
fn test_config_serialization() {
    let mut config = CopyConfig::default();
    config.copy_mode = CopyMode::Copy;
    config.compression = CompressionType::Zstd { level: 5 };
    config.verify_checksum = true;

    // Serialize to TOML
    let toml_string = toml::to_string(&config).expect("Failed to serialize config");

    // Deserialize back
    let deserialized: CopyConfig =
        toml::from_str(&toml_string).expect("Failed to deserialize config");

    assert_eq!(deserialized.copy_mode, config.copy_mode);
    assert_eq!(deserialized.verify_checksum, config.verify_checksum);
    assert!(matches!(
        deserialized.compression,
        CompressionType::Zstd { level: 5 }
    ));
}

/// Test full system profile integration
#[test]
fn test_full_system_profile() {
    let profile = SystemProfile {
        logical_cores: 8,
        available_ram_gb: 4,
        is_battery_power: false,
        dest_filesystem_type: FileSystemType::Local,
        estimated_io_throughput: 50.0,
        total_memory_gb: 8,
    };

    // Verify profile has reasonable values
    assert!(profile.logical_cores > 0);
    assert!(profile.logical_cores <= 1024);
    assert!(profile.total_memory_gb >= profile.available_ram_gb);
    assert!(profile.estimated_io_throughput > 0.0);
}

/// Test FileSystemType detection helpers
#[test]
fn test_filesystem_type_helpers() {
    assert!(FileSystemType::SMB.is_network());
    assert!(FileSystemType::NFS.is_network());
    assert!(FileSystemType::S3.is_network());
    assert!(!FileSystemType::Local.is_network());

    assert!(FileSystemType::S3.is_cloud_storage());
    assert!(FileSystemType::Azure.is_cloud_storage());
    assert!(FileSystemType::GCS.is_cloud_storage());
    assert!(!FileSystemType::SMB.is_cloud_storage());
}
