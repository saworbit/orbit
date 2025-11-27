//! SSH/SFTP Backend Integration Tests
//!
//! These tests verify the SSH backend implementation.
//!
//! ## Running Tests
//!
//! Basic tests (no SSH server required):
//! ```bash
//! cargo test --features ssh-backend --test ssh_backend_test
//! ```
//!
//! Integration tests (requires SSH server):
//! ```bash
//! cargo test --features ssh-backend --test ssh_backend_test -- --ignored
//! ```

#[cfg(feature = "ssh-backend")]
mod ssh_tests {
    use orbit::backend::{Backend, BackendConfig, SshAuth, SshConfig};
    use secrecy::SecretString;
    use std::path::Path;

    /// Test SSH config creation
    #[test]
    fn test_ssh_config_creation() {
        let config = SshConfig::new("example.com", "testuser", SshAuth::Agent);

        assert_eq!(config.host, "example.com");
        assert_eq!(config.username, "testuser");
        assert_eq!(config.port, 22);
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.compress, false);
    }

    /// Test SSH config builder pattern
    #[test]
    fn test_ssh_config_builder() {
        let config = SshConfig::new("example.com", "testuser", SshAuth::Agent)
            .with_port(2222)
            .with_timeout(60)
            .with_compression();

        assert_eq!(config.port, 2222);
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.compress, true);
    }

    /// Test SSH config from URI format
    #[test]
    fn test_ssh_config_from_uri() {
        let config = SshConfig::from_uri("user@server.com:2222", SshAuth::Agent);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.host, "server.com");
        assert_eq!(config.username, "user");
        assert_eq!(config.port, 2222);
    }

    /// Test SSH config from URI without port
    #[test]
    fn test_ssh_config_from_uri_default_port() {
        let config = SshConfig::from_uri("user@server.com", SshAuth::Agent);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.port, 22);
    }

    /// Test SSH auth variants
    #[test]
    fn test_ssh_auth_variants() {
        // Password auth
        let password_auth = SshAuth::Password(SecretString::new("secret".into()));
        assert!(matches!(password_auth, SshAuth::Password(_)));

        // Key file auth
        let key_auth = SshAuth::KeyFile {
            key_path: std::path::PathBuf::from("/path/to/key"),
            passphrase: None,
        };
        assert!(matches!(key_auth, SshAuth::KeyFile { .. }));

        // Agent auth
        let agent_auth = SshAuth::Agent;
        assert!(matches!(agent_auth, SshAuth::Agent));
    }

    /// Test backend config SSH variant
    #[test]
    fn test_backend_config_ssh() {
        let ssh_config = SshConfig::new("example.com", "user", SshAuth::Agent);
        let backend_config = BackendConfig::ssh(ssh_config.clone());

        assert_eq!(backend_config.backend_type(), "ssh");

        match backend_config {
            BackendConfig::Ssh(config) => {
                assert_eq!(config.host, "example.com");
                assert_eq!(config.username, "user");
            }
            _ => panic!("Expected SSH config"),
        }
    }

    /// Test URI parsing for SSH protocol
    #[test]
    #[cfg(feature = "backend-abstraction")]
    fn test_parse_ssh_uri() {
        let result = orbit::backend::parse_uri("ssh://user@example.com:22/remote/path");
        assert!(result.is_ok());

        let (config, path) = result.unwrap();
        assert_eq!(config.backend_type(), "ssh");
        assert_eq!(path, std::path::PathBuf::from("/remote/path"));

        match config {
            BackendConfig::Ssh(ssh_config) => {
                assert_eq!(ssh_config.host, "example.com");
                assert_eq!(ssh_config.username, "user");
                assert_eq!(ssh_config.port, 22);
            }
            _ => panic!("Expected SSH config"),
        }
    }

    /// Test URI parsing for SFTP protocol
    #[test]
    #[cfg(feature = "backend-abstraction")]
    fn test_parse_sftp_uri() {
        let result = orbit::backend::parse_uri("sftp://user@example.com/remote/path");
        assert!(result.is_ok());

        let (config, path) = result.unwrap();
        assert_eq!(config.backend_type(), "ssh");
        assert_eq!(path, std::path::PathBuf::from("/remote/path"));
    }

    /// Test URI parsing with query parameters
    #[test]
    #[cfg(feature = "backend-abstraction")]
    fn test_parse_ssh_uri_with_key() {
        let result =
            orbit::backend::parse_uri("ssh://user@example.com/path?key=/home/user/.ssh/id_rsa");
        assert!(result.is_ok());

        let (config, _) = result.unwrap();
        match config {
            BackendConfig::Ssh(ssh_config) => {
                assert!(matches!(ssh_config.auth, SshAuth::KeyFile { .. }));
            }
            _ => panic!("Expected SSH config"),
        }
    }

    /// Test URI parsing with agent parameter
    #[test]
    #[cfg(feature = "backend-abstraction")]
    fn test_parse_ssh_uri_with_agent() {
        let result = orbit::backend::parse_uri("ssh://user@example.com/path?agent=true");
        assert!(result.is_ok());

        let (config, _) = result.unwrap();
        match config {
            BackendConfig::Ssh(ssh_config) => {
                assert!(matches!(ssh_config.auth, SshAuth::Agent));
            }
            _ => panic!("Expected SSH config"),
        }
    }

    // =====================================================
    // INTEGRATION TESTS (Require SSH Server)
    // =====================================================

    /// Integration test: Connect to SSH server (requires live SSH server)
    ///
    /// To run: Set up SSH server and run with --ignored flag
    /// ```bash
    /// cargo test --features ssh-backend test_ssh_connection -- --ignored
    /// ```
    #[test]
    #[ignore] // Requires SSH server
    fn test_ssh_connection() {
        // This test requires a live SSH server
        // You can set up a local SSH server or use testcontainers

        // Example configuration - adjust for your environment
        let host = std::env::var("TEST_SSH_HOST").unwrap_or_else(|_| "localhost".to_string());
        let user = std::env::var("TEST_SSH_USER").unwrap_or_else(|_| "testuser".to_string());

        let config = SshConfig::new(host, user, SshAuth::Agent);

        // Attempt connection in async context
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let result = orbit::backend::SshBackend::connect(config).await;

            // Should succeed if SSH server is properly configured
            assert!(result.is_ok(), "SSH connection failed: {:?}", result.err());
        });
    }

    /// Integration test: File operations over SSH
    #[test]
    #[ignore] // Requires SSH server
    fn test_ssh_file_operations() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let host = std::env::var("TEST_SSH_HOST").unwrap_or_else(|_| "localhost".to_string());
            let user = std::env::var("TEST_SSH_USER").unwrap_or_else(|_| "testuser".to_string());

            let config = SshConfig::new(host, user, SshAuth::Agent);
            let backend = orbit::backend::SshBackend::connect(config)
                .await
                .expect("Failed to connect to SSH server");

            // Test stat operation
            let path = Path::new("/tmp/test_file.txt");
            let metadata_result = backend.stat(path).await;

            // Note: This will fail if file doesn't exist, which is expected
            // In a real test, you'd create the file first
            println!("Stat result: {:?}", metadata_result);

            assert_eq!(backend.backend_name(), "ssh");
        });
    }

    // =====================================================
    // FUTURE: TESTCONTAINERS INTEGRATION
    // =====================================================

    // TODO: Implement testcontainers-based integration tests
    //
    // Example skeleton:
    //
    // #[tokio::test]
    // #[cfg(feature = "testcontainers")]
    // async fn test_ssh_with_docker() {
    //     use testcontainers::{clients, images::generic::GenericImage};
    //
    //     let docker = clients::Cli::default();
    //
    //     // Use a lightweight SSH server image
    //     let image = GenericImage::new("lscr.io/linuxserver/openssh-server", "latest")
    //         .with_env_var("USER_NAME", "orbit")
    //         .with_env_var("USER_PASSWORD", "orbit123")
    //         .with_mapped_port(2222, 22);
    //
    //     let container = docker.run(image);
    //     let port = container.get_host_port_ipv4(2222);
    //
    //     let config = SshConfig::new(
    //         "127.0.0.1",
    //         "orbit",
    //         SshAuth::Password(SecretString::new("orbit123".into()))
    //     ).with_port(port);
    //
    //     let backend = orbit::backend::SshBackend::connect(config).await
    //         .expect("Failed to connect");
    //
    //     // Test operations
    //     assert_eq!(backend.backend_name(), "ssh");
    // }
}

#[cfg(not(feature = "ssh-backend"))]
mod no_ssh {
    #[test]
    fn test_ssh_feature_disabled() {
        // This test ensures the test file compiles even without ssh-backend feature
        assert!(true, "SSH backend feature is disabled");
    }
}
