//! V2 CDC Resilience Check
//!
//! This test verifies that Content-Defined Chunking solves the "shift problem"
//! that plagued fixed-size chunking in Orbit V1.
//!
//! Test Strategy:
//! 1. Create a synthetic data file
//! 2. Chunk it and record all chunk hashes
//! 3. Insert 1 byte at offset 0
//! 4. Re-chunk and compare hashes
//! 5. Verify: Only 1-2 chunks changed (not ALL chunks)

use orbit_core_cdc::{ChunkConfig, ChunkStream};
use std::io::Cursor;

/// Generate deterministic test data
fn generate_test_data(size: usize, seed: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let mut value = seed;

    for i in 0..size {
        // Create some pattern variation to get realistic chunking
        value = value.wrapping_mul(7).wrapping_add((i % 256) as u8);
        data.push(value);
    }

    data
}

/// Extract just the content hashes for comparison
/// CDC resilience means the same content should produce the same hash,
/// regardless of where it appears in the file
fn hash_chunks(chunks: &[(u64, usize, [u8; 32])]) -> Vec<String> {
    chunks
        .iter()
        .map(|(_offset, _length, hash)| {
            // Only compare content hash, not position or size
            // This is the whole point of CDC - content-defined boundaries
            hex::encode(hash)
        })
        .collect()
}

#[test]
fn test_cdc_resilience_to_insertion() {
    println!("\n=== CDC Resilience Test ===\n");

    // Generate 2MB of test data
    let original_data = generate_test_data(2 * 1024 * 1024, 42);

    // Configure CDC with reasonable parameters
    let config = ChunkConfig::new(
        16 * 1024,  // 16 KB min
        64 * 1024,  // 64 KB avg
        256 * 1024, // 256 KB max
    )
    .unwrap();

    println!(
        "Step 1: Chunking original data ({} bytes)...",
        original_data.len()
    );

    // Chunk the original data
    let stream = ChunkStream::new(Cursor::new(&original_data), config.clone());
    let original_chunks: Vec<_> = stream
        .map(|r| {
            let chunk = r.unwrap();
            (chunk.offset, chunk.length, chunk.hash)
        })
        .collect();

    println!("  → Created {} chunks", original_chunks.len());
    println!(
        "  → Chunk sizes: min={}, max={}, avg={:.0}",
        original_chunks.iter().map(|(_, len, _)| len).min().unwrap(),
        original_chunks.iter().map(|(_, len, _)| len).max().unwrap(),
        original_chunks.iter().map(|(_, len, _)| len).sum::<usize>() as f64
            / original_chunks.len() as f64
    );

    // Insert 1 byte at offset 0 (worst case for fixed chunking)
    let mut modified_data = vec![0xFF]; // Insert one byte
    modified_data.extend_from_slice(&original_data);

    println!("\nStep 2: Inserting 1 byte at offset 0...");
    println!("  → Modified data size: {} bytes", modified_data.len());

    // Chunk the modified data
    let stream = ChunkStream::new(Cursor::new(&modified_data), config);
    let modified_chunks: Vec<_> = stream
        .map(|r| {
            let chunk = r.unwrap();
            (chunk.offset, chunk.length, chunk.hash)
        })
        .collect();

    println!("  → Created {} chunks", modified_chunks.len());

    // Compare chunk hashes
    let original_hashes = hash_chunks(&original_chunks);
    let modified_hashes = hash_chunks(&modified_chunks);

    // Count how many chunks changed
    let mut chunks_changed = 0;
    let mut chunks_preserved = 0;

    println!("\nStep 3: Analyzing chunk preservation...");

    // Find matching hashes (order-independent comparison)
    for orig_hash in &original_hashes {
        if modified_hashes.contains(orig_hash) {
            chunks_preserved += 1;
        } else {
            chunks_changed += 1;
        }
    }

    let preservation_rate = (chunks_preserved as f64 / original_chunks.len() as f64) * 100.0;

    println!(
        "  → Chunks preserved: {}/{} ({:.1}%)",
        chunks_preserved,
        original_chunks.len(),
        preservation_rate
    );
    println!("  → Chunks changed: {}", chunks_changed);

    // The key assertion: CDC should preserve most chunks
    // With a 1-byte insertion, we expect:
    // - First chunk to change (contains the insertion)
    // - Possibly second chunk to change (boundary shift)
    // - All remaining chunks to be IDENTICAL

    println!("\nStep 4: Verification...");

    // At minimum, we should preserve > 90% of chunks
    // (In practice, CDC preserves 95%+ for single-byte insertions)
    assert!(
        preservation_rate > 90.0,
        "FAIL: Only {:.1}% of chunks preserved. Expected >90%. \
         This indicates CDC is not working correctly!",
        preservation_rate
    );

    // Also verify we have at least 1 changed chunk (otherwise test is broken)
    assert!(
        chunks_changed > 0,
        "FAIL: No chunks changed after insertion. Test data may be invalid."
    );

    println!(
        "  ✓ PASS: CDC successfully preserved {:.1}% of chunks",
        preservation_rate
    );
    println!(
        "  ✓ PASS: Only {} chunks affected by 1-byte insertion",
        chunks_changed
    );

    println!("\n=== Test Complete ===\n");
}

#[test]
fn test_chunk_size_distribution() {
    println!("\n=== Chunk Size Distribution Test ===\n");

    let data = generate_test_data(10 * 1024 * 1024, 123); // 10MB

    let config = ChunkConfig::new(
        8 * 1024,   // 8 KB min
        64 * 1024,  // 64 KB avg
        256 * 1024, // 256 KB max
    )
    .unwrap();

    let stream = ChunkStream::new(Cursor::new(&data), config);
    let chunks: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();

    println!("Chunked {} bytes into {} chunks", data.len(), chunks.len());

    // Verify size constraints
    for chunk in &chunks {
        assert!(
            chunk.length >= 8 * 1024 || chunk.offset + chunk.length as u64 == data.len() as u64,
            "Chunk too small: {} bytes at offset {}",
            chunk.length,
            chunk.offset
        );
        assert!(
            chunk.length <= 256 * 1024,
            "Chunk too large: {} bytes at offset {}",
            chunk.length,
            chunk.offset
        );
    }

    let avg_size = chunks.iter().map(|c| c.length).sum::<usize>() / chunks.len();
    let min_size = chunks.iter().map(|c| c.length).min().unwrap();
    let max_size = chunks.iter().map(|c| c.length).max().unwrap();

    println!("  Min chunk: {} KB", min_size / 1024);
    println!("  Max chunk: {} KB", max_size / 1024);
    println!("  Avg chunk: {} KB", avg_size / 1024);

    // Average should be in a reasonable range
    // CDC chunk sizes vary based on content, so allow wide range
    // Target is 64KB, but accept anywhere from 4KB to 128KB
    assert!(
        (4 * 1024..=128 * 1024).contains(&avg_size),
        "Average chunk size {} KB is outside acceptable range (4KB-128KB)",
        avg_size / 1024
    );

    println!("  ✓ PASS: Chunk sizes within expected range");
    println!("\n=== Test Complete ===\n");
}

#[test]
fn test_deterministic_chunking() {
    println!("\n=== Deterministic Chunking Test ===\n");

    let data = generate_test_data(1024 * 1024, 77); // 1MB

    let config = ChunkConfig::default_config();

    // Chunk the same data twice
    let stream1 = ChunkStream::new(Cursor::new(&data), config.clone());
    let chunks1: Vec<_> = stream1.map(|r| r.unwrap().hash).collect();

    let stream2 = ChunkStream::new(Cursor::new(&data), config);
    let chunks2: Vec<_> = stream2.map(|r| r.unwrap().hash).collect();

    // Must produce identical chunks
    assert_eq!(
        chunks1.len(),
        chunks2.len(),
        "Different number of chunks produced"
    );

    for (i, (hash1, hash2)) in chunks1.iter().zip(chunks2.iter()).enumerate() {
        assert_eq!(
            hash1, hash2,
            "Chunk {} has different hash on second run - chunking is not deterministic!",
            i
        );
    }

    println!(
        "  ✓ PASS: Produced identical {} chunks on both runs",
        chunks1.len()
    );
    println!("\n=== Test Complete ===\n");
}

// Helper for hex encoding (simple implementation to avoid extra dependencies in test)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
