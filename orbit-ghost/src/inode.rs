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
