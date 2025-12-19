//! Backend registry and factory for extensibility
//!
//! Provides a registry system for backend implementations with support for
//! custom backends and plugin-style architecture.

use super::error::{BackendError, BackendResult};
use super::{Backend, BackendConfig};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[cfg(feature = "backend-abstraction")]
use super::LocalBackend;

#[cfg(all(feature = "backend-abstraction", feature = "ssh-backend"))]
use super::SshBackend;

#[cfg(all(feature = "backend-abstraction", feature = "s3-native"))]
use super::S3Backend;

#[cfg(all(feature = "backend-abstraction", feature = "smb-native"))]
use super::SmbBackend;

#[cfg(all(feature = "backend-abstraction", feature = "azure-native"))]
use super::AzureBackend;

/// Factory function type for creating backends
pub type BackendFactory =
    Arc<dyn Fn(&BackendConfig) -> BoxFuture<BackendResult<Box<dyn Backend>>> + Send + Sync>;

/// Box future for async factory functions
pub type BoxFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>;

/// Backend registry for managing backend implementations
///
/// This registry allows registration of custom backend factories,
/// enabling plugin-style extensibility for Orbit.
///
/// # Example
///
/// ```no_run
/// use orbit::backend::{BackendRegistry, BackendConfig};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut registry = BackendRegistry::new();
///
///     // Register custom backend
///     // registry.register("custom", Arc::new(|config| {
///     //     Box::pin(async move {
///     //         // Create custom backend
///     //         Ok(Box::new(CustomBackend::new()) as Box<dyn Backend>)
///     //     })
///     // }));
///
///     // Create backend from config
///     let config = BackendConfig::local();
///     let backend = registry.create(&config).await?;
///
///     Ok(())
/// }
/// ```
pub struct BackendRegistry {
    factories: RwLock<HashMap<String, BackendFactory>>,
}

impl BackendRegistry {
    /// Create a new backend registry with default implementations
    pub fn new() -> Self {
        let registry = Self {
            factories: RwLock::new(HashMap::new()),
        };

        // Register built-in backends
        registry.register_builtin();

        registry
    }

    /// Register built-in backend factories
    fn register_builtin(&self) {
        // Local filesystem backend
        self.register(
            "local",
            Arc::new(|config| {
                let config = config.clone();
                Box::pin(async move {
                    match config {
                        BackendConfig::Local { root } => {
                            let backend = if let Some(root) = root {
                                LocalBackend::with_root(root)
                            } else {
                                LocalBackend::new()
                            };
                            Ok(Box::new(backend) as Box<dyn Backend>)
                        }
                        _ => Err(BackendError::InvalidConfig {
                            backend: "local".to_string(),
                            message: "Invalid configuration for local backend".to_string(),
                        }),
                    }
                })
            }),
        );

        // SSH backend
        #[cfg(feature = "ssh-backend")]
        self.register(
            "ssh",
            Arc::new(|config| {
                let config = config.clone();
                Box::pin(async move {
                    match config {
                        BackendConfig::Ssh(ssh_config) => {
                            let backend = SshBackend::connect(ssh_config).await?;
                            Ok(Box::new(backend) as Box<dyn Backend>)
                        }
                        _ => Err(BackendError::InvalidConfig {
                            backend: "ssh".to_string(),
                            message: "Invalid configuration for SSH backend".to_string(),
                        }),
                    }
                })
            }),
        );

        // S3 backend
        #[cfg(feature = "s3-native")]
        self.register(
            "s3",
            Arc::new(|config| {
                let config = config.clone();
                Box::pin(async move {
                    match config {
                        BackendConfig::S3 { config, prefix } => {
                            let backend = if let Some(prefix) = prefix {
                                S3Backend::with_prefix(config, prefix).await?
                            } else {
                                S3Backend::new(config).await?
                            };
                            Ok(Box::new(backend) as Box<dyn Backend>)
                        }
                        _ => Err(BackendError::InvalidConfig {
                            backend: "s3".to_string(),
                            message: "Invalid configuration for S3 backend".to_string(),
                        }),
                    }
                })
            }),
        );

        // SMB backend
        #[cfg(feature = "smb-native")]
        self.register(
            "smb",
            Arc::new(|config| {
                let config = config.clone();
                Box::pin(async move {
                    match config {
                        BackendConfig::Smb(smb_config) => {
                            let backend = SmbBackend::new(smb_config).await?;
                            Ok(Box::new(backend) as Box<dyn Backend>)
                        }
                        _ => Err(BackendError::InvalidConfig {
                            backend: "smb".to_string(),
                            message: "Invalid configuration for SMB backend".to_string(),
                        }),
                    }
                })
            }),
        );

        // Azure Blob Storage backend
        #[cfg(feature = "azure-native")]
        self.register(
            "azure",
            Arc::new(|config| {
                let config = config.clone();
                Box::pin(async move {
                    match config {
                        BackendConfig::Azure { config, prefix } => {
                            let backend = if let Some(prefix) = prefix {
                                AzureBackend::with_prefix(&config.container, prefix).await?
                            } else {
                                AzureBackend::new(&config.container).await?
                            };
                            Ok(Box::new(backend) as Box<dyn Backend>)
                        }
                        _ => Err(BackendError::InvalidConfig {
                            backend: "azure".to_string(),
                            message: "Invalid configuration for Azure backend".to_string(),
                        }),
                    }
                })
            }),
        );
    }

    /// Register a custom backend factory
    ///
    /// # Arguments
    ///
    /// * `name` - Backend name/identifier
    /// * `factory` - Factory function that creates the backend
    ///
    /// # Example
    ///
    /// ```ignore
    /// registry.register("mybackend", Arc::new(|config| {
    ///     Box::pin(async move {
    ///         let backend = MyBackend::new(config).await?;
    ///         Ok(Box::new(backend) as Box<dyn Backend>)
    ///     })
    /// }));
    /// ```
    pub fn register(&self, name: impl Into<String>, factory: BackendFactory) {
        let mut factories = self.factories.write().unwrap();
        factories.insert(name.into(), factory);
    }

    /// Unregister a backend
    pub fn unregister(&self, name: &str) -> bool {
        let mut factories = self.factories.write().unwrap();
        factories.remove(name).is_some()
    }

    /// Check if a backend is registered
    pub fn is_registered(&self, name: &str) -> bool {
        let factories = self.factories.read().unwrap();
        factories.contains_key(name)
    }

    /// List all registered backend names
    pub fn list_backends(&self) -> Vec<String> {
        let factories = self.factories.read().unwrap();
        factories.keys().cloned().collect()
    }

    /// Create a backend from configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Backend configuration
    ///
    /// # Returns
    ///
    /// Backend instance
    ///
    /// # Errors
    ///
    /// Returns error if backend type is not registered or creation fails
    pub async fn create(&self, config: &BackendConfig) -> BackendResult<Box<dyn Backend>> {
        let backend_type = config.backend_type();

        let factory = {
            let factories = self.factories.read().unwrap();
            factories.get(backend_type).cloned()
        };

        match factory {
            Some(factory) => factory(config).await,
            None => Err(BackendError::InvalidConfig {
                backend: backend_type.to_string(),
                message: format!("Backend type '{}' not registered", backend_type),
            }),
        }
    }

    /// Create a backend from URI
    ///
    /// # Arguments
    ///
    /// * `uri` - URI string (e.g., "s3://bucket/prefix", "ssh://user@host/path")
    ///
    /// # Returns
    ///
    /// Tuple of (Backend instance, base path)
    pub async fn create_from_uri(
        &self,
        uri: &str,
    ) -> BackendResult<(Box<dyn Backend>, std::path::PathBuf)> {
        let (config, path) = super::config::parse_uri(uri)?;
        let backend = self.create(&config).await?;
        Ok((backend, path))
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global backend registry instance
#[allow(dead_code)]
static GLOBAL_REGISTRY: once_cell::sync::Lazy<BackendRegistry> =
    once_cell::sync::Lazy::new(BackendRegistry::new);

/// Get the global backend registry
///
/// This provides a singleton registry that can be used throughout the application.
///
/// # Example
///
/// ```ignore
/// use orbit::backend::global_registry;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let registry = global_registry();
///
///     // List available backends
///     let backends = registry.list_backends();
///     println!("Available backends: {:?}", backends);
///
///     Ok(())
/// }
/// ```
#[allow(dead_code)]
pub fn global_registry() -> &'static BackendRegistry {
    &GLOBAL_REGISTRY
}

/// Convenience function to create a backend from URI using the global registry
///
/// # Example
///
/// ```ignore
/// use orbit::backend::create_backend_from_uri;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let (backend, path) = create_backend_from_uri("file:///tmp/data").await?;
///     println!("Created backend for path: {}", path.display());
///     Ok(())
/// }
/// ```
#[allow(dead_code)]
pub async fn create_backend_from_uri(
    uri: &str,
) -> BackendResult<(Box<dyn Backend>, std::path::PathBuf)> {
    global_registry().create_from_uri(uri).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = BackendRegistry::new();
        let backends = registry.list_backends();

        // Should have at least local backend registered
        assert!(backends.contains(&"local".to_string()));
    }

    #[test]
    fn test_backend_registered() {
        let registry = BackendRegistry::new();
        assert!(registry.is_registered("local"));
    }

    #[tokio::test]
    async fn test_create_local_backend() {
        let registry = BackendRegistry::new();
        let config = BackendConfig::local();
        let backend = registry.create(&config).await;

        assert!(backend.is_ok());
    }

    #[tokio::test]
    async fn test_create_from_uri() {
        let registry = BackendRegistry::new();
        let result = registry.create_from_uri("file:///tmp/test").await;

        assert!(result.is_ok());
        if let Ok((backend, path)) = result {
            assert_eq!(backend.backend_name(), "local");
            assert_eq!(path, std::path::PathBuf::from("/tmp/test"));
        }
    }

    #[test]
    fn test_global_registry() {
        let registry = global_registry();
        assert!(registry.is_registered("local"));
    }
}
