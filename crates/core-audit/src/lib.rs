//! Audit logging and telemetry for Orbit manifests
//!
//! This crate provides append-only JSON Lines logging for transfer jobs,
//! enabling compliance tracking, forensics, and performance analysis.
//!
//! # Key Concepts
//!
//! - **Telemetry Log**: Append-only JSON Lines stream of transfer events
//! - **Beacon**: Final job summary with optional cryptographic signature
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │      Transfer Operation             │
//! └──────────────┬──────────────────────┘
//!                │
//!                │ emit events
//!                ▼
//! ┌─────────────────────────────────────┐
//! │       TelemetryLogger               │
//! │  - job_start                        │
//! │  - file_start                       │
//! │  - window_ok                        │
//! │  - window_fail                      │
//! │  - job_complete                     │
//! └──────────────┬──────────────────────┘
//!                │
//!                │ write JSON Lines
//!                ▼
//! ┌─────────────────────────────────────┐
//! │      audit.jsonl                    │
//! │ {"ts":"...","event":"job_start"}    │
//! │ {"ts":"...","event":"window_ok"}    │
//! │ {"ts":"...","event":"job_complete"} │
//! └─────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use orbit_core_audit::TelemetryLogger;
//! use std::path::Path;
//!
//! let mut logger = TelemetryLogger::new(Path::new("audit.jsonl")).unwrap();
//!
//! logger.log_job_start("job-123", 3, 1024000).unwrap();
//! logger.log_window_ok("job-123", "file.bin", 0, 16777216, 0).unwrap();
//! logger.log_job_complete("job-123", "sha256:abc123", 3, 1024000).unwrap();
//! ```

pub mod beacon;
pub mod error;
pub mod provenance;
pub mod telemetry;

// Re-export main types
pub use beacon::{Beacon, BeaconBuilder};
pub use error::{Error, Result};
pub use provenance::{ProvenanceEvent, ProvenanceType};
pub use telemetry::{EventType, TelemetryEvent, TelemetryLogger};

#[cfg(test)]
mod tests {
    #[test]
    fn test_audit_module_exists() {
        // Basic smoke test that the module compiles
        // Module compiles successfully if this test runs
    }
}
