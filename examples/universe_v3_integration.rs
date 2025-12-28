//! Example: Universe V3 Integration with Transfer Pipeline
//!
//! This example demonstrates how to integrate the Universe V3 index
//! with Orbit's transfer pipeline for global deduplication.
//!
//! # Features Demonstrated
//!
//! 1. Content-Defined Chunking (CDC) for shift-resilient chunking
//! 2. Universe V3 index for O(log N) deduplication lookups
//! 3. Streaming chunk processing with O(1) memory
//! 4. Early-exit optimization (find first local copy)
//!
//! # Run This Example
//!
//! ```bash
//! cargo run --example universe_v3_integration --release
//! ```

use orbit_core_cdc::{ChunkConfig, ChunkStream};
use orbit_core_starmap::universe_v3::{ChunkLocation, Universe};
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌌 Universe V3 Integration Example\n");

    // ─────────────────────────────────────────────────────────────
    // Step 1: Create a Universe V3 database
    // ─────────────────────────────────────────────────────────────
    println!("📁 Creating Universe V3 database...");
    let db_file = NamedTempFile::new()?;
    let universe = Universe::open(db_file.path())?;
    println!("   ✅ Database created at: {:?}\n", db_file.path());

    // ─────────────────────────────────────────────────────────────
    // Step 2: Simulate processing a file with CDC
    // ─────────────────────────────────────────────────────────────
    println!("🔪 Processing file with Content-Defined Chunking...");

    // Create a test file with some duplicated content
    let content = create_test_content();
    let reader = Cursor::new(&content);

    let config = ChunkConfig {
        min_size: 8 * 1024,   // 8 KB
        avg_size: 64 * 1024,  // 64 KB
        max_size: 256 * 1024, // 256 KB
    };

    let stream = ChunkStream::new(reader, config.clone());
    let mut chunk_count = 0;
    let mut total_bytes = 0;

    for chunk_result in stream {
        let chunk = chunk_result?;
        chunk_count += 1;
        total_bytes += chunk.data.len();

        println!(
            "   Chunk {}: {} bytes, hash: {:02x}{:02x}...{:02x}{:02x}",
            chunk_count,
            chunk.data.len(),
            chunk.hash[0],
            chunk.hash[1],
            chunk.hash[30],
            chunk.hash[31]
        );

        // ─────────────────────────────────────────────────────────
        // Step 3: Check if chunk exists in Universe (dedup check)
        // ─────────────────────────────────────────────────────────
        if universe.has_chunk(&chunk.hash)? {
            println!("      ✨ DEDUP HIT! Chunk already exists, skipping transfer");

            // Find where this chunk exists using streaming API
            let mut found_local = false;
            universe.scan_chunk(&chunk.hash, |location| {
                println!("         Existing location: {:?}", location.path);
                if location.path.starts_with("/local") {
                    found_local = true;
                    false // Stop searching, we found a local copy
                } else {
                    true // Continue searching for local copy
                }
            })?;

            if found_local {
                println!("      📍 Local copy found, using zero-copy reference");
            }
        } else {
            println!("      💾 New chunk, storing in Universe");

            // Store chunk location in Universe
            let location = ChunkLocation::new(
                "local".to_string(),
                PathBuf::from(format!("/data/file_{}.bin", chunk_count)),
                chunk.offset,
                chunk.data.len() as u32,
            );

            universe.insert_chunk(chunk.hash, location)?;
        }
    }

    println!(
        "\n   ✅ Processed {} chunks ({} bytes)\n",
        chunk_count, total_bytes
    );

    // ─────────────────────────────────────────────────────────────
    // Step 4: Simulate processing a duplicate file
    // ─────────────────────────────────────────────────────────────
    println!("🔄 Processing duplicate file...");

    let reader2 = Cursor::new(&content); // Same content = duplicate
    let stream2 = ChunkStream::new(reader2, config);
    let mut dedup_hits = 0;
    let mut new_chunks = 0;

    for chunk_result in stream2 {
        let chunk = chunk_result?;

        if universe.has_chunk(&chunk.hash)? {
            dedup_hits += 1;
        } else {
            new_chunks += 1;
            let location = ChunkLocation::new(
                "local".to_string(),
                PathBuf::from("/data/duplicate.bin"),
                chunk.offset,
                chunk.data.len() as u32,
            );
            universe.insert_chunk(chunk.hash, location)?;
        }
    }

    println!("   ✅ Dedup hits: {}", dedup_hits);
    println!("   ✅ New chunks: {}", new_chunks);
    println!(
        "   ✅ Dedup ratio: {:.1}%\n",
        (dedup_hits as f64 / (dedup_hits + new_chunks) as f64) * 100.0
    );

    // ─────────────────────────────────────────────────────────────
    // Step 5: Demonstrate high-cardinality performance
    // ─────────────────────────────────────────────────────────────
    println!("⚡ Testing high-cardinality performance...");

    let hot_hash = [0x42; 32]; // A "hot" chunk with many duplicates

    // Insert 10,000 duplicate references (this would timeout in V2!)
    let start = std::time::Instant::now();
    for i in 0..10_000 {
        let location = ChunkLocation::new(
            "local".to_string(),
            PathBuf::from(format!("/data/dup_{}.bin", i)),
            0,
            4096,
        );
        universe.insert_chunk(hot_hash, location)?;
    }
    let duration = start.elapsed();

    println!("   ✅ Inserted 10,000 duplicates in {:?}", duration);
    println!(
        "   ✅ Avg per insert: {:.2}ms",
        duration.as_millis() as f64 / 10_000.0
    );

    // Verify using streaming API (O(1) memory)
    let mut count = 0;
    universe.scan_chunk(&hot_hash, |_| {
        count += 1;
        true
    })?;

    println!("   ✅ Verified {} locations using streaming scan\n", count);

    // ─────────────────────────────────────────────────────────────
    // Step 6: Performance summary
    // ─────────────────────────────────────────────────────────────
    println!("📊 Performance Summary:");
    println!("   • Insert complexity: O(log N) regardless of duplicate count");
    println!("   • Memory usage: O(1) with streaming scan_chunk() API");
    println!("   • Production ready: Handles billions of chunks\n");

    println!("✅ Universe V3 integration example complete!");

    Ok(())
}

/// Create test content with some pattern for CDC
fn create_test_content() -> Vec<u8> {
    let mut content = Vec::new();

    // Add some structured data that will chunk predictably
    for i in 0..100 {
        let block = format!(
            "Block {}: This is a test block with some content that will be chunked.\n\
             It contains multiple lines and will create several chunks.\n\
             The content has enough variation to trigger CDC boundaries.\n\
             Let's add some more data to make it interesting: {}\n\n",
            i,
            "x".repeat(500)
        );
        content.extend_from_slice(block.as_bytes());
    }

    content
}

/// Example: Integrate Universe V3 into a transfer function
///
/// This is a simplified example showing how you might integrate Universe V3
/// into an actual file transfer function.
#[allow(dead_code)]
fn transfer_with_dedup(
    source_path: &PathBuf,
    dest_path: &PathBuf,
    universe: &Universe,
) -> Result<TransferStats, Box<dyn std::error::Error>> {
    let mut stats = TransferStats::default();

    // Open source file
    let file = File::open(source_path)?;

    // Configure CDC
    let config = ChunkConfig::default();
    let stream = ChunkStream::new(file, config);

    // Process chunks
    for chunk_result in stream {
        let chunk = chunk_result?;

        // Check for dedup
        if universe.has_chunk(&chunk.hash)? {
            stats.chunks_deduped += 1;
            stats.bytes_saved += chunk.data.len() as u64;

            // Try to find a local copy using streaming API
            let mut local_path: Option<PathBuf> = None;
            universe.scan_chunk(&chunk.hash, |location| {
                if location.path.starts_with("/local") {
                    local_path = Some(location.path.clone());
                    false // Stop searching
                } else {
                    true // Continue
                }
            })?;

            if let Some(path) = local_path {
                // Use zero-copy reference or CoW if available
                println!("Using local copy: {:?}", path);
            }
        } else {
            // New chunk - transfer it
            stats.chunks_transferred += 1;
            stats.bytes_transferred += chunk.data.len() as u64;

            // Store in Universe for future dedup
            let location = ChunkLocation::new(
                "local".to_string(),
                dest_path.clone(),
                chunk.offset,
                chunk.data.len() as u32,
            );
            universe.insert_chunk(chunk.hash, location)?;
        }
    }

    Ok(stats)
}

#[derive(Debug, Default)]
struct TransferStats {
    chunks_transferred: usize,
    chunks_deduped: usize,
    bytes_transferred: u64,
    bytes_saved: u64,
}

impl TransferStats {
    #[allow(dead_code)]
    fn dedup_ratio(&self) -> f64 {
        let total_chunks = self.chunks_transferred + self.chunks_deduped;
        if total_chunks == 0 {
            0.0
        } else {
            self.chunks_deduped as f64 / total_chunks as f64
        }
    }
}
