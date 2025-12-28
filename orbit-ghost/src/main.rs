mod entangler;
mod fs;
mod inode;

use crate::entangler::{BlockRequest, Entangler};
use crate::fs::OrbitGhostFS;
use crate::inode::GhostFile;
use crossbeam_channel::unbounded;
use dashmap::DashMap;
use std::sync::Arc;
use std::thread;

const MOUNT_POINT: &str = "/tmp/orbit_ghost_mount";
const CACHE_DIR: &str = "/tmp/orbit_cache";

fn main() {
    env_logger::init();

    // 1. Setup Environment
    let _ = std::fs::remove_dir_all(MOUNT_POINT);
    let _ = std::fs::create_dir_all(MOUNT_POINT);
    let _ = std::fs::create_dir_all(CACHE_DIR);

    // 2. Load Manifest (Simulated)
    let inodes = Arc::new(DashMap::new());
    inodes.insert(
        2,
        GhostFile {
            name: "visionary_demo.mp4".to_string(),
            size: 50 * 1024 * 1024, // 50MB
            orbit_id: "file_123".to_string(),
            is_dir: false,
            blocks_present: vec![],
        },
    );

    // 3. Setup Channels
    let (priority_tx, priority_rx) = unbounded::<BlockRequest>();
    let entangler = Arc::new(Entangler::new(priority_tx));

    // 4. Start the Wormhole Transport (Background Thread)
    thread::spawn(move || {
        println!("[Wormhole] Transport Layer Active.");

        loop {
            // LISTEN for High Priority first (Quantum Mode)
            if let Ok(req) = priority_rx.recv() {
                println!(
                    "[Wormhole] ⚡ Intercepted PRIORITY request for Block {}",
                    req.block_index
                );

                // Simulate Network Latency
                thread::sleep(std::time::Duration::from_millis(500));

                // Generate Fake Data (Simulate Download)
                let path = format!("{}/{}_{}.bin", CACHE_DIR, req.file_id, req.block_index);
                let data = vec![0u8; 1024 * 1024]; // 1MB dummy data
                std::fs::write(path, data).unwrap();

                println!(
                    "[Wormhole] ✅ Block {} Downloaded & Cached.",
                    req.block_index
                );
            }
        }
    });

    // 5. Mount FUSE
    let fs = OrbitGhostFS {
        inodes,
        entangler,
        cache_path: CACHE_DIR.to_string(),
    };

    println!(
        "[Orbit] 🌌 Projecting Holographic Filesystem at {}",
        MOUNT_POINT
    );
    let options = vec![
        fuser::MountOption::RO,
        fuser::MountOption::FSName("orbit_ghost".to_string()),
        fuser::MountOption::AutoUnmount,
    ];

    fuser::mount2(fs, MOUNT_POINT, &options).unwrap();
}
