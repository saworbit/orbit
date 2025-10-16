/*!
 * Orbit - Intelligent file transfer library
 * 
 * Provides high-performance file copying with features including:
 * - Zero-copy system calls for maximum performance
 * - Compression (LZ4, Zstd)
 * - Resume capability for interrupted transfers
 * - Checksum verification
 * - Protocol abstraction (Local, SMB, S3, etc.)
 * - Parallel directory copying
 * - Bandwidth throttling
 * - Progress tracking
 */

pub mod config;
pub mod core;
pub mod error;
pub mod compression;
pub mod stats;
pub mod audit;
pub mod protocol;

// Re-export commonly used types for convenience
pub use config::{CopyConfig, CopyMode, CompressionType, SymlinkMode};
pub use error::{OrbitError, Result};
pub use core::{CopyStats, copy_file, copy_directory};
pub use core::zero_copy::{ZeroCopyCapabilities, ZeroCopyResult};
pub use stats::TransferStats;
pub use protocol::Protocol;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if zero-copy is available on this platform
/// 
/// # Example
/// ```
/// use orbit::is_zero_copy_available;
/// 
/// if is_zero_copy_available() {
///     println!("Zero-copy transfers available for maximum performance!");
/// }
/// ```
pub fn is_zero_copy_available() -> bool {
    ZeroCopyCapabilities::detect().available
}

/// Get detailed zero-copy capabilities for this platform
/// 
/// # Example
/// ```
/// use orbit::get_zero_copy_capabilities;
/// 
/// let caps = get_zero_copy_capabilities();
/// println!("Zero-copy method: {}", caps.method);
/// println!("Cross-filesystem: {}", caps.cross_filesystem);
/// ```
pub fn get_zero_copy_capabilities() -> ZeroCopyCapabilities {
    ZeroCopyCapabilities::detect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        assert_eq!(VERSION, "0.4.0");
    }

    #[test]
    fn test_zero_copy_detection() {
        // Should not panic
        let available = is_zero_copy_available();
        let caps = get_zero_copy_capabilities();
        
        assert_eq!(available, caps.available);
    }
}