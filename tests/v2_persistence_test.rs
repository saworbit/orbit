//! V2 Verification Suite: Stage 4 (Persistence)
//!
//! Validates:
//! 1. Universe DB creates/opens correctly.
//! 2. Data survives a "restart" (drop & re-open).
//! 3. Serialization/Deserialization works.

#[cfg(test)]
mod tests {
    use orbit_core_starmap::{ChunkLocation, Universe};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn verify_universe_persistence() {
        println!("\n🧪 Starting Persistence Test...");

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("universe.db");

        // Mock Data
        let hash = [1u8; 32]; // Fake Hash
        let location = ChunkLocation {
            path: PathBuf::from("/data/video.mp4"),
            offset: 1024,
            length: 4096,
        };

        // 1. RUN 1: Open and Insert
        {
            println!("   [Run 1] Opening DB and inserting chunk...");
            let universe = Universe::open(&db_path).expect("Failed to create DB");
            universe
                .insert_chunk(hash, location.clone())
                .expect("Insert failed");
            // universe is dropped here (simulating app close)
        }

        // 2. RUN 2: Re-open and Verify
        {
            println!("   [Run 2] Re-opening DB...");
            let universe = Universe::open(&db_path).expect("Failed to re-open DB");

            let found = universe.find_chunk(&hash).expect("Read failed");

            match found {
                Some(locations) => {
                    println!("   ✅ Found {} location(s)", locations.len());
                    assert_eq!(locations.len(), 1);
                    let loc = &locations[0];
                    println!("   ✅ Found Chunk at: {:?}", loc.path);
                    assert_eq!(loc.path, location.path);
                    assert_eq!(loc.offset, location.offset);
                    assert_eq!(loc.length, location.length);
                }
                None => panic!("❌ Chunk vanished after restart! Persistence failed."),
            }
        }

        println!("   ✅ PASS: Data survived restart!");
        println!("   ✅ PASS: Persistence verified!");
    }

    #[test]
    fn test_multiple_locations() {
        println!("\n🧪 Testing Multiple Locations...");

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("universe_multi.db");
        let hash = [0x42; 32];

        {
            let universe = Universe::open(&db_path).expect("Failed to create DB");

            // Insert same chunk at different locations
            universe
                .insert_chunk(
                    hash,
                    ChunkLocation::new(PathBuf::from("/file1.txt"), 0, 4096),
                )
                .expect("Insert failed");

            universe
                .insert_chunk(
                    hash,
                    ChunkLocation::new(PathBuf::from("/file2.txt"), 1024, 4096),
                )
                .expect("Insert failed");

            universe
                .insert_chunk(
                    hash,
                    ChunkLocation::new(PathBuf::from("/file3.txt"), 2048, 4096),
                )
                .expect("Insert failed");
        }

        // Verify after restart
        {
            let universe = Universe::open(&db_path).expect("Failed to re-open DB");
            let locations = universe
                .find_chunk(&hash)
                .expect("Read failed")
                .expect("Chunk not found");

            assert_eq!(locations.len(), 3);
            println!("   ✅ Found {} locations after restart", locations.len());
        }
    }

    #[test]
    fn test_has_chunk() {
        println!("\n🧪 Testing has_chunk()...");

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("universe_has.db");

        let existing_hash = [0xAA; 32];
        let missing_hash = [0xBB; 32];

        {
            let universe = Universe::open(&db_path).expect("Failed to create DB");

            universe
                .insert_chunk(
                    existing_hash,
                    ChunkLocation::new(PathBuf::from("/test.bin"), 0, 1024),
                )
                .expect("Insert failed");
        }

        // Verify after restart
        {
            let universe = Universe::open(&db_path).expect("Failed to re-open DB");

            assert!(
                universe
                    .has_chunk(&existing_hash)
                    .expect("has_chunk failed"),
                "Should have existing chunk"
            );
            assert!(
                !universe.has_chunk(&missing_hash).expect("has_chunk failed"),
                "Should not have missing chunk"
            );

            println!("   ✅ has_chunk() works correctly");
        }
    }

    #[test]
    fn test_empty_database() {
        println!("\n🧪 Testing Empty Database...");

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("universe_empty.db");

        let universe = Universe::open(&db_path).expect("Failed to create DB");

        let result = universe.find_chunk(&[0x00; 32]).expect("Read failed");
        assert!(result.is_none(), "Empty DB should return None");

        assert!(
            !universe.has_chunk(&[0x00; 32]).expect("has_chunk failed"),
            "Empty DB should not have any chunks"
        );

        println!("   ✅ Empty database behaves correctly");
    }
}
