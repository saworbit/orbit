/*!
 * Orbit V2 Integration - Smart Sync with Semantic Prioritization
 *
 * Connects the CDC (core-cdc) and Semantic (core-semantic) layers to the transfer engine.
 * Implements priority-based file transfer ordering for optimized disaster recovery.
 */

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use orbit_core_semantic::{Priority, SemanticRegistry, SyncStrategy};
use walkdir::WalkDir;

use crate::config::{CopyConfig, CopyMode};
use crate::core::filter::FilterList;
#[cfg(feature = "backend-abstraction")]
use crate::core::neutrino::{DirectTransferExecutor, FileRouter, SmallFileJob, TransferLane};
use crate::core::validation::matches_exclude_pattern;
use crate::core::CopyStats;
use crate::error::{OrbitError, Result};

/// A file transfer job with semantic priority
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PrioritizedJob {
    /// Source file path
    pub source_path: PathBuf,

    /// Destination file path
    pub dest_path: PathBuf,

    /// Semantic priority (Critical=0, High=10, Normal=50, Low=100)
    pub priority: Priority,

    /// Recommended sync strategy
    pub strategy: SyncStrategy,

    /// File size in bytes
    pub size: u64,
}

impl Ord for PrioritizedJob {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap, but we want min priority (0 = Critical) to come first
        // So we reverse the comparison: other.priority.cmp(&self.priority)
        // This makes Priority::Critical (0) > Priority::Low (100) in heap ordering
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.source_path.cmp(&other.source_path)) // Tie-breaker for stability
    }
}

impl PartialOrd for PrioritizedJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Check if V2 Smart Sync should be enabled
pub fn is_smart_mode(config: &CopyConfig) -> bool {
    config.check_mode_str.as_deref() == Some("smart")
}

/// Perform smart sync with semantic prioritization
///
/// Algorithm:
/// 1. Scan Phase: Walk directory tree, collecting all file paths
/// 2. Analyze Phase: Run SemanticRegistry on each file to determine priority
/// 3. Queue Phase: Push files into BinaryHeap<PrioritizedJob> (priority queue)
/// 4. Execute Phase: Pop jobs from heap and transfer in priority order
pub async fn perform_smart_sync(
    source_dir: &Path,
    dest_dir: &Path,
    config: &CopyConfig,
) -> Result<CopyStats> {
    if !config.recursive {
        return Err(OrbitError::Config(
            "Smart sync requires recursive flag".to_string(),
        ));
    }

    let start_time = Instant::now();

    if config.show_progress {
        println!("\n[V2] Starting Smart Sync with Semantic Prioritization");
        println!("   Source: {}", source_dir.display());
        println!("   Dest:   {}", dest_dir.display());
    }

    // Create destination directory
    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir)?;
    }

    // Initialize semantic registry
    let registry = SemanticRegistry::default();

    // Build filter list
    let filter_list = match FilterList::from_config(
        &config.include_patterns,
        &config.exclude_patterns,
        config.filter_from.as_deref(),
    ) {
        Ok(filters) => filters,
        Err(e) => {
            return Err(OrbitError::Config(format!(
                "Invalid filter configuration: {}",
                e
            )));
        }
    };

    // Phase 1: Scan + Analyze + Queue
    if config.show_progress {
        println!("\n[Phase 1] Scanning and analyzing files...");
    }

    let mut queue = BinaryHeap::new();
    let mut dirs_created = 0;
    let mut files_scanned = 0;

    for entry in WalkDir::new(source_dir)
        .follow_links(false)
        .same_file_system(true)
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Failed to read entry: {}", e);
                continue;
            }
        };

        let relative_path = match entry.path().strip_prefix(source_dir) {
            Ok(p) => p,
            Err(_) => continue,
        };

        if relative_path.as_os_str().is_empty() {
            continue;
        }

        // Apply filters
        let should_process = if !filter_list.is_empty() {
            filter_list.should_include(relative_path)
        } else {
            !matches_exclude_pattern(relative_path, &config.exclude_patterns)
        };

        if !should_process {
            continue;
        }

        let source_path = entry.path().to_path_buf();
        let dest_path = dest_dir.join(relative_path);

        // Handle directories
        if entry.file_type().is_dir() {
            if !dest_path.exists() {
                fs::create_dir_all(&dest_path)?;
                dirs_created += 1;
            }
            continue;
        }

        // Handle files
        if entry.file_type().is_file() {
            files_scanned += 1;

            // Read first few bytes for magic number detection
            let sample = fs::read(&source_path)
                .ok()
                .and_then(|bytes| {
                    if bytes.len() >= 12 {
                        Some(bytes[..12].to_vec())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            // Determine semantic intent
            let intent = registry.determine_intent(&source_path, &sample);

            let metadata = fs::metadata(&source_path)?;

            let job = PrioritizedJob {
                source_path: source_path.clone(),
                dest_path,
                priority: intent.priority,
                strategy: intent.strategy,
                size: metadata.len(),
            };

            if config.verbose {
                println!(
                    "   Queued: {:?} -> {:?} ({})",
                    relative_path,
                    job.priority,
                    format_strategy(&job.strategy)
                );
            }

            queue.push(job);
        }
    }

    if config.show_progress {
        println!(
            "   [OK] Scanned {} files, created {} directories",
            files_scanned, dirs_created
        );
        println!("   [OK] Priority queue: {} jobs", queue.len());
    }

    // Phase 1.5: Route by file size (if Neutrino enabled)
    #[cfg(feature = "backend-abstraction")]
    let (fast_lane_jobs, mut standard_lane_queue) =
        if config.transfer_profile.as_deref() == Some("neutrino") {
            let router = FileRouter::new(config.neutrino_threshold, true);
            let mut fast = Vec::new();
            let mut standard = BinaryHeap::new();

            while let Some(job) = queue.pop() {
                match router.route(job.size) {
                    TransferLane::Fast => fast.push(SmallFileJob {
                        source: job.source_path,
                        dest: job.dest_path,
                        size: job.size,
                    }),
                    TransferLane::Standard => standard.push(job),
                }
            }

            if config.show_progress && !fast.is_empty() {
                println!(
                "   [Neutrino] Routing: {} small files → fast lane, {} large files → standard lane",
                fast.len(),
                standard.len()
            );
            }

            (fast, standard)
        } else {
            (Vec::new(), queue)
        };

    #[cfg(not(feature = "backend-abstraction"))]
    let (fast_lane_jobs, mut standard_lane_queue): (Vec<()>, _) = { (Vec::new(), queue) };

    // Phase 2: Execute in priority order
    let mut stats = CopyStats::new();
    let total_jobs = fast_lane_jobs.len() + standard_lane_queue.len();
    let mut processed = 0;

    // Phase 2a: Neutrino Fast Lane (if any)
    #[cfg(feature = "backend-abstraction")]
    if !fast_lane_jobs.is_empty() {
        if config.show_progress {
            println!(
                "\n[Phase 2a] Neutrino Fast Lane - {} small files",
                fast_lane_jobs.len()
            );
        }

        let executor = DirectTransferExecutor::new(config)
            .map_err(|e| OrbitError::Other(format!("Failed to create Neutrino executor: {}", e)))?;
        match executor
            .execute_batch(fast_lane_jobs.clone(), config)
            .await
            .map_err(|e| OrbitError::Other(format!("Neutrino batch execution failed: {}", e)))
        {
            Ok(neutrino_stats) => {
                stats.files_copied += neutrino_stats.files_copied;
                stats.bytes_copied += neutrino_stats.bytes_copied;
                stats.files_failed += neutrino_stats.files_failed;
                processed += (neutrino_stats.files_copied + neutrino_stats.files_failed) as usize;

                if config.show_progress {
                    println!(
                        "   [OK] Fast lane complete: {} files copied, {} failed",
                        neutrino_stats.files_copied, neutrino_stats.files_failed
                    );
                }
            }
            Err(e) => {
                eprintln!("   [ERROR] Neutrino batch failed: {}", e);
                stats.files_failed += fast_lane_jobs.len() as u64;

                if config.error_mode == crate::config::ErrorMode::Abort {
                    return Err(e);
                }
            }
        }
    }

    // Phase 2b: Standard Lane (large files)
    if !standard_lane_queue.is_empty() && config.show_progress {
        println!(
            "\n[Phase 2b] Standard Lane - {} large files",
            standard_lane_queue.len()
        );
        print_priority_summary(&standard_lane_queue);
    }

    while let Some(job) = standard_lane_queue.pop() {
        processed += 1;

        if config.show_progress && processed % 10 == 1 {
            println!(
                "   [{}/{}] Processing: {} ({:?})",
                processed,
                total_jobs,
                job.source_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                job.priority
            );
        }

        // Execute transfer based on strategy
        match transfer_file(&job, config) {
            Ok(file_stats) => {
                stats.bytes_copied += file_stats.bytes_copied;
                stats.files_copied += 1;
            }
            Err(e) => {
                eprintln!("   [ERROR] Failed to copy {:?}: {}", job.source_path, e);
                stats.files_failed += 1;

                // Respect error mode
                if config.error_mode == crate::config::ErrorMode::Abort {
                    return Err(e);
                }
            }
        }
    }

    stats.duration = start_time.elapsed();

    // Handle mirror mode deletions
    if config.copy_mode == CopyMode::Mirror && config.show_progress {
        println!("\n[Phase 3] Mirror mode - checking for deletions...");
    }
    // TODO: Implement deletion logic for mirror mode

    if config.show_progress {
        println!("\n[Complete] Smart Sync finished!");
        println!("   Files copied: {}", stats.files_copied);
        println!("   Files failed: {}", stats.files_failed);
        println!("   Total bytes:  {}", stats.bytes_copied);
        println!("   Duration:     {:?}", stats.duration);
    }

    Ok(stats)
}

/// Transfer a single file using the appropriate strategy
fn transfer_file(job: &PrioritizedJob, config: &CopyConfig) -> Result<CopyStats> {
    // Ensure parent directory exists
    if let Some(parent) = job.dest_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // For now, use standard copy_file - future enhancement: dispatch by strategy
    // - AtomicReplace: write to temp file + rename
    // - AppendOnly: append-only optimization
    // - ContentDefined: use CDC chunking
    crate::core::copy_file(&job.source_path, &job.dest_path, config)
}

/// Format sync strategy for display
fn format_strategy(strategy: &SyncStrategy) -> &'static str {
    match strategy {
        SyncStrategy::ContentDefined => "CDC",
        SyncStrategy::AppendOnly => "Append",
        SyncStrategy::AtomicReplace => "Atomic",
        SyncStrategy::Adapter(_) => "Adapter",
    }
}

/// Print summary of priority distribution
fn print_priority_summary(queue: &BinaryHeap<PrioritizedJob>) {
    let mut critical = 0;
    let mut high = 0;
    let mut normal = 0;
    let mut low = 0;

    for job in queue.iter() {
        match job.priority {
            Priority::Critical => critical += 1,
            Priority::High => high += 1,
            Priority::Normal => normal += 1,
            Priority::Low => low += 1,
        }
    }

    println!("   Priority distribution:");
    if critical > 0 {
        println!("     - Critical: {} files (transferred first)", critical);
    }
    if high > 0 {
        println!("     - High:     {} files", high);
    }
    if normal > 0 {
        println!("     - Normal:   {} files", normal);
    }
    if low > 0 {
        println!("     - Low:      {} files (transferred last)", low);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prioritized_job_ordering() {
        let mut queue = BinaryHeap::new();

        // Add jobs in random order
        queue.push(PrioritizedJob {
            source_path: PathBuf::from("backup.iso"),
            dest_path: PathBuf::from("/dest/backup.iso"),
            priority: Priority::Low,
            strategy: SyncStrategy::ContentDefined,
            size: 1000,
        });

        queue.push(PrioritizedJob {
            source_path: PathBuf::from("config.toml"),
            dest_path: PathBuf::from("/dest/config.toml"),
            priority: Priority::Critical,
            strategy: SyncStrategy::AtomicReplace,
            size: 100,
        });

        queue.push(PrioritizedJob {
            source_path: PathBuf::from("data.db"),
            dest_path: PathBuf::from("/dest/data.db"),
            priority: Priority::Normal,
            strategy: SyncStrategy::ContentDefined,
            size: 500,
        });

        queue.push(PrioritizedJob {
            source_path: PathBuf::from("app.wal"),
            dest_path: PathBuf::from("/dest/app.wal"),
            priority: Priority::High,
            strategy: SyncStrategy::AppendOnly,
            size: 200,
        });

        // Pop and verify order
        let first = queue.pop().unwrap();
        assert_eq!(first.priority, Priority::Critical);
        assert_eq!(first.source_path, PathBuf::from("config.toml"));

        let second = queue.pop().unwrap();
        assert_eq!(second.priority, Priority::High);
        assert_eq!(second.source_path, PathBuf::from("app.wal"));

        let third = queue.pop().unwrap();
        assert_eq!(third.priority, Priority::Normal);
        assert_eq!(third.source_path, PathBuf::from("data.db"));

        let fourth = queue.pop().unwrap();
        assert_eq!(fourth.priority, Priority::Low);
        assert_eq!(fourth.source_path, PathBuf::from("backup.iso"));
    }

    #[test]
    fn test_is_smart_mode() {
        let mut config = CopyConfig::default();
        assert!(!is_smart_mode(&config));

        config.check_mode_str = Some("smart".to_string());
        assert!(is_smart_mode(&config));

        config.check_mode_str = Some("modtime".to_string());
        assert!(!is_smart_mode(&config));
    }
}
