//! Global application state for Nebula web interface

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Global application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// Magnetar job database for persistent job state
    pub magnetar_pool: SqlitePool,

    /// User authentication database
    pub user_pool: SqlitePool,

    /// Broadcast channel for real-time events
    pub event_tx: broadcast::Sender<OrbitEvent>,

    /// Backend configurations (S3, SMB credentials, etc.)
    pub backends: Arc<RwLock<HashMap<String, BackendConfig>>>,
}

impl AppState {
    /// Create new application state
    pub async fn new(
        magnetar_db_path: &str,
        user_db_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize Magnetar database pool
        let magnetar_pool = SqlitePool::connect(&format!("sqlite:{}", magnetar_db_path)).await?;

        // Initialize user database pool
        let user_pool = SqlitePool::connect(&format!("sqlite:{}", user_db_path)).await?;

        // Initialize user database schema
        crate::auth::init_user_db(&user_pool).await?;

        // Create default admin user if needed
        crate::auth::ensure_default_admin(&user_pool).await?;

        // Create broadcast channel for events (capacity: 1000 events)
        let (event_tx, _) = broadcast::channel(1000);

        // Initialize backends storage
        let backends = Arc::new(RwLock::new(HashMap::new()));

        Ok(AppState {
            magnetar_pool,
            user_pool,
            event_tx,
            backends,
        })
    }

    /// Subscribe to real-time events
    pub fn subscribe_events(&self) -> broadcast::Receiver<OrbitEvent> {
        self.event_tx.subscribe()
    }

    /// Emit event to all subscribers
    pub fn emit_event(&self, event: OrbitEvent) {
        // Ignore send errors (no subscribers is OK)
        let _ = self.event_tx.send(event);
    }
}

/// Real-time events broadcast to WebSocket clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrbitEvent {
    /// Job status updated
    JobUpdated {
        job_id: String,
        status: String,
        progress: f32,
        timestamp: i64,
    },

    /// Transfer speed update
    TransferSpeed {
        job_id: String,
        bytes_per_sec: u64,
        timestamp: i64,
    },

    /// Job completed successfully
    JobCompleted {
        job_id: String,
        total_bytes: u64,
        duration_ms: u64,
        timestamp: i64,
    },

    /// Job failed with error
    JobFailed {
        job_id: String,
        error: String,
        timestamp: i64,
    },

    /// Anomaly detected (unexpected speed drop, errors, etc.)
    AnomalyDetected {
        job_id: String,
        message: String,
        severity: String,
        timestamp: i64,
    },

    /// Chunk completed
    ChunkCompleted {
        job_id: String,
        chunk_id: u64,
        bytes: u64,
        timestamp: i64,
    },
}

impl OrbitEvent {
    /// Get the job ID associated with this event
    pub fn job_id(&self) -> &str {
        match self {
            OrbitEvent::JobUpdated { job_id, .. }
            | OrbitEvent::TransferSpeed { job_id, .. }
            | OrbitEvent::JobCompleted { job_id, .. }
            | OrbitEvent::JobFailed { job_id, .. }
            | OrbitEvent::AnomalyDetected { job_id, .. }
            | OrbitEvent::ChunkCompleted { job_id, .. } => job_id,
        }
    }

    /// Get the minimum role required to view this event
    pub fn required_role(&self) -> crate::auth::Role {
        // All events require at least Viewer role
        crate::auth::Role::Viewer
    }

    /// Create current timestamp
    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }

    /// Create a job updated event
    pub fn job_updated(job_id: String, status: String, progress: f32) -> Self {
        OrbitEvent::JobUpdated {
            job_id,
            status,
            progress,
            timestamp: Self::now(),
        }
    }

    /// Create a transfer speed event
    pub fn transfer_speed(job_id: String, bytes_per_sec: u64) -> Self {
        OrbitEvent::TransferSpeed {
            job_id,
            bytes_per_sec,
            timestamp: Self::now(),
        }
    }

    /// Create a job completed event
    pub fn job_completed(job_id: String, total_bytes: u64, duration_ms: u64) -> Self {
        OrbitEvent::JobCompleted {
            job_id,
            total_bytes,
            duration_ms,
            timestamp: Self::now(),
        }
    }

    /// Create a job failed event
    pub fn job_failed(job_id: String, error: String) -> Self {
        OrbitEvent::JobFailed {
            job_id,
            error,
            timestamp: Self::now(),
        }
    }
}

/// Backend configuration (S3, SMB, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub id: String,
    pub name: String,
    pub backend_type: BackendType,
    #[serde(skip_serializing)]
    pub credentials: BackendCredentials,
    pub created_at: i64,
}

/// Backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackendType {
    Local { root_path: String },
    S3 { bucket: String, region: String },
    SMB { host: String, share: String },
}

/// Backend credentials (encrypted in production)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendCredentials {
    None,
    S3 {
        access_key: String,
        secret_key: String,
    },
    SMB {
        username: String,
        password: String,
    },
}

impl BackendConfig {
    /// Create new backend configuration
    pub fn new(name: String, backend_type: BackendType, credentials: BackendCredentials) -> Self {
        BackendConfig {
            id: Uuid::new_v4().to_string(),
            name,
            backend_type,
            credentials,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}
