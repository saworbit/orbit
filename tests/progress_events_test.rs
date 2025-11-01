use orbit::config::CopyConfig;
use orbit::core::progress::{ProgressPublisher, ProgressEvent};
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::TempDir;

#[test]
fn test_progress_events_file_copy() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.txt");
    let dest = temp.path().join("dest.txt");

    // Create test file
    fs::write(&source, vec![0u8; 1024 * 100]).unwrap(); // 100KB file

    // Create publisher/subscriber
    let (publisher, subscriber) = ProgressPublisher::unbounded();
    let publisher = Arc::new(publisher);

    // Collect events in background thread
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let handle = thread::spawn(move || {
        for event in subscriber.receiver().iter() {
            events_clone.lock().unwrap().push(event);
        }
    });

    // Perform copy with progress events
    let config = CopyConfig::default();

    // Access internal implementation (for testing purposes)
    // In real usage, the CLI would provide the publisher
    use orbit::core::copy_file_impl;
    let result = copy_file_impl(&source, &dest, &config, Some(&publisher));

    assert!(result.is_ok(), "Copy should succeed");

    // Drop publisher to signal completion
    drop(publisher);
    handle.join().unwrap();

    // Verify events were emitted
    let collected_events = events.lock().unwrap();
    println!("Collected {} events", collected_events.len());

    // Should have at least TransferStart and TransferComplete
    assert!(collected_events.len() >= 2, "Should have start and complete events");

    // Verify event types
    let has_start = collected_events.iter().any(|e| matches!(e, ProgressEvent::TransferStart { .. }));
    let has_complete = collected_events.iter().any(|e| matches!(e, ProgressEvent::TransferComplete { .. }));

    assert!(has_start, "Should have TransferStart event");
    assert!(has_complete, "Should have TransferComplete event");

    // Print event summary
    for event in collected_events.iter() {
        match event {
            ProgressEvent::TransferStart { file_id, total_bytes, .. } => {
                println!("✓ TransferStart: {} bytes for {}", total_bytes, file_id.as_str());
            }
            ProgressEvent::TransferProgress { bytes_transferred, total_bytes, .. } => {
                println!("  Progress: {}/{} bytes", bytes_transferred, total_bytes);
            }
            ProgressEvent::TransferComplete { total_bytes, duration_ms, .. } => {
                println!("✓ TransferComplete: {} bytes in {}ms", total_bytes, duration_ms);
            }
            _ => {}
        }
    }
}

#[test]
fn test_progress_events_directory_copy() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let dest_dir = temp.path().join("dest");

    fs::create_dir(&source_dir).unwrap();

    // Create test files
    for i in 0..10 {
        fs::write(source_dir.join(format!("file{}.txt", i)), vec![0u8; 1024]).unwrap();
    }

    // Create publisher/subscriber
    let (publisher, subscriber) = ProgressPublisher::unbounded();
    let publisher = Arc::new(publisher);

    // Collect events
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let handle = thread::spawn(move || {
        for event in subscriber.receiver().iter() {
            events_clone.lock().unwrap().push(event);
        }
    });

    // Perform directory copy
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.show_progress = false;

    use orbit::core::copy_directory_impl;
    let result = copy_directory_impl(&source_dir, &dest_dir, &config, Some(&publisher));

    assert!(result.is_ok(), "Directory copy should succeed");

    drop(publisher);
    handle.join().unwrap();

    let collected_events = events.lock().unwrap();
    println!("Collected {} directory events", collected_events.len());

    // Should have scan start, complete, and batch complete
    let has_scan_start = collected_events.iter().any(|e| matches!(e, ProgressEvent::DirectoryScanStart { .. }));
    let has_scan_complete = collected_events.iter().any(|e| matches!(e, ProgressEvent::DirectoryScanComplete { .. }));
    let has_batch_complete = collected_events.iter().any(|e| matches!(e, ProgressEvent::BatchComplete { .. }));

    assert!(has_scan_start, "Should have DirectoryScanStart event");
    assert!(has_scan_complete, "Should have DirectoryScanComplete event");
    assert!(has_batch_complete, "Should have BatchComplete event");

    println!("✓ Directory copy emitted all expected events");
}
