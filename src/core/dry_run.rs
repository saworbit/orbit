/*!
 * Dry-run simulation mode
 *
 * This module provides simulation capabilities to preview file operations
 * without actually performing them. Useful for testing and planning.
 */

use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Dry-run operation tracker
#[derive(Debug, Clone)]
pub struct DryRunSimulator {
    enabled: bool,
    operations: Vec<DryRunOperation>,
}

/// Types of operations that can be simulated
#[derive(Debug, Clone)]
pub enum DryRunOperation {
    CopyFile {
        source: PathBuf,
        dest: PathBuf,
        size: u64,
        reason: String,
    },
    CreateDirectory {
        path: PathBuf,
    },
    SkipFile {
        source: PathBuf,
        reason: String,
    },
    UpdateFile {
        source: PathBuf,
        dest: PathBuf,
        old_size: u64,
        new_size: u64,
    },
    DeleteFile {
        path: PathBuf,
    },
}

impl DryRunSimulator {
    /// Create a new dry-run simulator
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            operations: Vec::new(),
        }
    }

    /// Check if dry-run mode is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a copy operation
    pub fn record_copy(&mut self, source: &Path, dest: &Path, size: u64, reason: &str) {
        if !self.enabled {
            return;
        }

        let op = DryRunOperation::CopyFile {
            source: source.to_path_buf(),
            dest: dest.to_path_buf(),
            size,
            reason: reason.to_string(),
        };

        info!(
            target: "dry_run",
            "[DRY-RUN] Would copy: {} -> {} ({} bytes) - {}",
            source.display(),
            dest.display(),
            size,
            reason
        );

        self.operations.push(op);
    }

    /// Record a directory creation
    pub fn record_mkdir(&mut self, path: &Path) {
        if !self.enabled {
            return;
        }

        let op = DryRunOperation::CreateDirectory {
            path: path.to_path_buf(),
        };

        info!(
            target: "dry_run",
            "[DRY-RUN] Would create directory: {}",
            path.display()
        );

        self.operations.push(op);
    }

    /// Record a skip operation
    pub fn record_skip(&mut self, source: &Path, reason: &str) {
        if !self.enabled {
            return;
        }

        let op = DryRunOperation::SkipFile {
            source: source.to_path_buf(),
            reason: reason.to_string(),
        };

        debug!(
            target: "dry_run",
            "[DRY-RUN] Would skip: {} - {}",
            source.display(),
            reason
        );

        self.operations.push(op);
    }

    /// Record an update operation
    pub fn record_update(&mut self, source: &Path, dest: &Path, old_size: u64, new_size: u64) {
        if !self.enabled {
            return;
        }

        let op = DryRunOperation::UpdateFile {
            source: source.to_path_buf(),
            dest: dest.to_path_buf(),
            old_size,
            new_size,
        };

        info!(
            target: "dry_run",
            "[DRY-RUN] Would update: {} -> {} ({} -> {} bytes)",
            source.display(),
            dest.display(),
            old_size,
            new_size
        );

        self.operations.push(op);
    }

    /// Record a delete operation
    pub fn record_delete(&mut self, path: &Path) {
        if !self.enabled {
            return;
        }

        let op = DryRunOperation::DeleteFile {
            path: path.to_path_buf(),
        };

        info!(
            target: "dry_run",
            "[DRY-RUN] Would delete: {}",
            path.display()
        );

        self.operations.push(op);
    }

    /// Get all recorded operations
    pub fn operations(&self) -> &[DryRunOperation] {
        &self.operations
    }

    /// Get summary statistics
    pub fn summary(&self) -> DryRunSummary {
        let mut copy_count = 0;
        let mut update_count = 0;
        let mut skip_count = 0;
        let mut delete_count = 0;
        let mut mkdir_count = 0;
        let mut total_bytes = 0u64;

        for op in &self.operations {
            match op {
                DryRunOperation::CopyFile { size, .. } => {
                    copy_count += 1;
                    total_bytes += size;
                }
                DryRunOperation::UpdateFile { new_size, .. } => {
                    update_count += 1;
                    total_bytes += new_size;
                }
                DryRunOperation::SkipFile { .. } => skip_count += 1,
                DryRunOperation::DeleteFile { .. } => delete_count += 1,
                DryRunOperation::CreateDirectory { .. } => mkdir_count += 1,
            }
        }

        DryRunSummary {
            copy_count,
            update_count,
            skip_count,
            delete_count,
            mkdir_count,
            total_bytes,
        }
    }

    /// Print summary to stdout
    pub fn print_summary(&self) {
        if !self.enabled {
            return;
        }

        let summary = self.summary();
        println!("\n╔═══════════════════════════════════════════════╗");
        println!("║           Dry-Run Summary                     ║");
        println!("╚═══════════════════════════════════════════════╝\n");

        if summary.copy_count > 0 {
            println!("  Files to copy:    {}", summary.copy_count);
        }
        if summary.update_count > 0 {
            println!("  Files to update:  {}", summary.update_count);
        }
        if summary.skip_count > 0 {
            println!("  Files to skip:    {}", summary.skip_count);
        }
        if summary.delete_count > 0 {
            println!("  Files to delete:  {}", summary.delete_count);
        }
        if summary.mkdir_count > 0 {
            println!("  Directories to create: {}", summary.mkdir_count);
        }

        println!(
            "  Total data size:  {} ({} bytes)",
            format_bytes(summary.total_bytes),
            summary.total_bytes
        );

        println!("\n  No changes were made (dry-run mode).");
        println!("  Run without --dry-run to perform the actual transfer.\n");
    }
}

/// Summary statistics for dry-run operations
#[derive(Debug, Clone)]
pub struct DryRunSummary {
    pub copy_count: usize,
    pub update_count: usize,
    pub skip_count: usize,
    pub delete_count: usize,
    pub mkdir_count: usize,
    pub total_bytes: u64,
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_dry_run_disabled() {
        let mut sim = DryRunSimulator::new(false);
        assert!(!sim.is_enabled());

        sim.record_copy(Path::new("a.txt"), Path::new("b.txt"), 1000, "test");
        assert_eq!(sim.operations().len(), 0);
    }

    #[test]
    fn test_dry_run_enabled() {
        let mut sim = DryRunSimulator::new(true);
        assert!(sim.is_enabled());

        sim.record_copy(Path::new("a.txt"), Path::new("b.txt"), 1000, "new file");
        sim.record_skip(Path::new("c.txt"), "already exists");
        sim.record_mkdir(Path::new("dir"));

        assert_eq!(sim.operations().len(), 3);

        let summary = sim.summary();
        assert_eq!(summary.copy_count, 1);
        assert_eq!(summary.skip_count, 1);
        assert_eq!(summary.mkdir_count, 1);
        assert_eq!(summary.total_bytes, 1000);
    }

    #[test]
    fn test_summary() {
        let mut sim = DryRunSimulator::new(true);

        sim.record_copy(Path::new("a.txt"), Path::new("b.txt"), 1000, "new");
        sim.record_update(Path::new("c.txt"), Path::new("d.txt"), 500, 1500);
        sim.record_delete(Path::new("e.txt"));

        let summary = sim.summary();
        assert_eq!(summary.copy_count, 1);
        assert_eq!(summary.update_count, 1);
        assert_eq!(summary.delete_count, 1);
        assert_eq!(summary.total_bytes, 2500);
    }
}
