//! SMB2/3 protocol support for Orbit
//!
//! This module provides native SMB2/3 client functionality using pure Rust.
//! It is gated behind the `smb-native` feature flag and disabled by default.
//!
//! # Feature Flag
//!
//! Enable with: `--features smb-native`
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(feature = "smb-native")]
//! # {
//! use orbit::protocols::smb::{SmbTarget, SmbAuth, SmbSecurity, client_for};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let target = SmbTarget {
//!     host: "fileserver.acme.corp".to_string(),
//!     share: "projects".to_string(),
//!     subpath: "alpha/reports".to_string(),
//!     port: None, // defaults to 445
//!     auth: SmbAuth::Ntlmv2 {
//!         username: "user".to_string(),
//!         password: crate::protocols::smb::Secret("pass".to_string()),
//!     },
//!     security: SmbSecurity::RequireEncryption,
//! };
//!
//! let mut client = client_for(&target).await?;
//! let data = client.read_file("Q4/summary.pdf", None).await?;
//! # Ok(())
//! # }
//! # }
//! ```

pub mod types;
pub mod error;
pub mod integration;

#[cfg(feature = "smb-native")]
pub mod native;

#[cfg(test)]
mod tests;

// Re-export key types
pub use types::*;
pub use error::*;

#[cfg(feature = "smb-native")]
pub use native::NativeSmbClient;

use async_trait::async_trait;
use bytes::Bytes;
use std::ops::Range;

/// SMB client trait - unified interface for SMB operations
#[async_trait]
pub trait SmbClient: Send + Sync {
    /// Connect to the SMB target
    async fn connect(&mut self, target: &SmbTarget) -> Result<(), SmbError>;
    
    /// List directory contents
    async fn list_dir(&self, rel: &str) -> Result<Vec<String>, SmbError>;
    
    /// Read file with optional byte range
    async fn read_file(&self, rel: &str, range: Option<Range<u64>>) -> Result<Bytes, SmbError>;
    
    /// Write file data
    async fn write_file(&self, rel: &str, data: Bytes) -> Result<(), SmbError>;
    
    /// Create directory
    async fn mkdir(&self, rel: &str) -> Result<(), SmbError>;
    
    /// Remove file or directory
    async fn remove(&self, rel: &str) -> Result<(), SmbError>;
    
    /// Rename/move file
    async fn rename(&self, from_rel: &str, to_rel: &str) -> Result<(), SmbError>;
    
    /// Get file metadata
    async fn metadata(&self, rel: &str) -> Result<SmbMetadata, SmbError>;
    
    /// Disconnect from the server
    async fn disconnect(&mut self) -> Result<(), SmbError>;
}

/// Factory function to create an SMB client
///
/// This function is only available when the `smb-native` feature is enabled.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "smb-native")]
/// # {
/// use orbit::protocols::smb::{SmbTarget, SmbAuth, SmbSecurity, client_for};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let target = SmbTarget {
///     host: "server".to_string(),
///     share: "data".to_string(),
///     subpath: "".to_string(),
///     port: None,
///     auth: SmbAuth::Anonymous,
///     security: SmbSecurity::Opportunistic,
/// };
///
/// let client = client_for(&target).await?;
/// # Ok(())
/// # }
/// # }
/// ```
#[cfg(feature = "smb-native")]
pub async fn client_for(target: &SmbTarget) -> Result<Box<dyn SmbClient>, SmbError> {
    Ok(Box::new(native::NativeSmbClient::new(target).await?))
}

#[cfg(not(feature = "smb-native"))]
pub async fn client_for(_target: &SmbTarget) -> Result<Box<dyn SmbClient>, SmbError> {
    Err(SmbError::Unsupported("smb-native feature is not enabled. Rebuild with --features smb-native"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smb_target_construction() {
        let target = SmbTarget {
            host: "server".to_string(),
            share: "share".to_string(),
            subpath: "path".to_string(),
            port: Some(445),
            auth: SmbAuth::Anonymous,
            security: SmbSecurity::Opportunistic,
        };
        
        assert_eq!(target.host, "server");
        assert_eq!(target.share, "share");
        assert_eq!(target.subpath, "path");
        assert_eq!(target.port, Some(445));
    }

    #[cfg(not(feature = "smb-native"))]
    #[tokio::test]
    async fn test_client_for_without_feature() {
        let target = SmbTarget::default();
        let result = client_for(&target).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("smb-native feature"));
    }
}