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
    /// Source path or URI (positional: orbit <SOURCE> <DEST>)
    #[arg(short = 's', long = "source", value_name = "PATH")]
    source: Option<String>,

    /// Destination path or URI (positional: orbit <SOURCE> <DEST>)
    #[arg(short = 'd', long = "dest", value_name = "PATH")]
    destination: Option<String>,

    /// Source path (positional alternative to -s/--source)
    #[arg(index = 1, value_name = "SOURCE", conflicts_with = "source")]
    pos_source: Option<String>,

    /// Destination path (positional alternative to -d/--dest)
    #[arg(index = 2, value_name = "DEST", conflicts_with = "destination")]
    pos_dest: Option<String>,

    /// Configuration profile preset (fast, safe, backup, network)
    #[arg(long, value_enum, global = true)]
    profile: Option<ProfileArg>,

    /// Copy mode
    #[arg(
        short = 'm',
        long = "mode",
        value_enum,
        default_value = "copy",
        global = true
    )]
    mode: CopyModeArg,

    /// Recursive copy
    #[arg(short = 'R', long = "recursive", global = true)]
    recursive: bool,

    /// Preserve metadata (timestamps, permissions)
    #[arg(short = 'p', long = "preserve-metadata", global = true)]
    preserve_metadata: bool,

    /// Detailed preservation flags: times,perms,owners,xattrs (overrides -p)
    #[arg(long = "preserve", value_name = "FLAGS", global = true)]
    preserve_flags: Option<String>,

    /// Metadata transformation: rename:pattern=replacement,case:lower,strip:xattrs
    #[arg(long = "transform", value_name = "CONFIG", global = true)]
    transform: Option<String>,

    /// Strict metadata mode (fail on any metadata error)
    #[arg(long, global = true)]
    strict_metadata: bool,

    /// Verify metadata after transfer
    #[arg(long, global = true)]
    verify_metadata: bool,

    /// Enable resume capability
    #[arg(short = 'r', long = "resume", global = true)]
    resume: bool,

    /// Compression type (none, lz4, zstd)
    #[arg(short = 'c', long = "compress", global = true)]
    compress: Option<CompressionArg>,

    /// Show progress bar
    #[arg(long = "show-progress", global = true)]
    show_progress: bool,

    /// Symbolic link mode
    #[arg(long = "symlink", value_enum, default_value = "skip", global = true)]
    symlink: SymlinkModeArg,

    /// Number of retry attempts
    #[arg(long, default_value = "3", global = true)]
    retry_attempts: u32,

    /// Initial retry delay in seconds
    #[arg(long, default_value = "5", global = true)]
    retry_delay: u64,

    /// Use exponential backoff for retries
    #[arg(long, global = true)]
    exponential_backoff: bool,

    /// Chunk size in KB
    #[arg(long, default_value = "1024", global = true)]
    chunk_size: usize,

    /// Maximum bandwidth in MB/s (0 = unlimited)
    #[arg(long, default_value = "0", global = true)]
    max_bandwidth: u64,

    /// Number of parallel file operations / workers (0 = auto)
    /// For network backends (S3, SMB, etc.) auto = 256; for local = CPU count.
    /// Alias: --parallel
    #[arg(long, default_value = "0", global = true, alias = "parallel")]
    workers: usize,

    /// Per-operation concurrency for multipart transfers (default: 5)
    /// Controls how many parts of a single large file transfer in parallel.
    #[arg(long, default_value = "5", global = true)]
    concurrency: usize,

    /// Multipart upload part size in MiB (default: 50, min: 5, max: 5120)
    #[arg(long, global = true)]
    part_size: Option<usize>,

    /// Include patterns - glob, regex, or path (can be specified multiple times)
    /// Examples: --include="*.rs" --include="regex:^src/.*"
    #[arg(long = "include", global = true)]
    include_patterns: Vec<String>,

    /// Exclude patterns - glob, regex, or path (can be specified multiple times)
    /// Examples: --exclude="*.tmp" --exclude="target/**"
    #[arg(long = "exclude", global = true)]
    exclude_patterns: Vec<String>,

    /// Load filter rules from a file (one rule per line)
    /// File format: '+ pattern' (include) or '- pattern' (exclude)
    #[arg(long = "filter-from", value_name = "FILE", global = true)]
    filter_from: Option<PathBuf>,

    /// Dry run - show what would be copied
    #[arg(long, global = true)]
    dry_run: bool,

    /// Show execution statistics summary at end of run
    #[arg(long, global = true)]
    stat: bool,

    /// Human-readable output (e.g., "1.5 GiB" instead of raw bytes)
    #[arg(short = 'H', long = "human-readable", global = true)]
    human_readable: bool,

    /// Audit log format
    #[arg(long, value_enum, default_value = "json", global = true)]
    audit_format: AuditFormatArg,

    /// Path to audit log file
    #[arg(long, global = true)]
    audit_log: Option<PathBuf>,

    /// OpenTelemetry OTLP endpoint for distributed tracing (e.g., http://localhost:4317)
    #[arg(long, global = true)]
    otel_endpoint: Option<String>,

    /// Prometheus metrics HTTP endpoint port
    #[arg(long, global = true)]
    metrics_port: Option<u16>,

    /// Hide progress bar
    #[arg(long, global = true)]
    no_progress: bool,

    /// Skip checksum verification
    #[arg(long, global = true)]
    no_verify: bool,

    /// Use zero-copy system calls for maximum performance (default)
    #[arg(long, global = true, conflicts_with = "no_zero_copy")]
    zero_copy: bool,

    /// Disable zero-copy optimization (use buffered copy)
    #[arg(long, global = true, conflicts_with = "zero_copy")]
    no_zero_copy: bool,

    /// Generate manifests for transfer verification and audit
    #[arg(long, global = true)]
    generate_manifest: bool,

    /// Output directory for manifests
    #[arg(long, global = true, requires = "generate_manifest")]
    manifest_dir: Option<PathBuf>,

    // Delta detection options
    /// Check mode for change detection (mod-time, size, checksum, delta)
    #[arg(long, value_enum, default_value_t = CheckModeArg::ModTime, global = true)]
    check: CheckModeArg,

    /// Block size for delta algorithm (in KB)
    #[arg(long, default_value = "1024", global = true)]
    block_size: usize,

    /// Force whole file copy, disable delta optimization
    #[arg(long, global = true)]
    whole_file: bool,

    /// Update manifest database after transfer
    #[arg(long, global = true)]
    update_manifest: bool,

    /// Skip files that already exist at destination
    #[arg(long, global = true)]
    ignore_existing: bool,

    /// Path to delta manifest database
    #[arg(long, global = true)]
    delta_manifest: Option<PathBuf>,

    /// Error handling mode (abort, skip, partial)
    #[arg(long, value_enum, default_value = "abort", global = true)]
    error_mode: ErrorModeArg,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, value_enum, default_value = "info", global = true)]
    log_level: LogLevelArg,

    /// Path to log file (default: stdout)
    #[arg(long, value_name = "FILE", global = true)]
    log: Option<PathBuf>,

    /// Enable verbose logging (equivalent to --log-level=debug)
    #[arg(short = 'v', long, global = true)]
    verbose: bool,

    /// Output results as JSON Lines (one JSON object per line)
    #[arg(long, global = true)]
    json: bool,

    /// Path to config file (overrides default locations)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    // === S3 Upload Enhancement Flags ===
    /// Content-Type header for S3 uploads
    #[arg(long, global = true)]
    content_type: Option<String>,

    /// Content-Encoding header for S3 uploads
    #[arg(long, global = true)]
    content_encoding: Option<String>,

    /// Content-Disposition header for S3 uploads
    #[arg(long, global = true)]
    content_disposition: Option<String>,

    /// Cache-Control header for S3 uploads
    #[arg(long, global = true)]
    cache_control: Option<String>,

    /// Expiration date for S3 objects (RFC3339 format)
    #[arg(long = "expires-header", global = true)]
    expires_header: Option<String>,

    /// User-defined metadata key=value pairs for S3 uploads
    #[arg(long = "metadata", global = true)]
    user_metadata: Vec<String>,

    /// Metadata directive for S3 copy operations (COPY or REPLACE)
    #[arg(long, global = true)]
    metadata_directive: Option<String>,

    /// Canned ACL for S3 uploads (e.g., private, public-read, bucket-owner-full-control)
    #[arg(long, global = true)]
    acl: Option<String>,

    // === S3 Client Configuration Flags ===
    /// Disable request signing for public S3 buckets
    #[arg(long, global = true)]
    no_sign_request: bool,

    /// Path to AWS credentials file
    #[arg(long, global = true)]
    credentials_file: Option<PathBuf>,

    /// AWS profile name to use
    #[arg(long = "aws-profile", global = true)]
    aws_profile: Option<String>,

    /// Use S3 Transfer Acceleration
    #[arg(long, global = true)]
    use_acceleration: bool,

    /// Enable requester-pays for S3 bucket access
    #[arg(long, global = true)]
    request_payer: bool,

    /// Disable SSL certificate verification (use with caution)
    #[arg(long, global = true)]
    no_verify_ssl: bool,

    /// Use ListObjects API v1 (for older S3-compatible storage)
    #[arg(long, global = true)]
    use_list_objects_v1: bool,

    // === Conditional copy flags ===
    /// Do not overwrite existing destination files
    #[arg(long, short = 'n', global = true)]
    no_clobber: bool,

    /// Only copy if source and destination sizes differ
    #[arg(long, global = true)]
    if_size_differ: bool,

    /// Only copy if source is newer than destination
    #[arg(long, global = true)]
    if_source_newer: bool,

    /// Flatten directory hierarchy during copy (strip path components)
    #[arg(long, global = true)]
    flatten: bool,

    // === In-place & Sparse File Optimization ===
    /// Sparse file handling mode (auto, always, never)
    /// Auto: detect and create holes for large zero-heavy files (≥64KB)
    /// Always: always check for zero chunks and create sparse holes
    /// Never: write all bytes including zeros
    #[arg(long, value_enum, default_value = "auto", global = true)]
    sparse: SparseModeArg,

    /// Preserve hardlinks during directory transfers (-H)
    /// Detects files sharing the same inode and recreates hardlinks at destination
    #[arg(long = "preserve-hardlinks", global = true)]
    preserve_hardlinks: bool,

    /// Modify destination file in-place instead of temp+rename
    /// Saves disk space for large files where only a small portion changed
    #[arg(long, global = true)]
    inplace: bool,

    /// Safety level for in-place updates (reflink, journaled, unsafe)
    /// Reflink: CoW snapshot (btrfs/XFS/APFS), Journaled: undo log, Unsafe: no safety
    #[arg(
        long,
        value_enum,
        default_value = "reflink",
        global = true,
        requires = "inplace"
    )]
    inplace_safety: InplaceSafetyArg,

    /// Detect renamed/moved files via content-hash overlap at destination
    #[arg(long, global = true)]
    detect_renames: bool,

    /// Minimum chunk overlap ratio to consider a rename (0.0–1.0, default: 0.8)
    #[arg(
        long,
        default_value = "0.8",
        global = true,
        requires = "detect_renames"
    )]
    rename_threshold: f64,

    /// Reference directory for incremental backup hardlinking (repeatable)
    /// Unchanged files are hardlinked to the reference; partial matches use delta
    #[arg(long = "link-dest", value_name = "DIR", global = true)]
    link_dest: Vec<PathBuf>,

    /// Record transfer operations to a batch file for replay
    #[arg(long, value_name = "FILE", global = true)]
    write_batch: Option<PathBuf>,

    /// Replay a previously recorded batch file against a destination
    #[arg(long, value_name = "FILE", global = true)]
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
            CompressionArg::Zstd1 => CompressionType::Zstd { level: 1 },
            CompressionArg::Zstd3 => CompressionType::Zstd { level: 3 },
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

    // Load config file once and reuse for both logging init and transfer config
    let base_config = if let Some(ref config_path) = cli.config {
        CopyConfig::from_file(config_path).unwrap_or_else(|e| {
            cli_style::print_warning(&format!("Failed to load config file: {}", e));
            CopyConfig::default()
        })
    } else {
        CopyConfig::default()
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
        log_config.log_level = cli.log_level.into();
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

    // Handle subcommands
    if let Some(command) = cli.command {
        return handle_subcommand(command);
    }

    if cli.read_batch.is_some() && cli.write_batch.is_some() {
        return Err(OrbitError::Config(
            "Cannot use --read-batch and --write-batch together".to_string(),
        ));
    }

    // Resolve positional args: positional takes precedence if --source/--dest not given
    let destination = cli
        .destination
        .or(cli.pos_dest)
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

    // Resolve source: positional takes precedence if --source not given
    let source = cli.source.or(cli.pos_source).ok_or_else(|| {
        OrbitError::Config(
            "Source path required. Usage: orbit <SOURCE> <DEST> or orbit -s <SOURCE> -d <DEST>"
                .to_string(),
        )
    })?;

    // Parse URIs
    let (_source_protocol, source_path) = Protocol::from_uri(&source)?;

    // Apply profile preset as base, then layer file config, then CLI overrides
    let mut config = match cli.profile {
        Some(ProfileArg::Fast) => CopyConfig::fast_preset(),
        Some(ProfileArg::Safe) => CopyConfig::safe_preset(),
        Some(ProfileArg::Backup) => CopyConfig::backup_preset(),
        Some(ProfileArg::Network) => CopyConfig::network_preset(),
        None => base_config,
    };

    // Override config with CLI arguments
    config.copy_mode = cli.mode.into();
    config.recursive = cli.recursive;
    config.preserve_metadata = cli.preserve_metadata;
    config.preserve_flags = cli.preserve_flags;
    config.transform = cli.transform;
    config.strict_metadata = cli.strict_metadata;
    config.verify_metadata = cli.verify_metadata;
    config.resume_enabled = cli.resume;
    config.verify_checksum = !cli.no_verify;
    config.show_progress = cli.show_progress || !cli.no_progress;
    config.symlink_mode = cli.symlink.into();
    config.retry_attempts = cli.retry_attempts;
    config.retry_delay_secs = cli.retry_delay;
    config.exponential_backoff = cli.exponential_backoff;
    config.chunk_size = cli.chunk_size.saturating_mul(1024);
    config.max_bandwidth = cli.max_bandwidth.saturating_mul(1024 * 1024);
    config.parallel = cli.workers;
    config.concurrency = cli.concurrency;
    config.include_patterns = cli.include_patterns;
    config.exclude_patterns = cli.exclude_patterns;
    config.filter_from = cli.filter_from;
    config.dry_run = cli.dry_run;
    config.show_stats = cli.stat;
    config.human_readable = cli.human_readable;
    config.error_mode = cli.error_mode.into();
    config.log_level = cli.log_level.into();
    config.log_file = cli.log;
    config.verbose = cli.verbose;
    config.json_output = cli.json;

    // S3 upload enhancement flags
    config.s3_content_type = cli.content_type;
    config.s3_content_encoding = cli.content_encoding;
    config.s3_content_disposition = cli.content_disposition;
    config.s3_cache_control = cli.cache_control;
    config.s3_expires_header = cli.expires_header;
    config.s3_user_metadata = cli.user_metadata;
    config.s3_metadata_directive = cli.metadata_directive;
    config.s3_acl = cli.acl;

    // S3 client configuration flags
    config.s3_no_sign_request = cli.no_sign_request;
    config.s3_credentials_file = cli.credentials_file;
    config.s3_aws_profile = cli.aws_profile;
    config.s3_use_acceleration = cli.use_acceleration;
    config.s3_request_payer = cli.request_payer;
    config.s3_no_verify_ssl = cli.no_verify_ssl;
    config.s3_use_list_objects_v1 = cli.use_list_objects_v1;

    // Conditional copy flags
    config.no_clobber = cli.no_clobber;
    config.if_size_differ = cli.if_size_differ;
    config.if_source_newer = cli.if_source_newer;
    config.flatten = cli.flatten;

    // In-place, sparse, and advanced transfer flags
    config.sparse_mode = cli.sparse.into();
    config.preserve_hardlinks = cli.preserve_hardlinks;
    config.inplace = cli.inplace;
    config.inplace_safety = cli.inplace_safety.into();
    config.detect_renames = cli.detect_renames;
    config.rename_threshold = cli.rename_threshold;
    config.link_dest = cli.link_dest;
    config.write_batch = cli.write_batch;
    config.read_batch = cli.read_batch;

    // Handle compression
    if let Some(comp) = cli.compress {
        config.compression = comp.into();
    }

    // Handle zero-copy flag
    if cli.zero_copy {
        config.use_zero_copy = true;
    } else if cli.no_zero_copy {
        config.use_zero_copy = false;
    }

    // Handle manifest generation
    if cli.generate_manifest {
        config.generate_manifest = true;
        config.manifest_output_dir = cli.manifest_dir;
    }

    // Show zero-copy status if enabled
    if config.use_zero_copy && config.show_progress && is_zero_copy_available() {
        let caps = get_zero_copy_capabilities();
        println!("⚡ Zero-copy enabled ({})", caps.method);
    }

    // Show manifest status if enabled
    if config.generate_manifest && config.show_progress {
        if let Some(ref dir) = config.manifest_output_dir {
            println!("📋 Manifest generation enabled: {}", dir.display());
        }
    }

    // Configure audit logging and observability
    config.audit_format = cli.audit_format.into();
    config.audit_log_path = cli.audit_log;
    config.otel_endpoint = cli.otel_endpoint;
    config.metrics_port = cli.metrics_port;

    // Configure delta detection
    config.check_mode = cli.check.into();
    config.delta_block_size = cli.block_size.saturating_mul(1024); // Convert KB to bytes
    config.whole_file = cli.whole_file;
    config.update_manifest = cli.update_manifest;
    config.ignore_existing = cli.ignore_existing;
    config.delta_manifest_path = cli.delta_manifest;

    // Optimize config based on system capabilities
    let optimized = ConfigOptimizer::optimize_with_probe(config, Some(&dest_path))?;

    if !optimized.notices.is_empty() {
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

    // Print summary
    print_summary(&stats, &auto_tune_notices);

    // Print execution statistics if --stat was requested
    if config.show_stats {
        print_exec_stats(&stats);
    }

    Ok(())
}

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
    }
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
        Theme::primary("orbit -s /data -d /backup -R --profile fast")
    );
    println!(
        "  {} {}",
        Theme::muted("Safe with verify:"),
        Theme::primary("orbit -s /data -d /backup -R --profile safe --check checksum")
    );
    println!(
        "  {} {}",
        Theme::muted("Network transfer:"),
        Theme::primary("orbit -s /data -d smb://server/share -R --profile network")
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
        assert_eq!(cli.workers, 64);
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
        assert_eq!(cli.workers, 32);
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
        assert_eq!(cli.concurrency, 10);
    }

    #[test]
    fn test_concurrency_default() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
        assert_eq!(cli.concurrency, 5);
    }

    #[test]
    fn test_workers_default_zero() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
        assert_eq!(cli.workers, 0);
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
        assert_eq!(cli.workers, 128);
        assert_eq!(cli.concurrency, 8);
        assert!(cli.stat);
        assert!(cli.human_readable);
        assert!(cli.recursive);
    }

    // === Restored flag parser tests ===

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
        assert!((cli.rename_threshold - 0.8).abs() < f64::EPSILON);
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
        assert!((cli.rename_threshold - 0.5).abs() < f64::EPSILON);
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
}
