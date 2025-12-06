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

        // TODO: This is where we'll integrate Orbit Core's copy logic
        // For now, simulate a transfer for demonstration
        let result = Self::simulate_transfer(&mut progress, &job).await;

        // Handle final state
        match result {
            Ok(bytes_transferred) => {
                info!(
                    "✅ Job #{} completed: {} bytes transferred",
                    job.id, bytes_transferred
                );

                let now = chrono::Utc::now().timestamp();
                let _ = sqlx::query("UPDATE jobs SET status = 'completed', updated_at = ? WHERE id = ?")
                    .bind(now)
                    .bind(job.id)
                    .execute(&pool)
                    .await;
            }
            Err(e) => {
                error!("❌ Job #{} failed: {}", job.id, e);

                let now = chrono::Utc::now().timestamp();
                let _ = sqlx::query("UPDATE jobs SET status = 'failed', updated_at = ? WHERE id = ?")
                    .bind(now)
                    .bind(job.id)
                    .execute(&pool)
                    .await;
            }
        }

        progress.finish();
    }

    /// Simulate a transfer (placeholder for Orbit Core integration)
    async fn simulate_transfer(
        progress: &mut MagnetarProgress,
        job: &Job,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        // In production, this will be replaced with:
        // orbit::core::copy::perform_copy(&config).await

        // Simulate file size discovery
        let total_size = Self::estimate_transfer_size(&job.source).await?;
        progress.set_length(total_size);

        info!(
            "Simulating transfer of {} bytes for job #{}",
            total_size, job.id
        );

        // Simulate chunked transfer
        let chunk_size = 1024 * 1024; // 1MB chunks
        let total_chunks = (total_size + chunk_size - 1) / chunk_size;

        for chunk_idx in 0..total_chunks {
            // Simulate chunk processing time
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Update progress
            let bytes_this_chunk = std::cmp::min(chunk_size, total_size - (chunk_idx * chunk_size));
            progress.inc(bytes_this_chunk);

            // Simulate occasional chunk completion events
            if chunk_idx % 10 == 0 {
                progress.chunk_completed(chunk_idx as usize, "simulated-checksum");
            }

            // Simulate occasional failures (1% failure rate)
            if chunk_idx % 100 == 50 {
                progress.chunk_failed(chunk_idx as usize, "simulated network timeout");
            }
        }

        Ok(total_size)
    }

    /// Estimate transfer size (placeholder)
    async fn estimate_transfer_size(
        _path: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        // In production, this will use std::fs::metadata or backend-specific APIs
        // For now, return a simulated size
        Ok(10 * 1024 * 1024) // 10 MB simulated transfer
    }
}
