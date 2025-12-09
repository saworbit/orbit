//! Compute Executor: The "Air Gap" Pattern
//!
//! This module provides the boundary between the Async Orchestration Layer
//! and the Blocking Compute Layer. It prevents CPU-intensive tasks from
//! starving the Tokio runtime.
//!
//! # The Problem
//!
//! CPU-bound operations (BLAKE3 hashing, Adler-32 checksums, Delta algorithms)
//! block the async reactor when called directly from tokio::spawn. This causes:
//! - Heartbeat failures in orbit-web
//! - Network timeouts (S3/SMB keep-alives)
//! - UI unresponsiveness during "Planning" phase
//!
//! # The Solution
//!
//! Use `tokio::task::spawn_blocking` to run compute tasks on a dedicated thread pool
//! that is separate from the async reactor threads.
//!
//! # Example
//!
//! ```no_run
//! use magnetar::executor::offload_compute;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // This would normally block the reactor
//!     let result = offload_compute(move || {
//!         // Expensive BLAKE3 hashing, delta generation, etc.
//!         expensive_cpu_task()
//!     }).await?;
//!
//!     Ok(())
//! }
//!
//! fn expensive_cpu_task() -> anyhow::Result<u64> {
//!     // Heavy CPU work here
//!     Ok(42)
//! }
//! ```

pub mod gigantor;
pub mod standard;

use anyhow::Result;
use tokio::task;

/// Offloads a heavy CPU task to a thread where blocking is acceptable.
///
/// This prevents starving the async reactor by running the task on
/// Tokio's blocking thread pool instead of the async worker threads.
///
/// # Parameters
///
/// - `task`: A closure that performs CPU-intensive work and returns a Result<T>
///
/// # Returns
///
/// The result of the task, or an error if the task panicked or failed
///
/// # Example
///
/// ```ignore
/// use magnetar::executor::offload_compute;
///
/// let hash = offload_compute(move || {
///     // This expensive hashing operation runs on a blocking thread
///     let mut hasher = blake3::Hasher::new();
///     hasher.update(&vec![0u8; 1_000_000]);
///     Ok(hasher.finalize().to_hex().to_string())
/// }).await?;
/// ```
pub async fn offload_compute<F, T>(task: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    task::spawn_blocking(task)
        .await
        .map_err(|join_err| anyhow::anyhow!("Compute task panicked: {}", join_err))?
}

/// Offloads multiple independent compute tasks in parallel using Rayon.
///
/// This is useful for operations like parallel signature generation where
/// multiple blocks can be hashed independently.
///
/// # Parameters
///
/// - `items`: A vector of items to process
/// - `task`: A closure that processes each item
///
/// # Returns
///
/// A vector of results in the same order as the input items
///
/// # Example
///
/// ```ignore
/// use magnetar::executor::offload_parallel_compute;
///
/// let data_blocks = vec![vec![0u8; 1024]; 100];
/// let hashes = offload_parallel_compute(data_blocks, |block| {
///     let mut hasher = blake3::Hasher::new();
///     hasher.update(&block);
///     Ok(hasher.finalize().to_hex().to_string())
/// }).await?;
/// ```
pub async fn offload_parallel_compute<F, T, I>(items: Vec<I>, task: F) -> Result<Vec<T>>
where
    F: Fn(I) -> Result<T> + Send + Sync + 'static,
    T: Send + 'static,
    I: Send + 'static,
{
    task::spawn_blocking(move || {
        use rayon::prelude::*;
        items.into_par_iter().map(task).collect::<Result<Vec<T>>>()
    })
    .await
    .map_err(|join_err| anyhow::anyhow!("Parallel compute task panicked: {}", join_err))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;

    /// Mock heavy compute function
    fn mock_heavy_compute() -> Result<u64> {
        let start = std::time::Instant::now();
        let mut n = 0u64;
        while start.elapsed() < Duration::from_millis(100) {
            n = n.wrapping_add(1);
        }
        Ok(n)
    }

    #[tokio::test]
    async fn test_offload_compute_basic() {
        let result = offload_compute(mock_heavy_compute).await;
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }

    #[tokio::test]
    async fn test_offload_compute_with_error() {
        let result =
            offload_compute(|| -> Result<()> { Err(anyhow::anyhow!("intentional error")) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parallel_compute() {
        let items = vec![1, 2, 3, 4, 5];
        let results = offload_parallel_compute(items, |x| Ok(x * 2)).await;
        assert!(results.is_ok());
        assert_eq!(results.unwrap(), vec![2, 4, 6, 8, 10]);
    }

    #[tokio::test]
    async fn test_async_starvation_prevention() {
        let heartbeat_count = Arc::new(AtomicUsize::new(0));
        let hb_clone = heartbeat_count.clone();

        // Start a heartbeat task (simulates websocket ping or S3 keep-alive)
        let heartbeat_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(50)).await;
                hb_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        // Run heavy compute using offload
        let result = offload_compute(mock_heavy_compute).await;
        assert!(result.is_ok());

        // Wait a bit more to accumulate heartbeats
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify heartbeat continued during compute
        let count = heartbeat_count.load(Ordering::SeqCst);

        // We ran for ~200ms total with 50ms heartbeat interval
        // We should see at least 3 heartbeats (being conservative)
        assert!(
            count >= 3,
            "Reactor was starved! Heartbeats: {}, Expected: >= 3",
            count
        );

        heartbeat_handle.abort();
    }
}
