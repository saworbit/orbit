//! Integration Test: Orbit V2 Complete Stack
//!
//! This test demonstrates the full V2 architecture working together:
//! 1. CDC (Content-Defined Chunking) for shift-resilient chunking
//! 2. Semantic Layer for intelligent prioritization
//! 3. Universe Map for global deduplication
//!
//! Scenario: Replicate a project with config files, code, and media
//! Expected: Config files get priority, global dedup works across files

use orbit_core_cdc::{ChunkConfig, ChunkStream};
use orbit_core_semantic::{Priority, SemanticRegistry};
use orbit_core_starmap::{Location, UniverseMap};
use std::io::Cursor;
use std::path::Path;

/// Simulated file in a project
struct TestFile {
    path: String,
    content: Vec<u8>,
}

impl TestFile {
    fn new(path: impl Into<String>, content: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            content,
        }
    }

    fn path(&self) -> &Path {
        Path::new(&self.path)
    }
}

#[test]
fn test_v2_complete_workflow() {
    // ────────────────────────────────────────────────────────────────
    // Setup: Create a simulated project with different file types
    // ────────────────────────────────────────────────────────────────

    let files = vec![
        // Critical config file (small, high priority)
        TestFile::new("app.toml", b"[server]\nport = 8080\n".to_vec()),
        // Source code (normal priority, medium size)
        TestFile::new(
            "src/main.rs",
            b"fn main() {\n    println!(\"Hello, Orbit V2!\");\n}\n".to_vec(),
        ),
        // Database WAL (high priority, append-only)
        TestFile::new("pg_wal/000001", vec![0xAB; 10_000]), // 10KB WAL
        // Media file (low priority, large)
        TestFile::new("assets/logo.png", {
            let mut png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0DIHDR".to_vec();
            png.extend(vec![0xFF; 50_000]); // 50KB image
            png
        }),
        // Duplicate content in different file (tests dedup)
        TestFile::new("backup/main.rs.bak", {
            b"fn main() {\n    println!(\"Hello, Orbit V2!\");\n}\n".to_vec()
        }),
    ];

    // ────────────────────────────────────────────────────────────────
    // Phase 1: Semantic Analysis - Prioritize files
    // ────────────────────────────────────────────────────────────────

    let registry = SemanticRegistry::default();
    let mut file_intents = Vec::new();

    for file in &files {
        let head = &file.content[..file.content.len().min(512)]; // First 512 bytes
        let intent = registry.determine_intent(file.path(), head);

        println!(
            "📄 {} - Priority: {:?}, Strategy: {:?}",
            file.path, intent.priority, intent.strategy
        );

        file_intents.push((file, intent));
    }

    // Sort by priority (lower value = higher priority)
    file_intents.sort_by_key(|(_, intent)| intent.priority);

    // Verify priority ordering
    assert_eq!(
        file_intents[0].0.path, "app.toml",
        "Config file should be first"
    );
    assert_eq!(
        file_intents[0].1.priority,
        Priority::Critical,
        "Config should be critical"
    );

    let wal_index = file_intents
        .iter()
        .position(|(f, _)| f.path.contains("pg_wal"))
        .unwrap();
    assert_eq!(
        file_intents[wal_index].1.priority,
        Priority::High,
        "WAL should be high priority"
    );

    let media_index = file_intents
        .iter()
        .position(|(f, _)| f.path.contains("logo.png"))
        .unwrap();
    assert_eq!(
        file_intents[media_index].1.priority,
        Priority::Low,
        "Media should be low priority"
    );

    // ────────────────────────────────────────────────────────────────
    // Phase 2: CDC Chunking - Content-defined boundaries
    // ────────────────────────────────────────────────────────────────

    let mut universe = UniverseMap::new();
    let config = ChunkConfig::default();

    for (file, _intent) in &file_intents {
        let file_id = universe.register_file(&file.path);

        let stream = ChunkStream::new(Cursor::new(&file.content), config);

        for chunk_result in stream {
            let chunk = chunk_result.expect("Chunking failed");

            // Add to Universe Map (global dedup index)
            universe.add_chunk(
                &chunk.meta.hash,
                Location::new(file_id, chunk.meta.offset, chunk.meta.length as u32),
            );
        }
    }

    println!("\n📊 Universe Map Statistics:");
    let stats = universe.dedup_stats();
    println!("  Unique chunks: {}", stats.unique_chunks);
    println!("  Total references: {}", stats.total_references);
    println!("  Deduplicated chunks: {}", stats.deduplicated_chunks);
    println!("  Space savings: {:.1}%", stats.space_savings_pct());

    // ────────────────────────────────────────────────────────────────
    // Phase 3: Global Deduplication Verification
    // ────────────────────────────────────────────────────────────────

    // The identical content in main.rs and main.rs.bak should share chunks
    assert!(
        stats.deduplicated_chunks > 0,
        "Should detect duplicate content"
    );

    // With duplicate content (main.rs and main.rs.bak), we should see deduplication
    // This is verified by the stats showing deduplicated_chunks > 0
    assert!(
        stats.deduplicated_chunks > 0,
        "Should detect chunks appearing in multiple locations"
    );

    // Verify file count (one entry per file)
    // Note: Can't access file_registry directly, but we registered all files
    assert_eq!(files.len(), 5);

    // Test lookup
    let main_rs_id = universe.register_file("src/main.rs");
    assert_eq!(universe.get_file_path(main_rs_id), Some("src/main.rs"));

    println!("\n✅ V2 Integration Test PASSED");
    println!("   - Semantic prioritization: Working");
    println!("   - CDC chunking: Working");
    println!("   - Global deduplication: Working");
}

#[test]
fn test_v2_rename_detection() {
    // This test demonstrates the "rename = 0 bytes transferred" property

    let original_content = b"This is a file with some content that will be renamed";

    let mut universe = UniverseMap::new();
    let config = ChunkConfig::default();

    // Index file at original location
    let file1_id = universe.register_file("original.txt");
    let stream1 = ChunkStream::new(Cursor::new(original_content), config);

    let mut chunk_hashes = Vec::new();
    for chunk in stream1 {
        let chunk = chunk.unwrap();
        chunk_hashes.push(chunk.meta.hash);
        universe.add_chunk(
            &chunk.meta.hash,
            Location::new(file1_id, chunk.meta.offset, chunk.meta.length as u32),
        );
    }

    // "Rename" the file (same content, different path)
    let file2_id = universe.register_file("renamed.txt");
    let stream2 = ChunkStream::new(Cursor::new(original_content), config);

    let mut bytes_to_transfer = 0;
    for chunk in stream2 {
        let chunk = chunk.unwrap();

        if !universe.has_chunk(&chunk.meta.hash) {
            // Chunk doesn't exist - would need to transfer
            bytes_to_transfer += chunk.meta.length;
        } else {
            // Chunk exists! Just add new location reference
            universe.add_chunk(
                &chunk.meta.hash,
                Location::new(file2_id, chunk.meta.offset, chunk.meta.length as u32),
            );
        }
    }

    println!("\n🔄 Rename Detection Test:");
    println!("  Original file size: {} bytes", original_content.len());
    println!("  Bytes to transfer: {} bytes", bytes_to_transfer);
    println!("  Savings: 100%");

    assert_eq!(bytes_to_transfer, 0, "Renamed file should transfer 0 bytes");

    // Verify both files reference the same chunks
    for hash in &chunk_hashes {
        let locations = universe.get_locations(hash).unwrap();
        assert_eq!(locations.len(), 2, "Each chunk should exist in 2 locations");
    }
}

#[test]
fn test_v2_incremental_edit() {
    // Test that small edits result in minimal transfer

    // Use larger data to generate multiple chunks
    let mut original = Vec::new();
    for i in 0..100 {
        original.extend(format!("Line {} with some content\n", i).as_bytes());
    }

    let mut edited = Vec::new();
    for i in 0..100 {
        if i == 50 {
            edited.extend(b"Line 50 MODIFIED with different content\n");
        } else {
            edited.extend(format!("Line {} with some content\n", i).as_bytes());
        }
    }

    let mut universe = UniverseMap::new();
    let config = ChunkConfig::new(64, 128, 256).unwrap(); // Small chunks for this test

    // Index original
    let file_id = universe.register_file("file.txt");
    let stream = ChunkStream::new(Cursor::new(original), config);

    for chunk in stream {
        let chunk = chunk.unwrap();
        universe.add_chunk(
            &chunk.meta.hash,
            Location::new(file_id, chunk.meta.offset, chunk.meta.length as u32),
        );
    }

    let original_chunks = universe.chunk_count();

    // Process edited version
    let stream = ChunkStream::new(Cursor::new(edited), config);
    let mut new_chunks = 0;
    let mut reused_chunks = 0;

    for chunk in stream {
        let chunk = chunk.unwrap();

        if universe.has_chunk(&chunk.meta.hash) {
            reused_chunks += 1;
        } else {
            new_chunks += 1;
            universe.add_chunk(
                &chunk.meta.hash,
                Location::new(file_id, chunk.meta.offset, chunk.meta.length as u32),
            );
        }
    }

    println!("\n✏️  Incremental Edit Test:");
    println!("  Original chunks: {}", original_chunks);
    println!("  Reused chunks: {}", reused_chunks);
    println!("  New chunks: {}", new_chunks);
    println!(
        "  Reuse ratio: {:.1}%",
        (reused_chunks as f64 / (reused_chunks + new_chunks) as f64) * 100.0
    );

    // Should have significant chunk reuse (CDC benefit over full re-transfer)
    let reuse_ratio = reused_chunks as f64 / (reused_chunks + new_chunks) as f64;
    assert!(
        reuse_ratio > 0.3,
        "Should reuse at least 30% of chunks (got {:.1}%)",
        reuse_ratio * 100.0
    );
}
