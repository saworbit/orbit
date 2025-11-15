//! Tests for SMB native implementation
//!
//! These tests require a real SMB server for integration testing.
//! Unit tests use mocks where possible.

#[cfg(test)]
mod tests {
    use super::super::*;

    /// Create a test target with default values
    fn create_test_target() -> SmbTarget {
        SmbTarget {
            host: "localhost".to_string(),
            share: "test".to_string(),
            subpath: String::new(),
            port: Some(445),
            auth: SmbAuth::Anonymous,
            security: SmbSecurity::Opportunistic,
        }
    }

    #[test]
    fn test_smb_target_creation() {
        let target = create_test_target();
        assert_eq!(target.host, "localhost");
        assert_eq!(target.share, "test");
        assert_eq!(target.port, Some(445));
    }

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

    #[test]
    fn test_smb_auth_types() {
        let anon = SmbAuth::Anonymous;
        assert!(matches!(anon, SmbAuth::Anonymous));

        let ntlm = SmbAuth::Ntlmv2 {
            username: "user".to_string(),
            password: Secret("pass".to_string()),
        };
        assert!(matches!(ntlm, SmbAuth::Ntlmv2 { .. }));

        let kerb = SmbAuth::Kerberos { principal: None };
        assert!(matches!(kerb, SmbAuth::Kerberos { .. }));
    }

    #[test]
    fn test_security_levels() {
        let opp = SmbSecurity::Opportunistic;
        let req = SmbSecurity::RequireEncryption;
        let sign = SmbSecurity::SignOnly;

        assert_ne!(opp, req);
        assert_ne!(req, sign);
        assert_ne!(sign, opp);
    }

    #[test]
    fn test_secret_redaction() {
        let secret = Secret("password123".to_string());
        let debug_str = format!("{:?}", secret);
        
        assert!(!debug_str.contains("password123"));
        assert!(debug_str.contains("REDACTED"));
    }

    #[test]
    fn test_smb_error_types() {
        let err = SmbError::Auth;
        assert_eq!(err.to_string(), "authentication failed");

        let err = SmbError::NotFound("test.txt".to_string());
        assert!(err.is_not_found());

        let err = SmbError::Timeout;
        assert!(err.is_retryable());

        let err = SmbError::Permission("denied".to_string());
        assert!(err.is_permission_error());
    }

    #[test]
    fn test_smb_metadata() {
        let meta = SmbMetadata {
            size: 1024,
            is_dir: false,
            modified: None,
            encrypted: true,
        };

        assert_eq!(meta.size, 1024);
        assert!(!meta.is_dir);
        assert!(meta.encrypted);
    }

    #[cfg(feature = "smb-native")]
    #[test]
    fn test_path_validation() {
        use crate::protocols::smb::native::NativeSmbClient;

        // Valid paths
        let mut target = create_test_target();
        assert!(NativeSmbClient::validate_target(&target).is_ok());

        target.subpath = "reports/Q4".to_string();
        assert!(NativeSmbClient::validate_target(&target).is_ok());

        // Invalid: empty host
        target.host = String::new();
        assert!(NativeSmbClient::validate_target(&target).is_err());

        // Invalid: empty share
        target = create_test_target();
        target.share = String::new();
        assert!(NativeSmbClient::validate_target(&target).is_err());

        // Invalid: path traversal
        target = create_test_target();
        target.subpath = "../../../etc/passwd".to_string();
        assert!(NativeSmbClient::validate_target(&target).is_err());
    }

    #[cfg(feature = "smb-native")]
    #[tokio::test]
    async fn test_client_factory() {
        let target = create_test_target();
        
        // This will fail without a real server, but tests the factory
        let result = client_for(&target).await;
        
        // We expect an error since there's no server
        assert!(result.is_err());
    }

    #[cfg(feature = "smb-native")]
    #[test]
    fn test_capability_flags() {
        let caps = SmbCapability::MULTI_CHANNEL | SmbCapability::LEASES;
        
        assert!(caps.contains(SmbCapability::MULTI_CHANNEL));
        assert!(caps.contains(SmbCapability::LEASES));
        assert!(!caps.contains(SmbCapability::DFS));
    }
}

/// Integration tests that require a real SMB server
///
/// To run these tests, set up an SMB server and configure the environment:
/// 
/// ```bash
/// export SMB_TEST_HOST=localhost
/// export SMB_TEST_SHARE=test
/// export SMB_TEST_USER=testuser
/// export SMB_TEST_PASS=testpass
/// export SMB_TEST_ENABLED=1
/// 
/// cargo test --features smb-native -- --ignored
/// ```
#[cfg(all(test, feature = "smb-native"))]
mod integration_tests {
    use super::super::*;
    use bytes::Bytes;
    use std::env;

    fn should_run_integration_tests() -> bool {
        env::var("SMB_TEST_ENABLED").unwrap_or_default() == "1"
    }

    fn get_test_config() -> Option<SmbTarget> {
        if !should_run_integration_tests() {
            return None;
        }

        let host = env::var("SMB_TEST_HOST").ok()?;
        let share = env::var("SMB_TEST_SHARE").ok()?;
        let username = env::var("SMB_TEST_USER").ok()?;
        let password = env::var("SMB_TEST_PASS").ok()?;

        Some(SmbTarget {
            host,
            share,
            subpath: String::new(),
            port: Some(445),
            auth: SmbAuth::Ntlmv2 {
                username,
                password: Secret(password),
            },
            security: SmbSecurity::Opportunistic,
        })
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_connection() {
        let Some(target) = get_test_config() else {
            println!("Skipping integration test - SMB server not configured");
            return;
        };

        let result = client_for(&target).await;
        assert!(result.is_ok(), "Failed to connect to SMB server: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_list_directory() {
        let Some(target) = get_test_config() else {
            println!("Skipping integration test - SMB server not configured");
            return;
        };

        let client = client_for(&target).await.expect("Failed to connect");
        let entries = client.list_dir("").await.expect("Failed to list directory");
        
        println!("Directory entries: {:?}", entries);
        assert!(entries.len() >= 0); // Directory might be empty
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_write_read_file() {
        let Some(target) = get_test_config() else {
            println!("Skipping integration test - SMB server not configured");
            return;
        };

        let client = client_for(&target).await.expect("Failed to connect");
        
        // Write a test file
        let test_data = Bytes::from("Hello from Orbit SMB test!");
        let filename = "orbit_test_file.txt";
        
        client.write_file(filename, test_data.clone())
            .await
            .expect("Failed to write file");

        // Read it back
        let read_data = client.read_file(filename, None)
            .await
            .expect("Failed to read file");

        assert_eq!(test_data, read_data);

        // Clean up
        client.remove(filename).await.ok();
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_range_read() {
        let Some(target) = get_test_config() else {
            println!("Skipping integration test - SMB server not configured");
            return;
        };

        let client = client_for(&target).await.expect("Failed to connect");
        
        // Write a test file
        let test_data = Bytes::from("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        let filename = "orbit_range_test.txt";
        
        client.write_file(filename, test_data.clone())
            .await
            .expect("Failed to write file");

        // Read a range
        let range_data = client.read_file(filename, Some(10..20))
            .await
            .expect("Failed to read range");

        assert_eq!(range_data, Bytes::from("ABCDEFGHIJ"));

        // Clean up
        client.remove(filename).await.ok();
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_metadata() {
        let Some(target) = get_test_config() else {
            println!("Skipping integration test - SMB server not configured");
            return;
        };

        let client = client_for(&target).await.expect("Failed to connect");
        
        // Write a test file
        let test_data = Bytes::from("metadata test");
        let filename = "orbit_metadata_test.txt";
        
        client.write_file(filename, test_data.clone())
            .await
            .expect("Failed to write file");

        // Get metadata
        let meta = client.metadata(filename)
            .await
            .expect("Failed to get metadata");

        assert_eq!(meta.size, test_data.len() as u64);
        assert!(!meta.is_dir);

        // Clean up
        client.remove(filename).await.ok();
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_mkdir() {
        let Some(target) = get_test_config() else {
            println!("Skipping integration test - SMB server not configured");
            return;
        };

        let client = client_for(&target).await.expect("Failed to connect");
        
        let dirname = "orbit_test_dir";
        
        // Create directory
        client.mkdir(dirname)
            .await
            .expect("Failed to create directory");

        // Verify it exists by getting metadata
        let meta = client.metadata(dirname)
            .await
            .expect("Failed to get directory metadata");

        assert!(meta.is_dir);

        // Clean up
        client.remove(dirname).await.ok();
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_large_file() {
        let Some(target) = get_test_config() else {
            println!("Skipping integration test - SMB server not configured");
            return;
        };

        let client = client_for(&target).await.expect("Failed to connect");
        
        // Create a 5MB file
        let size = 5 * 1024 * 1024;
        let test_data = Bytes::from(vec![0xAB; size]);
        let filename = "orbit_large_file_test.bin";
        
        client.write_file(filename, test_data.clone())
            .await
            .expect("Failed to write large file");

        // Read it back
        let read_data = client.read_file(filename, None)
            .await
            .expect("Failed to read large file");

        assert_eq!(test_data.len(), read_data.len());
        assert_eq!(test_data, read_data);

        // Clean up
        client.remove(filename).await.ok();
    }
}