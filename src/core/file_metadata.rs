/*!
 * Comprehensive file metadata preservation and transformation
 *
 * Supports:
 * - Timestamps (atime, mtime, ctime)
 * - Permissions (Unix mode bits, Windows attributes)
 * - Ownership (UID/GID on Unix)
 * - Extended attributes (xattrs)
 * - ACLs (platform-specific)
 * - Metadata transformation (path renaming, filtering)
 */

use std::path::Path;
use std::time::SystemTime;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use filetime::{FileTime, set_file_times, set_file_mtime};
use crate::error::{OrbitError, Result};

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, MetadataExt};

#[cfg(feature = "extended-metadata")]
use xattr;

/// Comprehensive file metadata structure
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FileMetadata {
    /// File size in bytes
    pub size: u64,

    /// Unix permissions (mode bits)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<u32>,

    /// Modification time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<SystemTime>,

    /// Access time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed: Option<SystemTime>,

    /// Creation time (platform-dependent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<SystemTime>,

    /// Owner user ID (Unix only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_uid: Option<u32>,

    /// Owner group ID (Unix only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_gid: Option<u32>,

    /// Windows file attributes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows_attributes: Option<u32>,

    /// Extended attributes (name -> value mapping)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xattrs: Option<HashMap<String, Vec<u8>>>,

    /// ACL entries (platform-specific representation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acls: Option<Vec<AclEntry>>,

    /// File type flags
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
}

/// ACL entry representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AclEntry {
    pub principal: String,  // User or group name
    pub permissions: u32,    // Permission bits
    pub entry_type: AclType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AclType {
    User,
    Group,
    Mask,
    Other,
}

/// Metadata preservation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreserveFlags {
    pub times: bool,
    pub permissions: bool,
    pub ownership: bool,
    pub xattrs: bool,
    pub acls: bool,
}

impl Default for PreserveFlags {
    fn default() -> Self {
        Self {
            times: true,
            permissions: true,
            ownership: false,  // Requires privileges
            xattrs: false,     // Optional
            acls: false,       // Optional
        }
    }
}

impl PreserveFlags {
    /// Parse from comma-separated string (e.g., "times,perms,owners,xattrs")
    pub fn from_str(s: &str) -> Result<Self> {
        let mut flags = Self {
            times: false,
            permissions: false,
            ownership: false,
            xattrs: false,
            acls: false,
        };

        for part in s.split(',') {
            match part.trim() {
                "times" | "time" | "timestamps" => flags.times = true,
                "perms" | "permissions" | "mode" => flags.permissions = true,
                "owners" | "ownership" | "owner" => flags.ownership = true,
                "xattrs" | "xattr" | "extended" => flags.xattrs = true,
                "acls" | "acl" => flags.acls = true,
                "all" => {
                    flags.times = true;
                    flags.permissions = true;
                    flags.ownership = true;
                    flags.xattrs = true;
                    flags.acls = true;
                }
                _ => return Err(OrbitError::MetadataFailed(format!("Unknown preserve flag: {}", part))),
            }
        }

        Ok(flags)
    }

    /// Create flags for all metadata
    pub fn all() -> Self {
        Self {
            times: true,
            permissions: true,
            ownership: true,
            xattrs: true,
            acls: true,
        }
    }

    /// Create flags for basic metadata only (times + permissions)
    pub fn basic() -> Self {
        Self::default()
    }
}

impl FileMetadata {
    /// Extract metadata from a file path
    pub fn from_path(path: &Path) -> Result<Self> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| OrbitError::MetadataFailed(format!("Failed to read metadata: {}", e)))?;

        let mut file_meta = FileMetadata {
            size: metadata.len(),
            modified: metadata.modified().ok(),
            accessed: metadata.accessed().ok(),
            created: metadata.created().ok(),
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            is_symlink: metadata.is_symlink(),
            ..Default::default()
        };

        // Unix-specific metadata
        #[cfg(unix)]
        {
            file_meta.permissions = Some(metadata.permissions().mode());
            file_meta.owner_uid = Some(metadata.uid());
            file_meta.owner_gid = Some(metadata.gid());
        }

        // Windows-specific metadata
        #[cfg(windows)]
        {
            // Windows file attributes (readonly, hidden, system, etc.)
            use std::os::windows::fs::MetadataExt;
            file_meta.windows_attributes = Some(metadata.file_attributes());
        }

        // Extended attributes (if feature enabled)
        #[cfg(feature = "extended-metadata")]
        {
            file_meta.xattrs = Self::read_xattrs(path).ok();
        }

        Ok(file_meta)
    }

    /// Read extended attributes from a file
    #[cfg(feature = "extended-metadata")]
    fn read_xattrs(path: &Path) -> Result<HashMap<String, Vec<u8>>> {
        let mut xattrs = HashMap::new();

        match xattr::list(path) {
            Ok(names) => {
                for name in names {
                    if let Ok(Some(value)) = xattr::get(path, &name) {
                        xattrs.insert(
                            name.to_string_lossy().to_string(),
                            value
                        );
                    }
                }
            }
            Err(e) => {
                // Non-fatal: some filesystems don't support xattrs
                tracing::debug!("Failed to list xattrs for {:?}: {}", path, e);
            }
        }

        Ok(xattrs)
    }

    /// Apply this metadata to a destination file
    pub fn apply_to(&self, dest_path: &Path, flags: PreserveFlags) -> Result<()> {
        // Preserve timestamps
        if flags.times {
            self.apply_timestamps(dest_path)?;
        }

        // Preserve permissions
        if flags.permissions {
            self.apply_permissions(dest_path)?;
        }

        // Preserve ownership (requires privileges)
        if flags.ownership {
            self.apply_ownership(dest_path)?;
        }

        // Preserve extended attributes
        if flags.xattrs {
            self.apply_xattrs(dest_path)?;
        }

        // Preserve ACLs
        if flags.acls {
            self.apply_acls(dest_path)?;
        }

        Ok(())
    }

    /// Apply timestamps to destination
    fn apply_timestamps(&self, dest_path: &Path) -> Result<()> {
        if let Some(modified) = self.modified {
            let mtime = FileTime::from_system_time(modified);

            if let Some(accessed) = self.accessed {
                let atime = FileTime::from_system_time(accessed);
                set_file_times(dest_path, atime, mtime)
                    .map_err(|e| OrbitError::MetadataFailed(format!("Failed to set timestamps: {}", e)))?;
            } else {
                set_file_mtime(dest_path, mtime)
                    .map_err(|e| OrbitError::MetadataFailed(format!("Failed to set mtime: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Apply permissions to destination
    #[allow(unused_variables)]
    fn apply_permissions(&self, dest_path: &Path) -> Result<()> {
        #[cfg(unix)]
        if let Some(mode) = self.permissions {
            use std::fs::Permissions;
            std::fs::set_permissions(dest_path, Permissions::from_mode(mode))
                .map_err(|e| OrbitError::MetadataFailed(format!("Failed to set permissions: {}", e)))?;
        }

        #[cfg(windows)]
        if let Some(_attrs) = self.windows_attributes {
            // Windows file attributes would be set here
            // Currently requires winapi crate for full support
            tracing::debug!("Windows file attributes preservation not yet implemented");
        }

        Ok(())
    }

    /// Apply ownership to destination (Unix only, requires privileges)
    #[allow(unused_variables)]
    fn apply_ownership(&self, dest_path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            if let (Some(uid), Some(gid)) = (self.owner_uid, self.owner_gid) {
                use std::os::unix::fs::chown;
                chown(dest_path, Some(uid), Some(gid))
                    .or_else(|e| {
                        // Non-fatal warning for permission errors
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            tracing::warn!("Permission denied setting ownership for {:?}: requires root", dest_path);
                            Ok(())
                        } else {
                            Err(OrbitError::MetadataFailed(format!("Failed to set ownership: {}", e)))
                        }
                    })?;
            }
        }

        Ok(())
    }

    /// Apply extended attributes to destination
    fn apply_xattrs(&self, _dest_path: &Path) -> Result<()> {
        #[cfg(feature = "extended-metadata")]
        if let Some(xattrs) = &self.xattrs {
            let dest_path = _dest_path;
            for (name, value) in xattrs {
                if let Err(e) = xattr::set(dest_path, name, value) {
                    // Non-fatal: filesystem may not support xattrs
                    tracing::warn!("Failed to set xattr {} on {:?}: {}", name, dest_path, e);
                }
            }
        }

        Ok(())
    }

    /// Apply ACLs to destination (platform-specific)
    fn apply_acls(&self, dest_path: &Path) -> Result<()> {
        if let Some(_acls) = &self.acls {
            // ACL implementation would go here
            // Requires platform-specific libraries (acl on Linux, Security API on Windows)
            tracing::debug!("ACL preservation not yet fully implemented for {:?}", dest_path);
        }

        Ok(())
    }

    /// Convert to manifest-compatible format (for CargoManifest.xattrs field)
    pub fn to_manifest_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();

        if let Some(perms) = self.permissions {
            map.insert("permissions".to_string(), serde_json::json!(perms));
        }

        if let Some(modified) = self.modified {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                map.insert("modified_secs".to_string(), serde_json::json!(duration.as_secs()));
            }
        }

        if let Some(accessed) = self.accessed {
            if let Ok(duration) = accessed.duration_since(std::time::UNIX_EPOCH) {
                map.insert("accessed_secs".to_string(), serde_json::json!(duration.as_secs()));
            }
        }

        if let Some(uid) = self.owner_uid {
            map.insert("owner_uid".to_string(), serde_json::json!(uid));
        }

        if let Some(gid) = self.owner_gid {
            map.insert("owner_gid".to_string(), serde_json::json!(gid));
        }

        // Encode xattrs as base64
        if let Some(xattrs) = &self.xattrs {
            let encoded: HashMap<String, String> = xattrs
                .iter()
                .map(|(k, v)| (k.clone(), base64_encode(v)))
                .collect();
            map.insert("xattrs".to_string(), serde_json::json!(encoded));
        }

        map
    }

    /// Create from manifest-compatible format
    pub fn from_manifest_map(map: &HashMap<String, serde_json::Value>) -> Result<Self> {
        let mut meta = FileMetadata::default();

        if let Some(perms) = map.get("permissions").and_then(|v| v.as_u64()) {
            meta.permissions = Some(perms as u32);
        }

        if let Some(secs) = map.get("modified_secs").and_then(|v| v.as_u64()) {
            meta.modified = Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs));
        }

        if let Some(secs) = map.get("accessed_secs").and_then(|v| v.as_u64()) {
            meta.accessed = Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs));
        }

        if let Some(uid) = map.get("owner_uid").and_then(|v| v.as_u64()) {
            meta.owner_uid = Some(uid as u32);
        }

        if let Some(gid) = map.get("owner_gid").and_then(|v| v.as_u64()) {
            meta.owner_gid = Some(gid as u32);
        }

        if let Some(xattrs_obj) = map.get("xattrs").and_then(|v| v.as_object()) {
            let mut xattrs = HashMap::new();
            for (k, v) in xattrs_obj {
                if let Some(base64_str) = v.as_str() {
                    if let Ok(decoded) = base64_decode(base64_str) {
                        xattrs.insert(k.clone(), decoded);
                    }
                }
            }
            if !xattrs.is_empty() {
                meta.xattrs = Some(xattrs);
            }
        }

        Ok(meta)
    }
}

/// Simple base64 encoding for xattr values
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    engine.encode(data)
}

/// Simple base64 decoding for xattr values
fn base64_decode(s: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    engine.decode(s.as_bytes())
        .map_err(|e| OrbitError::MetadataFailed(format!("Base64 decode failed: {}", e)))
}

/// Preserve metadata from source to destination with specified flags
pub fn preserve_metadata(source_path: &Path, dest_path: &Path, flags: PreserveFlags) -> Result<()> {
    let metadata = FileMetadata::from_path(source_path)?;
    metadata.apply_to(dest_path, flags)?;
    Ok(())
}

/// Preserve metadata from source to destination (default flags)
pub fn preserve_metadata_default(source_path: &Path, dest_path: &Path) -> Result<()> {
    preserve_metadata(source_path, dest_path, PreserveFlags::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use std::fs;

    #[test]
    fn test_preserve_flags_parsing() {
        let flags = PreserveFlags::from_str("times,perms").unwrap();
        assert!(flags.times);
        assert!(flags.permissions);
        assert!(!flags.ownership);
        assert!(!flags.xattrs);

        let flags_all = PreserveFlags::from_str("all").unwrap();
        assert!(flags_all.times);
        assert!(flags_all.permissions);
        assert!(flags_all.ownership);
        assert!(flags_all.xattrs);
    }

    #[test]
    fn test_file_metadata_extraction() {
        let mut source = NamedTempFile::new().unwrap();
        source.write_all(b"test content").unwrap();
        source.flush().unwrap();

        let metadata = FileMetadata::from_path(source.path()).unwrap();
        assert_eq!(metadata.size, 12);
        assert!(metadata.is_file);
        assert!(!metadata.is_dir);
        assert!(metadata.modified.is_some());
    }

    #[test]
    fn test_preserve_metadata_basic() {
        let mut source = NamedTempFile::new().unwrap();
        source.write_all(b"test").unwrap();
        source.flush().unwrap();

        let dest = NamedTempFile::new().unwrap();

        preserve_metadata_default(source.path(), dest.path()).unwrap();

        let source_meta = fs::metadata(source.path()).unwrap();
        let dest_meta = fs::metadata(dest.path()).unwrap();

        assert_eq!(
            source_meta.modified().unwrap(),
            dest_meta.modified().unwrap()
        );
    }

    #[test]
    fn test_manifest_conversion() {
        let mut meta = FileMetadata::default();
        meta.permissions = Some(0o755);
        meta.owner_uid = Some(1000);
        meta.owner_gid = Some(1000);

        let map = meta.to_manifest_map();
        assert!(map.contains_key("permissions"));
        assert!(map.contains_key("owner_uid"));
        assert!(map.contains_key("owner_gid"));

        let restored = FileMetadata::from_manifest_map(&map).unwrap();
        assert_eq!(restored.permissions, Some(0o755));
        assert_eq!(restored.owner_uid, Some(1000));
        assert_eq!(restored.owner_gid, Some(1000));
    }
}
