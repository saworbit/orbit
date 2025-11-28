//! Core-Semantic: Intent-Based Replication for Orbit V2
//!
//! This crate provides semantic analysis of files to determine HOW they should be replicated,
//! not just THAT they should be replicated. This enables intelligent prioritization and
//! strategy selection for disaster recovery scenarios.
//!
//! # Rationale
//!
//! In a disaster recovery scenario, "Time to Criticality" matters more than "Time to Completion".
//! A replicated database is useless without its config files. A VM won't boot without its kernel,
//! even if 99% of user data has been transferred.
//!
//! By analyzing file types and purposes, we can:
//! - Prioritize config files and executables over bulk data
//! - Use append-only streaming for WAL files (low latency)
//! - Skip partial sync for critical small files (atomic replacement)
//! - Schedule large blobs for background transfer
//!
//! # Example
//!
//! ```
//! use orbit_core_semantic::{SemanticRegistry, Priority};
//! use std::path::Path;
//!
//! let registry = SemanticRegistry::default();
//!
//! // Analyze a config file
//! let intent = registry.determine_intent(Path::new("app.toml"), b"[config]");
//! assert_eq!(intent.priority, Priority::Critical);
//!
//! // Analyze a database WAL
//! let intent = registry.determine_intent(Path::new("pg_wal/000001"), b"\x00\x01");
//! assert_eq!(intent.priority, Priority::High);
//!
//! // Analyze a video file
//! let intent = registry.determine_intent(Path::new("video.mp4"), b"ftyp");
//! assert_eq!(intent.priority, Priority::Low);
//! ```

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SemanticError {
    #[error("Adapter error: {0}")]
    Adapter(String),

    #[error("Invalid file type: {0}")]
    InvalidType(String),
}

pub type Result<T> = std::result::Result<T, SemanticError>;

/// Priority level for the transfer queue
///
/// Lower numeric values = higher priority.
/// This ensures critical files are replicated first during disaster recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Priority {
    /// Critical files: configs, lockfiles, keys, executables
    /// These must be transferred FIRST for system boot/recovery
    Critical = 0,

    /// High priority: WALs, transaction logs, incremental state
    /// Low-latency streaming for near-realtime replication
    High = 10,

    /// Normal priority: source code, documents, standard data files
    /// Default for most file types
    Normal = 50,

    /// Low priority: media, archives, backups, large blobs
    /// Can be deferred during recovery scenarios
    Low = 100,
}

/// Sync strategy for a file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStrategy {
    /// Use Content-Defined Chunking (CDC) for delta detection
    /// Best for large files with localized changes
    ContentDefined,

    /// Treat as append-only stream (tailing)
    /// Ideal for log files and WALs
    AppendOnly,

    /// Atomic replacement - transfer complete file or nothing
    /// For small critical files where partial state is dangerous
    AtomicReplace,

    /// Custom adapter logic (e.g., git graph traversal)
    Adapter(String),
}

/// Replication intent for a specific file
///
/// This encapsulates both the priority (when to sync) and strategy (how to sync).
#[derive(Debug, Clone)]
pub struct ReplicationIntent {
    /// Priority level for queuing
    pub priority: Priority,

    /// Synchronization strategy
    pub strategy: SyncStrategy,

    /// Human-readable description (for logging/UI)
    pub description: String,
}

impl ReplicationIntent {
    /// Create a new intent
    pub fn new(priority: Priority, strategy: SyncStrategy, description: impl Into<String>) -> Self {
        Self {
            priority,
            strategy,
            description: description.into(),
        }
    }

    /// Critical intent with atomic replacement
    pub fn critical(description: impl Into<String>) -> Self {
        Self::new(Priority::Critical, SyncStrategy::AtomicReplace, description)
    }

    /// High priority intent with append-only strategy
    pub fn high_append(description: impl Into<String>) -> Self {
        Self::new(Priority::High, SyncStrategy::AppendOnly, description)
    }

    /// Normal intent with CDC
    pub fn normal_cdc(description: impl Into<String>) -> Self {
        Self::new(Priority::Normal, SyncStrategy::ContentDefined, description)
    }

    /// Low priority intent with CDC
    pub fn low_cdc(description: impl Into<String>) -> Self {
        Self::new(Priority::Low, SyncStrategy::ContentDefined, description)
    }
}

/// Trait for semantic adapters
///
/// Each adapter analyzes a file and determines if it can handle it,
/// then provides the appropriate replication intent.
pub trait SemanticAdapter: Send + Sync {
    /// Can this adapter handle this file?
    ///
    /// # Arguments
    /// * `path` - File path for extension/name matching
    /// * `head_bytes` - First few KB of file content for magic number detection
    fn matches(&self, path: &Path, head_bytes: &[u8]) -> bool;

    /// Determine the replication intent for this file
    fn analyze(&self, path: &Path, head_bytes: &[u8]) -> ReplicationIntent;
}

// ────────────────────────────────────────────────────────────────────────────
// Built-in Adapters
// ────────────────────────────────────────────────────────────────────────────

/// Adapter for configuration files
///
/// Handles: .toml, .json, .yaml, .yml, .ini, .conf, .lock, .env
pub struct ConfigAdapter;

impl SemanticAdapter for ConfigAdapter {
    fn matches(&self, path: &Path, _head: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(
                ext.to_lowercase().as_str(),
                "toml" | "json" | "yaml" | "yml" | "ini" | "conf" | "lock" | "env"
            )
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Files without extension but well-known names
            matches!(
                name.to_lowercase().as_str(),
                "config" | "configuration" | ".env" | "dockerfile" | "makefile"
            )
        } else {
            false
        }
    }

    fn analyze(&self, path: &Path, _head: &[u8]) -> ReplicationIntent {
        ReplicationIntent::critical(format!(
            "Config: {}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        ))
    }
}

/// Adapter for Write-Ahead Logs (WAL) and transaction logs
///
/// Handles: PostgreSQL WAL, MySQL binlog, etc.
pub struct WalAdapter;

impl SemanticAdapter for WalAdapter {
    fn matches(&self, path: &Path, _head: &[u8]) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();

        // PostgreSQL WAL directory
        if path_str.contains("pg_wal") || path_str.contains("pg_xlog") {
            return true;
        }

        // Extension-based matching
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(ext.to_lowercase().as_str(), "wal" | "log" | "binlog")
        } else {
            false
        }
    }

    fn analyze(&self, path: &Path, _head: &[u8]) -> ReplicationIntent {
        ReplicationIntent::high_append(format!(
            "WAL: {}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        ))
    }
}

/// Adapter for media files (videos, images, audio)
///
/// These are typically large, immutable, and low-priority for recovery.
pub struct MediaAdapter;

impl SemanticAdapter for MediaAdapter {
    fn matches(&self, path: &Path, head: &[u8]) -> bool {
        // Extension-based matching
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(
                ext.to_lowercase().as_str(),
                "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" |  // Video
                "mp3" | "flac" | "wav" | "ogg" | "m4a" |          // Audio
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" // Images
            ) {
                return true;
            }
        }

        // Magic number detection for common formats
        if head.len() >= 12 {
            // MP4/MOV: starts with ftyp box
            if &head[4..8] == b"ftyp" {
                return true;
            }
            // PNG: 89 50 4E 47
            if head.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                return true;
            }
            // JPEG: FF D8 FF
            if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
                return true;
            }
        }

        false
    }

    fn analyze(&self, path: &Path, _head: &[u8]) -> ReplicationIntent {
        ReplicationIntent::low_cdc(format!(
            "Media: {}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        ))
    }
}

/// Default fallback adapter
///
/// Used when no specific adapter matches. Applies normal priority with CDC.
pub struct DefaultAdapter;

impl SemanticAdapter for DefaultAdapter {
    fn matches(&self, _path: &Path, _head: &[u8]) -> bool {
        true // Always matches as fallback
    }

    fn analyze(&self, path: &Path, _head: &[u8]) -> ReplicationIntent {
        ReplicationIntent::normal_cdc(format!(
            "Standard: {}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        ))
    }
}

/// Registry of semantic adapters
///
/// Maintains an ordered list of adapters and selects the first matching one.
pub struct SemanticRegistry {
    adapters: Vec<Box<dyn SemanticAdapter>>,
}

impl SemanticRegistry {
    /// Create a new registry with custom adapters
    pub fn new(adapters: Vec<Box<dyn SemanticAdapter>>) -> Self {
        Self { adapters }
    }

    /// Add an adapter to the registry
    pub fn add_adapter(&mut self, adapter: Box<dyn SemanticAdapter>) {
        self.adapters.push(adapter);
    }

    /// Determine the replication intent for a file
    ///
    /// # Arguments
    /// * `path` - File path
    /// * `head_bytes` - First few KB of file (for magic number detection)
    ///
    /// # Returns
    /// The intent from the first matching adapter, or default intent if none match.
    pub fn determine_intent(&self, path: &Path, head_bytes: &[u8]) -> ReplicationIntent {
        for adapter in &self.adapters {
            if adapter.matches(path, head_bytes) {
                return adapter.analyze(path, head_bytes);
            }
        }

        // Fallback to default
        DefaultAdapter.analyze(path, head_bytes)
    }
}

impl Default for SemanticRegistry {
    /// Create a registry with standard adapters
    ///
    /// Order matters - adapters are checked in sequence:
    /// 1. ConfigAdapter (highest priority)
    /// 2. WalAdapter (database logs)
    /// 3. MediaAdapter (low priority blobs)
    /// 4. DefaultAdapter (catch-all)
    fn default() -> Self {
        Self::new(vec![
            Box::new(ConfigAdapter),
            Box::new(WalAdapter),
            Box::new(MediaAdapter),
            // DefaultAdapter is applied via fallback, not registered
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Normal);
        assert!(Priority::Normal < Priority::Low);
    }

    #[test]
    fn test_config_adapter() {
        let adapter = ConfigAdapter;

        // Should match config files
        assert!(adapter.matches(Path::new("app.toml"), b""));
        assert!(adapter.matches(Path::new("config.json"), b""));
        assert!(adapter.matches(Path::new("Cargo.lock"), b""));
        assert!(adapter.matches(Path::new(".env"), b""));

        // Should not match other files
        assert!(!adapter.matches(Path::new("data.bin"), b""));
        assert!(!adapter.matches(Path::new("video.mp4"), b""));

        // Intent should be critical
        let intent = adapter.analyze(Path::new("app.toml"), b"");
        assert_eq!(intent.priority, Priority::Critical);
        assert_eq!(intent.strategy, SyncStrategy::AtomicReplace);
    }

    #[test]
    fn test_wal_adapter() {
        let adapter = WalAdapter;

        // Should match WAL files
        assert!(adapter.matches(Path::new("pg_wal/000001"), b""));
        assert!(adapter.matches(Path::new("/var/lib/postgresql/pg_wal/000002"), b""));
        assert!(adapter.matches(Path::new("database.wal"), b""));

        // Should not match other files
        assert!(!adapter.matches(Path::new("app.toml"), b""));

        // Intent should be high priority with append-only
        let intent = adapter.analyze(Path::new("pg_wal/000001"), b"");
        assert_eq!(intent.priority, Priority::High);
        assert_eq!(intent.strategy, SyncStrategy::AppendOnly);
    }

    #[test]
    fn test_media_adapter() {
        let adapter = MediaAdapter;

        // Extension-based matching
        assert!(adapter.matches(Path::new("video.mp4"), b""));
        assert!(adapter.matches(Path::new("photo.jpg"), b""));
        assert!(adapter.matches(Path::new("song.mp3"), b""));

        // Magic number matching - MP4
        let mp4_header = b"\x00\x00\x00\x20ftypmp42";
        assert!(adapter.matches(Path::new("unknown"), mp4_header));

        // Magic number matching - PNG (needs full header)
        let png_header = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0DIHDR";
        assert!(adapter.matches(Path::new("unknown"), png_header));

        // Magic number matching - JPEG (needs 12+ bytes)
        let jpeg_header = b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00\x01\x01\x00";
        assert!(adapter.matches(Path::new("unknown"), jpeg_header));

        // Intent should be low priority with CDC
        let intent = adapter.analyze(Path::new("video.mp4"), b"");
        assert_eq!(intent.priority, Priority::Low);
        assert_eq!(intent.strategy, SyncStrategy::ContentDefined);
    }

    #[test]
    fn test_default_adapter() {
        let adapter = DefaultAdapter;

        // Always matches
        assert!(adapter.matches(Path::new("anything"), b""));
        assert!(adapter.matches(Path::new("file.xyz"), b""));

        // Normal priority with CDC
        let intent = adapter.analyze(Path::new("data.bin"), b"");
        assert_eq!(intent.priority, Priority::Normal);
        assert_eq!(intent.strategy, SyncStrategy::ContentDefined);
    }

    #[test]
    fn test_registry_default() {
        let registry = SemanticRegistry::default();

        // Config files get critical priority
        let intent = registry.determine_intent(Path::new("app.toml"), b"");
        assert_eq!(intent.priority, Priority::Critical);

        // WAL files get high priority
        let intent = registry.determine_intent(Path::new("pg_wal/000001"), b"");
        assert_eq!(intent.priority, Priority::High);

        // Media files get low priority
        let intent = registry.determine_intent(Path::new("video.mp4"), b"");
        assert_eq!(intent.priority, Priority::Low);

        // Unknown files get normal priority
        let intent = registry.determine_intent(Path::new("data.bin"), b"");
        assert_eq!(intent.priority, Priority::Normal);
    }

    #[test]
    fn test_registry_order_matters() {
        let registry = SemanticRegistry::default();

        // WAL file should match WalAdapter before DefaultAdapter
        let intent = registry.determine_intent(Path::new("database.wal"), b"");
        assert_eq!(intent.priority, Priority::High);
        assert!(intent.description.contains("WAL"));
    }

    #[test]
    fn test_intent_builders() {
        let intent = ReplicationIntent::critical("test");
        assert_eq!(intent.priority, Priority::Critical);
        assert_eq!(intent.strategy, SyncStrategy::AtomicReplace);

        let intent = ReplicationIntent::high_append("test");
        assert_eq!(intent.priority, Priority::High);
        assert_eq!(intent.strategy, SyncStrategy::AppendOnly);

        let intent = ReplicationIntent::normal_cdc("test");
        assert_eq!(intent.priority, Priority::Normal);
        assert_eq!(intent.strategy, SyncStrategy::ContentDefined);

        let intent = ReplicationIntent::low_cdc("test");
        assert_eq!(intent.priority, Priority::Low);
        assert_eq!(intent.strategy, SyncStrategy::ContentDefined);
    }
}
