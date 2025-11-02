//! Crash recovery example for Magnetar
//!
//! Run with: cargo run --example crash_recovery --features sqlite
//!
//! This example simulates a crash mid-processing and demonstrates resumption.

use magnetar::JobStatus;
use anyhow::Result;

async fn simulate_processing(simulate_crash: bool) -> Result<()> {
    let mut store = magnetar::open("recovery_example.db").await?;

    // Check if we're resuming
    let pending = store.resume_pending(100).await?;

    if pending.is_empty() {
        println!("=== Starting fresh job ===\n");

        let manifest = toml::from_str(
            r#"
            [[chunks]]
            id = 1
            checksum = "chunk1"

            [[chunks]]
            id = 2
            checksum = "chunk2"

            [[chunks]]
            id = 3
            checksum = "chunk3"

            [[chunks]]
            id = 4
            checksum = "chunk4"

            [[chunks]]
            id = 5
            checksum = "chunk5"
            "#,
        )?;

        store.init_from_manifest(100, &manifest).await?;
        println!("✓ Initialized job with 5 chunks");
    } else {
        println!("=== Resuming job from crash ===\n");
        println!("Found {} pending chunks", pending.len());
        for chunk in &pending {
            println!("  Chunk {}: {}", chunk.chunk, chunk.checksum);
        }
        println!();
    }

    // Process chunks
    let mut processed = 0;
    while let Some(chunk) = store.claim_pending(100).await? {
        println!("Processing chunk {} ({})", chunk.chunk, chunk.checksum);

        // Simulate work
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Simulate crash after processing 2 chunks on first run
        if simulate_crash && processed == 2 {
            println!("\n💥 SIMULATED CRASH! (chunk {} was processing but not marked done)", chunk.chunk);
            println!("   On resume, this chunk will be claimed again.\n");
            return Ok(());
        }

        // Mark complete
        store.mark_status(100, chunk.chunk, JobStatus::Done, None).await?;
        println!("✓ Chunk {} completed", chunk.chunk);

        processed += 1;
    }

    let stats = store.get_stats(100).await?;
    println!("\n=== Job Complete ===");
    println!("Total processed: {}", stats.done);
    println!("Completion: {:.0}%", stats.completion_percent());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Magnetar Crash Recovery Example ===\n");

    // First run - simulate crash
    println!("--- First Run (will crash) ---\n");
    simulate_processing(true).await?;

    println!("\n--- Second Run (resume from crash) ---\n");
    simulate_processing(false).await?;

    // Clean up
    let mut store = magnetar::open("recovery_example.db").await?;
    store.delete_job(100).await?;
    println!("\n✓ Cleaned up");

    Ok(())
}
