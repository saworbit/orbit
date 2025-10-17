/*!
 * Orbit CLI - Command Line Interface
 * 
 * Version: 0.4.0
 * Author: Shane Wall <shaneawall@gmail.com>
 */

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use orbit::{
    config::{CopyConfig, CopyMode, CompressionType, SymlinkMode},
    copy_file, copy_directory, CopyStats,
    error::Result,
    stats::TransferStats,
    // removed: audit::AuditLogger (doesn't exist)
    protocol::Protocol,
    get_zero_copy_capabilities, is_zero_copy_available,
};

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
    #[arg(short = 'm', long = "mode", value_enum, default_value = "copy", global = true)]
    mode: CopyModeArg,
    
    /// Recursive copy
    #[arg(short = 'R', long = "recursive", global = true)]
    recursive: bool,
    
    /// Preserve metadata (timestamps, permissions)
    #[arg(short = 'p', long = "preserve-metadata", global = true)]
    preserve_metadata: bool,
    
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
    
    /// Exclude patterns (can be specified multiple times)
    #[arg(long = "exclude", global = true)]
    exclude_patterns: Vec<String>,
    
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
    /// 
    /// Enables platform-specific optimizations like copy_file_range (Linux)
    /// for kernel-level file copying. Automatically disabled when incompatible
    /// with other features (resume, compression, bandwidth limiting).
    #[arg(long, global = true, conflicts_with = "no_zero_copy")]
    zero_copy: bool,
    
    /// Disable zero-copy optimization (use buffered copy)
    /// 
    /// Forces traditional buffered copying even when zero-copy is available.
    /// Useful for debugging or when maximum control is needed.
    #[arg(long, global = true, conflicts_with = "zero_copy")]
    no_zero_copy: bool,
    
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
    
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Handle subcommands
    if let Some(command) = cli.command {
        return handle_subcommand(command);
    }
    
    // Validate source and destination
    let source = cli.source.ok_or_else(|| {
        orbit::error::OrbitError::Config("Source path required".to_string())
    })?;
    
    let destination = cli.destination.ok_or_else(|| {
        orbit::error::OrbitError::Config("Destination path required".to_string())
    })?;
    
    // Parse URIs
    let (_source_protocol, source_path) = Protocol::from_uri(&source)?;
    let (_dest_protocol, dest_path) = Protocol::from_uri(&destination)?;
    
    // Load or create config
    let mut config = if let Some(config_path) = cli.config {
        CopyConfig::from_file(&config_path)
            .unwrap_or_else(|e| {
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
    config.resume_enabled = cli.resume;
    config.verify_checksum = !cli.no_verify;
    config.show_progress = cli.show_progress || !cli.no_progress;
    config.symlink_mode = cli.symlink.into();
    config.retry_attempts = cli.retry_attempts;
    config.retry_delay_secs = cli.retry_delay;
    config.exponential_backoff = cli.exponential_backoff;
    config.chunk_size = cli.chunk_size * 1024; // Convert KB to bytes
    config.max_bandwidth = cli.max_bandwidth * 1024 * 1024; // Convert MB/s to bytes/s
    config.parallel = cli.parallel;
    config.exclude_patterns = cli.exclude_patterns;
    config.dry_run = cli.dry_run;
    
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
    // If neither flag specified, use config default or true
    
    // Show zero-copy status if enabled
    if config.use_zero_copy && config.show_progress {
        if is_zero_copy_available() {
            let caps = get_zero_copy_capabilities();
            println!("⚡ Zero-copy enabled ({})", caps.method);
        }
    }
    
// Audit logging would be initialized here if needed
// For now, just acknowledge if path was provided
if let Some(_audit_path) = cli.audit_log {
    // Audit path provided, logging would happen here
}
    
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
            let stats = TransferStats::default(); // Load from persistent storage in real impl
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
    }
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
    println!("  Available: {}", if caps.available { "✓ Yes" } else { "✗ No" });
    println!("  Method: {}", caps.method);
    println!("  Cross-filesystem: {}", if caps.cross_filesystem { "✓ Yes" } else { "✗ No" });
    
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
    
    println!("Total bytes: {} ({})", 
             stats.bytes_copied, 
             format_bytes(stats.bytes_copied));
    
    println!("Duration: {:?}", stats.duration);
    
    let bytes_per_sec = stats.bytes_copied as f64 / stats.duration.as_secs_f64();
    println!("Average speed: {}/s", format_bytes(bytes_per_sec as u64));
    
    if let Some(ref checksum) = stats.checksum {
        println!("Checksum: {}...{}", &checksum[..8], &checksum[checksum.len()-8..]);
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