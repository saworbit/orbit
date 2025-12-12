//! StarManager: Connection pool and session management for remote Stars

use crate::error::ConnectError;
use crate::system::RemoteSystem;
use orbit_core_interface::OrbitSystem;
use orbit_proto::{star_service_client::StarServiceClient, HandshakeRequest};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, info};

/// Status of a Star connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarStatus {
    /// Star is registered but not yet connected
    Registered,
    /// Star is connected and session is active
    Connected,
    /// Connection attempt failed
    Failed,
    /// Star is disconnected (session expired or network error)
    Disconnected,
}

/// Persistent record of a Star (typically stored in database)
#[derive(Debug, Clone)]
pub struct StarRecord {
    /// Unique identifier for this Star
    pub id: String,

    /// Network address of the Star (e.g., "http://10.0.0.5:50051")
    pub address: String,

    /// Authentication token for this Star (generated during registration)
    pub token: String,

    /// Display name for this Star (optional)
    pub name: Option<String>,

    /// Current connection status
    pub status: StarStatus,
}

impl StarRecord {
    /// Create a new Star record
    pub fn new(id: String, address: String, token: String) -> Self {
        Self {
            id,
            address,
            token,
            name: None,
            status: StarStatus::Registered,
        }
    }

    /// Create a new Star record with a display name
    pub fn with_name(id: String, address: String, token: String, name: String) -> Self {
        Self {
            id,
            address,
            token,
            name: Some(name),
            status: StarStatus::Registered,
        }
    }
}

/// Central manager for Star connections in the Nucleus
///
/// Responsibilities:
/// - Maintain a registry of known Stars
/// - Establish and cache gRPC connections
/// - Perform handshakes and manage sessions
/// - Provide `OrbitSystem` instances for each Star
///
/// # Example
///
/// ```rust,no_run
/// use orbit_connect::{StarManager, StarRecord};
/// use orbit_core_interface::OrbitSystem;
///
/// # async fn example() -> anyhow::Result<()> {
/// let mut manager = StarManager::new();
///
/// // Register a Star
/// let star = StarRecord::new(
///     "star-1".to_string(),
///     "http://10.0.0.5:50051".to_string(),
///     "secret-token-123".to_string(),
/// );
/// manager.register(star).await;
///
/// // Get a system for this Star (automatically connects)
/// let system = manager.get_system("star-1").await?;
///
/// // Use it like any OrbitSystem
/// let exists = system.exists(std::path::Path::new("/data/file.bin")).await;
/// # Ok(())
/// # }
/// ```
pub struct StarManager {
    /// Registry of known Stars (ID -> Record)
    registry: Arc<RwLock<HashMap<String, StarRecord>>>,

    /// Active connections (ID -> RemoteSystem)
    connections: Arc<RwLock<HashMap<String, Arc<RemoteSystem>>>>,
}

impl StarManager {
    /// Create a new empty StarManager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new Star in the registry
    ///
    /// This does not establish a connection immediately. Connections are lazy
    /// and established when `get_system()` is first called.
    pub async fn register(&self, star: StarRecord) {
        let star_id = star.id.clone();
        info!("Registering Star: {} ({})", star_id, star.address);

        let mut registry = self.registry.write().await;
        registry.insert(star_id, star);
    }

    /// Remove a Star from the registry and close its connection
    pub async fn unregister(&self, star_id: &str) -> Result<(), ConnectError> {
        info!("Unregistering Star: {}", star_id);

        // Remove from registry
        let mut registry = self.registry.write().await;
        registry
            .remove(star_id)
            .ok_or_else(|| ConnectError::StarNotFound(star_id.to_string()))?;

        // Close connection
        let mut connections = self.connections.write().await;
        connections.remove(star_id);

        Ok(())
    }

    /// Get an OrbitSystem for a specific Star
    ///
    /// This will:
    /// 1. Check if there's an active connection
    /// 2. If not, establish a new connection and perform handshake
    /// 3. Return an Arc'd RemoteSystem that can be cheaply cloned
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Star ID is not found in registry
    /// - Network connection fails
    /// - Handshake is rejected
    pub async fn get_system(&self, star_id: &str) -> Result<Arc<dyn OrbitSystem>, ConnectError> {
        // 1. Check if already connected
        {
            let connections = self.connections.read().await;
            if let Some(system) = connections.get(star_id) {
                debug!("Reusing existing connection to Star: {}", star_id);
                return Ok(system.clone() as Arc<dyn OrbitSystem>);
            }
        }

        // 2. Not connected - establish new connection
        info!("Establishing new connection to Star: {}", star_id);
        self.connect(star_id).await
    }

    /// Internal method to establish a new connection
    async fn connect(&self, star_id: &str) -> Result<Arc<dyn OrbitSystem>, ConnectError> {
        // Fetch Star record from registry
        let record = {
            let registry = self.registry.read().await;
            registry
                .get(star_id)
                .cloned()
                .ok_or_else(|| ConnectError::StarNotFound(star_id.to_string()))?
        };

        debug!("Connecting to Star {} at {}", record.id, record.address);

        // Establish gRPC channel
        let endpoint = Endpoint::from_shared(record.address.clone()).map_err(|e| {
            ConnectError::ConnectionFailed {
                star_id: star_id.to_string(),
                reason: format!("Invalid endpoint: {}", e),
            }
        })?;

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| ConnectError::ConnectionFailed {
                star_id: star_id.to_string(),
                reason: format!("Connection failed: {}", e),
            })?;

        // Perform handshake
        let session_id = self.handshake(&channel, &record).await?;

        // Create RemoteSystem
        let system = Arc::new(RemoteSystem::new(channel, session_id));

        // Cache the connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(star_id.to_string(), system.clone());
        }

        // Update status in registry
        {
            let mut registry = self.registry.write().await;
            if let Some(star) = registry.get_mut(star_id) {
                star.status = StarStatus::Connected;
            }
        }

        info!("Successfully connected to Star: {}", star_id);

        Ok(system as Arc<dyn OrbitSystem>)
    }

    /// Perform handshake with a Star
    async fn handshake(
        &self,
        channel: &Channel,
        record: &StarRecord,
    ) -> Result<String, ConnectError> {
        let mut client = StarServiceClient::new(channel.clone());

        let request = HandshakeRequest {
            star_token: record.token.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec![], // TODO: Add capability detection in future phase
        };

        debug!("Sending handshake to Star {}", record.id);

        let response = client
            .handshake(request)
            .await
            .map_err(|e| ConnectError::HandshakeFailed(format!("gRPC error: {}", e)))?;

        let handshake_response = response.into_inner();

        if !handshake_response.accepted {
            return Err(ConnectError::HandshakeFailed(
                "Star rejected handshake".to_string(),
            ));
        }

        info!(
            "Handshake successful with Star {} (session: {})",
            record.id, handshake_response.session_id
        );

        Ok(handshake_response.session_id)
    }

    /// Get all registered Stars
    pub async fn list_stars(&self) -> Vec<StarRecord> {
        let registry = self.registry.read().await;
        registry.values().cloned().collect()
    }

    /// Check if a Star is currently connected
    pub async fn is_connected(&self, star_id: &str) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(star_id)
    }

    /// Disconnect from a Star (but keep it registered)
    ///
    /// This is useful for graceful shutdown or when you want to force
    /// a reconnection on the next request.
    pub async fn disconnect(&self, star_id: &str) {
        info!("Disconnecting from Star: {}", star_id);

        let mut connections = self.connections.write().await;
        connections.remove(star_id);

        let mut registry = self.registry.write().await;
        if let Some(star) = registry.get_mut(star_id) {
            star.status = StarStatus::Disconnected;
        }
    }

    /// Disconnect from all Stars
    pub async fn disconnect_all(&self) {
        info!("Disconnecting from all Stars");

        let mut connections = self.connections.write().await;
        connections.clear();

        let mut registry = self.registry.write().await;
        for star in registry.values_mut() {
            star.status = StarStatus::Disconnected;
        }
    }
}

impl Default for StarManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_list_stars() {
        let manager = StarManager::new();

        let star1 = StarRecord::new(
            "star-1".to_string(),
            "http://localhost:50051".to_string(),
            "token1".to_string(),
        );

        let star2 = StarRecord::new(
            "star-2".to_string(),
            "http://localhost:50052".to_string(),
            "token2".to_string(),
        );

        manager.register(star1).await;
        manager.register(star2).await;

        let stars = manager.list_stars().await;
        assert_eq!(stars.len(), 2);
    }

    #[tokio::test]
    async fn test_unregister_star() {
        let manager = StarManager::new();

        let star = StarRecord::new(
            "star-1".to_string(),
            "http://localhost:50051".to_string(),
            "token1".to_string(),
        );

        manager.register(star).await;
        assert_eq!(manager.list_stars().await.len(), 1);

        manager.unregister("star-1").await.unwrap();
        assert_eq!(manager.list_stars().await.len(), 0);
    }

    #[tokio::test]
    async fn test_get_system_for_unknown_star() {
        let manager = StarManager::new();

        let result = manager.get_system("unknown").await;
        assert!(matches!(result, Err(ConnectError::StarNotFound(_))));
    }
}
