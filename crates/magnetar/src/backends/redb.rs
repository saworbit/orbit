//! redb backend implementation
//!
//! Pure Rust embedded database using MMAP for zero-copy reads.
//! Ideal for WASM/embedded environments where FFI is not available.

use crate::{JobState, JobStats, JobStatus, JobStore};
use anyhow::{Context, Result};
use async_trait::async_trait;
use redb::{Database, ReadableTable, TableDefinition};
use std::collections::HashMap;
use std::path::Path;

// Table definitions using composite keys
const CHUNKS_TABLE: TableDefinition<(i64, u64), &[u8]> = TableDefinition::new("chunks");
const DEPS_TABLE: TableDefinition<(i64, u64, u64), ()> = TableDefinition::new("dependencies");

/// redb-backed job store
pub struct RedbStore {
    db: Database,
}

impl RedbStore {
    /// Open or create a redb database at the specified path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Database::create(path).context("Failed to create redb database")?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _chunks = write_txn.open_table(CHUNKS_TABLE)?;
            let _deps = write_txn.open_table(DEPS_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db })
    }

    /// Serialize JobState to bytes
    fn serialize_state(state: &JobState) -> Result<Vec<u8>> {
        bincode::serialize(state).context("Failed to serialize JobState")
    }

    /// Deserialize JobState from bytes
    fn deserialize_state(bytes: &[u8]) -> Result<JobState> {
        bincode::deserialize(bytes).context("Failed to deserialize JobState")
    }
}

#[async_trait]
impl JobStore for RedbStore {
    async fn init_from_manifest(&mut self, job_id: i64, manifest: &toml::Value) -> Result<()> {
        let chunks = manifest
            .get("chunks")
            .and_then(|v| v.as_array())
            .context("Manifest must have 'chunks' array")?;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(CHUNKS_TABLE)?;

            for chunk_val in chunks {
                let chunk = chunk_val
                    .get("id")
                    .and_then(|v| v.as_integer())
                    .context("Chunk must have 'id' field")? as u64;

                let checksum = chunk_val
                    .get("checksum")
                    .and_then(|v| v.as_str())
                    .context("Chunk must have 'checksum' field")?;

                let state = JobState::new(job_id, chunk, checksum.to_string());
                let bytes = Self::serialize_state(&state)?;
                table.insert((job_id, chunk), bytes.as_slice())?;
            }
        }
        write_txn.commit()?;

        Ok(())
    }

    async fn claim_pending(&mut self, job_id: i64) -> Result<Option<JobState>> {
        let write_txn = self.db.begin_write()?;
        let result = {
            let mut table = write_txn.open_table(CHUNKS_TABLE)?;

            // Find first pending chunk (collect to avoid borrow issues)
            let range = (job_id, 0)..=(job_id, u64::MAX);
            let mut pending_chunk: Option<(u64, JobState)> = None;

            for item in table.range(range)? {
                let (key, bytes) = item?;
                let (_job_id, chunk) = key.value();
                let state = Self::deserialize_state(bytes.value())?;

                if state.status == JobStatus::Pending {
                    pending_chunk = Some((chunk, state));
                    break;
                }
            }

            // Now mutate outside the iteration
            if let Some((chunk, state)) = pending_chunk {
                let mut new_state = state;
                new_state.status = JobStatus::Processing;
                let new_bytes = Self::serialize_state(&new_state)?;
                table.insert((job_id, chunk), new_bytes.as_slice())?;
                Some(new_state)
            } else {
                None
            }
        };

        write_txn.commit()?;
        Ok(result)
    }

    async fn mark_status(
        &mut self,
        job_id: i64,
        chunk: u64,
        status: JobStatus,
        checksum: Option<String>,
    ) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(CHUNKS_TABLE)?;

            // Read state in separate scope to drop borrow
            let state_opt = {
                let bytes_guard = table.get((job_id, chunk))?;
                bytes_guard
                    .map(|b| Self::deserialize_state(b.value()))
                    .transpose()?
            };

            if let Some(mut state) = state_opt {
                state.status = status;
                if let Some(cs) = checksum {
                    state.checksum = cs;
                }
                state.error = None;

                let new_bytes = Self::serialize_state(&state)?;
                table.insert((job_id, chunk), new_bytes.as_slice())?;
            }
        }
        write_txn.commit()?;

        Ok(())
    }

    async fn mark_failed(&mut self, job_id: i64, chunk: u64, error: String) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(CHUNKS_TABLE)?;

            // Read state in separate scope to drop borrow
            let state_opt = {
                let bytes_guard = table.get((job_id, chunk))?;
                bytes_guard
                    .map(|b| Self::deserialize_state(b.value()))
                    .transpose()?
            };

            if let Some(mut state) = state_opt {
                state.status = JobStatus::Failed;
                state.error = Some(error);

                let new_bytes = Self::serialize_state(&state)?;
                table.insert((job_id, chunk), new_bytes.as_slice())?;
            }
        }
        write_txn.commit()?;

        Ok(())
    }

    async fn resume_pending(&self, job_id: i64) -> Result<Vec<JobState>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(CHUNKS_TABLE)?;

        let mut states = Vec::new();
        let range = (job_id, 0)..=(job_id, u64::MAX);

        for item in table.range(range)? {
            let (_, bytes) = item?;
            let state = Self::deserialize_state(bytes.value())?;
            if state.status == JobStatus::Pending {
                states.push(state);
            }
        }

        Ok(states)
    }

    async fn get_by_status(&self, job_id: i64, status: JobStatus) -> Result<Vec<JobState>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(CHUNKS_TABLE)?;

        let mut states = Vec::new();
        let range = (job_id, 0)..=(job_id, u64::MAX);

        for item in table.range(range)? {
            let (_, bytes) = item?;
            let state = Self::deserialize_state(bytes.value())?;
            if state.status == status {
                states.push(state);
            }
        }

        Ok(states)
    }

    async fn add_dependency(&mut self, job_id: i64, chunk: u64, deps: Vec<u64>) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(DEPS_TABLE)?;

            for dep in deps {
                table.insert((job_id, chunk, dep), ())?;
            }
        }
        write_txn.commit()?;

        Ok(())
    }

    async fn get_dependencies(&self, job_id: i64, chunk: u64) -> Result<Vec<u64>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(DEPS_TABLE)?;

        let mut deps = Vec::new();
        let range = (job_id, chunk, 0)..=(job_id, chunk, u64::MAX);

        for item in table.range(range)? {
            let (key, _) = item?;
            let (_job_id, _chunk, dep) = key.value();
            deps.push(dep);
        }

        Ok(deps)
    }

    async fn topo_sort_ready(&self, job_id: i64) -> Result<Vec<u64>> {
        let read_txn = self.db.begin_read()?;
        let chunks_table = read_txn.open_table(CHUNKS_TABLE)?;
        let deps_table = read_txn.open_table(DEPS_TABLE)?;

        // Build a map of chunk -> status
        let mut chunk_status: HashMap<u64, JobStatus> = HashMap::new();
        let range = (job_id, 0)..=(job_id, u64::MAX);

        for item in chunks_table.range(range)? {
            let (key, bytes) = item?;
            let (_job_id, chunk) = key.value();
            let state = Self::deserialize_state(bytes.value())?;
            chunk_status.insert(chunk, state.status);
        }

        // Find pending chunks with all dependencies done
        let mut ready = Vec::new();

        for (chunk, status) in &chunk_status {
            if *status != JobStatus::Pending {
                continue;
            }

            // Get dependencies
            let dep_range = (job_id, *chunk, 0)..=(job_id, *chunk, u64::MAX);
            let mut all_done = true;

            for item in deps_table.range(dep_range)? {
                let (key, _) = item?;
                let (_job_id, _chunk, dep) = key.value();
                if chunk_status.get(&dep) != Some(&JobStatus::Done) {
                    all_done = false;
                    break;
                }
            }

            if all_done {
                ready.push(*chunk);
            }
        }

        ready.sort();
        Ok(ready)
    }

    async fn get_chunk(&self, job_id: i64, chunk: u64) -> Result<Option<JobState>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(CHUNKS_TABLE)?;

        match table.get((job_id, chunk))? {
            Some(bytes) => Ok(Some(Self::deserialize_state(bytes.value())?)),
            None => Ok(None),
        }
    }

    async fn get_stats(&self, job_id: i64) -> Result<JobStats> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(CHUNKS_TABLE)?;

        let mut stats = JobStats {
            job_id,
            total_chunks: 0,
            pending: 0,
            processing: 0,
            done: 0,
            failed: 0,
        };

        let range = (job_id, 0)..=(job_id, u64::MAX);

        for item in table.range(range)? {
            let (_, bytes) = item?;
            let state = Self::deserialize_state(bytes.value())?;

            stats.total_chunks += 1;
            match state.status {
                JobStatus::Pending => stats.pending += 1,
                JobStatus::Processing => stats.processing += 1,
                JobStatus::Done => stats.done += 1,
                JobStatus::Failed => stats.failed += 1,
            }
        }

        Ok(stats)
    }

    #[cfg(feature = "analytics")]
    async fn export_to_parquet(&self, path: &str) -> Result<()> {
        use polars::prelude::*;

        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(CHUNKS_TABLE)?;

        let mut job_ids = Vec::new();
        let mut chunks = Vec::new();
        let mut checksums = Vec::new();
        let mut statuses = Vec::new();
        let mut errors = Vec::new();

        for item in table.iter()? {
            let (key, bytes) = item?;
            let (job_id, chunk) = key.value();
            let state = Self::deserialize_state(bytes.value())?;

            job_ids.push(job_id);
            chunks.push(chunk as i64);
            checksums.push(state.checksum);
            statuses.push(state.status.to_string());
            errors.push(state.error);
        }

        let df = DataFrame::new(vec![
            Series::new("job_id".into(), job_ids),
            Series::new("chunk".into(), chunks),
            Series::new("checksum".into(), checksums),
            Series::new("status".into(), statuses),
            Series::new("error".into(), errors),
        ])?;

        let mut file = std::fs::File::create(path)?;
        ParquetWriter::new(&mut file).finish(&mut df.clone())?;

        Ok(())
    }

    async fn new_job(
        &mut self,
        _source: String,
        _destination: String,
        _compress: bool,
        _verify: bool,
        _parallel: Option<usize>,
        _source_star_id: Option<String>,
        _dest_star_id: Option<String>,
    ) -> Result<i64> {
        // For the redb backend, we don't support auto-generated job IDs yet
        // Users should provide their own job IDs when using redb
        anyhow::bail!("Auto-generated job IDs are not supported in the redb backend. Please use the SQLite backend or provide your own job ID.")
    }

    async fn delete_job(&mut self, job_id: i64) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut chunks_table = write_txn.open_table(CHUNKS_TABLE)?;
            let mut deps_table = write_txn.open_table(DEPS_TABLE)?;

            // Delete all chunks
            let range = (job_id, 0)..=(job_id, u64::MAX);
            let keys_to_delete: Vec<_> = chunks_table
                .range(range)?
                .map(|item| item.map(|(k, _)| k.value()))
                .collect::<Result<_, _>>()?;

            for key in keys_to_delete {
                chunks_table.remove(key)?;
            }

            // Delete all dependencies
            let dep_range = (job_id, 0, 0)..=(job_id, u64::MAX, u64::MAX);
            let dep_keys_to_delete: Vec<_> = deps_table
                .range(dep_range)?
                .map(|item| item.map(|(k, _)| k.value()))
                .collect::<Result<_, _>>()?;

            for key in dep_keys_to_delete {
                deps_table.remove(key)?;
            }
        }
        write_txn.commit()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_redb_basic_flow() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = RedbStore::open(tmp.path())?;

        let manifest = toml::from_str(
            r#"
            [[chunks]]
            id = 1
            checksum = "abc123"

            [[chunks]]
            id = 2
            checksum = "def456"
            "#,
        )?;

        store.init_from_manifest(1, &manifest).await?;

        let chunk1 = store.claim_pending(1).await?.unwrap();
        assert_eq!(chunk1.chunk, 1);
        assert_eq!(chunk1.status, JobStatus::Processing);

        store.mark_status(1, 1, JobStatus::Done, None).await?;

        let stats = store.get_stats(1).await?;
        assert_eq!(stats.done, 1);
        assert_eq!(stats.pending, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_redb_dependencies() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = RedbStore::open(tmp.path())?;

        let manifest = toml::from_str(
            r#"
            [[chunks]]
            id = 1
            checksum = "a"

            [[chunks]]
            id = 2
            checksum = "b"

            [[chunks]]
            id = 3
            checksum = "c"
            "#,
        )?;

        store.init_from_manifest(1, &manifest).await?;
        store.add_dependency(1, 3, vec![1, 2]).await?;

        // Only chunks 1 and 2 should be ready
        let ready = store.topo_sort_ready(1).await?;
        assert_eq!(ready, vec![1, 2]);

        // Complete chunk 1
        store.mark_status(1, 1, JobStatus::Done, None).await?;
        let ready = store.topo_sort_ready(1).await?;
        assert_eq!(ready, vec![2]); // 3 still blocked

        // Complete chunk 2
        store.mark_status(1, 2, JobStatus::Done, None).await?;
        let ready = store.topo_sort_ready(1).await?;
        assert_eq!(ready, vec![3]); // Now 3 is ready

        Ok(())
    }
}
