//! Integration tests for Magnetar
//!
//! Tests both SQLite and redb backends with comprehensive scenarios.

use anyhow::Result;
use magnetar::{JobStatus, JobStore};
use tempfile::NamedTempFile;

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use super::*;
    use magnetar::SqliteStore;

    #[tokio::test]
    async fn test_sqlite_idempotent_claims() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = SqliteStore::open(tmp.path().to_str().unwrap()).await?;

        let manifest = toml::from_str(
            r#"
            [[chunks]]
            id = 1
            checksum = "a"
            "#,
        )?;

        store.init_from_manifest(1, &manifest).await?;

        // First claim
        let chunk1 = store.claim_pending(1).await?.unwrap();
        assert_eq!(chunk1.chunk, 1);
        assert_eq!(chunk1.status, JobStatus::Processing);

        // Second claim should return None (no more pending)
        let chunk2 = store.claim_pending(1).await?;
        assert!(chunk2.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_resume_after_processing() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = SqliteStore::open(tmp.path().to_str().unwrap()).await?;

        let manifest = toml::from_str(
            r#"
            [[chunks]]
            id = 1
            checksum = "a"

            [[chunks]]
            id = 2
            checksum = "b"
            "#,
        )?;

        store.init_from_manifest(1, &manifest).await?;

        // Claim and complete first chunk
        let chunk1 = store.claim_pending(1).await?.unwrap();
        store
            .mark_status(1, chunk1.chunk, JobStatus::Done, None)
            .await?;

        // Resume should only return second chunk
        let pending = store.resume_pending(1).await?;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].chunk, 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_dag_dependencies() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = SqliteStore::open(tmp.path().to_str().unwrap()).await?;

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

        // Task 3 depends on 1 and 2
        store.add_dependency(1, 3, vec![1, 2]).await?;

        // Initially, only 1 and 2 are ready
        let ready = store.topo_sort_ready(1).await?;
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&1));
        assert!(ready.contains(&2));

        // Complete task 1
        store.mark_status(1, 1, JobStatus::Done, None).await?;
        let ready = store.topo_sort_ready(1).await?;
        assert_eq!(ready, vec![2]); // Only 2 is ready now

        // Complete task 2
        store.mark_status(1, 2, JobStatus::Done, None).await?;
        let ready = store.topo_sort_ready(1).await?;
        assert_eq!(ready, vec![3]); // Now 3 is ready

        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_failed_chunks() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = SqliteStore::open(tmp.path().to_str().unwrap()).await?;

        let manifest = toml::from_str(
            r#"
            [[chunks]]
            id = 1
            checksum = "a"
            "#,
        )?;

        store.init_from_manifest(1, &manifest).await?;

        let chunk = store.claim_pending(1).await?.unwrap();
        store
            .mark_failed(1, chunk.chunk, "Test error".to_string())
            .await?;

        let failed = store.get_by_status(1, JobStatus::Failed).await?;
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].error, Some("Test error".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_stats() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = SqliteStore::open(tmp.path().to_str().unwrap()).await?;

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

        // Process one chunk
        let chunk = store.claim_pending(1).await?.unwrap();
        store
            .mark_status(1, chunk.chunk, JobStatus::Done, None)
            .await?;

        let stats = store.get_stats(1).await?;
        assert_eq!(stats.total_chunks, 3);
        assert_eq!(stats.done, 1);
        assert_eq!(stats.pending, 2);
        assert!(!stats.is_complete());
        assert!((stats.completion_percent() - 100.0 / 3.0).abs() < 0.001);

        Ok(())
    }
}

#[cfg(feature = "redb")]
mod redb_tests {
    use super::*;
    use magnetar::RedbStore;

    #[tokio::test]
    async fn test_redb_basic_flow() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let mut store = RedbStore::open(tmp.path())?;

        let manifest = toml::from_str(
            r#"
            [[chunks]]
            id = 1
            checksum = "abc"

            [[chunks]]
            id = 2
            checksum = "def"
            "#,
        )?;

        store.init_from_manifest(1, &manifest).await?;

        let chunk1 = store.claim_pending(1).await?.unwrap();
        assert_eq!(chunk1.chunk, 1);

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

        let ready = store.topo_sort_ready(1).await?;
        assert!(ready.contains(&1));
        assert!(ready.contains(&2));
        assert!(!ready.contains(&3));

        store.mark_status(1, 1, JobStatus::Done, None).await?;
        store.mark_status(1, 2, JobStatus::Done, None).await?;

        let ready = store.topo_sort_ready(1).await?;
        assert_eq!(ready, vec![3]);

        Ok(())
    }
}

// Cross-backend tests using the factory function
#[tokio::test]
async fn test_factory_function() -> Result<()> {
    #[cfg(feature = "sqlite")]
    {
        let tmp = NamedTempFile::new()?;
        let path = tmp.path().to_str().unwrap();

        let mut store = magnetar::open(path).await?;

        let manifest = toml::from_str(
            r#"
            [[chunks]]
            id = 1
            checksum = "test"
            "#,
        )?;

        store.init_from_manifest(1, &manifest).await?;
        let chunk = store.claim_pending(1).await?;
        assert!(chunk.is_some());
    }

    Ok(())
}
