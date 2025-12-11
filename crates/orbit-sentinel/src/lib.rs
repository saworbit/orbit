//! Orbit Sentinel: Autonomous Resilience Engine (Phase 5)
//!
//! The Sentinel is the "Immune System" of the Orbit Grid. It continuously
//! monitors the Global Universe Map to ensure data durability by automatically
//! replicating chunks that have fallen below the configured redundancy threshold.
//!
//! # Architecture: The OODA Loop
//!
//! ```text
//! ┌─────────────┐
//! │  Observe    │──> Scan Universe V3 for all chunks
//! └──────┬──────┘
//!        │
//!        v
//! ┌─────────────┐
//! │  Orient     │──> Count active copies per chunk
//! └──────┬──────┘
//!        │
//!        v
//! ┌─────────────┐
//! │  Decide     │──> Healthy? At Risk? Lost?
//! └──────┬──────┘
//!        │
//!        v
//! ┌─────────────┐
//! │  Act        │──> Trigger healing via Phase 4 P2P
//! └──────┬──────┘
//!        │
//!        └────> Loop
//! ```
//!
//! # Example
//!
//! ```no_run
//! use orbit_sentinel::{Sentinel, SentinelPolicy};
//! use orbit_core_starmap::universe_v3::Universe;
//! use orbit_connect::StarManager;
//! use orbit_star::auth::AuthService;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Setup
//! let universe = Arc::new(Universe::open("universe_v3.db")?);
//! let auth_service = Arc::new(AuthService::new("shared-secret"));
//! let star_manager = Arc::new(StarManager::new());
//!
//! // Configure policy
//! let policy = SentinelPolicy {
//!     min_redundancy: 2,
//!     max_parallel_heals: 10,
//!     scan_interval_s: 3600,
//!     healing_bandwidth_limit: Some(50 * 1024 * 1024), // 50 MB/s
//! };
//!
//! // Start Sentinel
//! let sentinel = Sentinel::new(universe, auth_service, star_manager, policy);
//! sentinel.run().await;
//! # Ok(())
//! # }
//! ```

pub mod daemon;
pub mod medic;
pub mod metrics;
pub mod policy;

pub use daemon::Sentinel;
pub use metrics::SweepStats;
pub use policy::SentinelPolicy;
