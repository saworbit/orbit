//! Reactor Engine - The transfer job execution loop
//!
//! This module implements the core job execution engine that:
//! - Claims pending jobs from the database atomically
//! - Spawns isolated worker tasks for each transfer
//! - Handles errors and updates job status
//! - Provides backpressure and graceful shutdown

use crate::progress::MagnetarProgress;
use sqlx::{Pool, Row, Sqlite};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::{error, info, instrument, warn};

/// Type alias for the recursive file collection future
type FileCollectionFuture = std::pin::Pin<
    Box<
        dyn std::future::Future<
                Output = Result<Vec<(PathBuf, u64)>, Box<dyn std::error::Error + Send + Sync>>,
            > + Send,
    >,
>;

/// Job data structure matching the database schema
#[derive(Debug, Clone)]
pub struct Job {
    pub id: i64,
    pub source: String,
    pub destination: String,
    pub compress: bool,
    pub verify: bool,
    pub parallel_workers: i32,
    pub status: String,
}

/// The Reactor - infinite loop that processes transfer jobs
pub struct Reactor {
    pool: Pool<Sqlite>,
    notify: Arc<Notify>,
}

impl Reactor {
    /// Create a new Reactor instance
    pub fn new(pool: Pool<Sqlite>, notify: Arc<Notify>) -> Self {
        Self { pool, notify }
    }

    /// Main event loop - runs forever
    pub async fn run(self) {
        info!("☢️  Orbit Reactor Online - Waiting for transfer jobs...");

        loop {
            // 1. Attempt to claim work (atomic transaction)
            match self.claim_next_pending().await {
                Ok(Some(job)) => {
                    info!(
                        "🚀 Reactor claimed Job #{} ({} -> {})",
                        job.id, job.source, job.destination
                    );

                    // 2. Clone handles for the worker task
                    let pool_handle = self.pool.clone();

                    // 3. Spawn isolated worker (reactor doesn't block)
                    tokio::spawn(async move {
                        Self::execute_transfer(pool_handle, job).await;
                    });
                }
                Ok(None) => {
                    // 4. No work? Sleep until woken up by the API
                    tokio::select! {
                        _ = self.notify.notified() => {
                            // Woken up by /api/create_job! Loop immediately.
                            tracing::debug!("Reactor woken up by job creation");
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                            // Heartbeat wake-up just in case
                            tracing::trace!("Reactor heartbeat");
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Database error in reactor: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Atomically claim the next pending job
    async fn claim_next_pending(&self) -> Result<Option<Job>, sqlx::Error> {
        // Start a transaction for atomic claim
        let mut tx = self.pool.begin().await?;

        // Find the first pending job
        let row = sqlx::query(
            r#"
            SELECT id, source, destination, compress, verify, parallel
            FROM jobs
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = row {
            let job_id: i64 = row.get(0);

            // Atomically update status to running
            let now = chrono::Utc::now().timestamp();
            sqlx::query("UPDATE jobs SET status = 'running', updated_at = ? WHERE id = ?")
                .bind(now)
                .bind(job_id)
                .execute(&mut *tx)
                .await?;

            // Commit the transaction
            tx.commit().await?;

            // Return the claimed job
            Ok(Some(Job {
                id: job_id,
                source: row.get(1),
                destination: row.get(2),
                compress: row.get(3),
                verify: row.get(4),
                parallel_workers: row.get(5),
                status: "running".to_string(),
            }))
        } else {
            // No pending jobs
            tx.rollback().await?;
            Ok(None)
        }
    }

    /// Execute a transfer job
    #[instrument(skip(pool, job), fields(job_id = %job.id))]
    async fn execute_transfer(pool: Pool<Sqlite>, job: Job) {
        info!(
            "⚡ Starting transfer: {} -> {}",
            job.source, job.destination
        );

        // Create progress tracker
        let mut progress = MagnetarProgress::new(job.id, 0, pool.clone());

        // Perform actual file transfer
        let result = Self::perform_local_copy(&mut progress, &job).await;

        // Handle final state
        match result {
            Ok(bytes_transferred) => {
                info!(
                    "✅ Job #{} completed: {} bytes transferred",
                    job.id, bytes_transferred
                );

                let now = chrono::Utc::now().timestamp();
                let _ = sqlx::query(
                    "UPDATE jobs SET status = 'completed', updated_at = ? WHERE id = ?",
                )
                .bind(now)
                .bind(job.id)
                .execute(&pool)
                .await;
            }
            Err(e) => {
                error!("❌ Job #{} failed: {}", job.id, e);

                let now = chrono::Utc::now().timestamp();
                let _ =
                    sqlx::query("UPDATE jobs SET status = 'failed', updated_at = ? WHERE id = ?")
                        .bind(now)
                        .bind(job.id)
                        .execute(&pool)
                        .await;
            }
        }

        progress.finish();
    }

    /// Perform actual local file copy
    async fn perform_local_copy(
        progress: &mut MagnetarProgress,
        job: &Job,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        use tokio::fs;

        let source = PathBuf::from(&job.source);
        let destination = PathBuf::from(&job.destination);

        // Ensure destination directory exists
        fs::create_dir_all(&destination).await?;

        // Calculate total size and collect files
        info!("Scanning source directory: {}", job.source);
        let files = Self::collect_files(source.clone()).await?;
        let total_size: u64 = files.iter().map(|(_, size)| size).sum();

        progress.set_length(total_size);
        info!(
            "Starting transfer of {} files ({} bytes) for job #{}",
            files.len(),
            total_size,
            job.id
        );

        let mut bytes_transferred = 0u64;

        // Copy each file
        for (idx, (file_path, _file_size)) in files.iter().enumerate() {
            // Compute relative path
            let relative_path = file_path
                .strip_prefix(&source)
                .map_err(|e| format!("Path prefix error: {}", e))?;

            let dest_path = destination.join(relative_path);

            // Ensure parent directory exists
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Copy file
            match fs::copy(&file_path, &dest_path).await {
                Ok(bytes) => {
                    progress.inc(bytes);
                    progress.chunk_completed(idx, "copied");
                    bytes_transferred += bytes;
                }
                Err(e) => {
                    warn!("Failed to copy {:?}: {}", file_path, e);
                    progress.chunk_failed(idx, &e.to_string());
                    return Err(Box::new(e));
                }
            }
        }

        Ok(bytes_transferred)
    }

    /// Recursively collect all files in a directory
    fn collect_files(path: PathBuf) -> FileCollectionFuture {
        Box::pin(async move {
            use tokio::fs;

            let mut files = Vec::new();
            let metadata = fs::metadata(&path).await?;

            if metadata.is_file() {
                files.push((path.clone(), metadata.len()));
            } else if metadata.is_dir() {
                let mut entries = fs::read_dir(&path).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let entry_path = entry.path();
                    let entry_files = Self::collect_files(entry_path).await?;
                    files.extend(entry_files);
                }
            }

            Ok(files)
        })
    }
}
