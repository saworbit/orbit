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
pub mod manifest_integration;

// Native SMB protocol support (feature-gated)
#[cfg(feature = "smb-native")]
pub mod protocols;

// Manifest system modules (re-exported from workspace crates)
pub use orbit_core_manifest as manifest;
pub use orbit_core_starmap as starmap;
pub use orbit_core_audit as manifest_audit;

// Re-export commonly used types for convenience
pub use config::{CopyConfig, CopyMode, CompressionType, SymlinkMode, ChunkingStrategy};
pub use error::{OrbitError, Result};
pub use core::{CopyStats, copy_file, copy_directory};
pub use core::zero_copy::{ZeroCopyCapabilities, ZeroCopyResult};
pub use stats::TransferStats;
pub use protocol::Protocol;

// Manifest system convenience exports
pub mod manifests {
    //! Manifest system for transfer planning, verification, and audit
    //!
    //! Provides Flight Plans, Cargo Manifests, Star Maps, and audit logging.
    
    pub use orbit_core_manifest::{
        FlightPlan, CargoManifest, Endpoint, Policy, Encryption,
        WindowMeta, Chunking, Digests, FileRef,
        validate_flight_plan, validate_cargo_manifest,
    };
    
    pub use orbit_core_starmap::{
        StarMapBuilder, StarMapReader, 
        ChunkMeta, BloomFilter, RankSelectBitmap,
    };
    
    pub use orbit_core_audit::{
        TelemetryLogger, TelemetryEvent, EventType, 
        Beacon, BeaconBuilder,
    };
    
    // Re-export integration helpers
    pub use crate::manifest_integration::{ManifestGenerator, should_generate_manifest};
}

// Native SMB protocol convenience exports (feature-gated)
#[cfg(feature = "smb-native")]
pub mod smb {
    //! Native SMB2/3 protocol support
    //!
    //! Pure Rust, async SMB client for direct network share access without OS mounts.
    //!
    //! # Example
    //!
    //! ```no_run
    //! # #[cfg(feature = "smb-native")]
    //! # {
    //! use orbit::smb::{SmbTarget, SmbAuth, SmbSecurity, client_for, Secret};
    //!
    //! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    //! let target = SmbTarget {
    //!     host: "fileserver".to_string(),
    //!     share: "data".to_string(),
    //!     subpath: "reports".to_string(),
    //!     port: None,
    //!     auth: SmbAuth::Ntlmv2 {
    //!         username: "user".to_string(),
    //!         password: Secret("pass".to_string()),
    //!     },
    //!     security: SmbSecurity::RequireEncryption,
    //! };
    //!
    //! let mut client = client_for(&target).await?;
    //! let data = client.read_file("Q4/report.pdf", None).await?;
    //! # Ok(())
    //! # }
    //! # }
    //! ```
    
    pub use crate::protocols::smb::{
        SmbTarget, SmbAuth, SmbSecurity, SmbMetadata, SmbCapability,
        SmbClient, SmbError, Secret, client_for,
    };
}

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
        // Verify VERSION is properly initialized from CARGO_PKG_VERSION
        assert!(!VERSION.is_empty());
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_zero_copy_detection() {
        // Should not panic
        let available = is_zero_copy_available();
        let caps = get_zero_copy_capabilities();
        
        assert_eq!(available, caps.available);
    }
    
    #[cfg(feature = "smb-native")]
    #[test]
    fn test_smb_module_available() {
        // Just verify the module is accessible when feature is enabled
        use crate::smb::SmbSecurity;
        let _ = SmbSecurity::Opportunistic;
    }
}