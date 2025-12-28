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
use crate::core::probe::Probe;
use anyhow::Result;
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
            .interact()?
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
        .interact()?;

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

    // 5. Security Bootstrap (Control Plane)
    println!("\n{}", style("Security Configuration").cyan().bold());
    let generate_secret = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Generate secure JWT secret for Web Dashboard?")
        .default(true)
        .interact()?;

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

    // 6. Persistence
    let config_dir = config_path.parent().unwrap();
    fs::create_dir_all(config_dir)?;
    config
        .to_file(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to save configuration: {}", e))?;

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
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
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
    }

    // Adjust chunk size based on available memory
    if profile.total_memory_gb < 2 {
        config.chunk_size = 512 * 1024; // 512 KB for low memory
    } else if profile.total_memory_gb >= 8 {
        config.chunk_size = 4 * 1024 * 1024; // 4 MB for high memory
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
