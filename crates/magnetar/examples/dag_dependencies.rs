//! DAG dependencies example for Magnetar
//!
//! Run with: cargo run --example dag_dependencies --features sqlite

use anyhow::Result;
use magnetar::JobStatus;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Magnetar DAG Dependencies Example ===\n");

    let mut store = magnetar::open("dag_example.db").await?;
    println!("✓ Opened job store");

    // Create a job with dependencies
    // Task 3 depends on tasks 1 and 2
    // Task 4 depends on task 3
    let manifest = toml::from_str(
        r#"
        [[chunks]]
        id = 1
        checksum = "task1"

        [[chunks]]
        id = 2
        checksum = "task2"

        [[chunks]]
        id = 3
        checksum = "task3"

        [[chunks]]
        id = 4
        checksum = "task4"
        "#,
    )?;

    store.init_from_manifest(1, &manifest).await?;
    println!("✓ Initialized job with 4 tasks");

    // Set up dependencies
    store.add_dependency(1, 3, vec![1, 2]).await?; // Task 3 depends on 1 and 2
    store.add_dependency(1, 4, vec![3]).await?; // Task 4 depends on 3
    println!("✓ Configured dependencies:");
    println!("  Task 3 → depends on [1, 2]");
    println!("  Task 4 → depends on [3]");

    // Process in topological order
    println!("\nProcessing tasks in dependency order...");

    loop {
        // Get ready tasks (no unresolved dependencies)
        let ready = store.topo_sort_ready(1).await?;

        if ready.is_empty() {
            let stats = store.get_stats(1).await?;
            if stats.pending == 0 {
                println!("\n✓ All tasks completed!");
                break;
            } else {
                println!(
                    "\n⚠ No ready tasks but {} pending - circular dependency?",
                    stats.pending
                );
                break;
            }
        }

        println!("\nReady tasks: {:?}", ready);

        // Process first ready task
        if let Some(chunk_id) = ready.first() {
            if let Some(chunk) = store.get_chunk(1, *chunk_id).await? {
                println!("  Processing task {} ({})", chunk.chunk, chunk.checksum);

                // Simulate work
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

                // Mark complete
                store
                    .mark_status(1, chunk.chunk, JobStatus::Done, None)
                    .await?;
                println!("  ✓ Task {} completed", chunk.chunk);
            }
        }
    }

    // Show final stats
    let final_stats = store.get_stats(1).await?;
    println!("\nFinal statistics:");
    println!("  Total tasks: {}", final_stats.total_chunks);
    println!("  Completed: {}", final_stats.done);
    println!("  Failed: {}", final_stats.failed);

    // Clean up
    store.delete_job(1).await?;
    println!("\n✓ Cleaned up");

    Ok(())
}
