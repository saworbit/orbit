//! Neutrino Fast Lane - High-performance pipeline for small files
//!
//! The Neutrino module provides a bifurcated pipeline strategy that bypasses
//! the heavy machinery of CDC chunking and deduplication for small files.
//!
//! # Architecture
//!
//! Files are routed based on size threshold (default: 8KB):
//! - **Fast Lane**: Small files use DirectTransferExecutor with high concurrency
//! - **Standard Lane**: Large files use existing CDC/deduplication pipeline
//!
//! # Performance Benefits
//!
//! - **Reduced CPU Load**: Skips BLAKE3 hashing and Adler-32 rolling hashes
//! - **Reduced DB Bloat**: Avoids starmap index entries for non-deduplicable files
//! - **Network Saturation**: High concurrency (100-500 tasks) maximizes throughput
//!
//! # Example
//!
//! ```bash
//! orbit copy --profile neutrino --recursive /source /dest
//! ```

pub mod executor;
pub mod router;

pub use executor::{DirectTransferExecutor, ExecutorStats, SmallFileJob};
pub use router::{FileRouter, TransferLane};
