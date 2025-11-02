/*!
 * Disk Guardian Example
 *
 * Demonstrates the disk guardian features:
 * - Pre-flight disk space checks with safety margins
 * - Filesystem integrity validation
 * - Optional filesystem watching
 * - Staging area for safe transfers
 */

use std::path::Path;
use orbit::core::disk_guardian::{
    ensure_transfer_safety,
    GuardianConfig,
    create_staging_area,
    estimate_directory_size,
};

#[allow(unused_imports)]
use orbit::core::disk_guardian::DiskWatcher;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Disk Guardian Demo ===\n");

    // Example 1: Basic disk space check with default safety margins
    println!("1. Basic disk space validation:");
    let dest_path = Path::new(".");
    let required_bytes = 1024 * 1024; // 1 MB

    let config = GuardianConfig::default();
    match ensure_transfer_safety(dest_path, required_bytes, &config) {
        Ok(_) => println!("   ✓ Sufficient disk space available (with 10% safety margin)"),
        Err(e) => println!("   ✗ Disk check failed: {}", e),
    }

    // Example 2: Custom safety margins
    println!("\n2. Custom safety margin (25%):");
    let custom_config = GuardianConfig {
        safety_margin_percent: 0.25, // 25% safety margin
        min_free_space: 500 * 1024 * 1024, // 500 MB minimum free
        check_integrity: true,
        enable_watching: false,
    };

    match ensure_transfer_safety(dest_path, required_bytes, &custom_config) {
        Ok(_) => println!("   ✓ Sufficient disk space with 25% safety margin"),
        Err(e) => println!("   ✗ Disk check failed: {}", e),
    }

    // Example 3: Create staging area for safe transfers
    println!("\n3. Creating staging area:");
    match create_staging_area(dest_path) {
        Ok(staging) => {
            println!("   ✓ Staging area created at: {:?}", staging.path());
            println!("   Staging area will be automatically cleaned up on drop");
            // staging is automatically cleaned up when it goes out of scope
        }
        Err(e) => println!("   ✗ Failed to create staging area: {}", e),
    }

    // Example 4: Estimate directory size
    println!("\n4. Estimating directory size:");
    match estimate_directory_size(Path::new("src")) {
        Ok(size) => {
            println!("   ✓ Source directory size: {} bytes", size);
            println!("   ✓ Human-readable: {:.2} MB", size as f64 / (1024.0 * 1024.0));
        }
        Err(e) => println!("   ✗ Failed to estimate size: {}", e),
    }

    // Example 5: Filesystem watcher (commented out as it's optional)
    println!("\n5. Filesystem watching:");
    println!("   (Optional feature - uncomment to enable)");

    /*
    let _watcher = DiskWatcher::new(dest_path, |event| {
        println!("   Filesystem event: {:?}", event);
    })?;

    println!("   ✓ Filesystem watcher active");
    println!("   Monitoring: {:?}", dest_path);

    // Keep the watcher alive for demonstration
    std::thread::sleep(std::time::Duration::from_secs(5));
    */

    // Example 6: Complete pre-flight check workflow
    println!("\n6. Complete pre-flight check workflow:");
    let source_dir = Path::new("src");
    let dest_dir = Path::new("target/orbit_demo");

    match estimate_directory_size(source_dir) {
        Ok(estimated_size) => {
            println!("   Step 1: Estimated size: {} bytes", estimated_size);

            match ensure_transfer_safety(dest_dir, estimated_size, &config) {
                Ok(_) => {
                    println!("   Step 2: Disk space validated ✓");
                    println!("   Step 3: Ready to transfer!");
                }
                Err(e) => {
                    println!("   Step 2: Pre-flight check failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("   Step 1: Failed to estimate size: {}", e);
        }
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}
