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

    /// Get backend type name
    pub fn backend_type(&self) -> &'static str {
        match self {
            Self::Local { .. } => "local",
            #[cfg(feature = "ssh-backend")]
            Self::Ssh(_) => "ssh",
            #[cfg(feature = "s3-native")]
            Self::S3 { .. } => "s3",
        }
    }
}

/// Parse a URI into backend configuration
///
/// Supported URI formats:
/// - `file:///path/to/dir` or `/path/to/dir` - Local filesystem
/// - `ssh://user@host:port/path` - SSH/SFTP (requires ssh-backend feature)
/// - `s3://bucket/prefix?region=us-east-1&endpoint=...` - S3 (requires s3-native feature)
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

        _ => Err(BackendError::InvalidConfig {
            backend: scheme.to_string(),
            message: format!("Unsupported URI scheme: {}", scheme),
        }),
    }
}

/// Parse backend configuration from environment variables
///
/// Looks for variables like:
/// - `ORBIT_BACKEND_TYPE` - Backend type (local, ssh, s3)
/// - `ORBIT_SSH_HOST`, `ORBIT_SSH_USER`, `ORBIT_SSH_KEY` - SSH config
/// - `ORBIT_S3_BUCKET`, `ORBIT_S3_REGION`, `ORBIT_S3_ENDPOINT` - S3 config
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
}
