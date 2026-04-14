/*! Orbit CLI */

use clap::{Parser, Subcommand, ValueEnum};
use orbit::{
    cli_style::{
        self, capability_table, format_bytes, format_duration, guidance_box, header_box,
        preset_table, print_error, section_header, transfer_summary_table, Icons, PresetInfo,
        Theme, TransferSummary,
    },
    commands::manifest::ManifestCommands,
    config::{
        AuditFormat, CompressionType, CopyConfig, CopyMode, ErrorMode, LogLevel, SymlinkMode,
    },
    copy_directory, copy_file,
    core::batch::TransferJournal,
    core::guidance::ConfigOptimizer,
    error::{OrbitError, Result, EXIT_SUCCESS},
    get_zero_copy_capabilities, is_zero_copy_available, logging,
    protocol::Protocol,
    stats::TransferStats,
    CopyStats,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "orbit")]
#[command(version, about = "Intelligent file transfer with compression, resume, and zero-copy optimization", long_about = None)]
struct Cli {
    // ── Transfer ────────────────────────────────────────────────────
    /// Source path or URI (positional: orbit <SOURCE> <DEST>)
    #[arg(
        short = 's',
        long = "source",
        value_name = "PATH",
        help_heading = "Transfer"
    )]
    source: Option<String>,

    /// Destination path or URI (positional: orbit <SOURCE> <DEST>)
    #[arg(
        short = 'd',
        long = "dest",
        value_name = "PATH",
        help_heading = "Transfer"
    )]
    destination: Option<String>,

    /// Source path (positional alternative to -s/--source)
    #[arg(index = 1, value_name = "SOURCE", conflicts_with = "source")]
    pos_source: Option<String>,

    /// Destination path (positional alternative to -d/--dest)
    #[arg(index = 2, value_name = "DEST", conflicts_with = "destination")]
    pos_dest: Option<String>,

    /// Configuration profile preset (fast, safe, backup, network)
    #[arg(long, value_enum, global = true, help_heading = "Transfer")]
    profile: Option<ProfileArg>,

    /// Copy mode
    #[arg(
        short = 'm',
        long = "mode",
        value_enum,
        global = true,
        help_heading = "Transfer"
    )]
    mode: Option<CopyModeArg>,

    /// Recursive copy (auto-detected for directory sources)
    #[arg(
        short = 'R',
        long = "recursive",
        global = true,
        help_heading = "Transfer"
    )]
    recursive: bool,

    /// Do not auto-detect recursive mode for directory sources
    #[arg(long, global = true, help_heading = "Transfer")]
    no_auto_recursive: bool,

    /// Preserve metadata (timestamps, permissions) [default: true]
    #[arg(
        short = 'p',
        long = "preserve-metadata",
        global = true,
        help_heading = "Transfer"
    )]
    preserve_metadata: bool,

    /// Disable metadata preservation
    #[arg(long, global = true, help_heading = "Transfer")]
    no_preserve_metadata: bool,

    /// Detailed preservation flags: times,perms,owners,xattrs (overrides -p)
    #[arg(
        long = "preserve",
        value_name = "FLAGS",
        global = true,
        help_heading = "Transfer"
    )]
    preserve_flags: Option<String>,

    /// Metadata transformation: rename:pattern=replacement,case:lower,strip:xattrs
    #[arg(
        long = "transform",
        value_name = "CONFIG",
        global = true,
        help_heading = "Transfer"
    )]
    transform: Option<String>,

    /// Strict metadata mode (fail on any metadata error)
    #[arg(long, global = true, help_heading = "Transfer")]
    strict_metadata: bool,

    /// Verify metadata after transfer
    #[arg(long, global = true, help_heading = "Transfer")]
    verify_metadata: bool,

    /// Enable resume capability
    #[arg(
        short = 'r',
        long = "resume",
        global = true,
        help_heading = "Reliability"
    )]
    resume: bool,

    /// Compression type (none, lz4, zstd, zstd:1, zstd:3, zstd:9, zstd:19)
    #[arg(
        short = 'c',
        long = "compress",
        global = true,
        help_heading = "Performance"
    )]
    compress: Option<CompressionArg>,

    /// Shorthand for --compress lz4
    #[arg(
        long,
        global = true,
        conflicts_with = "compress",
        help_heading = "Performance"
    )]
    lz4: bool,

    /// Shorthand for --compress zstd:3
    #[arg(long, global = true, conflicts_with_all = ["compress", "lz4"], help_heading = "Performance")]
    zstd: bool,

    /// Show progress bar (override config)
    #[arg(long = "show-progress", global = true, help_heading = "Output")]
    show_progress: bool,

    /// Symbolic link mode
    #[arg(long = "symlink", value_enum, global = true, help_heading = "Transfer")]
    symlink: Option<SymlinkModeArg>,

    // ── Reliability ────────────────────────────────────────────────
    /// Number of retry attempts
    #[arg(long, global = true, help_heading = "Reliability")]
    retry_attempts: Option<u32>,

    /// Initial retry delay in seconds
    #[arg(long, global = true, help_heading = "Reliability")]
    retry_delay: Option<u64>,

    /// Use exponential backoff for retries
    #[arg(long, global = true, help_heading = "Reliability")]
    exponential_backoff: bool,

    /// Error handling mode (abort, skip, partial)
    #[arg(long, value_enum, global = true, help_heading = "Reliability")]
    error_mode: Option<ErrorModeArg>,

    /// Skip checksum verification
    #[arg(long, global = true, help_heading = "Reliability")]
    no_verify: bool,

    // ── Performance ────────────────────────────────────────────────
    /// Chunk size in KB
    #[arg(long, global = true, help_heading = "Performance")]
    chunk_size: Option<usize>,

    /// Maximum bandwidth in MB/s (0 = unlimited)
    #[arg(long, global = true, help_heading = "Performance")]
    max_bandwidth: Option<u64>,

    /// Number of parallel file operations / workers (0 = auto)
    /// For network backends (S3, SMB, etc.) auto = 256; for local = CPU count.
    /// Alias: --parallel
    #[arg(long, global = true, alias = "parallel", help_heading = "Performance")]
    workers: Option<usize>,

    /// Per-operation concurrency for multipart transfers
    /// Controls how many parts of a single large file transfer in parallel.
    #[arg(long, global = true, help_heading = "Performance")]
    concurrency: Option<usize>,

    /// Multipart upload part size in MiB (min: 5, max: 5120)
    #[arg(long, global = true, help_heading = "Performance")]
    part_size: Option<usize>,

    /// Use zero-copy system calls for maximum performance
    #[arg(
        long,
        global = true,
        conflicts_with = "no_zero_copy",
        help_heading = "Performance"
    )]
    zero_copy: bool,

    /// Disable zero-copy optimization (use buffered copy)
    #[arg(
        long,
        global = true,
        conflicts_with = "zero_copy",
        help_heading = "Performance"
    )]
    no_zero_copy: bool,

    // ── Filtering ──────────────────────────────────────────────────
    /// Include patterns - glob, regex, or path (can be specified multiple times)
    /// Examples: --include="*.rs" --include="regex:^src/.*"
    #[arg(long = "include", global = true, help_heading = "Filtering")]
    include_patterns: Vec<String>,

    /// Exclude patterns - glob, regex, or path (can be specified multiple times)
    /// Examples: --exclude="*.tmp" --exclude="target/**"
    #[arg(long = "exclude", global = true, help_heading = "Filtering")]
    exclude_patterns: Vec<String>,

    /// Load filter rules from a file (one rule per line)
    /// File format: '+ pattern' (include) or '- pattern' (exclude)
    #[arg(
        long = "filter-from",
        value_name = "FILE",
        global = true,
        help_heading = "Filtering"
    )]
    filter_from: Option<PathBuf>,

    // ── Conditional Copy ───────────────────────────────────────────
    /// Do not overwrite existing destination files
    #[arg(long, short = 'n', global = true, help_heading = "Conditional Copy")]
    no_clobber: bool,

    /// Only copy if source and destination sizes differ
    #[arg(long, global = true, help_heading = "Conditional Copy")]
    if_size_differ: bool,

    /// Only copy if source is newer than destination
    #[arg(long, global = true, help_heading = "Conditional Copy")]
    if_source_newer: bool,

    /// Skip files that already exist at destination
    #[arg(long, global = true, help_heading = "Conditional Copy")]
    ignore_existing: bool,

    /// Flatten directory hierarchy during copy (strip path components)
    #[arg(long, global = true, help_heading = "Conditional Copy")]
    flatten: bool,

    // ── Output ─────────────────────────────────────────────────────
    /// Dry run - show what would be copied
    #[arg(long, global = true, help_heading = "Output")]
    dry_run: bool,

    /// Show execution statistics summary at end of run [default: true]
    #[arg(long, global = true, help_heading = "Output")]
    stat: bool,

    /// Disable execution statistics summary
    #[arg(long, global = true, help_heading = "Output")]
    no_stat: bool,

    /// Human-readable output (e.g., "1.5 GiB") [default: true]
    #[arg(
        short = 'H',
        long = "human-readable",
        global = true,
        help_heading = "Output"
    )]
    human_readable: bool,

    /// Raw byte output (disable human-readable formatting)
    #[arg(long, global = true, help_heading = "Output")]
    raw: bool,

    /// Hide progress bar
    #[arg(long, global = true, help_heading = "Output")]
    no_progress: bool,

    /// Suppress all non-essential output
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    quiet: bool,

    /// Output results as JSON Lines (one JSON object per line)
    #[arg(long, global = true, help_heading = "Output")]
    json: bool,

    // ── Observability ──────────────────────────────────────────────
    /// Audit log format
    #[arg(long, value_enum, global = true, help_heading = "Observability")]
    audit_format: Option<AuditFormatArg>,

    /// Path to audit log file
    #[arg(long, global = true, help_heading = "Observability")]
    audit_log: Option<PathBuf>,

    /// OpenTelemetry OTLP endpoint for distributed tracing (e.g., http://localhost:4317)
    #[arg(long, global = true, help_heading = "Observability")]
    otel_endpoint: Option<String>,

    /// Prometheus metrics HTTP endpoint port
    #[arg(long, global = true, help_heading = "Observability")]
    metrics_port: Option<u16>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, value_enum, global = true, help_heading = "Observability")]
    log_level: Option<LogLevelArg>,

    /// Path to log file (default: stdout)
    #[arg(
        long,
        value_name = "FILE",
        global = true,
        help_heading = "Observability"
    )]
    log: Option<PathBuf>,

    /// Enable verbose logging (equivalent to --log-level=debug)
    #[arg(short = 'v', long, global = true, help_heading = "Observability")]
    verbose: bool,

    /// Path to config file (overrides default locations)
    #[arg(long, global = true, help_heading = "Observability")]
    config: Option<PathBuf>,

    // ── S3 Options ─────────────────────────────────────────────────
    /// Content-Type header for S3 uploads
    #[arg(long, global = true, help_heading = "S3 Options")]
    content_type: Option<String>,

    /// Content-Encoding header for S3 uploads
    #[arg(long, global = true, help_heading = "S3 Options")]
    content_encoding: Option<String>,

    /// Content-Disposition header for S3 uploads
    #[arg(long, global = true, help_heading = "S3 Options")]
    content_disposition: Option<String>,

    /// Cache-Control header for S3 uploads
    #[arg(long, global = true, help_heading = "S3 Options")]
    cache_control: Option<String>,

    /// Expiration date for S3 objects (RFC3339 format)
    #[arg(long = "expires-header", global = true, help_heading = "S3 Options")]
    expires_header: Option<String>,

    /// User-defined metadata key=value pairs for S3 uploads
    #[arg(long = "metadata", global = true, help_heading = "S3 Options")]
    user_metadata: Vec<String>,

    /// Metadata directive for S3 copy operations (COPY or REPLACE)
    #[arg(long, global = true, help_heading = "S3 Options")]
    metadata_directive: Option<String>,

    /// Canned ACL for S3 uploads (e.g., private, public-read, bucket-owner-full-control)
    #[arg(long, global = true, help_heading = "S3 Options")]
    acl: Option<String>,

    /// Disable request signing for public S3 buckets
    #[arg(long, global = true, help_heading = "S3 Options")]
    no_sign_request: bool,

    /// Path to AWS credentials file
    #[arg(long, global = true, help_heading = "S3 Options")]
    credentials_file: Option<PathBuf>,

    /// AWS profile name to use
    #[arg(long = "aws-profile", global = true, help_heading = "S3 Options")]
    aws_profile: Option<String>,

    /// Use S3 Transfer Acceleration
    #[arg(long, global = true, help_heading = "S3 Options")]
    use_acceleration: bool,

    /// Enable requester-pays for S3 bucket access
    #[arg(long, global = true, help_heading = "S3 Options")]
    request_payer: bool,

    /// Disable SSL certificate verification (use with caution)
    #[arg(long, global = true, help_heading = "S3 Options")]
    no_verify_ssl: bool,

    /// Use ListObjects API v1 (for older S3-compatible storage)
    #[arg(long, global = true, help_heading = "S3 Options")]
    use_list_objects_v1: bool,

    // ── Advanced ───────────────────────────────────────────────────
    /// Generate manifests for transfer verification and audit
    #[arg(long, global = true, help_heading = "Advanced")]
    generate_manifest: bool,

    /// Output directory for manifests
    #[arg(
        long,
        global = true,
        requires = "generate_manifest",
        help_heading = "Advanced"
    )]
    manifest_dir: Option<PathBuf>,

    /// Check mode for change detection (mod-time, size, checksum, delta)
    #[arg(long, value_enum, global = true, help_heading = "Advanced")]
    check: Option<CheckModeArg>,

    /// Block size for delta algorithm (in KB)
    #[arg(long, global = true, help_heading = "Advanced")]
    block_size: Option<usize>,

    /// Force whole file copy, disable delta optimization
    #[arg(long, global = true, help_heading = "Advanced")]
    whole_file: bool,

    /// Update manifest database after transfer
    #[arg(long, global = true, help_heading = "Advanced")]
    update_manifest: bool,

    /// Path to delta manifest database
    #[arg(long, global = true, help_heading = "Advanced")]
    delta_manifest: Option<PathBuf>,

    /// Sparse file handling mode (auto, always, never)
    #[arg(long, value_enum, global = true, help_heading = "Advanced")]
    sparse: Option<SparseModeArg>,

    /// Preserve hardlinks during directory transfers (-H)
    #[arg(long = "preserve-hardlinks", global = true, help_heading = "Advanced")]
    preserve_hardlinks: bool,

    /// Modify destination file in-place instead of temp+rename
    #[arg(long, global = true, help_heading = "Advanced")]
    inplace: bool,

    /// Safety level for in-place updates (reflink, journaled, unsafe)
    #[arg(
        long,
        value_enum,
        global = true,
        requires = "inplace",
        help_heading = "Advanced"
    )]
    inplace_safety: Option<InplaceSafetyArg>,

    /// Detect renamed/moved files via content-hash overlap at destination
    #[arg(long, global = true, help_heading = "Advanced")]
    detect_renames: bool,

    /// Minimum chunk overlap ratio to consider a rename (0.0–1.0, default: 0.8)
    #[arg(
        long,
        global = true,
        requires = "detect_renames",
        help_heading = "Advanced"
    )]
    rename_threshold: Option<f64>,

    /// Reference directory for incremental backup hardlinking (repeatable)
    #[arg(
        long = "link-dest",
        value_name = "DIR",
        global = true,
        help_heading = "Advanced"
    )]
    link_dest: Vec<PathBuf>,

    /// Record transfer operations to a batch file for replay
    #[arg(long, value_name = "FILE", global = true, help_heading = "Advanced")]
    write_batch: Option<PathBuf>,

    /// Replay a previously recorded batch file against a destination
    #[arg(long, value_name = "FILE", global = true, help_heading = "Advanced")]
    read_batch: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Orbit configuration (interactive setup wizard)
    Init,

    /// Show transfer statistics
    Stats,

    /// Show configuration presets
    Presets,

    /// Show platform capabilities
    Capabilities,

    /// Diagnose common issues (config, permissions, connectivity)
    Doctor,

    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Manifest operations for planning, verification, and auditing
    #[command(subcommand)]
    Manifest(ManifestCommands),

    /// Execute batch commands from stdin or a file (one command per line)
    ///
    /// Each line may be a full orbit invocation or a shorthand:
    /// - orbit --source <src> --dest <dest> [options]
    /// - cp|copy <src> <dest> [options]
    /// - sync <src> <dest> [options]  (adds --mode sync)
    ///   Lines starting with '#' are comments. Empty lines are skipped.
    Run {
        /// Input file with commands (default: read from stdin)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Number of parallel workers (default: 256)
        #[arg(long, default_value = "256")]
        workers: usize,
    },

    /// Sync files (shorthand for --mode sync -R -p)
    Sync {
        /// Source path or URI
        source: String,
        /// Destination path or URI
        dest: String,
    },

    /// Backup files (shorthand for --profile backup -R -p)
    Backup {
        /// Source path or URI
        source: String,
        /// Destination path or URI
        dest: String,
    },

    /// Mirror files (shorthand for --mode mirror -R -p, deletes extras at dest)
    #[command(name = "mirror")]
    MirrorCmd {
        /// Source path or URI
        source: String,
        /// Destination path or URI
        dest: String,
    },

    /// Stream an S3 object to stdout
    #[cfg(feature = "s3-native")]
    Cat {
        /// S3 URI (s3://bucket/key)
        uri: String,
    },

    /// Stream stdin to an S3 object
    #[cfg(feature = "s3-native")]
    Pipe {
        /// S3 URI (s3://bucket/key)
        uri: String,
    },

    /// Generate a pre-signed URL for an S3 object
    #[cfg(feature = "s3-native")]
    Presign {
        /// S3 URI (s3://bucket/key)
        uri: String,

        /// Expiration time in seconds (default: 3600)
        #[arg(long, default_value = "3600")]
        expires: u64,
    },

    /// List S3 objects
    #[cfg(feature = "s3-native")]
    Ls {
        /// S3 URI (s3://bucket/prefix or s3://bucket/pattern*)
        uri: String,
        /// Show entity tags
        #[arg(long, short = 'e')]
        etag: bool,
        /// Show storage class
        #[arg(long, short = 's')]
        storage_class: bool,
        /// List all object versions
        #[arg(long)]
        all_versions: bool,
        /// Show full object path
        #[arg(long)]
        show_fullpath: bool,
    },

    /// Show S3 object metadata
    #[cfg(feature = "s3-native")]
    Head {
        /// S3 URI (s3://bucket/key)
        uri: String,
        /// Version ID to inspect
        #[arg(long)]
        version_id: Option<String>,
    },

    /// Show S3 storage usage (object count and total size)
    #[cfg(feature = "s3-native")]
    Du {
        /// S3 URI (s3://bucket/prefix)
        uri: String,
        /// Group results by storage class
        #[arg(long, short = 'g')]
        group: bool,
        /// Include all object versions
        #[arg(long)]
        all_versions: bool,
    },

    /// Delete S3 objects
    #[cfg(feature = "s3-native")]
    Rm {
        /// S3 URI (s3://bucket/key or s3://bucket/pattern*)
        uri: String,
        /// Delete all versions
        #[arg(long)]
        all_versions: bool,
        /// Delete specific version
        #[arg(long)]
        version_id: Option<String>,
        /// Dry run - show what would be deleted
        #[arg(long)]
        dry_run: bool,
    },

    /// Move (rename) S3 objects
    #[cfg(feature = "s3-native")]
    Mv {
        /// Source S3 URI
        source: String,
        /// Destination S3 URI
        dest: String,
    },

    /// Create an S3 bucket
    #[cfg(feature = "s3-native")]
    Mb {
        /// Bucket name (s3://bucket-name)
        bucket: String,
    },

    /// Remove an S3 bucket
    #[cfg(feature = "s3-native")]
    Rb {
        /// Bucket name (s3://bucket-name)
        bucket: String,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CopyModeArg {
    Copy,
    Sync,
    Update,
    Mirror,
}

impl From<CopyModeArg> for CopyMode {
    fn from(mode: CopyModeArg) -> Self {
        match mode {
            CopyModeArg::Copy => CopyMode::Copy,
            CopyModeArg::Sync => CopyMode::Sync,
            CopyModeArg::Update => CopyMode::Update,
            CopyModeArg::Mirror => CopyMode::Mirror,
        }
    }
}

/// Configuration profile presets
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ProfileArg {
    /// Maximum speed: zero-copy, no checksums, no resume
    Fast,
    /// Maximum reliability: checksums, resume, retries
    Safe,
    /// Optimized for backup: reliability + compression
    Backup,
    /// Optimized for network/cloud: compression, resume, retries
    Network,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CompressionArg {
    None,
    Lz4,
    /// Zstd with default level 3
    Zstd,
    #[value(name = "zstd:1")]
    Zstd1,
    #[value(name = "zstd:3")]
    Zstd3,
    #[value(name = "zstd:9")]
    Zstd9,
    #[value(name = "zstd:19")]
    Zstd19,
}

impl From<CompressionArg> for CompressionType {
    fn from(comp: CompressionArg) -> Self {
        match comp {
            CompressionArg::None => CompressionType::None,
            CompressionArg::Lz4 => CompressionType::Lz4,
            CompressionArg::Zstd | CompressionArg::Zstd3 => CompressionType::Zstd { level: 3 },
            CompressionArg::Zstd1 => CompressionType::Zstd { level: 1 },
            CompressionArg::Zstd9 => CompressionType::Zstd { level: 9 },
            CompressionArg::Zstd19 => CompressionType::Zstd { level: 19 },
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum SymlinkModeArg {
    Skip,
    Follow,
    Preserve,
}

impl From<SymlinkModeArg> for SymlinkMode {
    fn from(mode: SymlinkModeArg) -> Self {
        match mode {
            SymlinkModeArg::Skip => SymlinkMode::Skip,
            SymlinkModeArg::Follow => SymlinkMode::Follow,
            SymlinkModeArg::Preserve => SymlinkMode::Preserve,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum AuditFormatArg {
    Json,
    Text,
}

impl From<AuditFormatArg> for AuditFormat {
    fn from(format: AuditFormatArg) -> Self {
        match format {
            AuditFormatArg::Json => AuditFormat::Json,
            AuditFormatArg::Text => AuditFormat::Csv,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CheckModeArg {
    #[value(name = "modtime", alias = "mod-time")]
    ModTime,
    Size,
    Checksum,
    Delta,
}

impl From<CheckModeArg> for orbit::core::delta::CheckMode {
    fn from(mode: CheckModeArg) -> Self {
        match mode {
            CheckModeArg::ModTime => orbit::core::delta::CheckMode::ModTime,
            CheckModeArg::Size => orbit::core::delta::CheckMode::Size,
            CheckModeArg::Checksum => orbit::core::delta::CheckMode::Checksum,
            CheckModeArg::Delta => orbit::core::delta::CheckMode::Delta,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ErrorModeArg {
    Abort,
    Skip,
    Partial,
}

impl From<ErrorModeArg> for ErrorMode {
    fn from(arg: ErrorModeArg) -> Self {
        match arg {
            ErrorModeArg::Abort => ErrorMode::Abort,
            ErrorModeArg::Skip => ErrorMode::Skip,
            ErrorModeArg::Partial => ErrorMode::Partial,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum LogLevelArg {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevelArg> for LogLevel {
    fn from(arg: LogLevelArg) -> Self {
        match arg {
            LogLevelArg::Error => LogLevel::Error,
            LogLevelArg::Warn => LogLevel::Warn,
            LogLevelArg::Info => LogLevel::Info,
            LogLevelArg::Debug => LogLevel::Debug,
            LogLevelArg::Trace => LogLevel::Trace,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum SparseModeArg {
    Auto,
    Always,
    Never,
}

impl From<SparseModeArg> for orbit::core::sparse::SparseMode {
    fn from(arg: SparseModeArg) -> Self {
        match arg {
            SparseModeArg::Auto => orbit::core::sparse::SparseMode::Auto,
            SparseModeArg::Always => orbit::core::sparse::SparseMode::Always,
            SparseModeArg::Never => orbit::core::sparse::SparseMode::Never,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum InplaceSafetyArg {
    Reflink,
    Journaled,
    Unsafe,
}

impl From<InplaceSafetyArg> for orbit::config::InplaceSafety {
    fn from(arg: InplaceSafetyArg) -> Self {
        match arg {
            InplaceSafetyArg::Reflink => orbit::config::InplaceSafety::Reflink,
            InplaceSafetyArg::Journaled => orbit::config::InplaceSafety::Journaled,
            InplaceSafetyArg::Unsafe => orbit::config::InplaceSafety::Unsafe,
        }
    }
}

/// Apply auto-network overlay to a base config for remote destinations.
///
/// For each field, if the user's config still matches `CopyConfig::default()`,
/// we upgrade it to the network-friendly value. Fields the user customized
/// are left alone.
fn apply_auto_network(base: &CopyConfig) -> CopyConfig {
    let defaults = CopyConfig::default();
    let mut cfg = base.clone();
    let net = CopyConfig::network_preset();
    if cfg.resume_enabled == defaults.resume_enabled {
        cfg.resume_enabled = net.resume_enabled;
    }
    if cfg.exponential_backoff == defaults.exponential_backoff {
        cfg.exponential_backoff = net.exponential_backoff;
    }
    if cfg.compression == defaults.compression {
        cfg.compression = net.compression;
    }
    if cfg.retry_attempts == defaults.retry_attempts {
        cfg.retry_attempts = net.retry_attempts;
    }
    if cfg.parallel == defaults.parallel {
        cfg.parallel = net.parallel;
    }
    if cfg.sparse_mode == defaults.sparse_mode {
        cfg.sparse_mode = net.sparse_mode;
    }
    cfg.use_zero_copy = false;
    cfg
}

struct OutputModeInputs {
    json_output: bool,
    quiet: bool,
    raw: bool,
    cli_human_readable: bool,
    cli_stat: bool,
    cli_no_stat: bool,
    config_human_readable: bool,
    config_show_stats: bool,
}

/// Resolve output display modes from CLI flags and config.
///
/// Priority: explicit CLI flags > json/quiet/raw overrides > config file values.
/// When no CLI flag or mode override applies, the config file value is preserved.
fn resolve_output_modes(inputs: OutputModeInputs) -> (bool, bool) {
    // Human-readable: --raw and json mode force off, --human-readable forces on,
    // otherwise respect the config file value.
    let human_readable = if inputs.raw || inputs.json_output {
        false
    } else if inputs.cli_human_readable {
        true
    } else {
        inputs.config_human_readable
    };

    // Show stats: --no-stat / json / quiet force off, --stat forces on,
    // otherwise respect the config file value.
    let show_stats = if inputs.cli_no_stat || inputs.json_output || inputs.quiet {
        false
    } else if inputs.cli_stat {
        true
    } else {
        inputs.config_show_stats
    };

    (human_readable, show_stats)
}

/// Build a resolved transfer config from base config + CLI flags.
///
/// This is the single place where profile selection, auto-network merge,
/// shorthand defaults, output-mode resolution, and all CLI overrides are
/// applied. Returns `(config, json_output, quiet)` so downstream code
/// has a single source of truth for output suppression.
///
/// Fields NOT handled here (applied separately in `run()`):
/// S3 upload/client options, audit/observability, delta detection,
/// conditional copy, batch, manifest, hardlinks, renames, link-dest.
fn resolve_transfer_config(
    cli: &Cli,
    base_config: CopyConfig,
    is_shorthand: bool,
    shorthand_mode: Option<CopyMode>,
    effective_profile: Option<ProfileArg>,
    dest_is_remote: bool,
    source_is_dir: bool,
) -> (CopyConfig, bool, bool) {
    // ── Profile / auto-network base ──────────────────────────────
    let mut config = match effective_profile {
        Some(ProfileArg::Fast) => CopyConfig::fast_preset(),
        Some(ProfileArg::Safe) => CopyConfig::safe_preset(),
        Some(ProfileArg::Backup) => CopyConfig::backup_preset(),
        Some(ProfileArg::Network) => CopyConfig::network_preset(),
        None => {
            if dest_is_remote {
                apply_auto_network(&base_config)
            } else {
                base_config
            }
        }
    };

    // ── Shorthand defaults ───────────────────────────────────────
    if is_shorthand {
        config.recursive = true;
        config.preserve_metadata = true;
    }
    if let Some(mode) = shorthand_mode {
        config.copy_mode = mode;
    }

    // ── Output modes ─────────────────────────────────────────────
    let json_output = cli.json || config.json_output;
    let quiet = cli.quiet;

    let (human_readable, show_stats) = resolve_output_modes(OutputModeInputs {
        json_output,
        quiet,
        raw: cli.raw,
        cli_human_readable: cli.human_readable,
        cli_stat: cli.stat,
        cli_no_stat: cli.no_stat,
        config_human_readable: config.human_readable,
        config_show_stats: config.show_stats,
    });
    config.human_readable = human_readable;
    config.show_stats = show_stats;

    // ── Metadata ─────────────────────────────────────────────────
    if cli.no_preserve_metadata {
        config.preserve_metadata = false;
    } else if cli.preserve_metadata || cli.preserve_flags.is_some() {
        config.preserve_metadata = true;
    }
    if cli.strict_metadata {
        config.strict_metadata = true;
    }
    if cli.verify_metadata {
        config.verify_metadata = true;
    }

    // ── Recursive ────────────────────────────────────────────────
    if !cli.no_auto_recursive && source_is_dir {
        config.recursive = true;
    }
    if cli.recursive {
        config.recursive = true;
    }

    // ── Progress ─────────────────────────────────────────────────
    // JSON mode suppresses progress to keep stdout machine-readable.
    if cli.show_progress && !json_output {
        config.show_progress = true;
    } else if cli.no_progress || quiet || json_output {
        config.show_progress = false;
    }

    // ── Core transfer overrides ──────────────────────────────────
    if let Some(mode) = cli.mode {
        config.copy_mode = mode.into();
    }
    if cli.resume {
        config.resume_enabled = true;
    }
    if let Some(attempts) = cli.retry_attempts {
        config.retry_attempts = attempts;
    }
    if let Some(delay) = cli.retry_delay {
        config.retry_delay_secs = delay;
    }
    if cli.exponential_backoff {
        config.exponential_backoff = true;
    }
    if let Some(size) = cli.chunk_size {
        config.chunk_size = size.saturating_mul(1024);
    }
    if let Some(bw) = cli.max_bandwidth {
        config.max_bandwidth = bw.saturating_mul(1024 * 1024);
    }
    if let Some(w) = cli.workers {
        config.parallel = w;
    }
    if let Some(c) = cli.concurrency {
        config.concurrency = c;
    }
    if cli.dry_run {
        config.dry_run = true;
    }
    if let Some(level) = cli.log_level {
        config.log_level = level.into();
    }
    if cli.verbose {
        config.verbose = true;
    }
    if json_output {
        config.json_output = true;
    }
    if cli.no_verify {
        config.verify_checksum = false;
    }

    // ── Compression ──────────────────────────────────────────────
    if cli.zstd {
        config.compression = CompressionType::Zstd { level: 3 };
    } else if cli.lz4 {
        config.compression = CompressionType::Lz4;
    } else if let Some(comp) = cli.compress {
        config.compression = comp.into();
    }

    // ── Zero-copy ────────────────────────────────────────────────
    if cli.zero_copy {
        config.use_zero_copy = true;
    } else if cli.no_zero_copy {
        config.use_zero_copy = false;
    }

    (config, json_output, quiet)
}

/// Try to load config from the default location (~/.orbit/orbit.toml).
/// Returns Ok(Some(config)) on success, Ok(None) if no file exists,
/// or Err with warning message if file exists but is invalid.
fn load_default_config() -> (Option<CopyConfig>, Option<String>) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return (None, None),
    };
    let path = home.join(".orbit").join("orbit.toml");
    if !path.exists() {
        return (None, None);
    }
    match CopyConfig::from_file(&path) {
        Ok(cfg) => (Some(cfg), None),
        Err(e) => (
            None,
            Some(format!(
                "Invalid config file {}: {}. Falling back to defaults.",
                path.display(),
                e
            )),
        ),
    }
}

/// Check if default config file exists
fn default_config_exists() -> bool {
    dirs::home_dir()
        .map(|h| h.join(".orbit").join("orbit.toml").exists())
        .unwrap_or(false)
}

/// Detect if a URI refers to a remote/network protocol
fn is_remote_uri(uri: &str) -> bool {
    uri.starts_with("s3://")
        || uri.starts_with("smb://")
        || uri.starts_with("ssh://")
        || uri.starts_with("azure://")
        || uri.starts_with("gs://")
        || uri.starts_with("\\\\") // UNC paths
}

fn main() {
    let code = match run() {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            print_error(&format!("{}", e), e.suggestion());
            e.exit_code()
        }
    };
    std::process::exit(code);
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Load config: explicit --config > ~/.orbit/orbit.toml > defaults
    let base_config = if let Some(ref config_path) = cli.config {
        CopyConfig::from_file(config_path).unwrap_or_else(|e| {
            cli_style::print_warning(&format!("Failed to load config file: {}", e));
            CopyConfig::default()
        })
    } else {
        let (cfg, warning) = load_default_config();
        if let Some(msg) = warning {
            cli_style::print_warning(&msg);
        }
        cfg.unwrap_or_default()
    };

    // Check if we need to initialize logging for the command
    let needs_logging = if let Some(ref command) = cli.command {
        matches!(command, Commands::Manifest(_))
    } else {
        true // Main copy operation needs logging
    };

    // Initialize logging if needed
    if needs_logging {
        let mut log_config = base_config.clone();

        // Set logging config from CLI (including audit_log_path and otel_endpoint)
        if let Some(level) = cli.log_level {
            log_config.log_level = level.into();
        }
        log_config.log_file = cli.log.clone();
        log_config.verbose = cli.verbose;
        log_config.audit_log_path = cli.audit_log.clone();
        log_config.otel_endpoint = cli.otel_endpoint.clone();
        log_config.metrics_port = cli.metrics_port;

        // Initialize logging
        if let Err(e) = logging::init_logging(&log_config) {
            eprintln!("Warning: Failed to initialize logging: {}", e);
        }
    }

    // ── Route subcommands ──────────────────────────────────────────
    // Transfer shorthands (sync, backup, mirror) extract source/dest and
    // set mode/profile overrides, then fall through to the unified transfer
    // path so ALL global CLI flags are applied. Non-transfer subcommands
    // bail out early.
    let mut shorthand_mode: Option<CopyMode> = None;
    let mut shorthand_profile: Option<ProfileArg> = None;
    let mut shorthand_source: Option<String> = None;
    let mut shorthand_dest: Option<String> = None;

    if let Some(ref command) = cli.command {
        match command {
            // Transfer shorthands: extract data, fall through
            Commands::Sync { source, dest } => {
                shorthand_source = Some(source.clone());
                shorthand_dest = Some(dest.clone());
                shorthand_mode = Some(CopyMode::Sync);
            }
            Commands::Backup { source, dest } => {
                shorthand_source = Some(source.clone());
                shorthand_dest = Some(dest.clone());
                shorthand_profile = Some(ProfileArg::Backup);
            }
            Commands::MirrorCmd { source, dest } => {
                shorthand_source = Some(source.clone());
                shorthand_dest = Some(dest.clone());
                shorthand_mode = Some(CopyMode::Mirror);
            }
            // Non-transfer subcommands: handle and return
            _ => {
                // We need to move the command out for handle_subcommand.
                // Safe because we checked it's not a shorthand above.
                let command = cli.command.unwrap();
                return handle_subcommand(command);
            }
        }
    }

    if cli.read_batch.is_some() && cli.write_batch.is_some() {
        return Err(OrbitError::Config(
            "Cannot use --read-batch and --write-batch together".to_string(),
        ));
    }

    // ── Resolve source & dest ──────────────────────────────────────
    // Shorthand subcommands provide source/dest directly; otherwise
    // resolve from positional args or -s/-d flags.
    let is_shorthand = shorthand_source.is_some();

    let destination = shorthand_dest
        .or(cli.destination.clone())
        .or(cli.pos_dest.clone())
        .ok_or_else(|| OrbitError::Config("Destination path required. Usage: orbit <SOURCE> <DEST> or orbit -s <SOURCE> -d <DEST>".to_string()))?;

    let (_dest_protocol, dest_path) = Protocol::from_uri(&destination)?;

    if let Some(batch_path) = cli.read_batch.as_ref() {
        let journal = TransferJournal::load(batch_path).map_err(OrbitError::Io)?;
        let stats = journal.replay(&dest_path).map_err(OrbitError::Io)?;
        println!("Batch replay complete:");
        println!("  Files created: {}", stats.files_created);
        println!("  Files updated: {}", stats.files_updated);
        println!("  Files deleted: {}", stats.files_deleted);
        println!("  Hardlinks created: {}", stats.hardlinks_created);
        println!("  Bytes written: {}", stats.bytes_written);
        return Ok(());
    }

    let source = shorthand_source
        .or(cli.source.clone())
        .or(cli.pos_source.clone())
        .ok_or_else(|| {
            OrbitError::Config(
                "Source path required. Usage: orbit <SOURCE> <DEST> or orbit -s <SOURCE> -d <DEST>"
                    .to_string(),
            )
        })?;

    let (_source_protocol, source_path) = Protocol::from_uri(&source)?;

    // ── Resolve config: profile → auto-network → shorthand → output modes → CLI overrides
    let effective_profile = cli.profile.or(shorthand_profile);
    let dest_is_remote = is_remote_uri(&destination);

    let (mut config, json_output, quiet) = resolve_transfer_config(
        &cli,
        base_config.clone(),
        is_shorthand,
        shorthand_mode,
        effective_profile,
        dest_is_remote,
        source_path.is_dir(),
    );

    // ── Remaining CLI overrides that move owned values out of Cli ──
    // These are not in resolve_transfer_config because it takes &Cli.
    if let Some(flags) = cli.preserve_flags {
        config.preserve_flags = Some(flags);
    }
    config.transform = cli.transform.or(config.transform);
    if let Some(symlink) = cli.symlink {
        config.symlink_mode = symlink.into();
    }
    if let Some(err_mode) = cli.error_mode {
        config.error_mode = err_mode.into();
    }
    if cli.log.is_some() {
        config.log_file = cli.log;
    }
    if !cli.include_patterns.is_empty() {
        config.include_patterns = cli.include_patterns;
    }
    if !cli.exclude_patterns.is_empty() {
        config.exclude_patterns = cli.exclude_patterns;
    }
    if cli.filter_from.is_some() {
        config.filter_from = cli.filter_from;
    }

    // S3 upload enhancement flags
    if cli.content_type.is_some() {
        config.s3_content_type = cli.content_type;
    }
    if cli.content_encoding.is_some() {
        config.s3_content_encoding = cli.content_encoding;
    }
    if cli.content_disposition.is_some() {
        config.s3_content_disposition = cli.content_disposition;
    }
    if cli.cache_control.is_some() {
        config.s3_cache_control = cli.cache_control;
    }
    if cli.expires_header.is_some() {
        config.s3_expires_header = cli.expires_header;
    }
    if !cli.user_metadata.is_empty() {
        config.s3_user_metadata = cli.user_metadata;
    }
    if cli.metadata_directive.is_some() {
        config.s3_metadata_directive = cli.metadata_directive;
    }
    if cli.acl.is_some() {
        config.s3_acl = cli.acl;
    }

    // S3 client configuration flags
    if cli.no_sign_request {
        config.s3_no_sign_request = true;
    }
    if cli.credentials_file.is_some() {
        config.s3_credentials_file = cli.credentials_file;
    }
    if cli.aws_profile.is_some() {
        config.s3_aws_profile = cli.aws_profile;
    }
    if cli.use_acceleration {
        config.s3_use_acceleration = true;
    }
    if cli.request_payer {
        config.s3_request_payer = true;
    }
    if cli.no_verify_ssl {
        config.s3_no_verify_ssl = true;
    }
    if cli.use_list_objects_v1 {
        config.s3_use_list_objects_v1 = true;
    }

    // Conditional copy flags
    if cli.no_clobber {
        config.no_clobber = true;
    }
    if cli.if_size_differ {
        config.if_size_differ = true;
    }
    if cli.if_source_newer {
        config.if_source_newer = true;
    }
    if cli.flatten {
        config.flatten = true;
    }

    // In-place, sparse, and advanced transfer flags
    if let Some(sparse) = cli.sparse {
        config.sparse_mode = sparse.into();
    }
    if cli.preserve_hardlinks {
        config.preserve_hardlinks = true;
    }
    if cli.inplace {
        config.inplace = true;
    }
    if let Some(safety) = cli.inplace_safety {
        config.inplace_safety = safety.into();
    }
    if cli.detect_renames {
        config.detect_renames = true;
    }
    if let Some(threshold) = cli.rename_threshold {
        config.rename_threshold = threshold;
    }
    if !cli.link_dest.is_empty() {
        config.link_dest = cli.link_dest;
    }
    config.write_batch = cli.write_batch.or(config.write_batch);
    config.read_batch = cli.read_batch.or(config.read_batch);

    // Handle manifest generation
    if cli.generate_manifest {
        config.generate_manifest = true;
        config.manifest_output_dir = cli.manifest_dir;
    }

    // Show zero-copy status if enabled (suppressed in json/quiet mode)
    if config.use_zero_copy
        && config.show_progress
        && !json_output
        && !quiet
        && is_zero_copy_available()
    {
        let caps = get_zero_copy_capabilities();
        println!("⚡ Zero-copy enabled ({})", caps.method);
    }

    // Show manifest status if enabled (suppressed in json/quiet mode)
    if config.generate_manifest && config.show_progress && !json_output && !quiet {
        if let Some(ref dir) = config.manifest_output_dir {
            println!("📋 Manifest generation enabled: {}", dir.display());
        }
    }

    // Configure audit logging and observability
    if let Some(fmt) = cli.audit_format {
        config.audit_format = fmt.into();
    }
    if cli.audit_log.is_some() {
        config.audit_log_path = cli.audit_log;
    }
    if cli.otel_endpoint.is_some() {
        config.otel_endpoint = cli.otel_endpoint;
    }
    config.metrics_port = cli.metrics_port.or(config.metrics_port);

    // Configure delta detection
    if let Some(check) = cli.check {
        config.check_mode = check.into();
    }
    if let Some(bs) = cli.block_size {
        config.delta_block_size = bs.saturating_mul(1024);
    }
    if cli.whole_file {
        config.whole_file = true;
    }
    if cli.update_manifest {
        config.update_manifest = true;
    }
    if cli.ignore_existing {
        config.ignore_existing = true;
    }
    if cli.delta_manifest.is_some() {
        config.delta_manifest_path = cli.delta_manifest;
    }

    // First-run hint: suggest `orbit init` if no config file exists
    if !default_config_exists() && !quiet && !json_output {
        eprintln!(
            "{} Run '{}' to optimize for your hardware and use case.",
            Theme::muted("Tip:"),
            Theme::primary("orbit init")
        );
    }

    // Optimize config based on system capabilities
    let optimized = ConfigOptimizer::optimize_with_probe(config, Some(&dest_path))?;

    if !optimized.notices.is_empty() && !quiet && !json_output {
        guidance_box(&optimized.notices);
    }

    let config = optimized.final_config;

    // Collect auto-tune notices for the summary
    let auto_tune_notices: Vec<_> = optimized
        .notices
        .iter()
        .filter(|n| n.level == orbit::core::guidance::NoticeLevel::AutoTune)
        .cloned()
        .collect();

    // Perform the copy
    let stats = if source_path.is_dir() && config.recursive {
        copy_directory(&source_path, &dest_path, &config)?
    } else {
        copy_file(&source_path, &dest_path, &config)?
    };

    // Print summary (unless quiet or json mode)
    if !quiet && !json_output {
        print_summary(&stats, &auto_tune_notices);
    }

    // Print execution statistics if enabled (default: true now)
    if config.show_stats && !quiet && !json_output {
        print_exec_stats(&stats);
    }

    Ok(())
}

/// Handle non-transfer subcommands. Transfer shorthands (sync, backup, mirror)
/// are handled in run() by falling through to the unified transfer path.
fn handle_subcommand(command: Commands) -> Result<()> {
    match command {
        Commands::Init => orbit::commands::init::run_init_wizard(),
        Commands::Stats => {
            let stats = TransferStats::default();
            stats.print();
            Ok(())
        }
        Commands::Presets => {
            print_presets();
            Ok(())
        }
        Commands::Capabilities => {
            print_capabilities();
            Ok(())
        }
        Commands::Doctor => {
            run_doctor();
            Ok(())
        }
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            use clap_complete::generate;
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "orbit", &mut std::io::stdout());
            Ok(())
        }
        Commands::Manifest(manifest_cmd) => {
            orbit::commands::manifest::handle_manifest_command(manifest_cmd)
        }
        Commands::Run { file, workers } => {
            orbit::commands::batch::handle_run_command(file, workers)
        }
        #[cfg(feature = "s3-native")]
        Commands::Cat { uri } => orbit::commands::s3::handle_cat_command(&uri),
        #[cfg(feature = "s3-native")]
        Commands::Pipe { uri } => orbit::commands::s3::handle_pipe_command(&uri),
        #[cfg(feature = "s3-native")]
        Commands::Presign { uri, expires } => {
            orbit::commands::s3::handle_presign_command(&uri, expires)
        }
        #[cfg(feature = "s3-native")]
        Commands::Ls {
            uri,
            etag,
            storage_class,
            all_versions,
            show_fullpath,
        } => orbit::commands::s3::handle_ls_command(
            &uri,
            etag,
            storage_class,
            all_versions,
            show_fullpath,
        ),
        #[cfg(feature = "s3-native")]
        Commands::Head { uri, version_id } => {
            orbit::commands::s3::handle_head_command(&uri, version_id)
        }
        #[cfg(feature = "s3-native")]
        Commands::Du {
            uri,
            group,
            all_versions,
        } => orbit::commands::s3::handle_du_command(&uri, group, all_versions),
        #[cfg(feature = "s3-native")]
        Commands::Rm {
            uri,
            all_versions,
            version_id,
            dry_run,
        } => orbit::commands::s3::handle_rm_command(&uri, all_versions, version_id, dry_run),
        #[cfg(feature = "s3-native")]
        Commands::Mv { source, dest } => orbit::commands::s3::handle_mv_command(&source, &dest),
        #[cfg(feature = "s3-native")]
        Commands::Mb { bucket } => orbit::commands::s3::handle_mb_command(&bucket),
        #[cfg(feature = "s3-native")]
        Commands::Rb { bucket } => orbit::commands::s3::handle_rb_command(&bucket),
        // Transfer shorthands are handled in run() — this arm is unreachable
        // because run() extracts them before calling handle_subcommand().
        Commands::Sync { .. } | Commands::Backup { .. } | Commands::MirrorCmd { .. } => {
            unreachable!("transfer shorthands handled in run()")
        }
    }
}

fn run_doctor() {
    cli_style::print_banner();
    section_header(&format!("{} Orbit Doctor", Icons::WRENCH));
    println!();

    // 1. Config file
    let config_exists = default_config_exists();
    if config_exists {
        let home = dirs::home_dir().unwrap();
        let path = home.join(".orbit").join("orbit.toml");
        println!(
            "  {} {} {}",
            Icons::SUCCESS,
            Theme::muted("Config file:"),
            Theme::success(path.display())
        );
        // Try to parse it
        match CopyConfig::from_file(&path) {
            Ok(_) => println!(
                "  {} {}",
                Icons::SUCCESS,
                Theme::success("Config file is valid TOML")
            ),
            Err(e) => println!(
                "  {} {} {}",
                Icons::ERROR,
                Theme::error("Config parse error:"),
                e
            ),
        }
    } else {
        println!(
            "  {} {} {}",
            Icons::WARNING,
            Theme::warning("No config file found."),
            Theme::muted("Run 'orbit init' to create one.")
        );
    }
    println!();

    // 2. Platform capabilities
    section_header(&format!("{} Platform", Icons::GEAR));
    println!(
        "  {} {} {} / {}",
        Icons::BULLET,
        Theme::muted("OS:"),
        Theme::value(std::env::consts::OS),
        Theme::muted(std::env::consts::ARCH)
    );

    let caps = get_zero_copy_capabilities();
    println!(
        "  {} {} {}",
        if caps.available {
            Icons::SUCCESS
        } else {
            Icons::WARNING
        },
        Theme::muted("Zero-copy:"),
        if caps.available {
            Theme::success(caps.method)
        } else {
            Theme::warning("unavailable")
        }
    );
    println!();

    // 3. System probe
    section_header(&format!("{} Hardware", Icons::LIGHTNING));
    match orbit::core::probe::Probe::scan(&std::env::current_dir().unwrap_or_default()) {
        Ok(profile) => {
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("CPU cores:"),
                Theme::value(profile.logical_cores)
            );
            println!(
                "  {} {} {} GB",
                Icons::BULLET,
                Theme::muted("RAM:"),
                Theme::value(profile.available_ram_gb)
            );
            println!(
                "  {} {} ~{:.0} MB/s",
                Icons::BULLET,
                Theme::muted("I/O throughput:"),
                profile.estimated_io_throughput
            );
        }
        Err(e) => {
            println!(
                "  {} {} {}",
                Icons::WARNING,
                Theme::warning("Probe failed:"),
                e
            );
        }
    }
    println!();

    // 4. Feature flags
    section_header(&format!("{} Compiled Features", Icons::GEAR));
    let features: Vec<(&str, bool)> = vec![
        ("s3-native", cfg!(feature = "s3-native")),
        ("smb-native", cfg!(feature = "smb-native")),
        ("ssh-backend", cfg!(feature = "ssh-backend")),
        ("azure-native", cfg!(feature = "azure-native")),
        ("gcs-native", cfg!(feature = "gcs-native")),
    ];
    for (name, enabled) in &features {
        println!(
            "  {} {}",
            if *enabled {
                Icons::SUCCESS
            } else {
                Icons::BULLET
            },
            if *enabled {
                Theme::success(*name).to_string()
            } else {
                Theme::muted(*name).to_string()
            }
        );
    }
    println!();

    // 5. Environment
    section_header(&format!("{} Environment", Icons::GLOBE));
    let jwt_set = std::env::var("ORBIT_JWT_SECRET").is_ok();
    println!(
        "  {} {} {}",
        if jwt_set {
            Icons::SUCCESS
        } else {
            Icons::BULLET
        },
        Theme::muted("ORBIT_JWT_SECRET:"),
        if jwt_set {
            Theme::success("set")
        } else {
            Theme::muted("not set (dashboard auth disabled)")
        }
    );

    let stats_env = std::env::var("ORBIT_STATS").unwrap_or_else(|_| "on".to_string());
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("ORBIT_STATS:"),
        Theme::value(&stats_env)
    );
    println!();

    println!(
        "  {}",
        Theme::success("Doctor check complete. No critical issues found.")
    );
    println!();
}

fn print_presets() {
    cli_style::print_banner();
    section_header(&format!("{} Configuration Presets", Icons::GEAR));
    println!();
    println!(
        "  {}",
        Theme::muted("Use --profile <preset> to apply a configuration preset")
    );
    println!();

    let presets = vec![
        PresetInfo {
            icon: Icons::ROCKET,
            name: "FAST",
            checksum: false,
            resume: false,
            compression: "None".to_string(),
            zero_copy: true,
            best_for: "Local NVMe/SSD".to_string(),
        },
        PresetInfo {
            icon: Icons::SHIELD,
            name: "SAFE",
            checksum: true,
            resume: true,
            compression: "None".to_string(),
            zero_copy: false,
            best_for: "Critical data".to_string(),
        },
        PresetInfo {
            icon: Icons::LIGHTNING,
            name: "BACKUP",
            checksum: true,
            resume: true,
            compression: "Zstd:3".to_string(),
            zero_copy: false,
            best_for: "Reliable backups".to_string(),
        },
        PresetInfo {
            icon: Icons::GLOBE,
            name: "NETWORK",
            checksum: true,
            resume: true,
            compression: "Zstd:3".to_string(),
            zero_copy: false,
            best_for: "Remote/slow networks".to_string(),
        },
    ];

    println!("{}", preset_table(&presets));

    println!();
    section_header(&format!("{} Example Usage", Icons::SPARKLE));
    println!();
    println!(
        "  {} {}",
        Theme::muted("Fast local copy:"),
        Theme::primary("orbit /data /backup --profile fast")
    );
    println!(
        "  {} {}",
        Theme::muted("Sync two dirs:"),
        Theme::primary("orbit sync /data /backup")
    );
    println!(
        "  {} {}",
        Theme::muted("Backup with zstd:"),
        Theme::primary("orbit backup /data /nas/backup")
    );
    println!(
        "  {} {}",
        Theme::muted("Network transfer:"),
        Theme::primary("orbit /data s3://bucket/data")
    );
    println!(
        "  {} {}",
        Theme::muted("Mirror (delete extras):"),
        Theme::primary("orbit mirror /src /dst")
    );
    println!();
}

fn print_capabilities() {
    cli_style::print_banner();

    let caps = get_zero_copy_capabilities();

    // Platform info
    section_header(&format!("{} Platform", Icons::GEAR));
    println!(
        "  {} {} / {}",
        Icons::BULLET,
        Theme::value(std::env::consts::OS),
        Theme::muted(std::env::consts::ARCH)
    );
    println!();

    // Zero-copy capabilities
    section_header(&format!("{} Zero-Copy Engine", Icons::LIGHTNING));
    let zero_copy_items: Vec<(&str, bool, &str)> = vec![
        ("Zero-Copy Available", caps.available, caps.method),
        (
            "Cross-Filesystem",
            caps.cross_filesystem,
            if caps.cross_filesystem {
                "Can copy between different mounts"
            } else {
                "Same filesystem only"
            },
        ),
    ];
    println!("{}", capability_table(&zero_copy_items));

    // Compression
    section_header(&format!("{} Compression", Icons::GEAR));
    let compression_items: Vec<(&str, bool, &str)> = vec![
        ("LZ4", true, "Fast compression, lower ratio"),
        ("Zstd", true, "Balanced speed/ratio, levels 1-19"),
    ];
    println!("{}", capability_table(&compression_items));

    // Checksums
    section_header(&format!("{} Verification", Icons::SHIELD));
    let checksum_items: Vec<(&str, bool, &str)> = vec![
        ("SHA-256", true, "Cryptographic, standard"),
        ("BLAKE3", true, "Modern, parallelizable, faster"),
    ];
    println!("{}", capability_table(&checksum_items));

    // Protocols
    section_header(&format!("{} Storage Backends", Icons::GLOBE));
    let protocol_items: Vec<(&str, bool, &str)> = vec![
        ("Local Filesystem", true, "Production ready"),
        #[cfg(feature = "smb-native")]
        ("SMB/CIFS", true, "Native pure-Rust client"),
        #[cfg(not(feature = "smb-native"))]
        ("SMB/CIFS", false, "Enable with --features smb-native"),
        #[cfg(feature = "s3-native")]
        ("Amazon S3", true, "Multipart upload support"),
        #[cfg(not(feature = "s3-native"))]
        ("Amazon S3", false, "Enable with --features s3-native"),
        #[cfg(feature = "azure-native")]
        ("Azure Blob", true, "Via object_store"),
        #[cfg(not(feature = "azure-native"))]
        ("Azure Blob", false, "Enable with --features azure-native"),
        #[cfg(feature = "gcs-native")]
        ("Google Cloud Storage", true, "Via object_store"),
        #[cfg(not(feature = "gcs-native"))]
        (
            "Google Cloud Storage",
            false,
            "Enable with --features gcs-native",
        ),
        #[cfg(feature = "ssh-backend")]
        ("SSH/SFTP", true, "Via ssh2"),
        #[cfg(not(feature = "ssh-backend"))]
        ("SSH/SFTP", false, "Enable with --features ssh-backend"),
    ];
    println!("{}", capability_table(&protocol_items));

    // Manifest System
    section_header(&format!("{} Manifest System", Icons::MANIFEST));
    let manifest_items: Vec<(&str, bool, &str)> = vec![
        ("Flight Plans", true, "Transfer planning & metadata"),
        ("Cargo Manifests", true, "Chunk-level verification"),
        ("Star Maps", true, "Binary index for resume"),
        ("Audit Logging", true, "HMAC-chained event trail"),
    ];
    println!("{}", capability_table(&manifest_items));

    // Performance
    section_header(&format!("{} Performance Features", Icons::ROCKET));
    let perf_items: Vec<(&str, bool, &str)> = vec![
        ("Resume/Checkpoint", true, "Continue interrupted transfers"),
        ("Parallel Operations", true, "Multi-file concurrency"),
        ("Bandwidth Throttle", true, "Rate limiting via token bucket"),
        ("Progress Tracking", true, "Real-time progress bars"),
        ("Delta Detection", true, "Content-based change detection"),
    ];
    println!("{}", capability_table(&perf_items));

    println!();
}

fn print_exec_stats(stats: &CopyStats) {
    section_header(&format!("{} Execution Statistics", Icons::STATS));
    println!();

    let total = stats.files_copied + stats.files_skipped + stats.files_failed;
    let elapsed = stats.duration.as_secs_f64();
    let throughput = if elapsed > 0.0 {
        format_bytes((stats.bytes_copied as f64 / elapsed) as u64)
    } else {
        "N/A".to_string()
    };

    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Operation:"),
        Theme::value("Copy")
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Total files:"),
        Theme::value(total)
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Succeeded:"),
        Theme::success(stats.files_copied)
    );
    if stats.files_failed > 0 {
        println!(
            "  {} {} {}",
            Icons::BULLET,
            Theme::muted("Failed:"),
            Theme::error(stats.files_failed)
        );
    }
    if stats.files_skipped > 0 {
        println!(
            "  {} {} {}",
            Icons::BULLET,
            Theme::muted("Skipped:"),
            Theme::warning(stats.files_skipped)
        );
    }
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Total size:"),
        Theme::value(format_bytes(stats.bytes_copied))
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Elapsed:"),
        Theme::value(format_duration(elapsed))
    );
    println!(
        "  {} {} {}/s",
        Icons::BULLET,
        Theme::muted("Throughput:"),
        Theme::primary(throughput)
    );
    println!();
}

fn print_summary(stats: &CopyStats, auto_tune_notices: &[orbit::core::guidance::Notice]) {
    println!();
    header_box(
        "Transfer Complete",
        Some("All operations finished successfully"),
    );
    println!();

    let bytes_per_sec = if stats.duration.as_secs_f64() > 0.0 {
        stats.bytes_copied as f64 / stats.duration.as_secs_f64()
    } else {
        0.0
    };

    let checksum_display = stats.checksum.as_ref().map(|c| {
        if c.len() > 16 {
            format!("{}...{}", &c[..8], &c[c.len() - 8..])
        } else {
            c.clone()
        }
    });

    let compression_display = stats
        .compression_ratio
        .map(|r| format!("{:.1}% reduction", r));

    let summary = TransferSummary {
        files_copied: stats.files_copied,
        files_skipped: stats.files_skipped,
        files_failed: stats.files_failed,
        total_size: format_bytes(stats.bytes_copied),
        duration: format_duration(stats.duration.as_secs_f64()),
        speed: format!("{}/s", format_bytes(bytes_per_sec as u64)),
        checksum: checksum_display,
        compression_ratio: compression_display,
    };

    println!("{}", transfer_summary_table(&summary));

    // Display auto-tune notices if any
    let auto_tune: Vec<_> = auto_tune_notices
        .iter()
        .filter(|n| n.level == orbit::core::guidance::NoticeLevel::AutoTune)
        .collect();
    if !auto_tune.is_empty() {
        println!();
        section_header(&format!("{} Auto-Tuned Settings", Icons::GEAR));
        println!();
        for notice in &auto_tune {
            println!(
                "  {} {} {}",
                Icons::LIGHTNING,
                Theme::muted(&notice.code),
                notice.message
            );
        }
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use orbit::commands::batch::{normalize_batch_args, split_command_line};

    #[test]
    fn test_version() {
        // Just verify the CLI struct can be created
        let result = Cli::try_parse_from(["orbit", "--help"]);
        // --help causes an error exit, but that's fine
        assert!(result.is_err());
    }

    #[test]
    fn test_workers_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt", "--workers", "64"])
                .unwrap();
        assert_eq!(cli.workers, Some(64));
    }

    #[test]
    fn test_parallel_alias_for_workers() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src.txt",
            "-d",
            "dst.txt",
            "--parallel",
            "32",
        ])
        .unwrap();
        assert_eq!(cli.workers, Some(32));
    }

    #[test]
    fn test_concurrency_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src.txt",
            "-d",
            "dst.txt",
            "--concurrency",
            "10",
        ])
        .unwrap();
        assert_eq!(cli.concurrency, Some(10));
    }

    #[test]
    fn test_concurrency_default_none() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
        assert_eq!(cli.concurrency, None);
    }

    #[test]
    fn test_workers_default_none() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
        assert_eq!(cli.workers, None);
    }

    #[test]
    fn test_stat_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt", "--stat"]).unwrap();
        assert!(cli.stat);
    }

    #[test]
    fn test_stat_default_false() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
        assert!(!cli.stat);
    }

    #[test]
    fn test_human_readable_short_flag() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt", "-H"]).unwrap();
        assert!(cli.human_readable);
    }

    #[test]
    fn test_human_readable_long_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src.txt",
            "-d",
            "dst.txt",
            "--human-readable",
        ])
        .unwrap();
        assert!(cli.human_readable);
    }

    #[test]
    fn test_human_readable_default_false() {
        // The CLI flag defaults to false; the *config* defaults to true now
        let cli = Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
        assert!(!cli.human_readable);
    }

    #[test]
    fn test_run_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "run"]).unwrap();
        match cli.command {
            Some(Commands::Run { file, workers }) => {
                assert!(file.is_none());
                assert_eq!(workers, 256);
            }
            _ => panic!("Expected Run subcommand"),
        }
    }

    #[test]
    fn test_run_subcommand_with_file() {
        let cli = Cli::try_parse_from(["orbit", "run", "--file", "commands.txt"]).unwrap();
        match cli.command {
            Some(Commands::Run { file, workers }) => {
                assert_eq!(file, Some(PathBuf::from("commands.txt")));
                assert_eq!(workers, 256);
            }
            _ => panic!("Expected Run subcommand"),
        }
    }

    #[test]
    fn test_run_subcommand_with_workers() {
        let cli = Cli::try_parse_from(["orbit", "run", "--workers", "128"]).unwrap();
        match cli.command {
            Some(Commands::Run { file, workers }) => {
                assert!(file.is_none());
                assert_eq!(workers, 128);
            }
            _ => panic!("Expected Run subcommand"),
        }
    }

    #[test]
    fn test_split_command_line_basic() {
        let args = split_command_line("cp /src /dst --recursive").unwrap();
        assert_eq!(args, vec!["cp", "/src", "/dst", "--recursive"]);
    }

    #[test]
    fn test_split_command_line_quotes() {
        let args = split_command_line(r#"cp "a b.txt" "c d.txt" --recursive"#).unwrap();
        assert_eq!(args, vec!["cp", "a b.txt", "c d.txt", "--recursive"]);
    }

    #[test]
    fn test_split_command_line_windows_paths() {
        let args = split_command_line(r#"cp C:\data\file.txt D:\dest\file.txt"#).unwrap();
        assert_eq!(args, vec!["cp", r"C:\data\file.txt", r"D:\dest\file.txt"]);
    }

    #[test]
    fn test_normalize_batch_args_cp() {
        let args = normalize_batch_args(
            "cp /src /dst --recursive",
            vec![
                "cp".to_string(),
                "/src".to_string(),
                "/dst".to_string(),
                "--recursive".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(
            args,
            vec!["--source", "/src", "--dest", "/dst", "--recursive"]
        );
    }

    #[test]
    fn test_normalize_batch_args_orbit_prefix() {
        let args = normalize_batch_args(
            "orbit --source /src --dest /dst",
            vec![
                "orbit".to_string(),
                "--source".to_string(),
                "/src".to_string(),
                "--dest".to_string(),
                "/dst".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(args, vec!["--source", "/src", "--dest", "/dst"]);
    }

    #[test]
    fn test_normalize_batch_args_nested_run_rejected() {
        let err = normalize_batch_args(
            "orbit run --file commands.txt",
            vec![
                "orbit".to_string(),
                "run".to_string(),
                "--file".to_string(),
                "commands.txt".to_string(),
            ],
        )
        .unwrap_err();
        assert!(err.to_string().contains("Nested 'orbit run'"));
    }

    #[test]
    fn test_zero_copy_detection() {
        // Test that zero-copy detection works
        let available = is_zero_copy_available();
        // On Windows, this should be true (CopyFileExW)
        assert_eq!(available, is_zero_copy_available());
    }

    #[test]
    fn test_combined_flags() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--workers",
            "128",
            "--concurrency",
            "8",
            "--stat",
            "-H",
            "--recursive",
        ])
        .unwrap();
        assert_eq!(cli.workers, Some(128));
        assert_eq!(cli.concurrency, Some(8));
        assert!(cli.stat);
        assert!(cli.human_readable);
        assert!(cli.recursive);
    }

    // === Flag parser tests ===

    #[test]
    fn test_flatten_flag() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--flatten"]).unwrap();
        assert!(cli.flatten);
    }

    #[test]
    fn test_detect_renames_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--detect-renames"]).unwrap();
        assert!(cli.detect_renames);
        assert_eq!(cli.rename_threshold, None);
    }

    #[test]
    fn test_detect_renames_with_threshold() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--detect-renames",
            "--rename-threshold",
            "0.5",
        ])
        .unwrap();
        assert!(cli.detect_renames);
        assert!((cli.rename_threshold.unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_s3_content_type_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--content-type",
            "text/plain",
        ])
        .unwrap();
        assert_eq!(cli.content_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_s3_no_sign_request_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--no-sign-request"]).unwrap();
        assert!(cli.no_sign_request);
    }

    #[test]
    fn test_s3_aws_profile_flag() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--aws-profile", "prod"])
            .unwrap();
        assert_eq!(cli.aws_profile, Some("prod".to_string()));
    }

    #[test]
    fn test_s3_use_acceleration_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--use-acceleration"]).unwrap();
        assert!(cli.use_acceleration);
    }

    #[test]
    fn test_s3_flags_defaults() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst"]).unwrap();
        assert!(cli.content_type.is_none());
        assert!(cli.content_encoding.is_none());
        assert!(cli.acl.is_none());
        assert!(!cli.no_sign_request);
        assert!(cli.aws_profile.is_none());
        assert!(!cli.use_acceleration);
        assert!(!cli.request_payer);
        assert!(!cli.no_verify_ssl);
        assert!(!cli.flatten);
        assert!(!cli.detect_renames);
    }

    #[test]
    fn test_conditional_copy_defaults() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst"]).unwrap();
        assert!(!cli.no_clobber);
        assert!(!cli.if_size_differ);
        assert!(!cli.if_source_newer);
    }

    #[test]
    fn test_compression_shorthand_zstd() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--zstd"]).unwrap();
        assert!(cli.zstd);
        assert!(!cli.lz4);
        assert!(cli.compress.is_none());
    }

    #[test]
    fn test_compression_shorthand_lz4() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--lz4"]).unwrap();
        assert!(cli.lz4);
        assert!(!cli.zstd);
    }

    #[test]
    fn test_compress_bare_zstd() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--compress", "zstd"]).unwrap();
        let comp: CompressionType = cli.compress.unwrap().into();
        assert!(matches!(comp, CompressionType::Zstd { level: 3 }));
    }

    #[test]
    fn test_quiet_flag() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "-q"]).unwrap();
        assert!(cli.quiet);
    }

    #[test]
    fn test_raw_flag() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--raw"]).unwrap();
        assert!(cli.raw);
    }

    #[test]
    fn test_no_stat_flag() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--no-stat"]).unwrap();
        assert!(cli.no_stat);
    }

    #[test]
    fn test_no_preserve_metadata_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--no-preserve-metadata"])
                .unwrap();
        assert!(cli.no_preserve_metadata);
    }

    #[test]
    fn test_no_auto_recursive_flag() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--no-auto-recursive"])
            .unwrap();
        assert!(cli.no_auto_recursive);
    }

    #[test]
    fn test_sync_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "sync", "/src", "/dst"]).unwrap();
        match cli.command {
            Some(Commands::Sync { source, dest }) => {
                assert_eq!(source, "/src");
                assert_eq!(dest, "/dst");
            }
            _ => panic!("Expected Sync subcommand"),
        }
    }

    #[test]
    fn test_backup_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "backup", "/src", "/dst"]).unwrap();
        match cli.command {
            Some(Commands::Backup { source, dest }) => {
                assert_eq!(source, "/src");
                assert_eq!(dest, "/dst");
            }
            _ => panic!("Expected Backup subcommand"),
        }
    }

    #[test]
    fn test_mirror_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "mirror", "/src", "/dst"]).unwrap();
        match cli.command {
            Some(Commands::MirrorCmd { source, dest }) => {
                assert_eq!(source, "/src");
                assert_eq!(dest, "/dst");
            }
            _ => panic!("Expected Mirror subcommand"),
        }
    }

    #[test]
    fn test_doctor_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "doctor"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Doctor)));
    }

    #[test]
    fn test_profile_not_clobbered_by_defaults() {
        // When --profile safe is used without --retry-attempts,
        // retry_attempts should be None (not a default that overwrites the profile)
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--profile", "safe"]).unwrap();
        assert_eq!(cli.retry_attempts, None);
        assert_eq!(cli.retry_delay, None);
        assert_eq!(cli.chunk_size, None);
        assert_eq!(cli.workers, None);
        assert_eq!(cli.concurrency, None);
    }

    #[test]
    fn test_is_remote_uri() {
        assert!(is_remote_uri("s3://bucket/key"));
        assert!(is_remote_uri("smb://server/share"));
        assert!(is_remote_uri("ssh://host/path"));
        assert!(is_remote_uri("azure://container/blob"));
        assert!(is_remote_uri("gs://bucket/key"));
        assert!(is_remote_uri("\\\\server\\share"));
        assert!(!is_remote_uri("/local/path"));
        assert!(!is_remote_uri("./relative"));
        assert!(!is_remote_uri("C:\\Windows\\path"));
    }

    // === S3 subcommand parse tests ===

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_ls_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "ls", "s3://bucket/prefix"]).unwrap();
        match cli.command {
            Some(Commands::Ls {
                uri,
                etag,
                storage_class,
                all_versions,
                show_fullpath,
            }) => {
                assert_eq!(uri, "s3://bucket/prefix");
                assert!(!etag);
                assert!(!storage_class);
                assert!(!all_versions);
                assert!(!show_fullpath);
            }
            _ => panic!("Expected Ls subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_ls_subcommand_with_flags() {
        let cli = Cli::try_parse_from([
            "orbit",
            "ls",
            "s3://bucket/prefix",
            "-e",
            "-s",
            "--all-versions",
            "--show-fullpath",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Ls {
                uri,
                etag,
                storage_class,
                all_versions,
                show_fullpath,
            }) => {
                assert_eq!(uri, "s3://bucket/prefix");
                assert!(etag);
                assert!(storage_class);
                assert!(all_versions);
                assert!(show_fullpath);
            }
            _ => panic!("Expected Ls subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_head_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "head", "s3://bucket/key.txt"]).unwrap();
        match cli.command {
            Some(Commands::Head { uri, version_id }) => {
                assert_eq!(uri, "s3://bucket/key.txt");
                assert!(version_id.is_none());
            }
            _ => panic!("Expected Head subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_head_subcommand_with_version() {
        let cli = Cli::try_parse_from([
            "orbit",
            "head",
            "s3://bucket/key.txt",
            "--version-id",
            "abc123",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Head { uri, version_id }) => {
                assert_eq!(uri, "s3://bucket/key.txt");
                assert_eq!(version_id, Some("abc123".to_string()));
            }
            _ => panic!("Expected Head subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_du_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "du", "s3://bucket/prefix"]).unwrap();
        match cli.command {
            Some(Commands::Du {
                uri,
                group,
                all_versions,
            }) => {
                assert_eq!(uri, "s3://bucket/prefix");
                assert!(!group);
                assert!(!all_versions);
            }
            _ => panic!("Expected Du subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_du_subcommand_with_group() {
        let cli = Cli::try_parse_from(["orbit", "du", "s3://bucket/prefix", "--group"]).unwrap();
        match cli.command {
            Some(Commands::Du {
                uri,
                group,
                all_versions,
            }) => {
                assert_eq!(uri, "s3://bucket/prefix");
                assert!(group);
                assert!(!all_versions);
            }
            _ => panic!("Expected Du subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_rm_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "rm", "s3://bucket/key.txt"]).unwrap();
        match cli.command {
            Some(Commands::Rm {
                uri,
                all_versions,
                version_id,
                dry_run,
            }) => {
                assert_eq!(uri, "s3://bucket/key.txt");
                assert!(!all_versions);
                assert!(version_id.is_none());
                assert!(!dry_run);
            }
            _ => panic!("Expected Rm subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_rm_subcommand_with_dry_run() {
        let cli =
            Cli::try_parse_from(["orbit", "rm", "s3://bucket/prefix/*", "--dry-run"]).unwrap();
        match cli.command {
            Some(Commands::Rm {
                uri,
                all_versions,
                version_id,
                dry_run,
            }) => {
                assert_eq!(uri, "s3://bucket/prefix/*");
                assert!(!all_versions);
                assert!(version_id.is_none());
                assert!(dry_run);
            }
            _ => panic!("Expected Rm subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_mv_subcommand() {
        let cli =
            Cli::try_parse_from(["orbit", "mv", "s3://bucket/old.txt", "s3://bucket/new.txt"])
                .unwrap();
        match cli.command {
            Some(Commands::Mv { source, dest }) => {
                assert_eq!(source, "s3://bucket/old.txt");
                assert_eq!(dest, "s3://bucket/new.txt");
            }
            _ => panic!("Expected Mv subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_mb_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "mb", "s3://my-new-bucket"]).unwrap();
        match cli.command {
            Some(Commands::Mb { bucket }) => {
                assert_eq!(bucket, "s3://my-new-bucket");
            }
            _ => panic!("Expected Mb subcommand"),
        }
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_rb_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "rb", "s3://my-old-bucket"]).unwrap();
        match cli.command {
            Some(Commands::Rb { bucket }) => {
                assert_eq!(bucket, "s3://my-old-bucket");
            }
            _ => panic!("Expected Rb subcommand"),
        }
    }

    // === Output mode resolution tests ===

    #[test]
    fn test_resolve_output_modes_defaults() {
        // No flags, config defaults (true, true): both stay true
        let (hr, stats) = resolve_output_modes(OutputModeInputs {
            json_output: false,
            quiet: false,
            raw: false,
            cli_human_readable: false,
            cli_stat: false,
            cli_no_stat: false,
            config_human_readable: true,
            config_show_stats: true,
        });
        assert!(hr, "human_readable should default to true");
        assert!(stats, "show_stats should default to true");
    }

    #[test]
    fn test_resolve_output_modes_json_suppresses_both() {
        // json_output=true (from config or --json): both should be false
        let (hr, stats) = resolve_output_modes(OutputModeInputs {
            json_output: true,
            quiet: false,
            raw: false,
            cli_human_readable: false,
            cli_stat: false,
            cli_no_stat: false,
            config_human_readable: true,
            config_show_stats: true,
        });
        assert!(!hr, "json mode should disable human_readable");
        assert!(!stats, "json mode should disable show_stats");
    }

    #[test]
    fn test_resolve_output_modes_quiet_suppresses_stats() {
        // --quiet: stats off, human_readable still on
        let (hr, stats) = resolve_output_modes(OutputModeInputs {
            json_output: false,
            quiet: true,
            raw: false,
            cli_human_readable: false,
            cli_stat: false,
            cli_no_stat: false,
            config_human_readable: true,
            config_show_stats: true,
        });
        assert!(hr, "quiet should not affect human_readable");
        assert!(!stats, "quiet should disable show_stats");
    }

    #[test]
    fn test_resolve_output_modes_raw_disables_human() {
        // --raw: human_readable off, stats still on
        let (hr, stats) = resolve_output_modes(OutputModeInputs {
            json_output: false,
            quiet: false,
            raw: true,
            cli_human_readable: false,
            cli_stat: false,
            cli_no_stat: false,
            config_human_readable: true,
            config_show_stats: true,
        });
        assert!(!hr, "raw should disable human_readable");
        assert!(stats, "raw should not affect show_stats");
    }

    #[test]
    fn test_resolve_output_modes_no_stat_flag() {
        // --no-stat: stats off, human_readable on
        let (hr, stats) = resolve_output_modes(OutputModeInputs {
            json_output: false,
            quiet: false,
            raw: false,
            cli_human_readable: false,
            cli_stat: false,
            cli_no_stat: true,
            config_human_readable: true,
            config_show_stats: true,
        });
        assert!(hr);
        assert!(!stats, "--no-stat should disable show_stats");
    }

    #[test]
    fn test_resolve_output_modes_json_from_config_suppresses_output() {
        // Simulates CopyConfig.json_output=true being merged with cli.json=false
        // The resolved json_output would be true, so:
        let (hr, stats) = resolve_output_modes(OutputModeInputs {
            json_output: true,
            quiet: false,
            raw: false,
            cli_human_readable: false,
            cli_stat: false,
            cli_no_stat: false,
            config_human_readable: true,
            config_show_stats: true,
        });
        assert!(!hr, "config json_output should suppress human_readable");
        assert!(!stats, "config json_output should suppress show_stats");
    }

    #[test]
    fn test_resolve_output_modes_config_show_stats_false_preserved() {
        // Config file sets show_stats=false, no CLI override → stays false
        let (hr, stats) = resolve_output_modes(OutputModeInputs {
            json_output: false,
            quiet: false,
            raw: false,
            cli_human_readable: false,
            cli_stat: false,
            cli_no_stat: false,
            config_human_readable: true,
            config_show_stats: false,
        });
        assert!(hr, "human_readable should follow config (true)");
        assert!(!stats, "config show_stats=false should be preserved");
    }

    #[test]
    fn test_resolve_output_modes_config_human_readable_false_preserved() {
        // Config file sets human_readable=false, no CLI override → stays false
        let (hr, stats) = resolve_output_modes(OutputModeInputs {
            json_output: false,
            quiet: false,
            raw: false,
            cli_human_readable: false,
            cli_stat: false,
            cli_no_stat: false,
            config_human_readable: false,
            config_show_stats: true,
        });
        assert!(!hr, "config human_readable=false should be preserved");
        assert!(stats, "show_stats should follow config (true)");
    }

    #[test]
    fn test_resolve_output_modes_cli_overrides_config_false() {
        // Config file sets both false, but --human-readable and --stat override
        let (hr, stats) = resolve_output_modes(OutputModeInputs {
            json_output: false,
            quiet: false,
            raw: false,
            cli_human_readable: true,
            cli_stat: true,
            cli_no_stat: false,
            config_human_readable: false,
            config_show_stats: false,
        });
        assert!(hr, "--human-readable should override config false");
        assert!(stats, "--stat should override config false");
    }

    #[test]
    fn test_json_mode_suppresses_progress() {
        // --json should suppress progress even when --show-progress is passed
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "/src",
            "-d",
            "/dst",
            "--json",
            "--show-progress",
        ])
        .unwrap();

        let base = CopyConfig::default();
        let (config, json_output, _quiet) =
            resolve_transfer_config(&cli, base, false, None, None, false, false);

        assert!(json_output);
        assert!(
            !config.show_progress,
            "--json should suppress progress even with --show-progress"
        );
    }

    #[test]
    fn test_config_json_output_suppresses_progress() {
        // Config json_output=true (no CLI flag) should also suppress progress
        let cli = Cli::try_parse_from(["orbit", "-s", "/src", "-d", "/dst"]).unwrap();

        let base = CopyConfig {
            json_output: true,
            show_progress: true, // Explicitly on in config
            ..CopyConfig::default()
        };

        let (config, json_output, _quiet) =
            resolve_transfer_config(&cli, base, false, None, None, false, false);

        assert!(json_output);
        assert!(
            !config.show_progress,
            "config json_output should suppress progress"
        );
    }

    // === Auto-network merge tests ===

    #[test]
    fn test_auto_network_upgrades_defaults() {
        // A default config should be fully upgraded to network settings
        let base = CopyConfig::default();
        let merged = apply_auto_network(&base);
        let net = CopyConfig::network_preset();

        assert!(merged.resume_enabled, "should enable resume for remote");
        assert!(
            merged.exponential_backoff,
            "should enable backoff for remote"
        );
        assert_eq!(merged.retry_attempts, net.retry_attempts);
        assert_eq!(merged.parallel, net.parallel);
        assert!(!merged.use_zero_copy, "should disable zero-copy for remote");
        assert!(matches!(merged.compression, CompressionType::Zstd { .. }));
    }

    #[test]
    fn test_auto_network_preserves_custom_retry() {
        // User set retry_attempts=2 in config — should NOT be clobbered
        let base = CopyConfig {
            retry_attempts: 2,
            ..CopyConfig::default()
        };
        let merged = apply_auto_network(&base);
        assert_eq!(
            merged.retry_attempts, 2,
            "custom retry_attempts should be preserved"
        );
    }

    #[test]
    fn test_auto_network_preserves_custom_compression() {
        // User set compression=Lz4 in config — should NOT be clobbered to Zstd
        let base = CopyConfig {
            compression: CompressionType::Lz4,
            ..CopyConfig::default()
        };
        let merged = apply_auto_network(&base);
        assert!(
            matches!(merged.compression, CompressionType::Lz4),
            "custom compression should be preserved"
        );
    }

    #[test]
    fn test_auto_network_upgrades_default_retry() {
        // retry_attempts=3 (the default) should be upgraded to 10 (network)
        let base = CopyConfig::default();
        assert_eq!(base.retry_attempts, 3, "precondition: default is 3");
        let merged = apply_auto_network(&base);
        assert_eq!(
            merged.retry_attempts, 10,
            "default retry should be upgraded to network value"
        );
    }

    // === Shorthand subcommand CLI flag passthrough tests ===

    #[test]
    fn test_sync_with_global_flags() {
        // Global flags should parse alongside shorthand subcommands
        let cli = Cli::try_parse_from([
            "orbit",
            "sync",
            "/src",
            "/dst",
            "--quiet",
            "--zstd",
            "--retry-attempts",
            "7",
            "--workers",
            "16",
        ])
        .unwrap();

        // Shorthand data is present
        match &cli.command {
            Some(Commands::Sync { source, dest }) => {
                assert_eq!(source, "/src");
                assert_eq!(dest, "/dst");
            }
            _ => panic!("Expected Sync subcommand"),
        }

        // Global flags are accessible
        assert!(cli.quiet, "--quiet should be parsed for sync");
        assert!(cli.zstd, "--zstd should be parsed for sync");
        assert_eq!(
            cli.retry_attempts,
            Some(7),
            "--retry-attempts should be parsed for sync"
        );
        assert_eq!(cli.workers, Some(16), "--workers should be parsed for sync");
    }

    #[test]
    fn test_backup_with_global_flags() {
        let cli = Cli::try_parse_from([
            "orbit",
            "backup",
            "/src",
            "/dst",
            "--json",
            "--lz4",
            "--no-verify",
        ])
        .unwrap();

        match &cli.command {
            Some(Commands::Backup { .. }) => {}
            _ => panic!("Expected Backup subcommand"),
        }

        assert!(cli.json, "--json should be parsed for backup");
        assert!(cli.lz4, "--lz4 should be parsed for backup");
        assert!(cli.no_verify, "--no-verify should be parsed for backup");
    }

    #[test]
    fn test_mirror_with_global_flags() {
        let cli = Cli::try_parse_from([
            "orbit",
            "mirror",
            "/src",
            "/dst",
            "--raw",
            "--no-stat",
            "--resume",
        ])
        .unwrap();

        match &cli.command {
            Some(Commands::MirrorCmd { .. }) => {}
            _ => panic!("Expected Mirror subcommand"),
        }

        assert!(cli.raw, "--raw should be parsed for mirror");
        assert!(cli.no_stat, "--no-stat should be parsed for mirror");
        assert!(cli.resume, "--resume should be parsed for mirror");
    }

    // === Config resolution integration tests ===
    // These exercise the full resolve_transfer_config pipeline:
    // parsed Cli + base CopyConfig → resolved CopyConfig, proving that
    // shorthand subcommands with global flags produce the correct runtime config.

    #[test]
    fn test_sync_resolved_config_with_flags() {
        let cli = Cli::try_parse_from([
            "orbit",
            "sync",
            "/src",
            "/dst",
            "--quiet",
            "--zstd",
            "--retry-attempts",
            "7",
            "--workers",
            "16",
        ])
        .unwrap();

        let base = CopyConfig::default();
        let (config, json_output, quiet) = resolve_transfer_config(
            &cli,
            base,
            true,                 // is_shorthand
            Some(CopyMode::Sync), // shorthand_mode
            None,                 // no explicit profile
            false,                // local dest
            false,                // source is not a dir
        );

        assert!(!json_output);
        assert!(quiet, "quiet should be true");
        assert_eq!(
            config.copy_mode,
            CopyMode::Sync,
            "shorthand should set Sync mode"
        );
        assert!(
            matches!(config.compression, CompressionType::Zstd { level: 3 }),
            "--zstd should set Zstd compression"
        );
        assert_eq!(config.retry_attempts, 7, "--retry-attempts should override");
        assert_eq!(config.parallel, 16, "--workers should set parallel");
        assert!(config.recursive, "shorthand should enable recursive");
        assert!(
            config.preserve_metadata,
            "shorthand should enable preserve_metadata"
        );
        assert!(!config.show_stats, "quiet should suppress show_stats");
    }

    #[test]
    fn test_backup_resolved_config_with_json() {
        let cli = Cli::try_parse_from([
            "orbit",
            "backup",
            "/src",
            "/dst",
            "--json",
            "--lz4",
            "--no-verify",
        ])
        .unwrap();

        let base = CopyConfig::default();
        let (config, json_output, quiet) = resolve_transfer_config(
            &cli,
            base,
            true,                     // is_shorthand
            None,                     // backup uses profile, not mode
            Some(ProfileArg::Backup), // backup profile
            false,                    // local dest
            false,
        );

        assert!(json_output, "--json should set json_output");
        assert!(!quiet);
        assert!(
            !config.human_readable,
            "json mode should disable human_readable"
        );
        assert!(!config.show_stats, "json mode should disable show_stats");
        assert!(
            !config.show_progress,
            "json mode should disable show_progress"
        );
        assert!(config.json_output, "config.json_output should be true");
        assert!(
            matches!(config.compression, CompressionType::Lz4),
            "--lz4 should set Lz4 compression"
        );
        assert!(
            !config.verify_checksum,
            "--no-verify should disable checksum"
        );
    }

    #[test]
    fn test_mirror_resolved_config_with_raw() {
        let cli = Cli::try_parse_from([
            "orbit",
            "mirror",
            "/src",
            "/dst",
            "--raw",
            "--no-stat",
            "--resume",
        ])
        .unwrap();

        let base = CopyConfig::default();
        let (config, json_output, _quiet) = resolve_transfer_config(
            &cli,
            base,
            true,                   // is_shorthand
            Some(CopyMode::Mirror), // shorthand_mode
            None,
            false,
            false,
        );

        assert!(!json_output);
        assert_eq!(config.copy_mode, CopyMode::Mirror);
        assert!(
            !config.human_readable,
            "--raw should disable human_readable"
        );
        assert!(!config.show_stats, "--no-stat should disable show_stats");
        assert!(config.resume_enabled, "--resume should enable resume");
        assert!(config.recursive, "shorthand should enable recursive");
    }

    #[test]
    fn test_config_json_output_suppresses_human_output_e2e() {
        // Config file has json_output=true; no CLI --json flag.
        // The resolved config should suppress human_readable and show_stats.
        let cli = Cli::try_parse_from(["orbit", "-s", "/src", "-d", "/dst"]).unwrap();

        let base = CopyConfig {
            json_output: true,
            ..CopyConfig::default()
        };

        let (config, json_output, _quiet) = resolve_transfer_config(
            &cli, base, false, // not shorthand
            None, None, false, false,
        );

        assert!(json_output, "config json_output should propagate");
        assert!(
            !config.human_readable,
            "json mode should suppress human_readable"
        );
        assert!(!config.show_stats, "json mode should suppress show_stats");
        assert!(
            !config.show_progress,
            "json mode should suppress show_progress"
        );
    }

    #[test]
    fn test_config_show_stats_false_survives_resolution() {
        // Config file has show_stats=false, human_readable=false. No CLI overrides.
        let cli = Cli::try_parse_from(["orbit", "-s", "/src", "-d", "/dst"]).unwrap();

        let base = CopyConfig {
            show_stats: false,
            human_readable: false,
            ..CopyConfig::default()
        };

        let (config, _json_output, _quiet) =
            resolve_transfer_config(&cli, base, false, None, None, false, false);

        assert!(!config.show_stats, "config show_stats=false should survive");
        assert!(
            !config.human_readable,
            "config human_readable=false should survive"
        );
    }

    #[test]
    fn test_auto_network_merge_through_resolve() {
        // When dest is remote and no profile, auto-network should apply
        let cli = Cli::try_parse_from(["orbit", "-s", "/src", "-d", "s3://bucket/key"]).unwrap();

        let mut base = CopyConfig {
            retry_attempts: 2,
            ..CopyConfig::default()
        };
        base.retry_attempts = 2; // Custom value — should survive

        let (config, _json_output, _quiet) = resolve_transfer_config(
            &cli, base, false, None, None, true, // dest is remote
            false,
        );

        assert_eq!(
            config.retry_attempts, 2,
            "custom retry should survive auto-network"
        );
        assert!(config.resume_enabled, "auto-network should enable resume");
        assert!(
            config.exponential_backoff,
            "auto-network should enable backoff"
        );
        assert!(
            !config.use_zero_copy,
            "auto-network should disable zero-copy"
        );
    }
}
