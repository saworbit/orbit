# Disk Guardian: Pre-Flight Space & Integrity Checks

## Overview

The Disk Guardian module provides comprehensive pre-flight checks to prevent mid-transfer failures due to disk space exhaustion or filesystem issues. This feature prevents "disk full bombs" and ensures safe, reliable file transfers.

## Features

### 1. Enhanced Disk Space Validation

Unlike basic space checks, Disk Guardian includes:

- **Safety Margins**: Configurable percentage-based safety margin (default: 10%)
- **Minimum Free Space**: Always leaves a minimum amount of free space (default: 100 MB)
- **Early Detection**: Catches space issues before transfer begins
- **Usage Warnings**: Alerts when disk is >95% full

```rust
use orbit::core::disk_guardian::{ensure_transfer_safety, GuardianConfig};

let config = GuardianConfig {
    safety_margin_percent: 0.10,  // 10% extra space required
    min_free_space: 100 * 1024 * 1024,  // 100 MB minimum
    check_integrity: true,
    enable_watching: false,
};

ensure_transfer_safety(dest_path, required_bytes, &config)?;
```

### 2. Filesystem Integrity Checks

Validates the destination filesystem before transfer:

- **Write Permissions**: Tests actual write capability
- **Read-Only Detection**: Detects read-only mounts
- **Directory Creation**: Verifies ability to create directories

```rust
use orbit::core::disk_guardian::check_filesystem_integrity;

check_filesystem_integrity(dest_path)?;
```

### 3. Staging Area Support

Create temporary staging areas for atomic transfers:

```rust
use orbit::core::disk_guardian::create_staging_area;

// Creates a temporary directory that auto-cleans on drop
let staging = create_staging_area(dest_path)?;

// Perform transfer to staging area
copy_to_staging(source, staging.path())?;

// Move from staging to final destination (atomic)
std::fs::rename(staging.path(), dest_path)?;
```

### 4. Live Filesystem Watching (Optional)

Monitor filesystem events during long transfers:

```rust
use orbit::core::disk_guardian::DiskWatcher;

let watcher = DiskWatcher::new(dest_path, |event| {
    println!("Filesystem event: {:?}", event);
})?;

// Watcher remains active while in scope
perform_long_transfer()?;
```

### 5. Directory Size Estimation

Accurately estimate space needed for directory transfers:

```rust
use orbit::core::disk_guardian::estimate_directory_size;

let total_size = estimate_directory_size(source_dir)?;
println!("Transfer will require {} bytes", total_size);
```

## Integration with Orbit

### Automatic Integration

Disk Guardian is automatically integrated into Orbit's transfer operations:

1. **Single File Transfers**: Basic disk space check (backward compatible)
2. **Directory Transfers**: Full pre-flight check with size estimation

### Manual Integration

For custom use cases, use the enhanced validation:

```rust
use orbit::core::validation::validate_disk_space_enhanced;
use orbit::core::disk_guardian::GuardianConfig;

let config = GuardianConfig::default();
validate_disk_space_enhanced(dest_path, required_size, Some(&config))?;
```

## Configuration Options

### GuardianConfig

```rust
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
```

### Default Configuration

```rust
GuardianConfig {
    safety_margin_percent: 0.10,        // 10%
    min_free_space: 100 * 1024 * 1024,  // 100 MB
    check_integrity: true,
    enable_watching: false,
}
```

## Error Handling

Disk Guardian uses Orbit's standard error types:

```rust
use orbit::error::OrbitError;

match ensure_transfer_safety(dest, size, &config) {
    Ok(_) => println!("Ready to transfer"),
    Err(OrbitError::InsufficientDiskSpace { required, available }) => {
        eprintln!("Not enough space: need {}, have {}", required, available);
    }
    Err(OrbitError::MetadataFailed(msg)) => {
        eprintln!("Filesystem check failed: {}", msg);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Examples

### Example 1: Pre-flight Check Workflow

```rust
use orbit::core::disk_guardian::{estimate_directory_size, ensure_transfer_safety, GuardianConfig};

// Step 1: Estimate size
let estimated_size = estimate_directory_size(source_dir)?;
println!("Estimated: {} bytes", estimated_size);

// Step 2: Validate destination
let config = GuardianConfig::default();
ensure_transfer_safety(dest_dir, estimated_size, &config)?;

// Step 3: Proceed with transfer
copy_directory(source_dir, dest_dir, &copy_config)?;
```

### Example 2: Custom Safety Margins

```rust
// Conservative: 25% safety margin, 500 MB minimum free
let conservative = GuardianConfig {
    safety_margin_percent: 0.25,
    min_free_space: 500 * 1024 * 1024,
    check_integrity: true,
    enable_watching: false,
};

// Aggressive: 5% safety margin, 50 MB minimum free
let aggressive = GuardianConfig {
    safety_margin_percent: 0.05,
    min_free_space: 50 * 1024 * 1024,
    check_integrity: true,
    enable_watching: false,
};
```

### Example 3: Safe Transfer with Staging

```rust
use orbit::core::disk_guardian::create_staging_area;

// Create staging area
let staging = create_staging_area(dest_base_path)?;

// Transfer to staging
copy_file(source, &staging.path().join("file.dat"), &config)?;

// Verify integrity
verify_checksum(&staging.path().join("file.dat"))?;

// Atomic move to final location
std::fs::rename(
    staging.path().join("file.dat"),
    dest_path
)?;

// Staging auto-cleaned when dropped
```

## Running the Demo

```bash
cargo run --example disk_guardian_demo
```

This demonstrates:
- Basic disk space validation
- Custom safety margins
- Staging area creation
- Directory size estimation
- Complete pre-flight workflow

## Dependencies

- `sysinfo ^0.35` - System information and disk space queries
- `notify ^7.0` - Filesystem watching (optional feature)
- `tempfile ^3.10` - Temporary file/directory management

## Performance Considerations

1. **Directory Size Estimation**: Walks the entire directory tree
   - Use caching for repeated checks
   - Consider async estimation for large directories

2. **Filesystem Watching**: Overhead for event monitoring
   - Disabled by default
   - Enable only for critical long-running transfers

3. **Integrity Checks**: Minimal overhead
   - One-time check before transfer
   - Can be disabled if performance is critical

## Best Practices

1. **Always use pre-flight checks** for directory transfers
2. **Adjust safety margins** based on your use case
3. **Enable integrity checks** for critical transfers
4. **Use staging areas** for atomic transfers
5. **Monitor disk usage** during long transfers (optional)

## Future Enhancements

Potential future additions:
- Quota-aware space calculations
- Network filesystem detection
- Compression ratio estimation
- Multi-disk transfer orchestration
- Real-time space monitoring with callbacks
