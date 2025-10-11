/*!
 * Resume functionality for interrupted transfers
 */

use std::path::{Path, PathBuf};
use crate::error::Result;

/// Resume information for interrupted transfers
#[derive(Debug, Clone, Default)]
pub struct ResumeInfo {
    pub bytes_copied: u64,
    pub compressed_bytes: Option<u64>,
}

/// Load resume information from disk
pub fn load_resume_info(destination_path: &Path, compressed: bool) -> Result<ResumeInfo> {
    let resume_file_path = get_resume_file_path(destination_path, compressed);
    
    if !resume_file_path.exists() {
        return Ok(ResumeInfo::default());
    }
    
    let resume_data = std::fs::read_to_string(&resume_file_path)?;
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
    
    println!("Loaded resume info: {} bytes copied", bytes_copied);
    
    Ok(ResumeInfo {
        bytes_copied,
        compressed_bytes,
    })
}

/// Save current progress to resume file
pub fn save_resume_info(
    destination_path: &Path,
    bytes_copied: u64,
    compressed_bytes: Option<u64>,
    compressed: bool,
) -> Result<()> {
    let resume_file_path = get_resume_file_path(destination_path, compressed);
    
    let mut content = bytes_copied.to_string();
    if let Some(cb) = compressed_bytes {
        content.push('\n');
        content.push_str(&cb.to_string());
    }
    
    std::fs::write(&resume_file_path, content)?;
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