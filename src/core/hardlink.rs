/*!
 * Hardlink detection and preservation during directory transfers.
 *
 * When files share the same inode (hardlinks), copying them naively creates
 * independent copies, potentially multiplying disk usage by N. This module
 * detects hardlink groups during directory scanning and recreates them at the
 * destination.
 *
 * rsync requires an explicit `-H` flag and rebuilds hardlink groups from
 * inode tracking. Orbit does the same but integrates with the Star Map's
 * content-addressing so that hardlinked content is naturally deduplicated
 * even across different directory trees.
 */

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Tracks files by their (device, inode) identity to detect hardlink groups.
///
/// During a directory scan, each file is checked against this tracker.
/// The first file with a given inode is recorded; subsequent files with the
/// same inode are identified as hardlinks to the first.
#[derive(Debug)]
pub struct HardlinkTracker {
    /// Maps (device_id, inode) to the first path we encountered with that identity.
    seen: HashMap<InodeKey, PathBuf>,
    /// Total hardlinks detected (excluding the first occurrence of each group)
    pub links_detected: u64,
}

/// Platform-independent inode identity key.
/// On Unix: (dev, ino). On Windows: (volume_serial, file_index).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct InodeKey {
    device: u64,
    inode: u64,
}

impl HardlinkTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            seen: HashMap::new(),
            links_detected: 0,
        }
    }

    /// Check whether a file is a subsequent hardlink to an already-seen inode.
    ///
    /// Returns `Some(original_path)` if this file's inode was already seen,
    /// meaning we should create a hardlink at the destination instead of copying.
    /// Returns `None` if this is the first occurrence (should be copied normally).
    ///
    /// Only files with nlink > 1 are tracked — files with exactly one link
    /// cannot be hardlinks and are skipped for efficiency.
    #[cfg(unix)]
    pub fn check(&mut self, path: &Path, metadata: &std::fs::Metadata) -> Option<PathBuf> {
        use std::os::unix::fs::MetadataExt;

        let nlink = metadata.nlink();
        if nlink <= 1 {
            return None; // Single-link file, cannot be part of a hardlink group
        }

        let key = InodeKey {
            device: metadata.dev(),
            inode: metadata.ino(),
        };

        if let Some(original) = self.seen.get(&key) {
            self.links_detected += 1;
            Some(original.clone())
        } else {
            self.seen.insert(key, path.to_owned());
            None
        }
    }

    /// Windows implementation: uses file_index from BY_HANDLE_FILE_INFORMATION.
    ///
    /// Note: NTFS supports hardlinks but the metadata API differs from Unix.
    /// We use volume_serial_number + file_index as the identity key.
    #[cfg(windows)]
    pub fn check(&mut self, path: &Path, _metadata: &std::fs::Metadata) -> Option<PathBuf> {
        // On Windows, we need to open the file to get the file index
        // (the standard Metadata doesn't expose it directly).
        // Use file_index() which returns Option<u64> on nightly, or
        // fall back to the windows-specific API.
        match get_windows_file_identity(path) {
            Some(key) => {
                if let Some(original) = self.seen.get(&key) {
                    self.links_detected += 1;
                    Some(original.clone())
                } else {
                    self.seen.insert(key, path.to_owned());
                    None
                }
            }
            None => None, // Could not determine identity, treat as unique file
        }
    }

    /// Number of unique inode groups being tracked.
    pub fn groups_tracked(&self) -> usize {
        self.seen.len()
    }
}

impl Default for HardlinkTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Get Windows file identity (volume serial + file index) for hardlink detection.
///
/// Uses GetFileInformationByHandle via raw FFI to avoid depending on the
/// `windows-sys` crate. NTFS file indexes uniquely identify files, and
/// nNumberOfLinks tells us if hardlinks exist.
#[cfg(windows)]
fn get_windows_file_identity(path: &Path) -> Option<InodeKey> {
    use std::os::windows::io::AsRawHandle;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct ByHandleFileInformation {
        dwFileAttributes: u32,
        ftCreationTime: [u32; 2],
        ftLastAccessTime: [u32; 2],
        ftLastWriteTime: [u32; 2],
        dwVolumeSerialNumber: u32,
        nFileSizeHigh: u32,
        nFileSizeLow: u32,
        nNumberOfLinks: u32,
        nFileIndexHigh: u32,
        nFileIndexLow: u32,
    }

    extern "system" {
        fn GetFileInformationByHandle(
            hFile: *mut std::ffi::c_void,
            lpFileInformation: *mut ByHandleFileInformation,
        ) -> i32;
    }

    let file = std::fs::File::open(path).ok()?;
    let handle = file.as_raw_handle();

    unsafe {
        let mut info: ByHandleFileInformation = std::mem::zeroed();
        if GetFileInformationByHandle(handle as *mut _, &mut info) != 0 {
            if info.nNumberOfLinks <= 1 {
                return None;
            }
            Some(InodeKey {
                device: info.dwVolumeSerialNumber as u64,
                inode: ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64),
            })
        } else {
            None
        }
    }
}

/// Create a hardlink at `link_path` pointing to `original_path`.
///
/// The `original_path` must be a destination-side path that was already
/// written during this transfer. The hardlink shares the same inode and data.
pub fn create_hardlink(original_path: &Path, link_path: &Path) -> std::io::Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove existing file at link_path if present (hardlink creation fails otherwise)
    if link_path.exists() {
        std::fs::remove_file(link_path)?;
    }

    std::fs::hard_link(original_path, link_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_hardlink() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("original.txt");
        let link = dir.path().join("link.txt");

        std::fs::write(&original, b"hardlink test data").unwrap();
        create_hardlink(&original, &link).unwrap();

        // Both files should have the same content
        assert_eq!(
            std::fs::read(&original).unwrap(),
            std::fs::read(&link).unwrap()
        );

        // Modifying one should affect the other (they share data)
        std::fs::write(&link, b"modified").unwrap();
        assert_eq!(std::fs::read(&original).unwrap(), b"modified");
    }

    #[test]
    fn test_create_hardlink_with_parent_dirs() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("original.txt");
        let link = dir.path().join("sub").join("dir").join("link.txt");

        std::fs::write(&original, b"data").unwrap();
        create_hardlink(&original, &link).unwrap();

        assert_eq!(std::fs::read(&link).unwrap(), b"data");
    }

    #[test]
    fn test_create_hardlink_overwrites_existing() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("original.txt");
        let link = dir.path().join("link.txt");

        std::fs::write(&original, b"original data").unwrap();
        std::fs::write(&link, b"old data").unwrap();

        create_hardlink(&original, &link).unwrap();
        assert_eq!(std::fs::read(&link).unwrap(), b"original data");
    }

    #[cfg(unix)]
    #[test]
    fn test_hardlink_tracker_detects_links() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.txt");
        let file_b = dir.path().join("b.txt");

        std::fs::write(&file_a, b"shared data").unwrap();
        std::fs::hard_link(&file_a, &file_b).unwrap();

        let mut tracker = HardlinkTracker::new();

        let meta_a = std::fs::metadata(&file_a).unwrap();
        let meta_b = std::fs::metadata(&file_b).unwrap();

        // First occurrence — not a duplicate
        assert!(tracker.check(&file_a, &meta_a).is_none());

        // Second occurrence — detected as hardlink to first
        let original = tracker.check(&file_b, &meta_b);
        assert_eq!(original, Some(file_a));
        assert_eq!(tracker.links_detected, 1);
    }

    #[cfg(unix)]
    #[test]
    fn test_hardlink_tracker_ignores_single_link() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("solo.txt");
        std::fs::write(&file, b"solo").unwrap();

        let mut tracker = HardlinkTracker::new();
        let meta = std::fs::metadata(&file).unwrap();

        // File with nlink=1 should always return None
        assert!(tracker.check(&file, &meta).is_none());
        assert_eq!(tracker.groups_tracked(), 0);
    }

    #[test]
    fn test_create_hardlink_nonexistent_original() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("ghost.txt");
        let link = dir.path().join("link.txt");

        let result = create_hardlink(&original, &link);
        assert!(result.is_err());
    }

    #[test]
    fn test_hardlink_tracker_default() {
        let tracker = HardlinkTracker::default();
        assert_eq!(tracker.groups_tracked(), 0);
        assert_eq!(tracker.links_detected, 0);
    }

    #[cfg(unix)]
    #[test]
    fn test_hardlink_tracker_multiple_groups() {
        let dir = tempdir().unwrap();

        // Group 1: a.txt and b.txt
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, b"group 1 data").unwrap();
        std::fs::hard_link(&a, &b).unwrap();

        // Group 2: c.txt and d.txt
        let c = dir.path().join("c.txt");
        let d = dir.path().join("d.txt");
        std::fs::write(&c, b"group 2 data").unwrap();
        std::fs::hard_link(&c, &d).unwrap();

        // Solo file: e.txt (nlink=1)
        let e = dir.path().join("e.txt");
        std::fs::write(&e, b"solo").unwrap();

        let mut tracker = HardlinkTracker::new();

        let meta_a = std::fs::metadata(&a).unwrap();
        let meta_b = std::fs::metadata(&b).unwrap();
        let meta_c = std::fs::metadata(&c).unwrap();
        let meta_d = std::fs::metadata(&d).unwrap();
        let meta_e = std::fs::metadata(&e).unwrap();

        assert!(tracker.check(&a, &meta_a).is_none());
        assert!(tracker.check(&b, &meta_b).is_some());
        assert!(tracker.check(&c, &meta_c).is_none());
        assert!(tracker.check(&d, &meta_d).is_some());
        assert!(tracker.check(&e, &meta_e).is_none());

        assert_eq!(tracker.groups_tracked(), 2); // Only groups with nlink > 1
        assert_eq!(tracker.links_detected, 2);
    }

    #[cfg(unix)]
    #[test]
    fn test_hardlink_tracker_three_way_link() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        let c = dir.path().join("c.txt");

        std::fs::write(&a, b"triple").unwrap();
        std::fs::hard_link(&a, &b).unwrap();
        std::fs::hard_link(&a, &c).unwrap();

        let mut tracker = HardlinkTracker::new();

        let meta_a = std::fs::metadata(&a).unwrap();
        let meta_b = std::fs::metadata(&b).unwrap();
        let meta_c = std::fs::metadata(&c).unwrap();

        assert!(tracker.check(&a, &meta_a).is_none());
        let orig_b = tracker.check(&b, &meta_b);
        assert_eq!(orig_b, Some(a.clone()));
        let orig_c = tracker.check(&c, &meta_c);
        assert_eq!(orig_c, Some(a.clone()));

        assert_eq!(tracker.groups_tracked(), 1);
        assert_eq!(tracker.links_detected, 2);
    }
}
