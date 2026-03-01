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

    /// Number of parallel file operations / workers (0 = auto-detect)
    /// For network backends, auto = 256; for local, auto = CPU count
    #[serde(default)]
    pub parallel: usize,

    /// Per-operation concurrency (e.g., multipart upload/download parts)
    /// Default: 5. Controls how many parts of a single large file are
    /// transferred simultaneously.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

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

    /// Show execution statistics summary at end of run
    #[serde(default)]
    pub show_stats: bool,

    /// Human-readable output (e.g., "1.5 GiB" instead of raw bytes)
    #[serde(default)]
    pub human_readable: bool,

    /// Output results as JSON Lines instead of human-readable text
    #[serde(default)]
    pub json_output: bool,

    // === S3 Upload Enhancement Fields (Phase 3) ===
    /// Content-Type header for S3 uploads
    #[serde(default)]
    pub s3_content_type: Option<String>,

    /// Content-Encoding header for S3 uploads
    #[serde(default)]
    pub s3_content_encoding: Option<String>,

    /// Content-Disposition header for S3 uploads
    #[serde(default)]
    pub s3_content_disposition: Option<String>,

    /// Cache-Control header for S3 uploads
    #[serde(default)]
    pub s3_cache_control: Option<String>,

    /// Expiration date for S3 objects (RFC3339 format)
    #[serde(default)]
    pub s3_expires_header: Option<String>,

    /// User-defined metadata key=value pairs for S3 uploads
    #[serde(default)]
    pub s3_user_metadata: Vec<String>,

    /// Metadata directive for S3 copy operations (COPY or REPLACE)
    #[serde(default)]
    pub s3_metadata_directive: Option<String>,

    /// Canned ACL for S3 uploads
    #[serde(default)]
    pub s3_acl: Option<String>,

    // === S3 Client Configuration Fields (Phase 4) ===
    /// Disable request signing for public S3 buckets
    #[serde(default)]
    pub s3_no_sign_request: bool,

    /// Path to AWS credentials file
    #[serde(default)]
    pub s3_credentials_file: Option<std::path::PathBuf>,

    /// AWS profile name to use
    #[serde(default)]
    pub s3_aws_profile: Option<String>,

    /// Use S3 Transfer Acceleration
    #[serde(default)]
    pub s3_use_acceleration: bool,

    /// Enable requester-pays for S3 bucket access
    #[serde(default)]
    pub s3_request_payer: bool,

    /// Disable SSL certificate verification
    #[serde(default)]
    pub s3_no_verify_ssl: bool,

    /// Use ListObjects API v1 (for older S3-compatible storage)
    #[serde(default)]
    pub s3_use_list_objects_v1: bool,

    // === Conditional Copy & Transfer Options (Phase 5/6) ===
    /// Do not overwrite existing files
    #[serde(default)]
    pub no_clobber: bool,

    /// Only copy if sizes differ
    #[serde(default)]
    pub if_size_differ: bool,

    /// Only copy if source is newer
    #[serde(default)]
    pub if_source_newer: bool,

    /// Flatten directory hierarchy (strip path components)
    #[serde(default)]
    pub flatten: bool,

    // === rsync-Inspired Improvements ===
    /// Sparse file handling mode.
    /// Auto: detect zero-heavy chunks during CDC and write holes (skip for small files).
    /// Always: always create sparse holes for zero chunks.
    /// Never: write all bytes including zeros.
    #[serde(default)]
    pub sparse_mode: crate::core::sparse::SparseMode,

    /// Preserve hardlinks: detect files sharing the same inode during
    /// directory scan and recreate hardlink groups at the destination.
    #[serde(default)]
    pub preserve_hardlinks: bool,

    /// In-place file updates: modify destination files directly instead of
    /// writing to a temp file and renaming. Saves disk space for large files
    /// where only a small portion changed.
    #[serde(default)]
    pub inplace: bool,

    /// Safety level for in-place updates.
    /// "reflink": CoW snapshot before modify (btrfs/XFS/APFS — zero cost)
    /// "journaled": log original bytes to Magnetar before overwrite (any FS)
    /// "unsafe": direct overwrite with no recovery (user opt-in only)
    #[serde(default)]
    pub inplace_safety: InplaceSafety,

    /// Enable content-aware rename/move detection. Uses Star Map chunk
    /// overlap to find destination files that share content with source
    /// files, even if renamed or moved. Uses them as delta basis.
    #[serde(default)]
    pub detect_renames: bool,

    /// Rename detection overlap threshold (0.0–1.0). Files sharing at least
    /// this fraction of chunks are considered renames. Default: 0.8 (80%).
    #[serde(default = "default_rename_threshold")]
    pub rename_threshold: f64,

    /// Reference directories for incremental backups. Unchanged files are
    /// hardlinked to the reference instead of copied. Partial chunk matches
    /// use the reference as delta basis. Like rsync --link-dest but with
    /// chunk-level granularity.
    #[serde(default)]
    pub link_dest: Vec<PathBuf>,

    /// Write a transfer journal (batch file) recording all operations.
    /// Can be replayed against other destinations with --read-batch.
    #[serde(default)]
    pub write_batch: Option<PathBuf>,

    /// Read and replay a previously recorded transfer journal.
    #[serde(default)]
    pub read_batch: Option<PathBuf>,
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
            concurrency: default_concurrency(),
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
            show_stats: false,
            human_readable: false,
            json_output: false,
            // S3 upload enhancement fields (Phase 3)
            s3_content_type: None,
            s3_content_encoding: None,
            s3_content_disposition: None,
            s3_cache_control: None,
            s3_expires_header: None,
            s3_user_metadata: Vec::new(),
            s3_metadata_directive: None,
            s3_acl: None,
            // S3 client configuration fields (Phase 4)
            s3_no_sign_request: false,
            s3_credentials_file: None,
            s3_aws_profile: None,
            s3_use_acceleration: false,
            s3_request_payer: false,
            s3_no_verify_ssl: false,
            s3_use_list_objects_v1: false,
            // Conditional copy & transfer options (Phase 5/6)
            no_clobber: false,
            if_size_differ: false,
            if_source_newer: false,
            flatten: false,
            // rsync-inspired improvements
            sparse_mode: crate::core::sparse::SparseMode::Auto,
            preserve_hardlinks: false,
            inplace: false,
            inplace_safety: InplaceSafety::Reflink,
            detect_renames: false,
            rename_threshold: default_rename_threshold(),
            link_dest: Vec::new(),
            write_batch: None,
            read_batch: None,
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

/// Safety level for in-place file updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InplaceSafety {
    /// Use copy-on-write reflinks for zero-cost snapshots before modification.
    /// Supported on btrfs, XFS (4.x+), APFS. Falls back to journaled if unavailable.
    #[default]
    Reflink,

    /// Log original bytes to Magnetar state machine before each overwrite.
    /// Works on any filesystem. Slightly slower but crash-recoverable.
    Journaled,

    /// Direct overwrite with no recovery mechanism. Only for users who
    /// understand the risk of partial writes on crash.
    Unsafe,
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

fn default_concurrency() -> usize {
    5 // Per-operation concurrency (multipart parts in flight)
}

fn default_rename_threshold() -> f64 {
    0.8 // 80% chunk overlap = likely rename
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

    /// Resolve the effective worker count based on the backend type.
    /// If `parallel` is 0 (auto), returns CPU count for local ops
    /// or DEFAULT_NETWORK_WORKERS for network ops.
    pub fn resolve_workers(&self, is_network: bool) -> usize {
        if self.parallel > 0 {
            self.parallel
        } else if is_network {
            DEFAULT_NETWORK_WORKERS
        } else {
            get_cpu_count()
        }
    }

    /// Create a configuration optimized for network transfers
    pub fn network_preset() -> Self {
        Self {
            verify_checksum: true,
            resume_enabled: true,
            compression: CompressionType::Zstd { level: 3 },
            sparse_mode: crate::core::sparse::SparseMode::Never,
            retry_attempts: 10,
            exponential_backoff: true,
            use_zero_copy: false,
            parallel: DEFAULT_NETWORK_WORKERS,
            concurrency: 5,
            generate_manifest: false,
            manifest_output_dir: None,
            chunking_strategy: ChunkingStrategy::default(),
            ..Default::default()
        }
    }
}

/// Default worker count for network backends (S3, Azure, GCS, SSH, SMB)
/// Network operations are I/O-bound, so we can have many more concurrent
/// operations than CPU cores. Optimized for I/O-bound network operations (256 concurrent workers).
pub const DEFAULT_NETWORK_WORKERS: usize = 256;

/// Default worker count for local-to-local transfers (CPU-bound)
pub const DEFAULT_LOCAL_WORKERS: usize = 0; // 0 = auto-detect CPU count

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
        assert_eq!(config.concurrency, 5);
        assert!(!config.show_stats);
        assert!(!config.human_readable);
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
        assert_eq!(config.parallel, DEFAULT_NETWORK_WORKERS);
        assert_eq!(config.concurrency, 5);
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
    fn test_concurrency_default() {
        assert_eq!(default_concurrency(), 5);
    }

    #[test]
    fn test_resolve_workers_explicit() {
        let config = CopyConfig {
            parallel: 32,
            ..Default::default()
        };
        // Explicit value is always used regardless of backend type
        assert_eq!(config.resolve_workers(false), 32);
        assert_eq!(config.resolve_workers(true), 32);
    }

    #[test]
    fn test_resolve_workers_auto_local() {
        let config = CopyConfig::default(); // parallel = 0
        let workers = config.resolve_workers(false);
        // Should be CPU count, which is at least 1
        assert!(workers >= 1);
        assert!(workers <= 512); // sanity upper bound
    }

    #[test]
    fn test_resolve_workers_auto_network() {
        let config = CopyConfig::default(); // parallel = 0
        let workers = config.resolve_workers(true);
        assert_eq!(workers, DEFAULT_NETWORK_WORKERS);
    }

    #[test]
    fn test_network_workers_constant() {
        assert_eq!(DEFAULT_NETWORK_WORKERS, 256);
    }

    #[test]
    fn test_local_workers_constant() {
        assert_eq!(DEFAULT_LOCAL_WORKERS, 0);
    }

    #[test]
    fn test_new_fields_serialization() {
        let config = CopyConfig {
            concurrency: 10,
            show_stats: true,
            human_readable: true,
            ..Default::default()
        };

        let toml = toml::to_string(&config).unwrap();
        let deserialized: CopyConfig = toml::from_str(&toml).unwrap();
        assert_eq!(deserialized.concurrency, 10);
        assert!(deserialized.show_stats);
        assert!(deserialized.human_readable);
    }

    #[test]
    fn test_new_fields_deserialization_with_defaults() {
        // Old config files without new fields should still deserialize
        let toml_str = r#"
copy_mode = "copy"
recursive = false
"#;
        let config: CopyConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.concurrency, 5); // default
        assert!(!config.show_stats); // default
        assert!(!config.human_readable); // default
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

    #[test]
    fn test_default_config_new_fields() {
        let config = CopyConfig::default();
        // Conditional copy & output fields
        assert!(!config.json_output);
        assert!(!config.no_clobber);
        assert!(!config.if_size_differ);
        assert!(!config.if_source_newer);
        assert!(!config.flatten);
        // S3 upload enhancement fields (Phase 3)
        assert!(config.s3_content_type.is_none());
        assert!(config.s3_content_encoding.is_none());
        assert!(config.s3_content_disposition.is_none());
        assert!(config.s3_cache_control.is_none());
        assert!(config.s3_expires_header.is_none());
        assert!(config.s3_user_metadata.is_empty());
        assert!(config.s3_metadata_directive.is_none());
        assert!(config.s3_acl.is_none());
        // S3 client configuration fields (Phase 4)
        assert!(!config.s3_no_sign_request);
        assert!(config.s3_credentials_file.is_none());
        assert!(config.s3_aws_profile.is_none());
        assert!(!config.s3_use_acceleration);
        assert!(!config.s3_request_payer);
        assert!(!config.s3_no_verify_ssl);
        assert!(!config.s3_use_list_objects_v1);
    }

    #[test]
    fn test_new_fields_serialization_roundtrip() {
        let config = CopyConfig {
            // Set ALL new fields to non-default values
            json_output: true,
            no_clobber: true,
            if_size_differ: true,
            if_source_newer: true,
            flatten: true,
            s3_content_type: Some("application/octet-stream".to_string()),
            s3_content_encoding: Some("gzip".to_string()),
            s3_content_disposition: Some("attachment".to_string()),
            s3_cache_control: Some("max-age=3600".to_string()),
            s3_expires_header: Some("2030-01-01T00:00:00Z".to_string()),
            s3_user_metadata: vec!["key1=val1".to_string(), "key2=val2".to_string()],
            s3_metadata_directive: Some("REPLACE".to_string()),
            s3_acl: Some("public-read".to_string()),
            s3_no_sign_request: true,
            s3_credentials_file: Some(PathBuf::from("/home/user/.aws/credentials")),
            s3_aws_profile: Some("production".to_string()),
            s3_use_acceleration: true,
            s3_request_payer: true,
            s3_no_verify_ssl: true,
            s3_use_list_objects_v1: true,
            ..Default::default()
        };

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).unwrap();
        // Deserialize back
        let restored: CopyConfig = toml::from_str(&toml_str).unwrap();

        // Verify all fields match
        assert!(restored.json_output);
        assert!(restored.no_clobber);
        assert!(restored.if_size_differ);
        assert!(restored.if_source_newer);
        assert!(restored.flatten);
        assert_eq!(
            restored.s3_content_type,
            Some("application/octet-stream".to_string())
        );
        assert_eq!(restored.s3_content_encoding, Some("gzip".to_string()));
        assert_eq!(
            restored.s3_content_disposition,
            Some("attachment".to_string())
        );
        assert_eq!(restored.s3_cache_control, Some("max-age=3600".to_string()));
        assert_eq!(
            restored.s3_expires_header,
            Some("2030-01-01T00:00:00Z".to_string())
        );
        assert_eq!(restored.s3_user_metadata, vec!["key1=val1", "key2=val2"]);
        assert_eq!(restored.s3_metadata_directive, Some("REPLACE".to_string()));
        assert_eq!(restored.s3_acl, Some("public-read".to_string()));
        assert!(restored.s3_no_sign_request);
        assert_eq!(
            restored.s3_credentials_file,
            Some(PathBuf::from("/home/user/.aws/credentials"))
        );
        assert_eq!(restored.s3_aws_profile, Some("production".to_string()));
        assert!(restored.s3_use_acceleration);
        assert!(restored.s3_request_payer);
        assert!(restored.s3_no_verify_ssl);
        assert!(restored.s3_use_list_objects_v1);
    }

    #[test]
    fn test_log_level_to_tracing() {
        assert_eq!(LogLevel::Error.to_tracing_level(), tracing::Level::ERROR);
        assert_eq!(LogLevel::Warn.to_tracing_level(), tracing::Level::WARN);
        assert_eq!(LogLevel::Info.to_tracing_level(), tracing::Level::INFO);
        assert_eq!(LogLevel::Debug.to_tracing_level(), tracing::Level::DEBUG);
        assert_eq!(LogLevel::Trace.to_tracing_level(), tracing::Level::TRACE);
    }

    #[test]
    fn test_neutrino_threshold_default() {
        assert_eq!(default_neutrino_threshold(), 8192);
    }

    #[test]
    fn test_config_with_s3_fields_toml() {
        let toml_str = r#"
copy_mode = "copy"
recursive = true
s3_content_type = "text/html"
s3_content_encoding = "br"
s3_content_disposition = "inline"
s3_cache_control = "no-cache"
s3_expires_header = "2025-12-31T23:59:59Z"
s3_user_metadata = ["env=prod", "version=2"]
s3_metadata_directive = "COPY"
s3_acl = "private"
s3_no_sign_request = true
s3_credentials_file = "/etc/aws/creds"
s3_aws_profile = "staging"
s3_use_acceleration = true
s3_request_payer = true
s3_no_verify_ssl = true
s3_use_list_objects_v1 = true
"#;
        let config: CopyConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.s3_content_type, Some("text/html".to_string()));
        assert_eq!(config.s3_content_encoding, Some("br".to_string()));
        assert_eq!(config.s3_content_disposition, Some("inline".to_string()));
        assert_eq!(config.s3_cache_control, Some("no-cache".to_string()));
        assert_eq!(
            config.s3_expires_header,
            Some("2025-12-31T23:59:59Z".to_string())
        );
        assert_eq!(config.s3_user_metadata, vec!["env=prod", "version=2"]);
        assert_eq!(config.s3_metadata_directive, Some("COPY".to_string()));
        assert_eq!(config.s3_acl, Some("private".to_string()));
        assert!(config.s3_no_sign_request);
        assert_eq!(
            config.s3_credentials_file,
            Some(PathBuf::from("/etc/aws/creds"))
        );
        assert_eq!(config.s3_aws_profile, Some("staging".to_string()));
        assert!(config.s3_use_acceleration);
        assert!(config.s3_request_payer);
        assert!(config.s3_no_verify_ssl);
        assert!(config.s3_use_list_objects_v1);
    }
}
