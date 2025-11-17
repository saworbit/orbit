//! Progress registry for tracking job progress across connections

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::types::ProgressUpdate;

pub type ProgressSender = broadcast::Sender<ProgressUpdate>;
pub type ProgressReceiver = broadcast::Receiver<ProgressUpdate>;

/// Global registry for managing progress broadcasts per job
#[derive(Clone)]
pub struct ProgressRegistry {
    inner: Arc<RwLock<HashMap<String, ProgressSender>>>,
}

impl Default for ProgressRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressRegistry {
    /// Create a new progress registry
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new broadcast channel for a job
    pub async fn create(&self, job_id: String) -> ProgressSender {
        let (tx, _) = broadcast::channel(100);
        self.inner.write().await.insert(job_id, tx.clone());
        tx
    }

    /// Get the sender for a job
    pub async fn get(&self, job_id: &str) -> Option<ProgressSender> {
        self.inner.read().await.get(job_id).cloned()
    }

    /// Subscribe to progress updates for a job
    pub async fn subscribe(&self, job_id: &str) -> Option<ProgressReceiver> {
        self.inner.read().await.get(job_id).map(|tx| tx.subscribe())
    }

    /// Remove a job from the registry
    pub async fn remove(&self, job_id: &str) -> bool {
        self.inner.write().await.remove(job_id).is_some()
    }

    /// Get all active job IDs
    pub async fn active_jobs(&self) -> Vec<String> {
        self.inner.read().await.keys().cloned().collect()
    }
}
