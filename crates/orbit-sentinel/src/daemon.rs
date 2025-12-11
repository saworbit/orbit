//! Sentinel Daemon: The OODA Loop
//!
//! This module implements the main background loop that continuously monitors
//! the Universe and triggers healing operations when needed.

use crate::medic::Medic;
use crate::metrics::SweepStatsBuilder;
use crate::policy::SentinelPolicy;
use orbit_connect::StarManager;
use orbit_core_starmap::universe_v3::Universe;
use orbit_star::auth::AuthService;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// The Sentinel: Autonomous Resilience Engine
///
/// Runs an infinite OODA loop:
/// - **Observe:** Scan Universe V3 for all chunks
/// - **Orient:** Count active copies per chunk
/// - **Decide:** Determine which chunks need healing
/// - **Act:** Trigger Phase 4 P2P transfers
pub struct Sentinel {
    /// Reference to the Universe database
    universe: Arc<Universe>,

    /// Medic for executing healing operations
    medic: Arc<Medic>,

    /// Operational policy
    policy: SentinelPolicy,

    /// Semaphore to limit concurrent healing operations
    ///
    /// This prevents network storms by capping the number of
    /// simultaneous P2P transfers.
    repair_semaphore: Arc<Semaphore>,
}

impl Sentinel {
    /// Create a new Sentinel instance
    ///
    /// # Arguments
    ///
    /// * `universe` - Shared reference to the Universe V3 database
    /// * `auth_service` - Auth service for generating transfer tokens
    /// * `star_manager` - Star connection manager
    /// * `policy` - Operational policy (redundancy, scan interval, etc.)
    pub fn new(
        universe: Arc<Universe>,
        auth_service: Arc<AuthService>,
        star_manager: Arc<StarManager>,
        policy: SentinelPolicy,
    ) -> Self {
        // Validate policy
        if let Err(e) = policy.validate() {
            panic!("Invalid Sentinel policy: {}", e);
        }

        let medic = Arc::new(Medic::new(universe.clone(), star_manager, auth_service));

        let permits = policy.max_parallel_heals;

        Self {
            universe,
            medic,
            policy,
            repair_semaphore: Arc::new(Semaphore::new(permits)),
        }
    }

    /// Main event loop - runs forever
    ///
    /// This is typically spawned as a background tokio task:
    ///
    /// ```no_run
    /// # use orbit_sentinel::{Sentinel, SentinelPolicy};
    /// # use orbit_core_starmap::universe_v3::Universe;
    /// # use orbit_connect::StarManager;
    /// # use orbit_star::auth::AuthService;
    /// # use std::sync::Arc;
    /// # async fn example() -> anyhow::Result<()> {
    /// # let universe = Arc::new(Universe::open("universe.db")?);
    /// # let auth = Arc::new(AuthService::new("secret"));
    /// # let stars = Arc::new(StarManager::new());
    /// # let policy = SentinelPolicy::default();
    /// let sentinel = Sentinel::new(universe, auth, stars, policy);
    ///
    /// tokio::spawn(async move {
    ///     sentinel.run().await;
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(self) {
        info!(
            "🛡️  Sentinel Active | Min Redundancy: {} | Scan Interval: {}s | Max Parallel Heals: {}",
            self.policy.min_redundancy,
            self.policy.scan_interval_s,
            self.policy.max_parallel_heals
        );

        let mut interval = tokio::time::interval(Duration::from_secs(self.policy.scan_interval_s));

        loop {
            interval.tick().await;
            self.run_sweep().await;
        }
    }

    /// Execute a single Universe sweep
    ///
    /// This is the OODA loop iteration:
    /// 1. Scan all chunks in the Universe
    /// 2. Count active copies per chunk
    /// 3. Identify at-risk chunks
    /// 4. Spawn healing tasks for at-risk chunks
    ///
    /// Note: Public for integration testing
    pub async fn run_sweep(&self) {
        info!("🔭 Sentinel: Starting Universe Sweep...");
        let stats_builder = Arc::new(Mutex::new(SweepStatsBuilder::new()));

        // Observe: Scan all chunks from Universe V3
        let stats_clone = stats_builder.clone();
        let scan_result = self.universe.scan_all_chunks(move |hash, locations| {
            // Orient: Count active copies
            // TODO: Filter out offline/failed Stars
            // For now, we assume all Stars in the location list are active
            let active_count = locations.len();

            // Decide: Check redundancy status
            if active_count == 0 {
                // Red: Data Loss!
                stats_clone.lock().unwrap().record_lost();
                error!("💀 DATA LOSS: Chunk {:x?} has 0 active copies!", &hash[..8]);
            } else if active_count < self.policy.min_redundancy as usize {
                // Yellow: At Risk - Trigger Healing
                stats_clone.lock().unwrap().record_at_risk();
                warn!(
                    "⚠️  At Risk: Chunk {:x?} has {} copies (need {})",
                    &hash[..8],
                    active_count,
                    self.policy.min_redundancy
                );

                // Act: Spawn healing task (with semaphore to limit concurrency)
                let medic = self.medic.clone();
                let semaphore = self.repair_semaphore.clone();

                // Note: Healing stats are tracked separately to avoid complexity
                tokio::spawn(async move {
                    // Try to acquire a permit (non-blocking check)
                    if let Ok(permit) = semaphore.clone().try_acquire_owned() {
                        match medic.heal_chunk(hash, locations).await {
                            Ok(()) => {
                                info!("✅ Healing successful for chunk {:x?}", &hash[..8]);
                            }
                            Err(e) => {
                                error!("❌ Healing failed for chunk {:x?}: {}", &hash[..8], e);
                            }
                        }

                        drop(permit); // Release semaphore
                    } else {
                        warn!(
                            "⏸️  Healing skipped for {:x?}: max concurrent operations reached",
                            &hash[..8]
                        );
                    }
                });
            } else {
                // Green: Healthy
                stats_clone.lock().unwrap().record_healthy();
            }

            true // Continue scanning
        });

        if let Err(e) = scan_result {
            error!("❌ Universe scan failed: {}", e);
            return;
        }

        // Finalize stats
        let stats = stats_builder.lock().unwrap().clone().finish();

        // Report metrics
        info!("📊 {}", stats.summary());

        if stats.lost > 0 {
            error!(
                "🚨 CRITICAL: {} chunks have experienced DATA LOSS!",
                stats.lost
            );
        }

        if stats.at_risk > 0 {
            warn!(
                "⚠️  WARNING: {} chunks are at risk (below minimum redundancy)",
                stats.at_risk
            );
        }

        if stats.health_ratio() >= 0.99 {
            info!(
                "💚 Grid Health: Excellent ({:.1}%)",
                stats.health_ratio() * 100.0
            );
        } else if stats.health_ratio() >= 0.95 {
            info!(
                "💛 Grid Health: Good ({:.1}%)",
                stats.health_ratio() * 100.0
            );
        } else {
            warn!(
                "🔴 Grid Health: Poor ({:.1}%)",
                stats.health_ratio() * 100.0
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_connect::{StarManager, StarRecord};
    use orbit_core_starmap::universe_v3::ChunkLocation;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_sentinel_creation() {
        let universe = Arc::new(Universe::open(NamedTempFile::new().unwrap().path()).unwrap());
        let auth = Arc::new(AuthService::new("test-secret"));
        let stars = Arc::new(StarManager::new());
        let policy = SentinelPolicy::default();

        let sentinel = Sentinel::new(universe, auth, stars, policy);

        // Verify policy was applied
        assert_eq!(sentinel.policy.min_redundancy, 2);
        assert_eq!(sentinel.policy.max_parallel_heals, 10);
    }

    #[tokio::test]
    async fn test_sentinel_with_healthy_chunks() {
        let tmp_file = NamedTempFile::new().unwrap();
        let universe = Arc::new(Universe::open(tmp_file.path()).unwrap());
        let auth = Arc::new(AuthService::new("test-secret"));
        let stars = Arc::new(StarManager::new());

        // Register Stars
        stars
            .register(StarRecord::new(
                "star-1".to_string(),
                "http://localhost:50051".to_string(),
                "token1".to_string(),
            ))
            .await;
        stars
            .register(StarRecord::new(
                "star-2".to_string(),
                "http://localhost:50052".to_string(),
                "token2".to_string(),
            ))
            .await;

        // Insert a healthy chunk (2 copies = meets min_redundancy of 2)
        let hash = [0x42; 32];
        universe
            .insert_chunk(
                hash,
                ChunkLocation::new(
                    "star-1".to_string(),
                    PathBuf::from("/data/file.bin"),
                    0,
                    1024,
                ),
            )
            .unwrap();
        universe
            .insert_chunk(
                hash,
                ChunkLocation::new(
                    "star-2".to_string(),
                    PathBuf::from("/data/file.bin"),
                    0,
                    1024,
                ),
            )
            .unwrap();

        let policy = SentinelPolicy::default();
        let sentinel = Sentinel::new(universe, auth, stars, policy);

        // Run a single sweep (don't loop forever for the test)
        sentinel.run_sweep().await;

        // If we get here without panic, the sweep completed successfully
        // In a real test, we'd verify the stats were recorded correctly
    }

    #[tokio::test]
    #[should_panic(expected = "Invalid Sentinel policy")]
    async fn test_sentinel_invalid_policy() {
        let universe = Arc::new(Universe::open(NamedTempFile::new().unwrap().path()).unwrap());
        let auth = Arc::new(AuthService::new("test-secret"));
        let stars = Arc::new(StarManager::new());

        // Create invalid policy
        let mut policy = SentinelPolicy::default();
        policy.min_redundancy = 0; // Invalid!

        // Should panic
        Sentinel::new(universe, auth, stars, policy);
    }
}
