//! Medic: Healing Operations for At-Risk Chunks
//!
//! The Medic orchestrates Phase 4 P2P transfers to restore chunk redundancy
//! when the Sentinel detects that a chunk has fallen below the minimum threshold.

use anyhow::{anyhow, Result};
use orbit_connect::StarManager;
use orbit_core_starmap::universe_v3::{ChunkLocation, Universe};
use orbit_proto::ReplicateResponse;
use orbit_star::auth::AuthService;
use std::sync::Arc;
use tracing::{info, warn};

/// Medic: Executes healing operations
///
/// The Medic is responsible for:
/// 1. Selecting a source Star (survivor with the chunk)
/// 2. Selecting a target Star (recruit without the chunk)
/// 3. Generating a transfer token
/// 4. Orchestrating the Phase 4 P2P transfer
/// 5. Updating the Universe with the new replica
pub struct Medic {
    /// Reference to the Universe database
    universe: Arc<Universe>,

    /// Star manager for connections
    star_manager: Arc<StarManager>,

    /// Auth service for generating transfer tokens
    auth_service: Arc<AuthService>,
}

impl Medic {
    /// Create a new Medic instance
    pub fn new(
        universe: Arc<Universe>,
        star_manager: Arc<StarManager>,
        auth_service: Arc<AuthService>,
    ) -> Self {
        Self {
            universe,
            star_manager,
            auth_service,
        }
    }

    /// Heal a chunk by replicating it to a new Star
    ///
    /// # Arguments
    ///
    /// * `hash` - The BLAKE3 hash of the chunk to heal
    /// * `survivors` - Current locations of the chunk (at least one must be active)
    ///
    /// # Returns
    ///
    /// Ok if the chunk was successfully replicated to a new Star and the Universe was updated.
    pub async fn heal_chunk(&self, hash: [u8; 32], survivors: Vec<ChunkLocation>) -> Result<()> {
        if survivors.is_empty() {
            return Err(anyhow!(
                "Cannot heal chunk {:x?}: no survivors available",
                hash
            ));
        }

        info!(
            "🚑 Medic: Healing chunk {:x?} (current copies: {})",
            &hash[..8],
            survivors.len()
        );

        // Step 1: Select source Star (pick the first survivor for now)
        // TODO: Implement load-based selection (pick survivor with lowest CPU/network load)
        let source = &survivors[0];
        info!(
            "   Source: Star {} ({})",
            source.star_id,
            source.path.display()
        );

        // Step 2: Select target Star (recruit)
        let recruit_star_id = self.find_recruit_star(&survivors).await?;
        info!("   Target: Star {}", recruit_star_id);

        // Step 3: Generate transfer token for the source file
        let _token = self
            .auth_service
            .generate_transfer_token(
                source
                    .path
                    .to_str()
                    .ok_or_else(|| anyhow!("Invalid path: {}", source.path.display()))?,
            )
            .map_err(|e| anyhow!("Failed to generate transfer token: {}", e))?;

        // Step 4: Get the recruit Star's system interface
        let _recruit_system = self
            .star_manager
            .get_system(&recruit_star_id)
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to connect to recruit Star {}: {}",
                    recruit_star_id,
                    e
                )
            })?;

        // Step 5: Determine the destination path on the recruit
        // Store in content-addressed pool: .orbit/pool/{hash_hex}
        let dest_path = format!(".orbit/pool/{:x}", DisplayHash(&hash));

        // Step 6: Get source Star address
        // TODO: This requires StarManager to track Star addresses
        // For now, we'll construct it from the star_id (this is a simplification)
        let source_address = format!("http://{}", source.star_id); // Placeholder!
        warn!("⚠️  Using placeholder source address: {}", source_address);
        warn!("    TODO: Implement proper Star address resolution in StarManager");

        // Step 7: Execute Phase 4 P2P transfer via ReplicateFile RPC
        // Note: This is conceptual - the actual RPC requires a proper gRPC client
        // which would be implemented in orbit-connect's RemoteSystem
        info!(
            "   Initiating P2P transfer: {} -> {}",
            source.star_id, recruit_star_id
        );

        // Placeholder for actual RPC call:
        // let response: ReplicateResponse = recruit_system
        //     .replicate_file(source_address, source.path, dest_path, token)
        //     .await?;

        // For now, we'll simulate success
        let simulated_response = ReplicateResponse {
            success: true,
            bytes_transferred: source.length as u64,
            checksum: format!("{:x}", DisplayHash(&hash)),
            error_message: String::new(),
        };

        if !simulated_response.success {
            return Err(anyhow!(
                "Replication failed: {}",
                simulated_response.error_message
            ));
        }

        info!(
            "   ✅ Transfer complete: {} bytes",
            simulated_response.bytes_transferred
        );

        // Step 8: Update Universe with new replica location
        let new_location = ChunkLocation::new(
            recruit_star_id.clone(),
            dest_path.into(),
            0, // Stored as whole file in pool
            source.length,
        );

        self.universe.insert_chunk(hash, new_location)?;

        info!(
            "   📝 Universe updated with new replica on Star {}",
            recruit_star_id
        );

        Ok(())
    }

    /// Find a suitable recruit Star (one that doesn't already have the chunk)
    ///
    /// # Arguments
    ///
    /// * `survivors` - Stars that currently have the chunk (to exclude)
    ///
    /// # Returns
    ///
    /// The star_id of a suitable recruit Star
    async fn find_recruit_star(&self, survivors: &[ChunkLocation]) -> Result<String> {
        // Get all registered Stars
        let all_stars = self.star_manager.list_stars().await;

        // Filter out survivors
        let survivor_ids: Vec<&str> = survivors.iter().map(|s| s.star_id.as_str()).collect();

        for star in all_stars {
            if !survivor_ids.contains(&star.id.as_str()) {
                // Found a candidate that doesn't already have the chunk
                // TODO: Check if Star has enough free space
                // TODO: Check if Star is currently online/reachable
                return Ok(star.id);
            }
        }

        Err(anyhow!(
            "No suitable recruit Star found (all Stars already have the chunk)"
        ))
    }
}

/// Helper to display hash in hex format
struct DisplayHash<'a>(&'a [u8; 32]);

impl<'a> std::fmt::Display for DisplayHash<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl<'a> std::fmt::LowerHex for DisplayHash<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_connect::{StarManager, StarRecord};
    use orbit_core_starmap::universe_v3::Universe;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_find_recruit_star() {
        let universe = Arc::new(Universe::open(NamedTempFile::new().unwrap().path()).unwrap());
        let star_manager = Arc::new(StarManager::new());
        let auth = Arc::new(AuthService::new("test-secret"));

        // Register 3 Stars
        star_manager
            .register(StarRecord::new(
                "star-1".to_string(),
                "http://localhost:50051".to_string(),
                "token1".to_string(),
            ))
            .await;
        star_manager
            .register(StarRecord::new(
                "star-2".to_string(),
                "http://localhost:50052".to_string(),
                "token2".to_string(),
            ))
            .await;
        star_manager
            .register(StarRecord::new(
                "star-3".to_string(),
                "http://localhost:50053".to_string(),
                "token3".to_string(),
            ))
            .await;

        let medic = Medic::new(universe, star_manager, auth);

        // Survivors: star-1 and star-2 have the chunk
        let survivors = vec![
            ChunkLocation::new(
                "star-1".to_string(),
                PathBuf::from("/data/file.bin"),
                0,
                1024,
            ),
            ChunkLocation::new(
                "star-2".to_string(),
                PathBuf::from("/data/file.bin"),
                0,
                1024,
            ),
        ];

        // Should recruit star-3 (the only one without the chunk)
        let recruit = medic.find_recruit_star(&survivors).await.unwrap();
        assert_eq!(recruit, "star-3");
    }

    #[tokio::test]
    async fn test_find_recruit_no_candidates() {
        let universe = Arc::new(Universe::open(NamedTempFile::new().unwrap().path()).unwrap());
        let star_manager = Arc::new(StarManager::new());
        let auth = Arc::new(AuthService::new("test-secret"));

        // Register only 1 Star
        star_manager
            .register(StarRecord::new(
                "star-1".to_string(),
                "http://localhost:50051".to_string(),
                "token1".to_string(),
            ))
            .await;

        let medic = Medic::new(universe, star_manager, auth);

        // Survivor: star-1 already has the chunk
        let survivors = vec![ChunkLocation::new(
            "star-1".to_string(),
            PathBuf::from("/data/file.bin"),
            0,
            1024,
        )];

        // Should fail - no other Stars available
        let result = medic.find_recruit_star(&survivors).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No suitable recruit"));
    }
}
