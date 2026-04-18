/*!
 * Orbit Init Command - First-Run Onboarding Wizard
 *
 * This module provides an interactive setup wizard that:
 * 1. Scans the system environment
 * 2. Interviews the user about their use case
 * 3. Generates an optimized configuration
 * 4. Securely generates JWT secrets for the control plane
 * 5. Persists configuration to ~/.orbit/orbit.toml
 *
 * Version: 0.7.0
 * Phase: 5 - First-Run Onboarding
 */

use crate::config::{CompressionType, CopyConfig, CopyMode};
use crate::core::{probe::Probe, sparse::SparseMode};
use crate::error::{OrbitError, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use std::fs;
use std::path::{Path, PathBuf};

/// Run the interactive initialization wizard
pub fn run_init_wizard() -> Result<()> {
    print_welcome();

    // 1. Check for existing configuration
    let config_path = get_default_config_path()?;
    if config_path.exists()
        && !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Existing configuration found. Overwrite?")
            .default(false)
            .interact()
            .map_err(|e| OrbitError::Other(format!("Input error: {}", e)))?
    {
        println!("\n{}", style("Configuration unchanged.").cyan());
        return Ok(());
    }

    // 2. System Scan (silent)
    println!("\n{}", style("Scanning system environment...").cyan());
    let profile = match Probe::scan(&std::env::current_dir().unwrap_or_default()) {
        Ok(p) => {
            println!(
                "  {} CPU cores detected",
                style(p.logical_cores).green().bold()
            );
            println!(
                "  {} GB RAM available",
                style(p.available_ram_gb).green().bold()
            );
            println!(
                "  I/O throughput: ~{} MB/s",
                style(format!("{:.0}", p.estimated_io_throughput))
                    .green()
                    .bold()
            );
            Some(p)
        }
        Err(e) => {
            println!(
                "  {} Failed to probe environment: {}",
                style("Warning:").yellow(),
                e
            );
            None
        }
    };

    // 3. User Interview
    println!("\n{}", style("Configuration Setup").cyan().bold());
    let use_cases = &[
        "Backup (Reliability First)",
        "Sync (Speed First)",
        "Cloud Upload (Compression First)",
        "Network Transfer (Resume + Compression)",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What is your primary use case?")
        .default(0)
        .items(use_cases)
        .interact()
        .map_err(|e| OrbitError::Other(format!("Input error: {}", e)))?;

    // 4. Configuration Synthesis
    let mut config = match selection {
        0 => create_backup_config(),
        1 => create_sync_config(),
        2 => create_cloud_config(),
        3 => create_network_config(),
        _ => CopyConfig::default(),
    };

    // Apply environment-specific optimizations if we have a profile
    if let Some(prof) = profile {
        apply_profile_optimizations(&mut config, &prof);
    }

    // 5. Common exclusion patterns
    println!("\n{}", style("Default Exclusions").cyan().bold());
    let exclusion_presets = &[
        "Development (.git, node_modules, target/, __pycache__)",
        "Temporary files (*.tmp, *.log, *.swp, ~*)",
        "OS files (.DS_Store, Thumbs.db, desktop.ini)",
        "All of the above",
        "None — I'll configure manually",
    ];

    let excl_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Auto-exclude common junk files?")
        .default(3)
        .items(exclusion_presets)
        .interact()
        .map_err(|e| OrbitError::Other(format!("Input error: {}", e)))?;

    match excl_selection {
        0 => {
            config.exclude_patterns = vec![
                ".git/**".to_string(),
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "__pycache__/**".to_string(),
                ".venv/**".to_string(),
            ];
        }
        1 => {
            config.exclude_patterns = vec![
                "*.tmp".to_string(),
                "*.log".to_string(),
                "*.swp".to_string(),
                "~*".to_string(),
                "*.bak".to_string(),
            ];
        }
        2 => {
            config.exclude_patterns = vec![
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                "desktop.ini".to_string(),
                "._*".to_string(),
            ];
        }
        3 => {
            config.exclude_patterns = vec![
                ".git/**".to_string(),
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "__pycache__/**".to_string(),
                ".venv/**".to_string(),
                "*.tmp".to_string(),
                "*.log".to_string(),
                "*.swp".to_string(),
                "~*".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                "desktop.ini".to_string(),
            ];
        }
        _ => {} // None — leave empty
    }

    if !config.exclude_patterns.is_empty() {
        println!(
            "  {} {} patterns configured",
            style("✓").green().bold(),
            config.exclude_patterns.len()
        );
    }

    // 6. Shell completions
    println!("\n{}", style("Shell Completions").cyan().bold());
    let install_completions = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Install tab-completion for your shell?")
        .default(true)
        .interact()
        .map_err(|e| OrbitError::Other(format!("Input error: {}", e)))?;

    if install_completions {
        install_shell_completions();
    }

    // 7. Security Bootstrap (Control Plane)
    println!("\n{}", style("Security Configuration").cyan().bold());
    let generate_secret = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Generate secure JWT secret for Web Dashboard?")
        .default(false)
        .interact()
        .map_err(|e| OrbitError::Other(format!("Input error: {}", e)))?;

    if generate_secret {
        let secret = generate_jwt_secret();
        println!("\n  {} Generated JWT Secret:", style("✓").green().bold());
        println!("  {}", style(&secret).yellow());
        println!("\n  Add this to your environment:");
        println!(
            "  {}",
            style(format!("export ORBIT_JWT_SECRET={}", secret)).cyan()
        );
        println!(
            "  {}",
            style("(This will NOT be saved to the config file for security)").dim()
        );
    }

    // 8. Persistence
    let config_dir = config_path.parent().unwrap();
    fs::create_dir_all(config_dir)?;

    // Mark first-run tip as shown since they ran init
    let tip_path = config_dir.join(".tip-shown");
    let _ = fs::write(&tip_path, "");

    config
        .to_file(&config_path)
        .map_err(|e| OrbitError::Config(format!("Failed to save configuration: {}", e)))?;

    print_summary(&config_path, &config);

    Ok(())
}

/// Print welcome banner
fn print_welcome() {
    println!();
    println!(
        "{}",
        style("╔════════════════════════════════════════╗").cyan()
    );
    println!(
        "{}",
        style("║    🪐 Welcome to Orbit Setup           ║").cyan()
    );
    println!(
        "{}",
        style("╚════════════════════════════════════════╝").cyan()
    );
    println!();
    println!("This wizard will scan your system and create an optimized configuration.");
}

/// Get the default configuration file path
fn get_default_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| OrbitError::Config("Could not determine home directory".to_string()))?;
    Ok(home.join(".orbit").join("orbit.toml"))
}

/// Create a backup-optimized configuration
fn create_backup_config() -> CopyConfig {
    CopyConfig {
        copy_mode: CopyMode::Copy,
        verify_checksum: true,
        preserve_metadata: true,
        retry_attempts: 5,
        exponential_backoff: true,
        resume_enabled: true,
        ..Default::default()
    }
}

/// Create a sync-optimized configuration
fn create_sync_config() -> CopyConfig {
    CopyConfig {
        copy_mode: CopyMode::Sync,
        verify_checksum: false, // Trust modtime for speed
        preserve_metadata: true,
        parallel: 0, // Auto-detect
        use_zero_copy: true,
        ..Default::default()
    }
}

/// Create a cloud-optimized configuration
fn create_cloud_config() -> CopyConfig {
    CopyConfig {
        copy_mode: CopyMode::Copy,
        compression: CompressionType::Zstd { level: 3 },
        sparse_mode: SparseMode::Never,
        verify_checksum: true,
        retry_attempts: 10,
        exponential_backoff: true,
        resume_enabled: true,
        use_zero_copy: false, // Not effective with compression
        ..Default::default()
    }
}

/// Create a network-optimized configuration
fn create_network_config() -> CopyConfig {
    CopyConfig {
        copy_mode: CopyMode::Copy,
        compression: CompressionType::Zstd { level: 3 },
        sparse_mode: SparseMode::Never,
        verify_checksum: true,
        resume_enabled: true,
        retry_attempts: 10,
        exponential_backoff: true,
        parallel: 4,
        ..Default::default()
    }
}

/// Apply profile-based optimizations to the configuration
fn apply_profile_optimizations(
    config: &mut CopyConfig,
    profile: &crate::core::probe::SystemProfile,
) {
    // Adjust parallelism based on CPU cores
    if config.parallel == 0 {
        config.parallel = (profile.logical_cores / 2).clamp(1, 16);
    }

    // If I/O is slow but we have CPU, suggest compression
    if profile.estimated_io_throughput < 100.0
        && profile.logical_cores >= 4
        && matches!(config.compression, CompressionType::None)
    {
        config.compression = CompressionType::Lz4; // Fast compression
        config.sparse_mode = SparseMode::Never;
    }

    // Adjust chunk size based on available memory
    if profile.total_memory_gb < 2 {
        config.chunk_size = 512 * 1024; // 512 KB for low memory
    } else if profile.total_memory_gb >= 8 {
        config.chunk_size = 4 * 1024 * 1024; // 4 MB for high memory
    }
}

/// Detect the user's shell and install completions automatically
fn install_shell_completions() {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let shell_name = if shell.contains("zsh") {
        "zsh"
    } else if shell.contains("bash") {
        "bash"
    } else if shell.contains("fish") {
        "fish"
    } else if cfg!(windows) {
        "powershell"
    } else {
        println!(
            "  {} {}",
            style("⚠").yellow(),
            style("Could not detect shell. Run 'orbit completions <shell>' manually.").dim()
        );
        return;
    };

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            println!(
                "  {} {}",
                style("⚠").yellow(),
                style("Could not find home directory.").dim()
            );
            return;
        }
    };

    let (target_path, instruction) = match shell_name {
        "bash" => {
            let dir = home.join(".bash_completion.d");
            let _ = fs::create_dir_all(&dir);
            (
                dir.join("orbit"),
                "Sourced automatically by bash-completion",
            )
        }
        "zsh" => {
            let dir = home.join(".zsh").join("completions");
            let _ = fs::create_dir_all(&dir);
            (
                dir.join("_orbit"),
                "Add 'fpath=(~/.zsh/completions $fpath)' to .zshrc if not already set",
            )
        }
        "fish" => {
            let dir = home.join(".config").join("fish").join("completions");
            let _ = fs::create_dir_all(&dir);
            (dir.join("orbit.fish"), "Auto-loaded by fish")
        }
        "powershell" => {
            let dir = home.join("Documents").join("PowerShell");
            let _ = fs::create_dir_all(&dir);
            (
                dir.join("orbit_completions.ps1"),
                "Add '. ~/Documents/PowerShell/orbit_completions.ps1' to $PROFILE",
            )
        }
        _ => return,
    };

    // Generate completions by invoking ourselves as a subprocess.
    // We can't access Cli::command() from the lib crate, so we shell out.
    match std::process::Command::new("orbit")
        .args(["completions", shell_name])
        .output()
    {
        Ok(output) if output.status.success() => match fs::write(&target_path, &output.stdout) {
            Ok(()) => {
                println!(
                    "  {} Completions installed to {}",
                    style("✓").green().bold(),
                    style(target_path.display()).cyan()
                );
                if !instruction.is_empty() {
                    println!("  {} {}", style("ℹ").blue(), style(instruction).dim());
                }
            }
            Err(e) => {
                println!(
                    "  {} Failed to write completions: {}",
                    style("⚠").yellow(),
                    e
                );
            }
        },
        _ => {
            println!(
                "  {} Run '{}' to generate completions manually.",
                style("ℹ").blue(),
                style(format!("orbit completions {}", shell_name)).cyan()
            );
        }
    }
}

/// Generate a cryptographically secure JWT secret
fn generate_jwt_secret() -> String {
    // Generate 32 random alphanumeric characters
    (0..32)
        .map(|_| {
            let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            let idx = (rand::random::<u8>() as usize) % chars.len();
            chars[idx] as char
        })
        .collect()
}

/// Print configuration summary
fn print_summary(config_path: &Path, config: &CopyConfig) {
    println!();
    println!(
        "{}",
        style("╔════════════════════════════════════════╗").green()
    );
    println!(
        "{}",
        style("║    ✅ Configuration Saved              ║").green()
    );
    println!(
        "{}",
        style("╚════════════════════════════════════════╝").green()
    );
    println!();
    println!("  Location: {}", style(config_path.display()).cyan());
    println!();
    println!("  {}", style("Configuration Summary:").bold());
    println!("  ─────────────────────────");
    println!(
        "  Copy Mode:        {}",
        style(format!("{:?}", config.copy_mode)).yellow()
    );
    println!(
        "  Compression:      {}",
        style(format!("{:?}", config.compression)).yellow()
    );
    println!(
        "  Checksum Verify:  {}",
        style(config.verify_checksum).yellow()
    );
    println!(
        "  Resume:           {}",
        style(config.resume_enabled).yellow()
    );
    println!("  Parallel:         {}", style(config.parallel).yellow());
    println!(
        "  Retry Attempts:   {}",
        style(config.retry_attempts).yellow()
    );
    println!();
    println!("  {}", style("Next Steps:").bold());
    println!(
        "  1. Review the configuration: cat {}",
        config_path.display()
    );
    println!("  2. Set ORBIT_JWT_SECRET environment variable (if you generated one)");
    println!("  3. Run 'orbit --help' to see available commands");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_config() {
        let config = create_backup_config();
        assert_eq!(config.copy_mode, CopyMode::Copy);
        assert!(config.verify_checksum);
        assert!(config.preserve_metadata);
        assert!(config.resume_enabled);
        assert_eq!(config.retry_attempts, 5);
    }

    #[test]
    fn test_sync_config() {
        let config = create_sync_config();
        assert_eq!(config.copy_mode, CopyMode::Sync);
        assert!(!config.verify_checksum); // Speed over verification
        assert!(config.use_zero_copy);
    }

    #[test]
    fn test_cloud_config() {
        let config = create_cloud_config();
        assert!(matches!(config.compression, CompressionType::Zstd { .. }));
        assert_eq!(config.retry_attempts, 10);
        assert!(config.exponential_backoff);
    }

    #[test]
    fn test_network_config() {
        let config = create_network_config();
        assert!(matches!(config.compression, CompressionType::Zstd { .. }));
        assert!(config.resume_enabled);
        assert_eq!(config.parallel, 4);
    }

    #[test]
    fn test_jwt_secret_generation() {
        let secret = generate_jwt_secret();
        assert_eq!(secret.len(), 32);
        assert!(secret.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_profile_optimizations() {
        use crate::core::probe::{FileSystemType, SystemProfile};

        let mut config = CopyConfig::default();
        let profile = SystemProfile {
            logical_cores: 8,
            available_ram_gb: 4,
            is_battery_power: false,
            dest_filesystem_type: FileSystemType::Local,
            estimated_io_throughput: 50.0, // Slow I/O
            total_memory_gb: 8,
        };

        apply_profile_optimizations(&mut config, &profile);

        // Should enable compression due to slow I/O + CPU availability
        assert!(matches!(config.compression, CompressionType::Lz4));
    }
}
