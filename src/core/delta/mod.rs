/*!
 * Delta detection and efficient transfers module
 *
 * Implements intelligent change detection and partial transfers to minimize
 * data movement, inspired by rsync's delta algorithm with rclone-style checks.
 */

pub mod algorithm;
pub mod checksum;
pub mod transfer;
pub mod types;

pub use types::{
    BlockSignature, CheckMode, DeltaConfig, DeltaStats, HashAlgorithm, ManifestDb, ManifestEntry,
    PartialManifest,
};
// DeltaInstruction is module-internal
pub use transfer::{copy_with_delta, copy_with_delta_fallback, update_manifest_if_configured};
pub(crate) use types::DeltaInstruction;

use crate::error::Result;
use std::path::Path;

/// Determine if delta transfer should be used for the given file
pub fn should_use_delta(
    source_path: &Path,
    dest_path: &Path,
    config: &DeltaConfig,
) -> Result<bool> {
    // Don't use delta if whole_file is forced
    if config.whole_file {
        return Ok(false);
    }

    // Check if a valid partial manifest exists - if so, prioritize delta for resume
    if config.resume_enabled {
        let manifest_path = PartialManifest::manifest_path_for(dest_path);
        if manifest_path.exists() {
            if let Ok(manifest) = PartialManifest::load(&manifest_path) {
                if manifest.is_valid_for(source_path, dest_path) {
                    // Valid manifest exists, prioritize delta for resume
                    return Ok(true);
                }
            }
        }
    }

    // Don't use delta if destination doesn't exist
    if !dest_path.exists() {
        return Ok(false);
    }

    // Only use delta for Delta check mode
    if config.check_mode != CheckMode::Delta {
        return Ok(false);
    }

    // Get file sizes
    let source_metadata = std::fs::metadata(source_path)?;
    let dest_metadata = std::fs::metadata(dest_path)?;

    let source_size = source_metadata.len();
    let dest_size = dest_metadata.len();

    // Delta is most efficient when files are similar in size
    // If files differ by more than 50%, full copy might be faster
    let size_ratio = if dest_size > 0 {
        (source_size as f64 / dest_size as f64).max(dest_size as f64 / source_size as f64)
    } else {
        f64::INFINITY
    };

    // Skip delta for very small files (< 64KB) - overhead not worth it
    if source_size < 64 * 1024 {
        return Ok(false);
    }

    // Skip delta if sizes differ by more than 2x
    if size_ratio > 2.0 {
        return Ok(false);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_should_use_delta_whole_file_forced() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, b"test data").unwrap();
        fs::write(&dest, b"test").unwrap();

        let config = DeltaConfig {
            whole_file: true,
            check_mode: CheckMode::Delta,
            ..Default::default()
        };

        assert!(!should_use_delta(&source, &dest, &config).unwrap());
    }

    #[test]
    fn test_should_use_delta_dest_not_exists() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, b"test data").unwrap();

        let config = DeltaConfig {
            check_mode: CheckMode::Delta,
            ..Default::default()
        };

        assert!(!should_use_delta(&source, &dest, &config).unwrap());
    }

    #[test]
    fn test_should_use_delta_wrong_check_mode() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, b"test data").unwrap();
        fs::write(&dest, b"test").unwrap();

        let config = DeltaConfig {
            check_mode: CheckMode::ModTime,
            ..Default::default()
        };

        assert!(!should_use_delta(&source, &dest, &config).unwrap());
    }

    #[test]
    fn test_should_use_delta_file_too_small() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, b"small").unwrap();
        fs::write(&dest, b"tiny").unwrap();

        let config = DeltaConfig {
            check_mode: CheckMode::Delta,
            ..Default::default()
        };

        assert!(!should_use_delta(&source, &dest, &config).unwrap());
    }

    #[test]
    fn test_should_use_delta_valid_case() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Create files > 64KB with similar sizes
        let data = vec![0u8; 100 * 1024]; // 100KB
        fs::write(&source, &data).unwrap();
        fs::write(&dest, &data[..90 * 1024]).unwrap();

        let config = DeltaConfig {
            check_mode: CheckMode::Delta,
            ..Default::default()
        };

        assert!(should_use_delta(&source, &dest, &config).unwrap());
    }
}
