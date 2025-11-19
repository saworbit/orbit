# Resume System

Orbit's resume system provides intelligent recovery from interrupted file transfers with chunk-level verification and smart resume decision logic.

## Overview

The resume system allows Orbit to:
- **Resume interrupted transfers** from the exact byte offset where they stopped
- **Verify file integrity** using chunk-level digest tracking
- **Detect file modifications** and decide whether to resume, revalidate, or restart
- **Track transfer progress** with detailed telemetry for monitoring

## Features

### 1. Chunk-Level Digest Tracking

Every complete chunk (default 256KB) is hashed using BLAKE3 during transfer. These digests are stored in the resume info file, allowing:
- Verification of already-transferred chunks
- Fast-forward through verified ranges
- Detection of partial file corruption

```rust
// Resume info structure
pub struct ResumeInfo {
    pub bytes_copied: u64,
    pub compressed_bytes: Option<u64>,
    pub verified_chunks: HashMap<u32, String>,  // chunk_id -> BLAKE3 hex digest
    pub verified_windows: Vec<u32>,              // manifest window IDs
    pub file_mtime: Option<u64>,                 // modification time
    pub file_size: Option<u64>,                  // expected file size
}
```

### 2. Smart Resume Decision Logic

When resuming a transfer, Orbit analyzes the destination file and resume metadata to make an intelligent decision:

#### Resume (Safe)
- Destination file exists and matches expected size
- Modification time unchanged (within 2-second tolerance)
- File size matches resume progress
- **Action**: Continue from last checkpoint offset

```
🔄 Resume Decision: Resume
   Offset: 10.50 MB (42 chunks verified)
```

#### Revalidate (Suspicious)
- File was modified (newer timestamp)
- File grew beyond resume offset
- **Action**: Re-hash existing data, continue transfer

```
🔄 Resume Decision: Revalidate
   Reason: File was modified (127 sec newer than resume info)
```

#### Restart (Corrupted)
- Destination file missing
- File size mismatch
- File truncated below resume offset
- **Action**: Delete destination, start from beginning

```
🔄 Resume Decision: Restart
   Reason: File size mismatch: expected 10485760, found 5242880
```

#### StartFresh
- No previous resume info
- Fresh transfer start

### 3. Resume File Format

Resume information is stored in JSON format alongside the destination file:

```
destination.dat         # The actual file being transferred
destination.dat.resume  # Resume metadata (JSON)
```

Example resume file:
```json
{
  "bytes_copied": 10485760,
  "compressed_bytes": null,
  "verified_chunks": {
    "0": "a1b2c3d4e5f6...hex",
    "1": "f6e5d4c3b2a1...hex",
    "40": "1234567890ab...hex"
  },
  "verified_windows": [0, 1, 2],
  "file_mtime": 1730449856,
  "file_size": 104857600
}
```

**Backward Compatibility**: Legacy text-format resume files are still supported:
```
1024
```

### 4. Progress Telemetry Integration

Resume operations emit structured events through the progress telemetry system:

#### ResumeDecision Event
```json
{
  "type": "resume_decision",
  "file_id": "source.dat -> dest.dat",
  "decision": "Resume",
  "from_offset": 10485760,
  "verified_chunks": 42,
  "reason": null,
  "timestamp": 1730449856
}
```

#### ChunkVerification Event
```json
{
  "type": "chunk_verification",
  "file_id": "source.dat -> dest.dat",
  "chunk_id": 5,
  "chunk_size": 262144,
  "timestamp": 1730449856
}
```

#### ChunkVerified Event
```json
{
  "type": "chunk_verified",
  "file_id": "source.dat -> dest.dat",
  "chunk_id": 5,
  "digest": "a1b2c3d4e5f6789...full_blake3_hex",
  "timestamp": 1730449857
}
```

## Usage

### Enabling Resume

Resume is enabled by default for all transfers:

```bash
# Resume automatically enabled
orbit --source /data/large.iso --dest /backup/large.iso

# Explicitly enable with checkpointing every 5 seconds
orbit --source /data/large.iso --dest /backup/large.iso --resume
```

### Disabling Resume

```bash
orbit --source /data/file.dat --dest /backup/file.dat --no-resume
```

### Resume with Verbose Progress

```bash
orbit --source /data/large.iso --dest /backup/large.iso --progress --verbose
```

Output example:
```
📁 Transferring: /data/large.iso
🔄 Resume Decision for large.iso: Resume
   Offset: 10.50 MB (42 chunks verified)

   [████████████████████░░░░░░░] 75.0%  10.50 MB/s  ETA: 2s

   ✓ Complete: 102.40 MB in 9753ms (10.49 MB/s)
```

## Implementation Details

### Checkpoint Frequency

Resume info is saved every **5 seconds** during active transfers. This balances:
- **Frequent checkpoints**: Minimize re-transfer on interruption
- **Performance**: Avoid excessive I/O overhead

```rust
// Checkpoint for resume (every 5 seconds)
if config.resume_enabled && last_checkpoint.elapsed() > Duration::from_secs(5) {
    dest_file.flush()?;
    resume_info.bytes_copied = bytes_copied;
    save_resume_info_full(dest_path, &resume_info, false)?;
    last_checkpoint = Instant::now();
}
```

### Chunk Digest Recording

Digests are recorded for **complete chunks only** (exactly `chunk_size` bytes):

```rust
// Record chunk digest if this is a complete chunk
if config.resume_enabled && n == config.chunk_size {
    let chunk_id = (bytes_copied / config.chunk_size as u64) as u32;
    record_chunk_digest(chunk_id, &buffer[..n], &mut resume_info);
}
```

**Note**: Partial chunks at the end of files are not digested to avoid complexity. They are verified by the final file checksum.

### File Metadata Validation

Resume decisions use filesystem metadata to detect external modifications:

- **Modification time** (`mtime`): Detects if file was edited
- **File size**: Detects truncation or unexpected growth
- **Tolerance**: 2-second window for filesystem time precision

```rust
// Check if file was modified since resume info was saved
if let (Some(saved_mtime), Some(curr_mtime)) = (resume_info.file_mtime, current_mtime) {
    // Allow 2 second tolerance for filesystem time precision
    if curr_mtime > saved_mtime + 2 {
        return ResumeDecision::Revalidate {
            reason: format!(
                "File was modified ({} sec newer than resume info)",
                curr_mtime - saved_mtime
            ),
        };
    }
}
```

## Integration with Manifest System

The resume system integrates with Orbit's manifest-based verification:

### Window-Level Verification

Manifests define **windows** (groups of 64 chunks with 4-chunk overlap). The resume system tracks verified windows:

```rust
pub verified_windows: Vec<u32>,  // Window IDs from manifest
```

### Future: Manifest-Driven Resume

Planned for v0.6.0+:
- Load manifest chunks from `.cargo.json` files
- Validate chunks against manifest digests
- Skip already-verified windows during resume

```rust
// Planned feature
let manifest_chunks = load_manifest_chunks(&cargo_path)?;
validate_chunks(&dest_path, &resume_info, chunk_size, &manifest_chunks)?;
```

## Error Handling

### Validation Failures

If chunk validation fails during resume:

```rust
match validate_chunks(dest_path, &resume_info, chunk_size, publisher, &file_id) {
    Ok(failures) => {
        if failures > 0 {
            println!("Warning: {} chunks failed validation, will be re-verified", failures);
            resume_info.verified_chunks.clear();  // Clear and re-verify
        }
    }
    Err(e) => {
        println!("Chunk validation error: {}, clearing verified chunks", e);
        resume_info.verified_chunks.clear();
    }
}
```

### Corrupted Resume Files

If the resume file is corrupted:
1. JSON parsing fails
2. Falls back to legacy format
3. If legacy parsing fails, returns default `ResumeInfo` (starts fresh)

```rust
// Try JSON format first (new format)
if let Ok(info) = serde_json::from_str::<ResumeInfo>(&resume_data) {
    return Ok(info);
}

// Fall back to legacy format
// If legacy fails, returns ResumeInfo::default()
```

## Performance Considerations

### Resume Overhead

- **Checkpoint I/O**: JSON write every 5 seconds (~1KB typically)
- **Chunk hashing**: BLAKE3 is extremely fast (~3 GB/s on modern CPUs)
- **Verification**: Only during `Revalidate` decision (rare)

### Memory Usage

- Resume info kept in memory during transfer
- For large files with many chunks:
  - 100 GB file @ 256KB chunks = 400,000 chunks
  - ~400,000 * 64 bytes (hash) = 25.6 MB HashMap overhead
  - Acceptable for modern systems

### Optimization Tips

1. **Increase chunk size** for very large files to reduce chunk count:
   ```bash
   orbit --source huge.iso --dest backup.iso --chunk-size 1048576  # 1MB chunks
   ```

2. **Disable resume** for small files where overhead exceeds benefit:
   ```bash
   orbit --source small.txt --dest backup.txt --no-resume
   ```

3. **Use compression** to reduce transfer size (resume works with compression):
   ```bash
   orbit --source data.tar --dest backup.tar.zst --compress
   ```

## API Reference

### Core Functions

```rust
// Load resume information from disk
pub fn load_resume_info(destination_path: &Path, compressed: bool) -> Result<ResumeInfo>

// Save full resume information (JSON format)
pub fn save_resume_info_full(
    destination_path: &Path,
    info: &ResumeInfo,
    compressed: bool,
) -> Result<()>

// Decide resume strategy based on file state
pub fn decide_resume_strategy(
    destination_path: &Path,
    resume_info: &ResumeInfo,
) -> ResumeDecision

// Validate chunks against stored digests
pub fn validate_chunks(
    destination_path: &Path,
    resume_info: &ResumeInfo,
    chunk_size: usize,
    publisher: &ProgressPublisher,
    file_id: &FileId,
) -> Result<usize>

// Record chunk digest during copy
pub fn record_chunk_digest(
    chunk_id: u32,
    chunk_data: &[u8],
    resume_info: &mut ResumeInfo,
)
```

### Progress Events

```rust
// Emit resume decision
publisher.publish_resume_decision(
    &file_id,
    "Resume",
    from_offset,
    verified_chunks,
    None,
);

// Emit chunk verification started
publisher.publish_chunk_verification(&file_id, chunk_id, chunk_size);

// Emit chunk verified
publisher.publish_chunk_verified(&file_id, chunk_id, digest);
```

## Testing

Comprehensive test coverage for resume functionality:

```bash
# Run resume system tests
cargo test resume

# Specific test categories
cargo test test_resume_decision      # Resume decision logic
cargo test test_chunk                # Chunk verification
cargo test test_save_load            # Serialization
```

Test scenarios covered:
- ✅ Resume with valid destination file
- ✅ Restart when file is missing
- ✅ Restart on file size mismatch
- ✅ Restart on file truncation
- ✅ Revalidate on modification time change
- ✅ Chunk digest recording
- ✅ Chunk validation success
- ✅ Chunk validation failure
- ✅ JSON serialization round-trip
- ✅ Legacy format backward compatibility

## Troubleshooting

### "Resuming from byte X but file is smaller"

**Cause**: Destination file was truncated externally
**Solution**: Resume system will detect this and restart the transfer

### "Warning: N chunks failed validation"

**Cause**: Destination file content changed since checkpoint
**Solution**: Chunks will be cleared and re-verified during transfer

### "Resume info file corrupted"

**Cause**: Power loss during resume file write, disk error
**Solution**: Transfer will start fresh (resume file will be recreated)

### Resume file growing too large

**Cause**: Many small chunks creating large verified_chunks HashMap
**Solution**: Increase chunk size with `--chunk-size` option

## Future Enhancements

### Planned for v0.6.0+

1. **Full Manifest Integration**
   - Validate against `.cargo.json` manifest chunks
   - Window-level skip for verified ranges
   - Merkle tree verification

2. **Distributed Resume**
   - Share resume state across multiple workers
   - Parallel chunk verification
   - Coordinated resume for large files

3. **Cloud-Backed Resume**
   - Store resume state in S3/cloud storage
   - Resume across different machines
   - Disaster recovery scenarios

4. **Partial Chunk Resume**
   - Digest tracking for incomplete chunks
   - Sub-chunk verification
   - Minimize waste on interruption

## See Also

- [Manifest System](MANIFEST_SYSTEM.md) - Flight Plans, Cargo Manifests, and Star Maps
- [Zero-Copy](ZERO_COPY.md) - High-performance copy modes
- [S3 User Guide](S3_USER_GUIDE.md) - Cloud storage integration
