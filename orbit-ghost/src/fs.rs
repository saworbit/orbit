use crate::entangler::Entangler;
use crate::error::GhostError;
use crate::oracle::MetadataOracle;
use crate::translator::InodeTranslator;
use fuser::{Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request};
use std::ffi::OsStr;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

const BLOCK_SIZE: u64 = 1024 * 1024; // 1MB Blocks for simplicity
const TTL: Duration = Duration::from_secs(1); // Attribute TTL

pub struct OrbitGhostFS {
    oracle: Arc<dyn MetadataOracle>,
    translator: Arc<InodeTranslator>,
    entangler: Arc<Entangler>,
    runtime_handle: tokio::runtime::Handle,
    cache_path: String,
}

impl OrbitGhostFS {
    pub fn new(
        oracle: Arc<dyn MetadataOracle>,
        translator: Arc<InodeTranslator>,
        entangler: Arc<Entangler>,
        runtime_handle: tokio::runtime::Handle,
        cache_path: String,
    ) -> Self {
        Self {
            oracle,
            translator,
            entangler,
            runtime_handle,
            cache_path,
        }
    }

    /// Helper to run async queries synchronously (blocking bridge)
    fn block_on<F, T>(&self, future: F) -> Result<T, i32>
    where
        F: std::future::Future<Output = Result<T, GhostError>>,
    {
        self.runtime_handle.block_on(future).map_err(|e| {
            log::error!("Query failed: {}", e);
            e.to_errno()
        })
    }
}

impl Filesystem for OrbitGhostFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = match name.to_str() {
            Some(n) => n,
            None => {
                reply.error(libc::EINVAL);
                return;
            },
        };

        // Translate parent inode to artifact ID
        let parent_id = match self.translator.to_artifact_id(parent) {
            Ok(id) => id,
            Err(e) => {
                reply.error(e.to_errno());
                return;
            },
        };

        // Query database for child
        let entry = match self.block_on(self.oracle.lookup(&parent_id, name_str)) {
            Ok(Some(e)) => e,
            Ok(None) => {
                reply.error(libc::ENOENT);
                return;
            },
            Err(errno) => {
                reply.error(errno);
                return;
            },
        };

        // Allocate inode for child
        let inode = self.translator.get_or_allocate(&entry.id);

        log::debug!("lookup({}, {}) -> inode {}", parent, name_str, inode);
        reply.entry(&TTL, &entry.to_attr(inode), 0);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        // Special case: root directory
        if ino == 1 {
            let attr = fuser::FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: std::time::UNIX_EPOCH,
                mtime: std::time::UNIX_EPOCH,
                ctime: std::time::UNIX_EPOCH,
                crtime: std::time::UNIX_EPOCH,
                kind: fuser::FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: 501,
                gid: 20,
                rdev: 0,
                blksize: 512,
                flags: 0,
            };
            reply.attr(&TTL, &attr);
            return;
        }

        // Translate inode to artifact ID
        let artifact_id = match self.translator.to_artifact_id(ino) {
            Ok(id) => id,
            Err(e) => {
                reply.error(e.to_errno());
                return;
            },
        };

        // Query database for attributes
        let entry = match self.block_on(self.oracle.getattr(&artifact_id)) {
            Ok(e) => e,
            Err(errno) => {
                reply.error(errno);
                return;
            },
        };

        log::debug!("getattr({}) -> {}", ino, entry.name);
        reply.attr(&TTL, &entry.to_attr(ino));
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if offset > 0 {
            reply.ok();
            return;
        }

        // Add . and ..
        if reply.add(ino, 0, fuser::FileType::Directory, ".") {
            reply.ok();
            return;
        }
        if reply.add(ino, 1, fuser::FileType::Directory, "..") {
            reply.ok();
            return;
        }

        // Translate inode to artifact ID
        let artifact_id = match self.translator.to_artifact_id(ino) {
            Ok(id) => id,
            Err(e) => {
                reply.error(e.to_errno());
                return;
            },
        };

        // Query database for children
        let entries = match self.block_on(self.oracle.readdir(&artifact_id)) {
            Ok(e) => e,
            Err(errno) => {
                reply.error(errno);
                return;
            },
        };

        log::debug!("readdir({}) -> {} entries", ino, entries.len());

        for (idx, entry) in entries.iter().enumerate() {
            let child_inode = self.translator.get_or_allocate(&entry.id);
            let kind = if entry.is_dir {
                fuser::FileType::Directory
            } else {
                fuser::FileType::RegularFile
            };

            if reply.add(child_inode, (idx + 2) as i64, kind, &entry.name) {
                break;
            }
        }

        reply.ok();
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        // Translate inode to artifact ID
        let artifact_id = match self.translator.to_artifact_id(ino) {
            Ok(id) => id,
            Err(e) => {
                reply.error(e.to_errno());
                return;
            },
        };

        // 1. Calculate Block ID
        let start_block = offset as u64 / BLOCK_SIZE;
        let end_block = (offset as u64 + size as u64) / BLOCK_SIZE;

        let mut final_buffer = Vec::new();

        // 2. Iterate required blocks
        for block_idx in start_block..=end_block {
            // 3. QUANTUM ENTANGLEMENT MOMENT
            // If this call blocks, the application (e.g., Video Player) waits.
            // Behind the scenes, we are downloading at max speed.
            self.entangler
                .ensure_block_available(&artifact_id, block_idx);

            // 4. Read from Cache
            let path = format!("{}/{}_{}.bin", self.cache_path, artifact_id, block_idx);
            if let Ok(mut f) = std::fs::File::open(&path) {
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer).unwrap();
                final_buffer.extend_from_slice(&buffer);
            } else {
                // Should not happen if ensure_block_available works
                reply.error(libc::EIO);
                return;
            }
        }

        // 5. Slice exact byte range requested
        let relative_offset = (offset as u64 % BLOCK_SIZE) as usize;
        let read_len = size as usize;

        if final_buffer.len() >= relative_offset + read_len {
            reply.data(&final_buffer[relative_offset..relative_offset + read_len]);
        } else {
            // EOF handling
            if relative_offset < final_buffer.len() {
                reply.data(&final_buffer[relative_offset..]);
            } else {
                reply.data(&[]);
            }
        }
    }
}
