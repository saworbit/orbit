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

pub mod audit;
pub mod cli_progress;
pub mod compression;
pub mod config;
pub mod core;
pub mod error;
pub mod instrumentation;
pub mod logging;
pub mod manifest_integration;
pub mod protocol;
pub mod stats;
pub mod system; // Phase 1: OrbitSystem implementations
pub mod telemetry;

// Native SMB protocol support (feature-gated)
#[cfg(feature = "smb-native")]
pub mod protocols;

// Unified backend abstraction (feature-gated)
#[cfg(feature = "backend-abstraction")]
pub mod backend;

// Manifest system modules (re-exported from workspace crates)
pub use orbit_core_audit as manifest_audit;
pub use orbit_core_manifest as manifest;
pub use orbit_core_starmap as starmap;

// Re-export commonly used types for convenience
pub use config::{
    ChunkingStrategy, CompressionType, CopyConfig, CopyMode, ErrorMode, LogLevel, SymlinkMode,
};
pub use core::zero_copy::{ZeroCopyCapabilities, ZeroCopyResult};
pub use core::{copy_directory, copy_file, copy_file_with_stats, CopyStats};
pub use core::{copy_directory_impl, copy_file_impl, copy_file_impl_with_stats}; // For testing with progress events
pub use error::{ErrorCategory, OrbitError, Result};
pub use instrumentation::{OperationStats, StatsSnapshot};
pub use protocol::Protocol;
pub use stats::TransferStats;

// Manifest system convenience exports
pub mod manifests {
    //! Manifest system for transfer planning, verification, and audit
    //!
    //! Provides Flight Plans, Cargo Manifests, Star Maps, and audit logging.

    pub use orbit_core_manifest::{
        validate_cargo_manifest, validate_flight_plan, CargoManifest, Chunking, Digests,
        Encryption, Endpoint, FileRef, FlightPlan, Policy, WindowMeta,
    };

    pub use orbit_core_starmap::{
        BloomFilter, ChunkMeta, RankSelectBitmap, StarMapBuilder, StarMapReader,
    };

    pub use orbit_core_audit::{Beacon, BeaconBuilder, EventType, TelemetryEvent, TelemetryLogger};

    // Re-export integration helpers
    pub use crate::manifest_integration::{should_generate_manifest, ManifestGenerator};
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
        client_for, Secret, SmbAuth, SmbCapability, SmbClient, SmbError, SmbMetadata, SmbSecurity,
        SmbTarget,
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
