use crate::error::GhostError;
use crate::inode::GhostEntry;
use async_trait::async_trait;

/// MetadataOracle provides an abstraction over the metadata storage backend
/// (Magnetar database) for querying file hierarchy information.
#[async_trait]
pub trait MetadataOracle: Send + Sync {
    /// Get the root artifact ID for the configured job
    async fn get_root_id(&self) -> Result<String, GhostError>;

    /// Look up a child by name under parent directory
    async fn lookup(&self, parent_id: &str, name: &str) -> Result<Option<GhostEntry>, GhostError>;

    /// List all children of a directory
    async fn readdir(&self, parent_id: &str) -> Result<Vec<GhostEntry>, GhostError>;

    /// Get attributes for a specific artifact by ID
    async fn getattr(&self, id: &str) -> Result<GhostEntry, GhostError>;
}
