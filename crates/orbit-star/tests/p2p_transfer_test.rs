//! Integration tests for Phase 4: P2P Data Plane
//!
//! These tests verify that Stars can transfer data directly between each other
//! without routing through the Nucleus.
//!
//! # Test Topology
//!
//! ```text
//! Node 1: Star (Source)      → Port 50051
//! Node 2: Star (Destination) → Port 50052
//! Node 3: Test Orchestrator  → (Simulates Nucleus)
//! ```

use orbit_proto::star_service_client::StarServiceClient;
use orbit_proto::star_service_server::StarServiceServer;
use orbit_proto::{ReadStreamRequest, ReplicateRequest};
use orbit_star::auth::AuthService;
use orbit_star::server::StarImpl;
use std::net::SocketAddr;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::fs;
use tonic::transport::Server;

const TEST_AUTH_SECRET: &str = "test-secret-123";
const TEST_STAR_TOKEN: &str = "test-star-token";

/// Helper function to start a Star server on a specific port
async fn start_star_server(port: u16, data_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;

    let star = StarImpl::new(
        vec![data_dir],
        TEST_STAR_TOKEN.to_string(),
        TEST_AUTH_SECRET.to_string(),
    );

    Server::builder()
        .add_service(StarServiceServer::new(star))
        .serve(addr)
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_triangle_transfer() -> Result<(), Box<dyn std::error::Error>> {
    // ══════════════════════════════════════════════════════════
    // Setup: Create temporary directories for two Stars
    // ══════════════════════════════════════════════════════════
    let source_dir = tempdir()?;
    let dest_dir = tempdir()?;

    // Create test file on Source Star
    let test_file_path = source_dir.path().join("payload.dat");
    let test_content = "Hello from Star 1 - This is P2P transfer data!";
    fs::write(&test_file_path, test_content).await?;

    println!("✓ Created source file: {}", test_file_path.display());

    // ══════════════════════════════════════════════════════════
    // Start Source Star (Port 50051)
    // ══════════════════════════════════════════════════════════
    let source_dir_clone = source_dir.path().to_path_buf();
    tokio::spawn(async move {
        start_star_server(50051, source_dir_clone)
            .await
            .expect("Source star server failed");
    });

    // ══════════════════════════════════════════════════════════
    // Start Destination Star (Port 50052)
    // ══════════════════════════════════════════════════════════
    let dest_dir_clone = dest_dir.path().to_path_buf();
    tokio::spawn(async move {
        start_star_server(50052, dest_dir_clone)
            .await
            .expect("Destination star server failed");
    });

    // Wait for servers to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("✓ Started Source Star on port 50051");
    println!("✓ Started Destination Star on port 50052");

    // ══════════════════════════════════════════════════════════
    // Simulate Nucleus: Generate Transfer Token
    // ══════════════════════════════════════════════════════════
    let auth = AuthService::new(TEST_AUTH_SECRET);
    let token = auth.generate_transfer_token("/payload.dat")?;

    println!("✓ Generated transfer token");

    // ══════════════════════════════════════════════════════════
    // Simulate Nucleus: Handshake with Destination Star
    // ══════════════════════════════════════════════════════════
    let mut dest_client = StarServiceClient::connect("http://127.0.0.1:50052").await?;

    use orbit_proto::HandshakeRequest;
    let handshake_response = dest_client
        .handshake(HandshakeRequest {
            star_token: TEST_STAR_TOKEN.to_string(),
            version: "test".to_string(),
            capabilities: vec![],
        })
        .await?;

    assert!(handshake_response.get_ref().accepted);
    let session_id = handshake_response.get_ref().session_id.clone();

    println!("✓ Handshake successful with Destination Star");

    // ══════════════════════════════════════════════════════════
    // Execute: Command Destination to Pull from Source
    // ══════════════════════════════════════════════════════════
    let request = tonic::Request::new(ReplicateRequest {
        source_star_url: "http://127.0.0.1:50051".to_string(),
        remote_path: "/payload.dat".to_string(),
        local_path: "/received/payload.dat".to_string(),
        transfer_token: token,
        expected_size: test_content.len() as u64,
        expected_checksum: vec![],
    });

    // Add session authentication
    let mut request_with_auth = request;
    request_with_auth
        .metadata_mut()
        .insert("session-id", session_id.parse()?);

    let response = dest_client.replicate_file(request_with_auth).await?;

    println!("✓ ReplicateFile command sent");

    // ══════════════════════════════════════════════════════════
    // Verify: Check Transfer Success
    // ══════════════════════════════════════════════════════════
    let result = response.get_ref();
    assert!(result.success, "Transfer failed: {}", result.error_message);
    assert_eq!(result.bytes_transferred, test_content.len() as u64);
    assert!(!result.checksum.is_empty());

    println!("✓ Transfer complete: {} bytes", result.bytes_transferred);
    println!("✓ Checksum: {}", result.checksum);

    // ══════════════════════════════════════════════════════════
    // Verify: File Content Matches
    // ══════════════════════════════════════════════════════════
    let received_file = dest_dir.path().join("received/payload.dat");
    let received_content = fs::read_to_string(&received_file).await?;

    assert_eq!(received_content, test_content, "File content mismatch!");

    println!("✓ Content verification passed");
    println!("\n🎉 Triangle Test PASSED: Star → Star transfer successful!");

    Ok(())
}

#[tokio::test]
async fn test_invalid_token_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let source_dir = tempdir()?;
    let test_file = source_dir.path().join("secret.txt");
    fs::write(&test_file, "confidential data").await?;

    // Start Source Star
    let source_dir_clone = source_dir.path().to_path_buf();
    tokio::spawn(async move {
        start_star_server(50053, source_dir_clone)
            .await
            .expect("Source star failed");
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Try to access with invalid token
    let mut client = StarServiceClient::connect("http://127.0.0.1:50053").await?;

    let response = client
        .read_stream(ReadStreamRequest {
            path: "/secret.txt".to_string(),
            transfer_token: "invalid-token".to_string(),
        })
        .await;

    // Should be rejected
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::PermissionDenied);

    println!("✓ Invalid token correctly rejected");

    Ok(())
}

#[tokio::test]
async fn test_token_wrong_file_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let source_dir = tempdir()?;
    fs::write(source_dir.path().join("allowed.txt"), "allowed").await?;
    fs::write(source_dir.path().join("forbidden.txt"), "forbidden").await?;

    // Start Source Star
    let source_dir_clone = source_dir.path().to_path_buf();
    tokio::spawn(async move {
        start_star_server(50054, source_dir_clone)
            .await
            .expect("Source star failed");
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Generate token for allowed.txt
    let auth = AuthService::new(TEST_AUTH_SECRET);
    let token = auth.generate_transfer_token("/allowed.txt")?;

    // Try to access forbidden.txt with the token
    let mut client = StarServiceClient::connect("http://127.0.0.1:50054").await?;

    let response = client
        .read_stream(ReadStreamRequest {
            path: "/forbidden.txt".to_string(),
            transfer_token: token,
        })
        .await;

    // Should be rejected
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::PermissionDenied);

    println!("✓ Wrong file access correctly rejected");

    Ok(())
}

#[tokio::test]
async fn test_read_stream_direct() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let source_dir = tempdir()?;
    let test_file = source_dir.path().join("data.txt");
    let test_data = "A".repeat(200_000); // 200KB file (3+ chunks)
    fs::write(&test_file, &test_data).await?;

    // Start Source Star
    let source_dir_clone = source_dir.path().to_path_buf();
    tokio::spawn(async move {
        start_star_server(50055, source_dir_clone)
            .await
            .expect("Source star failed");
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Generate token
    let auth = AuthService::new(TEST_AUTH_SECRET);
    let token = auth.generate_transfer_token("/data.txt")?;

    // Connect and stream
    let mut client = StarServiceClient::connect("http://127.0.0.1:50055").await?;

    let mut stream = client
        .read_stream(ReadStreamRequest {
            path: "/data.txt".to_string(),
            transfer_token: token,
        })
        .await?
        .into_inner();

    // Collect all chunks
    let mut received_data = Vec::new();
    let mut chunk_count = 0;

    while let Some(response) = stream.message().await? {
        received_data.extend_from_slice(&response.chunk);
        chunk_count += 1;
    }

    // Verify
    assert_eq!(received_data.len(), test_data.len());
    assert_eq!(String::from_utf8(received_data)?, test_data);
    assert!(
        chunk_count >= 3,
        "Expected multiple chunks, got {}",
        chunk_count
    );

    println!("✓ ReadStream received {} chunks", chunk_count);
    println!("✓ All data verified");

    Ok(())
}
