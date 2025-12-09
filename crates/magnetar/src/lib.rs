//! Magnetar: Idempotent State Machine for Crash-Proof, Persistent Jobs
//!
//! # Overview
//!
//! Magnetar is a lightweight, embeddable state machine for managing idempotent jobs
//! with persistent storage. It transforms ephemeral manifests (e.g., TOML/JSON) into
//! durable DB schemas, enabling crash-proof resumption, atomic chunk claims, and
//! DAG-based dependency graphs.
//!
//! # Features
//!
//! - **Idempotent Claims**: Atomic "pending → processing" transitions
//! - **Job Resumption**: Query pending chunks post-crash
//! - **DAG Dependencies**: Support for dependency graphs
//! - **Manifest Migration**: TOML/JSON → DB bulk upsert
//! - **Multiple Backends**: SQLite (default) and redb (pure Rust)
//!
//! # Example
//!
//! ```no_run
//! use magnetar::{JobStore, JobStatus};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut store = magnetar::open("jobs.db").await?;
//!
//!     // Initialize from manifest
//!     let manifest = toml::from_str(r#"
//!         [[chunks]]
//!         id = 1
//!         checksum = "abc123"
//!
//!         [[chunks]]
//!         id = 2
//!         checksum = "def456"
//!     "#)?;
//!
//!     store.init_from_manifest(42, &manifest).await?;
//!
//!     // Process chunks
//!     while let Some(state) = store.claim_pending(42).await? {
//!         // Do work...
//!         store.mark_status(42, state.chunk, JobStatus::Done, None).await?;
//!     }
//!
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod backends;
pub mod config;
pub mod executor;
pub mod manager;
pub mod migration;
pub mod pipeline;

#[cfg(feature = "resilience")]
pub mod resilience;

#[cfg(feature = "sqlite")]
pub use backends::sqlite::SqliteStore;

#[cfg(feature = "redb")]
pub use backends::redb::RedbStore;

// Export manager types
pub use manager::{JobManager, ManagerConfig};

/// Job execution status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Chunk is waiting to be processed
    Pending,
    /// Chunk is currently being processed
    Processing,
    /// Chunk completed successfully
    Done,
    /// Chunk processing failed
    Failed,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Processing => write!(f, "processing"),
            JobStatus::Done => write!(f, "done"),
            JobStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(JobStatus::Pending),
            "processing" => Ok(JobStatus::Processing),
            "done" => Ok(JobStatus::Done),
            "failed" => Ok(JobStatus::Failed),
            _ => Err(anyhow::anyhow!("Invalid job status: {}", s)),
        }
    }
}

/// State of a single job chunk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobState {
    /// Job identifier
    pub job_id: i64,
    /// Chunk number within the job
    pub chunk: u64,
    /// Expected checksum for verification
    pub checksum: String,
    /// Current execution status
    pub status: JobStatus,
    /// Optional error message (for Failed status)
    pub error: Option<String>,
}

impl JobState {
    /// Create a new pending job state
    pub fn new(job_id: i64, chunk: u64, checksum: String) -> Self {
        Self {
            job_id,
            chunk,
            checksum,
            status: JobStatus::Pending,
            error: None,
        }
    }

    /// Create a job state with custom status
    pub fn with_status(job_id: i64, chunk: u64, checksum: String, status: JobStatus) -> Self {
        Self {
            job_id,
            chunk,
            checksum,
            status,
            error: None,
        }
    }
}

/// Dependency between job chunks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobDependency {
    /// Job identifier
    pub job_id: i64,
    /// Chunk that has dependencies
    pub chunk: u64,
    /// Chunk that must complete first
    pub depends_on: u64,
}

/// A status update event for asynchronous batch persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobUpdate {
    /// Chunk identifier
    pub chunk_id: u64,
    /// New status
    pub status: JobStatus,
    /// Optional checksum update
    pub checksum: Option<String>,
    /// Optional error message (for Failed status)
    pub error: Option<String>,
}

/// Core trait for persistent job storage backends
///
/// Implementations must ensure atomicity for claim operations and
/// maintain consistency across crashes.
#[async_trait]
pub trait JobStore: Send + Sync {
    /// Initialize job chunks from a TOML manifest
    ///
    /// Expected manifest format:
    /// ```toml
    /// [[chunks]]
    /// id = 1
    /// checksum = "abc123"
    /// ```
    async fn init_from_manifest(
        &mut self,
        job_id: i64,
        manifest: &toml::Value,
    ) -> anyhow::Result<()>;

    /// Atomically claim the next pending chunk
    ///
    /// Returns `None` if no pending chunks exist. The claimed chunk
    /// is marked as Processing and should be completed or failed.
    async fn claim_pending(&mut self, job_id: i64) -> anyhow::Result<Option<JobState>>;

    /// Atomically claim multiple pending chunks in one transaction
    ///
    /// Returns up to `limit` pending chunks, all marked as Processing.
    /// This reduces database lock contention during high-concurrency transfers.
    async fn claim_pending_batch(
        &mut self,
        job_id: i64,
        limit: usize,
    ) -> anyhow::Result<Vec<JobState>> {
        // Default implementation: claim one-by-one
        let mut batch = Vec::new();
        for _ in 0..limit {
            if let Some(state) = self.claim_pending(job_id).await? {
                batch.push(state);
            } else {
                break;
            }
        }
        Ok(batch)
    }

    /// Update the status of a chunk
    ///
    /// Optionally updates the checksum. For Failed status, include an error message.
    async fn mark_status(
        &mut self,
        job_id: i64,
        chunk: u64,
        status: JobStatus,
        checksum: Option<String>,
    ) -> anyhow::Result<()>;

    /// Mark a chunk as failed with an error message
    async fn mark_failed(&mut self, job_id: i64, chunk: u64, error: String) -> anyhow::Result<()> {
        self.mark_status(job_id, chunk, JobStatus::Failed, Some(error))
            .await
    }

    /// Apply a batch of status updates in a single transaction
    ///
    /// This is used by the JobManager to flush buffered updates efficiently.
    /// The default implementation applies updates one-by-one, but backends
    /// should override this for better performance.
    async fn apply_batch_updates(
        &mut self,
        job_id: i64,
        updates: &[JobUpdate],
    ) -> anyhow::Result<()> {
        // Default implementation: apply one-by-one
        for update in updates {
            self.mark_status(
                job_id,
                update.chunk_id,
                update.status,
                update.checksum.clone(),
            )
            .await?;
        }
        Ok(())
    }

    /// Get all pending chunks for a job (for resumption)
    async fn resume_pending(&self, job_id: i64) -> anyhow::Result<Vec<JobState>>;

    /// Get all chunks with a specific status
    async fn get_by_status(&self, job_id: i64, status: JobStatus) -> anyhow::Result<Vec<JobState>>;

    /// Add dependency relationships between chunks
    ///
    /// The chunk cannot be claimed until all dependencies are Done.
    async fn add_dependency(
        &mut self,
        job_id: i64,
        chunk: u64,
        deps: Vec<u64>,
    ) -> anyhow::Result<()>;

    /// Get all dependencies for a chunk
    async fn get_dependencies(&self, job_id: i64, chunk: u64) -> anyhow::Result<Vec<u64>>;

    /// Get chunks ready for processing (no unresolved dependencies)
    ///
    /// Performs topological sort to find chunks whose dependencies are all Done.
    async fn topo_sort_ready(&self, job_id: i64) -> anyhow::Result<Vec<u64>>;

    /// Get a specific chunk's state
    async fn get_chunk(&self, job_id: i64, chunk: u64) -> anyhow::Result<Option<JobState>>;

    /// Get statistics for a job
    async fn get_stats(&self, job_id: i64) -> anyhow::Result<JobStats>;

    /// Export all states to Parquet format for analytics
    #[cfg(feature = "analytics")]
    async fn export_to_parquet(&self, path: &str) -> anyhow::Result<()>;

    /// Create a new job and return its auto-generated ID
    ///
    /// Stores job metadata and returns the unique job ID that should be used
    /// for all subsequent operations (init_from_manifest, get_stats, etc.)
    async fn new_job(
        &mut self,
        source: String,
        destination: String,
        compress: bool,
        verify: bool,
        parallel: Option<usize>,
    ) -> anyhow::Result<i64>;

    /// Delete a job and all its chunks
    async fn delete_job(&mut self, job_id: i64) -> anyhow::Result<()>;
}

/// Statistics for a job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobStats {
    pub job_id: i64,
    pub total_chunks: u64,
    pub pending: u64,
    pub processing: u64,
    pub done: u64,
    pub failed: u64,
}

impl JobStats {
    /// Calculate completion percentage
    pub fn completion_percent(&self) -> f64 {
        if self.total_chunks == 0 {
            0.0
        } else {
            (self.done as f64 / self.total_chunks as f64) * 100.0
        }
    }

    /// Check if job is complete
    pub fn is_complete(&self) -> bool {
        self.done == self.total_chunks
    }

    /// Check if job has failures
    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }
}

/// Open a job store at the specified path
///
/// The backend is selected based on enabled features:
/// - SQLite (default): .db extension
/// - redb: .magnetar extension
#[allow(clippy::needless_return)]
pub async fn open(path: &str) -> anyhow::Result<Box<dyn JobStore>> {
    #[cfg(feature = "sqlite")]
    if path.ends_with(".db") || path.ends_with(".sqlite") || !path.contains('.') {
        let store = SqliteStore::open(path).await?;
        return Ok(Box::new(store));
    }

    #[cfg(feature = "redb")]
    if path.ends_with(".magnetar") || path.ends_with(".redb") {
        let store = RedbStore::open(path)?;
        return Ok(Box::new(store));
    }

    #[cfg(feature = "sqlite")]
    {
        // Default to SQLite
        let store = SqliteStore::open(path).await?;
        return Ok(Box::new(store));
    }

    #[cfg(not(feature = "sqlite"))]
    Err(anyhow::anyhow!(
        "No backend available for path: {}. Enable 'sqlite' or 'redb' feature.",
        path
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_display() {
        assert_eq!(JobStatus::Pending.to_string(), "pending");
        assert_eq!(JobStatus::Processing.to_string(), "processing");
        assert_eq!(JobStatus::Done.to_string(), "done");
        assert_eq!(JobStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_job_status_parse() {
        use std::str::FromStr;
        assert_eq!(JobStatus::from_str("pending").unwrap(), JobStatus::Pending);
        assert_eq!(JobStatus::from_str("DONE").unwrap(), JobStatus::Done);
        assert!(JobStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_job_state_new() {
        let state = JobState::new(1, 100, "abc123".to_string());
        assert_eq!(state.job_id, 1);
        assert_eq!(state.chunk, 100);
        assert_eq!(state.checksum, "abc123");
        assert_eq!(state.status, JobStatus::Pending);
    }

    #[test]
    fn test_job_stats() {
        let stats = JobStats {
            job_id: 1,
            total_chunks: 100,
            pending: 20,
            processing: 10,
            done: 65,
            failed: 5,
        };

        assert_eq!(stats.completion_percent(), 65.0);
        assert!(!stats.is_complete());
        assert!(stats.has_failures());
    }
}
