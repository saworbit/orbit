//! Cargo Manifest data structures and operations
//!
//! A Cargo Manifest describes a single file's chunking, windowing, and integrity information.

use crate::error::{Error, Result};
use crate::CARGO_MANIFEST_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

/// Cargo Manifest: per-file transfer manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CargoManifest {
    /// Schema version identifier
    pub schema: String,

    /// File path (relative to job root)
    pub path: String,

    /// File size in bytes
    pub size: u64,

    /// Chunking strategy used
    pub chunking: Chunking,

    /// File-level digests (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digests: Option<Digests>,

    /// Window metadata (integrity verification units)
    pub windows: Vec<WindowMeta>,

    /// Extended attributes (metadata, permissions, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xattrs: Option<HashMap<String, serde_json::Value>>,

    /// Overall file digest (set after completion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_digest: Option<String>,
}

/// Chunking configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Chunking {
    /// Chunking type (cdc or fixed)
    #[serde(rename = "type")]
    pub chunking_type: String,

    /// Average chunk size in KiB (for CDC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_kib: Option<u32>,

    /// CDC algorithm name (e.g., "gear", "buzhash")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algo: Option<String>,

    /// Fixed chunk size in KiB (for fixed chunking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_kib: Option<u32>,
}

/// Chunking types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkingType {
    /// Content-defined chunking
    Cdc,
    /// Fixed-size chunking
    Fixed,
}

impl ChunkingType {
    /// Convert to string representation
    pub fn as_str(&self) -> &str {
        match self {
            ChunkingType::Cdc => "cdc",
            ChunkingType::Fixed => "fixed",
        }
    }
}

impl FromStr for ChunkingType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "cdc" => Ok(ChunkingType::Cdc),
            "fixed" => Ok(ChunkingType::Fixed),
            _ => Err(Error::InvalidChunking(format!(
                "Unknown chunking type: {}",
                s
            ))),
        }
    }
}

/// File-level digest collection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Digests {
    /// BLAKE3 hash (32 bytes, hex-encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blake3: Option<String>,

    /// SHA-256 hash (32 bytes, hex-encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// Window metadata for integrity verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowMeta {
    /// Window identifier (sequential)
    pub id: u32,

    /// Index of first chunk in this window
    pub first_chunk: u32,

    /// Number of chunks in this window
    pub count: u16,

    /// Merkle tree root for this window (hex-encoded)
    pub merkle_root: String,

    /// Overlap with previous window (in chunks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlap: Option<u16>,
}

impl CargoManifest {
    /// Create a new Cargo Manifest with minimal required fields
    pub fn new<S: Into<String>>(path: S, size: u64, chunking: Chunking) -> Self {
        Self {
            schema: CARGO_MANIFEST_SCHEMA_VERSION.to_string(),
            path: path.into(),
            size,
            chunking,
            digests: None,
            windows: Vec::new(),
            xattrs: None,
            file_digest: None,
        }
    }

    /// Add a window to this manifest
    pub fn add_window(&mut self, window: WindowMeta) {
        self.windows.push(window);
    }

    /// Set file-level digests
    pub fn with_digests(mut self, digests: Digests) -> Self {
        self.digests = Some(digests);
        self
    }

    /// Set extended attributes
    pub fn with_xattrs(mut self, xattrs: HashMap<String, serde_json::Value>) -> Self {
        self.xattrs = Some(xattrs);
        self
    }

    /// Finalize with overall file digest
    pub fn finalize(&mut self, file_digest: String) {
        self.file_digest = Some(file_digest);
    }

    /// Check if the manifest is finalized
    pub fn is_finalized(&self) -> bool {
        self.file_digest.is_some()
    }

    /// Get total number of chunks across all windows
    pub fn total_chunks(&self) -> u32 {
        self.windows.iter().map(|w| w.count as u32).sum()
    }

    /// Validate manifest structure
    pub fn validate(&self) -> Result<()> {
        // Check schema version
        if self.schema != CARGO_MANIFEST_SCHEMA_VERSION {
            return Err(Error::version_mismatch(
                CARGO_MANIFEST_SCHEMA_VERSION,
                &self.schema,
            ));
        }

        // Check path is not empty
        if self.path.is_empty() {
            return Err(Error::validation("Path cannot be empty"));
        }

        // Check chunking configuration
        self.chunking.validate()?;

        // Check windows are sequential
        for (i, window) in self.windows.iter().enumerate() {
            if window.id as usize != i {
                return Err(Error::validation(format!(
                    "Window IDs must be sequential: expected {}, found {}",
                    i, window.id
                )));
            }

            if window.count == 0 {
                return Err(Error::validation(format!(
                    "Window {} has zero chunks",
                    window.id
                )));
            }

            if window.merkle_root.is_empty() {
                return Err(Error::validation(format!(
                    "Window {} missing merkle_root",
                    window.id
                )));
            }
        }

        Ok(())
    }

    /// Save Cargo Manifest to a JSON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load Cargo Manifest from a JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(Error::manifest_not_found(path));
        }

        let contents = std::fs::read_to_string(path)?;
        let manifest: CargoManifest = serde_json::from_str(&contents)?;

        // Validate after loading
        manifest.validate()?;

        Ok(manifest)
    }
}

impl Chunking {
    /// Create CDC chunking configuration
    pub fn cdc(avg_kib: u32, algo: &str) -> Self {
        Self {
            chunking_type: ChunkingType::Cdc.as_str().to_string(),
            avg_kib: Some(avg_kib),
            algo: Some(algo.to_string()),
            fixed_kib: None,
        }
    }

    /// Create fixed chunking configuration
    pub fn fixed(size_kib: u32) -> Self {
        Self {
            chunking_type: ChunkingType::Fixed.as_str().to_string(),
            avg_kib: None,
            algo: None,
            fixed_kib: Some(size_kib),
        }
    }

    /// Validate chunking configuration
    pub fn validate(&self) -> Result<()> {
        let chunk_type = ChunkingType::from_str(&self.chunking_type)?;

        match chunk_type {
            ChunkingType::Cdc => {
                if self.avg_kib.is_none() {
                    return Err(Error::InvalidChunking(
                        "CDC chunking requires avg_kib".to_string(),
                    ));
                }
                if self.algo.is_none() {
                    return Err(Error::InvalidChunking(
                        "CDC chunking requires algo".to_string(),
                    ));
                }
            }
            ChunkingType::Fixed => {
                if self.fixed_kib.is_none() {
                    return Err(Error::InvalidChunking(
                        "Fixed chunking requires fixed_kib".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

impl Digests {
    /// Create a new Digests with no hashes set
    pub fn new() -> Self {
        Self {
            blake3: None,
            sha256: None,
        }
    }

    /// Set BLAKE3 hash
    pub fn with_blake3<S: Into<String>>(mut self, hash: S) -> Self {
        self.blake3 = Some(hash.into());
        self
    }

    /// Set SHA-256 hash
    pub fn with_sha256<S: Into<String>>(mut self, hash: S) -> Self {
        self.sha256 = Some(hash.into());
        self
    }
}

impl Default for Digests {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowMeta {
    /// Create a new window metadata entry
    pub fn new(id: u32, first_chunk: u32, count: u16, merkle_root: String) -> Self {
        Self {
            id,
            first_chunk,
            count,
            merkle_root,
            overlap: None,
        }
    }

    /// Set overlap with previous window
    pub fn with_overlap(mut self, overlap: u16) -> Self {
        self.overlap = Some(overlap);
        self
    }

    /// Get the last chunk index in this window
    pub fn last_chunk(&self) -> u32 {
        self.first_chunk + (self.count as u32) - 1
    }

    /// Check if this window contains a given chunk index
    pub fn contains_chunk(&self, chunk_idx: u32) -> bool {
        chunk_idx >= self.first_chunk && chunk_idx <= self.last_chunk()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_manifest_creation() {
        let chunking = Chunking::cdc(256, "gear");
        let manifest = CargoManifest::new("data/file.bin", 1024000, chunking);

        assert_eq!(manifest.schema, CARGO_MANIFEST_SCHEMA_VERSION);
        assert_eq!(manifest.path, "data/file.bin");
        assert_eq!(manifest.size, 1024000);
        assert!(!manifest.is_finalized());
    }

    #[test]
    fn test_cdc_chunking() {
        let chunking = Chunking::cdc(256, "gear");
        assert_eq!(chunking.chunking_type, "cdc");
        assert_eq!(chunking.avg_kib, Some(256));
        assert_eq!(chunking.algo, Some("gear".to_string()));
        assert!(chunking.validate().is_ok());
    }

    #[test]
    fn test_fixed_chunking() {
        let chunking = Chunking::fixed(1024);
        assert_eq!(chunking.chunking_type, "fixed");
        assert_eq!(chunking.fixed_kib, Some(1024));
        assert!(chunking.validate().is_ok());
    }

    #[test]
    fn test_invalid_cdc_chunking() {
        let mut chunking = Chunking::cdc(256, "gear");
        chunking.avg_kib = None; // Make it invalid

        let result = chunking.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("avg_kib"));
    }

    #[test]
    fn test_window_meta() {
        let window = WindowMeta::new(0, 0, 64, "abc123".to_string()).with_overlap(4);

        assert_eq!(window.id, 0);
        assert_eq!(window.first_chunk, 0);
        assert_eq!(window.count, 64);
        assert_eq!(window.last_chunk(), 63);
        assert_eq!(window.overlap, Some(4));
    }

    #[test]
    fn test_window_contains_chunk() {
        let window = WindowMeta::new(0, 10, 20, "abc123".to_string());

        assert!(!window.contains_chunk(9));
        assert!(window.contains_chunk(10));
        assert!(window.contains_chunk(15));
        assert!(window.contains_chunk(29));
        assert!(!window.contains_chunk(30));
    }

    #[test]
    fn test_digests_builder() {
        let digests = Digests::new()
            .with_blake3("blake3hash")
            .with_sha256("sha256hash");

        assert_eq!(digests.blake3, Some("blake3hash".to_string()));
        assert_eq!(digests.sha256, Some("sha256hash".to_string()));
    }

    #[test]
    fn test_add_window() {
        let chunking = Chunking::cdc(256, "gear");
        let mut manifest = CargoManifest::new("file.bin", 1024, chunking);

        manifest.add_window(WindowMeta::new(0, 0, 64, "root1".to_string()));
        manifest.add_window(WindowMeta::new(1, 60, 64, "root2".to_string()));

        assert_eq!(manifest.windows.len(), 2);
        assert_eq!(manifest.total_chunks(), 128);
    }

    #[test]
    fn test_finalization() {
        let chunking = Chunking::fixed(1024);
        let mut manifest = CargoManifest::new("file.bin", 1024, chunking);

        assert!(!manifest.is_finalized());

        manifest.finalize("sha256:finaldigest".to_string());

        assert!(manifest.is_finalized());
        assert_eq!(manifest.file_digest, Some("sha256:finaldigest".to_string()));
    }

    #[test]
    fn test_validation_empty_path() {
        let chunking = Chunking::fixed(1024);
        let manifest = CargoManifest::new("", 1024, chunking);

        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validation_sequential_windows() {
        let chunking = Chunking::fixed(1024);
        let mut manifest = CargoManifest::new("file.bin", 1024, chunking);

        manifest.add_window(WindowMeta::new(0, 0, 10, "root1".to_string()));
        manifest.add_window(WindowMeta::new(2, 10, 10, "root2".to_string())); // Skip ID 1

        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sequential"));
    }

    #[test]
    fn test_serialization() {
        let chunking = Chunking::cdc(256, "gear");
        let mut manifest = CargoManifest::new("file.bin", 1024, chunking);
        manifest.add_window(WindowMeta::new(0, 0, 64, "abc123".to_string()));

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: CargoManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(manifest.path, deserialized.path);
        assert_eq!(manifest.size, deserialized.size);
        assert_eq!(manifest.windows.len(), deserialized.windows.len());
    }
}
