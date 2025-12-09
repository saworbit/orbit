//! Neutrino Executor: High-concurrency handler for small files.
//!
//! Unlike the standard executor which limits concurrency to avoid CPU starvation
//! (due to hashing), this executor is I/O bound and can scale to much higher
//! concurrency levels (100-500 concurrent tasks).

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;

use crate::config::{CopyConfig, ErrorMode};

/// Statistics for Neutrino executor batch operations
#[derive(Debug, Clone, Default)]
pub struct ExecutorStats {
    /// Number of files successfully copied
    pub files_copied: u64,

    /// Number of files that failed to copy
    pub files_failed: u64,

    /// Total bytes copied
    pub bytes_copied: u64,

    /// Duration of the batch operation
    pub duration: Duration,

    /// Number of concurrent tasks spawned
    pub tasks_spawned: usize,
}

impl ExecutorStats {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Represents a small file job for fast lane processing
#[derive(Debug, Clone)]
pub struct SmallFileJob {
    /// Source path
    pub source: PathBuf,

    /// Destination path
    pub dest: PathBuf,

    /// File size in bytes
    pub size: u64,
}

/// Result of a single file transfer
#[derive(Debug, Clone, Copy)]
struct TransferResult {
    bytes: u64,
    success: bool,
}

/// Batches small file transfers to maximize throughput.
///
/// # Design Philosophy
///
/// For small files (<8KB), CDC chunking and deduplication overhead exceeds
/// potential savings. Neutrino bypasses all heavy machinery:
///
/// - **No CDC chunking** - Direct file copy
/// - **No deduplication** - No starmap index lookups
/// - **No compression** - Unless explicitly configured (future)
/// - **High concurrency** - 100-500 concurrent tokio tasks
///
/// # Concurrency Model
///
/// Uses tokio's async runtime with:
/// - `JoinSet` for managing concurrent tasks
/// - `Semaphore` for concurrency limiting
/// - Async file I/O (tokio::fs) to avoid blocking
///
/// # Example
///
/// ```no_run
/// use orbit::core::neutrino::{DirectTransferExecutor, SmallFileJob};
/// use orbit::config::CopyConfig;
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = CopyConfig::default();
/// let executor = DirectTransferExecutor::new(&config)?;
///
/// let jobs = vec![
///     SmallFileJob {
///         source: "file1.txt".into(),
///         dest: "dest/file1.txt".into(),
///         size: 1024,
///     },
/// ];
///
/// let stats = executor.execute_batch(jobs, &config).await?;
/// println!("Copied {} files", stats.files_copied);
/// # Ok(())
/// # }
/// ```
pub struct DirectTransferExecutor {
    /// Maximum concurrent transfers
    concurrency: usize,

    /// Statistics accumulator (shared across tasks)
    stats: Arc<Mutex<ExecutorStats>>,
}

impl DirectTransferExecutor {
    /// Default concurrency limit for Neutrino executor
    const DEFAULT_CONCURRENCY: usize = 200;

    /// Creates a new Neutrino executor with configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Copy configuration (used to determine concurrency)
    ///
    /// # Returns
    ///
    /// A new DirectTransferExecutor instance
    pub fn new(config: &CopyConfig) -> Result<Self> {
        let concurrency = if config.parallel > 0 {
            config.parallel
        } else {
            Self::DEFAULT_CONCURRENCY
        };

        Ok(Self {
            concurrency,
            stats: Arc::new(Mutex::new(ExecutorStats::new())),
        })
    }

    /// Execute a batch of small file transfers with high concurrency
    ///
    /// # Arguments
    ///
    /// * `jobs` - Vector of small file jobs to process
    /// * `config` - Copy configuration for metadata preservation, error handling, etc.
    ///
    /// # Returns
    ///
    /// Aggregated statistics for the batch operation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - ErrorMode::Abort is set and any file fails
    /// - Critical system errors occur (out of file descriptors, etc.)
    pub async fn execute_batch(
        &self,
        jobs: Vec<SmallFileJob>,
        config: &CopyConfig,
    ) -> Result<ExecutorStats> {
        let start_time = Instant::now();
        let total_jobs = jobs.len();

        if total_jobs == 0 {
            return Ok(ExecutorStats::new());
        }

        // Semaphore to limit concurrent tasks
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut join_set = JoinSet::new();

        let stats_clone = self.stats.clone();

        // Spawn concurrent tasks
        for job in jobs {
            let permit = semaphore.clone().acquire_owned().await?;
            let config_clone = config.clone();
            let stats_ref = stats_clone.clone();

            join_set.spawn(async move {
                let result = Self::transfer_single_file(job, &config_clone).await;

                // Update stats
                let mut stats = stats_ref.lock().await;
                match result {
                    Ok(transfer_result) => {
                        if transfer_result.success {
                            stats.files_copied += 1;
                            stats.bytes_copied += transfer_result.bytes;
                        } else {
                            stats.files_failed += 1;
                        }
                    }
                    Err(_) => {
                        stats.files_failed += 1;
                    }
                }

                drop(permit); // Release semaphore
                result
            });
        }

        // Collect results and check for errors
        let mut first_error: Option<anyhow::Error> = None;

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(_)) => {
                    // Task completed successfully
                }
                Ok(Err(e)) => {
                    // Transfer error
                    if config.error_mode == ErrorMode::Abort && first_error.is_none() {
                        first_error = Some(e);
                    }
                }
                Err(e) => {
                    // JoinHandle error (task panic)
                    if config.error_mode == ErrorMode::Abort && first_error.is_none() {
                        first_error = Some(anyhow::anyhow!("Task panic: {}", e));
                    }
                }
            }
        }

        // Finalize stats
        let mut final_stats = self.stats.lock().await.clone();
        final_stats.duration = start_time.elapsed();
        final_stats.tasks_spawned = total_jobs;

        // Return error if in abort mode and error occurred
        if let Some(err) = first_error {
            return Err(err);
        }

        Ok(final_stats)
    }

    /// Performs the raw transfer bypassing CDC and Starmap
    ///
    /// This is the core transfer logic for small files:
    /// 1. Read file into memory (safe for <8KB files)
    /// 2. Write directly to destination
    /// 3. Preserve metadata if configured
    async fn transfer_single_file(
        job: SmallFileJob,
        config: &CopyConfig,
    ) -> Result<TransferResult> {
        // Ensure destination directory exists
        if let Some(parent) = job.dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Direct async copy (no CDC, no chunking)
        let bytes_copied = tokio::fs::copy(&job.source, &job.dest).await?;

        // Preserve metadata if configured
        if config.preserve_metadata {
            // Use synchronous metadata preservation for now
            // (tokio::fs doesn't provide all metadata operations)
            let source_meta = std::fs::metadata(&job.source)?;
            let dest_file = std::fs::File::open(&job.dest)?;

            // Preserve times
            if let Ok(modified) = source_meta.modified() {
                dest_file.set_modified(modified)?;
            }

            // Preserve permissions (Unix)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(source_meta.permissions().mode());
                std::fs::set_permissions(&job.dest, permissions)?;
            }

            // Note: Full metadata preservation (xattrs, ACLs, etc.) could be added
            // by calling file_metadata::copy_metadata() but that may add overhead
        }

        Ok(TransferResult {
            bytes: bytes_copied,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_executor_basic() -> Result<()> {
        let temp_src = tempdir()?;
        let temp_dest = tempdir()?;

        // Create test files
        let file1 = temp_src.path().join("file1.txt");
        fs::write(&file1, b"Hello, World!").await?;

        let jobs = vec![SmallFileJob {
            source: file1.clone(),
            dest: temp_dest.path().join("file1.txt"),
            size: 13,
        }];

        let config = CopyConfig::default();
        let executor = DirectTransferExecutor::new(&config)?;

        let stats = executor.execute_batch(jobs, &config).await?;

        assert_eq!(stats.files_copied, 1);
        assert_eq!(stats.files_failed, 0);
        assert_eq!(stats.bytes_copied, 13);

        // Verify file content
        let content = fs::read_to_string(temp_dest.path().join("file1.txt")).await?;
        assert_eq!(content, "Hello, World!");

        Ok(())
    }

    #[tokio::test]
    async fn test_executor_multiple_files() -> Result<()> {
        let temp_src = tempdir()?;
        let temp_dest = tempdir()?;

        // Create 10 test files
        let mut jobs = Vec::new();
        for i in 0..10 {
            let filename = format!("file_{}.txt", i);
            let src_path = temp_src.path().join(&filename);
            let content = format!("Content {}", i);
            fs::write(&src_path, content.as_bytes()).await?;

            jobs.push(SmallFileJob {
                source: src_path,
                dest: temp_dest.path().join(&filename),
                size: content.len() as u64,
            });
        }

        let config = CopyConfig::default();
        let executor = DirectTransferExecutor::new(&config)?;

        let stats = executor.execute_batch(jobs, &config).await?;

        assert_eq!(stats.files_copied, 10);
        assert_eq!(stats.files_failed, 0);
        assert!(stats.bytes_copied > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_executor_empty_batch() -> Result<()> {
        let config = CopyConfig::default();
        let executor = DirectTransferExecutor::new(&config)?;

        let stats = executor.execute_batch(vec![], &config).await?;

        assert_eq!(stats.files_copied, 0);
        assert_eq!(stats.files_failed, 0);
        assert_eq!(stats.bytes_copied, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_executor_nested_directories() -> Result<()> {
        let temp_src = tempdir()?;
        let temp_dest = tempdir()?;

        // Create nested source file
        let nested = temp_src.path().join("nested").join("deep");
        fs::create_dir_all(&nested).await?;
        let src_file = nested.join("file.txt");
        fs::write(&src_file, b"nested content").await?;

        let jobs = vec![SmallFileJob {
            source: src_file,
            dest: temp_dest
                .path()
                .join("nested")
                .join("deep")
                .join("file.txt"),
            size: 14,
        }];

        let config = CopyConfig::default();
        let executor = DirectTransferExecutor::new(&config)?;

        let stats = executor.execute_batch(jobs, &config).await?;

        assert_eq!(stats.files_copied, 1);

        // Verify nested directory was created
        assert!(temp_dest.path().join("nested").join("deep").exists());
        let content = fs::read_to_string(
            temp_dest
                .path()
                .join("nested")
                .join("deep")
                .join("file.txt"),
        )
        .await?;
        assert_eq!(content, "nested content");

        Ok(())
    }
}
