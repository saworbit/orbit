//! Pipeline Routing: The "Sieve" Pattern
//!
//! This module implements the routing logic that directs files to the appropriate
//! execution lane based on their characteristics (size, type, etc.).
//!
//! # The Three-Lane Architecture
//!
//! - **Neutrino** (<8KB): Direct transfer, no CDC
//! - **Equilibrium** (8KB-1GB): Standard CDC with 64KB chunks
//! - **Gigantor** (>1GB): Tiered CDC with 1-4MB chunks
//!
//! # Example
//!
//! ```no_run
//! use magnetar::pipeline::PipelineRouter;
//!
//! let file_size = 5_000_000_000; // 5GB
//! let strategy = PipelineRouter::select_strategy(file_size);
//! let config = PipelineRouter::optimal_chunk_config(file_size);
//! ```

pub mod router;

pub use router::{PipelineRouter, TransferStrategy};
