use crate::config::GhostConfig;
use crate::error::GhostError;
use crate::inode::GhostEntry;
use crate::oracle::MetadataOracle;
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::Row;

/// MagnetarAdapter connects to the Magnetar SQLite database and implements
/// the MetadataOracle trait for querying artifact metadata.
pub struct MagnetarAdapter {
    pool: SqlitePool,
    job_id: i64,
    config: GhostConfig,
}

impl MagnetarAdapter {
    /// Create a new adapter connected to the Magnetar database
    pub async fn new(db_path: &str, job_id: i64) -> Result<Self, GhostError> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(false); // DB should already exist with migrations

        let pool = SqlitePool::connect_with(options).await?;

        Ok(Self {
            pool,
            job_id,
            config: GhostConfig::default(),
        })
    }
}

#[async_trait]
impl MetadataOracle for MagnetarAdapter {
    async fn get_root_id(&self) -> Result<String, GhostError> {
        let row =
            sqlx::query("SELECT id FROM artifacts WHERE job_id = ? AND parent_id IS NULL LIMIT 1")
                .bind(self.job_id)
                .fetch_optional(&self.pool)
                .await?;

        row.map(|r| r.get("id"))
            .ok_or_else(|| GhostError::NotFound("Root artifact".to_string()))
    }

    async fn lookup(&self, parent_id: &str, name: &str) -> Result<Option<GhostEntry>, GhostError> {
        let row = sqlx::query(
            "SELECT id, name, size, is_dir, mtime FROM artifacts
             WHERE job_id = ? AND parent_id = ? AND name = ?",
        )
        .bind(self.job_id)
        .bind(parent_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| GhostEntry {
            id: r.get("id"),
            name: r.get("name"),
            size: r.get::<i64, _>("size") as u64,
            is_dir: r.get("is_dir"),
            mtime: r.get::<i64, _>("mtime") as u64,
        }))
    }

    async fn readdir(&self, parent_id: &str) -> Result<Vec<GhostEntry>, GhostError> {
        let rows = sqlx::query(
            "SELECT id, name, size, is_dir, mtime FROM artifacts
             WHERE job_id = ? AND parent_id = ? ORDER BY name ASC",
        )
        .bind(self.job_id)
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| GhostEntry {
                id: r.get("id"),
                name: r.get("name"),
                size: r.get::<i64, _>("size") as u64,
                is_dir: r.get("is_dir"),
                mtime: r.get::<i64, _>("mtime") as u64,
            })
            .collect())
    }

    async fn getattr(&self, id: &str) -> Result<GhostEntry, GhostError> {
        let row = sqlx::query(
            "SELECT id, name, size, is_dir, mtime FROM artifacts
             WHERE job_id = ? AND id = ?",
        )
        .bind(self.job_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| GhostEntry {
            id: r.get("id"),
            name: r.get("name"),
            size: r.get::<i64, _>("size") as u64,
            is_dir: r.get("is_dir"),
            mtime: r.get::<i64, _>("mtime") as u64,
        })
        .ok_or_else(|| GhostError::NotFound(id.to_string()))
    }
}
