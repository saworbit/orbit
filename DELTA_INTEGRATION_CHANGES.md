# Delta Integration - Changes Summary

This document tracks the changes needed to integrate delta detection into Orbit.

## Files to Modify

### 1. src/core/mod.rs
- Add `pub mod delta;` after other module declarations
- Add `delta_stats: Option<delta::DeltaStats>` to CopyStats struct
- Update CopyStats::new() to include `delta_stats: None`
- Add `with_delta()` method to CopyStats
- Update all CopyStats initializers in this file to include `delta_stats: None`

### 2. src/config.rs
- Add delta-related imports
- Add 8 delta fields to CopyConfig struct
- Add `default_delta_block_size()` function
- Update CopyConfig::default() to initialize delta fields

### 3. src/main.rs
- Add CheckModeArg enum and From impl
- Add CLI flags for delta (--check, --block-size, --whole-file, etc.)
- Update config building to set delta fields from CLI

### 4. src/core/validation.rs
- Add delta imports
- Add `files_need_transfer()` function
- Add `should_use_delta_transfer()` function

### 5. src/core/transfer.rs
- Add delta imports
- Update `copy_direct()` to check for delta first
- Add `copy_with_delta_integration()` function

### 6. Fix all CopyStats initializers
- src/core/zero_copy.rs
- src/core/buffered.rs
- src/core/directory.rs
- src/compression/mod.rs (2 locations)

All need `delta_stats: None` added to CopyStats struct initialization.
