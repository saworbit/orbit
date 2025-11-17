/*!
 * Metadata preservation orchestration
 *
 * Provides high-level operations for preserving and transforming metadata
 * across file transfers, including:
 * - Extraction of metadata from source
 * - Transformation and filtering
 * - Application to destination
 * - Backend-aware operations
 * - Manifest integration
 */

use crate::core::file_metadata::{FileMetadata, PreserveFlags};
use crate::core::transform::{transform_metadata, TransformConfig};
use crate::error::Result;
use std::path::Path;

#[cfg(feature = "backend-abstraction")]
use crate::backend::{Backend, BackendResult};

/// Metadata preservation operation
pub struct MetadataPreserver {
    /// Flags controlling what metadata to preserve
    pub preserve_flags: PreserveFlags,

    /// Transformation configuration
    pub transform_config: Option<TransformConfig>,

    /// Strictness mode (fail on any error vs. warn and continue)
    pub strict: bool,
}

impl Default for MetadataPreserver {
    fn default() -> Self {
        Self {
            preserve_flags: PreserveFlags::default(),
            transform_config: None,
            strict: false,
        }
    }
}

impl MetadataPreserver {
    /// Create a new metadata preserver with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set preservation flags
    pub fn with_flags(mut self, flags: PreserveFlags) -> Self {
        self.preserve_flags = flags;
        self
    }

    /// Set transformation configuration
    pub fn with_transforms(mut self, config: TransformConfig) -> Self {
        self.transform_config = Some(config);
        self
    }

    /// Set strictness mode
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Preserve metadata from source to destination (local filesystem)
    pub fn preserve_local(&self, source: &Path, dest: &Path) -> Result<()> {
        // Extract metadata from source
        let mut metadata = FileMetadata::from_path(source)?;

        // Apply transformations if configured
        if let Some(transform_config) = &self.transform_config {
            transform_metadata(&mut metadata, transform_config);
        }

        // Apply metadata to destination
        match metadata.apply_to(dest, self.preserve_flags) {
            Ok(_) => Ok(()),
            Err(e) if !self.strict => {
                tracing::warn!(
                    "Non-fatal metadata preservation error for {:?}: {}",
                    dest,
                    e
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Preserve metadata using backend abstraction
    #[cfg(feature = "backend-abstraction")]
    pub async fn preserve_backend(
        &self,
        source_backend: &dyn Backend,
        source_path: &Path,
        dest_backend: &dyn Backend,
        dest_path: &Path,
    ) -> BackendResult<()> {
        // Get metadata from source backend
        let source_meta = source_backend.stat(source_path).await?;

        // Convert to our comprehensive metadata format
        let mut metadata = FileMetadata {
            size: source_meta.size,
            permissions: source_meta.permissions,
            modified: source_meta.modified,
            accessed: source_meta.accessed,
            created: source_meta.created,
            is_file: source_meta.is_file,
            is_dir: source_meta.is_dir,
            is_symlink: source_meta.is_symlink,
            ..Default::default()
        };

        // Get extended attributes if supported
        if self.preserve_flags.xattrs && dest_backend.supports("get_xattrs") {
            if let Ok(xattrs) = source_backend.get_xattrs(source_path).await {
                metadata.xattrs = Some(xattrs);
            }
        }

        // Apply transformations if configured
        if let Some(transform_config) = &self.transform_config {
            transform_metadata(&mut metadata, transform_config);
        }

        // Apply metadata to destination backend
        self.apply_to_backend(&metadata, dest_backend, dest_path)
            .await
    }

    /// Apply metadata to a backend
    #[cfg(feature = "backend-abstraction")]
    async fn apply_to_backend(
        &self,
        metadata: &FileMetadata,
        backend: &dyn Backend,
        path: &Path,
    ) -> BackendResult<()> {
        use crate::backend::BackendError;

        // Set permissions if requested and supported
        if self.preserve_flags.permissions {
            if let Some(mode) = metadata.permissions {
                if backend.supports("set_permissions") {
                    match backend.set_permissions(path, mode).await {
                        Ok(_) => {}
                        Err(e) if !self.strict => {
                            tracing::warn!("Failed to set permissions on {:?}: {}", path, e);
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }

        // Set timestamps if requested and supported
        if self.preserve_flags.times {
            if backend.supports("set_timestamps") {
                match backend
                    .set_timestamps(path, metadata.accessed, metadata.modified)
                    .await
                {
                    Ok(_) => {}
                    Err(e) if !self.strict => {
                        tracing::warn!("Failed to set timestamps on {:?}: {}", path, e);
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // Set ownership if requested and supported
        if self.preserve_flags.ownership {
            if backend.supports("set_ownership") {
                match backend
                    .set_ownership(path, metadata.owner_uid, metadata.owner_gid)
                    .await
                {
                    Ok(_) => {}
                    Err(BackendError::PermissionDenied { .. }) if !self.strict => {
                        tracing::warn!(
                            "Permission denied setting ownership on {:?} (requires privileges)",
                            path
                        );
                    }
                    Err(e) if !self.strict => {
                        tracing::warn!("Failed to set ownership on {:?}: {}", path, e);
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // Set extended attributes if requested and supported
        if self.preserve_flags.xattrs {
            if let Some(xattrs) = &metadata.xattrs {
                if backend.supports("set_xattrs") {
                    match backend.set_xattrs(path, xattrs).await {
                        Ok(_) => {}
                        Err(e) if !self.strict => {
                            tracing::warn!("Failed to set xattrs on {:?}: {}", path, e);
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract metadata from source, apply transformations, and return result
    pub fn extract_and_transform(&self, source: &Path) -> Result<FileMetadata> {
        let mut metadata = FileMetadata::from_path(source)?;

        if let Some(transform_config) = &self.transform_config {
            transform_metadata(&mut metadata, transform_config);
        }

        Ok(metadata)
    }
}

/// Verify that metadata was correctly preserved
pub fn verify_metadata(source: &Path, dest: &Path, flags: PreserveFlags) -> Result<bool> {
    let source_meta = FileMetadata::from_path(source)?;
    let dest_meta = FileMetadata::from_path(dest)?;

    let mut matches = true;

    // Verify timestamps
    if flags.times {
        if source_meta.modified != dest_meta.modified {
            tracing::warn!("Modification time mismatch for {:?}", dest);
            matches = false;
        }
    }

    // Verify permissions
    if flags.permissions {
        if source_meta.permissions != dest_meta.permissions {
            tracing::warn!("Permissions mismatch for {:?}", dest);
            matches = false;
        }
    }

    // Verify ownership
    if flags.ownership {
        if source_meta.owner_uid != dest_meta.owner_uid
            || source_meta.owner_gid != dest_meta.owner_gid
        {
            tracing::warn!("Ownership mismatch for {:?}", dest);
            matches = false;
        }
    }

    // Verify xattrs
    if flags.xattrs {
        if source_meta.xattrs != dest_meta.xattrs {
            tracing::warn!("Extended attributes mismatch for {:?}", dest);
            matches = false;
        }
    }

    Ok(matches)
}

/// Metadata preservation statistics
#[derive(Debug, Clone, Default)]
pub struct MetadataStats {
    pub files_processed: u64,
    pub permissions_preserved: u64,
    pub timestamps_preserved: u64,
    pub ownership_preserved: u64,
    pub xattrs_preserved: u64,
    pub errors: u64,
    pub warnings: u64,
}

impl MetadataStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record successful permission preservation
    pub fn record_permissions(&mut self) {
        self.permissions_preserved += 1;
    }

    /// Record successful timestamp preservation
    pub fn record_timestamps(&mut self) {
        self.timestamps_preserved += 1;
    }

    /// Record successful ownership preservation
    pub fn record_ownership(&mut self) {
        self.ownership_preserved += 1;
    }

    /// Record successful xattr preservation
    pub fn record_xattrs(&mut self) {
        self.xattrs_preserved += 1;
    }

    /// Record an error
    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    /// Record a warning
    pub fn record_warning(&mut self) {
        self.warnings += 1;
    }

    /// Increment file count
    pub fn increment_files(&mut self) {
        self.files_processed += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_metadata_preserver_default() {
        let preserver = MetadataPreserver::new();
        assert!(preserver.preserve_flags.times);
        assert!(preserver.preserve_flags.permissions);
        assert!(!preserver.preserve_flags.ownership);
        assert!(!preserver.preserve_flags.xattrs);
        assert!(!preserver.strict);
    }

    #[test]
    fn test_metadata_preserver_with_flags() {
        let preserver = MetadataPreserver::new()
            .with_flags(PreserveFlags::all())
            .strict(true);

        assert!(preserver.preserve_flags.times);
        assert!(preserver.preserve_flags.permissions);
        assert!(preserver.preserve_flags.ownership);
        assert!(preserver.preserve_flags.xattrs);
        assert!(preserver.strict);
    }

    #[test]
    fn test_preserve_local_basic() {
        let mut source = NamedTempFile::new().unwrap();
        source.write_all(b"test content").unwrap();
        source.flush().unwrap();

        let dest = NamedTempFile::new().unwrap();

        let preserver = MetadataPreserver::new();
        preserver
            .preserve_local(source.path(), dest.path())
            .unwrap();

        // Verify metadata was preserved
        let source_meta = std::fs::metadata(source.path()).unwrap();
        let dest_meta = std::fs::metadata(dest.path()).unwrap();

        assert_eq!(
            source_meta.modified().unwrap(),
            dest_meta.modified().unwrap()
        );
    }

    #[test]
    fn test_extract_and_transform() {
        let mut source = NamedTempFile::new().unwrap();
        source.write_all(b"test").unwrap();
        source.flush().unwrap();

        let preserver = MetadataPreserver::new();
        let metadata = preserver.extract_and_transform(source.path()).unwrap();

        assert!(metadata.is_file);
        assert!(!metadata.is_dir);
        assert_eq!(metadata.size, 4);
    }

    #[test]
    fn test_metadata_stats() {
        let mut stats = MetadataStats::new();
        stats.increment_files();
        stats.record_permissions();
        stats.record_timestamps();

        assert_eq!(stats.files_processed, 1);
        assert_eq!(stats.permissions_preserved, 1);
        assert_eq!(stats.timestamps_preserved, 1);
        assert_eq!(stats.errors, 0);
    }
}
