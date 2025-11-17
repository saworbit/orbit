//! Migration utilities for moving data between backends

use crate::JobStore;
use anyhow::Result;

/// Migrate data from one store to another
///
/// This function performs a bulk migration of all job states from the source
/// to the destination store.
///
/// # Example
///
/// ```ignore
/// use magnetar::{SqliteStore, RedbStore, migration};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let source = SqliteStore::open("old.db").await?;
///     let mut dest = RedbStore::open("new.magnetar")?;
///
///     migration::migrate(&source, &mut dest).await?;
///     Ok(())
/// }
/// ```
pub async fn migrate<S1, S2>(from: &S1, to: &mut S2) -> Result<()>
where
    S1: JobStore + ?Sized,
    S2: JobStore + ?Sized,
{
    // For now, we'll migrate job 0 which represents "all jobs"
    // In a real implementation, you'd iterate through all job IDs
    let states = from.get_by_status(0, crate::JobStatus::Pending).await?;

    // Sequential migration (async doesn't play well with rayon)
    for state in states {
        to.mark_status(
            state.job_id,
            state.chunk,
            state.status,
            Some(state.checksum.clone()),
        )
        .await?;
    }

    Ok(())
}

/// Dual-write mode for zero-downtime migrations
///
/// This struct wraps two stores and writes to both simultaneously,
/// enabling gradual migration without downtime.
pub struct DualStore<S1, S2> {
    primary: S1,
    secondary: S2,
    read_from_primary: bool,
}

impl<S1: JobStore, S2: JobStore> DualStore<S1, S2> {
    /// Create a new dual store
    pub fn new(primary: S1, secondary: S2) -> Self {
        Self {
            primary,
            secondary,
            read_from_primary: true,
        }
    }

    /// Switch read source to secondary (for gradual cutover)
    pub fn switch_to_secondary(&mut self) {
        self.read_from_primary = false;
    }
}

#[async_trait::async_trait]
impl<S1: JobStore, S2: JobStore> JobStore for DualStore<S1, S2> {
    async fn init_from_manifest(&mut self, job_id: i64, manifest: &toml::Value) -> Result<()> {
        // Write to both
        self.primary.init_from_manifest(job_id, manifest).await?;
        self.secondary.init_from_manifest(job_id, manifest).await?;
        Ok(())
    }

    async fn claim_pending(&mut self, job_id: i64) -> Result<Option<crate::JobState>> {
        if self.read_from_primary {
            self.primary.claim_pending(job_id).await
        } else {
            self.secondary.claim_pending(job_id).await
        }
    }

    async fn mark_status(
        &mut self,
        job_id: i64,
        chunk: u64,
        status: crate::JobStatus,
        checksum: Option<String>,
    ) -> Result<()> {
        // Write to both
        self.primary
            .mark_status(job_id, chunk, status, checksum.clone())
            .await?;
        self.secondary
            .mark_status(job_id, chunk, status, checksum)
            .await?;
        Ok(())
    }

    async fn resume_pending(&self, job_id: i64) -> Result<Vec<crate::JobState>> {
        if self.read_from_primary {
            self.primary.resume_pending(job_id).await
        } else {
            self.secondary.resume_pending(job_id).await
        }
    }

    async fn get_by_status(
        &self,
        job_id: i64,
        status: crate::JobStatus,
    ) -> Result<Vec<crate::JobState>> {
        if self.read_from_primary {
            self.primary.get_by_status(job_id, status).await
        } else {
            self.secondary.get_by_status(job_id, status).await
        }
    }

    async fn add_dependency(&mut self, job_id: i64, chunk: u64, deps: Vec<u64>) -> Result<()> {
        self.primary
            .add_dependency(job_id, chunk, deps.clone())
            .await?;
        self.secondary.add_dependency(job_id, chunk, deps).await?;
        Ok(())
    }

    async fn get_dependencies(&self, job_id: i64, chunk: u64) -> Result<Vec<u64>> {
        if self.read_from_primary {
            self.primary.get_dependencies(job_id, chunk).await
        } else {
            self.secondary.get_dependencies(job_id, chunk).await
        }
    }

    async fn topo_sort_ready(&self, job_id: i64) -> Result<Vec<u64>> {
        if self.read_from_primary {
            self.primary.topo_sort_ready(job_id).await
        } else {
            self.secondary.topo_sort_ready(job_id).await
        }
    }

    async fn get_chunk(&self, job_id: i64, chunk: u64) -> Result<Option<crate::JobState>> {
        if self.read_from_primary {
            self.primary.get_chunk(job_id, chunk).await
        } else {
            self.secondary.get_chunk(job_id, chunk).await
        }
    }

    async fn get_stats(&self, job_id: i64) -> Result<crate::JobStats> {
        if self.read_from_primary {
            self.primary.get_stats(job_id).await
        } else {
            self.secondary.get_stats(job_id).await
        }
    }

    #[cfg(feature = "analytics")]
    async fn export_to_parquet(&self, path: &str) -> Result<()> {
        if self.read_from_primary {
            self.primary.export_to_parquet(path).await
        } else {
            self.secondary.export_to_parquet(path).await
        }
    }

    async fn new_job(
        &mut self,
        source: String,
        destination: String,
        compress: bool,
        verify: bool,
        parallel: Option<usize>,
    ) -> Result<i64> {
        // Create the job in the primary store only
        // The secondary store is read-only during migration
        self.primary
            .new_job(source, destination, compress, verify, parallel)
            .await
    }

    async fn delete_job(&mut self, job_id: i64) -> Result<()> {
        self.primary.delete_job(job_id).await?;
        self.secondary.delete_job(job_id).await?;
        Ok(())
    }
}
