//! Integration tests for S3 protocol
//!
//! These tests require a running S3-compatible service (AWS S3, MinIO, LocalStack, etc.)
//! Set the following environment variables to run tests:
//!
//! - `S3_TEST_BUCKET`: Bucket name for testing
//! - `S3_TEST_REGION`: AWS region (default: us-east-1)
//! - `S3_TEST_ENDPOINT`: Custom endpoint for MinIO/LocalStack (optional)
//! - `AWS_ACCESS_KEY_ID`: Access key (optional, uses credential chain if not set)
//! - `AWS_SECRET_ACCESS_KEY`: Secret key (optional, uses credential chain if not set)
//! - `S3_TESTS_ENABLED`: Set to "1" to enable integration tests

use super::*;
use bytes::Bytes;
use std::env;

/// Check if S3 integration tests should run
fn s3_tests_enabled() -> bool {
    env::var("S3_TESTS_ENABLED").unwrap_or_default() == "1"
}

/// Get test configuration from environment
fn get_test_config() -> S3Config {
    let bucket = env::var("S3_TEST_BUCKET").unwrap_or_else(|_| "orbit-test-bucket".to_string());

    let region = env::var("S3_TEST_REGION").ok();

    let endpoint = env::var("S3_TEST_ENDPOINT").ok();

    let access_key = env::var("AWS_ACCESS_KEY_ID").ok();

    let secret_key = env::var("AWS_SECRET_ACCESS_KEY").ok();

    let mut config = S3Config::new(bucket);
    config.region = region;
    config.endpoint = endpoint;
    config.access_key = access_key;
    config.secret_key = secret_key;

    // Use path-style for MinIO/LocalStack
    if config.endpoint.is_some() {
        config.force_path_style = true;
    }

    config
}

#[tokio::test]
#[ignore]
async fn test_connection() {
    if !s3_tests_enabled() {
        println!("Skipping S3 integration test - set S3_TESTS_ENABLED=1 to run");
        return;
    }

    let config = get_test_config();
    let client = S3Client::new(config)
        .await
        .expect("Failed to create client");

    client
        .test_connection()
        .await
        .expect("Failed to connect to S3");
}

#[tokio::test]
#[ignore]
async fn test_upload_download_small_file() {
    if !s3_tests_enabled() {
        return;
    }

    let config = get_test_config();
    let client = S3Client::new(config)
        .await
        .expect("Failed to create client");

    // Upload test data
    let test_data = b"Hello, S3!";
    let key = "test/small-file.txt";

    client
        .upload_bytes(Bytes::from(test_data.to_vec()), key)
        .await
        .expect("Failed to upload");

    // Verify exists
    assert!(client.exists(key).await.expect("Failed to check existence"));

    // Download and verify
    let downloaded = client
        .download_bytes(key)
        .await
        .expect("Failed to download");
    assert_eq!(&downloaded[..], test_data);

    // Cleanup
    client.delete(key).await.expect("Failed to delete");
}

#[tokio::test]
#[ignore]
async fn test_list_objects() {
    if !s3_tests_enabled() {
        return;
    }

    let config = get_test_config();
    let client = S3Client::new(config)
        .await
        .expect("Failed to create client");

    // Upload test objects
    let prefix = "test/list/";
    for i in 1..=5 {
        let key = format!("{}file{}.txt", prefix, i);
        client
            .upload_bytes(Bytes::from(format!("Content {}", i)), &key)
            .await
            .expect("Failed to upload");
    }

    // List objects
    let result = client.list_objects(prefix).await.expect("Failed to list");
    assert!(result.objects.len() >= 5);

    // Cleanup
    for obj in result.objects {
        client.delete(&obj.key).await.ok();
    }
}

#[tokio::test]
#[ignore]
async fn test_get_metadata() {
    if !s3_tests_enabled() {
        return;
    }

    let config = get_test_config();
    let client = S3Client::new(config)
        .await
        .expect("Failed to create client");

    // Upload test data
    let test_data = b"Metadata test";
    let key = "test/metadata.txt";

    client
        .upload_bytes(Bytes::from(test_data.to_vec()), key)
        .await
        .expect("Failed to upload");

    // Get metadata
    let metadata = client
        .get_metadata(key)
        .await
        .expect("Failed to get metadata");
    assert_eq!(metadata.size, test_data.len() as u64);
    assert_eq!(metadata.key, key);

    // Cleanup
    client.delete(key).await.expect("Failed to delete");
}

#[tokio::test]
#[ignore]
async fn test_copy_object() {
    if !s3_tests_enabled() {
        return;
    }

    let config = get_test_config();
    let client = S3Client::new(config)
        .await
        .expect("Failed to create client");

    // Upload source
    let test_data = b"Copy test";
    let source_key = "test/source.txt";
    let dest_key = "test/destination.txt";

    client
        .upload_bytes(Bytes::from(test_data.to_vec()), source_key)
        .await
        .expect("Failed to upload");

    // Copy
    client
        .copy_object(source_key, dest_key)
        .await
        .expect("Failed to copy");

    // Verify destination exists
    assert!(client
        .exists(dest_key)
        .await
        .expect("Failed to check existence"));

    // Cleanup
    client.delete(source_key).await.ok();
    client.delete(dest_key).await.ok();
}

#[tokio::test]
#[ignore]
async fn test_object_not_found() {
    if !s3_tests_enabled() {
        return;
    }

    let config = get_test_config();
    let client = S3Client::new(config)
        .await
        .expect("Failed to create client");

    let key = "test/nonexistent.txt";

    // Should return false for exists
    assert!(!client.exists(key).await.expect("Failed to check existence"));

    // Should return error for download
    let result = client.download_bytes(key).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), S3Error::NotFound { .. }));
}

#[tokio::test]
#[ignore]
async fn test_delete_nonexistent() {
    if !s3_tests_enabled() {
        return;
    }

    let config = get_test_config();
    let client = S3Client::new(config)
        .await
        .expect("Failed to create client");

    let key = "test/nonexistent-delete.txt";

    // Deleting non-existent object should succeed (S3 is idempotent)
    client
        .delete(key)
        .await
        .expect("Failed to delete non-existent");
}

#[cfg(test)]
mod unit_tests {
    use super::super::*;

    #[test]
    fn test_config_builder() {
        let config = S3ConfigBuilder::new("test-bucket".to_string())
            .region("us-west-2".to_string())
            .chunk_size(10 * 1024 * 1024)
            .build()
            .unwrap();

        assert_eq!(config.bucket, "test-bucket");
        assert_eq!(config.region, Some("us-west-2".to_string()));
    }

    #[test]
    fn test_storage_class_conversions() {
        let sc = S3StorageClass::Standard;
        let aws_sc = sc.to_aws();
        let back = S3StorageClass::from_aws(&aws_sc);
        assert_eq!(sc, back);
    }

    #[test]
    fn test_error_retryability() {
        assert!(S3Error::Network("error".to_string()).is_retryable());
        assert!(S3Error::Timeout("timeout".to_string()).is_retryable());
        assert!(!S3Error::InvalidKey("bad".to_string()).is_retryable());
    }
}
