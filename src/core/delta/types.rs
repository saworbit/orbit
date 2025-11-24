/*!
 * Types and enums for delta detection and efficient transfers
 */

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

/// Detection mode for determining which files to transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckMode {
    /// Compare modification time and size (fastest, like rclone default)
    ModTime,

    /// Compare size only
    Size,

    /// Full content hashing (MD5, BLAKE3) for accuracy
    Checksum,

    /// Block-based diff for partial updates (rsync-inspired)
    Delta,
}

impl Default for CheckMode {
    fn default() -> Self {
        Self::ModTime
    }
}

impl fmt::Display for CheckMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModTime => write!(f, "modtime"),
            Self::Size => write!(f, "size"),
            Self::Checksum => write!(f, "checksum"),
            Self::Delta => write!(f, "delta"),
        }
    }
}

impl std::str::FromStr for CheckMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "modtime" | "mod-time" | "time" => Ok(Self::ModTime),
            "size" => Ok(Self::Size),
            "checksum" | "hash" => Ok(Self::Checksum),
            "delta" | "rsync" => Ok(Self::Delta),
            _ => Err(format!(
                "Invalid check mode: {}. Valid options: modtime, size, checksum, delta",
                s
            )),
        }
    }
}

/// Hash algorithm for checksums
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    /// BLAKE3 (fast, secure, default)
    Blake3,

    /// MD5 (legacy compatibility)
    Md5,

    /// SHA-256
    Sha256,
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        Self::Blake3
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blake3 => write!(f, "blake3"),
            Self::Md5 => write!(f, "md5"),
            Self::Sha256 => write!(f, "sha256"),
        }
    }
}

/// Configuration for delta detection and transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaConfig {
    /// Detection mode
    pub check_mode: CheckMode,

    /// Block size for delta algorithm (default: 1MB)
    pub block_size: usize,

    /// Force full file copy (xcopy-like, disable delta)
    pub whole_file: bool,

    /// Update manifest database after transfer
    pub update_manifest: bool,

    /// Skip files that already exist at destination
    pub ignore_existing: bool,

    /// Hash algorithm to use
    pub hash_algorithm: HashAlgorithm,

    /// Enable parallel hashing
    pub parallel_hashing: bool,

    /// Manifest database path (optional)
    pub manifest_path: Option<std::path::PathBuf>,

    /// Enable resume capability for interrupted delta transfers
    #[serde(default = "default_resume_enabled")]
    pub resume_enabled: bool,

    /// Chunk size for resume tracking (default: 1MB, must be <= block_size)
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
}

fn default_resume_enabled() -> bool {
    true
}

fn default_chunk_size() -> usize {
    1024 * 1024 // 1MB
}

impl Default for DeltaConfig {
    fn default() -> Self {
        Self {
            check_mode: CheckMode::ModTime,
            block_size: 1024 * 1024, // 1MB
            whole_file: false,
            update_manifest: false,
            ignore_existing: false,
            hash_algorithm: HashAlgorithm::Blake3,
            parallel_hashing: true,
            manifest_path: None,
            resume_enabled: true,
            chunk_size: 1024 * 1024, // 1MB
        }
    }
}

impl DeltaConfig {
    /// Create a new delta config with the specified check mode
    pub fn with_check_mode(mut self, mode: CheckMode) -> Self {
        self.check_mode = mode;
        self
    }

    /// Set the block size
    pub fn with_block_size(mut self, size: usize) -> Self {
        self.block_size = size;
        self
    }

    /// Enable/disable whole file copy
    pub fn with_whole_file(mut self, whole_file: bool) -> Self {
        self.whole_file = whole_file;
        self
    }

    /// Enable/disable manifest updates
    pub fn with_manifest_updates(mut self, update: bool) -> Self {
        self.update_manifest = update;
        self
    }

    /// Set manifest path
    pub fn with_manifest_path(mut self, path: std::path::PathBuf) -> Self {
        self.manifest_path = Some(path);
        self
    }

    /// Enable/disable resume capability
    pub fn with_resume_enabled(mut self, enabled: bool) -> Self {
        self.resume_enabled = enabled;
        self
    }

    /// Set chunk size for resume tracking
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }
}

/// A block signature for delta detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSignature {
    /// Block offset in the file
    pub offset: u64,

    /// Block length
    pub length: usize,

    /// Weak rolling checksum (Adler-32)
    pub weak_hash: u32,

    /// Strong hash (BLAKE3 or MD5)
    pub strong_hash: Vec<u8>,
}

impl BlockSignature {
    /// Create a new block signature
    pub fn new(offset: u64, length: usize, weak_hash: u32, strong_hash: Vec<u8>) -> Self {
        Self {
            offset,
            length,
            weak_hash,
            strong_hash,
        }
    }
}

/// Instructions for reconstructing a file from delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaInstruction {
    /// Copy data from existing destination file
    Copy {
        /// Source offset in destination file
        src_offset: u64,
        /// Destination offset in new file
        dest_offset: u64,
        /// Number of bytes to copy
        length: usize,
    },

    /// Insert new data from source
    Data {
        /// Destination offset in new file
        dest_offset: u64,
        /// Raw bytes to insert
        bytes: Vec<u8>,
    },
}

/// Statistics for delta transfer operations
#[derive(Debug, Clone, Default)]
pub struct DeltaStats {
    /// Total number of blocks in file
    pub total_blocks: u64,

    /// Number of blocks matched (reused from destination)
    pub blocks_matched: u64,

    /// Number of blocks transferred
    pub blocks_transferred: u64,

    /// Total bytes in file
    pub total_bytes: u64,

    /// Bytes saved by reusing existing blocks
    pub bytes_saved: u64,

    /// Bytes actually transferred
    pub bytes_transferred: u64,

    /// Compression ratio (bytes_saved / total_bytes)
    pub savings_ratio: f64,

    /// Number of chunks resumed from partial manifest
    pub chunks_resumed: u64,

    /// Bytes skipped due to resume (already processed)
    pub bytes_skipped: u64,

    /// Whether this transfer was resumed from a partial manifest
    pub was_resumed: bool,
}

/// Partial manifest for tracking delta transfer progress
///
/// This enables resume capability for interrupted delta transfers.
/// The manifest is stored as a temp file (e.g., `{dest}.delta.partial.json`)
/// and cleaned up on successful completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialManifest {
    /// Source file path
    pub source_path: PathBuf,

    /// Destination file path
    pub dest_path: PathBuf,

    /// Chunk size used for tracking
    pub chunk_size: usize,

    /// List of processed chunks: (chunk_index, BLAKE3 hash hex string)
    pub processed_chunks: Vec<(usize, String)>,

    /// Bytes written so far (diff applied up to this point)
    pub diff_applied_up_to: u64,

    /// Source file size at start of transfer
    pub source_size: u64,

    /// Source file modification time at start (for validation)
    #[serde(with = "system_time_serde")]
    pub source_mtime: SystemTime,

    /// Destination file size at start of transfer
    pub dest_size: u64,

    /// Block size used for delta algorithm
    pub block_size: usize,

    /// Final checksum (populated when complete)
    pub checksum: Option<String>,

    /// Version of the manifest format (for future compatibility)
    pub version: u32,
}

/// Serde support for SystemTime
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        (duration.as_secs(), duration.subsec_nanos()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (secs, nanos): (u64, u32) = Deserialize::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::new(secs, nanos))
    }
}

impl PartialManifest {
    /// Current manifest format version
    pub const VERSION: u32 = 1;

    /// Create a new partial manifest for a delta transfer
    pub fn new(
        source_path: &std::path::Path,
        dest_path: &std::path::Path,
        chunk_size: usize,
        block_size: usize,
        source_size: u64,
        source_mtime: SystemTime,
        dest_size: u64,
    ) -> Self {
        Self {
            source_path: source_path.to_path_buf(),
            dest_path: dest_path.to_path_buf(),
            chunk_size,
            processed_chunks: Vec::new(),
            diff_applied_up_to: 0,
            source_size,
            source_mtime,
            dest_size,
            block_size,
            checksum: None,
            version: Self::VERSION,
        }
    }

    /// Get the manifest file path for a given destination
    pub fn manifest_path_for(dest_path: &std::path::Path) -> PathBuf {
        let mut path = dest_path.to_path_buf();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        path.set_file_name(format!("{}.delta.partial.json", file_name));
        path
    }

    /// Load a manifest from disk
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid manifest: {}", e),
            )
        })
    }

    /// Save the manifest to disk
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)
    }

    /// Validate that the manifest matches the current source file
    ///
    /// Returns true if the manifest is still valid for resume
    pub fn is_valid_for(&self, source_path: &std::path::Path, dest_path: &std::path::Path) -> bool {
        // Check paths match
        if self.source_path != source_path || self.dest_path != dest_path {
            return false;
        }

        // Check version compatibility
        if self.version != Self::VERSION {
            return false;
        }

        // Check source file still exists and hasn't changed
        if let Ok(metadata) = std::fs::metadata(source_path) {
            if metadata.len() != self.source_size {
                return false;
            }
            if let Ok(mtime) = metadata.modified() {
                // Allow small time differences (1 second) for filesystem precision
                let diff = if mtime > self.source_mtime {
                    mtime.duration_since(self.source_mtime).ok()
                } else {
                    self.source_mtime.duration_since(mtime).ok()
                };
                if let Some(diff) = diff {
                    if diff.as_secs() > 1 {
                        return false;
                    }
                }
            }
        } else {
            return false;
        }

        true
    }

    /// Record a processed chunk
    pub fn record_chunk(&mut self, chunk_index: usize, hash: String) {
        self.processed_chunks.push((chunk_index, hash));
    }

    /// Update the bytes written progress
    pub fn update_progress(&mut self, bytes_written: u64) {
        self.diff_applied_up_to = bytes_written;
    }

    /// Check if a chunk has already been processed
    pub fn is_chunk_processed(&self, chunk_index: usize) -> bool {
        self.processed_chunks
            .iter()
            .any(|(idx, _)| *idx == chunk_index)
    }

    /// Get the number of processed chunks
    pub fn processed_count(&self) -> usize {
        self.processed_chunks.len()
    }
}

impl DeltaStats {
    /// Create a new DeltaStats
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate the savings ratio
    pub fn calculate_savings_ratio(&mut self) {
        if self.total_bytes > 0 {
            self.savings_ratio = self.bytes_saved as f64 / self.total_bytes as f64;
        }
    }

    /// Add statistics from another delta operation
    pub fn merge(&mut self, other: &DeltaStats) {
        self.total_blocks += other.total_blocks;
        self.blocks_matched += other.blocks_matched;
        self.blocks_transferred += other.blocks_transferred;
        self.total_bytes += other.total_bytes;
        self.bytes_saved += other.bytes_saved;
        self.bytes_transferred += other.bytes_transferred;
        self.calculate_savings_ratio();
    }
}

impl fmt::Display for DeltaStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Delta: {}/{} blocks matched ({:.1}% savings, {}/{} bytes transferred)",
            self.blocks_matched,
            self.total_blocks,
            self.savings_ratio * 100.0,
            self.bytes_transferred,
            self.total_bytes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_mode_parsing() {
        assert_eq!("modtime".parse::<CheckMode>().unwrap(), CheckMode::ModTime);
        assert_eq!("size".parse::<CheckMode>().unwrap(), CheckMode::Size);
        assert_eq!(
            "checksum".parse::<CheckMode>().unwrap(),
            CheckMode::Checksum
        );
        assert_eq!("delta".parse::<CheckMode>().unwrap(), CheckMode::Delta);
        assert_eq!("rsync".parse::<CheckMode>().unwrap(), CheckMode::Delta);
        assert!("invalid".parse::<CheckMode>().is_err());
    }

    #[test]
    fn test_delta_config_builder() {
        let config = DeltaConfig::default()
            .with_check_mode(CheckMode::Delta)
            .with_block_size(512 * 1024)
            .with_whole_file(true);

        assert_eq!(config.check_mode, CheckMode::Delta);
        assert_eq!(config.block_size, 512 * 1024);
        assert!(config.whole_file);
    }

    #[test]
    fn test_delta_stats_calculation() {
        let mut stats = DeltaStats {
            total_blocks: 100,
            blocks_matched: 80,
            blocks_transferred: 20,
            total_bytes: 100_000_000,
            bytes_saved: 80_000_000,
            bytes_transferred: 20_000_000,
            savings_ratio: 0.0,
            chunks_resumed: 0,
            bytes_skipped: 0,
            was_resumed: false,
        };

        stats.calculate_savings_ratio();
        assert_eq!(stats.savings_ratio, 0.8);
    }

    #[test]
    fn test_delta_stats_merge() {
        let mut stats1 = DeltaStats {
            total_blocks: 100,
            blocks_matched: 80,
            blocks_transferred: 20,
            total_bytes: 100_000_000,
            bytes_saved: 80_000_000,
            bytes_transferred: 20_000_000,
            savings_ratio: 0.8,
            chunks_resumed: 0,
            bytes_skipped: 0,
            was_resumed: false,
        };

        let stats2 = DeltaStats {
            total_blocks: 50,
            blocks_matched: 30,
            blocks_transferred: 20,
            total_bytes: 50_000_000,
            bytes_saved: 30_000_000,
            bytes_transferred: 20_000_000,
            savings_ratio: 0.6,
            chunks_resumed: 0,
            bytes_skipped: 0,
            was_resumed: false,
        };

        stats1.merge(&stats2);
        assert_eq!(stats1.total_blocks, 150);
        assert_eq!(stats1.blocks_matched, 110);
        assert_eq!(stats1.total_bytes, 150_000_000);
        assert_eq!(stats1.bytes_saved, 110_000_000);
    }
}
