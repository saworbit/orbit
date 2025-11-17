/*!
 * File metadata preservation (timestamps, permissions)
 */

use crate::error::{OrbitError, Result};
use filetime::{set_file_times, FileTime};
use std::path::Path;

/// Preserve file metadata from source to destination
pub fn preserve_metadata(source_path: &Path, dest_path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(source_path).map_err(|e| {
        OrbitError::MetadataFailed(format!("Failed to read source metadata: {}", e))
    })?;

    // Preserve permissions
    std::fs::set_permissions(dest_path, metadata.permissions())
        .map_err(|e| OrbitError::MetadataFailed(format!("Failed to set permissions: {}", e)))?;

    // Preserve timestamps
    let accessed = FileTime::from_last_access_time(&metadata);
    let modified = FileTime::from_last_modification_time(&metadata);

    set_file_times(dest_path, accessed, modified)
        .map_err(|e| OrbitError::MetadataFailed(format!("Failed to set timestamps: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_preserve_metadata() {
        let mut source = NamedTempFile::new().unwrap();
        source.write_all(b"test").unwrap();
        source.flush().unwrap();

        let dest = NamedTempFile::new().unwrap();

        preserve_metadata(source.path(), dest.path()).unwrap();

        let source_meta = fs::metadata(source.path()).unwrap();
        let dest_meta = fs::metadata(dest.path()).unwrap();

        assert_eq!(
            source_meta.modified().unwrap(),
            dest_meta.modified().unwrap()
        );
    }
}
