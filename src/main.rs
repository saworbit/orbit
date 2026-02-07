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
    error::{OrbitError, Result},
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

    /// Number of parallel file operations (0 = auto)
    #[arg(long, default_value = "0", global = true)]
    parallel: usize,

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

fn main() -> Result<()> {
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
    config.parallel = cli.parallel;
    config.include_patterns = cli.include_patterns;
    config.exclude_patterns = cli.exclude_patterns;
    config.filter_from = cli.filter_from;
    config.dry_run = cli.dry_run;
    config.error_mode = cli.error_mode.into();
    config.log_level = cli.log_level.into();
    config.log_file = cli.log;
    config.verbose = cli.verbose;

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
    std::process::exit(1);
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
            std::process::exit(1);
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
        std::process::exit(1);
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
        std::process::exit(1);
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
        std::process::exit(1);
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
    std::process::exit(1);
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
