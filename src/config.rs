/*!
 * Configuration structures and defaults for Orbit
 */

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::error::{OrbitError, Result};

/// How to handle symbolic links during copy operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymlinkMode {
    /// Copy the symbolic link itself (preserve as symlink)
    Preserve,
    /// Follow the link and copy the target file/directory
    Follow,
    /// Skip symbolic links entirely
    Skip,
}

impl Default for SymlinkMode {
    fn default() -> Self {
        Self::Preserve
    }
}

/// Copy operation modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopyMode {
    /// Always copy files
    Copy,
    /// Only copy files that don't exist or are newer
    Sync,
    /// Only copy newer files
    Update,
    /// Mirror source to destination (delete extra files)
    Mirror,
}

impl Default for CopyMode {
    fn default() -> Self {
        Self::Copy
    }
}

/// Compression algorithm and level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    /// No compression
    None,
    /// LZ4 compression (fast)
    Lz4,
    /// Zstd compression with level (1-22, higher = better compression but slower)
    Zstd { level: i32 },
}

impl Default for CompressionType {
    fn default() -> Self {
        Self::None
    }
}

impl CompressionType {
    /// Parse compression type from string (e.g., "zstd:3", "lz4", "none")
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "lz4" => Ok(Self::Lz4),
            s if s.starts_with("zstd:") => {
                let level_str = s.strip_prefix("zstd:").unwrap();
                let level = level_str.parse::<i32>()
                    .map_err(|_| OrbitError::Config(format!("Invalid zstd level: {}", level_str)))?;
                
                if !(1..=22).contains(&level) {
                    return Err(OrbitError::Config(
                        format!("Zstd level must be 1-22, got {}", level)
                    ));
                }
                
                Ok(Self::Zstd { level })
            }
            "zstd" => Ok(Self::Zstd { level: 3 }), // Default zstd level
            _ => Err(OrbitError::Config(format!("Unknown compression type: {}", s))),
        }
    }
}

/// Audit log format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditFormat {
    /// JSON Lines format (one JSON object per line)
    Json,
    /// CSV format
    Csv,
}

impl Default for AuditFormat {
    fn default() -> Self {
        Self::Json
    }
}

/// Main configuration for copy operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyConfig {
    /// Compression type to use
    pub compression: CompressionType,
    
    /// Whether resume functionality is enabled
    pub resume_enabled: bool,
    
    /// Size of chunks for buffered I/O operations (in bytes)
    pub chunk_size: usize,
    
    /// Whether to preserve file metadata (timestamps, permissions)
    pub preserve_metadata: bool,
    
    /// How to handle symbolic links
    pub symlink_mode: SymlinkMode,
    
    /// Whether to copy directories recursively
    pub recursive: bool,
    
    /// Copy mode (copy, sync, update, mirror)
    pub copy_mode: CopyMode,
    
    /// Number of retry attempts on failure
    pub retry_attempts: u32,
    
    /// Initial delay between retry attempts in seconds
    pub retry_delay_secs: u64,
    
    /// Whether to use exponential backoff for retries
    pub exponential_backoff: bool,
    
    /// Maximum bandwidth in bytes per second (0 = unlimited)
    pub max_bandwidth: u64,
    
    /// Number of parallel file operations (0 = auto, based on CPU count)
    pub parallel: usize,
    
    /// Patterns to exclude (glob patterns)
    pub exclude_patterns: Vec<String>,
    
    /// Dry run mode (don't actually copy)
    pub dry_run: bool,
    
    /// Audit log format
    pub audit_format: AuditFormat,
    
    /// Path to audit log file (None = default location)
    pub audit_log_path: Option<PathBuf>,
    
    /// Show progress bar
    pub show_progress: bool,
    
    /// Verify checksums after copy
    pub verify_checksum: bool,
}

impl Default for CopyConfig {
    fn default() -> Self {
        Self {
            compression: CompressionType::None,
            resume_enabled: false,
            chunk_size: 1024 * 1024, // 1MB default
            preserve_metadata: true,
            symlink_mode: SymlinkMode::Preserve,
            recursive: false,
            copy_mode: CopyMode::Copy,
            retry_attempts: 3,
            retry_delay_secs: 5,
            exponential_backoff: true,
            max_bandwidth: 0, // Unlimited
            parallel: 0, // Auto-detect
            exclude_patterns: Vec::new(),
            dry_run: false,
            audit_format: AuditFormat::Json,
            audit_log_path: None,
            show_progress: true,
            verify_checksum: true,
        }
    }
}

/// Configuration file structure (loaded from TOML)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub defaults: ConfigDefaults,
    
    #[serde(default)]
    pub exclude: ExcludeConfig,
    
    #[serde(default)]
    pub audit: AuditConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ConfigDefaults {
    pub compress: Option<String>,
    pub chunk_size: Option<usize>,
    pub retry_attempts: Option<u32>,
    pub preserve_metadata: Option<bool>,
    pub parallel: Option<usize>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ExcludeConfig {
    #[serde(default)]
    pub patterns: Vec<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AuditConfig {
    pub format: Option<AuditFormat>,
    pub path: Option<PathBuf>,
}


impl ConfigFile {
    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| OrbitError::Config(format!("Failed to read config file: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| OrbitError::Config(format!("Failed to parse config file: {}", e)))
    }
    
    /// Load configuration with fallback priority:
    /// 1. ./orbit.toml (project-specific)
    /// 2. ~/.orbit/orbit.toml (user defaults)
    /// 3. Built-in defaults
    pub fn load_with_fallback() -> Self {
        // Try project-local config
        if let Ok(config) = Self::load(Path::new("orbit.toml")) {
            return config;
        }
        
        // Try user config
        if let Some(home) = dirs::home_dir() {
            let user_config = home.join(".orbit").join("orbit.toml");
            if let Ok(config) = Self::load(&user_config) {
                return config;
            }
        }
        
        // Fall back to defaults
        Self::default()
    }
    
    /// Apply this config file to a CopyConfig
    pub fn apply_to(&self, config: &mut CopyConfig) {
        if let Some(ref compress) = self.defaults.compress {
            if let Ok(compression) = CompressionType::from_str(compress) {
                config.compression = compression;
            }
        }
        
        if let Some(chunk_size) = self.defaults.chunk_size {
            config.chunk_size = chunk_size * 1024; // Config file is in KB
        }
        
        if let Some(retry_attempts) = self.defaults.retry_attempts {
            config.retry_attempts = retry_attempts;
        }
        
        if let Some(preserve_metadata) = self.defaults.preserve_metadata {
            config.preserve_metadata = preserve_metadata;
        }
        
        if let Some(parallel) = self.defaults.parallel {
            config.parallel = parallel;
        }
        
        config.exclude_patterns.extend(self.exclude.patterns.clone());
        
        if let Some(format) = self.audit.format {
            config.audit_format = format;
        }
        
        if let Some(ref path) = self.audit.path {
            config.audit_log_path = Some(path.clone());
        }
    }
}


// Add dirs dependency for home directory detection
mod dirs {
    use std::path::PathBuf;
    
    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_type_parsing() {
        assert_eq!(
            CompressionType::from_str("none").unwrap(),
            CompressionType::None
        );
        assert_eq!(
            CompressionType::from_str("lz4").unwrap(),
            CompressionType::Lz4
        );
        assert_eq!(
            CompressionType::from_str("zstd:3").unwrap(),
            CompressionType::Zstd { level: 3 }
        );
        assert_eq!(
            CompressionType::from_str("zstd").unwrap(),
            CompressionType::Zstd { level: 3 }
        );
    }

    #[test]
    fn test_invalid_compression() {
        assert!(CompressionType::from_str("invalid").is_err());
        assert!(CompressionType::from_str("zstd:99").is_err());
        assert!(CompressionType::from_str("zstd:0").is_err());
    }

    #[test]
    fn test_default_config() {
        let config = CopyConfig::default();
        assert_eq!(config.chunk_size, 1024 * 1024);
        assert_eq!(config.retry_attempts, 3);
        assert!(config.verify_checksum);
    }
}