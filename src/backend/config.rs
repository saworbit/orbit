//! Backend configuration and URI parsing
//!
//! Provides configuration structures and URI parsing for backend initialization.

use super::error::{BackendError, BackendResult};
use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(feature = "ssh-backend")]
use super::ssh::{SshAuth, SshConfig};

#[cfg(feature = "s3-native")]
use crate::protocol::s3::S3Config;

#[cfg(feature = "smb-native")]
use super::smb::SmbConfig;

/// Unified backend configuration
#[derive(Debug, Clone)]
pub enum BackendConfig {
    /// Local filesystem backend
    Local {
        /// Optional root directory
        root: Option<PathBuf>,
    },

    /// SSH/SFTP backend
    #[cfg(feature = "ssh-backend")]
    Ssh(SshConfig),

    /// S3-compatible storage backend
    #[cfg(feature = "s3-native")]
    S3 {
        /// S3 configuration
        config: S3Config,
        /// Optional prefix (like a root directory)
        prefix: Option<String>,
    },

    /// SMB/CIFS network share backend
    #[cfg(feature = "smb-native")]
    Smb(SmbConfig),
}

impl BackendConfig {
    /// Create a local backend configuration
    pub fn local() -> Self {
        Self::Local { root: None }
    }

    /// Create a local backend with root directory
    pub fn local_with_root<P: Into<PathBuf>>(root: P) -> Self {
        Self::Local {
            root: Some(root.into()),
        }
    }

    /// Create SSH backend configuration
    #[cfg(feature = "ssh-backend")]
    pub fn ssh(config: SshConfig) -> Self {
        Self::Ssh(config)
    }

    /// Create S3 backend configuration
    #[cfg(feature = "s3-native")]
    pub fn s3(config: S3Config) -> Self {
        Self::S3 {
            config,
            prefix: None,
        }
    }

    /// Create S3 backend with prefix
    #[cfg(feature = "s3-native")]
    pub fn s3_with_prefix(config: S3Config, prefix: impl Into<String>) -> Self {
        Self::S3 {
            config,
            prefix: Some(prefix.into()),
        }
    }

    /// Create SMB backend configuration
    #[cfg(feature = "smb-native")]
    pub fn smb(config: SmbConfig) -> Self {
        Self::Smb(config)
    }

    /// Get backend type name
    pub fn backend_type(&self) -> &'static str {
        match self {
            Self::Local { .. } => "local",
            #[cfg(feature = "ssh-backend")]
            Self::Ssh(_) => "ssh",
            #[cfg(feature = "s3-native")]
            Self::S3 { .. } => "s3",
            #[cfg(feature = "smb-native")]
            Self::Smb(_) => "smb",
        }
    }
}

/// Parse a URI into backend configuration
///
/// Supported URI formats:
/// - `file:///path/to/dir` or `/path/to/dir` - Local filesystem
/// - `ssh://user@host:port/path` - SSH/SFTP (requires ssh-backend feature)
/// - `s3://bucket/prefix?region=us-east-1&endpoint=...` - S3 (requires s3-native feature)
/// - `smb://[user[:pass]@]host[:port]/share/path` - SMB/CIFS (requires smb-native feature)
///
/// # Query Parameters
///
/// SSH URIs:
/// - `key=/path/to/key` - Path to SSH private key
/// - `password=secret` - SSH password (not recommended, use key)
/// - `agent=true` - Use SSH agent
///
/// S3 URIs:
/// - `region=us-east-1` - AWS region
/// - `endpoint=http://localhost:9000` - Custom endpoint (MinIO, etc.)
/// - `access_key=KEY` - AWS access key
/// - `secret_key=SECRET` - AWS secret key
/// - `path_style=true` - Force path-style addressing
///
/// SMB URIs:
/// - `security=require_encryption` - Require SMB3 encryption (default: opportunistic)
/// - `security=sign_only` - Only sign, no encryption
/// - `security=opportunistic` - Use encryption if available
///
/// # Examples
///
/// ```
/// use orbit::backend::parse_uri;
///
/// // Local filesystem
/// let config = parse_uri("file:///tmp/data").unwrap();
/// let config = parse_uri("/tmp/data").unwrap();
///
/// // SSH
/// # #[cfg(feature = "ssh-backend")]
/// let config = parse_uri("ssh://user@example.com:22/remote/path?key=/home/user/.ssh/id_rsa").unwrap();
///
/// // S3
/// # #[cfg(feature = "s3-native")]
/// let config = parse_uri("s3://my-bucket/prefix?region=us-west-2").unwrap();
///
/// // SMB
/// # #[cfg(feature = "smb-native")]
/// let config = parse_uri("smb://user:pass@fileserver/share/path?security=require_encryption").unwrap();
/// ```
pub fn parse_uri(uri: &str) -> BackendResult<(BackendConfig, PathBuf)> {
    // Handle simple local paths (no scheme)
    if !uri.contains("://") && !uri.starts_with("s3://") {
        return Ok((BackendConfig::Local { root: None }, PathBuf::from(uri)));
    }

    // Parse URL
    let url = url::Url::parse(uri).map_err(|e| BackendError::InvalidConfig {
        backend: "unknown".to_string(),
        message: format!("Invalid URI: {}", e),
    })?;

    let scheme = url.scheme();

    match scheme {
        "file" => {
            let path = url.path();
            Ok((BackendConfig::Local { root: None }, PathBuf::from(path)))
        }

        #[cfg(feature = "ssh-backend")]
        "ssh" | "sftp" => {
            let host = url
                .host_str()
                .ok_or_else(|| BackendError::InvalidConfig {
                    backend: "ssh".to_string(),
                    message: "Missing host in SSH URI".to_string(),
                })?
                .to_string();

            let port = url.port().unwrap_or(22);

            let username = if !url.username().is_empty() {
                url.username().to_string()
            } else {
                std::env::var("USER")
                    .or_else(|_| std::env::var("USERNAME"))
                    .unwrap_or_else(|_| "root".to_string())
            };

            let path = url.path().to_string();

            // Parse query parameters for authentication
            let query_pairs: HashMap<String, String> = url
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            let auth = if let Some(key_path) = query_pairs.get("key") {
                let passphrase = query_pairs
                    .get("passphrase")
                    .map(|p| secrecy::SecretString::new(p.clone().into_boxed_str()));
                SshAuth::KeyFile {
                    key_path: PathBuf::from(key_path),
                    passphrase,
                }
            } else if let Some(password) = query_pairs.get("password") {
                SshAuth::Password(secrecy::SecretString::new(password.clone().into_boxed_str()))
            } else if query_pairs
                .get("agent")
                .map(|v| v == "true")
                .unwrap_or(false)
            {
                SshAuth::Agent
            } else {
                // Default to agent
                SshAuth::Agent
            };

            let config = SshConfig::new(host, username, auth).with_port(port);

            Ok((BackendConfig::Ssh(config), PathBuf::from(path)))
        }

        #[cfg(feature = "s3-native")]
        "s3" => {
            let bucket = url
                .host_str()
                .ok_or_else(|| BackendError::InvalidConfig {
                    backend: "s3".to_string(),
                    message: "Missing bucket in S3 URI".to_string(),
                })?
                .to_string();

            let prefix = url.path().trim_start_matches('/').to_string();
            let path = PathBuf::from(&prefix);

            // Parse query parameters
            let query_pairs: HashMap<String, String> = url
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            let mut s3_config = S3Config::new(bucket);

            if let Some(region) = query_pairs.get("region") {
                s3_config.region = Some(region.clone());
            }

            if let Some(endpoint) = query_pairs.get("endpoint") {
                s3_config.endpoint = Some(endpoint.clone());
            }

            if let Some(access_key) = query_pairs.get("access_key") {
                s3_config.access_key = Some(access_key.clone());
            }

            if let Some(secret_key) = query_pairs.get("secret_key") {
                s3_config.secret_key = Some(secret_key.clone());
            }

            if let Some(path_style) = query_pairs.get("path_style") {
                s3_config.force_path_style = path_style == "true";
            }

            let config = if prefix.is_empty() {
                BackendConfig::S3 {
                    config: s3_config,
                    prefix: None,
                }
            } else {
                BackendConfig::S3 {
                    config: s3_config,
                    prefix: Some(prefix),
                }
            };

            Ok((config, path))
        }

        #[cfg(feature = "smb-native")]
        "smb" | "cifs" => {
            use crate::protocols::smb::SmbSecurity;

            let host = url
                .host_str()
                .ok_or_else(|| BackendError::InvalidConfig {
                    backend: "smb".to_string(),
                    message: "Missing host in SMB URI".to_string(),
                })?
                .to_string();

            let port = url.port();

            // Parse path: first segment is share, rest is subpath
            let path_segments: Vec<&str> = url
                .path()
                .trim_start_matches('/')
                .split('/')
                .filter(|s| !s.is_empty())
                .collect();

            if path_segments.is_empty() {
                return Err(BackendError::InvalidConfig {
                    backend: "smb".to_string(),
                    message: "Missing share name in SMB URI".to_string(),
                });
            }

            let share = path_segments[0].to_string();
            let subpath = if path_segments.len() > 1 {
                Some(path_segments[1..].join("/"))
            } else {
                None
            };

            // Extract credentials from URI
            let username = if !url.username().is_empty() {
                Some(url.username().to_string())
            } else {
                None
            };

            let password = url.password().map(|p| p.to_string());

            // Parse query parameters
            let query_pairs: HashMap<String, String> = url
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            // Determine security setting
            let security = match query_pairs.get("security").map(|s| s.as_str()) {
                Some("require_encryption") => SmbSecurity::RequireEncryption,
                Some("sign_only") => SmbSecurity::SignOnly,
                Some("opportunistic") | None => SmbSecurity::Opportunistic,
                Some(other) => {
                    return Err(BackendError::InvalidConfig {
                        backend: "smb".to_string(),
                        message: format!("Invalid security setting: {}", other),
                    });
                }
            };

            // Build config
            let mut smb_config = SmbConfig::new(host, share).with_security(security);

            if let Some(port) = port {
                smb_config = smb_config.with_port(port);
            }

            if let Some(username) = username {
                smb_config = smb_config.with_username(username);
            }

            if let Some(password) = password {
                smb_config = smb_config.with_password(password);
            }

            if let Some(subpath) = &subpath {
                smb_config = smb_config.with_subpath(subpath.clone());
            }

            let path = PathBuf::from(subpath.unwrap_or_default());

            Ok((BackendConfig::Smb(smb_config), path))
        }

        _ => Err(BackendError::InvalidConfig {
            backend: scheme.to_string(),
            message: format!("Unsupported URI scheme: {}", scheme),
        }),
    }
}

/// Parse backend configuration from environment variables
///
/// Looks for variables like:
/// - `ORBIT_BACKEND_TYPE` - Backend type (local, ssh, s3, smb)
/// - `ORBIT_SSH_HOST`, `ORBIT_SSH_USER`, `ORBIT_SSH_KEY` - SSH config
/// - `ORBIT_S3_BUCKET`, `ORBIT_S3_REGION`, `ORBIT_S3_ENDPOINT` - S3 config
/// - `ORBIT_SMB_HOST`, `ORBIT_SMB_SHARE`, `ORBIT_SMB_USER`, `ORBIT_SMB_PASSWORD` - SMB config
pub fn from_env() -> BackendResult<BackendConfig> {
    let backend_type = std::env::var("ORBIT_BACKEND_TYPE")
        .unwrap_or_else(|_| "local".to_string())
        .to_lowercase();

    match backend_type.as_str() {
        "local" => {
            let root = std::env::var("ORBIT_LOCAL_ROOT").ok().map(PathBuf::from);
            Ok(BackendConfig::Local { root })
        }

        #[cfg(feature = "ssh-backend")]
        "ssh" => {
            let host =
                std::env::var("ORBIT_SSH_HOST").map_err(|_| BackendError::InvalidConfig {
                    backend: "ssh".to_string(),
                    message: "ORBIT_SSH_HOST not set".to_string(),
                })?;

            let username =
                std::env::var("ORBIT_SSH_USER").map_err(|_| BackendError::InvalidConfig {
                    backend: "ssh".to_string(),
                    message: "ORBIT_SSH_USER not set".to_string(),
                })?;

            let port = std::env::var("ORBIT_SSH_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(22);

            let auth = if let Ok(key_path) = std::env::var("ORBIT_SSH_KEY") {
                let passphrase = std::env::var("ORBIT_SSH_PASSPHRASE")
                    .ok()
                    .map(|p| secrecy::SecretString::new(p.into_boxed_str()));
                SshAuth::KeyFile {
                    key_path: PathBuf::from(key_path),
                    passphrase,
                }
            } else if let Ok(password) = std::env::var("ORBIT_SSH_PASSWORD") {
                SshAuth::Password(secrecy::SecretString::new(password.into_boxed_str()))
            } else {
                SshAuth::Agent
            };

            let config = SshConfig::new(host, username, auth).with_port(port);
            Ok(BackendConfig::Ssh(config))
        }

        #[cfg(feature = "s3-native")]
        "s3" => {
            let bucket =
                std::env::var("ORBIT_S3_BUCKET").map_err(|_| BackendError::InvalidConfig {
                    backend: "s3".to_string(),
                    message: "ORBIT_S3_BUCKET not set".to_string(),
                })?;

            let mut s3_config = S3Config::new(bucket);

            if let Ok(region) = std::env::var("ORBIT_S3_REGION") {
                s3_config.region = Some(region);
            }

            if let Ok(endpoint) = std::env::var("ORBIT_S3_ENDPOINT") {
                s3_config.endpoint = Some(endpoint);
            }

            if let Ok(access_key) = std::env::var("AWS_ACCESS_KEY_ID") {
                s3_config.access_key = Some(access_key);
            }

            if let Ok(secret_key) = std::env::var("AWS_SECRET_ACCESS_KEY") {
                s3_config.secret_key = Some(secret_key);
            }

            let prefix = std::env::var("ORBIT_S3_PREFIX").ok();

            Ok(BackendConfig::S3 {
                config: s3_config,
                prefix,
            })
        }

        #[cfg(feature = "smb-native")]
        "smb" => {
            use crate::protocols::smb::SmbSecurity;

            let host =
                std::env::var("ORBIT_SMB_HOST").map_err(|_| BackendError::InvalidConfig {
                    backend: "smb".to_string(),
                    message: "ORBIT_SMB_HOST not set".to_string(),
                })?;

            let share =
                std::env::var("ORBIT_SMB_SHARE").map_err(|_| BackendError::InvalidConfig {
                    backend: "smb".to_string(),
                    message: "ORBIT_SMB_SHARE not set".to_string(),
                })?;

            let mut smb_config = SmbConfig::new(host, share);

            if let Ok(port) = std::env::var("ORBIT_SMB_PORT") {
                if let Ok(port) = port.parse() {
                    smb_config = smb_config.with_port(port);
                }
            }

            if let Ok(username) = std::env::var("ORBIT_SMB_USER") {
                smb_config = smb_config.with_username(username);
            }

            if let Ok(password) = std::env::var("ORBIT_SMB_PASSWORD") {
                smb_config = smb_config.with_password(password);
            }

            if let Ok(subpath) = std::env::var("ORBIT_SMB_PATH") {
                smb_config = smb_config.with_subpath(subpath);
            }

            // Parse security setting
            let security = match std::env::var("ORBIT_SMB_SECURITY")
                .unwrap_or_default()
                .to_lowercase()
                .as_str()
            {
                "require_encryption" => SmbSecurity::RequireEncryption,
                "sign_only" => SmbSecurity::SignOnly,
                _ => SmbSecurity::Opportunistic,
            };
            smb_config = smb_config.with_security(security);

            Ok(BackendConfig::Smb(smb_config))
        }

        _ => Err(BackendError::InvalidConfig {
            backend: backend_type,
            message: "Unknown backend type".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_uri() {
        let (config, path) = parse_uri("/tmp/test").unwrap();
        assert!(matches!(config, BackendConfig::Local { .. }));
        assert_eq!(path, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_parse_file_uri() {
        let (config, path) = parse_uri("file:///tmp/test").unwrap();
        assert!(matches!(config, BackendConfig::Local { .. }));
        assert_eq!(path, PathBuf::from("/tmp/test"));
    }

    #[test]
    #[cfg(feature = "ssh-backend")]
    fn test_parse_ssh_uri() {
        let (config, path) = parse_uri("ssh://user@example.com:22/remote/path").unwrap();
        assert!(matches!(config, BackendConfig::Ssh(_)));
        assert_eq!(path, PathBuf::from("/remote/path"));
    }

    #[test]
    #[cfg(feature = "s3-native")]
    fn test_parse_s3_uri() {
        let (config, path) = parse_uri("s3://my-bucket/prefix/path?region=us-east-1").unwrap();
        if let BackendConfig::S3 { config, prefix } = config {
            assert_eq!(config.bucket, "my-bucket");
            assert_eq!(config.region, Some("us-east-1".to_string()));
            assert_eq!(prefix, Some("prefix/path".to_string()));
        } else {
            panic!("Expected S3 config");
        }
    }

    #[test]
    fn test_backend_type() {
        let config = BackendConfig::local();
        assert_eq!(config.backend_type(), "local");
    }

    #[test]
    #[cfg(feature = "smb-native")]
    fn test_parse_smb_uri() {
        let (config, path) = parse_uri("smb://user:pass@fileserver/share/path/to/file").unwrap();
        if let BackendConfig::Smb(smb_config) = config {
            assert_eq!(smb_config.host, "fileserver");
            assert_eq!(smb_config.share, "share");
            assert_eq!(smb_config.username, Some("user".to_string()));
            assert_eq!(smb_config.password, Some("pass".to_string()));
            assert_eq!(smb_config.subpath, Some("path/to/file".to_string()));
        } else {
            panic!("Expected SMB config");
        }
    }

    #[test]
    #[cfg(feature = "smb-native")]
    fn test_parse_smb_uri_with_security() {
        let (config, _) =
            parse_uri("smb://server/share?security=require_encryption").unwrap();
        if let BackendConfig::Smb(smb_config) = config {
            assert_eq!(smb_config.host, "server");
            assert_eq!(smb_config.share, "share");
            assert_eq!(
                smb_config.security,
                crate::protocols::smb::SmbSecurity::RequireEncryption
            );
        } else {
            panic!("Expected SMB config");
        }
    }

    #[test]
    #[cfg(feature = "smb-native")]
    fn test_parse_smb_uri_with_port() {
        let (config, _) = parse_uri("smb://server:8445/share").unwrap();
        if let BackendConfig::Smb(smb_config) = config {
            assert_eq!(smb_config.host, "server");
            assert_eq!(smb_config.port, Some(8445));
            assert_eq!(smb_config.share, "share");
        } else {
            panic!("Expected SMB config");
        }
    }

    #[test]
    #[cfg(feature = "smb-native")]
    fn test_smb_backend_type() {
        let config = BackendConfig::smb(SmbConfig::new("server", "share"));
        assert_eq!(config.backend_type(), "smb");
    }
}
