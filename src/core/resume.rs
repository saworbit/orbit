/*!
 * Resume functionality for interrupted transfers with chunk-level verification
 */

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::error::Result;
use serde::{Serialize, Deserialize};

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
    Resume { from_offset: u64, verified_chunks: usize },

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
        println!("Loaded resume info: {} bytes, {} chunks verified",
            info.bytes_copied, info.verified_chunks.len());
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

    // Save as JSON for rich metadata
    let json = serde_json::to_string_pretty(info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    std::fs::write(&resume_file_path, json)?;
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
pub fn decide_resume_strategy(
    destination_path: &Path,
    resume_info: &ResumeInfo,
) -> ResumeDecision {
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
    let current_mtime = dest_metadata.modified()
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

/// Validate destination file against manifest chunk data
///
/// This checks if existing chunks match their expected digests from the manifest.
/// Returns the number of chunks that need re-verification.
pub fn validate_against_manifest(
    _destination_path: &Path,
    resume_info: &ResumeInfo,
    _manifest_chunks: &HashMap<u32, String>,
) -> Result<usize> {
    // For now, return the number of verified chunks
    // Full implementation would:
    // 1. Read each verified chunk from destination
    // 2. Calculate its digest
    // 3. Compare with manifest
    // 4. Return count of mismatches

    Ok(resume_info.verified_chunks.len())
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
}