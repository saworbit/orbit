//! SSH/SFTP backend implementation
//!
//! Provides async access to remote filesystems over SSH using SFTP protocol.

use super::error::{BackendError, BackendResult};
use super::types::{DirEntry, ListOptions, ListStream, Metadata, ReadStream, WriteOptions};
use super::Backend;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream;
use secrecy::{ExposeSecret, SecretString};
use ssh2::{Session, Sftp};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncRead;

/// SSH authentication method
#[derive(Debug, Clone)]
pub enum SshAuth {
    /// Password authentication
    Password(SecretString),

    /// Public key authentication with private key file
    KeyFile {
        /// Path to private key file
        key_path: PathBuf,
        /// Optional passphrase for the key
        passphrase: Option<SecretString>,
    },

    /// SSH agent authentication
    Agent,
}

/// SSH backend configuration
#[derive(Debug, Clone)]
pub struct SshConfig {
    /// Hostname or IP address
    pub host: String,

    /// Port (default: 22)
    pub port: u16,

    /// Username
    pub username: String,

    /// Authentication method
    pub auth: SshAuth,

    /// Connection timeout in seconds
    pub timeout_secs: u64,

    /// Compression enabled
    pub compress: bool,
}

impl SshConfig {
    /// Create a new SSH configuration
    ///
    /// # Arguments
    ///
    /// * `host` - Hostname or IP address
    /// * `username` - SSH username
    /// * `auth` - Authentication method
    pub fn new(host: impl Into<String>, username: impl Into<String>, auth: SshAuth) -> Self {
        Self {
            host: host.into(),
            port: 22,
            username: username.into(),
            auth,
            timeout_secs: 30,
            compress: false,
        }
    }

    /// Set the port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the connection timeout
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Enable compression
    pub fn with_compression(mut self) -> Self {
        self.compress = true;
        self
    }

    /// Parse from URI-style string (e.g., "user@host:port")
    pub fn from_uri(uri: &str, auth: SshAuth) -> BackendResult<Self> {
        let parts: Vec<&str> = uri.split('@').collect();
        if parts.len() != 2 {
            return Err(BackendError::InvalidConfig {
                backend: "ssh".to_string(),
                message: "URI must be in format user@host[:port]".to_string(),
            });
        }

        let username = parts[0].to_string();
        let host_port: Vec<&str> = parts[1].split(':').collect();
        let host = host_port[0].to_string();
        let port = if host_port.len() > 1 {
            host_port[1]
                .parse()
                .map_err(|_| BackendError::InvalidConfig {
                    backend: "ssh".to_string(),
                    message: "Invalid port number".to_string(),
                })?
        } else {
            22
        };

        Ok(Self {
            host,
            port,
            username,
            auth,
            timeout_secs: 30,
            compress: false,
        })
    }
}

/// SSH/SFTP backend
///
/// This backend provides access to remote filesystems over SSH using the SFTP protocol.
/// It uses the `ssh2` crate for SSH connectivity.
///
/// # Memory Usage
///
/// **Note:** The current implementation of `read()` and `write()` buffers entire files
/// in memory. This provides maximum compatibility and simplicity but is not suitable
/// for files larger than available RAM. Streaming support is planned for v0.6.0.
///
/// # Thread Safety
///
/// This backend relies on `ssh2` v0.9+ internal thread safety mechanisms.
///
/// # Example
///
/// ```ignore
/// use orbit::backend::{Backend, SshBackend, SshConfig, SshAuth};
/// use secrecy::SecretString;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = SshConfig::new(
///         "example.com",
///         "user",
///         SshAuth::Password(SecretString::new("password".into()))
///     );
///
///     let backend = SshBackend::connect(config).await?;
///     let meta = backend.stat(Path::new("/remote/file.txt")).await?;
///     println!("Size: {} bytes", meta.size);
///
///     Ok(())
/// }
/// ```
pub struct SshBackend {
    #[allow(dead_code)]
    config: SshConfig,
    session: Arc<Session>,
    sftp: Arc<Sftp>,
}

impl SshBackend {
    /// Connect to SSH server and create backend
    pub async fn connect(config: SshConfig) -> BackendResult<Self> {
        // SSH operations are blocking, so run in blocking task
        let backend = tokio::task::spawn_blocking(move || Self::connect_blocking(config))
            .await
            .map_err(|e| BackendError::Other {
                backend: "ssh".to_string(),
                message: format!("Task join error: {}", e),
            })??;

        Ok(backend)
    }

    /// Blocking SSH connection
    fn connect_blocking(config: SshConfig) -> BackendResult<Self> {
        // Connect to TCP socket
        let addr = format!("{}:{}", config.host, config.port);
        let tcp = TcpStream::connect(&addr).map_err(|e| BackendError::ConnectionFailed {
            backend: "ssh".to_string(),
            endpoint: addr.clone(),
            source: Some(Box::new(e)),
        })?;

        // Set timeout
        let timeout = std::time::Duration::from_secs(config.timeout_secs);
        tcp.set_read_timeout(Some(timeout)).ok();
        tcp.set_write_timeout(Some(timeout)).ok();

        // Create SSH session
        let mut session = Session::new().map_err(|e| BackendError::Other {
            backend: "ssh".to_string(),
            message: format!("Failed to create SSH session: {}", e),
        })?;

        session.set_tcp_stream(tcp);
        session.set_timeout(config.timeout_secs as u32 * 1000); // milliseconds
        session.set_compress(config.compress);

        // Handshake
        session
            .handshake()
            .map_err(|e| BackendError::ConnectionFailed {
                backend: "ssh".to_string(),
                endpoint: addr.clone(),
                source: Some(Box::new(e)),
            })?;

        // Authenticate
        match &config.auth {
            SshAuth::Password(password) => {
                session
                    .userauth_password(&config.username, password.expose_secret())
                    .map_err(|e| BackendError::AuthenticationFailed {
                        backend: "ssh".to_string(),
                        message: format!("Password authentication failed: {}", e),
                    })?;
            }
            SshAuth::KeyFile {
                key_path,
                passphrase,
            } => {
                let pass: Option<&str> = passphrase.as_ref().map(|p| p.expose_secret());
                session
                    .userauth_pubkey_file(&config.username, None, key_path, pass)
                    .map_err(|e| BackendError::AuthenticationFailed {
                        backend: "ssh".to_string(),
                        message: format!("Key file authentication failed: {}", e),
                    })?;
            }
            SshAuth::Agent => {
                let mut agent =
                    session
                        .agent()
                        .map_err(|e| BackendError::AuthenticationFailed {
                            backend: "ssh".to_string(),
                            message: format!("Failed to connect to SSH agent: {}", e),
                        })?;

                agent
                    .connect()
                    .map_err(|e| BackendError::AuthenticationFailed {
                        backend: "ssh".to_string(),
                        message: format!("Failed to connect to SSH agent: {}", e),
                    })?;

                agent
                    .list_identities()
                    .map_err(|e| BackendError::AuthenticationFailed {
                        backend: "ssh".to_string(),
                        message: format!("Failed to list SSH agent identities: {}", e),
                    })?;

                let identities =
                    agent
                        .identities()
                        .map_err(|e| BackendError::AuthenticationFailed {
                            backend: "ssh".to_string(),
                            message: format!("Failed to get SSH agent identities: {}", e),
                        })?;

                let mut authenticated = false;
                for identity in identities {
                    if agent.userauth(&config.username, &identity).is_ok() {
                        authenticated = true;
                        break;
                    }
                }

                if !authenticated {
                    return Err(BackendError::AuthenticationFailed {
                        backend: "ssh".to_string(),
                        message: "No valid identity found in SSH agent".to_string(),
                    });
                }
            }
        }

        // Verify authentication
        if !session.authenticated() {
            return Err(BackendError::AuthenticationFailed {
                backend: "ssh".to_string(),
                message: "Authentication failed".to_string(),
            });
        }

        // Open SFTP channel
        let sftp = session.sftp().map_err(|e| BackendError::Other {
            backend: "ssh".to_string(),
            message: format!("Failed to open SFTP channel: {}", e),
        })?;

        Ok(Self {
            config,
            session: Arc::new(session),
            sftp: Arc::new(sftp),
        })
    }

    /// Convert ssh2::FileStat to backend Metadata
    #[allow(dead_code)]
    fn convert_metadata(&self, _path: &Path, stat: &ssh2::FileStat) -> Metadata {
        let is_file = stat.is_file();
        let is_dir = stat.is_dir();
        let size = stat.size.unwrap_or(0);

        let mut metadata = if is_file {
            Metadata::file(size)
        } else if is_dir {
            Metadata::directory()
        } else {
            Metadata::symlink(size)
        };

        if let Some(mtime) = stat.mtime {
            metadata.modified = Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(mtime));
        }

        if let Some(atime) = stat.atime {
            metadata.accessed = Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(atime));
        }

        metadata.permissions = stat.perm.map(|p| p & 0o777);

        metadata
    }

    /// List directory recursively (wrapper that uses standalone impl)
    #[allow(dead_code)]
    fn list_recursive_blocking(
        &self,
        path: &Path,
        base_path: &Path,
        options: &ListOptions,
        current_depth: usize,
        entries: &mut Vec<DirEntry>,
    ) -> BackendResult<()> {
        list_recursive_blocking_impl(&self.sftp, path, base_path, options, current_depth, entries)
    }
}

/// Convert ssh2::FileStat to backend Metadata (standalone function)
fn convert_stat_to_metadata(stat: &ssh2::FileStat) -> Metadata {
    let is_file = stat.is_file();
    let is_dir = stat.is_dir();
    let size = stat.size.unwrap_or(0);

    let mut metadata = if is_file {
        Metadata::file(size)
    } else if is_dir {
        Metadata::directory()
    } else {
        Metadata::symlink(size)
    };

    if let Some(mtime) = stat.mtime {
        metadata.modified = Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(mtime));
    }

    if let Some(atime) = stat.atime {
        metadata.accessed = Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(atime));
    }

    metadata.permissions = stat.perm.map(|p| p & 0o777);

    metadata
}

/// List directory recursively (standalone function for use in spawn_blocking)
fn list_recursive_blocking_impl(
    sftp: &Sftp,
    path: &Path,
    base_path: &Path,
    options: &ListOptions,
    current_depth: usize,
    entries: &mut Vec<DirEntry>,
) -> BackendResult<()> {
    // Check limits
    if let Some(max_depth) = options.max_depth {
        if current_depth >= max_depth {
            return Ok(());
        }
    }

    if let Some(max_entries) = options.max_entries {
        if entries.len() >= max_entries {
            return Ok(());
        }
    }

    // Read directory
    let dir_entries = sftp.readdir(path).map_err(|e| {
        if e.code() == ssh2::ErrorCode::Session(-31) {
            // SFTP_NO_SUCH_FILE
            BackendError::NotFound {
                path: path.to_path_buf(),
                backend: "ssh".to_string(),
            }
        } else {
            BackendError::Other {
                backend: "ssh".to_string(),
                message: format!("Failed to read directory: {}", e),
            }
        }
    })?;

    for (entry_path, stat) in dir_entries {
        // Skip hidden files if needed
        if !options.include_hidden {
            if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
        }

        let relative_path = entry_path.strip_prefix(base_path).unwrap_or(&entry_path);
        let metadata = convert_stat_to_metadata(&stat);

        entries.push(DirEntry::new(
            relative_path.to_path_buf(),
            entry_path.clone(),
            metadata.clone(),
        ));

        // Recurse if needed
        if options.recursive && stat.is_dir() {
            list_recursive_blocking_impl(
                sftp,
                &entry_path,
                base_path,
                options,
                current_depth + 1,
                entries,
            )?;
        }
    }

    Ok(())
}

#[async_trait]
impl Backend for SshBackend {
    async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
        let path = path.to_path_buf();
        let sftp = self.sftp.clone();

        tokio::task::spawn_blocking(move || {
            let stat = sftp.stat(&path).map_err(|e| {
                if e.code() == ssh2::ErrorCode::Session(-31) {
                    BackendError::NotFound {
                        path: path.clone(),
                        backend: "ssh".to_string(),
                    }
                } else {
                    BackendError::Other {
                        backend: "ssh".to_string(),
                        message: format!("Failed to stat: {}", e),
                    }
                }
            })?;

            let metadata = if stat.is_file() {
                Metadata::file(stat.size.unwrap_or(0))
            } else if stat.is_dir() {
                Metadata::directory()
            } else {
                Metadata::symlink(stat.size.unwrap_or(0))
            };

            Ok(metadata.with_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(stat.mtime.unwrap_or(0)),
            ))
        })
        .await
        .map_err(|e| BackendError::Other {
            backend: "ssh".to_string(),
            message: format!("Task join error: {}", e),
        })?
    }

    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<ListStream> {
        use futures::stream::StreamExt;

        let path = path.to_path_buf();
        let sftp = self.sftp.clone();

        // Collect entries in blocking task, then convert to stream
        let entries = tokio::task::spawn_blocking(move || -> BackendResult<Vec<_>> {
            let mut entries = Vec::new();
            list_recursive_blocking_impl(&sftp, &path, &path, &options, 0, &mut entries)?;
            Ok(entries)
        })
        .await
        .map_err(|e| BackendError::Other {
            backend: "ssh".to_string(),
            message: format!("Task join error: {}", e),
        })??;

        // Convert Vec to stream
        let stream = stream::iter(entries.into_iter().map(Ok)).boxed();
        Ok(stream)
    }

    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        // Warning for large files
        let metadata = self.stat(path).await?;
        if metadata.size > 1_000_000_000 {
            // 1GB
            tracing::warn!(
                "Orbit SSH: Reading large file ({} bytes) into memory. Consider splitting or using a different protocol until v0.6.0 streaming support.",
                metadata.size
            );
        }

        let path = path.to_path_buf();
        let sftp = self.sftp.clone();

        // Read entire file in blocking task
        let data = tokio::task::spawn_blocking(move || {
            use std::io::Read;

            let mut file = sftp.open(&path).map_err(|e| {
                if e.code() == ssh2::ErrorCode::Session(-31) {
                    BackendError::NotFound {
                        path: path.clone(),
                        backend: "ssh".to_string(),
                    }
                } else {
                    BackendError::Other {
                        backend: "ssh".to_string(),
                        message: format!("Failed to open file: {}", e),
                    }
                }
            })?;

            // Note: This buffers the full file. See struct docs.
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).map_err(BackendError::Io)?;
            Ok::<_, BackendError>(Bytes::from(buffer))
        })
        .await
        .map_err(|e| BackendError::Other {
            backend: "ssh".to_string(),
            message: format!("Task join error: {}", e),
        })??;

        // Create stream from data
        let stream = stream::once(async move { Ok(data) });
        Ok(Box::pin(stream))
    }

    async fn write(
        &self,
        path: &Path,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        _size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64> {
        use tokio::io::AsyncReadExt;

        let path = path.to_path_buf();
        let sftp = self.sftp.clone();

        // Read all data into memory first (SSH2 is synchronous)
        // TODO: For v0.6.0, implement chunked streaming with async-ssh2-lite or similar
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .await
            .map_err(BackendError::from)?;

        tokio::task::spawn_blocking(move || {
            use std::io::Write;

            // Create parent directories if needed
            if options.create_parents {
                if let Some(parent) = path.parent() {
                    sftp.mkdir(parent, 0o755).ok(); // Ignore errors if already exists
                }
            }

            // Check if file exists
            if !options.overwrite && sftp.stat(&path).is_ok() {
                return Err(BackendError::AlreadyExists { path: path.clone() });
            }

            let mut file = sftp.create(&path).map_err(|e| BackendError::Other {
                backend: "ssh".to_string(),
                message: format!("Failed to create file: {}", e),
            })?;

            file.write_all(&buffer).map_err(BackendError::Io)?;

            // Set permissions if specified
            if let Some(perms) = options.permissions {
                file.setstat(ssh2::FileStat {
                    size: None,
                    uid: None,
                    gid: None,
                    perm: Some(perms),
                    atime: None,
                    mtime: None,
                })
                .ok();
            }

            Ok(buffer.len() as u64)
        })
        .await
        .map_err(|e| BackendError::Other {
            backend: "ssh".to_string(),
            message: format!("Task join error: {}", e),
        })?
    }

    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let path = path.to_path_buf();
        let sftp = self.sftp.clone();

        tokio::task::spawn_blocking(move || {
            let stat = sftp.stat(&path).map_err(|_| BackendError::NotFound {
                path: path.clone(),
                backend: "ssh".to_string(),
            })?;

            if stat.is_dir() {
                if recursive {
                    // Recursively delete directory contents
                    fn remove_dir_all(sftp: &Sftp, path: &Path) -> BackendResult<()> {
                        for (entry_path, stat) in
                            sftp.readdir(path).map_err(|e| BackendError::Other {
                                backend: "ssh".to_string(),
                                message: format!("Failed to read directory: {}", e),
                            })?
                        {
                            if stat.is_dir() {
                                remove_dir_all(sftp, &entry_path)?;
                            } else {
                                sftp.unlink(&entry_path).map_err(|e| BackendError::Other {
                                    backend: "ssh".to_string(),
                                    message: format!("Failed to delete file: {}", e),
                                })?;
                            }
                        }
                        sftp.rmdir(path).map_err(|e| BackendError::Other {
                            backend: "ssh".to_string(),
                            message: format!("Failed to delete directory: {}", e),
                        })?;
                        Ok(())
                    }

                    remove_dir_all(&sftp, &path)?;
                } else {
                    sftp.rmdir(&path)
                        .map_err(|_e| BackendError::DirectoryNotEmpty { path: path.clone() })?;
                }
            } else {
                sftp.unlink(&path).map_err(|e| BackendError::Other {
                    backend: "ssh".to_string(),
                    message: format!("Failed to delete file: {}", e),
                })?;
            }

            Ok(())
        })
        .await
        .map_err(|e| BackendError::Other {
            backend: "ssh".to_string(),
            message: format!("Task join error: {}", e),
        })?
    }

    async fn mkdir(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let path = path.to_path_buf();
        let sftp = self.sftp.clone();

        tokio::task::spawn_blocking(move || {
            if sftp.stat(&path).is_ok() {
                return Err(BackendError::AlreadyExists { path: path.clone() });
            }

            if recursive {
                // Create parent directories
                let mut current = PathBuf::new();
                for component in path.components() {
                    current.push(component);
                    if sftp.stat(&current).is_err() {
                        sftp.mkdir(&current, 0o755).ok();
                    }
                }
            } else {
                sftp.mkdir(&path, 0o755).map_err(|e| BackendError::Other {
                    backend: "ssh".to_string(),
                    message: format!("Failed to create directory: {}", e),
                })?;
            }

            Ok(())
        })
        .await
        .map_err(|e| BackendError::Other {
            backend: "ssh".to_string(),
            message: format!("Task join error: {}", e),
        })?
    }

    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()> {
        let src = src.to_path_buf();
        let dest = dest.to_path_buf();
        let sftp = self.sftp.clone();

        tokio::task::spawn_blocking(move || {
            // Check source exists
            sftp.stat(&src).map_err(|_| BackendError::NotFound {
                path: src.clone(),
                backend: "ssh".to_string(),
            })?;

            // Check destination doesn't exist
            if sftp.stat(&dest).is_ok() {
                return Err(BackendError::AlreadyExists { path: dest.clone() });
            }

            sftp.rename(&src, &dest, None)
                .map_err(|e| BackendError::Other {
                    backend: "ssh".to_string(),
                    message: format!("Failed to rename: {}", e),
                })?;

            Ok(())
        })
        .await
        .map_err(|e| BackendError::Other {
            backend: "ssh".to_string(),
            message: format!("Task join error: {}", e),
        })?
    }

    fn backend_name(&self) -> &str {
        "ssh"
    }
}

// Note: Drop implementation to properly close SSH connection
impl Drop for SshBackend {
    fn drop(&mut self) {
        // Close SFTP channel and SSH session
        // ssh2::Sftp and Session handle cleanup automatically
        let _ = self.session.disconnect(None, "Closing connection", None);
    }
}
