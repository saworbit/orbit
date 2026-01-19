/// Orbit Audit Logging Demo
///
/// This example demonstrates the V3 Unified Observability system with
/// cryptographic audit logging and distributed tracing.
///
/// Run with:
///   export ORBIT_AUDIT_SECRET="your-secret-key-here"
///   cargo run --example audit_logging_demo --features backend-abstraction
use orbit::config::CopyConfig;
use orbit::logging::init_logging;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary directories for testing
    let temp_dir = TempDir::new()?;
    let audit_log = temp_dir.path().join("audit.jsonl");

    println!("===========================================");
    println!("Orbit V3 Audit Logging Demo");
    println!("===========================================");
    println!();
    println!("Audit log location: {:?}", audit_log);
    println!();

    // Check if ORBIT_AUDIT_SECRET is set
    if std::env::var("ORBIT_AUDIT_SECRET").is_err() {
        println!("⚠️  WARNING: ORBIT_AUDIT_SECRET not set!");
        println!("   Audit logging will be disabled.");
        println!("   Set it with: export ORBIT_AUDIT_SECRET='your-secret-key'");
        println!();
    } else {
        println!("✓ ORBIT_AUDIT_SECRET detected");
        println!("✓ Cryptographic audit chaining enabled");
        println!();
    }

    // Configure logging with audit trail
    let config = CopyConfig {
        audit_log_path: Some(audit_log.clone()),
        verbose: true,
        ..Default::default()
    };

    println!("Initializing logging system...");
    init_logging(&config)?;
    println!("✓ Logging initialized");
    println!();

    // Demonstrate tracing with structured events
    println!("Emitting sample events with W3C Trace Context...");
    println!();

    // Simulate a file transfer job
    let job_span = tracing::info_span!("file_transfer", job_id = "demo-job-001");
    let _guard = job_span.enter();

    tracing::info!(
        files = 3,
        total_bytes = 1024,
        protocol = "local",
        "Starting file transfer job"
    );

    // Simulate file operations
    for i in 1..=3 {
        let file_span = tracing::info_span!(
            "file_operation",
            file_id = format!("file-{}", i),
            operation = "copy"
        );
        let _file_guard = file_span.enter();

        tracing::info!(
            source = format!("/source/file{}.txt", i),
            destination = format!("/dest/file{}.txt", i),
            bytes = 1024,
            "File transfer started"
        );

        // Simulate some work
        std::thread::sleep(std::time::Duration::from_millis(10));

        tracing::info!(
            bytes_transferred = 1024,
            duration_ms = 10,
            checksum = format!("abc123{}", i),
            "File transfer completed"
        );
    }

    tracing::info!(
        duration_ms = 100,
        digest = "final-checksum-abc123",
        "Job completed successfully"
    );

    drop(_guard);
    println!();

    // Check if audit log was created
    if audit_log.exists() {
        let content = std::fs::read_to_string(&audit_log)?;
        let line_count = content.lines().count();

        println!("===========================================");
        println!("✓ Audit Log Generated");
        println!("===========================================");
        println!("Location: {:?}", audit_log);
        println!("Records:  {}", line_count);
        println!();

        // Show first record
        if let Some(first_line) = content.lines().next() {
            println!("First record (formatted):");
            println!("{}", first_line);
            println!();

            // Parse and show structure
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(first_line) {
                println!("Event structure:");
                println!(
                    "  trace_id:       {}",
                    event["trace_id"].as_str().unwrap_or("N/A")
                );
                println!(
                    "  span_id:        {}",
                    event["span_id"].as_str().unwrap_or("N/A")
                );
                println!(
                    "  sequence:       {}",
                    event["sequence"].as_u64().unwrap_or(0)
                );
                println!(
                    "  integrity_hash: {}...",
                    event["integrity_hash"]
                        .as_str()
                        .unwrap_or("N/A")
                        .chars()
                        .take(16)
                        .collect::<String>()
                );
                println!(
                    "  timestamp:      {}",
                    event["timestamp"].as_str().unwrap_or("N/A")
                );
                println!();
            }
        }

        println!("===========================================");
        println!("Verification Instructions");
        println!("===========================================");
        println!();
        println!("1. Verify the audit log integrity:");
        println!("   export ORBIT_AUDIT_SECRET='your-secret-key'");
        println!("   python3 scripts/verify_audit.py {:?}", audit_log);
        println!();
        println!("2. Try tampering with the log:");
        println!("   sed -i 's/2025/2026/' {:?}", audit_log);
        println!("   python3 scripts/verify_audit.py {:?}", audit_log);
        println!("   (Should detect tampering!)");
        println!();
        println!("3. View with OpenTelemetry (if configured):");
        println!("   Set --otel-endpoint http://jaeger:4317");
        println!("   View traces at http://localhost:16686");
        println!();
    } else {
        println!("⚠️  No audit log created");
        println!("   This may be because ORBIT_AUDIT_SECRET is not set");
    }

    println!("===========================================");
    println!("Demo Complete!");
    println!("===========================================");

    Ok(())
}
