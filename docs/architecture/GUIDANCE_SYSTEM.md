# Guidance System Architecture

**Version**: 1.0
**Module**: `src/core/guidance.rs`
**Status**: Implemented

## Overview

The Guidance System (internally called the "Flight Computer") is a configuration validation and optimization layer that sits between configuration loading and execution. It ensures that user-provided configurations are safe, logically consistent, and optimized for the specific hardware and use case.

## Purpose

The Guidance System serves three critical functions:

1. **Sanitization**: Corrects invalid or dangerous combinations of flags
2. **Optimization**: Downgrades aggressive settings when hardware or logic dictates
3. **Notification**: Informs users why their configuration was changed through tiered notices

## Design Pattern

The Guidance System follows the **Interceptor / Pre-processor** pattern:

```
Config Load → CLI Overrides → 🚀 GUIDANCE → Optimized Config → Execution
```

It acts as a gatekeeper, ensuring that no invalid configuration reaches the actual execution layer.

## Core Components

### 1. Guidance Struct

The main entry point for the guidance system. It provides a single static method:

```rust
pub struct Guidance;

impl Guidance {
    pub fn plan(config: CopyConfig) -> Result<FlightPlan>
}
```

### 2. FlightPlan Struct

The output of the guidance check, containing both the optimized configuration and any notices generated:

```rust
pub struct FlightPlan {
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
}
```

## Implemented Rules

The guidance system enforces the following rules, listed in order of evaluation:

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

## User Experience

When guidance rules are triggered, users see a formatted output:

```
┌── 🛰️  Orbit Guidance System ───────────────────────┐
│ 🚀 Strategy: Disabling zero-copy to allow streaming checksum verification
│ 🛡️  Safety: Disabling resume capability to prevent compressed stream corruption
└────────────────────────────────────────────────────┘
```

This provides:
- **Transparency**: Users understand what changed and why
- **Education**: Users learn about configuration trade-offs
- **Trust**: System doesn't fail silently or make hidden changes

## Integration Points

### Main Execution Flow

In `src/main.rs`, guidance is called immediately after all CLI overrides:

```rust
// 1. Load raw configuration
let mut config = CopyConfig::from_file(config_path)?;

// 2. Apply CLI overrides
config.resume_enabled = cli.resume;
// ... (other overrides)

// 3. 🚀 GUIDANCE PASS: Sanitize and Optimize
let flight_plan = Guidance::plan(config)?;

// 4. Display notices to user
if !flight_plan.notices.is_empty() {
    println!("┌── 🛰️  Orbit Guidance System ───────────────────────┐");
    for notice in &flight_plan.notices {
        println!("│ {}", notice);
    }
    println!("└────────────────────────────────────────────────────┘");
}

// 5. Execute using optimized config
let config = flight_plan.config;
copy_file(&source, &dest, &config)?;
```

## Testing

The guidance system includes comprehensive testing at two levels:

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

1. **Add the check** in `Guidance::plan()`:
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

       let plan = Guidance::plan(config).unwrap();

       assert_eq!(plan.config.feature_a, false);
       assert!(plan.notices.iter().any(|n| n.category == "Category"));
   }
   ```

3. **Document the rule** in this file under "Implemented Rules"

4. **Update CHANGELOG.md** with the new rule

## Philosophy: User Intent vs Technical Reality

The Guidance System embodies a key design principle of Orbit:

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

The guidance system is designed to never fail. Even if individual capability detection fails, we fall back to safe defaults. The only error case is if the configuration is fundamentally invalid at the type level (which should be caught earlier).

## Performance Impact

The guidance system adds minimal overhead:
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

- Implementation: [src/core/guidance.rs](../../src/core/guidance.rs)
- Integration: [src/main.rs](../../src/main.rs)
- Tests: [tests/guidance_integration.rs](../../tests/guidance_integration.rs)
- Related: [Zero-Copy Architecture](ZERO_COPY.md)
- Related: [Resume System](RESUME_SYSTEM.md)
