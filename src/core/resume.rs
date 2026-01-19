/*!
 * Resume functionality for interrupted transfers with chunk-level verification
 */

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Resume information for interrupted transfers with chunk-level tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResumeInfo {
    /// Total bytes copied
    pub bytes_copied: u64,

    /// Compressed bytes (if compression is used)
    pub compressed_bytes: Option<u64>,

    /// Verified chunk digests (chunk_id -> digest_hex)
    #[serde(default)]
    pub verified_chunks: HashMap<u32, String>,

    /// Manifest window IDs that have been verified
    #[serde(default)]
    pub verified_windows: Vec<u32>,

    /// File modification time when resume info was saved (unix timestamp)
    #[serde(default)]
    pub file_mtime: Option<u64>,

    /// File size when resume info was saved
    #[serde(default)]
    pub file_size: Option<u64>,
}

/// Decision on how to handle an interrupted transfer
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeDecision {
    /// Resume from the saved offset (fast-forward)
    Resume {
        from_offset: u64,
        verified_chunks: usize,
    },

    /// Re-validate existing data before resuming
    Revalidate { reason: String },

    /// Restart the transfer from beginning
    Restart { reason: String },

    /// Start fresh (no previous resume data)
    StartFresh,
}

/// Load resume information from disk
pub fn load_resume_info(destination_path: &Path, compressed: bool) -> Result<ResumeInfo> {
    let resume_file_path = get_resume_file_path(destination_path, compressed);

    if !resume_file_path.exists() {
        return Ok(ResumeInfo::default());
    }

    let resume_data = std::fs::read_to_string(&resume_file_path)?;

    // Try JSON format first (new format)
    if let Ok(info) = serde_json::from_str::<ResumeInfo>(&resume_data) {
        println!(
            "Loaded resume info: {} bytes, {} chunks verified",
            info.bytes_copied,
            info.verified_chunks.len()
        );
        return Ok(info);
    }

    // Fall back to legacy format for backward compatibility
    let lines: Vec<&str> = resume_data.lines().collect();
    if lines.is_empty() {
        return Ok(ResumeInfo::default());
    }

    let bytes_copied: u64 = lines[0].parse().unwrap_or(0);
    let compressed_bytes = if lines.len() > 1 {
        lines[1].parse().ok()
    } else {
        None
    };

    println!("Loaded legacy resume info: {} bytes copied", bytes_copied);

    Ok(ResumeInfo {
        bytes_copied,
        compressed_bytes,
        ..Default::default()
    })
}

/// Save current progress to resume file (legacy signature for backward compatibility)
pub fn save_resume_info(
    destination_path: &Path,
    bytes_copied: u64,
    compressed_bytes: Option<u64>,
    compressed: bool,
) -> Result<()> {
    let info = ResumeInfo {
        bytes_copied,
        compressed_bytes,
        ..Default::default()
    };
    save_resume_info_full(destination_path, &info, compressed)
}

/// Save full resume information with chunk tracking
pub fn save_resume_info_full(
    destination_path: &Path,
    info: &ResumeInfo,
    compressed: bool,
) -> Result<()> {
    let resume_file_path = get_resume_file_path(destination_path, compressed);

    let temp_extension = resume_file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!("{ext}.tmp"))
        .unwrap_or_else(|| "tmp".to_string());
    let temp_path = resume_file_path.with_extension(temp_extension);

    // Save as JSON for rich metadata
    let json = serde_json::to_string_pretty(info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    {
        let mut file = File::create(&temp_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
    }

    if let Ok(ms) = std::env::var("ORBIT_RESUME_SLEEP_BEFORE_RENAME_MS") {
        if let Ok(ms) = ms.parse::<u64>() {
            std::thread::sleep(Duration::from_millis(ms));
        }
    }

    std::fs::rename(&temp_path, &resume_file_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        std::io::Error::other(format!("Failed to atomically rename resume file: {}", e))
    })?;
    Ok(())
}

/// Clean up resume information after successful completion
pub fn cleanup_resume_info(destination_path: &Path, compressed: bool) {
    let resume_file_path = get_resume_file_path(destination_path, compressed);
    if resume_file_path.exists() {
        let _ = std::fs::remove_file(&resume_file_path);
    }
}

/// Get the path for resume information file
fn get_resume_file_path(destination_path: &Path, compressed: bool) -> PathBuf {
    if compressed {
        destination_path.with_extension("orbit_resume_compressed")
    } else {
        destination_path.with_extension("orbit_resume")
    }
}

/// Decide whether to resume, revalidate, or restart based on destination state
///
/// This function compares the resume information with the actual destination file
/// to make an intelligent decision about how to proceed.
pub fn decide_resume_strategy(destination_path: &Path, resume_info: &ResumeInfo) -> ResumeDecision {
    // If no resume data, start fresh
    if resume_info.bytes_copied == 0 {
        return ResumeDecision::StartFresh;
    }

    // Check if destination file exists
    let dest_metadata = match std::fs::metadata(destination_path) {
        Ok(meta) => meta,
        Err(_) => {
            return ResumeDecision::Restart {
                reason: "Destination file no longer exists".to_string(),
            };
        }
    };

    let current_size = dest_metadata.len();

    // Get current file modification time
    let current_mtime = dest_metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    // Check if file size changed unexpectedly
    if let Some(saved_size) = resume_info.file_size {
        if current_size != saved_size && current_size != resume_info.bytes_copied {
            return ResumeDecision::Restart {
                reason: format!(
                    "File size mismatch: expected {} or {}, found {}",
                    saved_size, resume_info.bytes_copied, current_size
                ),
            };
        }
    }

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

    // Check if current file size matches resume progress
    if current_size < resume_info.bytes_copied {
        return ResumeDecision::Restart {
            reason: format!(
                "File truncated: resume says {}, file has {}",
                resume_info.bytes_copied, current_size
            ),
        };
    }

    if current_size > resume_info.bytes_copied {
        return ResumeDecision::Revalidate {
            reason: format!(
                "File grown: resume says {}, file has {}",
                resume_info.bytes_copied, current_size
            ),
        };
    }

    // All checks passed - safe to resume
    ResumeDecision::Resume {
        from_offset: resume_info.bytes_copied,
        verified_chunks: resume_info.verified_chunks.len(),
    }
}

/// Validate destination file chunks against stored digests
///
/// Reads chunks from the destination file, calculates their digests,
/// and compares with the stored verified_chunks in resume_info.
/// Emits ChunkVerification and ChunkVerified events through the publisher.
///
/// Returns the number of chunks that failed verification.
pub fn validate_chunks(
    destination_path: &Path,
    resume_info: &ResumeInfo,
    chunk_size: usize,
    publisher: &super::progress::ProgressPublisher,
    file_id: &super::progress::FileId,
) -> Result<usize> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    if resume_info.verified_chunks.is_empty() {
        return Ok(0);
    }

    let mut file = File::open(destination_path)?;
    let mut failures = 0;
    let mut buffer = vec![0u8; chunk_size];

    for (chunk_id, expected_digest) in &resume_info.verified_chunks {
        // Calculate chunk offset and size
        let chunk_offset = (*chunk_id as u64) * (chunk_size as u64);
        let remaining = resume_info.bytes_copied.saturating_sub(chunk_offset);
        let this_chunk_size = remaining.min(chunk_size as u64);

        if this_chunk_size == 0 {
            continue;
        }

        // Emit chunk verification start event
        publisher.publish_chunk_verification(file_id, *chunk_id, this_chunk_size);

        // Read chunk from file
        file.seek(SeekFrom::Start(chunk_offset))?;
        let bytes_read = file.read(&mut buffer[..this_chunk_size as usize])?;

        if bytes_read != this_chunk_size as usize {
            failures += 1;
            continue;
        }

        // Calculate BLAKE3 hash
        let hash = blake3::hash(&buffer[..bytes_read]);
        let digest_hex = hex::encode(hash.as_bytes());

        // Compare with expected digest
        if &digest_hex == expected_digest {
            // Emit chunk verified event
            publisher.publish_chunk_verified(file_id, *chunk_id, digest_hex);
        } else {
            failures += 1;
        }
    }

    Ok(failures)
}

/// Calculate and store chunk digest for a newly copied chunk
///
/// This is called during the copy process to track verified chunks.
pub fn record_chunk_digest(chunk_id: u32, chunk_data: &[u8], resume_info: &mut ResumeInfo) {
    let hash = blake3::hash(chunk_data);
    let digest_hex = hex::encode(hash.as_bytes());
    resume_info.verified_chunks.insert(chunk_id, digest_hex);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load_resume_info() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("test.txt");

        save_resume_info(&dest, 1024, None, false).unwrap();
        let info = load_resume_info(&dest, false).unwrap();

        assert_eq!(info.bytes_copied, 1024);
        assert_eq!(info.compressed_bytes, None);

        cleanup_resume_info(&dest, false);
        let info2 = load_resume_info(&dest, false).unwrap();
        assert_eq!(info2.bytes_copied, 0);
    }

    #[test]
    fn test_compressed_resume_info() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("test.txt");

        save_resume_info(&dest, 2048, Some(1024), true).unwrap();
        let info = load_resume_info(&dest, true).unwrap();

        assert_eq!(info.bytes_copied, 2048);
        assert_eq!(info.compressed_bytes, Some(1024));
    }

    #[test]
    fn test_resume_decision_start_fresh() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("test.txt");

        let resume_info = ResumeInfo::default();
        let decision = decide_resume_strategy(&dest, &resume_info);

        assert!(matches!(decision, ResumeDecision::StartFresh));
    }

    #[test]
    fn test_resume_decision_restart_file_missing() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("test.txt");

        let resume_info = ResumeInfo {
            bytes_copied: 1024,
            file_size: Some(2048),
            ..Default::default()
        };

        let decision = decide_resume_strategy(&dest, &resume_info);

        match decision {
            ResumeDecision::Restart { reason } => {
                assert!(reason.contains("no longer exists"));
            }
            _ => panic!("Expected Restart decision"),
        }
    }

    #[test]
    fn test_resume_decision_resume_valid() {
        use std::io::Write;

        let dir = tempdir().unwrap();
        let dest = dir.path().join("test.txt");

        // Create a partial file
        let mut file = std::fs::File::create(&dest).unwrap();
        file.write_all(&[0u8; 1024]).unwrap();
        drop(file);

        let metadata = std::fs::metadata(&dest).unwrap();
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        let resume_info = ResumeInfo {
            bytes_copied: 1024,
            file_size: Some(1024),
            file_mtime: mtime,
            ..Default::default()
        };

        let decision = decide_resume_strategy(&dest, &resume_info);

        match decision {
            ResumeDecision::Resume {
                from_offset,
                verified_chunks,
            } => {
                assert_eq!(from_offset, 1024);
                assert_eq!(verified_chunks, 0);
            }
            _ => panic!("Expected Resume decision, got {:?}", decision),
        }
    }

    #[test]
    fn test_resume_decision_restart_file_truncated() {
        use std::io::Write;

        let dir = tempdir().unwrap();
        let dest = dir.path().join("test.txt");

        // Create a file smaller than expected
        let mut file = std::fs::File::create(&dest).unwrap();
        file.write_all(&[0u8; 512]).unwrap();
        drop(file);

        let resume_info = ResumeInfo {
            bytes_copied: 1024,
            file_size: Some(1024),
            ..Default::default()
        };

        let decision = decide_resume_strategy(&dest, &resume_info);

        match decision {
            ResumeDecision::Restart { reason } => {
                // File size mismatch check happens before truncation check
                assert!(reason.contains("mismatch") || reason.contains("truncated"));
            }
            _ => panic!("Expected Restart decision"),
        }
    }

    #[test]
    fn test_chunk_digest_recording() {
        let mut resume_info = ResumeInfo::default();
        let chunk_data = b"Hello, World!";

        record_chunk_digest(0, chunk_data, &mut resume_info);

        assert_eq!(resume_info.verified_chunks.len(), 1);
        assert!(resume_info.verified_chunks.contains_key(&0));

        // Verify the digest is a valid hex string (64 chars for BLAKE3)
        let digest = resume_info.verified_chunks.get(&0).unwrap();
        assert_eq!(digest.len(), 64); // BLAKE3 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_chunk_validation_success() {
        use std::io::Write;

        let dir = tempdir().unwrap();
        let dest = dir.path().join("test.txt");

        // Create a file with known content
        let chunk_data = vec![0x42u8; 1024];
        let mut file = std::fs::File::create(&dest).unwrap();
        file.write_all(&chunk_data).unwrap();
        drop(file);

        // Record the chunk digest
        let mut resume_info = ResumeInfo {
            bytes_copied: 1024,
            ..Default::default()
        };
        record_chunk_digest(0, &chunk_data, &mut resume_info);

        // Validate chunks
        let (publisher, _subscriber) = super::super::progress::ProgressPublisher::unbounded();
        let file_id =
            super::super::progress::FileId::new(&std::path::PathBuf::from("source"), &dest);

        let failures = validate_chunks(&dest, &resume_info, 1024, &publisher, &file_id).unwrap();
        assert_eq!(failures, 0);
    }

    #[test]
    fn test_chunk_validation_failure() {
        use std::io::Write;

        let dir = tempdir().unwrap();
        let dest = dir.path().join("test.txt");

        // Create a file with different content than expected
        let chunk_data = vec![0x42u8; 1024];
        let different_data = vec![0x99u8; 1024];

        let mut file = std::fs::File::create(&dest).unwrap();
        file.write_all(&different_data).unwrap();
        drop(file);

        // Record digest for original data
        let mut resume_info = ResumeInfo {
            bytes_copied: 1024,
            ..Default::default()
        };
        record_chunk_digest(0, &chunk_data, &mut resume_info);

        // Validate chunks (should fail)
        let (publisher, _subscriber) = super::super::progress::ProgressPublisher::unbounded();
        let file_id =
            super::super::progress::FileId::new(&std::path::PathBuf::from("source"), &dest);

        let failures = validate_chunks(&dest, &resume_info, 1024, &publisher, &file_id).unwrap();
        assert_eq!(failures, 1);
    }

    #[test]
    fn test_save_load_resume_info_full() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("test.txt");

        let mut resume_info = ResumeInfo {
            bytes_copied: 2048,
            file_size: Some(4096),
            file_mtime: Some(1234567890),
            ..Default::default()
        };
        resume_info.verified_chunks.insert(0, "abc123".to_string());
        resume_info.verified_chunks.insert(1, "def456".to_string());
        resume_info.verified_windows.push(0);

        save_resume_info_full(&dest, &resume_info, false).unwrap();
        let loaded = load_resume_info(&dest, false).unwrap();

        assert_eq!(loaded.bytes_copied, 2048);
        assert_eq!(loaded.file_size, Some(4096));
        assert_eq!(loaded.file_mtime, Some(1234567890));
        assert_eq!(loaded.verified_chunks.len(), 2);
        assert_eq!(loaded.verified_windows.len(), 1);
    }
}
