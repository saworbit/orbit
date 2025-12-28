use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};

// A request for a specific chunk of data
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BlockRequest {
    pub file_id: String,
    pub block_index: u64,
}

pub struct Entangler {
    // Communication pipe to the Wormhole Transport logic
    priority_tx: Sender<BlockRequest>,

    // Notification mechanism: Waiters waiting for a specific block
    // Key: (FileID, BlockIndex), Value: Condvar to wake up the FUSE thread
    waiting_rooms: Arc<Mutex<HashMap<BlockRequest, Arc<Condvar>>>>,
}

impl Entangler {
    pub fn new(priority_tx: Sender<BlockRequest>) -> Self {
        Self {
            priority_tx,
            waiting_rooms: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Called by FUSE read()
    /// Returns immediately if data exists, otherwise blocks until downloaded.
    pub fn ensure_block_available(&self, file_id: &str, block_index: u64) {
        let req = BlockRequest {
            file_id: file_id.to_string(),
            block_index,
        };

        // 1. Check if we need to request it (Simulation: check if file exists on disk)
        if self.is_block_on_disk(file_id, block_index) {
            return;
        }

        // 2. Setup the waiting room (Condvar)
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let waiter = pair.1.clone();

        {
            let mut rooms = self.waiting_rooms.lock().unwrap();
            // If multiple threads read the same missing block, they share the condvar
            rooms.entry(req.clone()).or_insert(Arc::new(Condvar::new()));
        }

        // 3. Send High-Priority Signal to Wormhole
        println!(
            "[Orbit-Ghost] 🚀 Quantum Request: {} Block {}",
            file_id, block_index
        );
        let _ = self.priority_tx.send(req.clone());

        // 4. Block thread until the Transport layer notifies us
        // In a real implementation, we wait on the Condvar.
        // For this demo, we'll simulate a blocking wait for the file to appear.
        loop {
            if self.is_block_on_disk(file_id, block_index) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn is_block_on_disk(&self, file_id: &str, block_index: u64) -> bool {
        // In reality, check the block bitmap or cache directory
        let path = format!("/tmp/orbit_cache/{}_{}.bin", file_id, block_index);
        std::path::Path::new(&path).exists()
    }
}
