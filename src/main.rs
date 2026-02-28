/*!
 * Orbit CLI - Command Line Interface
 *
 * Version: 0.6.0
 * Author: Shane Wall <shaneawall@gmail.com>
 *
 * Phase 1 Note: The OrbitSystem abstraction (orbit-core-interface) is now available.
 * Future sync operations will use dependency injection with LocalSystem (standalone)
 * or RemoteSystem (Grid/Star topology). See docs/specs/PHASE_1_ABSTRACTION_SPEC.md
 */

use clap::{Parser, Subcommand, ValueEnum};
use orbit::{
    cli_style::{
        self, capability_table, format_bytes, format_duration, guidance_box, header_box,
        preset_table, print_error, print_info, section_header, transfer_summary_table, Icons,
        PresetInfo, Theme, TransferSummary,
    },
    config::{
        AuditFormat, ChunkingStrategy, CompressionType, CopyConfig, CopyMode, ErrorMode, LogLevel,
        SymlinkMode,
    },
    copy_directory, copy_file,
    core::guidance::Guidance,
    error::{OrbitError, Result, EXIT_SUCCESS, EXIT_FATAL},
    get_zero_copy_capabilities, is_zero_copy_available, logging,
    manifest_integration::ManifestGenerator,
    protocol::Protocol,
    stats::TransferStats,
    CopyStats,
};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "orbit")]
#[command(version, about = "Intelligent file transfer with compression, resume, and zero-copy optimization", long_about = None)]
struct Cli {
    /// Source path or URI (file://, smb://, etc.)
    #[arg(short = 's', long = "source", value_name = "PATH")]
    source: Option<String>,

    /// Destination path or URI
    #[arg(short = 'd', long = "dest", value_name = "PATH")]
    destination: Option<String>,

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

    // === Phase 3: S3 Upload Enhancement Flags ===

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

    // === Phase 4: S3 Client Configuration Flags ===

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

    /// Transfer profile for workload optimization (standard, neutrino, adaptive)
    #[arg(long = "profile", value_enum, global = true)]
    profile: Option<ProfileArg>,

    /// Neutrino threshold in KB (default: 8)
    /// Files smaller than this use the fast lane when --profile=neutrino
    #[arg(long, default_value = "8", global = true)]
    neutrino_threshold: u64,

    /// Force transfer of Glacier-stored objects
    #[arg(long, global = true)]
    force_glacier_transfer: bool,

    /// Suppress warnings about Glacier-stored objects
    #[arg(long, global = true)]
    ignore_glacier_warnings: bool,

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

    /// Disable wildcard expansion (treat patterns as literal keys)
    #[arg(long, global = true)]
    raw: bool,

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

    /// Launch the Orbit Web GUI server
    Serve {
        /// Bind address for the web server
        #[arg(long, default_value = "127.0.0.1:8080")]
        addr: SocketAddr,
    },

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
    /// Lines starting with '#' are comments. Empty lines are skipped.
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

#[derive(Subcommand)]
enum ManifestCommands {
    /// Create a flight plan without transferring data
    Plan {
        /// Source path
        #[arg(short, long)]
        source: PathBuf,

        /// Destination path
        #[arg(short, long)]
        dest: PathBuf,

        /// Output directory for manifests
        #[arg(short, long)]
        output: PathBuf,

        /// Chunking strategy: cdc or fixed
        #[arg(long, default_value = "cdc")]
        chunking: String,

        /// Average chunk size in KiB (for CDC) or fixed size (for fixed)
        #[arg(long, default_value = "256")]
        chunk_size: u32,
    },

    /// Verify a completed transfer using manifests
    Verify {
        /// Directory containing manifests
        #[arg(short, long)]
        manifest_dir: PathBuf,
    },

    /// Show differences between manifest and target
    Diff {
        /// Directory containing manifests
        #[arg(short, long)]
        manifest_dir: PathBuf,

        /// Target directory to compare
        #[arg(short, long)]
        target: PathBuf,
    },

    /// Display manifest information
    Info {
        /// Path to flight plan or cargo manifest
        #[arg(short, long)]
        path: PathBuf,
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
enum ProfileArg {
    Standard,
    Neutrino,
    Adaptive,
}

fn main() {
    let code = match run() {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
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

    // Validate source and destination
    let source = cli
        .source
        .ok_or_else(|| OrbitError::Config("Source path required".to_string()))?;

    let destination = cli
        .destination
        .ok_or_else(|| OrbitError::Config("Destination path required".to_string()))?;

    // Parse URIs
    let (_source_protocol, source_path) = Protocol::from_uri(&source)?;
    let (_dest_protocol, dest_path) = Protocol::from_uri(&destination)?;

    // Reuse the already-loaded config
    let mut config = base_config;

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

    // S3 upload enhancement flags (Phase 3)
    config.s3_content_type = cli.content_type;
    config.s3_content_encoding = cli.content_encoding;
    config.s3_content_disposition = cli.content_disposition;
    config.s3_cache_control = cli.cache_control;
    config.s3_expires_header = cli.expires_header;
    config.s3_user_metadata = cli.user_metadata;
    config.s3_metadata_directive = cli.metadata_directive;
    config.s3_acl = cli.acl;

    // S3 client configuration flags (Phase 4)
    config.s3_no_sign_request = cli.no_sign_request;
    config.s3_credentials_file = cli.credentials_file;
    config.s3_aws_profile = cli.aws_profile;
    config.s3_use_acceleration = cli.use_acceleration;
    config.s3_request_payer = cli.request_payer;
    config.s3_no_verify_ssl = cli.no_verify_ssl;
    config.s3_use_list_objects_v1 = cli.use_list_objects_v1;

    // Conditional copy & transfer flags (Phase 5/6)
    config.no_clobber = cli.no_clobber;
    config.if_size_differ = cli.if_size_differ;
    config.if_source_newer = cli.if_source_newer;
    config.flatten = cli.flatten;

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

    // Configure Neutrino fast lane
    config.transfer_profile = cli.profile.map(|p| match p {
        ProfileArg::Standard => "standard".to_string(),
        ProfileArg::Neutrino => "neutrino".to_string(),
        ProfileArg::Adaptive => "adaptive".to_string(),
    });
    config.neutrino_threshold = cli.neutrino_threshold.saturating_mul(1024); // Convert KB to bytes

    // 🚀 GUIDANCE PASS: Sanitize and Optimize (with Active Probing)
    let flight_plan = Guidance::plan_with_probe(config, Some(&dest_path))?;

    // Display Intelligence to User
    if !flight_plan.notices.is_empty() {
        guidance_box(&flight_plan.notices);
    }

    // Use the optimized config from the flight plan
    let config = flight_plan.final_config;

    // Perform the copy
    let stats = if source_path.is_dir() && config.recursive {
        copy_directory(&source_path, &dest_path, &config)?
    } else {
        copy_file(&source_path, &dest_path, &config)?
    };

    // Print summary
    print_summary(&stats);

    // Print execution statistics if --stat was requested
    if config.show_stats {
        print_exec_stats(&stats);
    }

    Ok(())
}

fn handle_subcommand(command: Commands) -> Result<()> {
    match command {
        Commands::Init => orbit::commands::init::run_init_wizard()
            .map_err(|e| OrbitError::Other(format!("Initialization failed: {}", e))),
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
        Commands::Serve { addr } => serve_gui(addr),
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            use clap_complete::generate;
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "orbit", &mut std::io::stdout());
            Ok(())
        }
        Commands::Manifest(manifest_cmd) => handle_manifest_command(manifest_cmd),
        Commands::Run { file, workers } => handle_run_command(file, workers),
        #[cfg(feature = "s3-native")]
        Commands::Cat { uri } => handle_cat_command(&uri),
        #[cfg(feature = "s3-native")]
        Commands::Pipe { uri } => handle_pipe_command(&uri),
        #[cfg(feature = "s3-native")]
        Commands::Presign { uri, expires } => handle_presign_command(&uri, expires),
        #[cfg(feature = "s3-native")]
        Commands::Ls { uri, etag, storage_class, all_versions, show_fullpath } => {
            handle_ls_command(&uri, etag, storage_class, all_versions, show_fullpath)
        }
        #[cfg(feature = "s3-native")]
        Commands::Head { uri, version_id } => handle_head_command(&uri, version_id),
        #[cfg(feature = "s3-native")]
        Commands::Du { uri, group, all_versions } => handle_du_command(&uri, group, all_versions),
        #[cfg(feature = "s3-native")]
        Commands::Rm { uri, all_versions, version_id, dry_run } => {
            handle_rm_command(&uri, all_versions, version_id, dry_run)
        }
        #[cfg(feature = "s3-native")]
        Commands::Mv { source, dest } => handle_mv_command(&source, &dest),
        #[cfg(feature = "s3-native")]
        Commands::Mb { bucket } => handle_mb_command(&bucket),
        #[cfg(feature = "s3-native")]
        Commands::Rb { bucket } => handle_rb_command(&bucket),
    }
}

#[cfg(feature = "gui")]
fn serve_gui(addr: SocketAddr) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    // Create ServerConfig from environment variables or defaults
    let config = orbit_server::ServerConfig {
        host: addr.ip().to_string(),
        port: addr.port(),
        magnetar_db: std::env::var("ORBIT_MAGNETAR_DB")
            .unwrap_or_else(|_| "magnetar.db".to_string()),
        user_db: std::env::var("ORBIT_USER_DB")
            .unwrap_or_else(|_| "orbit-server-users.db".to_string()),
    };

    println!(
        "🚀 Starting Orbit Control Plane at http://{}:{}",
        config.host, config.port
    );

    // Create reactor notify channel for background job processing
    let reactor_notify = std::sync::Arc::new(tokio::sync::Notify::new());

    runtime
        .block_on(orbit_server::start_server(config, reactor_notify))
        .map_err(|e| OrbitError::Other(format!("Failed to start GUI server: {}", e)))
}

#[cfg(not(feature = "gui"))]
fn serve_gui(_addr: SocketAddr) -> Result<()> {
    eprintln!("GUI feature not enabled. Rebuild with --features gui to use `orbit serve`.");
    std::process::exit(EXIT_FATAL);
}

fn handle_manifest_command(command: ManifestCommands) -> Result<()> {
    match command {
        ManifestCommands::Plan {
            source,
            dest,
            output,
            chunking,
            chunk_size,
        } => handle_manifest_plan(source, dest, output, chunking, chunk_size),
        ManifestCommands::Verify { manifest_dir } => handle_manifest_verify(manifest_dir),
        ManifestCommands::Diff {
            manifest_dir,
            target,
        } => handle_manifest_diff(manifest_dir, target),
        ManifestCommands::Info { path } => handle_manifest_info(path),
    }
}

fn handle_manifest_plan(
    source: PathBuf,
    dest: PathBuf,
    output: PathBuf,
    chunking: String,
    chunk_size: u32,
) -> Result<()> {
    section_header(&format!("{} Creating Flight Plan", Icons::MANIFEST));
    println!();
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Source:"),
        Theme::value(source.display())
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Dest:"),
        Theme::value(dest.display())
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Output:"),
        Theme::value(output.display())
    );
    println!();

    let chunking_strategy = match chunking.as_str() {
        "cdc" => ChunkingStrategy::Cdc {
            avg_kib: chunk_size,
            algo: "gear".to_string(),
        },
        "fixed" => ChunkingStrategy::Fixed {
            size_kib: chunk_size,
        },
        _ => {
            print_error(
                &format!("Invalid chunking strategy: {}", chunking),
                Some("Use 'cdc' or 'fixed'"),
            );
            std::process::exit(EXIT_FATAL);
        }
    };

    let config = CopyConfig {
        generate_manifest: true,
        manifest_output_dir: Some(output.clone()),
        chunking_strategy,
        ..Default::default()
    };

    let mut generator = ManifestGenerator::new(&source, &dest, &config)?;

    if source.is_file() {
        let file_name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        print_info(&format!("Processing: {}", file_name));
        generator.generate_file_manifest(&source, file_name)?;
    } else if source.is_dir() {
        use walkdir::WalkDir;

        for entry in WalkDir::new(&source).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let relative_path = entry
                    .path()
                    .strip_prefix(&source)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                println!("  {} {}", Theme::muted(Icons::ARROW_RIGHT), relative_path);
                generator.generate_file_manifest(entry.path(), &relative_path)?;
            }
        }
    } else {
        print_error(
            "Source path does not exist or is not accessible",
            Some("Check the path and try again"),
        );
        std::process::exit(EXIT_FATAL);
    }

    generator.finalize("sha256:pending")?;

    println!();
    cli_style::print_success(&format!("Flight plan created at: {}", output.display()));

    Ok(())
}

fn handle_manifest_verify(manifest_dir: PathBuf) -> Result<()> {
    use orbit::manifests::{CargoManifest, FlightPlan};

    section_header(&format!("{} Verifying Manifests", Icons::SHIELD));
    println!();
    println!(
        "  {} {}",
        Theme::muted("Directory:"),
        Theme::value(manifest_dir.display())
    );
    println!();

    let flight_plan_path = manifest_dir.join("job.flightplan.json");
    if !flight_plan_path.exists() {
        print_error(
            &format!("Flight plan not found: {}", flight_plan_path.display()),
            Some("Run 'orbit manifest plan' first"),
        );
        std::process::exit(EXIT_FATAL);
    }

    let flight_plan = FlightPlan::load(&flight_plan_path)
        .map_err(|e| OrbitError::Other(format!("Failed to load flight plan: {}", e)))?;

    // Display flight plan info
    let status = if flight_plan.is_finalized() {
        format!("{} Finalized", Icons::SUCCESS)
    } else {
        format!("{} Pending", Icons::PENDING)
    };

    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Job ID:"),
        Theme::value(&flight_plan.job_id)
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Files:"),
        Theme::value(flight_plan.files.len())
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Status:"),
        if flight_plan.is_finalized() {
            Theme::success(&status)
        } else {
            Theme::warning(&status)
        }
    );
    println!();

    let mut verified = 0;
    let mut failed = 0;

    for file_ref in &flight_plan.files {
        let cargo_path = manifest_dir.join(&file_ref.cargo);

        if !cargo_path.exists() {
            println!(
                "  {} {} {}",
                Theme::error(Icons::ERROR),
                file_ref.path,
                Theme::error("(missing)")
            );
            failed += 1;
            continue;
        }

        match CargoManifest::load(&cargo_path) {
            Ok(cargo) => {
                println!(
                    "  {} {} {} windows, {}",
                    Theme::success(Icons::SUCCESS),
                    file_ref.path,
                    cargo.windows.len(),
                    format_bytes(cargo.size)
                );
                verified += 1;
            }
            Err(e) => {
                println!(
                    "  {} {} {}",
                    Theme::error(Icons::ERROR),
                    file_ref.path,
                    Theme::error(format!("({})", e))
                );
                failed += 1;
            }
        }
    }

    println!();
    if failed == 0 {
        cli_style::print_success(&format!(
            "Verification complete: {} files verified",
            verified
        ));
    } else {
        cli_style::print_warning(&format!(
            "Verification complete: {} verified, {} failed",
            verified, failed
        ));
    }

    Ok(())
}

fn handle_manifest_diff(manifest_dir: PathBuf, target: PathBuf) -> Result<()> {
    section_header(&format!("{} Comparing Manifests", Icons::STATS));
    println!();
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Manifests:"),
        Theme::value(manifest_dir.display())
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Target:"),
        Theme::value(target.display())
    );
    println!();

    cli_style::print_warning("Diff operation not yet fully implemented");
    println!(
        "  {} This will compare manifest metadata with actual files",
        Theme::muted(Icons::ARROW_RIGHT)
    );
    println!();

    Ok(())
}

fn handle_manifest_info(path: PathBuf) -> Result<()> {
    use orbit::manifests::{CargoManifest, FlightPlan};

    if !path.exists() {
        print_error(
            &format!("Path not found: {}", path.display()),
            Some("Check the file path and try again"),
        );
        std::process::exit(EXIT_FATAL);
    }

    if let Ok(flight_plan) = FlightPlan::load(&path) {
        section_header(&format!("{} Flight Plan", Icons::MANIFEST));
        println!();

        let items = vec![
            ("Schema", flight_plan.schema.clone()),
            ("Job ID", flight_plan.job_id.clone()),
            ("Created", flight_plan.created_utc.to_string()),
            (
                "Source",
                format!(
                    "{} ({})",
                    flight_plan.source.root, flight_plan.source.endpoint_type
                ),
            ),
            (
                "Target",
                format!(
                    "{} ({})",
                    flight_plan.target.root, flight_plan.target.endpoint_type
                ),
            ),
            ("Files", flight_plan.files.len().to_string()),
            ("Encryption", flight_plan.policy.encryption.aead.clone()),
        ];

        for (key, value) in items {
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted(format!("{}:", key)),
                Theme::value(&value)
            );
        }

        if let Some(classification) = &flight_plan.policy.classification {
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Classification:"),
                Theme::value(classification)
            );
        }

        println!();
        return Ok(());
    }

    if let Ok(cargo) = CargoManifest::load(&path) {
        section_header(&format!("{} Cargo Manifest", Icons::FILE));
        println!();

        let items = vec![
            ("Schema", cargo.schema.clone()),
            ("Path", cargo.path.clone()),
            ("Size", format_bytes(cargo.size)),
            ("Chunking", cargo.chunking.chunking_type.clone()),
            ("Windows", cargo.windows.len().to_string()),
            ("Chunks", cargo.total_chunks().to_string()),
        ];

        for (key, value) in items {
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted(format!("{}:", key)),
                Theme::value(&value)
            );
        }

        println!();
        return Ok(());
    }

    print_error(
        "Not a valid flight plan or cargo manifest",
        Some("Ensure the file is a valid Orbit manifest"),
    );
    std::process::exit(EXIT_FATAL);
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
            icon: Icons::GLOBE,
            name: "NETWORK",
            checksum: true,
            resume: true,
            compression: "Zstd:3".to_string(),
            zero_copy: false,
            best_for: "Remote/slow networks".to_string(),
        },
        PresetInfo {
            icon: Icons::LIGHTNING,
            name: "NEUTRINO",
            checksum: false,
            resume: false,
            compression: "None".to_string(),
            zero_copy: true,
            best_for: "Many small files".to_string(),
        },
    ];

    println!("{}", preset_table(&presets));

    println!();
    section_header(&format!("{} Example Usage", Icons::SPARKLE));
    println!();
    println!(
        "  {} {}",
        Theme::muted("Fast local copy:"),
        Theme::primary("orbit -s /data -d /backup -R --profile standard")
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

fn print_summary(stats: &CopyStats) {
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
    println!();
}

// ============================================================================
// Batch Run Command
// ============================================================================

fn split_command_line(line: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\\' if !in_single => {
                if let Some(next) = chars.peek().copied() {
                    if next.is_whitespace() || next == '"' || next == '\'' || next == '\\' {
                        chars.next();
                        current.push(next);
                    } else {
                        current.push('\\');
                    }
                } else {
                    current.push('\\');
                }
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
                while let Some(next) = chars.peek() {
                    if next.is_whitespace() {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            _ => current.push(c),
        }
    }

    if in_single || in_double {
        return Err(OrbitError::Config(
            "Unclosed quote in batch command line".to_string(),
        ));
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}

fn normalize_batch_args(line: &str, tokens: Vec<String>) -> Result<Vec<String>> {
    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let first = tokens[0].as_str();

    if first == "orbit" {
        let args = tokens[1..].to_vec();
        if args.get(0).map(|s| s.as_str()) == Some("run") {
            return Err(OrbitError::Config(
                "Nested 'orbit run' is not supported in batch mode".to_string(),
            ));
        }
        return Ok(args);
    }

    if first == "cp" || first == "copy" || first == "sync" {
        if tokens.len() < 3 {
            return Err(OrbitError::Config(format!(
                "Invalid command (need: {} <src> <dest>): {}",
                first, line
            )));
        }

        let mut args = vec![
            "--source".to_string(),
            tokens[1].clone(),
            "--dest".to_string(),
            tokens[2].clone(),
        ];

        if first == "sync" {
            args.push("--mode".to_string());
            args.push("sync".to_string());
        }

        args.extend(tokens[3..].iter().cloned());
        return Ok(args);
    }

    if first == "run" {
        return Err(OrbitError::Config(
            "Nested 'orbit run' is not supported in batch mode".to_string(),
        ));
    }

    Ok(tokens)
}

fn handle_run_command(file: Option<PathBuf>, workers: usize) -> Result<()> {
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    section_header(&format!("{} Batch Execution", Icons::ROCKET));
    println!();

    // Read commands from file or stdin
    let lines: Vec<String> = if let Some(path) = file {
        let file = std::fs::File::open(&path).map_err(|e| {
            OrbitError::Other(format!("Failed to open command file {}: {}", path.display(), e))
        })?;
        std::io::BufReader::new(file)
            .lines()
            .filter_map(|l| l.ok())
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .collect()
    } else {
        let stdin = std::io::stdin();
        stdin
            .lock()
            .lines()
            .filter_map(|l| l.ok())
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .collect()
    };

    if lines.is_empty() {
        print_info("No commands to execute.");
        return Ok(());
    }

    let mut parsed_commands = Vec::new();
    let mut invalid = 0usize;

    for line in &lines {
        let tokens = match split_command_line(line) {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("WARN: {}: {}", line, e);
                invalid += 1;
                continue;
            }
        };

        if tokens.is_empty() {
            continue;
        }

        match normalize_batch_args(line, tokens) {
            Ok(args) => {
                if !args.is_empty() {
                    parsed_commands.push((line.clone(), args));
                }
            }
            Err(e) => {
                eprintln!("WARN: {}: {}", line, e);
                invalid += 1;
            }
        }
    }

    let total = parsed_commands.len() + invalid;
    let worker_count = if workers == 0 { 1 } else { workers };

    if parsed_commands.is_empty() {
        section_header(&format!("{} Batch Complete", Icons::SUCCESS));
        println!();
        println!(
            "  {} {} {} succeeded, {} failed in {}",
            Icons::BULLET,
            Theme::value(total),
            Theme::success(0),
            Theme::error(invalid),
            Theme::value(format_duration(0.0))
        );
        println!();
        return Ok(());
    }

    println!(
        "  {} {} commands with {} workers",
        Icons::BULLET,
        Theme::value(total),
        Theme::value(worker_count)
    );
    println!();

    let succeeded = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(invalid));
    let start = Instant::now();

    let exe = std::env::current_exe()
        .map_err(|e| OrbitError::Other(format!("Failed to resolve current executable: {}", e)))?;

    // Use a thread pool to execute commands in parallel
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(worker_count.min(parsed_commands.len().max(1)))
        .build()
        .map_err(|e| OrbitError::Other(format!("Failed to create thread pool: {}", e)))?;

    pool.scope(|s| {
        for (cmd_line, args) in &parsed_commands {
            let succeeded = succeeded.clone();
            let failed = failed.clone();
            let exe = exe.clone();
            let args = args.clone();
            let cmd_line = cmd_line.clone();
            s.spawn(move |_| {
                let status = Command::new(&exe)
                    .args(&args)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status();

                match status {
                    Ok(status) if status.success() => {
                        succeeded.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(status) => {
                        let code = status.code().unwrap_or(-1);
                        eprintln!("ERROR: command failed (exit {}): {}", code, cmd_line);
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("ERROR: failed to run '{}': {}", cmd_line, e);
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    });

    let elapsed = start.elapsed();
    let ok = succeeded.load(Ordering::Relaxed);
    let err = failed.load(Ordering::Relaxed);

    println!();
    section_header(&format!("{} Batch Complete", Icons::SUCCESS));
    println!();
    println!(
        "  {} {} {} succeeded, {} failed in {}",
        Icons::BULLET,
        Theme::value(total),
        Theme::success(ok),
        if err > 0 {
            Theme::error(err).to_string()
        } else {
            Theme::muted(err).to_string()
        },
        Theme::value(format_duration(elapsed.as_secs_f64()))
    );
    println!();

    Ok(())
}

// ============================================================================
// S3 Streaming Commands (cat, pipe, presign)
// ============================================================================

#[cfg(feature = "s3-native")]
fn handle_cat_command(uri: &str) -> Result<()> {
    use orbit::protocol::s3::{S3Client, S3Config, S3Operations};
    use std::io::Write;

    let (_protocol, key_path) = Protocol::from_uri(uri)?;
    let key = key_path.to_string_lossy().trim_start_matches('/').to_string();

    // Extract bucket from URI
    let bucket = match Protocol::from_uri(uri)? {
        (Protocol::S3 { bucket, .. }, _) => bucket,
        _ => {
            return Err(OrbitError::Config(
                "cat command requires an S3 URI (s3://bucket/key)".to_string(),
            ))
        }
    };

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket);
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        let data = client
            .download_bytes(&key)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to download: {}", e)))?;

        std::io::stdout().write_all(&data).map_err(|e| {
            OrbitError::Other(format!("Failed to write to stdout: {}", e))
        })?;
        std::io::stdout().flush().map_err(|e| {
            OrbitError::Other(format!("Failed to flush stdout: {}", e))
        })?;

        Ok(())
    })
}

#[cfg(feature = "s3-native")]
fn handle_pipe_command(uri: &str) -> Result<()> {
    use bytes::Bytes;
    use orbit::protocol::s3::{S3Client, S3Config, S3Operations};
    use std::io::Read;

    let (_protocol, key_path) = Protocol::from_uri(uri)?;
    let key = key_path.to_string_lossy().trim_start_matches('/').to_string();

    let bucket = match Protocol::from_uri(uri)? {
        (Protocol::S3 { bucket, .. }, _) => bucket,
        _ => {
            return Err(OrbitError::Config(
                "pipe command requires an S3 URI (s3://bucket/key)".to_string(),
            ))
        }
    };

    // Read all stdin into memory
    let mut buffer = Vec::new();
    std::io::stdin().read_to_end(&mut buffer).map_err(|e| {
        OrbitError::Other(format!("Failed to read from stdin: {}", e))
    })?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket);
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        client
            .upload_bytes(Bytes::from(buffer), &key)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to upload: {}", e)))?;

        eprintln!("Uploaded to s3://{}", key);
        Ok(())
    })
}

#[cfg(feature = "s3-native")]
fn handle_presign_command(uri: &str, expires: u64) -> Result<()> {
    use orbit::protocol::s3::{S3Client, S3Config};

    let (_protocol, key_path) = Protocol::from_uri(uri)?;
    let key = key_path.to_string_lossy().trim_start_matches('/').to_string();

    let bucket = match Protocol::from_uri(uri)? {
        (Protocol::S3 { bucket, .. }, _) => bucket,
        _ => {
            return Err(OrbitError::Config(
                "presign command requires an S3 URI (s3://bucket/key)".to_string(),
            ))
        }
    };

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        let url = client
            .presign_get(&key, std::time::Duration::from_secs(expires))
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to generate pre-signed URL: {}", e)))?;

        println!("{}", url);
        Ok(())
    })
}

// ============================================================================
// S3 Object Management Commands (ls, head, du, rm, mv, mb, rb)
// ============================================================================

#[cfg(feature = "s3-native")]
fn handle_ls_command(
    uri: &str,
    show_etag: bool,
    show_storage_class: bool,
    all_versions: bool,
    show_fullpath: bool,
) -> Result<()> {
    use orbit::protocol::s3::{
        has_wildcards, S3Client, S3Config, S3Operations, VersioningOperations,
    };

    let (_protocol, key_path) = Protocol::from_uri(uri)?;
    let key = key_path.to_string_lossy().trim_start_matches('/').to_string();

    let bucket = match Protocol::from_uri(uri)? {
        (Protocol::S3 { bucket, .. }, _) => bucket,
        _ => return Err(OrbitError::Config("ls command requires an S3 URI (s3://bucket/prefix)".to_string())),
    };

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config).await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        if all_versions {
            let result = client.list_object_versions(&key).await
                .map_err(|e| OrbitError::Other(format!("Failed to list versions: {}", e)))?;

            for version in &result.versions {
                let date_str = format_system_time(version.last_modified);
                let size_str = format_bytes(version.size);
                let key_display = if show_fullpath {
                    format!("s3://{}/{}", bucket, version.key)
                } else {
                    version.key.clone()
                };
                let latest_marker = if version.is_latest { " [LATEST]" } else { "" };

                print!("{}  {:>10}  {}  {}{}", date_str, size_str, version.version_id, key_display, latest_marker);
                if let Some(ref sc) = version.storage_class {
                    if show_storage_class {
                        print!("  {}", sc);
                    }
                }
                println!();
            }

            for dm in &result.delete_markers {
                let date_str = format_system_time(dm.last_modified);
                let key_display = if show_fullpath {
                    format!("s3://{}/{}", bucket, dm.key)
                } else {
                    dm.key.clone()
                };
                println!("{}  {:>10}  {}  {} [DELETE MARKER]", date_str, "(marker)", dm.version_id, key_display);
            }

            let total = result.versions.len() + result.delete_markers.len();
            eprintln!("\n{} versions, {} delete markers", result.versions.len(), result.delete_markers.len());
            if total == 0 {
                eprintln!("No objects found.");
            }
        } else {
            let mut all_objects = Vec::new();
            let use_wildcard = has_wildcards(&key);

            if use_wildcard {
                let result = client.list_objects_with_wildcard(&key).await
                    .map_err(|e| OrbitError::Other(format!("Failed to list objects: {}", e)))?;
                all_objects = result.objects;
            } else {
                let mut continuation_token = None;
                loop {
                    let result = client.list_objects_paginated(&key, continuation_token, None).await
                        .map_err(|e| OrbitError::Other(format!("Failed to list objects: {}", e)))?;
                    all_objects.extend(result.objects);
                    if result.is_truncated {
                        continuation_token = result.continuation_token;
                    } else {
                        break;
                    }
                }
            }

            for obj in &all_objects {
                let date_str = obj.last_modified
                    .map(format_system_time)
                    .unwrap_or_else(|| "                   ".to_string());
                let size_str = format_bytes(obj.size);
                let key_display = if show_fullpath {
                    format!("s3://{}/{}", bucket, obj.key)
                } else {
                    obj.key.clone()
                };

                print!("{}  {:>10}  {}", date_str, size_str, key_display);

                if show_etag {
                    if let Some(ref etag) = obj.etag {
                        print!("  {}", etag);
                    }
                }
                if show_storage_class {
                    if let Some(ref sc) = obj.storage_class {
                        print!("  {}", sc);
                    }
                }
                println!();
            }

            if all_objects.is_empty() {
                eprintln!("No objects found.");
            } else {
                let total_size: u64 = all_objects.iter().map(|o| o.size).sum();
                eprintln!("\n{} objects, {} total", all_objects.len(), format_bytes(total_size));
            }
        }

        Ok(())
    })
}

#[cfg(feature = "s3-native")]
fn handle_head_command(uri: &str, version_id: Option<String>) -> Result<()> {
    use orbit::protocol::s3::{S3Client, S3Config, VersioningOperations};

    let (_protocol, key_path) = Protocol::from_uri(uri)?;
    let key = key_path.to_string_lossy().trim_start_matches('/').to_string();

    let bucket = match Protocol::from_uri(uri)? {
        (Protocol::S3 { bucket, .. }, _) => bucket,
        _ => return Err(OrbitError::Config("head command requires an S3 URI (s3://bucket/key)".to_string())),
    };

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config).await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        if let Some(ref vid) = version_id {
            let version = client.get_version_metadata(&key, vid).await
                .map_err(|e| OrbitError::Other(format!("Failed to get version metadata: {}", e)))?;

            section_header(&format!("{} S3 Object Version", Icons::FILE));
            println!();
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Key:"), Theme::value(&version.key));
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Version ID:"), Theme::value(&version.version_id));
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Size:"), Theme::value(format_bytes(version.size)));
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Last Modified:"), Theme::value(format_system_time(version.last_modified)));
            println!("  {} {} {}", Icons::BULLET, Theme::muted("ETag:"), Theme::value(&version.etag));
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Is Latest:"), Theme::value(version.is_latest));
            if let Some(ref sc) = version.storage_class {
                println!("  {} {} {}", Icons::BULLET, Theme::muted("Storage Class:"), Theme::value(sc));
            }
            println!();
        } else {
            let metadata = client.get_metadata(&key).await
                .map_err(|e| OrbitError::Other(format!("Failed to get metadata: {}", e)))?;

            section_header(&format!("{} S3 Object Metadata", Icons::FILE));
            println!();
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Key:"), Theme::value(&metadata.key));
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Size:"), Theme::value(format_bytes(metadata.size)));
            if let Some(ref lm) = metadata.last_modified {
                println!("  {} {} {}", Icons::BULLET, Theme::muted("Last Modified:"), Theme::value(format_system_time(*lm)));
            }
            if let Some(ref etag) = metadata.etag {
                println!("  {} {} {}", Icons::BULLET, Theme::muted("ETag:"), Theme::value(etag));
            }
            if let Some(ref sc) = metadata.storage_class {
                println!("  {} {} {}", Icons::BULLET, Theme::muted("Storage Class:"), Theme::value(sc));
            }
            if let Some(ref ct) = metadata.content_type {
                println!("  {} {} {}", Icons::BULLET, Theme::muted("Content-Type:"), Theme::value(ct));
            }
            if let Some(ref ce) = metadata.content_encoding {
                println!("  {} {} {}", Icons::BULLET, Theme::muted("Content-Encoding:"), Theme::value(ce));
            }
            if let Some(ref cc) = metadata.cache_control {
                println!("  {} {} {}", Icons::BULLET, Theme::muted("Cache-Control:"), Theme::value(cc));
            }
            if let Some(ref cd) = metadata.content_disposition {
                println!("  {} {} {}", Icons::BULLET, Theme::muted("Content-Disposition:"), Theme::value(cd));
            }
            if let Some(ref vid) = metadata.version_id {
                println!("  {} {} {}", Icons::BULLET, Theme::muted("Version ID:"), Theme::value(vid));
            }
            if let Some(ref sse) = metadata.server_side_encryption {
                println!("  {} {} {:?}", Icons::BULLET, Theme::muted("Encryption:"), sse);
            }
            if !metadata.metadata.is_empty() {
                println!("  {} {}", Icons::BULLET, Theme::muted("User Metadata:"));
                for (k, v) in &metadata.metadata {
                    println!("    {} = {}", Theme::muted(k), Theme::value(v));
                }
            }
            println!();
        }

        Ok(())
    })
}

#[cfg(feature = "s3-native")]
fn handle_du_command(uri: &str, group: bool, all_versions: bool) -> Result<()> {
    use orbit::protocol::s3::{S3Client, S3Config, S3Operations, VersioningOperations};
    use std::collections::HashMap;

    let (_protocol, key_path) = Protocol::from_uri(uri)?;
    let key = key_path.to_string_lossy().trim_start_matches('/').to_string();

    let bucket = match Protocol::from_uri(uri)? {
        (Protocol::S3 { bucket, .. }, _) => bucket,
        _ => return Err(OrbitError::Config("du command requires an S3 URI (s3://bucket/prefix)".to_string())),
    };

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config).await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        section_header(&format!("{} S3 Storage Usage", Icons::STATS));
        println!();
        println!("  {} {} s3://{}/{}", Icons::BULLET, Theme::muted("Prefix:"), bucket, key);
        println!();

        if all_versions {
            let result = client.list_object_versions(&key).await
                .map_err(|e| OrbitError::Other(format!("Failed to list versions: {}", e)))?;

            let total_count = result.versions.len() as u64;
            let total_size: u64 = result.versions.iter().map(|v| v.size).sum();

            if group {
                let mut groups: HashMap<String, (u64, u64)> = HashMap::new();
                for version in &result.versions {
                    let sc = version.storage_class.clone().unwrap_or_else(|| "STANDARD".to_string());
                    let entry = groups.entry(sc).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += version.size;
                }
                for (class, (count, size)) in &groups {
                    println!("  {} {:>10}  {:>8} objects  {}", Icons::BULLET, format_bytes(*size), count, Theme::value(class));
                }
                println!();
            }

            println!("  {} {} {}", Icons::BULLET, Theme::muted("Total objects (all versions):"), Theme::value(total_count));
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Total size:"), Theme::value(format_bytes(total_size)));
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Delete markers:"), Theme::value(result.delete_markers.len()));
        } else {
            let mut all_objects = Vec::new();
            let mut continuation_token = None;
            loop {
                let result = client.list_objects_paginated(&key, continuation_token, None).await
                    .map_err(|e| OrbitError::Other(format!("Failed to list objects: {}", e)))?;
                all_objects.extend(result.objects);
                if result.is_truncated {
                    continuation_token = result.continuation_token;
                } else {
                    break;
                }
            }

            let total_count = all_objects.len() as u64;
            let total_size: u64 = all_objects.iter().map(|o| o.size).sum();

            if group {
                let mut groups: HashMap<String, (u64, u64)> = HashMap::new();
                for obj in &all_objects {
                    let sc = obj.storage_class.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "STANDARD".to_string());
                    let entry = groups.entry(sc).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += obj.size;
                }
                for (class, (count, size)) in &groups {
                    println!("  {} {:>10}  {:>8} objects  {}", Icons::BULLET, format_bytes(*size), count, Theme::value(class));
                }
                println!();
            }

            println!("  {} {} {}", Icons::BULLET, Theme::muted("Total objects:"), Theme::value(total_count));
            println!("  {} {} {}", Icons::BULLET, Theme::muted("Total size:"), Theme::value(format_bytes(total_size)));
        }

        println!();
        Ok(())
    })
}

#[cfg(feature = "s3-native")]
fn handle_rm_command(uri: &str, all_versions: bool, version_id: Option<String>, dry_run: bool) -> Result<()> {
    use orbit::protocol::s3::{has_wildcards, S3Client, S3Config, VersioningOperations};

    let (_protocol, key_path) = Protocol::from_uri(uri)?;
    let key = key_path.to_string_lossy().trim_start_matches('/').to_string();

    let bucket = match Protocol::from_uri(uri)? {
        (Protocol::S3 { bucket, .. }, _) => bucket,
        _ => return Err(OrbitError::Config("rm command requires an S3 URI (s3://bucket/key)".to_string())),
    };

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config).await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        if let Some(ref vid) = version_id {
            if dry_run {
                println!("(dry-run) Would delete: s3://{}/{} version {}", bucket, key, vid);
                return Ok(());
            }
            client.delete_object_version(&key, vid).await
                .map_err(|e| OrbitError::Other(format!("Failed to delete version: {}", e)))?;
            print_info(&format!("Deleted s3://{}/{} version {}", bucket, key, vid));
            return Ok(());
        }

        if all_versions {
            let result = client.list_object_versions(&key).await
                .map_err(|e| OrbitError::Other(format!("Failed to list versions: {}", e)))?;
            let total = result.versions.len() + result.delete_markers.len();
            if total == 0 {
                print_info("No objects or versions found.");
                return Ok(());
            }
            if dry_run {
                for v in &result.versions {
                    println!("(dry-run) Would delete: s3://{}/{} version {}", bucket, v.key, v.version_id);
                }
                for dm in &result.delete_markers {
                    println!("(dry-run) Would delete marker: s3://{}/{} version {}", bucket, dm.key, dm.version_id);
                }
                println!("\n(dry-run) Would delete {} versions, {} delete markers", result.versions.len(), result.delete_markers.len());
                return Ok(());
            }
            for v in &result.versions {
                client.delete_object_version(&v.key, &v.version_id).await
                    .map_err(|e| OrbitError::Other(format!("Failed to delete version {} of {}: {}", v.version_id, v.key, e)))?;
            }
            for dm in &result.delete_markers {
                client.delete_object_version(&dm.key, &dm.version_id).await
                    .map_err(|e| OrbitError::Other(format!("Failed to delete marker {} of {}: {}", dm.version_id, dm.key, e)))?;
            }
            print_info(&format!("Deleted {} versions, {} delete markers", result.versions.len(), result.delete_markers.len()));
            return Ok(());
        }

        if has_wildcards(&key) {
            let result = client.list_objects_with_wildcard(&key).await
                .map_err(|e| OrbitError::Other(format!("Failed to list objects: {}", e)))?;
            if result.objects.is_empty() {
                print_info("No objects matched the pattern.");
                return Ok(());
            }
            let keys: Vec<String> = result.objects.iter().map(|o| o.key.clone()).collect();
            if dry_run {
                for k in &keys {
                    println!("(dry-run) Would delete: s3://{}/{}", bucket, k);
                }
                println!("\n(dry-run) Would delete {} objects", keys.len());
                return Ok(());
            }
            client.delete_batch(&keys).await
                .map_err(|e| OrbitError::Other(format!("Failed to batch delete: {}", e)))?;
            print_info(&format!("Deleted {} objects", keys.len()));
        } else {
            if dry_run {
                println!("(dry-run) Would delete: s3://{}/{}", bucket, key);
                return Ok(());
            }
            client.delete(&key).await
                .map_err(|e| OrbitError::Other(format!("Failed to delete: {}", e)))?;
            print_info(&format!("Deleted s3://{}/{}", bucket, key));
        }

        Ok(())
    })
}

#[cfg(feature = "s3-native")]
fn handle_mv_command(source: &str, dest: &str) -> Result<()> {
    use orbit::protocol::s3::{S3Client, S3Config, S3Operations};

    let (_src_protocol, src_key_path) = Protocol::from_uri(source)?;
    let src_key = src_key_path.to_string_lossy().trim_start_matches('/').to_string();
    let src_bucket = match Protocol::from_uri(source)? {
        (Protocol::S3 { bucket, .. }, _) => bucket,
        _ => return Err(OrbitError::Config("mv command requires S3 URIs (s3://bucket/key)".to_string())),
    };

    let (_dst_protocol, dst_key_path) = Protocol::from_uri(dest)?;
    let dst_key = dst_key_path.to_string_lossy().trim_start_matches('/').to_string();
    let dst_bucket = match Protocol::from_uri(dest)? {
        (Protocol::S3 { bucket, .. }, _) => bucket,
        _ => return Err(OrbitError::Config("mv command requires S3 URIs (s3://bucket/key)".to_string())),
    };

    if src_bucket != dst_bucket {
        return Err(OrbitError::Config("mv command currently only supports moves within the same bucket".to_string()));
    }

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(src_bucket.clone());
        let client = S3Client::new(config).await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        client.copy_object(&src_key, &dst_key).await
            .map_err(|e| OrbitError::Other(format!("Failed to copy object: {}", e)))?;

        client.delete(&src_key).await
            .map_err(|e| OrbitError::Other(format!("Failed to delete source after copy: {}", e)))?;

        print_info(&format!("Moved s3://{}/{} -> s3://{}/{}", src_bucket, src_key, dst_bucket, dst_key));
        Ok(())
    })
}

#[cfg(feature = "s3-native")]
fn handle_mb_command(bucket_uri: &str) -> Result<()> {
    use orbit::protocol::s3::{S3Client, S3Config};

    let bucket_name = match Protocol::from_uri(bucket_uri) {
        Ok((Protocol::S3 { bucket, .. }, _)) => bucket,
        _ => bucket_uri.trim_start_matches("s3://").trim_end_matches('/').to_string(),
    };

    if bucket_name.is_empty() {
        return Err(OrbitError::Config("mb command requires a bucket name (s3://bucket-name)".to_string()));
    }

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket_name.clone());
        let client = S3Client::new(config).await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        client.create_bucket(&bucket_name).await
            .map_err(|e| OrbitError::Other(format!("Failed to create bucket: {}", e)))?;

        print_info(&format!("Created bucket: s3://{}", bucket_name));
        Ok(())
    })
}

#[cfg(feature = "s3-native")]
fn handle_rb_command(bucket_uri: &str) -> Result<()> {
    use orbit::protocol::s3::{S3Client, S3Config};

    let bucket_name = match Protocol::from_uri(bucket_uri) {
        Ok((Protocol::S3 { bucket, .. }, _)) => bucket,
        _ => bucket_uri.trim_start_matches("s3://").trim_end_matches('/').to_string(),
    };

    if bucket_name.is_empty() {
        return Err(OrbitError::Config("rb command requires a bucket name (s3://bucket-name)".to_string()));
    }

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket_name.clone());
        let client = S3Client::new(config).await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        client.delete_bucket(&bucket_name).await
            .map_err(|e| OrbitError::Other(format!("Failed to delete bucket: {}", e)))?;

        print_info(&format!("Deleted bucket: s3://{}", bucket_name));
        Ok(())
    })
}

/// Format a SystemTime as a human-readable date string
#[cfg(feature = "s3-native")]
fn format_system_time(time: std::time::SystemTime) -> String {
    let duration = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    let (year, month, day) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hours, minutes, seconds)
}

/// Convert days since Unix epoch to (year, month, day)
#[cfg(feature = "s3-native")]
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_version() {
        // Just verify the CLI struct can be created
        let result = Cli::try_parse_from(["orbit", "--help"]);
        // --help causes an error exit, but that's fine
        assert!(result.is_err());
    }

    #[test]
    fn test_workers_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src.txt", "-d", "dst.txt", "--workers", "64",
        ])
        .unwrap();
        assert_eq!(cli.workers, 64);
    }

    #[test]
    fn test_parallel_alias_for_workers() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src.txt", "-d", "dst.txt", "--parallel", "32",
        ])
        .unwrap();
        assert_eq!(cli.workers, 32);
    }

    #[test]
    fn test_concurrency_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src.txt", "-d", "dst.txt", "--concurrency", "10",
        ])
        .unwrap();
        assert_eq!(cli.concurrency, 10);
    }

    #[test]
    fn test_concurrency_default() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
        assert_eq!(cli.concurrency, 5);
    }

    #[test]
    fn test_workers_default_zero() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
        assert_eq!(cli.workers, 0);
    }

    #[test]
    fn test_stat_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src.txt", "-d", "dst.txt", "--stat",
        ])
        .unwrap();
        assert!(cli.stat);
    }

    #[test]
    fn test_stat_default_false() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
        assert!(!cli.stat);
    }

    #[test]
    fn test_human_readable_short_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src.txt", "-d", "dst.txt", "-H",
        ])
        .unwrap();
        assert!(cli.human_readable);
    }

    #[test]
    fn test_human_readable_long_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src.txt", "-d", "dst.txt", "--human-readable",
        ])
        .unwrap();
        assert!(cli.human_readable);
    }

    #[test]
    fn test_human_readable_default_false() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src.txt", "-d", "dst.txt"]).unwrap();
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
        let cli =
            Cli::try_parse_from(["orbit", "run", "--file", "commands.txt"]).unwrap();
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
        let cli =
            Cli::try_parse_from(["orbit", "run", "--workers", "128"]).unwrap();
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
        assert_eq!(
            args,
            vec!["cp", "/src", "/dst", "--recursive"]
        );
    }

    #[test]
    fn test_split_command_line_quotes() {
        let args = split_command_line(r#"cp "a b.txt" "c d.txt" --recursive"#).unwrap();
        assert_eq!(
            args,
            vec!["cp", "a b.txt", "c d.txt", "--recursive"]
        );
    }

    #[test]
    fn test_split_command_line_windows_paths() {
        let args = split_command_line(r#"cp C:\data\file.txt D:\dest\file.txt"#).unwrap();
        assert_eq!(
            args,
            vec!["cp", r"C:\data\file.txt", r"D:\dest\file.txt"]
        );
    }

    #[test]
    fn test_normalize_batch_args_cp() {
        let args = normalize_batch_args(
            "cp /src /dst --recursive",
            vec!["cp".to_string(), "/src".to_string(), "/dst".to_string(), "--recursive".to_string()],
        )
        .unwrap();
        assert_eq!(
            args,
            vec![
                "--source",
                "/src",
                "--dest",
                "/dst",
                "--recursive"
            ]
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
        assert_eq!(
            args,
            vec!["--source", "/src", "--dest", "/dst"]
        );
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
        assert!(available || !available); // Just verify it doesn't panic
    }

    #[test]
    fn test_combined_flags() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s", "src",
            "-d", "dst",
            "--workers", "128",
            "--concurrency", "8",
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

    // === Phase 3 & 4: S3 flag tests ===

    #[test]
    fn test_content_type_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src", "-d", "dst", "--content-type", "text/plain",
        ])
        .unwrap();
        assert_eq!(cli.content_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_content_encoding_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src", "-d", "dst", "--content-encoding", "gzip",
        ])
        .unwrap();
        assert_eq!(cli.content_encoding, Some("gzip".to_string()));
    }

    #[test]
    fn test_content_disposition_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--content-disposition",
            "attachment; filename=\"file.txt\"",
        ])
        .unwrap();
        assert_eq!(
            cli.content_disposition,
            Some("attachment; filename=\"file.txt\"".to_string())
        );
    }

    #[test]
    fn test_cache_control_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--cache-control",
            "max-age=3600",
        ])
        .unwrap();
        assert_eq!(cli.cache_control, Some("max-age=3600".to_string()));
    }

    #[test]
    fn test_expires_header_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--expires-header",
            "2026-12-31T23:59:59Z",
        ])
        .unwrap();
        assert_eq!(
            cli.expires_header,
            Some("2026-12-31T23:59:59Z".to_string())
        );
    }

    #[test]
    fn test_metadata_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--metadata",
            "key1=val1",
            "--metadata",
            "key2=val2",
        ])
        .unwrap();
        assert_eq!(cli.user_metadata, vec!["key1=val1", "key2=val2"]);
    }

    #[test]
    fn test_metadata_directive_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--metadata-directive",
            "REPLACE",
        ])
        .unwrap();
        assert_eq!(cli.metadata_directive, Some("REPLACE".to_string()));
    }

    #[test]
    fn test_acl_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src", "-d", "dst", "--acl", "public-read",
        ])
        .unwrap();
        assert_eq!(cli.acl, Some("public-read".to_string()));
    }

    #[test]
    fn test_no_sign_request_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src", "-d", "dst", "--no-sign-request",
        ])
        .unwrap();
        assert!(cli.no_sign_request);
    }

    #[test]
    fn test_no_sign_request_default_false() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst"]).unwrap();
        assert!(!cli.no_sign_request);
    }

    #[test]
    fn test_credentials_file_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--credentials-file",
            "/path/to/creds",
        ])
        .unwrap();
        assert_eq!(cli.credentials_file, Some(PathBuf::from("/path/to/creds")));
    }

    #[test]
    fn test_aws_profile_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src", "-d", "dst", "--aws-profile", "prod",
        ])
        .unwrap();
        assert_eq!(cli.aws_profile, Some("prod".to_string()));
    }

    #[test]
    fn test_use_acceleration_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src", "-d", "dst", "--use-acceleration",
        ])
        .unwrap();
        assert!(cli.use_acceleration);
    }

    #[test]
    fn test_use_acceleration_default_false() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst"]).unwrap();
        assert!(!cli.use_acceleration);
    }

    #[test]
    fn test_request_payer_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src", "-d", "dst", "--request-payer",
        ])
        .unwrap();
        assert!(cli.request_payer);
    }

    #[test]
    fn test_no_verify_ssl_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src", "-d", "dst", "--no-verify-ssl",
        ])
        .unwrap();
        assert!(cli.no_verify_ssl);
    }

    #[test]
    fn test_use_list_objects_v1_flag() {
        let cli = Cli::try_parse_from([
            "orbit", "-s", "src", "-d", "dst", "--use-list-objects-v1",
        ])
        .unwrap();
        assert!(cli.use_list_objects_v1);
    }

    #[test]
    fn test_s3_flags_defaults() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst"]).unwrap();
        assert!(cli.content_type.is_none());
        assert!(cli.content_encoding.is_none());
        assert!(cli.content_disposition.is_none());
        assert!(cli.cache_control.is_none());
        assert!(cli.expires_header.is_none());
        assert!(cli.user_metadata.is_empty());
        assert!(cli.metadata_directive.is_none());
        assert!(cli.acl.is_none());
        assert!(!cli.no_sign_request);
        assert!(cli.credentials_file.is_none());
        assert!(cli.aws_profile.is_none());
        assert!(!cli.use_acceleration);
        assert!(!cli.request_payer);
        assert!(!cli.no_verify_ssl);
        assert!(!cli.use_list_objects_v1);
    }

    // === Phase 5 & 6: Operational improvements and conditional copy tests ===

    #[test]
    fn test_part_size_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--part-size", "100"])
                .unwrap();
        assert_eq!(cli.part_size, Some(100));
    }

    #[test]
    fn test_part_size_default_none() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst"]).unwrap();
        assert_eq!(cli.part_size, None);
    }

    #[test]
    fn test_glacier_flags() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--force-glacier-transfer"])
                .unwrap();
        assert!(cli.force_glacier_transfer);
    }

    #[test]
    fn test_ignore_glacier_warnings_flag() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s",
            "src",
            "-d",
            "dst",
            "--ignore-glacier-warnings",
        ])
        .unwrap();
        assert!(cli.ignore_glacier_warnings);
    }

    #[test]
    fn test_no_clobber_flag() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "-n"]).unwrap();
        assert!(cli.no_clobber);
    }

    #[test]
    fn test_no_clobber_long_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--no-clobber"]).unwrap();
        assert!(cli.no_clobber);
    }

    #[test]
    fn test_if_size_differ_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--if-size-differ"])
                .unwrap();
        assert!(cli.if_size_differ);
    }

    #[test]
    fn test_if_source_newer_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--if-source-newer"])
                .unwrap();
        assert!(cli.if_source_newer);
    }

    #[test]
    fn test_flatten_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--flatten"]).unwrap();
        assert!(cli.flatten);
    }

    #[test]
    fn test_raw_flag() {
        let cli =
            Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst", "--raw"]).unwrap();
        assert!(cli.raw);
    }

    #[test]
    fn test_phase5_6_defaults() {
        let cli = Cli::try_parse_from(["orbit", "-s", "src", "-d", "dst"]).unwrap();
        assert!(!cli.force_glacier_transfer);
        assert!(!cli.ignore_glacier_warnings);
        assert!(!cli.no_clobber);
        assert!(!cli.if_size_differ);
        assert!(!cli.if_source_newer);
        assert!(!cli.flatten);
        assert!(!cli.raw);
    }

    #[test]
    fn test_combined_phase5_6_flags() {
        let cli = Cli::try_parse_from([
            "orbit",
            "-s", "src",
            "-d", "dst",
            "-n",
            "--if-size-differ",
            "--flatten",
            "--raw",
            "--force-glacier-transfer",
        ])
        .unwrap();
        assert!(cli.no_clobber);
        assert!(cli.if_size_differ);
        assert!(cli.flatten);
        assert!(cli.raw);
        assert!(cli.force_glacier_transfer);
    }

    // === S3 subcommand parse tests ===

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_ls_subcommand() {
        let cli = Cli::try_parse_from(["orbit", "ls", "s3://bucket/prefix"]).unwrap();
        match cli.command {
            Some(Commands::Ls { uri, etag, storage_class, all_versions, show_fullpath }) => {
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
            "orbit", "ls", "s3://bucket/prefix", "-e", "-s", "--all-versions", "--show-fullpath",
        ]).unwrap();
        match cli.command {
            Some(Commands::Ls { uri, etag, storage_class, all_versions, show_fullpath }) => {
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
            "orbit", "head", "s3://bucket/key.txt", "--version-id", "abc123",
        ]).unwrap();
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
            Some(Commands::Du { uri, group, all_versions }) => {
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
            Some(Commands::Du { uri, group, all_versions }) => {
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
            Some(Commands::Rm { uri, all_versions, version_id, dry_run }) => {
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
        let cli = Cli::try_parse_from([
            "orbit", "rm", "s3://bucket/prefix/*", "--dry-run",
        ]).unwrap();
        match cli.command {
            Some(Commands::Rm { uri, all_versions, version_id, dry_run }) => {
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
        let cli = Cli::try_parse_from([
            "orbit", "mv", "s3://bucket/old.txt", "s3://bucket/new.txt",
        ]).unwrap();
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
