//! Core manifest data structures for Orbit
//!
//! This crate provides the control plane for Orbit transfers through manifest files.
//! Manifests enable planning, verification, resume, audit, and policy enforcement.
//!
//! # Key Concepts
//!
//! - **Flight Plan**: Job-level plan containing transfer metadata, policy, and file references
//! - **Cargo Manifest**: Per-file manifest with chunking details and integrity information
//! - **Content ID**: BLAKE3 hash used to identify chunks uniquely
//!
//! # Example
//!
//! ```no_run
//! use orbit_core_manifest::{FlightPlan, Endpoint, Policy, Encryption};
//! use std::path::PathBuf;
//!
//! let flight_plan = FlightPlan::new(
//!     Endpoint::filesystem("/data/source"),
//!     Endpoint::filesystem("/data/target"),
//!     Policy::default_with_encryption(Encryption::aes256_gcm("env:ORBIT_KEY")),
//! );
//! ```

pub mod error;
pub mod flightplan;
pub mod cargo;
pub mod validate;

// Re-export main types for convenience
pub use error::{Error, Result};
pub use flightplan::{
    FlightPlan, Endpoint, Policy, Encryption, CapacityVector, 
    Eta, FileRef, EndpointType,
};
pub use cargo::{
    CargoManifest, Chunking, ChunkingType, Digests, WindowMeta,
};
pub use validate::{validate_flight_plan, validate_cargo_manifest};

/// Schema version for Flight Plan
pub const FLIGHT_PLAN_SCHEMA_VERSION: &str = "orbit.flightplan.v1";

/// Schema version for Cargo Manifest
pub const CARGO_MANIFEST_SCHEMA_VERSION: &str = "orbit.cargo.v1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_versions() {
        assert_eq!(FLIGHT_PLAN_SCHEMA_VERSION, "orbit.flightplan.v1");
        assert_eq!(CARGO_MANIFEST_SCHEMA_VERSION, "orbit.cargo.v1");
    }
}