use crate::entangler::Entangler;
use crate::inode::GhostFile;
use dashmap::DashMap;
use fuser::{Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request};
use std::ffi::OsStr;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

const BLOCK_SIZE: u64 = 1024 * 1024; // 1MB Blocks for simplicity

pub struct OrbitGhostFS {
    pub inodes: Arc<DashMap<u64, GhostFile>>,
    pub entangler: Arc<Entangler>,
    pub cache_path: String,
}

impl Filesystem for OrbitGhostFS {
    fn lookup(&mut self, _req: &Request, _parent: u64, name: &OsStr, reply: ReplyEntry) {
        // Scan dashmap for parent + name. Ideally use a BTree for hierarchy.
        // Simplified: Just returning a hardcoded match for demo.
        let name_str = name.to_str().unwrap();

        for r in self.inodes.iter() {
            if r.value().name == name_str {
                reply.entry(&Duration::new(1, 0), &r.value().to_attr(*r.key()), 0);
                return;
            }
        }
        reply.error(libc::ENOENT);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match self.inodes.get(&ino) {
            Some(f) => reply.attr(&Duration::new(1, 0), &f.to_attr(ino)),
            None => reply.error(libc::ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        // In standard FUSE, offset 0 is "start".
        if offset == 0 {
            reply.add(1, 0, fuser::FileType::Directory, ".");
            reply.add(1, 1, fuser::FileType::Directory, "..");

            for r in self.inodes.iter() {
                if *r.key() != 1 {
                    // Skip root
                    let kind = if r.value().is_dir {
                        fuser::FileType::Directory
                    } else {
                        fuser::FileType::RegularFile
                    };
                    // The magic: users see files instantly because we fake this list from the Manifest
                    reply.add(*r.key(), offset + 2, kind, &r.value().name);
                }
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
        let inode_entry = self.inodes.get(&ino);
        if inode_entry.is_none() {
            reply.error(libc::ENOENT);
            return;
        }
        let file_info = inode_entry.unwrap();

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
                .ensure_block_available(&file_info.orbit_id, block_idx);

            // 4. Read from Cache (Simulated)
            let path = format!(
                "{}/{}_{}.bin",
                self.cache_path, file_info.orbit_id, block_idx
            );
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
