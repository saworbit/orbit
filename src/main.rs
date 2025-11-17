/*!
 * Orbit CLI - Command Line Interface
 *
 * Version: 0.4.0
 * Author: Shane Wall <shaneawall@gmail.com>
 */

use clap::{Parser, Subcommand, ValueEnum};
use orbit::{
    config::{
        AuditFormat, ChunkingStrategy, CompressionType, CopyConfig, CopyMode, ErrorMode, LogLevel,
        SymlinkMode,
    },
    copy_directory, copy_file,
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
    /// Check mode for change detection (modtime, size, checksum, delta)
    #[arg(long, value_enum, default_value = "modtime", global = true)]
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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check if we need to initialize logging for the command
    let needs_logging = if let Some(ref command) = cli.command {
        matches!(command, Commands::Manifest(_))
    } else {
        true // Main copy operation needs logging
    };

    // Initialize logging if needed
    if needs_logging {
        let mut config = if let Some(ref config_path) = cli.config {
            CopyConfig::from_file(config_path).unwrap_or_else(|e| {
                eprintln!("Warning: Failed to load config file: {}", e);
                CopyConfig::default()
            })
        } else {
            CopyConfig::default()
        };

        // Set logging config from CLI
        config.log_level = cli.log_level.into();
        config.log_file = cli.log.clone();
        config.verbose = cli.verbose;

        // Initialize logging
        if let Err(e) = logging::init_logging(&config) {
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

    // Load or create config
    let mut config = if let Some(config_path) = cli.config {
        CopyConfig::from_file(&config_path).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config file: {}", e);
            CopyConfig::default()
        })
    } else {
        CopyConfig::default()
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
    config.chunk_size = cli.chunk_size * 1024;
    config.max_bandwidth = cli.max_bandwidth * 1024 * 1024;
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
    if config.use_zero_copy && config.show_progress {
        if is_zero_copy_available() {
            let caps = get_zero_copy_capabilities();
            println!("⚡ Zero-copy enabled ({})", caps.method);
        }
    }

    // Show manifest status if enabled
    if config.generate_manifest && config.show_progress {
        if let Some(ref dir) = config.manifest_output_dir {
            println!("📋 Manifest generation enabled: {}", dir.display());
        }
    }

    // Configure audit logging
    config.audit_format = cli.audit_format.into();
    config.audit_log_path = cli.audit_log;

    // Configure delta detection
    config.check_mode = cli.check.into();
    config.delta_block_size = cli.block_size * 1024; // Convert KB to bytes
    config.whole_file = cli.whole_file;
    config.update_manifest = cli.update_manifest;
    config.ignore_existing = cli.ignore_existing;
    config.delta_manifest_path = cli.delta_manifest;

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

    runtime
        .block_on(orbit_web::start_server(addr))
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
    println!("📋 Creating flight plan...");
    println!("  Source: {}", source.display());
    println!("  Dest:   {}", dest.display());
    println!("  Output: {}", output.display());

    let chunking_strategy = match chunking.as_str() {
        "cdc" => ChunkingStrategy::Cdc {
            avg_kib: chunk_size,
            algo: "gear".to_string(),
        },
        "fixed" => ChunkingStrategy::Fixed {
            size_kib: chunk_size,
        },
        _ => {
            eprintln!(
                "❌ Invalid chunking strategy: {} (use 'cdc' or 'fixed')",
                chunking
            );
            std::process::exit(1);
        }
    };

    let mut config = CopyConfig::default();
    config.generate_manifest = true;
    config.manifest_output_dir = Some(output.clone());
    config.chunking_strategy = chunking_strategy;

    let mut generator = ManifestGenerator::new(&source, &dest, &config)?;

    if source.is_file() {
        let file_name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        println!("  Generating manifest for: {}", file_name);
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

                println!("  Generating manifest for: {}", relative_path);
                generator.generate_file_manifest(entry.path(), &relative_path)?;
            }
        }
    } else {
        eprintln!("❌ Source path does not exist or is not accessible");
        std::process::exit(1);
    }

    generator.finalize("sha256:pending")?;

    println!("✅ Flight plan created at: {}", output.display());

    Ok(())
}

fn handle_manifest_verify(manifest_dir: PathBuf) -> Result<()> {
    use orbit::manifests::{CargoManifest, FlightPlan};

    println!("🔍 Verifying manifests in: {}", manifest_dir.display());

    let flight_plan_path = manifest_dir.join("job.flightplan.json");
    if !flight_plan_path.exists() {
        eprintln!("❌ Flight plan not found: {}", flight_plan_path.display());
        std::process::exit(1);
    }

    let flight_plan = FlightPlan::load(&flight_plan_path)
        .map_err(|e| OrbitError::Other(format!("Failed to load flight plan: {}", e)))?;

    println!("  Job ID: {}", flight_plan.job_id);
    println!("  Files: {}", flight_plan.files.len());
    println!(
        "  Status: {}",
        if flight_plan.is_finalized() {
            "✅ Finalized"
        } else {
            "⏳ Pending"
        }
    );

    for file_ref in &flight_plan.files {
        let cargo_path = manifest_dir.join(&file_ref.cargo);

        if !cargo_path.exists() {
            println!("  ❌ {}: Cargo manifest missing", file_ref.path);
            continue;
        }

        match CargoManifest::load(&cargo_path) {
            Ok(cargo) => {
                println!(
                    "  ✅ {}: {} windows, {} bytes",
                    file_ref.path,
                    cargo.windows.len(),
                    cargo.size
                );
            }
            Err(e) => {
                println!("  ❌ {}: Invalid manifest - {}", file_ref.path, e);
            }
        }
    }

    println!("✅ Verification complete");

    Ok(())
}

fn handle_manifest_diff(manifest_dir: PathBuf, target: PathBuf) -> Result<()> {
    println!("📊 Comparing manifests with target...");
    println!("  Manifests: {}", manifest_dir.display());
    println!("  Target:    {}", target.display());

    println!("⚠️  Diff operation not yet fully implemented");
    println!("    This will compare manifest metadata with actual files");

    Ok(())
}

fn handle_manifest_info(path: PathBuf) -> Result<()> {
    use orbit::manifests::{CargoManifest, FlightPlan};

    if !path.exists() {
        eprintln!("❌ Path not found: {}", path.display());
        std::process::exit(1);
    }

    if let Ok(flight_plan) = FlightPlan::load(&path) {
        println!("📋 Flight Plan");
        println!("  Schema:  {}", flight_plan.schema);
        println!("  Job ID:  {}", flight_plan.job_id);
        println!("  Created: {}", flight_plan.created_utc);
        println!(
            "  Source:  {} ({})",
            flight_plan.source.root, flight_plan.source.endpoint_type
        );
        println!(
            "  Target:  {} ({})",
            flight_plan.target.root, flight_plan.target.endpoint_type
        );
        println!("  Files:   {}", flight_plan.files.len());

        println!("  Policy:");
        println!("    Encryption: {}", flight_plan.policy.encryption.aead);
        if let Some(classification) = &flight_plan.policy.classification {
            println!("    Classification: {}", classification);
        }

        return Ok(());
    }

    if let Ok(cargo) = CargoManifest::load(&path) {
        println!("📦 Cargo Manifest");
        println!("  Schema:  {}", cargo.schema);
        println!("  Path:    {}", cargo.path);
        println!("  Size:    {} bytes", cargo.size);
        println!("  Chunking: {}", cargo.chunking.chunking_type);
        println!("  Windows: {}", cargo.windows.len());
        println!("  Chunks:  {}", cargo.total_chunks());

        return Ok(());
    }

    eprintln!("❌ Not a valid flight plan or cargo manifest");
    std::process::exit(1);
}

fn print_presets() {
    println!("Available Configuration Presets:\n");

    println!("🚀 FAST (--preset fast)");
    println!("   - No checksum verification");
    println!("   - No resume capability");
    println!("   - No compression");
    println!("   - Zero-copy enabled");
    println!("   - Parallel operations: auto");
    println!("   Best for: Local copies on fast storage (NVMe, SSD)\n");

    println!("🛡️  SAFE (--preset safe)");
    println!("   - Checksum verification enabled");
    println!("   - Resume capability enabled");
    println!("   - 5 retry attempts with exponential backoff");
    println!("   - Zero-copy disabled (buffered for control)");
    println!("   Best for: Critical data, unreliable media\n");

    println!("🌐 NETWORK (--preset network)");
    println!("   - Checksum verification enabled");
    println!("   - Resume capability enabled");
    println!("   - Zstd compression (level 3)");
    println!("   - 10 retry attempts with exponential backoff");
    println!("   - Zero-copy disabled (compression needed)");
    println!("   Best for: Remote transfers, slow networks\n");
}

fn print_capabilities() {
    println!("Orbit Platform Capabilities\n");
    println!("═══════════════════════════════════════════════\n");

    let caps = get_zero_copy_capabilities();

    println!("Zero-Copy Support:");
    println!(
        "  Available: {}",
        if caps.available { "✓ Yes" } else { "✗ No" }
    );
    println!("  Method: {}", caps.method);
    println!(
        "  Cross-filesystem: {}",
        if caps.cross_filesystem {
            "✓ Yes"
        } else {
            "✗ No"
        }
    );

    println!("\nPlatform: {}", std::env::consts::OS);
    println!("Architecture: {}", std::env::consts::ARCH);

    println!("\nCompression Support:");
    println!("  LZ4: ✓ Yes");
    println!("  Zstd: ✓ Yes");

    println!("\nProtocol Support:");
    println!("  Local filesystem: ✓ Production");
    println!("  SMB/CIFS: ⚠ Experimental");
    println!("  S3: ⏳ Planned");
    println!("  Azure Blob: ⏳ Planned");
    println!("  Google Cloud Storage: ⏳ Planned");

    println!("\nManifest System:");
    println!("  Flight Plans: ✓ Yes");
    println!("  Cargo Manifests: ✓ Yes");
    println!("  Star Maps: ✓ Yes");
    println!("  Telemetry Logging: ✓ Yes");
    println!("  Verification: ✓ Yes");

    println!("\nPerformance Features:");
    println!("  Resume: ✓ Yes");
    println!("  Parallel operations: ✓ Yes");
    println!("  Bandwidth throttling: ✓ Yes");
    println!("  Progress tracking: ✓ Yes");
    println!("  Checksum verification: ✓ Yes (SHA-256)");

    println!("\n═══════════════════════════════════════════════");
}

fn print_summary(stats: &CopyStats) {
    println!("\n╔═══════════════════════════════════════════════╗");
    println!("║              Transfer Complete                ║");
    println!("╚═══════════════════════════════════════════════╝\n");

    println!("Files copied: {}", stats.files_copied);
    println!("Files skipped: {}", stats.files_skipped);

    if stats.files_failed > 0 {
        println!("Files failed: {}", stats.files_failed);
    }

    println!(
        "Total bytes: {} ({})",
        stats.bytes_copied,
        format_bytes(stats.bytes_copied)
    );

    println!("Duration: {:?}", stats.duration);

    let bytes_per_sec = stats.bytes_copied as f64 / stats.duration.as_secs_f64();
    println!("Average speed: {}/s", format_bytes(bytes_per_sec as u64));

    if let Some(ref checksum) = stats.checksum {
        println!(
            "Checksum: {}...{}",
            &checksum[..8],
            &checksum[checksum.len() - 8..]
        );
    }

    if let Some(ratio) = stats.compression_ratio {
        println!("Compression ratio: {:.1}%", ratio);
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f = bytes as f64;
    let base = 1024.0_f64;
    let exp = (bytes_f.ln() / base.ln()).floor() as usize;
    let exp = exp.min(UNITS.len() - 1);

    let value = bytes_f / base.powi(exp as i32);

    if exp == 0 {
        format!("{} {}", bytes, UNITS[exp])
    } else {
        format!("{:.2} {}", value, UNITS[exp])
    }
}
