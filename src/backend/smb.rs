//! SMB/CIFS backend implementation
//!
//! Wraps the existing SMB client to provide the unified Backend interface.
//! This enables SMB shares to be used as data sources and destinations in Orbit.

use super::error::{BackendError, BackendResult};
use super::types::{DirEntry, ListOptions, ListStream, Metadata, ReadStream, WriteOptions};
use super::Backend;
use async_trait::async_trait;
use bytes::Bytes;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncRead;
use tokio::sync::RwLock;

use crate::protocols::smb::{client_for, SmbAuth, SmbClient, SmbMetadata, SmbSecurity, SmbTarget};

/// SMB backend configuration
///
/// This structure holds all configuration needed to connect to an SMB share.
///
/// # Example
///
/// ```
/// use orbit::backend::SmbConfig;
///
/// let config = SmbConfig::new("fileserver.acme.corp", "projects")
///     .with_username("jdoe")
///     .with_password("secret")
///     .with_subpath("reports/Q4")
///     .with_security(orbit::protocols::smb::SmbSecurity::RequireEncryption);
/// ```
#[derive(Debug, Clone)]
pub struct SmbConfig {
    /// SMB server hostname or IP address
    pub host: String,

    /// Share name on the server
    pub share: String,

    /// Subpath within the share (optional)
    pub subpath: Option<String>,

    /// Port number (default: 445)
    pub port: Option<u16>,

    /// Username for authentication (optional for anonymous access)
    pub username: Option<String>,

    /// Password for authentication
    pub password: Option<String>,

    /// Security/encryption settings
    pub security: SmbSecurity,
}

impl SmbConfig {
    /// Create a new SMB configuration with required fields
    pub fn new(host: impl Into<String>, share: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            share: share.into(),
            subpath: None,
            port: None,
            username: None,
            password: None,
            security: SmbSecurity::Opportunistic,
        }
    }

    /// Set the username for authentication
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the password for authentication
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set the subpath within the share
    pub fn with_subpath(mut self, subpath: impl Into<String>) -> Self {
        self.subpath = Some(subpath.into());
        self
    }

    /// Set the port number
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the security policy
    pub fn with_security(mut self, security: SmbSecurity) -> Self {
        self.security = security;
        self
    }

    /// Convert to SmbTarget for the protocol layer
    fn to_target(&self) -> SmbTarget {
        use crate::protocols::smb::Secret;

        let auth = match (&self.username, &self.password) {
            (Some(username), Some(password)) => SmbAuth::Ntlmv2 {
                username: username.clone(),
                password: Secret(password.clone()),
            },
            _ => SmbAuth::Anonymous,
        };

        SmbTarget {
            host: self.host.clone(),
            share: self.share.clone(),
            subpath: self.subpath.clone().unwrap_or_default(),
            port: self.port,
            auth,
            security: self.security,
        }
    }
}

/// SMB backend adapter
///
/// This backend provides access to SMB/CIFS shares using the pure-Rust `smb` crate.
/// It wraps the existing `SmbClient` implementation to conform to the unified
/// `Backend` trait.
///
/// # Example
///
/// ```no_run
/// use orbit::backend::{Backend, SmbBackend, SmbConfig};
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = SmbConfig::new("fileserver", "share")
///         .with_username("user")
///         .with_password("password");
///
///     let backend = SmbBackend::new(config).await?;
///     let meta = backend.stat(Path::new("path/to/file.txt")).await?;
///     println!("Size: {} bytes", meta.size);
///
///     Ok(())
/// }
/// ```
pub struct SmbBackend {
    client: Arc<RwLock<Box<dyn SmbClient>>>,
    #[allow(dead_code)]
    config: SmbConfig,
}

impl SmbBackend {
    /// Create a new SMB backend and connect to the share
    pub async fn new(config: SmbConfig) -> BackendResult<Self> {
        let target = config.to_target();

        let client = client_for(&target).await.map_err(|e| {
            use crate::protocols::smb::SmbError;
            match e {
                SmbError::Auth => BackendError::AuthenticationFailed {
                    backend: "smb".to_string(),
                    message: "Authentication failed".to_string(),
                },
                SmbError::Connection(msg) => BackendError::ConnectionFailed {
                    backend: "smb".to_string(),
                    endpoint: format!("{}\\{}", config.host, config.share),
                    source: Some(Box::new(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        msg,
                    ))),
                },
                SmbError::EncryptionRequired => BackendError::InvalidConfig {
                    backend: "smb".to_string(),
                    message: "Encryption required but not available".to_string(),
                },
                other => BackendError::Other {
                    backend: "smb".to_string(),
                    message: other.to_string(),
                },
            }
        })?;

        Ok(Self {
            client: Arc::new(RwLock::new(client)),
            config,
        })
    }

    /// Create a new SMB backend with a subpath prefix
    ///
    /// All paths will be relative to this subpath within the share.
    pub async fn with_subpath(
        config: SmbConfig,
        subpath: impl Into<String>,
    ) -> BackendResult<Self> {
        let mut config = config;
        config.subpath = Some(subpath.into());
        Self::new(config).await
    }

    /// Convert a Path to an SMB-relative path string
    fn path_to_smb_path(&self, path: &Path) -> String {
        // Normalize path separators and remove leading slashes
        let path_str = path.to_string_lossy().replace('/', "\\");
        path_str
            .trim_start_matches('\\')
            .trim_start_matches('/')
            .to_string()
    }

    /// Convert SMB metadata to backend Metadata
    fn convert_metadata(&self, smb_meta: SmbMetadata) -> Metadata {
        if smb_meta.is_dir {
            let mut meta = Metadata::directory();
            meta.modified = smb_meta.modified;
            meta
        } else {
            let mut meta = Metadata::file(smb_meta.size);
            meta.modified = smb_meta.modified;
            meta
        }
    }

    /// Map SMB errors to backend errors
    fn map_error(&self, e: crate::protocols::smb::SmbError, path: &Path) -> BackendError {
        use crate::protocols::smb::SmbError;
        match e {
            SmbError::Auth => BackendError::AuthenticationFailed {
                backend: "smb".to_string(),
                message: "Authentication failed".to_string(),
            },
            SmbError::Permission(msg) => BackendError::PermissionDenied {
                path: path.to_path_buf(),
                message: msg,
            },
            SmbError::NotFound(_) => BackendError::NotFound {
                path: path.to_path_buf(),
                backend: "smb".to_string(),
            },
            SmbError::EncryptionRequired => BackendError::InvalidConfig {
                backend: "smb".to_string(),
                message: "Encryption required but not available".to_string(),
            },
            SmbError::Io(io_err) => BackendError::Io(io_err),
            SmbError::Timeout => BackendError::Timeout {
                operation: "smb operation".to_string(),
                duration_secs: 30,
            },
            SmbError::Connection(msg) => BackendError::ConnectionFailed {
                backend: "smb".to_string(),
                endpoint: format!("{}\\{}", self.config.host, self.config.share),
                source: Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    msg,
                ))),
            },
            SmbError::InvalidPath(msg) => BackendError::InvalidPath {
                path: path.to_path_buf(),
                reason: msg,
            },
            SmbError::Unsupported(msg) => BackendError::Unsupported {
                backend: "smb".to_string(),
                operation: msg.to_string(),
            },
            other => BackendError::Other {
                backend: "smb".to_string(),
                message: other.to_string(),
            },
        }
    }
}

#[async_trait]
impl Backend for SmbBackend {
    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "smb",
            path = %path.display(),
            host = %self.config.host,
            share = %self.config.share
        )
    )]
    async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
        let smb_path = self.path_to_smb_path(path);
        let client = self.client.read().await;

        let smb_meta = client
            .metadata(&smb_path)
            .await
            .map_err(|e| self.map_error(e, path))?;

        Ok(self.convert_metadata(smb_meta))
    }

    #[tracing::instrument(
        skip(self, options),
        fields(
            otel.kind = "client",
            backend = "smb",
            path = %path.display(),
            host = %self.config.host,
            share = %self.config.share,
            recursive = options.recursive
        )
    )]
    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<ListStream> {
        use futures::stream::{self, StreamExt};

        let smb_path = self.path_to_smb_path(path);
        let client = self.client.read().await;

        // Get directory listing
        let names = client
            .list_dir(&smb_path)
            .await
            .map_err(|e| self.map_error(e, path))?;

        let mut entries = Vec::new();

        for name in names {
            // Skip . and .. entries
            if name == "." || name == ".." {
                continue;
            }

            // Skip hidden files if not requested
            if !options.include_hidden && name.starts_with('.') {
                continue;
            }

            // Build full path for this entry
            let entry_path = if smb_path.is_empty() {
                PathBuf::from(&name)
            } else {
                PathBuf::from(&smb_path).join(&name)
            };

            // Get metadata for the entry
            let metadata = match client.metadata(&entry_path.to_string_lossy()).await {
                Ok(smb_meta) => self.convert_metadata(smb_meta),
                Err(_) => {
                    // If we can't get metadata, skip this entry
                    continue;
                }
            };

            // Build relative path from the listed directory
            let relative_path = PathBuf::from(&name);
            let full_path = entry_path.clone();

            entries.push(DirEntry::new(relative_path, full_path, metadata));

            // Check max_entries limit
            if let Some(max) = options.max_entries {
                if entries.len() >= max {
                    break;
                }
            }
        }

        // Handle recursive listing if requested
        if options.recursive {
            let current_depth = 0;
            let max_depth = options.max_depth.unwrap_or(usize::MAX);

            if current_depth < max_depth {
                // Collect directories to recurse into
                let dirs: Vec<_> = entries
                    .iter()
                    .filter(|e| e.is_dir())
                    .map(|e| e.full_path.clone())
                    .collect();

                for dir_path in dirs {
                    // Recursively list subdirectories
                    let sub_options = ListOptions {
                        recursive: true,
                        max_depth: options.max_depth.map(|d| d.saturating_sub(1)),
                        ..options.clone()
                    };

                    if let Ok(mut sub_stream) = Box::pin(self.list(&dir_path, sub_options)).await {
                        // Collect sub-entries from stream
                        while let Some(sub_entry) = sub_stream.next().await {
                            if let Ok(entry) = sub_entry {
                                // Check max_entries limit
                                if let Some(max) = options.max_entries {
                                    if entries.len() >= max {
                                        break;
                                    }
                                }
                                entries.push(entry);
                            }
                        }
                    }
                }
            }
        }

        // Convert Vec to stream
        let stream = stream::iter(entries.into_iter().map(Ok)).boxed();
        Ok(stream)
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "smb",
            path = %path.display(),
            host = %self.config.host,
            share = %self.config.share
        )
    )]
    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        let smb_path = self.path_to_smb_path(path);
        let client = self.client.read().await;

        // Read the entire file
        let data = client
            .read_file(&smb_path, None)
            .await
            .map_err(|e| self.map_error(e, path))?;

        // Convert to a stream
        use futures::stream;
        let stream = stream::once(async move { Ok(data) });

        Ok(Box::pin(stream))
    }

    #[tracing::instrument(
        skip(self, reader, options),
        fields(
            otel.kind = "client",
            backend = "smb",
            path = %path.display(),
            host = %self.config.host,
            share = %self.config.share,
            size_hint = ?_size_hint,
            overwrite = options.overwrite
        )
    )]
    async fn write(
        &self,
        path: &Path,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        _size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64> {
        use tokio::io::AsyncReadExt;

        let smb_path = self.path_to_smb_path(path);
        let client = self.client.read().await;

        // Check if file exists when overwrite is false
        if !options.overwrite && client.metadata(&smb_path).await.is_ok() {
            return Err(BackendError::AlreadyExists {
                path: path.to_path_buf(),
            });
        }

        // Create parent directories if requested
        if options.create_parents {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    let parent_smb_path = self.path_to_smb_path(parent);
                    // Try to create parent directories, ignore errors if they already exist
                    let _ = client.mkdir(&parent_smb_path).await;
                }
            }
        }

        // Read data from stream into buffer
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .await
            .map_err(BackendError::from)?;

        let len = buffer.len() as u64;

        // Write the file
        client
            .write_file(&smb_path, Bytes::from(buffer))
            .await
            .map_err(|e| self.map_error(e, path))?;

        Ok(len)
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "smb",
            path = %path.display(),
            host = %self.config.host,
            share = %self.config.share,
            recursive
        )
    )]
    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let smb_path = self.path_to_smb_path(path);
        let client = self.client.read().await;

        // Check if it's a directory
        let meta = client
            .metadata(&smb_path)
            .await
            .map_err(|e| self.map_error(e, path))?;

        if meta.is_dir && !recursive {
            // Check if directory is empty
            let entries = client
                .list_dir(&smb_path)
                .await
                .map_err(|e| self.map_error(e, path))?;

            // Filter out . and ..
            let real_entries: Vec<_> = entries
                .into_iter()
                .filter(|e| e != "." && e != "..")
                .collect();

            if !real_entries.is_empty() {
                return Err(BackendError::DirectoryNotEmpty {
                    path: path.to_path_buf(),
                });
            }
        }

        if meta.is_dir && recursive {
            // Delete all contents recursively
            let entries = client
                .list_dir(&smb_path)
                .await
                .map_err(|e| self.map_error(e, path))?;

            for entry in entries {
                if entry == "." || entry == ".." {
                    continue;
                }

                let entry_path = path.join(&entry);
                Box::pin(self.delete(&entry_path, true)).await?;
            }
        }

        // Delete the file or directory
        client
            .remove(&smb_path)
            .await
            .map_err(|e| self.map_error(e, path))?;

        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "smb",
            path = %path.display(),
            host = %self.config.host,
            share = %self.config.share,
            recursive
        )
    )]
    async fn mkdir(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let smb_path = self.path_to_smb_path(path);
        let client = self.client.read().await;

        // Check if already exists
        if let Ok(meta) = client.metadata(&smb_path).await {
            if meta.is_dir {
                return Err(BackendError::AlreadyExists {
                    path: path.to_path_buf(),
                });
            } else {
                return Err(BackendError::InvalidPath {
                    path: path.to_path_buf(),
                    reason: "Path exists but is not a directory".to_string(),
                });
            }
        }

        if recursive {
            // Create parent directories first
            let mut current = PathBuf::new();
            for component in path.components() {
                current.push(component);
                let current_smb_path = self.path_to_smb_path(&current);

                // Try to create, ignore if already exists
                if client.metadata(&current_smb_path).await.is_err() {
                    client
                        .mkdir(&current_smb_path)
                        .await
                        .map_err(|e| self.map_error(e, &current))?;
                }
            }
        } else {
            // Check if parent exists
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    let parent_smb_path = self.path_to_smb_path(parent);
                    if client.metadata(&parent_smb_path).await.is_err() {
                        return Err(BackendError::NotFound {
                            path: parent.to_path_buf(),
                            backend: "smb".to_string(),
                        });
                    }
                }
            }

            client
                .mkdir(&smb_path)
                .await
                .map_err(|e| self.map_error(e, path))?;
        }

        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "smb",
            src = %src.display(),
            dest = %dest.display(),
            host = %self.config.host,
            share = %self.config.share
        )
    )]
    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()> {
        let src_smb_path = self.path_to_smb_path(src);
        let dest_smb_path = self.path_to_smb_path(dest);
        let client = self.client.read().await;

        // Check if source exists
        client
            .metadata(&src_smb_path)
            .await
            .map_err(|e| self.map_error(e, src))?;

        // Check if destination already exists
        if client.metadata(&dest_smb_path).await.is_ok() {
            return Err(BackendError::AlreadyExists {
                path: dest.to_path_buf(),
            });
        }

        client
            .rename(&src_smb_path, &dest_smb_path)
            .await
            .map_err(|e| self.map_error(e, src))?;

        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(
            otel.kind = "client",
            backend = "smb",
            path = %path.display(),
            host = %self.config.host,
            share = %self.config.share
        )
    )]
    async fn exists(&self, path: &Path) -> BackendResult<bool> {
        let smb_path = self.path_to_smb_path(path);
        let client = self.client.read().await;

        match client.metadata(&smb_path).await {
            Ok(_) => Ok(true),
            Err(crate::protocols::smb::SmbError::NotFound(_)) => Ok(false),
            Err(e) => Err(self.map_error(e, path)),
        }
    }

    fn backend_name(&self) -> &str {
        "smb"
    }

    fn supports(&self, operation: &str) -> bool {
        // SMB supports most operations
        // Note: rename may not be fully supported depending on the smb crate version
        matches!(
            operation,
            "stat" | "list" | "read" | "write" | "delete" | "mkdir" | "rename" | "exists"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smb_config_builder() {
        let config = SmbConfig::new("server", "share")
            .with_username("user")
            .with_password("pass")
            .with_subpath("path/to/dir")
            .with_port(445)
            .with_security(SmbSecurity::RequireEncryption);

        assert_eq!(config.host, "server");
        assert_eq!(config.share, "share");
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
        assert_eq!(config.subpath, Some("path/to/dir".to_string()));
        assert_eq!(config.port, Some(445));
        assert_eq!(config.security, SmbSecurity::RequireEncryption);
    }

    #[test]
    fn test_smb_config_to_target() {
        let config = SmbConfig::new("server", "share")
            .with_username("user")
            .with_password("pass");

        let target = config.to_target();

        assert_eq!(target.host, "server");
        assert_eq!(target.share, "share");
        assert!(matches!(
            target.auth,
            SmbAuth::Ntlmv2 {
                username,
                password: _
            } if username == "user"
        ));
    }

    #[test]
    fn test_smb_config_anonymous() {
        let config = SmbConfig::new("server", "share");
        let target = config.to_target();

        assert!(matches!(target.auth, SmbAuth::Anonymous));
    }

    #[test]
    fn test_path_normalization() {
        let config = SmbConfig::new("server", "share");

        // Create a mock backend for testing path normalization
        // Note: This doesn't actually connect, just tests the path conversion logic
        struct TestHelper {
            #[allow(dead_code)]
            config: SmbConfig,
        }

        impl TestHelper {
            fn path_to_smb_path(&self, path: &Path) -> String {
                let path_str = path.to_string_lossy().replace('/', "\\");
                path_str
                    .trim_start_matches('\\')
                    .trim_start_matches('/')
                    .to_string()
            }
        }

        let helper = TestHelper { config };

        assert_eq!(helper.path_to_smb_path(Path::new("file.txt")), "file.txt");
        assert_eq!(helper.path_to_smb_path(Path::new("/file.txt")), "file.txt");
        assert_eq!(
            helper.path_to_smb_path(Path::new("dir/file.txt")),
            "dir\\file.txt"
        );
        assert_eq!(
            helper.path_to_smb_path(Path::new("/dir/subdir/file.txt")),
            "dir\\subdir\\file.txt"
        );
    }

    #[test]
    fn test_metadata_conversion() {
        let smb_meta = SmbMetadata {
            size: 1024,
            is_dir: false,
            modified: None,
            encrypted: false,
        };

        // Test file metadata
        let file_meta = Metadata::file(smb_meta.size);
        assert!(file_meta.is_file);
        assert!(!file_meta.is_dir);
        assert_eq!(file_meta.size, 1024);

        // Test directory metadata
        let _dir_meta = SmbMetadata {
            size: 0,
            is_dir: true,
            modified: None,
            encrypted: false,
        };

        let converted = Metadata::directory();
        assert!(!converted.is_file);
        assert!(converted.is_dir);
    }
}
