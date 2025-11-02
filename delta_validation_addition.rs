// Add these imports at the top after existing imports
use crate::config::{CopyMode, CopyConfig};
use super::delta::{CheckMode, self};

// Add these functions before the #[cfg(test)] section

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
        CheckMode::Size => {
            Ok(source_meta.len() != dest_meta.len())
        }
        CheckMode::Checksum | CheckMode::Delta => {
            if source_meta.len() != dest_meta.len() {
                return Ok(true);
            }
            Ok(true)
        }
    }
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
        parallel_hashing: config.parallel_hashing,
        manifest_path: config.delta_manifest_path.clone(),
    };

    delta::should_use_delta(source_path, dest_path, &delta_config)
}
