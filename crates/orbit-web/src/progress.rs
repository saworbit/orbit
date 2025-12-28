//! Progress Bridge - Connects Orbit Core progress events to database storage
//!
//! This module provides a database-backed progress implementation that:
//! - Receives high-frequency progress events from Orbit Core
//! - Debounces updates to prevent database lock contention
//! - Writes aggregated progress to SQLite for UI polling

use sqlx::{Pool, Sqlite};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

/// Messages sent from the transfer thread to the DB writer
#[derive(Debug)]
enum UpdateMsg {
    Bytes { total: u64 },
    ChunkSuccess,
    ChunkFail { index: usize, error: String },
    Finish,
}

/// A database-backed progress tracker with debouncing
///
/// This struct implements a non-blocking progress system that:
/// - Accepts rapid updates from Orbit Core (up to thousands per second)
/// - Batches updates and writes to DB every 250ms
/// - Prevents database lock contention during high-speed transfers
pub struct MagnetarProgress {
    job_id: i64,
    // Shared state for atomic speed calculations
    transferred: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    // Channel to the async background task
    tx: mpsc::Sender<UpdateMsg>,
}

impl MagnetarProgress {
    /// Create a new progress tracker for a job
    ///
    /// # Arguments
    /// * `job_id` - The database job ID to update
    /// * `initial_total` - Initial estimate of total bytes (may be updated)
    /// * `pool` - SQLite connection pool for database writes
    pub fn new(job_id: i64, initial_total: u64, pool: Pool<Sqlite>) -> Self {
        let (tx, mut rx) = mpsc::channel(1000); // Buffer up to 1000 events
        let transferred = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(initial_total));

        let total_clone = total.clone();

        // Spawn the debouncer task
        tokio::spawn(async move {
            let mut last_db_update = Instant::now();
            #[allow(unused_assignments)]
            let mut dirty = false;
            let mut current_bytes = 0u64;
            let mut current_total = initial_total;
            let mut completed_chunks = 0i64;
            let mut failed_chunks = 0i64;

            while let Some(msg) = rx.recv().await {
                match msg {
                    UpdateMsg::Bytes { total } => {
                        current_bytes = total;
                        dirty = true;
                    }
                    UpdateMsg::ChunkSuccess => {
                        completed_chunks += 1;
                        dirty = true;
                    }
                    UpdateMsg::ChunkFail { index, error } => {
                        failed_chunks += 1;
                        dirty = true;
                        tracing::warn!("Chunk {} failed: {}", index, error);
                    }
                    UpdateMsg::Finish => {
                        // Force final update
                        let progress = if current_total > 0 {
                            (current_bytes as f64 / current_total as f64 * 100.0) as f32
                        } else {
                            100.0
                        };

                        let now = chrono::Utc::now().timestamp();
                        let _ = sqlx::query(
                            "UPDATE jobs SET progress = ?, completed_chunks = ?, failed_chunks = ?, updated_at = ? WHERE id = ?"
                        )
                        .bind(progress)
                        .bind(completed_chunks)
                        .bind(failed_chunks)
                        .bind(now)
                        .bind(job_id)
                        .execute(&pool)
                        .await;

                        tracing::info!(
                            "Job {} final progress: {}% ({}/{} bytes, {} completed, {} failed)",
                            job_id,
                            progress,
                            current_bytes,
                            current_total,
                            completed_chunks,
                            failed_chunks
                        );
                        return;
                    }
                }

                // Debounce: Only write to DB every 250ms
                if dirty && last_db_update.elapsed().as_millis() > 250 {
                    current_total = total_clone.load(Ordering::Relaxed);
                    let progress = if current_total > 0 {
                        (current_bytes as f64 / current_total as f64 * 100.0) as f32
                    } else {
                        0.0
                    };

                    let now = chrono::Utc::now().timestamp();
                    let result = sqlx::query(
                        "UPDATE jobs SET progress = ?, completed_chunks = ?, failed_chunks = ?, updated_at = ? WHERE id = ?"
                    )
                    .bind(progress)
                    .bind(completed_chunks)
                    .bind(failed_chunks)
                    .bind(now)
                    .bind(job_id)
                    .execute(&pool)
                    .await;

                    if let Err(e) = result {
                        tracing::error!("Failed to update progress for job {}: {}", job_id, e);
                    }

                    last_db_update = Instant::now();
                    #[allow(unused_assignments)]
                    {
                        dirty = false;
                    }
                }
            }
        });

        Self {
            job_id,
            transferred,
            total,
            tx,
        }
    }

    /// Update the total size estimate
    pub fn set_length(&mut self, len: u64) {
        self.total.store(len, Ordering::Relaxed);
    }

    /// Increment transferred bytes
    pub fn inc(&mut self, delta: u64) {
        let current = self.transferred.fetch_add(delta, Ordering::Relaxed) + delta;
        // Non-blocking send - if channel is full, we drop the update (backpressure)
        let _ = self.tx.try_send(UpdateMsg::Bytes { total: current });
    }

    /// Mark the transfer as finished
    pub fn finish(&mut self) {
        // Ensure the finish message always gets through
        let _ = self.tx.blocking_send(UpdateMsg::Finish);
    }

    /// Record a successful chunk completion
    pub fn chunk_completed(&mut self, _chunk_idx: usize, _checksum: &str) {
        let _ = self.tx.try_send(UpdateMsg::ChunkSuccess);
    }

    /// Record a chunk failure
    pub fn chunk_failed(&mut self, chunk_idx: usize, err: &str) {
        let _ = self.tx.try_send(UpdateMsg::ChunkFail {
            index: chunk_idx,
            error: err.to_string(),
        });
    }
}

impl Drop for MagnetarProgress {
    fn drop(&mut self) {
        tracing::debug!("Progress tracker for job {} dropped", self.job_id);
    }
}
