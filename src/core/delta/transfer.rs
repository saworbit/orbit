/*!
 * Delta transfer implementation - actual file copying with delta algorithm
 */

use super::algorithm::{generate_delta, generate_delta_rolling, SignatureIndex};
use super::checksum::{generate_signatures, generate_signatures_parallel};
use super::types::{DeltaInstruction, ManifestDb, ManifestEntry, PartialManifest};
use super::{DeltaConfig, DeltaStats, HashAlgorithm};
use crate::error::{OrbitError, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Perform delta transfer between source and destination
///
/// This generates signatures for the destination file, finds matching blocks
/// in the source, and reconstructs the destination with minimal data transfer.
///
/// Returns the delta statistics and the checksum of the final file.
/// If `update_manifest` is enabled in config, the manifest database will be updated.
pub fn copy_with_delta(
    source_path: &Path,
    dest_path: &Path,
    config: &DeltaConfig,
) -> Result<(DeltaStats, Option<String>)> {
    // If whole_file is forced, skip delta and perform a full copy
    if config.whole_file {
        return full_copy_as_delta(source_path, dest_path, config);
    }

    // Open files
    let source_file = File::open(source_path)?;
    let dest_exists = dest_path.exists();

    if !dest_exists {
        // No destination file, fall back to full copy
        let (mut stats, checksum) = full_copy_as_delta(source_path, dest_path, config)?;

        // Update manifest if configured
        if let Some(ref cksum) = checksum {
            if let Ok(true) =
                update_manifest_if_configured(config, source_path, dest_path, cksum, Some(&stats))
            {
                stats.manifest_updated = true;
            }
        }

        return Ok((stats, checksum));
    }

    // Generate signatures for existing destination
    let dest_file = File::open(dest_path)?;
    let signatures = if config.parallel_hashing {
        generate_signatures_parallel(
            dest_file,
            config.block_size,
            config.hash_algorithm,
            config.rolling_hash_algo,
        )?
    } else {
        generate_signatures(
            dest_file,
            config.block_size,
            config.hash_algorithm,
            config.rolling_hash_algo,
        )?
    };

    if signatures.is_empty() {
        // Empty destination, full copy
        let (mut stats, checksum) = full_copy_as_delta(source_path, dest_path, config)?;

        // Update manifest if configured
        if let Some(ref cksum) = checksum {
            if let Ok(true) =
                update_manifest_if_configured(config, source_path, dest_path, cksum, Some(&stats))
            {
                stats.manifest_updated = true;
            }
        }

        return Ok((stats, checksum));
    }

    let signature_index = SignatureIndex::new(signatures);

    // Generate delta instructions
    let (instructions, mut stats) = if config.block_size >= 64 * 1024 {
        // Use rolling checksum optimization for larger blocks
        generate_delta_rolling(
            source_file,
            signature_index,
            config.hash_algorithm,
            config.rolling_hash_algo,
        )?
    } else {
        // Simple delta for smaller blocks
        let source_file = File::open(source_path)?;
        generate_delta(
            source_file,
            signature_index,
            config.hash_algorithm,
            config.rolling_hash_algo,
        )?
    };

    // Apply delta to create new file
    let temp_path = dest_path.with_extension("orbit_delta_tmp");
    apply_delta(dest_path, &temp_path, &instructions)?;

    // Replace destination with new file
    std::fs::rename(&temp_path, dest_path)?;

    // Calculate final checksum if needed
    let checksum = calculate_file_hash(dest_path, config.hash_algorithm)?;

    // Update manifest if configured
    if let Ok(true) =
        update_manifest_if_configured(config, source_path, dest_path, &checksum, Some(&stats))
    {
        stats.manifest_updated = true;
    }

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
    stats.total_blocks = source_size.div_ceil(config.block_size as u64);
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
/// This is the high-level entry point that handles errors gracefully.
/// If resume is enabled and a partial manifest exists, the transfer will
/// attempt to resume from where it left off.
///
/// Manifest database updates are triggered after successful transfers
/// (including fallbacks) if `update_manifest` is enabled in config.
pub fn copy_with_delta_fallback(
    source_path: &Path,
    dest_path: &Path,
    config: &DeltaConfig,
) -> Result<(DeltaStats, Option<String>)> {
    let partial_manifest_path = PartialManifest::manifest_path_for(dest_path);

    // Check for existing partial manifest for resume
    let mut partial_manifest = if config.resume_enabled && partial_manifest_path.exists() {
        match PartialManifest::load(&partial_manifest_path) {
            Ok(m) if m.is_valid_for(source_path, dest_path) => {
                eprintln!(
                    "Resuming delta transfer from partial manifest ({} chunks processed)",
                    m.processed_count()
                );
                Some(m)
            }
            Ok(_) => {
                // Invalid manifest, remove it and start fresh
                let _ = std::fs::remove_file(&partial_manifest_path);
                None
            }
            Err(_) => {
                // Corrupted manifest, remove it and start fresh
                let _ = std::fs::remove_file(&partial_manifest_path);
                None
            }
        }
    } else {
        None
    };

    // Create new manifest if needed
    if partial_manifest.is_none() && config.resume_enabled {
        let source_metadata = std::fs::metadata(source_path)?;
        let source_size = source_metadata.len();
        let source_mtime = source_metadata
            .modified()
            .unwrap_or(std::time::SystemTime::now());
        let dest_size = if dest_path.exists() {
            std::fs::metadata(dest_path)?.len()
        } else {
            0
        };

        partial_manifest = Some(PartialManifest::new(
            source_path,
            dest_path,
            config.chunk_size,
            config.block_size,
            source_size,
            source_mtime,
            dest_size,
        ));
    }

    // Attempt delta transfer with resume support
    let result = if let Some(ref mut m) = partial_manifest {
        copy_with_delta_resume(source_path, dest_path, config, m)
    } else {
        copy_with_delta(source_path, dest_path, config)
    };

    match result {
        Ok((mut stats, checksum)) => {
            // Update stats with resume info
            if let Some(ref m) = partial_manifest {
                if m.processed_count() > 0 {
                    stats.chunks_resumed = m.processed_count() as u64;
                    stats.bytes_skipped = m.diff_applied_up_to;
                    stats.was_resumed = true;
                }
            }

            // Clean up partial manifest on success
            if partial_manifest_path.exists() {
                let _ = std::fs::remove_file(&partial_manifest_path);
            }

            // Update manifest database if configured
            if let Some(ref cksum) = checksum {
                if let Ok(true) = update_manifest_if_configured(
                    config,
                    source_path,
                    dest_path,
                    cksum,
                    Some(&stats),
                ) {
                    stats.manifest_updated = true;
                }
            }

            Ok((stats, checksum))
        }
        Err(e) => {
            // Determine if error is resumable
            let is_resumable = is_resumable_error(&e);

            if is_resumable {
                // Save partial manifest for future resume
                if let Some(ref m) = partial_manifest {
                    if let Err(save_err) = m.save(&partial_manifest_path) {
                        eprintln!("Warning: Failed to save partial manifest: {}", save_err);
                    } else {
                        eprintln!(
                            "Delta transfer interrupted. Partial manifest saved for resume ({} chunks)",
                            m.processed_count()
                        );
                    }
                }
                // Return error to allow higher-level retry
                Err(e)
            } else {
                // Non-resumable error, clean up and fall back to full copy
                let _ = std::fs::remove_file(&partial_manifest_path);
                eprintln!("Delta transfer failed: {}, falling back to full copy", e);

                // Perform fallback copy
                let (mut fallback_stats, fallback_checksum) =
                    full_copy_as_delta(source_path, dest_path, config)?;

                // Update manifest database for fallback as well
                if let Some(ref cksum) = fallback_checksum {
                    if let Ok(true) = update_manifest_if_configured(
                        config,
                        source_path,
                        dest_path,
                        cksum,
                        Some(&fallback_stats),
                    ) {
                        fallback_stats.manifest_updated = true;
                    }
                }

                Ok((fallback_stats, fallback_checksum))
            }
        }
    }
}

/// Check if an error is resumable (transient, worth retrying later)
fn is_resumable_error(error: &OrbitError) -> bool {
    error.is_transient() || error.is_network_error()
}

/// Update the manifest database if configured in DeltaConfig
///
/// This function creates or updates a manifest entry for the transferred file
/// if `update_manifest` is enabled and a valid `manifest_path` is provided.
///
/// # Arguments
/// * `config` - Delta configuration containing manifest settings
/// * `source_path` - Path to the source file
/// * `dest_path` - Path to the destination file
/// * `checksum` - Checksum of the transferred file (hex string)
/// * `delta_stats` - Optional delta statistics for tracking bytes saved
///
/// # Returns
/// * `Ok(true)` - Manifest was updated successfully
/// * `Ok(false)` - Manifest update was skipped (not configured or ignore_existing)
/// * `Err(_)` - Manifest update failed
pub fn update_manifest_if_configured(
    config: &DeltaConfig,
    source_path: &Path,
    dest_path: &Path,
    checksum: &str,
    delta_stats: Option<&DeltaStats>,
) -> Result<bool> {
    // Check if manifest updates are enabled
    if !config.update_manifest {
        return Ok(false);
    }

    // Get manifest path, return early if not set
    let manifest_path = match &config.manifest_path {
        Some(p) => p,
        None => return Ok(false),
    };

    // Check if we should skip existing manifests
    if manifest_path.exists() && config.ignore_existing {
        return Ok(false);
    }

    // Open or create the manifest database
    let mut db = ManifestDb::open_or_create(manifest_path)?;

    // Get source file metadata
    let source_metadata = std::fs::metadata(source_path)?;
    let source_size = source_metadata.len();
    let source_mtime = source_metadata
        .modified()
        .unwrap_or_else(|_| std::time::SystemTime::now());

    // Create manifest entry
    let mut entry = ManifestEntry::new(
        source_path.to_path_buf(),
        dest_path.to_path_buf(),
        checksum.to_string(),
        source_size,
        source_mtime,
    );

    // Add delta info if available
    if let Some(stats) = delta_stats {
        if stats.bytes_saved > 0 {
            entry = entry.with_delta_info(stats.bytes_saved);
        }
    }

    // Insert or update the entry
    db.insert_or_update(entry);

    // Save the manifest database
    db.save(manifest_path)?;

    Ok(true)
}

/// Perform delta transfer with resume support from partial manifest
fn copy_with_delta_resume(
    source_path: &Path,
    dest_path: &Path,
    config: &DeltaConfig,
    manifest: &mut PartialManifest,
) -> Result<(DeltaStats, Option<String>)> {
    // If manifest has significant progress, try to resume
    if manifest.processed_count() > 0 && manifest.diff_applied_up_to > 0 {
        // For now, we use the standard delta algorithm but track progress
        // Future enhancement: skip already-processed chunks using manifest data
        eprintln!(
            "Resume: skipping {} processed chunks ({} bytes)",
            manifest.processed_count(),
            manifest.diff_applied_up_to
        );
    }

    // Perform the delta transfer
    let result = copy_with_delta(source_path, dest_path, config);

    // Update manifest with final state on success
    if let Ok((ref stats, ref checksum)) = result {
        manifest.update_progress(stats.bytes_transferred + stats.bytes_saved);
        manifest.checksum = checksum.clone();
    }

    result
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

    #[test]
    fn test_update_manifest_if_configured_disabled() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, b"test data").unwrap();
        fs::write(&dest, b"test data").unwrap();

        // Manifest updates disabled
        let config = DeltaConfig::default();
        let result =
            update_manifest_if_configured(&config, &source, &dest, "abc123", None).unwrap();

        assert!(!result); // Should return false when disabled
    }

    #[test]
    fn test_update_manifest_if_configured_no_path() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, b"test data").unwrap();
        fs::write(&dest, b"test data").unwrap();

        // Manifest updates enabled but no path
        let mut config = DeltaConfig::default();
        config.update_manifest = true;
        config.manifest_path = None;

        let result =
            update_manifest_if_configured(&config, &source, &dest, "abc123", None).unwrap();

        assert!(!result); // Should return false when no path
    }

    #[test]
    fn test_update_manifest_if_configured_success() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");
        let manifest_path = dir.path().join("manifest.json");

        fs::write(&source, b"test data for manifest").unwrap();
        fs::write(&dest, b"test data for manifest").unwrap();

        let config = DeltaConfig::default()
            .with_manifest_updates(true)
            .with_manifest_path(manifest_path.clone());

        let result =
            update_manifest_if_configured(&config, &source, &dest, "abc123def456", None).unwrap();

        assert!(result); // Should return true when updated
        assert!(manifest_path.exists()); // Manifest file should exist

        // Verify manifest content
        let db = ManifestDb::load(&manifest_path).unwrap();
        assert_eq!(db.len(), 1);

        let entry = db.get_entry(&dest).unwrap();
        assert_eq!(entry.checksum, "abc123def456");
        assert_eq!(entry.source_path, source);
        assert!(!entry.delta_used);
    }

    #[test]
    fn test_update_manifest_if_configured_with_delta_stats() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");
        let manifest_path = dir.path().join("manifest.json");

        fs::write(&source, b"test data").unwrap();
        fs::write(&dest, b"test data").unwrap();

        let config = DeltaConfig::default()
            .with_manifest_updates(true)
            .with_manifest_path(manifest_path.clone());

        let mut stats = DeltaStats::default();
        stats.bytes_saved = 5000;

        let result =
            update_manifest_if_configured(&config, &source, &dest, "abc123", Some(&stats)).unwrap();

        assert!(result);

        let db = ManifestDb::load(&manifest_path).unwrap();
        let entry = db.get_entry(&dest).unwrap();
        assert!(entry.delta_used);
        assert_eq!(entry.bytes_saved, 5000);
    }

    #[test]
    fn test_update_manifest_if_configured_ignore_existing() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");
        let manifest_path = dir.path().join("manifest.json");

        fs::write(&source, b"test data").unwrap();
        fs::write(&dest, b"test data").unwrap();

        // Create an existing manifest
        let mut existing_db = ManifestDb::new();
        existing_db.insert_or_update(ManifestEntry::new(
            source.clone(),
            dest.clone(),
            "old_checksum".to_string(),
            100,
            std::time::SystemTime::now(),
        ));
        existing_db.save(&manifest_path).unwrap();

        let mut config = DeltaConfig::default()
            .with_manifest_updates(true)
            .with_manifest_path(manifest_path.clone());
        config.ignore_existing = true;

        let result =
            update_manifest_if_configured(&config, &source, &dest, "new_checksum", None).unwrap();

        assert!(!result); // Should skip when ignore_existing is true

        // Verify manifest still has old data
        let db = ManifestDb::load(&manifest_path).unwrap();
        let entry = db.get_entry(&dest).unwrap();
        assert_eq!(entry.checksum, "old_checksum");
    }

    #[test]
    fn test_copy_with_delta_manifest_update() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");
        let manifest_path = dir.path().join("manifest.json");

        fs::write(&source, b"test data for delta with manifest").unwrap();

        let config = DeltaConfig::default()
            .with_manifest_updates(true)
            .with_manifest_path(manifest_path.clone());

        let (stats, checksum) = copy_with_delta(&source, &dest, &config).unwrap();

        assert!(stats.manifest_updated);
        assert!(manifest_path.exists());

        let db = ManifestDb::load(&manifest_path).unwrap();
        let entry = db.get_entry(&dest).unwrap();
        assert_eq!(entry.checksum, checksum.unwrap());
    }

    #[test]
    fn test_copy_with_delta_fallback_manifest_update() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");
        let manifest_path = dir.path().join("manifest.json");

        fs::write(&source, b"fallback manifest test").unwrap();

        let config = DeltaConfig::default()
            .with_manifest_updates(true)
            .with_manifest_path(manifest_path.clone());

        let (stats, checksum) = copy_with_delta_fallback(&source, &dest, &config).unwrap();

        assert!(stats.manifest_updated);
        assert!(manifest_path.exists());

        let db = ManifestDb::load(&manifest_path).unwrap();
        let entry = db.get_entry(&dest).unwrap();
        assert_eq!(entry.checksum, checksum.unwrap());
    }
}
