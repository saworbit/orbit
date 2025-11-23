//! Global application state for Nebula web interface

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
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

    /// Pipeline configurations (DAG workflows)
    pub pipelines: Arc<RwLock<HashMap<String, Pipeline>>>,
}

impl AppState {
    /// Create new application state
    pub async fn new(
        magnetar_db_path: &str,
        user_db_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send>> {
        // Initialize Magnetar database pool (create if doesn't exist)
        let magnetar_pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", magnetar_db_path))
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        // Initialize user database pool (create if doesn't exist)
        let user_pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", user_db_path))
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        // Initialize user database schema
        crate::auth::init_user_db(&user_pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        // Create default admin user if needed
        crate::auth::ensure_default_admin(&user_pool)
            .await
            .map_err(|e| {
                Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send>
            })?;

        // Initialize jobs table in magnetar database (INTEGER for timestamps as Unix epoch)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                destination TEXT NOT NULL,
                compress BOOLEAN NOT NULL DEFAULT 0,
                verify BOOLEAN NOT NULL DEFAULT 0,
                parallel INTEGER,
                status TEXT NOT NULL DEFAULT 'pending',
                progress REAL NOT NULL DEFAULT 0.0,
                total_chunks INTEGER NOT NULL DEFAULT 0,
                completed_chunks INTEGER NOT NULL DEFAULT 0,
                failed_chunks INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&magnetar_pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        tracing::info!("Jobs table initialized");

        // Initialize pipelines table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pipelines (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                nodes_json TEXT NOT NULL DEFAULT '[]',
                edges_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'draft',
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&magnetar_pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        tracing::info!("Pipelines table initialized");

        // Create broadcast channel for events (capacity: 1000 events)
        let (event_tx, _) = broadcast::channel(1000);

        // Initialize backends storage
        let backends = Arc::new(RwLock::new(HashMap::new()));

        // Load pipelines from database
        let pipelines = Arc::new(RwLock::new(HashMap::new()));
        let rows = sqlx::query(
            "SELECT id, name, description, nodes_json, edges_json, status, created_at, updated_at FROM pipelines"
        )
        .fetch_all(&magnetar_pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        {
            let mut pipelines_guard = pipelines.write().await;
            for row in rows {
                let id: String = row.get(0);
                let name: String = row.get(1);
                let description: String = row.get(2);
                let nodes_json: String = row.get(3);
                let edges_json: String = row.get(4);
                let status_str: String = row.get(5);
                let created_at: i64 = row.get(6);
                let updated_at: i64 = row.get(7);

                let nodes: Vec<PipelineNode> =
                    serde_json::from_str(&nodes_json).unwrap_or_default();
                let edges: Vec<PipelineEdge> =
                    serde_json::from_str(&edges_json).unwrap_or_default();
                let status = match status_str.as_str() {
                    "ready" => PipelineStatus::Ready,
                    "running" => PipelineStatus::Running,
                    "completed" => PipelineStatus::Completed,
                    "failed" => PipelineStatus::Failed,
                    "paused" => PipelineStatus::Paused,
                    _ => PipelineStatus::Draft,
                };

                pipelines_guard.insert(
                    id.clone(),
                    Pipeline {
                        id,
                        name,
                        description,
                        nodes,
                        edges,
                        status,
                        created_at,
                        updated_at,
                    },
                );
            }
            tracing::info!("Loaded {} pipelines from database", pipelines_guard.len());
        }

        Ok(AppState {
            magnetar_pool,
            user_pool,
            event_tx,
            backends,
            pipelines,
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

// =============================================================================
// PIPELINE DATA STRUCTURES (DAG-based workflow)
// =============================================================================

/// A pipeline is a DAG of transfer operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<PipelineNode>,
    pub edges: Vec<PipelineEdge>,
    pub status: PipelineStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Pipeline {
    /// Create a new empty pipeline
    pub fn new(name: String, description: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Pipeline {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            nodes: Vec::new(),
            edges: Vec::new(),
            status: PipelineStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a node to the pipeline
    pub fn add_node(&mut self, node: PipelineNode) {
        self.nodes.push(node);
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// Add an edge (connection) between nodes
    pub fn add_edge(&mut self, edge: PipelineEdge) -> Result<(), String> {
        // Validate source and target nodes exist
        let source_exists = self.nodes.iter().any(|n| n.id == edge.source_node_id);
        let target_exists = self.nodes.iter().any(|n| n.id == edge.target_node_id);

        if !source_exists {
            return Err(format!("Source node {} not found", edge.source_node_id));
        }
        if !target_exists {
            return Err(format!("Target node {} not found", edge.target_node_id));
        }

        // Check for cycles (simple check: no self-loops)
        if edge.source_node_id == edge.target_node_id {
            return Err("Self-loops are not allowed".to_string());
        }

        self.edges.push(edge);
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Remove a node and all connected edges
    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.retain(|n| n.id != node_id);
        self.edges
            .retain(|e| e.source_node_id != node_id && e.target_node_id != node_id);
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// Remove an edge
    pub fn remove_edge(&mut self, edge_id: &str) {
        self.edges.retain(|e| e.id != edge_id);
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

/// Pipeline execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    Draft,     // Being edited, not ready to run
    Ready,     // Validated and ready to execute
    Running,   // Currently executing
    Completed, // Successfully finished
    Failed,    // Execution failed
    Paused,    // Execution paused
}

impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineStatus::Draft => write!(f, "draft"),
            PipelineStatus::Ready => write!(f, "ready"),
            PipelineStatus::Running => write!(f, "running"),
            PipelineStatus::Completed => write!(f, "completed"),
            PipelineStatus::Failed => write!(f, "failed"),
            PipelineStatus::Paused => write!(f, "paused"),
        }
    }
}

/// A node in the pipeline DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineNode {
    pub id: String,
    pub node_type: PipelineNodeType,
    pub name: String,
    pub position: NodePosition,
    pub config: NodeConfig,
}

impl PipelineNode {
    /// Create a new pipeline node
    pub fn new(node_type: PipelineNodeType, name: String, x: f64, y: f64) -> Self {
        PipelineNode {
            id: Uuid::new_v4().to_string(),
            node_type,
            name,
            position: NodePosition { x, y },
            config: NodeConfig::default(),
        }
    }
}

/// Position of a node on the canvas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

/// Types of nodes available in the pipeline editor
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineNodeType {
    /// Source node - where data comes from
    Source,
    /// Destination node - where data goes to
    Destination,
    /// Transfer node - copy data from source to destination
    Transfer,
    /// Transform node - modify data in transit (compression, encryption)
    Transform,
    /// Filter node - filter files by pattern
    Filter,
    /// Merge node - combine multiple inputs
    Merge,
    /// Split node - split output to multiple destinations
    Split,
    /// Conditional node - branch based on conditions
    Conditional,
}

impl std::fmt::Display for PipelineNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineNodeType::Source => write!(f, "source"),
            PipelineNodeType::Destination => write!(f, "destination"),
            PipelineNodeType::Transfer => write!(f, "transfer"),
            PipelineNodeType::Transform => write!(f, "transform"),
            PipelineNodeType::Filter => write!(f, "filter"),
            PipelineNodeType::Merge => write!(f, "merge"),
            PipelineNodeType::Split => write!(f, "split"),
            PipelineNodeType::Conditional => write!(f, "conditional"),
        }
    }
}

/// Configuration for a pipeline node
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeConfig {
    /// Path for source/destination nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Backend ID for source/destination nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_id: Option<String>,
    /// File pattern for filter nodes (glob)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Enable compression for transform nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,
    /// Enable encryption for transform nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypt: Option<bool>,
    /// Enable verification for transfer nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify: Option<bool>,
    /// Number of parallel workers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_workers: Option<u32>,
    /// Condition expression for conditional nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// An edge connecting two nodes in the DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEdge {
    pub id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    /// Output port name on source (default: "out")
    pub source_port: String,
    /// Input port name on target (default: "in")
    pub target_port: String,
}

impl PipelineEdge {
    /// Create a new edge with default ports
    pub fn new(source_node_id: String, target_node_id: String) -> Self {
        PipelineEdge {
            id: Uuid::new_v4().to_string(),
            source_node_id,
            target_node_id,
            source_port: "out".to_string(),
            target_port: "in".to_string(),
        }
    }

    /// Create an edge with specific ports
    pub fn with_ports(
        source_node_id: String,
        target_node_id: String,
        source_port: String,
        target_port: String,
    ) -> Self {
        PipelineEdge {
            id: Uuid::new_v4().to_string(),
            source_node_id,
            target_node_id,
            source_port,
            target_port,
        }
    }
}
