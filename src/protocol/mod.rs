/*!
 * Protocol abstraction layer for Orbit
 * 
 * Supports multiple storage backends:
 * - Local filesystem
 * - SMB/CIFS network shares (via native implementation in protocols::smb)
 * - Future: S3, Azure Blob, GCS
 */

pub mod local;
pub mod uri;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use crate::error::Result;

/// File metadata across protocols
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub modified: Option<std::time::SystemTime>,
    pub permissions: Option<u32>,
}

/// Storage backend trait - unified interface for all protocols
pub trait StorageBackend: Send + Sync {
    /// Open a file for reading
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>>;
    
    /// Open a file for writing
    fn open_write(&self, path: &Path, append: bool) -> Result<Box<dyn Write + Send>>;
    
    /// Get file metadata
    fn metadata(&self, path: &Path) -> Result<FileMetadata>;
    
    /// Check if path exists
    fn exists(&self, path: &Path) -> Result<bool>;
    
    /// Create directory
    fn create_dir_all(&self, path: &Path) -> Result<()>;
    
    /// List directory contents
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    
    /// Remove file
    fn remove_file(&self, path: &Path) -> Result<()>;
    
    /// Flush/sync file to storage
    fn sync(&self, path: &Path) -> Result<()>;
    
    /// Get protocol name for logging
    fn protocol_name(&self) -> &'static str;
}

/// Protocol enum - represents different storage backends
#[derive(Debug, Clone)]
pub enum Protocol {
    Local,
    Smb {
        server: String,
        share: String,
        username: Option<String>,
        password: Option<String>,
        domain: Option<String>,
    },
}

impl Protocol {
    /// Create a backend instance from protocol configuration
    pub fn create_backend(&self) -> Result<Box<dyn StorageBackend>> {
        match self {
            Protocol::Local => Ok(Box::new(local::LocalBackend::new())),
            Protocol::Smb { .. } => {
                // SMB native implementation is in protocols::smb module
                // This requires the smb-native feature flag
                Err(crate::error::OrbitError::Config(
                    "SMB protocol requires smb-native feature. Use protocols::smb module directly.".to_string()
                ))
            }
        }
    }
    
    /// Parse a URI into protocol and path
    pub fn from_uri(uri: &str) -> Result<(Protocol, PathBuf)> {
        uri::parse_uri(uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_protocol() {
        let (protocol, path) = Protocol::from_uri("/tmp/test.txt").unwrap();
        assert!(matches!(protocol, Protocol::Local));
        assert_eq!(path, PathBuf::from("/tmp/test.txt"));
    }

    #[test]
    fn test_smb_protocol_parsing() {
        let (protocol, path) = Protocol::from_uri("smb://server/share/path/file.txt").unwrap();
        match protocol {
            Protocol::Smb { server, share, .. } => {
                assert_eq!(server, "server");
                assert_eq!(share, "share");
            }
            _ => panic!("Expected SMB protocol"),
        }
        assert_eq!(path, PathBuf::from("/path/file.txt"));
    }
    
    #[test]
    fn test_local_backend_creation() {
        let protocol = Protocol::Local;
        let backend = protocol.create_backend();
        assert!(backend.is_ok());
    }
}