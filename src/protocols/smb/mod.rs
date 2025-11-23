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
//! ```ignore
//! use orbit::protocols::smb::{SmbTarget, SmbAuth, SmbSecurity, Secret, client_for};
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let target = SmbTarget {
//!         host: "fileserver.acme.corp".to_string(),
//!         share: "projects".to_string(),
//!         subpath: "alpha/reports".to_string(),
//!         port: None, // defaults to 445
//!         auth: SmbAuth::Ntlmv2 {
//!             username: "user".to_string(),
//!             password: Secret("pass".to_string()),
//!         },
//!         security: SmbSecurity::RequireEncryption,
//!     };
//!
//!     let mut client = client_for(&target).await?;
//!     let data = client.read_file("Q4/summary.pdf", None).await?;
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod integration;
pub mod types;

#[cfg(feature = "smb-native")]
pub mod native;

#[cfg(test)]
mod tests;

// Re-export key types
pub use error::*;
pub use types::*;

#[cfg(feature = "smb-native")]
pub use native::NativeSmbClient;

use async_trait::async_trait;
use bytes::Bytes;
use std::ops::Range;

/// SMB client trait - unified interface for SMB operations
#[async_trait]
pub trait SmbClient: Send + Sync {
    /// Connect to the SMB target
    async fn connect(&mut self, target: &SmbTarget) -> Result<()>;

    /// List directory contents
    async fn list_dir(&self, rel: &str) -> Result<Vec<String>>;

    /// Read file with optional byte range
    async fn read_file(&self, rel: &str, range: Option<Range<u64>>) -> Result<Bytes>;

    /// Write file data
    async fn write_file(&self, rel: &str, data: Bytes) -> Result<()>;

    /// Create directory
    async fn mkdir(&self, rel: &str) -> Result<()>;

    /// Remove file or directory
    async fn remove(&self, rel: &str) -> Result<()>;

    /// Rename/move file
    async fn rename(&self, from_rel: &str, to_rel: &str) -> Result<()>;

    /// Get file metadata
    async fn metadata(&self, rel: &str) -> Result<SmbMetadata>;

    /// Disconnect from the server
    async fn disconnect(&mut self) -> Result<()>;
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
pub async fn client_for(target: &SmbTarget) -> Result<Box<dyn SmbClient>> {
    Ok(Box::new(native::NativeSmbClient::new(target).await?))
}

#[cfg(not(feature = "smb-native"))]
pub async fn client_for(_target: &SmbTarget) -> Result<Box<dyn SmbClient>> {
    Err(SmbError::Unsupported(
        "smb-native feature is not enabled. Rebuild with --features smb-native",
    ))
}
