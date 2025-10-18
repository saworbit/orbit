/*!
 * Configuration types for Orbit
 */

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Chunking strategy for manifest generation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkingStrategy {
    /// Content-defined chunking with average size in KiB
    Cdc { avg_kib: u32, algo: String },
    /// Fixed-size chunks in KiB
    Fixed { size_kib: u32 },
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        ChunkingStrategy::Cdc {
            avg_kib: 256,
            algo: "gear".to_string(),
        }
    }
}

/// Main configuration for copy operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyConfig {
    /// Copy mode (copy, sync, update, mirror)
    #[serde(default)]
    pub copy_mode: CopyMode,
    
    /// Enable recursive directory copying
    #[serde(default)]
    pub recursive: bool,
    
    /// Preserve file metadata (timestamps, permissions)
    #[serde(default)]
    pub preserve_metadata: bool,
    
    /// Enable resume capability for interrupted transfers
    #[serde(default)]
    pub resume_enabled: bool,
    
    /// Enable checksum verification
    #[serde(default = "default_true")]
    pub verify_checksum: bool,
    
    /// Compression type
    #[serde(default)]
    pub compression: CompressionType,
    
    /// Show progress bar
    #[serde(default = "default_true")]
    pub show_progress: bool,
    
    /// Chunk size in bytes for buffered I/O
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    
    /// Number of retry attempts on failure
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    
    /// Retry delay in seconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay_secs: u64,
    
    /// Use exponential backoff for retries
    #[serde(default)]
    pub exponential_backoff: bool,
    
    /// Maximum bandwidth in bytes per second (0 = unlimited)
    #[serde(default)]
    pub max_bandwidth: u64,
    
    /// Number of parallel operations (0 = sequential)
    #[serde(default)]
    pub parallel: usize,
    
    /// Symbolic link handling mode
    #[serde(default)]
    pub symlink_mode: SymlinkMode,
    
    /// Exclude patterns (glob patterns)
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    
    /// Dry run mode (don't actually copy)
    #[serde(default)]
    pub dry_run: bool,
    
    /// Use zero-copy system calls when available
    #[serde(default = "default_true")]
    pub use_zero_copy: bool,
    
    /// Generate manifests for transfers
    #[serde(default)]
    pub generate_manifest: bool,
    
    /// Output directory for manifests
    #[serde(default)]
    pub manifest_output_dir: Option<PathBuf>,
    
    /// Chunking strategy for manifest generation
    #[serde(default)]
    pub chunking_strategy: ChunkingStrategy,
}

impl Default for CopyConfig {
    fn default() -> Self {
        Self {
            copy_mode: CopyMode::Copy,
            recursive: false,
            preserve_metadata: false,
            resume_enabled: false,
            verify_checksum: true,
            compression: CompressionType::None,
            show_progress: true,
            chunk_size: default_chunk_size(),
            retry_attempts: default_retry_attempts(),
            retry_delay_secs: default_retry_delay(),
            exponential_backoff: false,
            max_bandwidth: 0,
            parallel: 0,
            symlink_mode: SymlinkMode::Skip,
            exclude_patterns: Vec::new(),
            dry_run: false,
            use_zero_copy: true,
            generate_manifest: false,
            manifest_output_dir: None,
            chunking_strategy: ChunkingStrategy::default(),
        }
    }
}

/// Copy mode determines how files are copied
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopyMode {
    /// Copy all files unconditionally
    Copy,
    
    /// Only copy if source is newer or different size
    Sync,
    
    /// Only copy if source is newer
    Update,
    
    /// Mirror copy and delete files in destination that don't exist in source
    Mirror,
}

impl Default for CopyMode {
    fn default() -> Self {
        CopyMode::Copy
    }
}

/// Compression type for file transfers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    /// No compression
    None,
    
    /// LZ4 compression (fast)
    Lz4,
    
    /// Zstd compression with level (1-22)
    #[serde(rename = "zstd")]
    Zstd { level: i32 },
}

impl Default for CompressionType {
    fn default() -> Self {
        CompressionType::None
    }
}

/// Symbolic link handling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymlinkMode {
    /// Skip symbolic links
    Skip,
    
    /// Follow symbolic links and copy target
    Follow,
    
    /// Preserve symbolic links as-is
    Preserve,
}

impl Default for SymlinkMode {
    fn default() -> Self {
        SymlinkMode::Skip
    }
}

/// Format for audit logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditFormat {
    /// JSON Lines format (one JSON object per line)
    Json,
    /// CSV format with header
    Csv,
}

impl Default for AuditFormat {
    fn default() -> Self {
        Self::Json
    }
}

// Default value functions for serde
fn default_true() -> bool {
    true
}

fn default_chunk_size() -> usize {
    1024 * 1024 // 1 MB
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_retry_delay() -> u64 {
    5
}

impl CopyConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: CopyConfig = toml::from_str(&contents)?;
        Ok(config)
    }
    
    /// Save configuration to a TOML file
    pub fn to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
    
    /// Create a configuration optimized for maximum speed
    pub fn fast_preset() -> Self {
        Self {
            verify_checksum: false,
            resume_enabled: false,
            compression: CompressionType::None,
            use_zero_copy: true,
            parallel: get_cpu_count(),
            generate_manifest: false,
            manifest_output_dir: None,
            chunking_strategy: ChunkingStrategy::default(),
            ..Default::default()
        }
    }
    
    /// Create a configuration optimized for reliability
    pub fn safe_preset() -> Self {
        Self {
            verify_checksum: true,
            resume_enabled: true,
            retry_attempts: 5,
            exponential_backoff: true,
            use_zero_copy: false,
            generate_manifest: false,
            manifest_output_dir: None,
            chunking_strategy: ChunkingStrategy::default(),
            ..Default::default()
        }
    }
    
    /// Create a configuration optimized for network transfers
    pub fn network_preset() -> Self {
        Self {
            verify_checksum: true,
            resume_enabled: true,
            compression: CompressionType::Zstd { level: 3 },
            retry_attempts: 10,
            exponential_backoff: true,
            use_zero_copy: false,
            generate_manifest: false,
            manifest_output_dir: None,
            chunking_strategy: ChunkingStrategy::default(),
            ..Default::default()
        }
    }
}

/// Get the number of available CPU cores
fn get_cpu_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CopyConfig::default();
        assert_eq!(config.copy_mode, CopyMode::Copy);
        assert!(config.verify_checksum);
        assert!(config.use_zero_copy);
        assert!(!config.resume_enabled);
        assert!(!config.generate_manifest);
    }

    #[test]
    fn test_fast_preset() {
        let config = CopyConfig::fast_preset();
        assert!(!config.verify_checksum);
        assert!(config.use_zero_copy);
        assert!(config.parallel > 0);
        assert!(!config.generate_manifest);
    }

    #[test]
    fn test_safe_preset() {
        let config = CopyConfig::safe_preset();
        assert!(config.verify_checksum);
        assert!(config.resume_enabled);
        assert!(!config.use_zero_copy);
    }

    #[test]
    fn test_network_preset() {
        let config = CopyConfig::network_preset();
        assert!(config.verify_checksum);
        assert!(config.resume_enabled);
        assert!(matches!(config.compression, CompressionType::Zstd { .. }));
        assert!(!config.use_zero_copy);
    }

    #[test]
    fn test_serialization() {
        let config = CopyConfig::default();
        let toml = toml::to_string(&config).unwrap();
        let deserialized: CopyConfig = toml::from_str(&toml).unwrap();
        assert_eq!(config.use_zero_copy, deserialized.use_zero_copy);
    }
    
    #[test]
    fn test_cpu_count() {
        let count = get_cpu_count();
        assert!(count > 0, "CPU count should be greater than 0");
        assert!(count <= 256, "CPU count seems unreasonably high");
    }
    
    #[test]
    fn test_default_values() {
        assert_eq!(default_chunk_size(), 1024 * 1024);
        assert_eq!(default_retry_attempts(), 3);
        assert_eq!(default_retry_delay(), 5);
        assert!(default_true());
    }
    
    #[test]
    fn test_chunking_strategy_default() {
        let strategy = ChunkingStrategy::default();
        assert!(matches!(strategy, ChunkingStrategy::Cdc { .. }));
    }
}