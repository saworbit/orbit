//! SQLite backend implementation
//!
//! Provides persistent storage using SQLite with WAL mode for concurrency.
//! This is the default backend and offers SQL-based analytics capabilities.

use crate::{JobState, JobStats, JobStatus, JobStore};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, migrate::MigrateDatabase};
use std::str::FromStr;

/// SQLite-backed job store
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    /// Open or create a SQLite database at the specified path
    ///
    /// Automatically runs migrations to set up the schema.
    pub async fn open(path: &str) -> Result<Self> {
        // Create database if it doesn't exist
        let db_url = if path.starts_with("sqlite://") {
            path.to_string()
        } else {
            format!("sqlite://{}", path)
        };

        // Create database if it doesn't exist
        if !sqlx::Sqlite::database_exists(&db_url).await.unwrap_or(false) {
            sqlx::Sqlite::create_database(&db_url).await
                .context("Failed to create database")?;
        }

        // Configure connection with WAL mode for better concurrency
        let options = SqliteConnectOptions::from_str(&db_url)?
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .context("Failed to connect to database")?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("Failed to run migrations")?;

        Ok(Self { pool })
    }

    /// Get the underlying pool (for advanced usage)
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// Helper to convert SqliteRow to JobState
fn row_to_job_state(row: &SqliteRow) -> Result<JobState> {
    let status_str: String = row.try_get("status")?;
    let status = JobStatus::from_str(&status_str)?;
    let error: Option<String> = row.try_get("error")?;

    Ok(JobState {
        job_id: row.try_get("job_id")?,
        chunk: row.try_get::<i64, _>("chunk")? as u64,
        checksum: row.try_get("checksum")?,
        status,
        error,
    })
}

#[async_trait]
impl JobStore for SqliteStore {
    async fn init_from_manifest(&mut self, job_id: i64, manifest: &toml::Value) -> Result<()> {
        let chunks = manifest
            .get("chunks")
            .and_then(|v| v.as_array())
            .context("Manifest must have 'chunks' array")?;

        let mut tx = self.pool.begin().await?;

        for chunk_val in chunks {
            let chunk = chunk_val
                .get("id")
                .and_then(|v| v.as_integer())
                .context("Chunk must have 'id' field")? as u64;

            let checksum = chunk_val
                .get("checksum")
                .and_then(|v| v.as_str())
                .context("Chunk must have 'checksum' field")?;

            sqlx::query(
                "INSERT INTO chunks (job_id, chunk, checksum, status) VALUES (?, ?, ?, 'pending')
                 ON CONFLICT (job_id, chunk) DO UPDATE SET checksum = excluded.checksum"
            )
            .bind(job_id)
            .bind(chunk as i64)
            .bind(checksum)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn claim_pending(&mut self, job_id: i64) -> Result<Option<JobState>> {
        let mut tx = self.pool.begin().await?;

        // Atomic claim: find first pending and mark as processing
        let row = sqlx::query(
            "UPDATE chunks
             SET status = 'processing'
             WHERE rowid = (
                 SELECT rowid FROM chunks
                 WHERE job_id = ? AND status = 'pending'
                 ORDER BY chunk ASC
                 LIMIT 1
             )
             RETURNING job_id, chunk, checksum, status, error"
        )
        .bind(job_id)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        match row {
            Some(r) => Ok(Some(row_to_job_state(&r)?)),
            None => Ok(None),
        }
    }

    async fn mark_status(
        &mut self,
        job_id: i64,
        chunk: u64,
        status: JobStatus,
        checksum: Option<String>,
    ) -> Result<()> {
        if let Some(cs) = checksum {
            sqlx::query(
                "UPDATE chunks SET status = ?, checksum = ?, error = NULL WHERE job_id = ? AND chunk = ?"
            )
            .bind(status.to_string())
            .bind(cs)
            .bind(job_id)
            .bind(chunk as i64)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE chunks SET status = ? WHERE job_id = ? AND chunk = ?"
            )
            .bind(status.to_string())
            .bind(job_id)
            .bind(chunk as i64)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn mark_failed(&mut self, job_id: i64, chunk: u64, error: String) -> Result<()> {
        sqlx::query(
            "UPDATE chunks SET status = 'failed', error = ? WHERE job_id = ? AND chunk = ?"
        )
        .bind(error)
        .bind(job_id)
        .bind(chunk as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn resume_pending(&self, job_id: i64) -> Result<Vec<JobState>> {
        let rows = sqlx::query(
            "SELECT job_id, chunk, checksum, status, error FROM chunks
             WHERE job_id = ? AND status = 'pending'
             ORDER BY chunk ASC"
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_job_state).collect()
    }

    async fn get_by_status(&self, job_id: i64, status: JobStatus) -> Result<Vec<JobState>> {
        let rows = sqlx::query(
            "SELECT job_id, chunk, checksum, status, error FROM chunks
             WHERE job_id = ? AND status = ?
             ORDER BY chunk ASC"
        )
        .bind(job_id)
        .bind(status.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_job_state).collect()
    }

    async fn add_dependency(&mut self, job_id: i64, chunk: u64, deps: Vec<u64>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for dep in deps {
            sqlx::query(
                "INSERT INTO dependencies (job_id, chunk, depends_on) VALUES (?, ?, ?)
                 ON CONFLICT DO NOTHING"
            )
            .bind(job_id)
            .bind(chunk as i64)
            .bind(dep as i64)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_dependencies(&self, job_id: i64, chunk: u64) -> Result<Vec<u64>> {
        let rows = sqlx::query(
            "SELECT depends_on FROM dependencies WHERE job_id = ? AND chunk = ?"
        )
        .bind(job_id)
        .bind(chunk as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| row.get::<i64, _>("depends_on") as u64)
            .collect())
    }

    async fn topo_sort_ready(&self, job_id: i64) -> Result<Vec<u64>> {
        // Find all pending chunks whose dependencies are all done
        let rows = sqlx::query(
            "SELECT c.chunk FROM chunks c
             WHERE c.job_id = ? AND c.status = 'pending'
             AND NOT EXISTS (
                 SELECT 1 FROM dependencies d
                 JOIN chunks dc ON d.job_id = dc.job_id AND d.depends_on = dc.chunk
                 WHERE d.job_id = c.job_id
                 AND d.chunk = c.chunk
                 AND dc.status != 'done'
             )
             ORDER BY c.chunk ASC"
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| row.get::<i64, _>("chunk") as u64)
            .collect())
    }

    async fn get_chunk(&self, job_id: i64, chunk: u64) -> Result<Option<JobState>> {
        let row = sqlx::query(
            "SELECT job_id, chunk, checksum, status, error FROM chunks
             WHERE job_id = ? AND chunk = ?"
        )
        .bind(job_id)
        .bind(chunk as i64)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(row_to_job_state(&r)?)),
            None => Ok(None),
        }
    }

    async fn get_stats(&self, job_id: i64) -> Result<JobStats> {
        let row = sqlx::query(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN status = 'processing' THEN 1 ELSE 0 END) as processing,
                SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END) as done,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed
             FROM chunks WHERE job_id = ?"
        )
        .bind(job_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(JobStats {
            job_id,
            total_chunks: row.get::<i64, _>("total") as u64,
            pending: row.get::<i64, _>("pending") as u64,
            processing: row.get::<i64, _>("processing") as u64,
            done: row.get::<i64, _>("done") as u64,
            failed: row.get::<i64, _>("failed") as u64,
        })
    }

    #[cfg(feature = "analytics")]
    async fn export_to_parquet(&self, path: &str) -> Result<()> {
        use polars::prelude::*;

        let rows = sqlx::query(
            "SELECT job_id, chunk, checksum, status, error FROM chunks ORDER BY job_id, chunk"
        )
        .fetch_all(&self.pool)
        .await?;

        let job_ids: Vec<i64> = rows.iter().map(|r| r.get("job_id")).collect();
        let chunks: Vec<i64> = rows.iter().map(|r| r.get("chunk")).collect();
        let checksums: Vec<String> = rows.iter().map(|r| r.get("checksum")).collect();
        let statuses: Vec<String> = rows.iter().map(|r| r.get("status")).collect();
        let errors: Vec<Option<String>> = rows.iter().map(|r| r.get("error")).collect();

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
        source: String,
        destination: String,
        compress: bool,
        verify: bool,
        parallel: Option<usize>,
    ) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO jobs (source, destination, compress, verify, parallel) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&source)
        .bind(&destination)
        .bind(compress)
        .bind(verify)
        .bind(parallel.map(|p| p as i64))
        .execute(&self.pool)
        .await?;

        // SQLite returns the last inserted row ID
        Ok(result.last_insert_rowid())
    }

    async fn delete_job(&mut self, job_id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM dependencies WHERE job_id = ?")
            .bind(job_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM chunks WHERE job_id = ?")
            .bind(job_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM jobs WHERE id = ?")
            .bind(job_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_sqlite_basic_flow() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = SqliteStore::open(tmp.path().to_str().unwrap()).await?;

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
    async fn test_new_job_creates_numeric_id() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = SqliteStore::open(tmp.path().to_str().unwrap()).await?;

        // Create a new job
        let job_id = store
            .new_job(
                "/source/path".to_string(),
                "/dest/path".to_string(),
                true,
                false,
                Some(4),
            )
            .await?;

        // Job ID should be auto-generated (1 for the first job)
        assert_eq!(job_id, 1);

        // Create a second job
        let job_id2 = store
            .new_job(
                "/source2/path".to_string(),
                "/dest2/path".to_string(),
                false,
                true,
                None,
            )
            .await?;

        // Second job should get ID 2
        assert_eq!(job_id2, 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_job_lifecycle_with_auto_id() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = SqliteStore::open(tmp.path().to_str().unwrap()).await?;

        // Create a new job
        let job_id = store
            .new_job(
                "/source/path".to_string(),
                "/dest/path".to_string(),
                true,
                true,
                Some(2),
            )
            .await?;

        // Create and initialize a manifest
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

        store.init_from_manifest(job_id, &manifest).await?;

        // Get stats - should show 2 pending chunks
        let stats = store.get_stats(job_id).await?;
        assert_eq!(stats.total_chunks, 2);
        assert_eq!(stats.pending, 2);
        assert_eq!(stats.done, 0);

        // Process a chunk
        let chunk = store.claim_pending(job_id).await?.unwrap();
        assert_eq!(chunk.chunk, 1);
        store.mark_status(job_id, 1, JobStatus::Done, None).await?;

        // Check stats again
        let stats = store.get_stats(job_id).await?;
        assert_eq!(stats.done, 1);
        assert_eq!(stats.pending, 1);

        // Delete the job
        store.delete_job(job_id).await?;

        // Stats should show all zeros after deletion
        let stats = store.get_stats(job_id).await?;
        assert_eq!(stats.total_chunks, 0);
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.done, 0);
        assert_eq!(stats.failed, 0);

        Ok(())
    }
}
