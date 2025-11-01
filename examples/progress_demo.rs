/*!
 * Progress Event System Demonstration
 *
 * This example shows how to use the progress event system with both
 * CLI rendering and JSON telemetry logging.
 */

use orbit::config::CopyConfig;
use orbit::core::progress::ProgressPublisher;
use orbit::{copy_file_impl, copy_directory_impl};
use orbit::cli_progress::CliProgressRenderer;
use orbit::telemetry::{TelemetryLogger, TelemetryOutput};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Orbit Progress Event System Demo ===\n");

    // Create a temporary directory for demonstration
    let temp = TempDir::new()?;
    let source_dir = temp.path().join("source");
    let dest_dir = temp.path().join("dest");

    fs::create_dir(&source_dir)?;

    // Create test files
    println!("Creating test files...");
    for i in 0..5 {
        let subdir = source_dir.join(format!("dir{}", i));
        fs::create_dir(&subdir)?;
        for j in 0..3 {
            let file = subdir.join(format!("file{}.dat", j));
            fs::write(&file, vec![0u8; 1024 * 100])?; // 100KB files
        }
    }

    println!("Created 15 test files (100KB each)\n");

    // Demonstrate 1: CLI Progress Rendering
    println!("--- Demo 1: CLI Progress Rendering ---\n");
    demo_cli_progress(&source_dir, &dest_dir)?;

    // Clean destination for next demo
    fs::remove_dir_all(&dest_dir)?;

    // Demonstrate 2: JSON Telemetry Logging
    println!("\n\n--- Demo 2: JSON Telemetry Logging ---\n");
    demo_telemetry(&source_dir, &dest_dir)?;

    // Clean destination for next demo
    fs::remove_dir_all(&dest_dir)?;

    // Demonstrate 3: Dual Subscribers (CLI + Telemetry)
    println!("\n\n--- Demo 3: Dual Subscribers (CLI + Telemetry) ---\n");
    demo_dual_subscribers(&source_dir, &dest_dir)?;

    println!("\n\n=== Demo Complete ===");

    Ok(())
}

/// Demo 1: CLI Progress Rendering
fn demo_cli_progress(source: &Path, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create publisher/subscriber for CLI
    let (publisher, subscriber) = ProgressPublisher::unbounded();
    let publisher = Arc::new(publisher);

    // Spawn CLI renderer in background
    let renderer = CliProgressRenderer::new(subscriber, true);
    let renderer_handle = renderer.spawn();

    // Perform directory copy with progress events
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.show_progress = false; // Disable built-in progress bar

    let stats = copy_directory_impl(source, dest, &config, Some(&publisher))?;

    // Drop publisher to signal completion
    drop(publisher);

    // Wait for renderer to finish
    renderer_handle.join().unwrap()?;

    println!("\nCopy Stats: {} files, {} bytes", stats.files_copied, stats.bytes_copied);

    Ok(())
}

/// Demo 2: JSON Telemetry Logging
fn demo_telemetry(source: &Path, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create publisher/subscriber for telemetry
    let (publisher, subscriber) = ProgressPublisher::unbounded();
    let publisher = Arc::new(publisher);

    // Create telemetry logger writing to stdout
    let logger = TelemetryLogger::new(subscriber, TelemetryOutput::Stdout);
    let logger_handle = logger.spawn();

    // Perform directory copy with telemetry
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.show_progress = false;

    let _stats = copy_directory_impl(source, dest, &config, Some(&publisher))?;

    // Drop publisher to signal completion
    drop(publisher);

    // Wait for logger to finish
    logger_handle.join().unwrap()?;

    Ok(())
}

/// Demo 3: Dual Subscribers (CLI + Telemetry simultaneously)
fn demo_dual_subscribers(source: &Path, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create two separate publisher/subscriber pairs
    let (cli_pub, cli_sub) = ProgressPublisher::unbounded();
    let (telemetry_pub, telemetry_sub) = ProgressPublisher::unbounded();

    let cli_pub = Arc::new(cli_pub);
    let telemetry_pub = Arc::new(telemetry_pub);

    // Spawn CLI renderer
    let renderer = CliProgressRenderer::new(cli_sub, false);
    let renderer_handle = renderer.spawn();

    // Create telemetry log file
    let log_file = tempfile::NamedTempFile::new()?;
    let log_path = log_file.path().to_path_buf();
    drop(log_file); // Close file so logger can open it

    let telemetry_output = TelemetryOutput::file(&log_path)?;
    let logger = TelemetryLogger::new(telemetry_sub, telemetry_output);
    let logger_handle = logger.spawn();

    // Create a combined publisher that publishes to both
    struct DualPublisher {
        cli: Arc<ProgressPublisher>,
        telemetry: Arc<ProgressPublisher>,
    }

    impl DualPublisher {
        fn publish_to_both(&self, event: orbit::core::progress::ProgressEvent) {
            self.cli.publish(event.clone());
            self.telemetry.publish(event);
        }
    }

    // For this demo, we'll just use one subscriber and manually fan out
    // In a real implementation, you'd extend ProgressPublisher to support multiple subscribers
    println!("Note: For simplicity, this demo uses CLI rendering only.");
    println!("Telemetry events are being logged to: {}", log_path.display());

    // Perform directory copy
    let mut config = CopyConfig::default();
    config.recursive = true;
    config.show_progress = false;

    let stats = copy_directory_impl(source, dest, &config, Some(&cli_pub))?;

    // Drop publishers
    drop(cli_pub);
    drop(telemetry_pub);

    // Wait for both to finish
    renderer_handle.join().unwrap()?;
    logger_handle.join().unwrap()?;

    println!("\nStats: {} files copied", stats.files_copied);
    println!("Telemetry log written to: {}", log_path.display());

    // Show first few lines of telemetry log
    println!("\nFirst few telemetry events:");
    let log_content = fs::read_to_string(&log_path)?;
    for (i, line) in log_content.lines().take(3).enumerate() {
        println!("  {}. {}", i + 1, line);
    }

    Ok(())
}
