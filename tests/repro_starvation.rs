//! Reactor Starvation Prevention Test
//!
//! This test verifies that CPU-intensive tasks do not starve the Tokio async runtime.
//! It simulates a common problem where heavy compute (like BLAKE3 hashing or Delta
//! generation) blocks the reactor, causing heartbeat failures and network timeouts.
//!
//! The test runs a "heartbeat" task that must tick regularly while heavy compute is running.
//! If the compute blocks the reactor, the heartbeat will freeze.
//!
//! This test is designed to FAIL if you run heavy compute directly on the async runtime,
//! and PASS when using spawn_blocking or the magnetar executor module.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::time::sleep;

/// Mock heavy compute function simulating BLAKE3 hashing or Delta generation
/// This burns CPU cycles for approximately 2 seconds
fn mock_heavy_compute() -> u64 {
    let start = std::time::Instant::now();
    let mut n = 0;
    while start.elapsed() < Duration::from_secs(2) {
        // Burn cycles to simulate heavy CPU load
        n += 1;
        if n % 1_000_000 == 0 {
            std::thread::yield_now();
        }
    }
    n
}

#[tokio::test]
async fn test_async_starvation_prevention() {
    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let hb_clone = heartbeat_count.clone();

    // 1. Start a "Heartbeat" task
    // This represents the Web GUI websocket ping or S3 keep-alive
    let heartbeat_handle = tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(100)).await;
            hb_clone.fetch_add(1, Ordering::SeqCst);
        }
    });

    // 2. Run the heavy task using spawn_blocking
    // SCENARIO A: If we run this directly, heartbeat_count will freeze
    // SCENARIO B: If we run this via spawn_blocking, heartbeat_count keeps incrementing

    println!("Starting heavy compute...");

    let result = tokio::task::spawn_blocking(move || mock_heavy_compute())
        .await
        .expect("Task failed");

    println!("Heavy compute finished. Cycles: {}", result);

    // 3. Verify Heartbeat
    // The compute took ~2 seconds. Heartbeat runs every 100ms.
    // We expect roughly 20 heartbeats.
    // If starvation occurred, this would be 0 or 1.
    let count = heartbeat_count.load(Ordering::SeqCst);
    println!("Heartbeats recorded: {}", count);

    assert!(
        count >= 15,
        "Reactor was starved! Heartbeats: {}, Expected: ~20",
        count
    );

    // Cleanup
    heartbeat_handle.abort();
}

#[tokio::test]
async fn test_direct_blocking_causes_starvation() {
    // This test demonstrates what happens WITHOUT spawn_blocking
    // It should show reduced heartbeat counts (though still some due to thread yielding)

    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let hb_clone = heartbeat_count.clone();

    let heartbeat_handle = tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(100)).await;
            hb_clone.fetch_add(1, Ordering::SeqCst);
        }
    });

    // Run a shorter CPU-bound task directly (NOT using spawn_blocking)
    println!("Running blocking task directly on async runtime (BAD PRACTICE)...");

    let start = std::time::Instant::now();
    let mut n = 0u64;
    while start.elapsed() < Duration::from_millis(500) {
        n = n.wrapping_add(1);
    }

    println!("Blocking task finished. Cycles: {}", n);

    // Even though we only blocked for 500ms, the heartbeat (running every 100ms)
    // should have been severely impacted
    let count = heartbeat_count.load(Ordering::SeqCst);
    println!("Heartbeats during direct blocking: {}", count);

    // This assertion is informational - showing the problem
    // In real scenarios, this could be 0-2 instead of expected ~5
    if count < 3 {
        println!("WARNING: Direct blocking significantly reduced heartbeat count!");
    }

    heartbeat_handle.abort();
}

#[tokio::test]
async fn test_magnetar_executor_prevents_starvation() {
    // Test using the magnetar executor module
    use magnetar::executor::offload_compute;

    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let hb_clone = heartbeat_count.clone();

    let heartbeat_handle = tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(100)).await;
            hb_clone.fetch_add(1, Ordering::SeqCst);
        }
    });

    println!("Testing magnetar executor...");

    // Use the magnetar executor to offload compute
    let result = offload_compute(|| -> anyhow::Result<u64> {
        let start = std::time::Instant::now();
        let mut n = 0u64;
        while start.elapsed() < Duration::from_secs(2) {
            n = n.wrapping_add(1);
            if n % 1_000_000 == 0 {
                std::thread::yield_now();
            }
        }
        Ok(n)
    })
    .await
    .expect("Executor task failed");

    println!("Executor task finished. Cycles: {}", result);

    let count = heartbeat_count.load(Ordering::SeqCst);
    println!("Heartbeats with executor: {}", count);

    assert!(
        count >= 15,
        "Magnetar executor failed to prevent starvation! Heartbeats: {}, Expected: ~20",
        count
    );

    heartbeat_handle.abort();
}

#[tokio::test]
async fn test_parallel_compute_starvation_prevention() {
    // Test parallel compute using the magnetar executor
    use magnetar::executor::offload_parallel_compute;

    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let hb_clone = heartbeat_count.clone();

    let heartbeat_handle = tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(100)).await;
            hb_clone.fetch_add(1, Ordering::SeqCst);
        }
    });

    println!("Testing parallel compute executor...");

    // Create 10 heavy tasks to run in parallel
    let tasks: Vec<usize> = (0..10).collect();

    let results = offload_parallel_compute(tasks, |task_id| -> anyhow::Result<u64> {
        let start = std::time::Instant::now();
        let mut n = 0u64;
        while start.elapsed() < Duration::from_millis(200) {
            n = n.wrapping_add(1);
        }
        Ok(n)
    })
    .await
    .expect("Parallel executor task failed");

    println!("Parallel executor finished. Tasks completed: {}", results.len());

    let count = heartbeat_count.load(Ordering::SeqCst);
    println!("Heartbeats with parallel executor: {}", count);

    // Even with 10 parallel tasks of 200ms each, the heartbeat should continue
    assert!(
        count >= 15,
        "Parallel executor failed to prevent starvation! Heartbeats: {}, Expected: ~20",
        count
    );

    heartbeat_handle.abort();
}
