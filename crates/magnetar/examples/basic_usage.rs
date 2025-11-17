//! Basic usage example for Magnetar
//!
//! Run with: cargo run --example basic_usage --features sqlite

use anyhow::Result;
use magnetar::JobStatus;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Magnetar Basic Usage Example ===\n");

    // Open a SQLite store
    let mut store = magnetar::open("example_jobs.db").await?;
    println!("✓ Opened job store");

    // Define a manifest with chunks to process
    let manifest = toml::from_str(
        r#"
        [[chunks]]
        id = 1
        checksum = "abc123"

        [[chunks]]
        id = 2
        checksum = "def456"

        [[chunks]]
        id = 3
        checksum = "789ghi"
        "#,
    )?;

    // Initialize job from manifest
    store.init_from_manifest(42, &manifest).await?;
    println!("✓ Initialized job 42 with 3 chunks");

    // Get initial stats
    let stats = store.get_stats(42).await?;
    println!("\nInitial stats:");
    println!("  Total: {}", stats.total_chunks);
    println!("  Pending: {}", stats.pending);
    println!("  Done: {}", stats.done);

    // Process chunks one by one
    println!("\nProcessing chunks...");
    let mut processed = 0;

    while let Some(chunk) = store.claim_pending(42).await? {
        println!(
            "  Claimed chunk {} (checksum: {})",
            chunk.chunk, chunk.checksum
        );

        // Simulate processing work
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Mark as done
        store
            .mark_status(42, chunk.chunk, JobStatus::Done, None)
            .await?;
        processed += 1;
        println!("  ✓ Completed chunk {}", chunk.chunk);
    }

    println!("\nProcessed {} chunks", processed);

    // Final stats
    let final_stats = store.get_stats(42).await?;
    println!("\nFinal stats:");
    println!("  Total: {}", final_stats.total_chunks);
    println!("  Pending: {}", final_stats.pending);
    println!("  Done: {}", final_stats.done);
    println!("  Completion: {:.1}%", final_stats.completion_percent());

    // Clean up
    store.delete_job(42).await?;
    println!("\n✓ Cleaned up job 42");

    Ok(())
}
