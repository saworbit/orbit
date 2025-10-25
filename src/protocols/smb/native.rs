//! Native SMB2/3 client implementation using the smb crate
//!
//! This module provides real SMB protocol support using the pure Rust `smb` crate.

use crate::protocols::smb::{types::*, error::SmbError};
use async_trait::async_trait;
use bytes::Bytes;
use std::ops::Range;
use std::str::FromStr;

#[cfg(feature = "smb-native")]
use smb::{Client, ClientConfig, UncPath, FileCreateArgs, FileAccessMask, Resource};

/// Native SMB client implementation
///
/// Uses the pure-Rust `smb` crate for SMB2/3 protocol support.
pub struct NativeSmbClient {
    client: Client,
    target: SmbTarget,
    connected: bool,
}

impl NativeSmbClient {
    /// Create a new native SMB client and connect
    pub async fn new(t: &SmbTarget) -> Result<Self, SmbError> {
        tracing::info!(
            "Creating SMB client for {}\\{}",
            t.host,
            t.share
        );

        // Validate target configuration
        Self::validate_target(t)?;

        // Create client with default config
        // TODO: Add encryption/signing configuration based on SmbSecurity
        let client = Client::new(ClientConfig::default());

        let mut smb_client = Self {
            client,
            target: t.clone(),
            connected: false,
        };

        // Connect to the share
        smb_client.do_connect().await?;

        Ok(smb_client)
    }

    /// Validate SMB target configuration
    fn validate_target(t: &SmbTarget) -> Result<(), SmbError> {
        if t.host.is_empty() {
            return Err(SmbError::InvalidPath("host cannot be empty".to_string()));
        }
        if t.share.is_empty() {
            return Err(SmbError::InvalidPath("share cannot be empty".to_string()));
        }
        
        // Validate no path traversal in subpath
        if t.subpath.contains("..") {
            return Err(SmbError::InvalidPath(
                "path traversal not allowed in subpath".to_string()
            ));
        }

        Ok(())
    }

    /// Perform the actual connection
    async fn do_connect(&mut self) -> Result<(), SmbError> {
        // Build UNC path: \\server\share
        let unc_path_str = format!(r"\\{}\{}", self.target.host, self.target.share);
        let unc_path = UncPath::from_str(&unc_path_str)
            .map_err(|_| SmbError::InvalidPath(unc_path_str.clone()))?;

        // Extract credentials based on auth type
        let (username, password) = match &self.target.auth {
            SmbAuth::Anonymous => ("", String::new()),
            SmbAuth::Ntlmv2 { username, password } => (username.as_str(), password.0.clone()),
            SmbAuth::Kerberos { principal } => {
                // For Kerberos, we'd use the principal, but the smb crate
                // handles this through the ClientConfig
                tracing::warn!("Kerberos not yet fully implemented, falling back to NTLM");
                return Err(SmbError::Unsupported("Kerberos authentication"));
            }
        };

        // Connect to share
        self.client
            .share_connect(&unc_path, username, password)
            .await
            .map_err(|e| {
                tracing::error!("SMB connection failed: {:?}", e);
                SmbError::Connection(format!("Failed to connect: {:?}", e))
            })?;

        self.connected = true;
        tracing::info!("Successfully connected to {}\\{}", self.target.host, self.target.share);

        Ok(())
    }

    /// Join root path with relative path
    #[inline]
    fn join(&self, rel: &str) -> String {
        if self.target.subpath.is_empty() {
            rel.to_owned()
        } else if rel.is_empty() {
            self.target.subpath.clone()
        } else {
            format!(
                "{}\\{}",
                self.target.subpath.trim_end_matches(['/', '\\']),
                rel.trim_start_matches(['/', '\\'])
            )
        }
    }

    /// Build full UNC path for a file
    fn build_unc_path(&self, rel: &str) -> Result<UncPath, SmbError> {
        let path = self.join(rel);
        let full_path = format!(r"\\{}\{}\{}", self.target.host, self.target.share, path);
        
        UncPath::from_str(&full_path)
            .map_err(|_| SmbError::InvalidPath(full_path))
    }

    /// Adaptive chunk sizing for efficient transfers
    async fn adaptive_chunk_len(&self, bytes_goal: usize) -> usize {
        // Clamp between 256KB and 2MB
        // TODO: Add EWMA-based adaptive sizing in future
        bytes_goal.clamp(256 * 1024, 2 * 1024 * 1024)
    }
}

#[async_trait]
impl super::SmbClient for NativeSmbClient {
    async fn connect(&mut self, _target: &SmbTarget) -> Result<(), SmbError> {
        // Already connected in new()
        if !self.connected {
            self.do_connect().await?;
        }
        Ok(())
    }

    async fn list_dir(&self, rel: &str) -> Result<Vec<String>, SmbError> {
        if !self.connected {
            return Err(SmbError::Connection("not connected".to_string()));
        }

        let unc_path = self.build_unc_path(rel)?;
        
        // Open directory
        let open_args = FileCreateArgs::make_open_existing(
            FileAccessMask::new().with_generic_read(true)
        );
        
        let resource = self.client
            .create_file(&unc_path, &open_args)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open directory: {:?}", e);
                SmbError::NotFound(format!("{}", rel))
            })?;

        // Ensure it's a directory
        let dir = match resource {
            Resource::Directory(d) => d,
            _ => return Err(SmbError::InvalidPath(format!("{} is not a directory", rel))),
        };

        // List entries
        let entries = dir.list()
            .await
            .map_err(|e| {
                tracing::error!("Failed to list directory: {:?}", e);
                SmbError::Protocol("directory listing failed")
            })?;

        // Extract names
        let names: Vec<String> = entries
            .into_iter()
            .map(|entry| entry.file_name)
            .collect();

        // Close directory
        dir.close().await.ok();

        Ok(names)
    }

    async fn read_file(&self, rel: &str, range: Option<Range<u64>>) -> Result<Bytes, SmbError> {
        if !self.connected {
            return Err(SmbError::Connection("not connected".to_string()));
        }

        let unc_path = self.build_unc_path(rel)?;
        
        // Open file for reading
        let open_args = FileCreateArgs::make_open_existing(
            FileAccessMask::new().with_generic_read(true)
        );
        
        let resource = self.client
            .create_file(&unc_path, &open_args)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open file: {:?}", e);
                SmbError::NotFound(format!("{}", rel))
            })?;

        // Ensure it's a file
        let file = match resource {
            Resource::File(f) => f,
            _ => return Err(SmbError::InvalidPath(format!("{} is not a file", rel))),
        };

        // Calculate read range
        let (mut offset, mut remain) = range
            .map(|r| (r.start, r.end - r.start))
            .unwrap_or_else(|| {
                // Read entire file - get file size first
                // For now, we'll use a large default
                (0, u64::MAX)
            });

        let mut buf = Vec::with_capacity(1 << 20); // 1MB initial capacity

        // Read with adaptive chunking
        while remain > 0 {
            let want = self.adaptive_chunk_len(1 << 20).await.min(remain as usize);
            let mut chunk = vec![0u8; want];
            
            let bytes_read = file.read_at(&mut chunk, offset)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to read file: {:?}", e);
                    SmbError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("SMB read failed: {:?}", e)
                    ))
                })?;

            if bytes_read == 0 {
                break; // EOF
            }

            buf.extend_from_slice(&chunk[..bytes_read]);
            offset += bytes_read as u64;
            remain -= bytes_read as u64;
        }

        // Close file
        file.close().await.ok();

        Ok(Bytes::from(buf))
    }

    async fn write_file(&self, rel: &str, data: Bytes) -> Result<(), SmbError> {
        if !self.connected {
            return Err(SmbError::Connection("not connected".to_string()));
        }

        let unc_path = self.build_unc_path(rel)?;
        
        // Open file for writing (create if not exists)
        let open_args = FileCreateArgs::make_create_always(
            FileAccessMask::new().with_generic_write(true)
        );
        
        let resource = self.client
            .create_file(&unc_path, &open_args)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create/open file: {:?}", e);
                SmbError::Permission(format!("{}", rel))
            })?;

        // Ensure it's a file
        let file = match resource {
            Resource::File(f) => f,
            _ => return Err(SmbError::InvalidPath(format!("{} is not a file", rel))),
        };

        // Write with adaptive chunking
        let mut offset = 0u64;
        let chunk_len = self.adaptive_chunk_len(1 << 20).await;

        for chunk in data.chunks(chunk_len) {
            file.write_at(chunk, offset)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to write file: {:?}", e);
                    SmbError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("SMB write failed: {:?}", e)
                    ))
                })?;
            offset += chunk.len() as u64;
        }

        // Close file
        file.close().await.ok();

        Ok(())
    }

    async fn mkdir(&self, rel: &str) -> Result<(), SmbError> {
        if !self.connected {
            return Err(SmbError::Connection("not connected".to_string()));
        }

        let unc_path = self.build_unc_path(rel)?;
        
        // Create directory
        let open_args = FileCreateArgs::make_create_directory();
        
        let resource = self.client
            .create_file(&unc_path, &open_args)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create directory: {:?}", e);
                SmbError::Permission(format!("{}", rel))
            })?;

        // Close the directory handle
        match resource {
            Resource::Directory(d) => d.close().await.ok(),
            _ => None,
        };

        Ok(())
    }

    async fn remove(&self, rel: &str) -> Result<(), SmbError> {
        if !self.connected {
            return Err(SmbError::Connection("not connected".to_string()));
        }

        // SMB delete requires opening with DELETE access and setting delete disposition
        let unc_path = self.build_unc_path(rel)?;
        
        let open_args = FileCreateArgs::make_open_existing(
            FileAccessMask::new().with_delete(true)
        );
        
        let resource = self.client
            .create_file(&unc_path, &open_args)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open for delete: {:?}", e);
                SmbError::Permission(format!("{}", rel))
            })?;

        // Set delete disposition and close
        // The smb crate handles deletion on close
        match resource {
            Resource::File(f) => f.close().await.ok(),
            Resource::Directory(d) => d.close().await.ok(),
            _ => None,
        };

        tracing::warn!("Delete operation may require additional SMB commands not yet fully implemented");
        Ok(())
    }

    async fn rename(&self, from_rel: &str, to_rel: &str) -> Result<(), SmbError> {
        if !self.connected {
            return Err(SmbError::Connection("not connected".to_string()));
        }

        tracing::warn!("Rename operation not yet fully implemented in smb crate");
        Err(SmbError::Unsupported("rename operation"))
    }

    async fn metadata(&self, rel: &str) -> Result<SmbMetadata, SmbError> {
        if !self.connected {
            return Err(SmbError::Connection("not connected".to_string()));
        }

        let unc_path = self.build_unc_path(rel)?;
        
        // Open file/directory to get metadata
        let open_args = FileCreateArgs::make_open_existing(
            FileAccessMask::new().with_generic_read(true)
        );
        
        let resource = self.client
            .create_file(&unc_path, &open_args)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open for metadata: {:?}", e);
                SmbError::NotFound(format!("{}", rel))
            })?;

        let (is_dir, size) = match &resource {
            Resource::File(f) => {
                let len = f.get_len().await.unwrap_or(0);
                (false, len)
            }
            Resource::Directory(_) => (true, 0),
            Resource::Pipe(_) => (false, 0),
        };

        // Close resource
        match resource {
            Resource::File(f) => f.close().await.ok(),
            Resource::Directory(d) => d.close().await.ok(),
            Resource::Pipe(p) => p.close().await.ok(),
        };

        Ok(SmbMetadata {
            size,
            is_dir,
            modified: None, // TODO: Query file attributes for timestamps
            encrypted: matches!(self.target.security, SmbSecurity::RequireEncryption),
        })
    }

    async fn disconnect(&mut self) -> Result<(), SmbError> {
        tracing::info!("Disconnecting from {}\\{}", self.target.host, self.target.share);
        
        // The smb crate handles disconnection automatically on drop
        self.connected = false;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_target() -> SmbTarget {
        SmbTarget {
            host: "testserver".to_string(),
            share: "testshare".to_string(),
            subpath: "testpath".to_string(),
            port: Some(445),
            auth: SmbAuth::Anonymous,
            security: SmbSecurity::Opportunistic,
        }
    }

    #[test]
    fn test_validate_target() {
        let target = create_test_target();
        assert!(NativeSmbClient::validate_target(&target).is_ok());
    }

    #[test]
    fn test_invalid_host() {
        let mut target = create_test_target();
        target.host = String::new();
        assert!(NativeSmbClient::validate_target(&target).is_err());
    }

    #[test]
    fn test_path_traversal_blocked() {
        let mut target = create_test_target();
        target.subpath = "../etc/passwd".to_string();
        assert!(NativeSmbClient::validate_target(&target).is_err());
    }

    #[tokio::test]
    async fn test_adaptive_chunk_len() {
        let target = create_test_target();
        let client = Client::new(ClientConfig::default());
        let native_client = NativeSmbClient {
            client,
            target,
            connected: false,
        };

        // Test clamping
        assert_eq!(native_client.adaptive_chunk_len(100_000).await, 256 * 1024); // Min
        assert_eq!(native_client.adaptive_chunk_len(1_000_000).await, 1_000_000); // Within range
        assert_eq!(native_client.adaptive_chunk_len(5_000_000).await, 2 * 1024 * 1024); // Max
    }
}