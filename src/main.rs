/*!
 * Orbit CLI - Command Line Interface
 * 
 * Version: 0.3.0
 * Author: Shane Wall <shaneawall@gmail.com>
 */

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use orbit::{
    config::{CopyConfig, CompressionType, SymlinkMode, CopyMode, AuditFormat, ConfigFile},
    core::{copy_file, copy_directory},
    audit,
    VERSION,
};

#[derive(Parser)]
#[command(name = "orbit")]
#[command(version = VERSION)]
#[command(about = "Open Resilient Bulk Information Transfer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Source file or directory
    #[arg(short, long, value_name = "PATH", global = true)]
    source: Option<PathBuf>,
    
    /// Destination file or directory
    #[arg(short, long, value_name = "PATH", global = true)]
    destination: Option<PathBuf>,
    
    /// Compression type: none, lz4, zstd[:level]
    #[arg(short, long, value_name = "TYPE", default_value = "none", global = true)]
    compress: String,
    
    /// Enable resume functionality
    #[arg(short, long, global = true)]
    resume: bool,
    
    /// Copy directories recursively
    #[arg(short = 'R', long, global = true)]
    recursive: bool,
    
    /// Preserve file metadata (timestamps, permissions)
    #[arg(short, long, global = true)]
    preserve_metadata: bool,
    
    /// How to handle symbolic links
    #[arg(short = 'L', long, value_enum, default_value = "preserve", global = true)]
    symlinks: SymlinkModeArg,
    
    /// Copy mode
    #[arg(short = 'm', long, value_enum, default_value = "copy", global = true)]
    mode: CopyModeArg,
    
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
    
    /// Path to config file (overrides default locations)
    #[arg(long, global = true)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show transfer statistics from audit log
    Stats {
        /// Path to audit log file (default: orbit_audit.log)
        #[arg(short, long)]
        log: Option<PathBuf>,
        
        /// Audit log format
        #[arg(short, long, value_enum, default_value = "json")]
        format: AuditFormatArg,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum SymlinkModeArg {
    Preserve,
    Follow,
    Skip,
}

impl From<SymlinkModeArg> for SymlinkMode {
    fn from(arg: SymlinkModeArg) -> Self {
        match arg {
            SymlinkModeArg::Preserve => SymlinkMode::Preserve,
            SymlinkModeArg::Follow => SymlinkMode::Follow,
            SymlinkModeArg::Skip => SymlinkMode::Skip,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CopyModeArg {
    Copy,
    Sync,
    Update,
    Mirror,
}

impl From<CopyModeArg> for CopyMode {
    fn from(arg: CopyModeArg) -> Self {
        match arg {
            CopyModeArg::Copy => CopyMode::Copy,
            CopyModeArg::Sync => CopyMode::Sync,
            CopyModeArg::Update => CopyMode::Update,
            CopyModeArg::Mirror => CopyMode::Mirror,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum AuditFormatArg {
    Json,
    Csv,
}

impl From<AuditFormatArg> for AuditFormat {
    fn from(arg: AuditFormatArg) -> Self {
        match arg {
            AuditFormatArg::Json => AuditFormat::Json,
            AuditFormatArg::Csv => AuditFormat::Csv,
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> orbit::error::Result<()> {
    let cli = Cli::parse();
    
    // Handle subcommands first
    if let Some(command) = cli.command {
        match command {
            Commands::Stats { log, format } => {
                let log_path = log.unwrap_or_else(|| PathBuf::from("orbit_audit.log"));
                let audit_format = format.into();
                
                if !log_path.exists() {
                    eprintln!("❌ Audit log not found: {:?}", log_path);
                    eprintln!("💡 Tip: Specify a different log file with --log <path>");
                    process::exit(1);
                }
                
                use orbit::stats::TransferStats;
                let stats = TransferStats::from_audit_log(&log_path, audit_format)?;
                
                if stats.total_operations == 0 {
                    println!("📊 No transfer operations found in audit log.");
                    println!("💡 Tip: Run some transfers first, then check stats again!");
                    return Ok(());
                }
                
                stats.print();
                return Ok(());
            }
        }
    }
    
    // Require source and destination for copy operations
    let source = cli.source.ok_or_else(|| {
        orbit::error::OrbitError::Config(
            "Source path is required. Use -s or --source".to_string()
        )
    })?;
    
    let destination = cli.destination.ok_or_else(|| {
        orbit::error::OrbitError::Config(
            "Destination path is required. Use -d or --destination".to_string()
        )
    })?;
    
    // Load configuration file
    let config_file = if let Some(ref config_path) = cli.config {
        ConfigFile::load(config_path)?
    } else {
        ConfigFile::load_with_fallback()
    };
    
    // Build configuration (CLI args override config file)
    let mut config = CopyConfig::default();
    
    // Apply config file first
    config_file.apply_to(&mut config);
    
    // Then apply CLI arguments (which take precedence)
    config.compression = CompressionType::from_str(&cli.compress)?;
    config.resume_enabled = cli.resume;
    config.recursive = cli.recursive;
    config.preserve_metadata = cli.preserve_metadata;
    config.symlink_mode = cli.symlinks.into();
    config.copy_mode = cli.mode.into();
    config.retry_attempts = cli.retry_attempts;
    config.retry_delay_secs = cli.retry_delay;
    config.exponential_backoff = cli.exponential_backoff;
    config.chunk_size = cli.chunk_size * 1024; // Convert KB to bytes
    config.max_bandwidth = cli.max_bandwidth * 1024 * 1024; // Convert MB/s to bytes/s
    config.parallel = cli.parallel;
    config.dry_run = cli.dry_run;
    config.audit_format = cli.audit_format.into();
    config.audit_log_path = cli.audit_log;
    config.show_progress = !cli.no_progress;
    config.verify_checksum = !cli.no_verify;
    
    // Add CLI exclude patterns to config
    config.exclude_patterns.extend(cli.exclude_patterns);
    
    // Validate source exists
    if !source.exists() {
        eprintln!("Error: Source path does not exist: {:?}", source);
        process::exit(1);
    }
    
    // Determine if source is file or directory
    let source_metadata = std::fs::metadata(&source)?;
    
    if source_metadata.is_dir() {
        if !config.recursive {
            eprintln!("Error: Source is a directory but --recursive flag not specified.");
            eprintln!("Use -R or --recursive to copy directories.");
            process::exit(1);
        }
        
        println!("📂 Copying directory tree: {:?} -> {:?}", source, destination);
        
        let stats = copy_directory(&source, &destination, &config)?;
        
        println!("\n✅ Directory copy completed successfully!");
        println!("   Files copied: {}", stats.files_copied);
        println!("   Files skipped: {}", stats.files_skipped);
        println!("   Total bytes: {} ({:.2} MB)", stats.bytes_copied, stats.bytes_copied as f64 / 1_048_576.0);
        println!("   Duration: {:?}", stats.duration);
        
    } else {
        println!("📄 Copying file: {:?} -> {:?}", source, destination);
        
        let stats = copy_file(&source, &destination, &config)?;
        
        // Write audit log
        audit::write_audit_log(
            &source,
            &destination,
            &stats,
            "success",
            1,
            None,
            config.audit_format,
            config.audit_log_path.as_deref(),
        )?;
        
        println!("\n✅ File copied successfully!");
        println!("   Bytes copied: {} ({:.2} MB)", stats.bytes_copied, stats.bytes_copied as f64 / 1_048_576.0);
        println!("   Duration: {:?}", stats.duration);
        
        if let Some(checksum) = stats.checksum {
            println!("   Checksum: {}", checksum);
        }
        
        if let Some(ratio) = stats.compression_ratio {
            println!("   Compression ratio: {:.1}%", ratio);
        }
    }
    
    Ok(())
}