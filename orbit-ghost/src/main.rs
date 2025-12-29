mod adapter;
mod config;
mod entangler;
mod error;
mod fs;
mod inode;
mod oracle;
mod translator;

use adapter::MagnetarAdapter;
use clap::Parser;
use crossbeam_channel::unbounded;
use entangler::{BlockRequest, Entangler};
use fs::OrbitGhostFS;
use std::sync::Arc;
use std::thread;
use translator::InodeTranslator;

#[derive(Parser)]
#[clap(
    name = "orbit-ghost",
    about = "FUSE filesystem with on-demand block fetching from Magnetar database"
)]
struct Cli {
    /// Path to Magnetar database
    #[clap(short, long, default_value = "magnetar.db")]
    database: String,

    /// Job ID to mount
    #[clap(short, long)]
    job_id: i64,

    /// Mount point directory
    #[clap(short, long, default_value = "/tmp/orbit_ghost_mount")]
    mount_point: String,

    /// Cache directory for downloaded blocks
    #[clap(short, long, default_value = "/tmp/orbit_cache")]
    cache_dir: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Cli::parse();

    log::info!("Orbit GhostFS Phase 2 - Materialization Layer");
    log::info!("Database: {}", args.database);
    log::info!("Job ID: {}", args.job_id);

    // 1. Setup Environment
    let _ = std::fs::remove_dir_all(&args.mount_point);
    std::fs::create_dir_all(&args.mount_point)?;
    std::fs::create_dir_all(&args.cache_dir)?;

    // 2. Initialize Magnetar Adapter
    log::info!("Connecting to Magnetar database...");
    let adapter = MagnetarAdapter::new(&args.database, args.job_id).await?;

    // Verify root artifact exists
    let root_id = adapter.get_root_id().await?;
    log::info!("Root artifact ID: {}", root_id);

    // 3. Create InodeTranslator
    let translator = Arc::new(InodeTranslator::new());

    // 4. Setup Channels for Entangler
    let (priority_tx, priority_rx) = unbounded::<BlockRequest>();
    let entangler = Arc::new(Entangler::new(priority_tx));

    // 5. Start the Wormhole Transport (Background Thread)
    let cache_dir_clone = args.cache_dir.clone();
    thread::spawn(move || {
        log::info!("Wormhole transport layer active");

        loop {
            // LISTEN for High Priority first (Quantum Mode)
            if let Ok(req) = priority_rx.recv() {
                log::debug!(
                    "Priority request for block {} of file {}",
                    req.block_index,
                    req.file_id
                );

                // Simulate Network Latency
                thread::sleep(std::time::Duration::from_millis(500));

                // Generate Fake Data (Simulate Download)
                let path = format!(
                    "{}/{}_{}.bin",
                    cache_dir_clone, req.file_id, req.block_index
                );
                let data = vec![0u8; 1024 * 1024]; // 1MB dummy data
                if let Err(e) = std::fs::write(&path, data) {
                    log::error!("Failed to write block {}: {}", req.block_index, e);
                } else {
                    log::debug!("Block {} downloaded & cached", req.block_index);
                }
            }
        }
    });

    // 6. Get runtime handle BEFORE blocking on FUSE mount
    let handle = tokio::runtime::Handle::current();

    // 7. Create Filesystem
    let fs = OrbitGhostFS::new(
        Arc::new(adapter),
        translator,
        entangler,
        handle,
        args.cache_dir.clone(),
    );

    // 8. Mount FUSE (blocks forever)
    log::info!("Mounting filesystem at {}", args.mount_point);
    let options = vec![
        fuser::MountOption::RO,
        fuser::MountOption::FSName("orbit_ghost".to_string()),
        fuser::MountOption::AutoUnmount,
    ];

    fuser::mount2(fs, &args.mount_point, &options)?;
    Ok(())
}
