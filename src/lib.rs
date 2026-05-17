/*! Orbit - High-performance file transfer library */

pub mod audit;
pub mod cli_progress;
pub mod cli_style;
pub mod commands;
pub mod compression;
pub mod config;
pub mod core;
pub mod error;
pub mod instrumentation;
pub mod logging;
pub mod manifest_integration;
pub mod output;
pub mod protocol;
pub mod stats;
#[cfg(feature = "orbit-system")]
pub mod system;
pub mod telemetry;

#[cfg(feature = "smb-native")]
pub mod protocols;

#[cfg(feature = "backend-abstraction")]
pub mod backend;

// Workspace crate re-exports
pub use orbit_core_audit as manifest_audit;
pub use orbit_core_manifest as manifest;
pub use orbit_core_starmap as starmap;

// Commonly used types
pub use config::{
    ChunkingStrategy, CompressionType, CopyConfig, CopyMode, ErrorMode, LogLevel, SymlinkMode,
};
pub use core::zero_copy::{ZeroCopyCapabilities, ZeroCopyResult};
pub use core::{copy_directory, copy_file, copy_file_with_stats, CopyStats};
pub use core::{copy_directory_impl, copy_file_impl, copy_file_impl_with_stats};
pub use error::{ErrorCategory, OrbitError, Result};
pub use instrumentation::{OperationStats, StatsSnapshot};
pub use protocol::Protocol;
pub use stats::TransferStats;

/// Manifest system re-exports
pub mod manifests {
    pub use orbit_core_manifest::{
        validate_cargo_manifest, validate_flight_plan, CargoManifest, Chunking, Digests,
        Encryption, Endpoint, FileRef, FlightPlan, Policy, WindowMeta,
    };

    pub use orbit_core_starmap::{
        BloomFilter, ChunkMeta, RankSelectBitmap, StarMapBuilder, StarMapReader,
    };

    pub use orbit_core_audit::{Beacon, BeaconBuilder, EventType, TelemetryEvent, TelemetryLogger};

    pub use crate::manifest_integration::{should_generate_manifest, ManifestGenerator};
}

#[cfg(feature = "smb-native")]
pub mod smb {
    pub use crate::protocols::smb::{
        client_for, Secret, SmbAuth, SmbCapability, SmbClient, SmbError, SmbMetadata, SmbSecurity,
        SmbTarget,
    };
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn is_zero_copy_available() -> bool {
    ZeroCopyCapabilities::detect().available
}

pub fn get_zero_copy_capabilities() -> ZeroCopyCapabilities {
    ZeroCopyCapabilities::detect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_zero_copy_detection() {
        let available = is_zero_copy_available();
        let caps = get_zero_copy_capabilities();
        assert_eq!(available, caps.available);
    }
}
