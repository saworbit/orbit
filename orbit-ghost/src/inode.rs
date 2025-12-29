use fuser::FileAttr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct GhostFile {
    pub name: String,
    pub size: u64,
    pub orbit_id: String, // Reference to internal Orbit ID
    pub is_dir: bool,
    // Bitmap of blocks present locally. In prod, use a RoaringBitmap.
    pub blocks_present: Vec<bool>,
}

impl GhostFile {
    pub fn to_attr(&self, inode: u64) -> FileAttr {
        FileAttr {
            ino: inode,
            size: self.size,
            blocks: (self.size + 511) / 512,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: if self.is_dir {
                fuser::FileType::Directory
            } else {
                fuser::FileType::RegularFile
            },
            perm: if self.is_dir { 0o755 } else { 0o644 },
            nlink: 1,
            uid: 501, // Demo user
            gid: 20,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }
}

// New struct for database-backed entries (Phase 2)
#[derive(Debug, Clone)]
pub struct GhostEntry {
    pub id: String, // Artifact ID from database
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub mtime: u64, // Unix timestamp
}

impl GhostEntry {
    pub fn to_attr(&self, inode: u64) -> FileAttr {
        use std::time::Duration;

        FileAttr {
            ino: inode,
            size: self.size,
            blocks: (self.size + 511) / 512,
            atime: UNIX_EPOCH + Duration::from_secs(self.mtime),
            mtime: UNIX_EPOCH + Duration::from_secs(self.mtime),
            ctime: UNIX_EPOCH + Duration::from_secs(self.mtime),
            crtime: UNIX_EPOCH + Duration::from_secs(self.mtime),
            kind: if self.is_dir {
                fuser::FileType::Directory
            } else {
                fuser::FileType::RegularFile
            },
            perm: if self.is_dir { 0o755 } else { 0o644 },
            nlink: 1,
            uid: 501,
            gid: 20,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }
}
