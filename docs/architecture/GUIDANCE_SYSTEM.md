# Config Optimizer Architecture

**Version**: 2.0 (Enhanced with Active Probing)
**Modules**: `src/core/guidance.rs`, `src/core/probe.rs`
**Status**: Implemented

## Overview

The Config Optimizer is a configuration validation and optimization layer that sits between configuration loading and execution. It ensures that user-provided configurations are safe, logically consistent, and optimized for the specific hardware and use case.

**NEW in v0.7.0:** Enhanced with active environment probing and intelligent auto-tuning based on detected hardware, I/O performance, and destination type.

## Purpose

The Config Optimizer serves five critical functions:

1. **Sanitization**: Corrects invalid or dangerous combinations of flags
2. **Optimization**: Downgrades aggressive settings when hardware or logic dictates
3. **Notification**: Informs users why their configuration was changed through tiered notices
4. **⭐ Active Probing** (NEW v0.7.0): Detects system hardware and destination environment
5. **⭐ Auto-Tuning** (NEW v0.7.0): Optimizes based on detected CPU, RAM, I/O, and filesystem type

## Architecture Evolution

### Version 1.0 (v0.6.0 and earlier)
- Static validation rules
- Configuration conflict resolution
- Hardware capability detection (passive)

### Version 2.0 (v0.7.0)
- ✅ All v1.0 features
- ✅ **Active system probing** — CPU, RAM, I/O throughput
- ✅ **Destination type detection** — Local, SMB, NFS, S3, Azure, GCS
- ✅ **Environment-aware auto-tuning** — 4 new intelligent rules

## Design Pattern

The Config Optimizer follows the **Interceptor / Pre-processor** pattern:

```
Config Load → CLI Overrides → 🚀 CONFIG OPTIMIZER → Optimized Config → Execution
```

It acts as a gatekeeper, ensuring that no invalid configuration reaches the actual execution layer.

## Core Components

### 1. ConfigOptimizer Struct

The main entry point for the config optimizer. It provides a single static method:

```rust
pub struct ConfigOptimizer;

impl ConfigOptimizer {
    pub fn optimize(config: CopyConfig) -> Result<OptimizedConfig>
}
```

### 2. OptimizedConfig Struct

The output of the optimization check, containing both the optimized configuration and any notices generated:

```rust
pub struct OptimizedConfig {
    pub config: CopyConfig,
    pub notices: Vec<Notice>,
}
```

### 3. Notice System

Notices are categorized by severity and provide contextual information:

```rust
pub struct Notice {
    pub level: NoticeLevel,
    pub category: &'static str,
    pub message: String,
}

pub enum NoticeLevel {
    Info,          // Informational only, no changes made
    Warning,       // Configuration adjusted due to limitations
    Optimization,  // Configuration optimized for performance
    Safety,        // Configuration changed to prevent data corruption
    AutoTune,      // NEW v0.7.0: Environment-based auto-tuning
}
```

## Active Probing System (v0.7.0)

### 4. System Probe Module

**Location**: `src/core/probe.rs`

The probe module actively scans the system environment to gather metrics for intelligent decision-making:

```rust
pub struct SystemProfile {
    pub logical_cores: usize,              // CPU core count
    pub available_ram_gb: u64,             // Available RAM in GB
    pub total_memory_gb: u64,              // Total system memory
    pub is_battery_power: bool,            // Battery status (future)
    pub dest_filesystem_type: FileSystemType,  // Destination type
    pub estimated_io_throughput: f64,      // I/O speed in MB/s
}

pub enum FileSystemType {
    Local,   // Local filesystem
    SMB,     // SMB/CIFS network share
    NFS,     // NFS network filesystem
    S3,      // Amazon S3
    Azure,   // Azure Blob Storage
    GCS,     // Google Cloud Storage
    Unknown, // Could not determine
}

impl Probe {
    pub fn scan(dest_path: &Path) -> Result<SystemProfile>
}
```

**Detection Methods:**

1. **CPU Cores**: via `sysinfo` crate's CPU enumeration
2. **RAM**: via `sysinfo` crate's memory statistics
3. **I/O Throughput**: 10MB write benchmark to destination
4. **Filesystem Type**: URI parsing + mount point analysis

## Implemented Rules

### Static Validation Rules (v1.0)

The config optimizer enforces the following rules, listed in order of evaluation:

### Rule 1: Hardware Reality (Zero-Copy Support)

**Conflict**: Zero-copy requested but not supported by OS/hardware

**Resolution**: Disable zero-copy, emit Warning

**Rationale**: Zero-copy operations require specific kernel support. If the platform doesn't support it, we must fall back to buffered I/O.

**Example**:
```
⚠️  Hardware: Zero-copy not supported on linux (none). Disabling optimization.
```

### Rule 2: Integrity Strategy (Zero-Copy vs Checksum)

**Conflict**: Zero-copy + Checksum verification enabled

**Resolution**: Disable zero-copy, emit Optimization notice

**Rationale**: Zero-copy transfers data kernel-to-kernel without passing through userspace. To verify a checksum, we'd need to read the file again, effectively doubling I/O. It's faster to use buffered copy with streaming checksum calculation.

**Example**:
```
🚀 Strategy: Disabling zero-copy to allow streaming checksum verification (faster than Zero-Copy + Read-Back).
```

### Rule 3: The Integrity Paradox (Resume vs Checksum)

**Conflict**: Resume + Checksum verification enabled

**Resolution**: Disable checksum verification, emit Safety notice

**Rationale**: Streaming checksum verification requires reading the entire file from the beginning. When resuming a partial transfer, we skip the beginning, making full-file verification impossible.

**Example**:
```
🛡️  Integrity: Resume enabled; disabling streaming checksum verification (requires full file read).
```

### Rule 4: Data Safety (Resume vs Compression)

**Conflict**: Resume + Compression enabled

**Resolution**: Disable resume, emit Safety notice

**Rationale**: Standard compression streams (LZ4, Zstd) cannot be safely resumed by appending data. The compression context would be lost, corrupting the output.

**Example**:
```
🛡️  Safety: Disabling resume capability to prevent compressed stream corruption.
```

### Rule 5: Seeking Precision (Zero-Copy vs Resume)

**Conflict**: Zero-copy + Resume enabled

**Resolution**: Disable zero-copy, emit Optimization notice

**Rationale**: Zero-copy typically requires whole file descriptor or block transfers. Precise byte-level seeking for resume is more reliable with buffered I/O.

**Example**:
```
🚀 Precision: Resume enabled; disabling zero-copy to support precise offset seeking.
```

### Rule 6: The Observer Effect (Manifest vs Zero-Copy)

**Conflict**: Manifest generation + Zero-copy enabled

**Resolution**: Disable zero-copy, emit Optimization notice

**Rationale**: Manifest generation requires content inspection (hashing/chunking) which cannot be done when data stays in kernel space via zero-copy.

**Example**:
```
🚀 Visibility: Manifest generation requires content inspection. Disabling zero-copy.
```

### Rule 7: The Patchwork Problem (Delta vs Zero-Copy)

**Conflict**: Delta transfer mode + Zero-copy enabled

**Resolution**: Disable zero-copy, emit Optimization notice

**Rationale**: Delta transfers require application-level patch logic that zero-copy bypasses entirely.

**Example**:
```
🚀 Logic: Delta transfer active. Disabling zero-copy to handle patch application.
```

### Rule 8: The Speed Limit (macOS Bandwidth)

**Conflict**: macOS + Zero-copy + Bandwidth limit

**Resolution**: Disable zero-copy, emit Warning notice

**Rationale**: macOS's `fcopyfile` system call cannot be throttled. To enforce bandwidth limits, we must fall back to buffered I/O.

**Example**:
```
⚠️  Control: macOS zero-copy (fcopyfile) cannot be throttled. Disabling zero-copy to enforce limit.
```

### Rule 9: Visual Noise (Parallel vs Progress)

**Conflict**: Parallel transfers + Progress bars enabled

**Resolution**: No change, emit Info notice

**Rationale**: Running multiple progress bars concurrently may cause visual artifacts in the terminal output.

**Example**:
```
ℹ️  UX: Parallel transfer with progress bars may cause visual artifacts.
```

### Rule 10: Performance Warning (Sync/Update + Checksum)

**Conflict**: Sync/Update mode + Checksum check mode

**Resolution**: No change, emit Info notice

**Rationale**: This combination forces full file reads on both source and destination, which may be slower than expected. Users should be aware of the performance implications.

**Example**:
```
ℹ️  Performance: 'Checksum' check mode enabled with Sync/Update. This forces full file reads on both ends.
```

### Rule 11: Physics (Compression vs Encryption) - Placeholder

**Status**: Placeholder for future implementation

**Conflict**: Compression + Encryption enabled

**Resolution**: TBD (ensure compression runs before encryption or disable compression)

**Rationale**: Encrypted data has high entropy and cannot be effectively compressed. Compression must happen before encryption.

---

### Active Auto-Tuning Rules (v2.0)

These rules use system profiling to intelligently optimize configuration based on the detected environment.

### Active Rule 1: Network Share Auto-Tuning

**Trigger**: Destination is SMB or NFS filesystem

**Actions**:
- Enable resume capability if not already enabled
- Increase retry attempts to minimum 5 (if lower)

**Rationale**: Network filesystems are inherently unreliable compared to local storage. Resume allows recovery from connection drops, and increased retries handle transient network issues.

**Example**:
```
🔧 Network: Detected SMB destination. Enabling resume capability for reliability.
🔧 Network: Increased retry attempts to 5 for network filesystem reliability.
```

### Active Rule 2: CPU-Rich / IO-Poor Optimization

**Trigger**: System has ≥8 CPU cores AND I/O throughput <50 MB/s AND compression disabled

**Action**: Enable Zstd:3 compression

**Rationale**: When CPU is abundant but I/O is slow (e.g., high-end server with slow HDD), compression trades spare CPU cycles for reduced I/O operations. The net result is faster transfers despite compression overhead.

**Example**:
```
🔧 Performance: Detected slow I/O (42.3 MB/s) with 16 CPU cores.
                Enabling Zstd:3 compression to trade CPU for throughput.
```

### Active Rule 3: Low Memory Warning

**Trigger**: Available RAM <1 GB AND parallel operations >4

**Action**: Reduce parallel workers to 2

**Rationale**: Each parallel worker consumes memory for buffering. Low memory situations risk OOM errors or heavy swapping, which degrades performance more than reduced parallelism.

**Example**:
```
🔧 Memory: Low available memory (0 GB). Reduced parallel operations from 8 to 2.
```

### Active Rule 4: Cloud Storage Optimization

**Trigger**: Destination is S3, Azure Blob, or Google Cloud Storage

**Actions**:
- Enable Zstd:3 compression if not already enabled
- Increase retry attempts to minimum 10
- Enable exponential backoff

**Rationale**:
- Compression reduces network transfer size/cost
- Cloud APIs have transient failures requiring more retries
- Exponential backoff prevents API rate limiting/throttling

**Example**:
```
🔧 Cloud: Detected cloud storage destination. Enabling compression to reduce network transfer.
🔧 Cloud: Increased retry attempts to 10 for cloud storage reliability.
🔧 Cloud: Enabled exponential backoff for cloud API rate limiting.
```

---

## User Experience

When guidance rules are triggered, users see a formatted output:

**v0.7.0 Example (with Active Probing):**
```
┌── 🛰️  Orbit Config Optimizer ─────────────────────────┐
│ 🚀 Strategy: Disabling zero-copy to allow streaming checksum verification
│ 🛡️  Safety: Disabling resume capability to prevent compressed stream corruption
│ 🔧 Network: Detected SMB destination. Enabling resume for reliability.
│ 🔧 Performance: Detected slow I/O (45.2 MB/s) with 16 cores. Enabling Zstd:3.
└────────────────────────────────────────────────────────┘
```

**Notice Icons:**
- 🚀 **Optimization**: Performance improvement
- 🛡️ **Safety**: Data integrity protection
- ⚠️ **Warning**: Hardware/OS limitation
- ℹ️ **Info**: Informational notice
- 🔧 **AutoTune**: Environment-based optimization (NEW v0.7.0)

This provides:
- **Transparency**: Users understand what changed and why
- **Education**: Users learn about configuration trade-offs
- **Trust**: System doesn't fail silently or make hidden changes
- **Intelligence**: Auto-tuning adapts to your specific environment (v0.7.0)

## Integration Points

### Main Execution Flow

In `src/main.rs`, the config optimizer is called immediately after all CLI overrides:

**v0.7.0 (with Active Probing):**
```rust
// 1. Load raw configuration
let mut config = CopyConfig::from_file(config_path)?;

// 2. Apply CLI overrides
config.resume_enabled = cli.resume;
// ... (other overrides)

// 3. 🚀 CONFIG OPTIMIZER PASS: Sanitize and Optimize (with Active Probing)
let optimized = ConfigOptimizer::optimize_with_probe(config, Some(&dest_path))?;

// 4. Display notices to user
if !optimized.notices.is_empty() {
    println!("┌── 🛰️  Orbit Config Optimizer ─────────────────────────┐");
    for notice in &optimized.notices {
        println!("│ {}", notice);
    }
    println!("└────────────────────────────────────────────────────────┘");
}

// 5. Execute using optimized config
let config = optimized.final_config;
copy_file(&source, &dest, &config)?;
```

**Backward Compatibility:**
```rust
// Old API still works (no probing, static rules only)
let optimized = ConfigOptimizer::optimize(config)?;

// New API with active probing (recommended)
let optimized = ConfigOptimizer::optimize_with_probe(config, Some(&dest_path))?;
```

## Testing

The config optimizer includes comprehensive testing at two levels:

### Unit Tests (`src/core/guidance.rs`)

Tests individual rules in isolation:
- `test_safety_resume_vs_compression` - Rule 4
- `test_strategy_zerocopy_vs_checksum` - Rule 2
- `test_paradox_resume_vs_checksum` - Rule 3
- `test_observer_manifest_vs_zerocopy` - Rule 6
- `test_patchwork_delta_vs_zerocopy` - Rule 7
- `test_speed_limit_macos_bandwidth` - Rule 8 (macOS only)
- `test_visual_noise_parallel_progress` - Rule 9
- `test_performance_warning_sync_checksum` - Rule 10
- `test_clean_config_minimal_notices`
- `test_multiple_rules_triggered`
- `test_notice_display_format`

### Integration Tests (`tests/guidance_integration.rs`)

Tests the guidance system in real-world scenarios:
- `test_guidance_resume_vs_compression_safety`
- `test_guidance_zerocopy_vs_checksum_optimization`
- `test_guidance_zerocopy_vs_resume_precision`
- `test_guidance_sync_checksum_performance_info`
- `test_guidance_multiple_rules_triggered`
- `test_guidance_clean_config_no_notices`
- `test_guidance_with_actual_copy_operation`
- `test_guidance_display_format`
- `test_guidance_preserves_other_config_options`
- `test_guidance_cli_output` - CLI integration test
- `test_guidance_manifest_vs_zerocopy` - Rule 6
- `test_guidance_delta_vs_zerocopy` - Rule 7
- `test_guidance_parallel_progress_ux` - Rule 9
- `test_guidance_resume_vs_checksum_integrity` - Rule 3

## Adding New Rules

To add a new guidance rule:

1. **Add the check** in `ConfigOptimizer::optimize()`:
   ```rust
   if config.feature_a && config.feature_b {
       notices.push(Notice {
           level: NoticeLevel::Safety,
           category: "Category",
           message: "Explanation of what changed and why".to_string(),
       });
       config.feature_a = false; // or adjust as needed
   }
   ```

2. **Add a unit test** in `src/core/guidance.rs`:
   ```rust
   #[test]
   fn test_new_rule_feature_a_vs_feature_b() {
       let mut config = CopyConfig::default();
       config.feature_a = true;
       config.feature_b = true;

       let result = ConfigOptimizer::optimize(config).unwrap();

       assert_eq!(result.config.feature_a, false);
       assert!(result.notices.iter().any(|n| n.category == "Category"));
   }
   ```

3. **Document the rule** in this file under "Implemented Rules"

4. **Update CHANGELOG.md** with the new rule

## Philosophy: User Intent vs Technical Reality

The Config Optimizer embodies a key design principle of Orbit:

> **Users express intent. The system ensures technical correctness.**

Rather than rejecting invalid configurations with errors, we:
1. Understand what the user is trying to achieve
2. Automatically correct the configuration to achieve that goal safely
3. Explain what we changed and why

This approach:
- **Reduces friction**: Users don't need to be experts to get correct behavior
- **Educates**: Users learn from the notices about configuration trade-offs
- **Maintains trust**: Changes are transparent and well-justified

## Error Handling

The config optimizer is designed to never fail. Even if individual capability detection fails, we fall back to safe defaults. The only error case is if the configuration is fundamentally invalid at the type level (which should be caught earlier).

## Performance Impact

The config optimizer adds minimal overhead:
- Single pass through configuration (~10-20 boolean checks)
- No I/O operations (except one-time capability detection)
- Negligible impact compared to actual file transfer time

## Future Enhancements

Potential improvements for future versions:

1. **Configuration Profiles**: Named presets that encode common rule outcomes
   ```bash
   orbit --profile=fast  # Implies zero-copy, no checksum, no compression
   orbit --profile=safe  # Implies resume, checksum, retry
   ```

2. **Rule Priorities**: Allow users to hint at priorities when multiple rules conflict
   ```bash
   orbit --prefer=speed  # Favor zero-copy over checksum
   orbit --prefer=safety # Favor resume over performance
   ```

3. **Dry-Run Mode for Guidance**: Show what would be changed without executing
   ```bash
   orbit --guidance-only source.txt dest.txt
   ```

4. **Machine-Readable Output**: JSON format for tool integration
   ```bash
   orbit --guidance-format=json
   ```

## References

### Implementation
- **Config Optimizer Core**: [src/core/guidance.rs](../../src/core/guidance.rs)
- **System Probe**: [src/core/probe.rs](../../src/core/probe.rs) ⭐ NEW v0.7.0
- **Init Wizard**: [src/commands/init.rs](../../src/commands/init.rs) ⭐ NEW v0.7.0
- **Integration**: [src/main.rs](../../src/main.rs)

### Tests
- **Guidance Tests**: [tests/guidance_integration.rs](../../tests/guidance_integration.rs)
- **Init Tests**: [tests/init_generation_test.rs](../../tests/init_generation_test.rs) ⭐ NEW v0.7.0
- **Probe Tests**: In [src/core/probe.rs](../../src/core/probe.rs) (7 tests)

### User Documentation
- **Init Wizard Guide**: [docs/guides/INIT_WIZARD_GUIDE.md](../guides/INIT_WIZARD_GUIDE.md) ⭐ NEW v0.7.0
- **Active Guidance Guide**: [docs/guides/ACTIVE_GUIDANCE_GUIDE.md](../guides/ACTIVE_GUIDANCE_GUIDE.md) ⭐ NEW v0.7.0

### Related Architecture
- [Zero-Copy Architecture](ZERO_COPY.md)
- [Resume System](RESUME_SYSTEM.md)
- [Manifest System](MANIFEST_SYSTEM.md)
