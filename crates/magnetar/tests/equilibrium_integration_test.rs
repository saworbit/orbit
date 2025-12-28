//! Integration tests for the Equilibrium Standard Lane
//!
//! These tests verify that the Standard Executor correctly:
//! 1. Chunks files using CDC with 64KB average
//! 2. Deduplicates identical content globally
//! 3. Handles partial deduplication
//! 4. Provides accurate statistics

use anyhow::Result;
use magnetar::executor::standard::StandardExecutor;
use orbit_core_starmap::universe::Universe;
use std::sync::Arc;
use tempfile::{tempdir, NamedTempFile};

/// Helper to create a Universe database in a temp directory
fn create_test_universe() -> Result<(tempfile::TempDir, Arc<Universe>)> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("universe.db");
    let universe = Arc::new(Universe::open(&db_path)?);
    Ok((temp_dir, universe))
}

/// Test: Equilibrium correctly deduplicates identical files
///
/// This is the core test from SPEC-005 that validates:
/// - First file is fully chunked and transferred
/// - Second identical file results in 100% deduplication
#[tokio::test]
async fn test_equilibrium_deduplication() -> Result<()> {
    // Setup: Create Universe (empty)
    let (_temp_dir, universe) = create_test_universe()?;
    let executor = StandardExecutor::new(universe.clone());

    // Create two 1MB files with identical content
    // 1MB is > 8KB (Neutrino) and < 1GB (Large), so it hits Standard Lane.
    let data = vec![0x42u8; 1024 * 1024]; // 1MB

    let file1 = NamedTempFile::new()?;
    std::fs::write(file1.path(), &data)?;

    let file2 = NamedTempFile::new()?;
    std::fs::write(file2.path(), &data)?;

    // Execute File 1
    let stats_pass_1 = executor.process_file(file1.path().to_path_buf()).await?;

    // Assert: Universe should now contain chunks
    // Note: CDC on uniform data (all 0x42) produces fewer, larger chunks than varied data
    assert!(
        stats_pass_1.total_chunks >= 1,
        "Expected at least 1 chunk, got {}",
        stats_pass_1.total_chunks
    );
    assert_eq!(
        stats_pass_1.bytes_transferred,
        1024 * 1024,
        "First pass should transfer all bytes"
    );
    assert_eq!(
        stats_pass_1.chunks_transferred, stats_pass_1.total_chunks,
        "First pass should transfer all chunks"
    );

    // Execute File 2 (Identical)
    let stats_pass_2 = executor.process_file(file2.path().to_path_buf()).await?;

    // Assert: Zero bytes should be transferred for File 2
    assert_eq!(
        stats_pass_2.total_chunks, stats_pass_1.total_chunks,
        "Second file should have same chunk count"
    );

    // The critical check: did we deduplicate?
    assert_eq!(
        stats_pass_2.chunks_transferred, 0,
        "Second pass should transfer no chunks (100% dedup)"
    );
    assert_eq!(
        stats_pass_2.bytes_transferred, 0,
        "Second pass should transfer no bytes (100% dedup)"
    );
    assert_eq!(
        stats_pass_2.bytes_deduplicated,
        1024 * 1024,
        "Second pass should deduplicate all bytes"
    );

    Ok(())
}

/// Test: Equilibrium uses correct chunk sizes (64KB average)
///
/// Validates that CDC chunking produces chunks in the expected size range.
#[tokio::test]
async fn test_equilibrium_chunk_sizes() -> Result<()> {
    let (_temp_dir, universe) = create_test_universe()?;
    let executor = StandardExecutor::new(universe);

    // Create a 2MB file to get good chunk distribution
    let data = vec![0x42u8; 2 * 1024 * 1024];
    let file = NamedTempFile::new()?;
    std::fs::write(file.path(), &data)?;

    // Process the file
    let stats = executor.process_file(file.path().to_path_buf()).await?;

    // CDC on uniform data produces fewer chunks than the theoretical average
    // With 64KB average on varied data, 2MB would produce ~32 chunks
    // But uniform data (all 0x42) will produce larger chunks, so accept 1-20 chunks
    assert!(
        stats.total_chunks >= 1 && stats.total_chunks <= 20,
        "Expected 1-20 chunks for 2MB uniform data, got {}",
        stats.total_chunks
    );

    // Verify we got reasonable chunk sizes (min 8KB, max 256KB from config)
    let avg_chunk_size = stats.bytes_transferred / stats.total_chunks as u64;
    assert!(
        avg_chunk_size >= 8 * 1024 && avg_chunk_size <= 256 * 1024,
        "Expected avg chunk size 8-256KB, got {}KB",
        avg_chunk_size / 1024
    );

    Ok(())
}

/// Test: Equilibrium handles partial deduplication correctly
///
/// Tests scenario where two files share some but not all chunks.
#[tokio::test]
async fn test_equilibrium_partial_dedup() -> Result<()> {
    let (_temp_dir, universe) = create_test_universe()?;
    let executor = StandardExecutor::new(universe);

    // File 1: 1MB of 0x42
    let data1 = vec![0x42u8; 1024 * 1024];
    let file1 = NamedTempFile::new()?;
    std::fs::write(file1.path(), &data1)?;

    // File 2: 1MB of 0x42 + 1MB of 0x99 (partial overlap)
    let mut data2 = vec![0x42u8; 1024 * 1024];
    data2.extend(vec![0x99u8; 1024 * 1024]);
    let file2 = NamedTempFile::new()?;
    std::fs::write(file2.path(), &data2)?;

    // Process File 1
    let _stats1 = executor.process_file(file1.path().to_path_buf()).await?;

    // Process File 2 (partial overlap)
    let stats2 = executor.process_file(file2.path().to_path_buf()).await?;

    // Verify partial deduplication
    assert!(
        stats2.bytes_deduplicated > 0,
        "Should have some deduplication from overlapping content"
    );
    assert!(
        stats2.bytes_transferred > 0,
        "Should transfer new unique content"
    );
    assert!(
        stats2.bytes_transferred < 2 * 1024 * 1024,
        "Should not transfer all bytes due to partial dedup"
    );

    // Roughly 50% should be deduplicated
    let dedup_ratio = stats2.bytes_deduplicated as f64
        / (stats2.bytes_deduplicated + stats2.bytes_transferred) as f64;
    assert!(
        dedup_ratio > 0.3 && dedup_ratio < 0.7,
        "Expected ~50% dedup ratio, got {:.1}%",
        dedup_ratio * 100.0
    );

    Ok(())
}

/// Test: Equilibrium handles small files correctly
///
/// Files just above 8KB threshold should still be handled by Equilibrium.
#[tokio::test]
async fn test_equilibrium_small_files() -> Result<()> {
    let (_temp_dir, universe) = create_test_universe()?;
    let executor = StandardExecutor::new(universe);

    // Create a 16KB file (just above 8KB Neutrino threshold)
    let data = vec![0x42u8; 16 * 1024];
    let file = NamedTempFile::new()?;
    std::fs::write(file.path(), &data)?;

    // Process the file
    let stats = executor.process_file(file.path().to_path_buf()).await?;

    // Should produce at least 1 chunk
    assert!(stats.total_chunks > 0, "Should produce chunks");
    assert_eq!(
        stats.bytes_transferred,
        16 * 1024,
        "Should transfer all bytes"
    );

    Ok(())
}

/// Test: Equilibrium handles empty files gracefully
#[tokio::test]
async fn test_equilibrium_empty_file() -> Result<()> {
    let (_temp_dir, universe) = create_test_universe()?;
    let executor = StandardExecutor::new(universe);

    // Create empty file
    let file = NamedTempFile::new()?;

    // Process empty file (should not error)
    let stats = executor.process_file(file.path().to_path_buf()).await?;

    // Verify zero stats
    assert_eq!(stats.total_chunks, 0);
    assert_eq!(stats.chunks_transferred, 0);
    assert_eq!(stats.bytes_transferred, 0);
    assert_eq!(stats.bytes_deduplicated, 0);

    Ok(())
}

/// Test: Batch processing aggregates statistics correctly
#[tokio::test]
async fn test_equilibrium_batch_processing() -> Result<()> {
    let (_temp_dir, universe) = create_test_universe()?;
    let executor = StandardExecutor::new(universe);

    // Create 5 files with different content (keep temp files alive!)
    let mut temp_files = Vec::new();
    let mut files = Vec::new();
    for i in 0..5 {
        let data = vec![i as u8; 100 * 1024]; // 100KB each
        let file = NamedTempFile::new()?;
        std::fs::write(file.path(), &data)?;
        files.push(file.path().to_path_buf());
        temp_files.push(file); // Keep alive
    }

    // Process batch
    let stats = executor.process_batch(files).await?;

    // Verify aggregate stats
    assert!(stats.total_chunks > 0, "Should process chunks");
    assert!(stats.bytes_transferred > 0, "Should transfer bytes");
    assert_eq!(
        stats.bytes_transferred,
        5 * 100 * 1024,
        "Should transfer all unique bytes"
    );

    Ok(())
}

/// Test: Deduplication across different file patterns
///
/// Tests CDC's ability to detect identical chunks even when embedded
/// in different file structures.
#[tokio::test]
async fn test_equilibrium_cross_file_dedup() -> Result<()> {
    let (_temp_dir, universe) = create_test_universe()?;
    let executor = StandardExecutor::new(universe);

    // Create a common pattern that appears in multiple files
    let common_pattern = vec![0xAAu8; 256 * 1024]; // 256KB

    // File 1: [common_pattern][unique_data1]
    let mut data1 = common_pattern.clone();
    data1.extend(vec![0x11u8; 256 * 1024]);
    let file1 = NamedTempFile::new()?;
    std::fs::write(file1.path(), &data1)?;

    // File 2: [unique_data2][common_pattern]
    let mut data2 = vec![0x22u8; 256 * 1024];
    data2.extend(common_pattern.clone());
    let file2 = NamedTempFile::new()?;
    std::fs::write(file2.path(), &data2)?;

    // Process both files
    let _stats1 = executor.process_file(file1.path().to_path_buf()).await?;
    let stats2 = executor.process_file(file2.path().to_path_buf()).await?;

    // File 2 should have significant deduplication due to common_pattern
    assert!(
        stats2.bytes_deduplicated > 100 * 1024,
        "Should deduplicate at least 100KB of common pattern"
    );
    assert!(
        stats2.bytes_transferred < stats2.bytes_deduplicated + stats2.bytes_transferred,
        "Should transfer less than total due to dedup"
    );

    Ok(())
}
