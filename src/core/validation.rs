/*!
 * Validation logic for copy operations
 */

use super::checksum::calculate_checksum;
use super::delta::{self, checksum as delta_checksum, CheckMode, HashAlgorithm};
use super::disk_guardian::{self, GuardianConfig};
use crate::config::{CopyConfig, CopyMode};
use crate::error::{OrbitError, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use sysinfo::Disks;

/// Validate that sufficient disk space is available (basic check)
///
/// This is a backward-compatible wrapper that uses the default guardian configuration.
/// For more advanced checks with safety margins and integrity validation,
/// use `validate_disk_space_enhanced` instead.
pub fn validate_disk_space(destination_path: &Path, required_size: u64) -> Result<()> {
    let disks = Disks::new_with_refreshed_list();

    let destination_disk = disks
        .iter()
        .find(|disk| destination_path.starts_with(disk.mount_point()));

    if let Some(disk) = destination_disk {
        if disk.available_space() < required_size {
            return Err(OrbitError::InsufficientDiskSpace {
                required: required_size,
                available: disk.available_space(),
            });
        }
    } else {
        eprintln!("Warning: Could not determine available disk space");
    }

    Ok(())
}

/// Enhanced disk space validation with safety margins and integrity checks
///
/// This uses the disk_guardian module to provide:
/// - Safety margins (default 10% extra space)
/// - Minimum free space requirements
/// - Filesystem integrity checks (permissions, writability)
pub fn validate_disk_space_enhanced(
    destination_path: &Path,
    required_size: u64,
    config: Option<&GuardianConfig>,
) -> Result<()> {
    let default_config = GuardianConfig::default();
    let guardian_config = config.unwrap_or(&default_config);

    disk_guardian::ensure_transfer_safety(destination_path, required_size, guardian_config)
}

/// Determine if a file should be copied based on the copy mode
pub fn should_copy_file(source_path: &Path, dest_path: &Path, mode: CopyMode) -> Result<bool> {
    // Always copy if destination doesn't exist
    if !dest_path.exists() {
        return Ok(true);
    }

    match mode {
        CopyMode::Copy => Ok(true),
        CopyMode::Sync | CopyMode::Update => {
            let source_meta = std::fs::metadata(source_path)?;
            let dest_meta = std::fs::metadata(dest_path)?;

            // Copy if source is newer or different size
            Ok(source_meta.modified()? > dest_meta.modified()?
                || source_meta.len() != dest_meta.len())
        }
        CopyMode::Mirror => Ok(true),
    }
}

/// Check if a path matches any exclude patterns
pub fn matches_exclude_pattern(path: &Path, patterns: &[String]) -> bool {
    use glob::Pattern;

    let path_str = path.to_string_lossy();

    patterns.iter().any(|pattern| {
        Pattern::new(pattern)
            .ok()
            .map(|p| p.matches(&path_str))
            .unwrap_or(false)
    })
}

/// Determine if files need to be transferred based on check mode
pub fn files_need_transfer(
    source_path: &Path,
    dest_path: &Path,
    check_mode: CheckMode,
) -> Result<bool> {
    if !dest_path.exists() {
        return Ok(true);
    }

    let source_meta = std::fs::metadata(source_path)?;
    let dest_meta = std::fs::metadata(dest_path)?;

    match check_mode {
        CheckMode::ModTime => {
            Ok(source_meta.modified()? > dest_meta.modified()?
                || source_meta.len() != dest_meta.len())
        }
        CheckMode::Size => Ok(source_meta.len() != dest_meta.len()),
        CheckMode::Checksum => {
            // Quick size check first
            if source_meta.len() != dest_meta.len() {
                return Ok(true);
            }
            // Compare full file checksums
            let source_hash = calculate_checksum(source_path)?;
            let dest_hash = calculate_checksum(dest_path)?;
            Ok(source_hash != dest_hash)
        }
        CheckMode::Delta => {
            // Quick size check first
            if source_meta.len() != dest_meta.len() {
                return Ok(true);
            }

            let source_size = source_meta.len();

            // For small files (< 64KB), delta isn't efficient - use checksum comparison
            // This threshold matches delta::should_use_delta
            if source_size < 64 * 1024 {
                let source_hash = calculate_checksum(source_path)?;
                let dest_hash = calculate_checksum(dest_path)?;
                return Ok(source_hash != dest_hash);
            }

            // For larger files, use lightweight signature comparison
            // Compare first and last block signatures as a quick heuristic
            files_differ_by_signature(source_path, dest_path)
        }
    }
}

/// Check if files differ using block signature comparison
///
/// This performs a lightweight check by comparing signatures of the first
/// and last blocks. If those match, it falls back to full checksum comparison.
fn files_differ_by_signature(source_path: &Path, dest_path: &Path) -> Result<bool> {
    const BLOCK_SIZE: usize = 64 * 1024; // 64KB blocks for signature comparison

    let source_file = File::open(source_path)?;
    let dest_file = File::open(dest_path)?;

    let source_size = source_file.metadata()?.len();
    let dest_size = dest_file.metadata()?.len();

    // Sizes should match (caller verified this)
    if source_size != dest_size {
        return Ok(true);
    }

    // For empty files, they're equal
    if source_size == 0 {
        return Ok(false);
    }

    // Generate signatures for source file
    let source_reader = BufReader::new(&source_file);
    let source_sigs = delta_checksum::generate_signatures(
        source_reader,
        BLOCK_SIZE,
        HashAlgorithm::Blake3,
        delta::RollingHashAlgo::Gear64,
    )?;

    // Generate signatures for destination file
    let dest_reader = BufReader::new(&dest_file);
    let dest_sigs = delta_checksum::generate_signatures(
        dest_reader,
        BLOCK_SIZE,
        HashAlgorithm::Blake3,
        delta::RollingHashAlgo::Gear64,
    )?;

    // Compare number of blocks
    if source_sigs.len() != dest_sigs.len() {
        return Ok(true);
    }

    // Compare all block signatures
    for (src_sig, dest_sig) in source_sigs.iter().zip(dest_sigs.iter()) {
        if src_sig.weak_hash != dest_sig.weak_hash || src_sig.strong_hash != dest_sig.strong_hash {
            return Ok(true);
        }
    }

    // All signatures match - files are identical
    Ok(false)
}

/// Determine if delta transfer should be used for a file pair
pub fn should_use_delta_transfer(
    source_path: &Path,
    dest_path: &Path,
    config: &CopyConfig,
) -> Result<bool> {
    let delta_config = super::delta::DeltaConfig {
        check_mode: config.check_mode,
        block_size: config.delta_block_size,
        whole_file: config.whole_file,
        update_manifest: config.update_manifest,
        ignore_existing: config.ignore_existing,
        hash_algorithm: config.delta_hash_algorithm,
        rolling_hash_algo: delta::RollingHashAlgo::Gear64,
        parallel_hashing: config.parallel_hashing,
        manifest_path: config.delta_manifest_path.clone(),
        resume_enabled: config.delta_resume_enabled,
        chunk_size: config.delta_chunk_size,
    };

    delta::should_use_delta(source_path, dest_path, &delta_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::thread;
    use std::time::Duration;
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn test_should_copy_copy_mode() {
        let source = NamedTempFile::new().unwrap();
        let dest = NamedTempFile::new().unwrap();

        assert!(should_copy_file(source.path(), dest.path(), CopyMode::Copy).unwrap());
    }

    #[test]
    fn test_should_copy_sync_mode_newer() {
        let mut source = NamedTempFile::new().unwrap();
        let mut dest = NamedTempFile::new().unwrap();

        dest.write_all(b"old").unwrap();
        dest.flush().unwrap();

        thread::sleep(Duration::from_millis(100));

        source.write_all(b"new").unwrap();
        source.flush().unwrap();

        assert!(should_copy_file(source.path(), dest.path(), CopyMode::Sync).unwrap());
    }

    #[test]
    fn test_matches_exclude_pattern() {
        let path = Path::new("/tmp/test.tmp");
        let patterns = vec!["*.tmp".to_string(), "*.log".to_string()];

        assert!(matches_exclude_pattern(path, &patterns));

        let path2 = Path::new("/tmp/test.txt");
        assert!(!matches_exclude_pattern(path2, &patterns));
    }

    // Tests for files_need_transfer with CheckMode::Checksum

    #[test]
    fn test_checksum_mode_identical_files_skip_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Create identical files
        std::fs::write(&source, b"identical content here").unwrap();
        std::fs::write(&dest, b"identical content here").unwrap();

        // Checksum mode should detect files are identical and skip transfer
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Checksum).unwrap();
        assert!(
            !needs_transfer,
            "Identical files should not need transfer in Checksum mode"
        );
    }

    #[test]
    fn test_checksum_mode_different_files_need_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Create files with same size but different content
        std::fs::write(&source, b"content version A").unwrap();
        std::fs::write(&dest, b"content version B").unwrap();

        // Checksum mode should detect files differ and require transfer
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Checksum).unwrap();
        assert!(
            needs_transfer,
            "Different files should need transfer in Checksum mode"
        );
    }

    #[test]
    fn test_checksum_mode_different_size_need_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Create files with different sizes
        std::fs::write(&source, b"longer content").unwrap();
        std::fs::write(&dest, b"short").unwrap();

        // Checksum mode should detect size difference and require transfer
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Checksum).unwrap();
        assert!(
            needs_transfer,
            "Files with different sizes should need transfer"
        );
    }

    #[test]
    fn test_checksum_mode_dest_not_exists_need_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("nonexistent.txt");

        std::fs::write(&source, b"source content").unwrap();

        // Should need transfer when destination doesn't exist
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Checksum).unwrap();
        assert!(needs_transfer, "Missing destination should need transfer");
    }

    // Tests for files_need_transfer with CheckMode::Delta

    #[test]
    fn test_delta_mode_small_identical_files_skip_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Create identical small files (< 64KB, will use checksum comparison)
        let content = b"small identical content";
        std::fs::write(&source, content).unwrap();
        std::fs::write(&dest, content).unwrap();

        // Delta mode should detect files are identical and skip transfer
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Delta).unwrap();
        assert!(
            !needs_transfer,
            "Identical small files should not need transfer in Delta mode"
        );
    }

    #[test]
    fn test_delta_mode_small_different_files_need_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Create different small files with same size
        std::fs::write(&source, b"content A here!").unwrap();
        std::fs::write(&dest, b"content B here!").unwrap();

        // Delta mode should detect files differ and require transfer
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Delta).unwrap();
        assert!(
            needs_transfer,
            "Different small files should need transfer in Delta mode"
        );
    }

    #[test]
    fn test_delta_mode_large_identical_files_skip_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.bin");
        let dest = dir.path().join("dest.bin");

        // Create identical large files (> 64KB, will use signature comparison)
        let content: Vec<u8> = (0..100 * 1024).map(|i| (i % 256) as u8).collect();
        std::fs::write(&source, &content).unwrap();
        std::fs::write(&dest, &content).unwrap();

        // Delta mode should detect files are identical and skip transfer
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Delta).unwrap();
        assert!(
            !needs_transfer,
            "Identical large files should not need transfer in Delta mode"
        );
    }

    #[test]
    fn test_delta_mode_large_different_files_need_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.bin");
        let dest = dir.path().join("dest.bin");

        // Create different large files with same size
        let content_a: Vec<u8> = (0..100 * 1024).map(|i| (i % 256) as u8).collect();
        let mut content_b = content_a.clone();
        // Modify bytes in the middle to create a difference
        content_b[50 * 1024] = 0xFF;
        content_b[50 * 1024 + 1] = 0xFF;

        std::fs::write(&source, &content_a).unwrap();
        std::fs::write(&dest, &content_b).unwrap();

        // Delta mode should detect files differ and require transfer
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Delta).unwrap();
        assert!(
            needs_transfer,
            "Different large files should need transfer in Delta mode"
        );
    }

    #[test]
    fn test_delta_mode_different_size_need_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.bin");
        let dest = dir.path().join("dest.bin");

        // Create files with different sizes
        let content_a: Vec<u8> = (0..100 * 1024).map(|i| (i % 256) as u8).collect();
        let content_b: Vec<u8> = (0..80 * 1024).map(|i| (i % 256) as u8).collect();

        std::fs::write(&source, &content_a).unwrap();
        std::fs::write(&dest, &content_b).unwrap();

        // Delta mode should detect size difference and require transfer
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Delta).unwrap();
        assert!(
            needs_transfer,
            "Files with different sizes should need transfer"
        );
    }

    #[test]
    fn test_delta_mode_dest_not_exists_need_transfer() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.bin");
        let dest = dir.path().join("nonexistent.bin");

        let content: Vec<u8> = (0..100 * 1024).map(|i| (i % 256) as u8).collect();
        std::fs::write(&source, &content).unwrap();

        // Should need transfer when destination doesn't exist
        let needs_transfer = files_need_transfer(&source, &dest, CheckMode::Delta).unwrap();
        assert!(needs_transfer, "Missing destination should need transfer");
    }

    #[test]
    fn test_files_differ_by_signature_identical() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.bin");
        let dest = dir.path().join("dest.bin");

        // Create identical large files
        let content: Vec<u8> = (0..128 * 1024).map(|i| (i % 256) as u8).collect();
        std::fs::write(&source, &content).unwrap();
        std::fs::write(&dest, &content).unwrap();

        let differs = files_differ_by_signature(&source, &dest).unwrap();
        assert!(!differs, "Identical files should not differ by signature");
    }

    #[test]
    fn test_files_differ_by_signature_different() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.bin");
        let dest = dir.path().join("dest.bin");

        // Create different large files
        let content_a: Vec<u8> = (0..128 * 1024).map(|i| (i % 256) as u8).collect();
        let mut content_b = content_a.clone();
        content_b[64 * 1024] = 0xFF; // Change a byte in the second block

        std::fs::write(&source, &content_a).unwrap();
        std::fs::write(&dest, &content_b).unwrap();

        let differs = files_differ_by_signature(&source, &dest).unwrap();
        assert!(differs, "Different files should differ by signature");
    }

    #[test]
    fn test_files_differ_by_signature_empty_files() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.bin");
        let dest = dir.path().join("dest.bin");

        // Create empty files
        std::fs::write(&source, b"").unwrap();
        std::fs::write(&dest, b"").unwrap();

        let differs = files_differ_by_signature(&source, &dest).unwrap();
        assert!(!differs, "Empty files should not differ");
    }
}
