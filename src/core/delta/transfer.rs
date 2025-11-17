/*!
 * Delta transfer implementation - actual file copying with delta algorithm
 */

use super::algorithm::{generate_delta, generate_delta_rolling, SignatureIndex};
use super::checksum::{generate_signatures, generate_signatures_parallel};
use super::types::DeltaInstruction;
use super::{DeltaConfig, DeltaStats, HashAlgorithm};
use crate::error::Result;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Perform delta transfer between source and destination
///
/// This generates signatures for the destination file, finds matching blocks
/// in the source, and reconstructs the destination with minimal data transfer.
///
/// Returns the delta statistics and the checksum of the final file.
pub fn copy_with_delta(
    source_path: &Path,
    dest_path: &Path,
    config: &DeltaConfig,
) -> Result<(DeltaStats, Option<String>)> {
    // Open files
    let source_file = File::open(source_path)?;
    let dest_exists = dest_path.exists();

    if !dest_exists {
        // No destination file, fall back to full copy
        return full_copy_as_delta(source_path, dest_path, config);
    }

    // Generate signatures for existing destination
    let dest_file = File::open(dest_path)?;
    let signatures = if config.parallel_hashing {
        generate_signatures_parallel(dest_file, config.block_size, config.hash_algorithm)?
    } else {
        generate_signatures(dest_file, config.block_size, config.hash_algorithm)?
    };

    if signatures.is_empty() {
        // Empty destination, full copy
        return full_copy_as_delta(source_path, dest_path, config);
    }

    let signature_index = SignatureIndex::new(signatures);

    // Generate delta instructions
    let (instructions, stats) = if config.block_size >= 64 * 1024 {
        // Use rolling checksum optimization for larger blocks
        generate_delta_rolling(source_file, signature_index, config.hash_algorithm)?
    } else {
        // Simple delta for smaller blocks
        let source_file = File::open(source_path)?;
        generate_delta(source_file, signature_index, config.hash_algorithm)?
    };

    // Apply delta to create new file
    let temp_path = dest_path.with_extension("orbit_delta_tmp");
    apply_delta(dest_path, &temp_path, &instructions)?;

    // Replace destination with new file
    std::fs::rename(&temp_path, dest_path)?;

    // Calculate final checksum if needed
    let checksum = calculate_file_hash(dest_path, config.hash_algorithm)?;

    Ok((stats, Some(checksum)))
}

/// Apply delta instructions to reconstruct a file
fn apply_delta(old_path: &Path, new_path: &Path, instructions: &[DeltaInstruction]) -> Result<()> {
    let mut old_file = File::open(old_path)?;
    let mut new_file = File::create(new_path)?;

    for instruction in instructions {
        match instruction {
            DeltaInstruction::Copy {
                src_offset,
                dest_offset: _,
                length,
            } => {
                // Copy block from old file
                let mut buffer = vec![0u8; *length];
                old_file.seek(SeekFrom::Start(*src_offset))?;
                old_file.read_exact(&mut buffer)?;
                new_file.write_all(&buffer)?;
            }
            DeltaInstruction::Data {
                dest_offset: _,
                bytes,
            } => {
                // Write new data
                new_file.write_all(bytes)?;
            }
        }
    }

    new_file.sync_all()?;
    Ok(())
}

/// Perform a full copy but return delta-style statistics
fn full_copy_as_delta(
    source_path: &Path,
    dest_path: &Path,
    config: &DeltaConfig,
) -> Result<(DeltaStats, Option<String>)> {
    let source_size = std::fs::metadata(source_path)?.len();

    // Simple full copy
    std::fs::copy(source_path, dest_path)?;

    // Calculate stats for full copy
    let mut stats = DeltaStats::new();
    stats.total_bytes = source_size;
    stats.bytes_transferred = source_size;
    stats.total_blocks = (source_size + config.block_size as u64 - 1) / config.block_size as u64;
    stats.blocks_transferred = stats.total_blocks;
    stats.blocks_matched = 0;
    stats.bytes_saved = 0;
    stats.calculate_savings_ratio();

    let checksum = calculate_file_hash(dest_path, config.hash_algorithm)?;

    Ok((stats, Some(checksum)))
}

/// Calculate file hash
fn calculate_file_hash(path: &Path, algorithm: HashAlgorithm) -> Result<String> {
    use super::checksum::StrongHasher;

    let mut file = File::open(path)?;
    let mut hasher = StrongHasher::new(algorithm);
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB chunks

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Copy file with delta transfer, with fallback to full copy on errors
///
/// This is the high-level entry point that handles errors gracefully
pub fn copy_with_delta_fallback(
    source_path: &Path,
    dest_path: &Path,
    config: &DeltaConfig,
) -> Result<(DeltaStats, Option<String>)> {
    match copy_with_delta(source_path, dest_path, config) {
        Ok(result) => Ok(result),
        Err(e) => {
            eprintln!("Delta transfer failed: {}, falling back to full copy", e);
            full_copy_as_delta(source_path, dest_path, config)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_full_copy_as_delta() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, b"test data for full copy").unwrap();

        let config = DeltaConfig::default();
        let (stats, checksum) = full_copy_as_delta(&source, &dest, &config).unwrap();

        assert_eq!(stats.total_bytes, 23);
        assert_eq!(stats.bytes_transferred, 23);
        assert_eq!(stats.blocks_matched, 0);
        assert!(checksum.is_some());
        assert_eq!(fs::read(&dest).unwrap(), b"test data for full copy");
    }

    #[test]
    fn test_copy_with_delta_no_dest() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, b"new file content").unwrap();

        let config = DeltaConfig::default();
        let (stats, _) = copy_with_delta(&source, &dest, &config).unwrap();

        // Should do full copy since dest doesn't exist
        assert_eq!(stats.blocks_matched, 0);
        assert_eq!(stats.total_bytes, 16);
        assert_eq!(fs::read(&dest).unwrap(), b"new file content");
    }

    #[test]
    fn test_copy_with_delta_identical_files() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        let data = vec![0u8; 100 * 1024]; // 100KB
        fs::write(&source, &data).unwrap();
        fs::write(&dest, &data).unwrap();

        let config = DeltaConfig {
            block_size: 10 * 1024, // 10KB blocks
            ..Default::default()
        };

        let (stats, _) = copy_with_delta(&source, &dest, &config).unwrap();

        // Should match all blocks
        assert!(stats.blocks_matched > 0);
        assert_eq!(stats.savings_ratio, 1.0); // 100% savings
        assert_eq!(stats.bytes_saved, stats.total_bytes);
    }

    #[test]
    fn test_copy_with_delta_partial_match() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Create similar but different files
        let mut source_data = vec![0u8; 100 * 1024];
        source_data[50 * 1024..60 * 1024].fill(255); // Modify middle section

        let dest_data = vec![0u8; 100 * 1024];

        fs::write(&source, &source_data).unwrap();
        fs::write(&dest, &dest_data).unwrap();

        let config = DeltaConfig {
            block_size: 10 * 1024,
            ..Default::default()
        };

        let (stats, _) = copy_with_delta(&source, &dest, &config).unwrap();

        // Should have some matches
        assert!(stats.blocks_matched > 0);
        assert!(stats.blocks_matched < stats.total_blocks);
        assert!(stats.savings_ratio > 0.0 && stats.savings_ratio < 1.0);

        // Verify file content matches source
        let result_data = fs::read(&dest).unwrap();
        assert_eq!(result_data, source_data);
    }

    #[test]
    fn test_calculate_file_hash() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, b"test data").unwrap();

        let hash1 = calculate_file_hash(&path, HashAlgorithm::Blake3).unwrap();
        let hash2 = calculate_file_hash(&path, HashAlgorithm::Blake3).unwrap();

        // Same file should produce same hash
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // BLAKE3 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_copy_with_delta_fallback() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, b"fallback test").unwrap();

        let config = DeltaConfig::default();
        let (stats, _) = copy_with_delta_fallback(&source, &dest, &config).unwrap();

        // Should succeed even though dest doesn't exist
        assert_eq!(fs::read(&dest).unwrap(), b"fallback test");
        assert_eq!(stats.total_bytes, 13);
    }
}
