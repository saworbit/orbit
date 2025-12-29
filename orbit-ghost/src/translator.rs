use crate::error::GhostError;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// InodeTranslator provides bidirectional mapping between FUSE inodes (u64)
/// and Orbit artifact IDs (String). This enables lazy inode allocation as
/// files are accessed, without requiring a full database scan at mount time.
pub struct InodeTranslator {
    /// Forward mapping: inode → artifact_id
    inode_to_id: DashMap<u64, String>,
    /// Reverse mapping: artifact_id → inode (ensures stable inodes per session)
    id_to_inode: DashMap<String, u64>,
    /// Atomic counter for allocating new inodes
    next_inode: AtomicU64,
}

impl InodeTranslator {
    /// Create a new translator with root inode pre-allocated
    pub fn new() -> Self {
        let translator = Self {
            inode_to_id: DashMap::new(),
            id_to_inode: DashMap::new(),
            next_inode: AtomicU64::new(2), // Start at 2 (1 is reserved for root)
        };

        // Bootstrap root inode
        translator.inode_to_id.insert(1, "root".to_string());
        translator.id_to_inode.insert("root".to_string(), 1);

        translator
    }

    /// Get existing inode for artifact ID, or allocate a new one
    pub fn get_or_allocate(&self, artifact_id: &str) -> u64 {
        // Fast path: check if already allocated
        if let Some(inode) = self.id_to_inode.get(artifact_id) {
            return *inode;
        }

        // Slow path: allocate new inode atomically
        let inode = self.next_inode.fetch_add(1, Ordering::SeqCst);
        self.inode_to_id.insert(inode, artifact_id.to_string());
        self.id_to_inode.insert(artifact_id.to_string(), inode);

        inode
    }

    /// Translate inode to artifact ID (reverse lookup)
    pub fn to_artifact_id(&self, inode: u64) -> Result<String, GhostError> {
        self.inode_to_id
            .get(&inode)
            .map(|entry| entry.value().clone())
            .ok_or(GhostError::InvalidInode(inode))
    }
}
