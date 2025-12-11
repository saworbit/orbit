//! Universe V3 Scalability Test Suite
//!
//! This test validates that the V3 Multimap architecture solves the O(N²)
//! write amplification bottleneck present in V2.
//!
//! The tests verify:
//! 1. Insert performance remains constant as duplicate count grows
//! 2. Memory usage stays bounded during reads
//! 3. Data integrity is maintained across all operations

use orbit_core_starmap::universe_v3::{ChunkLocation, Universe};
use std::path::PathBuf;
use std::time::Instant;
use tempfile::NamedTempFile;

/// High-cardinality torture test: Insert tens of thousands of duplicates
/// and verify that performance does NOT degrade quadratically.
#[test]
fn test_high_cardinality_insertions() {
    // Setup
    let tmp_file = NamedTempFile::new().unwrap();
    let universe = Universe::open(tmp_file.path()).unwrap();
    let hash = [0xAA; 32]; // The "hot" chunk with many duplicates

    println!("🚀 Starting High-Cardinality Torture Test...");

    let total_inserts = 20_000;
    let batch_size = 1_000;
    let mut batch_times = Vec::new();

    let start_total = Instant::now();

    // Insert duplicates in batches and measure each batch time
    for i in 0..(total_inserts / batch_size) {
        let batch_start = Instant::now();

        // Insert a batch of duplicates
        for j in 0..batch_size {
            let id = (i * batch_size) + j;
            let loc = ChunkLocation::new(
                "local".to_string(),
                PathBuf::from(format!("/data/file_{}.bin", id)),
                0,
                4096,
            );
            universe.insert_chunk(hash, loc).unwrap();
        }

        let duration = batch_start.elapsed();
        batch_times.push(duration);

        println!(
            "   Batch {:02}: {:?} for {} inserts (Total: {})",
            i,
            duration,
            batch_size,
            (i + 1) * batch_size
        );
    }

    let total_duration = start_total.elapsed();
    println!("✅ Total time: {:?}", total_duration);

    // Analysis: Check for performance degradation
    // In O(N²) scenario (V2), the last batch would be significantly slower than the first.
    // In O(log N) scenario (V3), they should be roughly equal (within noise tolerance).

    let first_batch = batch_times.first().unwrap().as_secs_f64();
    let last_batch = batch_times.last().unwrap().as_secs_f64();

    println!("📊 Performance Analysis:");
    println!("   First batch: {:.4}s", first_batch);
    println!("   Last batch:  {:.4}s", last_batch);

    // Calculate performance ratio
    let ratio = if first_batch > 0.0 {
        last_batch / first_batch
    } else {
        1.0
    };
    println!("   Ratio (last/first): {:.2}x", ratio);

    // Allow some variance for disk I/O noise, but if last batch is > 5x first batch,
    // we likely have a performance regression.
    // In V2, 20k items would make the last batch ~200x slower.
    assert!(
        ratio < 5.0,
        "Performance degraded non-linearly! Write amplification detected. Ratio: {:.2}x",
        ratio
    );

    // Verification: Read back all entries to ensure data integrity
    println!("🔍 Verifying data integrity...");
    let read_start = Instant::now();
    let mut count = 0;
    universe
        .scan_chunk(&hash, |_| {
            count += 1;
            true
        })
        .unwrap();

    println!("   Read back {} items in {:?}", count, read_start.elapsed());
    assert_eq!(count, total_inserts as usize);

    println!("✅ Scalability test PASSED!");
}

/// Mixed workload test: Multiple different chunks with varying duplicate counts
#[test]
fn test_mixed_workload() {
    let tmp_file = NamedTempFile::new().unwrap();
    let universe = Universe::open(tmp_file.path()).unwrap();

    println!("🔄 Testing mixed workload...");

    // Insert 3 different chunks with different duplicate counts
    let h1 = [0x11; 32]; // 1 duplicate
    let h2 = [0x22; 32]; // 10 duplicates
    let h3 = [0x33; 32]; // 100 duplicates

    // Chunk h1: Single location
    universe
        .insert_chunk(
            h1,
            ChunkLocation::new("local".to_string(), PathBuf::from("file_a.bin"), 0, 1024),
        )
        .unwrap();

    // Chunk h2: 10 locations
    for i in 0..10 {
        universe
            .insert_chunk(
                h2,
                ChunkLocation::new(
                    "local".to_string(),
                    PathBuf::from(format!("file_b_{}.bin", i)),
                    0,
                    2048,
                ),
            )
            .unwrap();
    }

    // Chunk h3: 100 locations
    for i in 0..100 {
        universe
            .insert_chunk(
                h3,
                ChunkLocation::new(
                    "local".to_string(),
                    PathBuf::from(format!("file_c_{}.bin", i)),
                    0,
                    4096,
                ),
            )
            .unwrap();
    }

    // Verify existence
    assert!(universe.has_chunk(&h1).unwrap());
    assert!(universe.has_chunk(&h2).unwrap());
    assert!(universe.has_chunk(&h3).unwrap());

    // Verify counts
    let mut h1_count = 0;
    universe
        .scan_chunk(&h1, |_| {
            h1_count += 1;
            true
        })
        .unwrap();
    assert_eq!(h1_count, 1);

    let mut h2_count = 0;
    universe
        .scan_chunk(&h2, |_| {
            h2_count += 1;
            true
        })
        .unwrap();
    assert_eq!(h2_count, 10);

    let mut h3_count = 0;
    universe
        .scan_chunk(&h3, |_| {
            h3_count += 1;
            true
        })
        .unwrap();
    assert_eq!(h3_count, 100);

    println!("✅ Mixed workload test PASSED!");
}

/// Memory efficiency test: Verify streaming reads don't exhaust memory
#[test]
fn test_streaming_memory_efficiency() {
    let tmp_file = NamedTempFile::new().unwrap();
    let universe = Universe::open(tmp_file.path()).unwrap();

    println!("💾 Testing streaming memory efficiency...");

    let hash = [0xBB; 32];
    let large_count = 10_000;

    // Insert many duplicates
    for i in 0..large_count {
        universe
            .insert_chunk(
                hash,
                ChunkLocation::new(
                    "local".to_string(),
                    PathBuf::from(format!("file_{}.bin", i)),
                    0,
                    8192,
                ),
            )
            .unwrap();
    }

    // Use scan_chunk to process without loading all into memory
    let mut processed = 0;
    let mut early_exit_count = 0;

    universe
        .scan_chunk(&hash, |_loc| {
            processed += 1;
            // Simulate early exit after finding first 5 matches
            if processed >= 5 {
                early_exit_count = processed;
                false // Stop iteration
            } else {
                true // Continue
            }
        })
        .unwrap();

    assert_eq!(early_exit_count, 5);
    println!(
        "   Processed {} items out of {} (early exit worked)",
        early_exit_count, large_count
    );

    // Now verify we can still get all if needed
    let iter = universe.find_chunk(hash).unwrap();
    let all_count = iter.len();
    assert_eq!(all_count, large_count);

    println!("✅ Streaming memory test PASSED!");
}

/// Persistence test: Verify data survives database close/reopen
#[test]
fn test_persistence_across_restarts() {
    let tmp_file = NamedTempFile::new().unwrap();
    let path = tmp_file.path().to_path_buf();

    println!("💿 Testing persistence across restarts...");

    let hash = [0xDD; 32];

    // Phase 1: Insert data
    {
        let universe = Universe::open(&path).unwrap();
        for i in 0..100 {
            universe
                .insert_chunk(
                    hash,
                    ChunkLocation::new(
                        "local".to_string(),
                        PathBuf::from(format!("persistent_{}.bin", i)),
                        0,
                        1024,
                    ),
                )
                .unwrap();
        }
        // universe goes out of scope, DB closes
    }

    // Phase 2: Reopen and verify
    {
        let universe = Universe::open(&path).unwrap();
        assert!(universe.has_chunk(&hash).unwrap());

        let mut count = 0;
        universe
            .scan_chunk(&hash, |_| {
                count += 1;
                true
            })
            .unwrap();
        assert_eq!(count, 100);

        println!("   Successfully recovered {} entries after restart", count);
    }

    println!("✅ Persistence test PASSED!");
}

/// Benchmark comparison helper (informational only, not a test assertion)
#[test]
fn test_benchmark_insert_performance() {
    let tmp_file = NamedTempFile::new().unwrap();
    let universe = Universe::open(tmp_file.path()).unwrap();

    println!("⚡ Benchmarking insert performance...");

    let hash = [0xEE; 32];
    let sample_sizes = [100, 1000, 5000, 10000];

    for &target in &sample_sizes {
        let start = Instant::now();

        for i in 0..target {
            universe
                .insert_chunk(
                    hash,
                    ChunkLocation::new(
                        "local".to_string(),
                        PathBuf::from(format!("bench_{}.bin", i)),
                        0,
                        4096,
                    ),
                )
                .unwrap();
        }

        let duration = start.elapsed();
        let per_insert = duration.as_micros() as f64 / target as f64;

        println!(
            "   {} inserts: {:?} ({:.2} µs per insert)",
            target, duration, per_insert
        );
    }

    println!("✅ Benchmark completed!");
}
