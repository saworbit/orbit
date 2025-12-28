//! Integration tests for orbit-connect
//!
//! These tests verify the RemoteSystem and StarManager functionality.
//! They require a running orbit-star instance for full integration testing.

use orbit_connect::{StarManager, StarRecord, StarStatus};

#[tokio::test]
async fn test_star_manager_registration() {
    let manager = StarManager::new();

    let star = StarRecord::new(
        "test-star-1".to_string(),
        "http://localhost:50051".to_string(),
        "test-token-123".to_string(),
    );

    manager.register(star).await;

    let stars = manager.list_stars().await;
    assert_eq!(stars.len(), 1);
    assert_eq!(stars[0].id, "test-star-1");
    assert_eq!(stars[0].status, StarStatus::Registered);
}

#[tokio::test]
async fn test_star_manager_unregister() {
    let manager = StarManager::new();

    let star = StarRecord::new(
        "test-star-2".to_string(),
        "http://localhost:50052".to_string(),
        "test-token-456".to_string(),
    );

    manager.register(star).await;
    assert_eq!(manager.list_stars().await.len(), 1);

    manager.unregister("test-star-2").await.unwrap();
    assert_eq!(manager.list_stars().await.len(), 0);
}

#[tokio::test]
async fn test_star_manager_disconnect() {
    let manager = StarManager::new();

    let star = StarRecord::new(
        "test-star-3".to_string(),
        "http://localhost:50053".to_string(),
        "test-token-789".to_string(),
    );

    manager.register(star).await;

    // Initially not connected
    assert!(!manager.is_connected("test-star-3").await);

    // Disconnect (should be a no-op if not connected)
    manager.disconnect("test-star-3").await;

    // Verify still in registry
    assert_eq!(manager.list_stars().await.len(), 1);
}

#[tokio::test]
async fn test_star_record_with_name() {
    let star = StarRecord::with_name(
        "test-star-4".to_string(),
        "http://localhost:50054".to_string(),
        "test-token-abc".to_string(),
        "My Test Star".to_string(),
    );

    assert_eq!(star.id, "test-star-4");
    assert_eq!(star.name, Some("My Test Star".to_string()));
    assert_eq!(star.status, StarStatus::Registered);
}

// NOTE: The following tests require a running orbit-star instance and are disabled by default
// To run them, start an orbit-star instance and remove the #[ignore] attribute

#[tokio::test]
#[ignore]
async fn test_remote_system_connection() {
    // This test requires:
    // 1. orbit-star running on localhost:50051
    // 2. A valid test token configured in the star
    // 3. Test data files available

    use std::path::Path;

    let manager = StarManager::new();

    let star = StarRecord::new(
        "live-test-star".to_string(),
        "http://localhost:50051".to_string(),
        "your-test-token-here".to_string(),
    );

    manager.register(star).await;

    // Attempt to connect
    let system = manager.get_system("live-test-star").await;

    match system {
        Ok(sys) => {
            // Test basic operations
            let test_path = Path::new("/test");
            let exists = sys.exists(test_path).await;
            println!("Test path exists: {}", exists);
        }
        Err(e) => {
            eprintln!("Failed to connect to Star: {}", e);
            panic!("Connection failed: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_remote_hash_calculation() {
    // This test requires:
    // 1. orbit-star running on localhost:50051
    // 2. A test file at /data/test.bin on the Star

    use std::path::Path;

    let manager = StarManager::new();

    let star = StarRecord::new(
        "hash-test-star".to_string(),
        "http://localhost:50051".to_string(),
        "your-test-token-here".to_string(),
    );

    manager.register(star).await;

    let system = manager.get_system("hash-test-star").await.unwrap();

    // Calculate hash of first 1024 bytes
    let hash = system
        .calculate_hash(Path::new("/data/test.bin"), 0, 1024)
        .await;

    match hash {
        Ok(h) => {
            println!("Hash calculated: {}", hex::encode(h));
            assert_eq!(h.len(), 32); // BLAKE3 hash is 32 bytes
        }
        Err(e) => {
            eprintln!("Hash calculation failed: {}", e);
            panic!("Hash calculation failed: {}", e);
        }
    }
}
