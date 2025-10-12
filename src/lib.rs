/*!
 * Orbit - Open Resilient Bulk Information Transfer
 * 
 * A robust, cross-platform file transfer library with:
 * - SHA-256 checksum verification
 * - LZ4 and Zstd compression
 * - Resume capability for interrupted transfers
 * - Configurable retry logic with exponential backoff
 * - Parallel file copying
 * - Protocol abstraction (local, SMB, future: S3, etc.)
 * - Comprehensive audit logging
 * 
 * Version: 0.3.0
 * Author: Shane Wall <shaneawall@gmail.com>
 */

pub mod config;
pub mod core;
pub mod compression;
pub mod audit;
pub mod error;
pub mod stats;
pub mod protocol;

// Re-export commonly used types
pub use config::{CopyConfig, CompressionType, SymlinkMode, CopyMode, AuditFormat};
pub use core::{copy_file, copy_directory, CopyStats};
pub use error::{OrbitError, Result};
pub use stats::TransferStats;
pub use protocol::{Protocol, StorageBackend};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}