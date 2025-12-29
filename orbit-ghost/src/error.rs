use thiserror::Error;

#[derive(Debug, Error)]
pub enum GhostError {
    #[error("Database query failed: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Artifact not found: {0}")]
    NotFound(String),

    #[error("Database operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Invalid inode: {0}")]
    InvalidInode(u64),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl GhostError {
    /// Convert to FUSE error code (libc errno)
    pub fn to_errno(&self) -> i32 {
        match self {
            GhostError::NotFound(_) | GhostError::InvalidInode(_) => libc::ENOENT,
            GhostError::Timeout(_) => libc::ETIMEDOUT,
            GhostError::Database(_) | GhostError::Io(_) => libc::EIO,
        }
    }
}
