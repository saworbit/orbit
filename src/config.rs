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

    /// Detailed metadata preservation flags (overrides preserve_metadata if set)
    /// Format: "times,perms,owners,xattrs" or "all"
    #[serde(default)]
    pub preserve_flags: Option<String>,

    /// Metadata transformation configuration
    /// Format: "rename:pattern=replacement,case:lower,strip:xattrs"
    #[serde(default)]
    pub transform: Option<String>,

    /// Strict metadata preservation (fail on any metadata error)
    #[serde(default)]
    pub strict_metadata: bool,

    /// Verify metadata after transfer
    #[serde(default)]
    pub verify_metadata: bool,

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

    /// Error handling mode
    #[serde(default)]
    pub error_mode: ErrorMode,

    /// Log level for diagnostic output
    #[serde(default)]
    pub log_level: LogLevel,

    /// Log file path (None = stdout)
    #[serde(default)]
    pub log_file: Option<PathBuf>,

    /// Enable verbose logging (shorthand for log_level = debug)
    #[serde(default)]
    pub verbose: bool,

    /// Include patterns (glob, regex, or path - can be specified multiple times)
    #[serde(default)]
    pub include_patterns: Vec<String>,

    /// Exclude patterns (glob, regex, or path - can be specified multiple times)
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Load filter rules from a file
    #[serde(default)]
    pub filter_from: Option<PathBuf>,

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

    /// Audit log format
    #[serde(default)]
    pub audit_format: AuditFormat,

    /// Path to audit log file
    #[serde(default)]
    pub audit_log_path: Option<PathBuf>,

    // Delta detection options
    /// Check mode for delta detection (modtime, size, checksum, delta)
    #[serde(default)]
    pub check_mode: crate::core::delta::CheckMode,

    /// Block size for delta algorithm (default: 1MB)
    #[serde(default = "default_delta_block_size")]
    pub delta_block_size: usize,

    /// Force whole file copy, disable delta
    #[serde(default)]
    pub whole_file: bool,

    /// Update manifest database after transfer
    #[serde(default)]
    pub update_manifest: bool,

    /// Skip files that already exist at destination
    #[serde(default)]
    pub ignore_existing: bool,

    /// Hash algorithm for delta (blake3, md5, sha256)
    #[serde(default)]
    pub delta_hash_algorithm: crate::core::delta::HashAlgorithm,

    /// Enable parallel hashing for delta
    #[serde(default = "default_true")]
    pub parallel_hashing: bool,

    /// Path to delta manifest database
    #[serde(default)]
    pub delta_manifest_path: Option<PathBuf>,

    /// Enable resume capability for delta transfers (default: true)
    #[serde(default = "default_true")]
    pub delta_resume_enabled: bool,

    /// Chunk size for delta resume tracking (default: 1MB)
    #[serde(default = "default_delta_block_size")]
    pub delta_chunk_size: usize,

    /// Raw check mode string for custom modes like "smart" (V2 integration)
    /// Falls back to standard check_mode if not recognized
    #[serde(default)]
    pub check_mode_str: Option<String>,

    /// Transfer profile: "neutrino" for small-file optimization
    /// Options: "standard", "neutrino", "adaptive"
    #[serde(default)]
    pub transfer_profile: Option<String>,

    /// Neutrino threshold in bytes (default: 8192 = 8KB)
    /// Files smaller than this use the fast lane
    #[serde(default = "default_neutrino_threshold")]
    pub neutrino_threshold: u64,

    // === V3 Observability Configuration ===
    /// OpenTelemetry OTLP endpoint for distributed tracing
    /// Example: "http://localhost:4317" (Jaeger, Honeycomb, Datadog)
    #[serde(default)]
    pub otel_endpoint: Option<String>,

    /// Prometheus metrics HTTP endpoint port
    /// Set to enable /metrics endpoint (e.g., 9090)
    #[serde(default)]
    pub metrics_port: Option<u16>,
}

impl Default for CopyConfig {
    fn default() -> Self {
        Self {
            copy_mode: CopyMode::Copy,
            recursive: false,
            preserve_metadata: false,
            preserve_flags: None,
            transform: None,
            strict_metadata: false,
            verify_metadata: false,
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
            error_mode: ErrorMode::Abort,
            log_level: LogLevel::Info,
            log_file: None,
            verbose: false,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            filter_from: None,
            dry_run: false,
            use_zero_copy: true,
            generate_manifest: false,
            manifest_output_dir: None,
            chunking_strategy: ChunkingStrategy::default(),
            audit_format: AuditFormat::Json,
            audit_log_path: None,
            check_mode: crate::core::delta::CheckMode::ModTime,
            delta_block_size: default_delta_block_size(),
            whole_file: false,
            update_manifest: false,
            ignore_existing: false,
            delta_hash_algorithm: crate::core::delta::HashAlgorithm::Blake3,
            parallel_hashing: true,
            delta_manifest_path: None,
            check_mode_str: None,
            delta_resume_enabled: true,
            delta_chunk_size: default_delta_block_size(),
            transfer_profile: None,
            neutrino_threshold: default_neutrino_threshold(),
            otel_endpoint: None,
            metrics_port: None,
        }
    }
}

/// Copy mode determines how files are copied
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CopyMode {
    /// Copy all files unconditionally
    #[default]
    Copy,

    /// Only copy if source is newer or different size
    Sync,

    /// Only copy if source is newer
    Update,

    /// Mirror copy and delete files in destination that don't exist in source
    Mirror,
}

/// Compression type for file transfers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    /// No compression
    #[default]
    None,

    /// LZ4 compression (fast)
    Lz4,

    /// Zstd compression with level (1-22)
    #[serde(rename = "zstd")]
    Zstd { level: i32 },
}

/// Symbolic link handling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SymlinkMode {
    /// Skip symbolic links
    #[default]
    Skip,

    /// Follow symbolic links and copy target
    Follow,

    /// Preserve symbolic links as-is
    Preserve,
}

/// Error handling mode determines behavior on errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ErrorMode {
    /// Abort on first error
    #[default]
    Abort,

    /// Skip failed files and continue
    Skip,

    /// Keep partial files on error (for resume)
    Partial,
}

/// Log level for diagnostic output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Only errors
    Error,

    /// Warnings and errors
    Warn,

    /// Info, warnings, and errors
    #[default]
    Info,

    /// Debug and above
    Debug,

    /// All messages including traces
    Trace,
}

impl LogLevel {
    /// Convert to tracing::Level
    pub fn to_tracing_level(&self) -> tracing::Level {
        match self {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}

/// Format for audit logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AuditFormat {
    /// JSON Lines format (one JSON object per line)
    #[default]
    Json,
    /// CSV format with header
    Csv,
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

fn default_delta_block_size() -> usize {
    1024 * 1024 // 1 MB
}

fn default_neutrino_threshold() -> u64 {
    8 * 1024 // 8 KB
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

    #[test]
    fn test_readme_config_example() {
        // Verify the README configuration example can be deserialized
        let toml_str = r#"
copy_mode = "copy"
recursive = true
preserve_metadata = true
resume_enabled = true
verify_checksum = true
compression = { zstd = { level = 5 } }
show_progress = true
chunk_size = 1048576
retry_attempts = 3
retry_delay_secs = 2
exponential_backoff = true
max_bandwidth = 0
parallel = 4
symlink_mode = "skip"
exclude_patterns = ["*.tmp", "*.log", ".git/*", "node_modules/*"]
dry_run = false
use_zero_copy = true
generate_manifest = false
audit_format = "json"
audit_log_path = "/var/log/orbit_audit.log"
"#;

        let config: CopyConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.copy_mode, CopyMode::Copy);
        assert!(config.recursive);
        assert!(config.preserve_metadata);
        assert!(config.resume_enabled);
        assert!(config.verify_checksum);
        assert!(matches!(
            config.compression,
            CompressionType::Zstd { level: 5 }
        ));
        assert!(config.show_progress);
        assert_eq!(config.chunk_size, 1048576);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay_secs, 2);
        assert!(config.exponential_backoff);
        assert_eq!(config.max_bandwidth, 0);
        assert_eq!(config.parallel, 4);
        assert_eq!(config.symlink_mode, SymlinkMode::Skip);
        assert_eq!(config.exclude_patterns.len(), 4);
        assert!(!config.dry_run);
        assert!(config.use_zero_copy);
        assert!(!config.generate_manifest);
        assert_eq!(config.audit_format, AuditFormat::Json);
        assert_eq!(
            config.audit_log_path,
            Some(PathBuf::from("/var/log/orbit_audit.log"))
        );
    }
}
