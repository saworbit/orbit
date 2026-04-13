# Phase 4 Specification: The Data Plane (Star-to-Star Transfer)

**Status:** DRAFT
**Target Version:** v0.6.0-alpha.4
**Prerequisites:** Phase 3 (Nucleus Client)

---

## 1. Executive Summary

Phase 4 implements the **Third-Party Transfer mechanism** that enables true peer-to-peer data movement in the Orbit Grid.

### The Problem

Currently, `magnetar` executes transfers by reading from Source and writing to Destination. If both are remote `RemoteSystem`s, the data flows:

```
Star A → Nucleus → Star B
```

This creates a **"hairpin" bottleneck** and saturates the Nucleus's bandwidth. The Nucleus becomes the limiting factor in grid throughput.

### The Solution

We implement a **Direct Link protocol**. The Nucleus commands the Destination Star to pull data directly from the Source Star:

```
Star A → Star B
```

The Nucleus acts as the **orchestrator**, not the transport layer.

### The Result

- **Infinite horizontal scaling** of bandwidth
- Nucleus CPU/network resources freed for control plane operations
- Multi-terabyte transfers between Stars without Nucleus involvement
- Foundation for advanced features: multi-source streaming, erasure coding, and parallel chunk transfers

---

## 2. Architecture: The PeerTransfer Protocol

### 2.1 Pull-Based Design Philosophy

We chose a **Pull-Based architecture** (Destination pulls from Source) because:

1. **Alignment with Orbit's Flow Control:** Orbit uses receiver-driven backpressure (see Phase 1). The destination controls the read rate.
2. **Firewall-Friendly:** Destination initiates the connection. No need for Source to accept inbound connections from unknown peers.
3. **Security:** Source validates authorization before sending data, not after.
4. **Resilience:** Destination can retry reads without Source needing to track state.

### 2.2 Three-Actor Model

```
┌──────────────┐
│   Nucleus    │  ← Control Plane (Orchestration)
│  (magnetar)  │
└───────┬──────┘
        │ ① Command: "Dest, pull from Src"
        │
        ├─────────────────────────┐
        │                         │
        ▼                         ▼
┌────────────┐            ┌────────────┐
│  Star A    │            │  Star B    │
│  (Source)  │◄───────────│  (Dest)    │
└────────────┘  ② Data    └────────────┘
              ReadStream
```

**Actor Responsibilities:**

| Actor | Role | Actions |
|-------|------|---------|
| **Nucleus** | Orchestrator | Issues `ReplicateFile` command to Destination, generates security tokens |
| **Star B (Dest)** | Puller | Receives command, connects to Source, writes data to local disk |
| **Star A (Source)** | Server | Validates token, serves file chunks via streaming RPC |

---

## 3. Protocol Updates

### 3.1 New RPC Methods

We extend the `StarService` in `orbit.proto` with two new methods:

**File:** `crates/orbit-proto/proto/orbit.proto`

```protobuf
service StarService {
  // ... existing Phase 2 methods (Ping, GetFileMetadata) ...
  // ... existing Phase 3 methods (CreateFile, DeleteFile) ...

  // ═══════════════════════════════════════════════════════════
  // Phase 4: Data Plane (Peer-to-Peer Transfer)
  // ═══════════════════════════════════════════════════════════

  // ① Command Interface (Nucleus → Destination Star)
  // "Please pull data from the remote source and save it locally."
  rpc ReplicateFile (ReplicateRequest) returns (ReplicateResponse);

  // ② Data Access Interface (Destination Star → Source Star)
  // "Give me the byte stream for this file."
  rpc ReadStream (ReadStreamRequest) returns (stream ReadStreamResponse);
}
```

### 3.2 Message Definitions

```protobuf
// ═══════════════════════════════════════════════════════════
// ReplicateFile: Nucleus commands Destination to pull
// ═══════════════════════════════════════════════════════════

message ReplicateRequest {
  // Where to pull from
  string source_star_url = 1;     // e.g., "http://10.0.0.5:50051"
  string remote_path = 2;         // File path on Source Star

  // Where to save
  string local_path = 3;          // Destination path on this Star

  // Security
  string transfer_token = 4;      // JWT signed by Nucleus

  // Metadata
  uint64 expected_size = 5;       // For progress tracking (optional)
  bytes expected_checksum = 6;    // SHA-256 for verification (optional)
}

message ReplicateResponse {
  bool success = 1;
  uint64 bytes_transferred = 2;
  string checksum = 3;            // SHA-256 of transferred data
  string error_message = 4;       // Populated if success = false
}

// ═══════════════════════════════════════════════════════════
// ReadStream: Star-to-Star data transfer
// ═══════════════════════════════════════════════════════════

message ReadStreamRequest {
  string path = 1;                // File to read
  string transfer_token = 2;      // Authorization token from Nucleus

  // Future: byte range support for resumable transfers
  // uint64 offset = 3;
  // uint64 length = 4;
}

message ReadStreamResponse {
  bytes chunk = 1;                // File data (up to 64KB per message)
}
```

---

## 4. Security: Signed Transfer Tokens

### 4.1 The Problem

**Challenge:** Star A doesn't share a database with Star B. When Star B requests a file, how does Star A know the request is authorized?

**Non-Solution:** Require B to authenticate with A. This creates a key distribution nightmare.

**Orbit Solution:** **Stateless Authorization Tokens** (JWT).

### 4.2 Token Flow

```
┌──────────┐
│ Nucleus  │
└────┬─────┘
     │ ① Generate JWT
     │    sub: "transfer"
     │    allow_file: "/data/report.csv"
     │    exp: 3600 (1 hour)
     │    Signature: HMAC-SHA256(ORBIT_AUTH_SECRET)
     │
     ▼
┌────────────┐
│  Star B    │ ② Receive token in ReplicateRequest
│  (Dest)    │
└─────┬──────┘
      │ ③ Forward token in ReadStreamRequest
      ▼
┌────────────┐
│  Star A    │ ④ Verify signature using local ORBIT_AUTH_SECRET
│  (Source)  │ ⑤ Check claims: file path, expiration
└────────────┘ ⑥ Serve file if valid
```

### 4.3 Token Structure (JWT Claims)

```json
{
  "sub": "transfer",
  "allow_file": "/data/images/img1.jpg",
  "exp": 1735680000,
  "iat": 1735676400,
  "iss": "orbit-nucleus"
}
```

**Security Properties:**

- **Confidentiality:** Not required (token authorizes a specific file, not credentials)
- **Integrity:** HMAC signature prevents tampering
- **Expiration:** Short-lived (1 hour default) to limit exposure
- **Single-Use (Future):** Add `jti` (JWT ID) and track used tokens in Redis

### 4.4 Secret Distribution

**Requirement:** All Stars must share `ORBIT_AUTH_SECRET` to verify tokens.

**Distribution Mechanisms:**

1. **Environment Variable:** Set `ORBIT_AUTH_SECRET` on all nodes (suitable for single-tenant deployments)
2. **Phase 3 Handshake Extension (Recommended):** Nucleus sends secret during `RegisterStar` (encrypted via TLS)
3. **Kubernetes Secret:** Mount as volume in containerized deployments

**Implementation Note:** For v0.6.0-alpha.4, we use **Environment Variable** for simplicity.

---

## 5. Implementation: orbit-star Updates

### 5.1 ReadStream Implementation (Source Star)

**File:** `crates/orbit-star/src/server.rs`

```rust
use tokio::io::AsyncReadExt;
use tokio_stream::wrappers::ReceiverStream;

impl StarService for StarServer {
    type ReadStreamStream = ReceiverStream<Result<ReadStreamResponse, Status>>;

    async fn read_stream(
        &self,
        request: Request<ReadStreamRequest>,
    ) -> Result<Response<Self::ReadStreamStream>, Status> {
        let req = request.into_inner();

        // ──────────────────────────────────────────────────────
        // Step 1: Verify Transfer Token (Stateless)
        // ──────────────────────────────────────────────────────
        self.auth_verifier
            .verify_transfer_token(&req.transfer_token, &req.path)
            .map_err(|e| {
                tracing::warn!("Transfer token verification failed: {}", e);
                Status::permission_denied("Invalid or expired transfer token")
            })?;

        tracing::info!("Authorized ReadStream request for: {}", req.path);

        // ──────────────────────────────────────────────────────
        // Step 2: Secure Path (Prevent Directory Traversal)
        // ──────────────────────────────────────────────────────
        let full_path = self.path_jail.secure_path(&req.path).map_err(|e| {
            Status::invalid_argument(format!("Invalid path: {}", e))
        })?;

        // ──────────────────────────────────────────────────────
        // Step 3: Open File
        // ──────────────────────────────────────────────────────
        let mut file = tokio::fs::File::open(&full_path).await.map_err(|e| {
            Status::not_found(format!("File not found: {}", e))
        })?;

        // ──────────────────────────────────────────────────────
        // Step 4: Stream Chunks to Client
        // ──────────────────────────────────────────────────────
        let (tx, rx) = tokio::sync::mpsc::channel(4); // 4-message buffer

        tokio::spawn(async move {
            let mut buffer = vec![0u8; 64 * 1024]; // 64KB chunks
            loop {
                match file.read(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let chunk = buffer[..n].to_vec();
                        if tx
                            .send(Ok(ReadStreamResponse { chunk }))
                            .await
                            .is_err()
                        {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(Status::internal(e.to_string()))).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
```

**Key Design Decisions:**

- **Chunk Size:** 64KB balances memory usage and RPC overhead
- **Async I/O:** Uses `tokio::fs` for non-blocking disk reads
- **Backpressure:** Channel buffer (4 messages) prevents unlimited memory growth
- **Error Handling:** Stream errors are sent as RPC errors, not panics

---

### 5.2 ReplicateFile Implementation (Destination Star)

**File:** `crates/orbit-star/src/server.rs`

```rust
use tokio::io::AsyncWriteExt;
use sha2::{Sha256, Digest};

impl StarService for StarServer {
    async fn replicate_file(
        &self,
        request: Request<ReplicateRequest>,
    ) -> Result<Response<ReplicateResponse>, Status> {
        let req = request.into_inner();

        tracing::info!(
            "ReplicateFile: {} → {} (from {})",
            req.remote_path,
            req.local_path,
            req.source_star_url
        );

        // ──────────────────────────────────────────────────────
        // Step 1: Prepare Local Destination
        // ──────────────────────────────────────────────────────
        let save_path = self.path_jail.secure_path(&req.local_path).map_err(|e| {
            Status::invalid_argument(format!("Invalid local path: {}", e))
        })?;

        // Create parent directory if needed
        if let Some(parent) = save_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Status::internal(format!("Failed to create directory: {}", e))
            })?;
        }

        // ──────────────────────────────────────────────────────
        // Step 2: Connect to Source Star
        // ──────────────────────────────────────────────────────
        // TODO: Implement connection pooling for production
        let mut source_client = StarServiceClient::connect(req.source_star_url.clone())
            .await
            .map_err(|e| {
                Status::unavailable(format!("Cannot connect to source: {}", e))
            })?;

        // ──────────────────────────────────────────────────────
        // Step 3: Request Data Stream
        // ──────────────────────────────────────────────────────
        let stream_req = ReadStreamRequest {
            path: req.remote_path.clone(),
            transfer_token: req.transfer_token,
        };

        let mut stream = source_client
            .read_stream(stream_req)
            .await
            .map_err(|e| {
                Status::internal(format!("ReadStream failed: {}", e))
            })?
            .into_inner();

        // ──────────────────────────────────────────────────────
        // Step 4: Write to Disk (with Checksum Verification)
        // ──────────────────────────────────────────────────────
        let mut file = tokio::fs::File::create(&save_path).await.map_err(|e| {
            Status::internal(format!("Cannot create file: {}", e))
        })?;

        let mut hasher = Sha256::new();
        let mut total_bytes = 0u64;

        while let Some(response) = stream.message().await? {
            let chunk = response.chunk;
            file.write_all(&chunk).await.map_err(|e| {
                Status::internal(format!("Write failed: {}", e))
            })?;
            hasher.update(&chunk);
            total_bytes += chunk.len() as u64;
        }

        file.sync_all().await.map_err(|e| {
            Status::internal(format!("Sync failed: {}", e))
        })?;

        let checksum = format!("{:x}", hasher.finalize());

        // ──────────────────────────────────────────────────────
        // Step 5: Verify Expected Size (if provided)
        // ──────────────────────────────────────────────────────
        if req.expected_size > 0 && total_bytes != req.expected_size {
            tracing::error!(
                "Size mismatch: expected {}, got {}",
                req.expected_size,
                total_bytes
            );
            // Optionally delete the incomplete file
            let _ = tokio::fs::remove_file(&save_path).await;
            return Err(Status::data_loss("File size mismatch"));
        }

        tracing::info!(
            "Transfer complete: {} bytes, checksum: {}",
            total_bytes,
            checksum
        );

        Ok(Response::new(ReplicateResponse {
            success: true,
            bytes_transferred: total_bytes,
            checksum,
            error_message: String::new(),
        }))
    }
}
```

**Key Design Decisions:**

- **Streaming Write:** Never loads entire file into memory
- **Incremental Hashing:** Computes SHA-256 during write (zero-cost verification)
- **Atomic Safety:** Future enhancement: write to `.tmp` file, then atomic rename
- **Size Verification:** Detects truncated transfers
- **Error Recovery:** Future: implement retry with byte-range requests

---

### 5.3 JWT Token Service

**File:** `crates/orbit-star/src/auth.rs` (new file)

```rust
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct TransferClaims {
    sub: String,          // "transfer"
    allow_file: String,   // "/data/file.txt"
    exp: u64,             // Expiration timestamp
    iat: u64,             // Issued at
    iss: String,          // "orbit-nucleus"
}

pub struct AuthService {
    secret: Vec<u8>,
}

impl AuthService {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
        }
    }

    /// Generate a transfer token (called by Nucleus)
    pub fn generate_transfer_token(&self, file_path: &str) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = TransferClaims {
            sub: "transfer".to_string(),
            allow_file: file_path.to_string(),
            exp: now + 3600, // 1 hour validity
            iat: now,
            iss: "orbit-nucleus".to_string(),
        };

        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| e.to_string())
    }

    /// Verify a transfer token (called by Source Star)
    pub fn verify_transfer_token(&self, token: &str, requested_path: &str) -> Result<(), String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["orbit-nucleus"]);

        let token_data = decode::<TransferClaims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &validation,
        )
        .map_err(|e| format!("Token decode failed: {}", e))?;

        // Verify the token authorizes this specific file
        if token_data.claims.allow_file != requested_path {
            return Err(format!(
                "Token allows '{}', but requested '{}'",
                token_data.claims.allow_file, requested_path
            ));
        }

        Ok(())
    }
}
```

---

## 6. Implementation: magnetar Integration

### 6.1 Executor Logic Update

**File:** `crates/magnetar/src/executor.rs`

```rust
impl Executor {
    pub async fn execute_copy(&self, job: &Job) -> Result<()> {
        // ══════════════════════════════════════════════════════
        // SCENARIO A: Remote Star → Remote Star (P2P Transfer)
        // ══════════════════════════════════════════════════════
        if let (Some(src_star_id), Some(dest_star_id)) =
            (&job.source_star_id, &job.dest_star_id)
        {
            tracing::info!(
                "Detected remote-to-remote transfer: {} → {}",
                src_star_id,
                dest_star_id
            );

            // ────────────────────────────────────────────────
            // Step 1: Resolve Star Configurations
            // ────────────────────────────────────────────────
            let src_star = self
                .star_registry
                .get_star(src_star_id)
                .await
                .ok_or_else(|| anyhow!("Source star not found"))?;

            let dest_system = self
                .star_registry
                .get_system(dest_star_id)
                .await
                .ok_or_else(|| anyhow!("Destination star not found"))?;

            // ────────────────────────────────────────────────
            // Step 2: Generate Transfer Token
            // ────────────────────────────────────────────────
            let token = self
                .auth_service
                .generate_transfer_token(&job.source_path)?;

            // ────────────────────────────────────────────────
            // Step 3: Command Destination to Pull
            // ────────────────────────────────────────────────
            // We need to downcast OrbitSystem to RemoteSystem
            // (Only RemoteSystem implements ReplicateFile)
            if let Some(remote_dest) = dest_system.as_any().downcast_ref::<RemoteSystem>() {
                let result = remote_dest
                    .replicate_file(ReplicateRequest {
                        source_star_url: src_star.address.clone(),
                        remote_path: job.source_path.clone(),
                        local_path: job.dest_path.clone(),
                        transfer_token: token,
                        expected_size: 0, // Optional: query source first
                        expected_checksum: vec![],
                    })
                    .await?;

                if !result.success {
                    return Err(anyhow!("Transfer failed: {}", result.error_message));
                }

                tracing::info!(
                    "P2P transfer complete: {} bytes ({})",
                    result.bytes_transferred,
                    result.checksum
                );

                return Ok(());
            } else {
                tracing::warn!("Destination is not a RemoteSystem, falling back to relay mode");
            }
        }

        // ══════════════════════════════════════════════════════
        // SCENARIO B: Fallback (Nucleus-Relayed Transfer)
        // ══════════════════════════════════════════════════════
        // Use existing read/write loop through Nucleus
        self.execute_copy_via_nucleus(job).await
    }

    async fn execute_copy_via_nucleus(&self, job: &Job) -> Result<()> {
        // ... existing implementation ...
    }
}
```

### 6.2 OrbitSystem Trait Extension

**File:** `crates/core-resilience/src/orbit_system.rs`

```rust
#[async_trait]
pub trait OrbitSystem: Send + Sync {
    // ... existing methods ...

    /// Downcast to concrete type for advanced operations
    fn as_any(&self) -> &dyn std::any::Any;
}

// Update RemoteSystem implementation
impl OrbitSystem for RemoteSystem {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

**Alternative Design:** Add `replicate_file()` to `OrbitSystem` trait and return `Err(Unsupported)` for `LocalSystem`.

---

## 7. Testing Strategy

### 7.1 The Triangle Test

**Objective:** Prove that data flows Star → Star without touching Nucleus.

**Topology:**

```
Node 1: orbit-star (Source)  → Port 50051
Node 2: orbit-star (Dest)    → Port 50052
Node 3: Test Orchestrator    → (Simulates Nucleus)
```

**Procedure:**

```bash
# Node 1: Start Source Star
export ORBIT_AUTH_SECRET="test-secret-123"
export ORBIT_DATA_DIR="/tmp/star1"
mkdir -p /tmp/star1/data
echo "Hello from Star 1" > /tmp/star1/data/payload.dat
orbit-star --port 50051

# Node 2: Start Destination Star
export ORBIT_AUTH_SECRET="test-secret-123"
export ORBIT_DATA_DIR="/tmp/star2"
mkdir -p /tmp/star2
orbit-star --port 50052

# Node 3: Run Test Script
cargo test test_triangle_transfer
```

**Test Code:**

```rust
#[tokio::test]
async fn test_triangle_transfer() {
    // Setup
    let auth = AuthService::new("test-secret-123");
    let token = auth.generate_transfer_token("/data/payload.dat").unwrap();

    // Connect to Destination
    let mut dest_client = StarServiceClient::connect("http://localhost:50052")
        .await
        .unwrap();

    // Issue Replicate Command
    let request = ReplicateRequest {
        source_star_url: "http://localhost:50051".into(),
        remote_path: "/data/payload.dat".into(),
        local_path: "/received/payload.dat".into(),
        transfer_token: token,
        expected_size: 0,
        expected_checksum: vec![],
    };

    let response = dest_client.replicate_file(request).await.unwrap();

    // Assertions
    assert!(response.into_inner().success);
    let content = std::fs::read_to_string("/tmp/star2/received/payload.dat").unwrap();
    assert_eq!(content, "Hello from Star 1");
}
```

### 7.2 Bandwidth Verification Test

**Objective:** Prove Nucleus bandwidth usage is near-zero during P2P transfer.

**Approach:**

1. Transfer 100MB file using **Nucleus relay mode** (existing behavior)
   - Monitor Nucleus network traffic: should be ~200MB (100MB in + 100MB out)
2. Transfer 100MB file using **P2P mode** (Phase 4)
   - Monitor Nucleus network traffic: should be <1KB (only RPC metadata)

**Tooling:** Use `nethogs` or `iftop` to monitor per-process network usage.

### 7.3 Security Tests

```rust
#[tokio::test]
async fn test_invalid_token_rejected() {
    let mut client = StarServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let response = client
        .read_stream(ReadStreamRequest {
            path: "/data/secret.txt".into(),
            transfer_token: "invalid-token".into(),
        })
        .await;

    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::PermissionDenied);
}

#[tokio::test]
async fn test_expired_token_rejected() {
    let auth = AuthService::new("test-secret");

    // Create expired token (modify claims manually)
    let expired_token = /* ... create token with exp in the past ... */;

    let mut client = StarServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let response = client
        .read_stream(ReadStreamRequest {
            path: "/data/file.txt".into(),
            transfer_token: expired_token,
        })
        .await;

    assert!(response.is_err());
}

#[tokio::test]
async fn test_wrong_file_rejected() {
    let auth = AuthService::new("test-secret");
    let token = auth.generate_transfer_token("/data/allowed.txt").unwrap();

    let mut client = StarServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    // Try to access a different file
    let response = client
        .read_stream(ReadStreamRequest {
            path: "/data/forbidden.txt".into(),
            transfer_token: token,
        })
        .await;

    assert!(response.is_err());
}
```

---

## 8. Performance Considerations

### 8.1 Bottleneck Analysis

| Component | Throughput Limit | Mitigation |
|-----------|------------------|------------|
| **Disk I/O** | ~500 MB/s (SSD) | Use `O_DIRECT` for large files (bypass page cache) |
| **Network** | 1-10 Gbps | TCP tuning: increase window size, enable TCP BBR |
| **gRPC Overhead** | ~2-3% CPU | Acceptable for 64KB chunks |
| **Memory** | Unbounded streams | Limit concurrent transfers per Star (semaphore) |

### 8.2 Chunk Size Tuning

**Current:** 64KB per `ReadStreamResponse`

**Trade-offs:**

- **Larger chunks** (1MB): Fewer RPC messages, higher memory usage
- **Smaller chunks** (16KB): More CPU overhead, better flow control

**Recommendation:** Make configurable via `ORBIT_TRANSFER_CHUNK_SIZE` env var.

### 8.3 Connection Pooling

**Problem:** `ReplicateFile` creates a new gRPC connection to Source for every transfer.

**Solution:** Implement a connection pool in `RemoteSystem`:

```rust
pub struct RemoteSystem {
    client_pool: Arc<ClientPool>,
}

impl RemoteSystem {
    async fn replicate_file(&self, req: ReplicateRequest) -> Result<...> {
        let mut client = self.client_pool.get(&req.source_star_url).await?;
        // ... use client ...
    }
}
```

**Implementation:** Phase 4.1 (post-alpha.4).

---

## 9. Future Enhancements

### 9.1 Resumable Transfers (Phase 4.1)

**Feature:** Support byte-range reads to resume interrupted transfers.

**Protocol Change:**

```protobuf
message ReadStreamRequest {
  string path = 1;
  string transfer_token = 2;
  uint64 offset = 3;  // Start at this byte
  uint64 length = 4;  // Read this many bytes (0 = EOF)
}
```

**Implementation:** Destination tracks `bytes_transferred` in a state file. On retry, use `offset` to resume.

### 9.2 Multi-Source Streaming (Phase 4.2)

**Feature:** Download different chunks of the same file from multiple Sources in parallel.

**Use Case:** 100GB file stored across 10 Stars → download 10GB from each Star concurrently.

**Protocol:** Add `chunk_id` to `ReadStreamRequest`.

### 9.3 Checksum Verification (Phase 4.1)

**Current State:** We compute SHA-256 but don't verify against expected value.

**Enhancement:**

1. Source includes `expected_checksum` in metadata query
2. Destination compares after transfer
3. Auto-retry on mismatch (up to 3 attempts)

### 9.4 Compression (Phase 4.2)

**Feature:** Compress data during transfer for files with high compression ratio.

**Protocol:**

```protobuf
message ReadStreamRequest {
  // ...
  string compression = 5;  // "gzip", "zstd", "none"
}
```

**Implementation:** Transparent compression/decompression in stream handlers.

### 9.5 Encryption (Phase 5)

**Feature:** End-to-end encryption for data in transit (beyond TLS).

**Approach:** Encrypt chunks with AES-256-GCM, include key in transfer token.

---

## 10. Migration Guide

### 10.1 Backwards Compatibility

**Guarantee:** Phase 4 does **not** break existing behavior.

- **Old Nucleus + Old Stars:** Continue using relay mode (no change)
- **New Nucleus + Old Stars:** Falls back to relay mode (Stars don't implement `ReplicateFile`)
- **New Nucleus + New Stars:** Automatically uses P2P mode

**Detection:** Nucleus tries P2P, catches `Unimplemented` error, falls back.

### 10.2 Deployment Strategy

**Recommended Rollout:**

1. **Week 1:** Deploy Phase 4 Stars (no traffic change yet)
2. **Week 2:** Deploy Nucleus with P2P logic (auto-enables for compatible Stars)
3. **Week 3:** Monitor metrics, verify bandwidth reduction

**Rollback:** Downgrade Nucleus → all transfers use relay mode again.

---

## 11. Deliverables Checklist

- [ ] **Protocol Definition**
  - [ ] Add `ReplicateFile` RPC to `orbit.proto`
  - [ ] Add `ReadStream` RPC to `orbit.proto`
  - [ ] Define `ReplicateRequest`, `ReplicateResponse`, `ReadStreamRequest`, `ReadStreamResponse` messages
  - [ ] Update `buf.build` schema (if using Buf Schema Registry)

- [ ] **Star Server (orbit-star)**
  - [ ] Implement `read_stream()` method (file serving)
  - [ ] Implement `replicate_file()` method (file pulling)
  - [ ] Add `AuthService` with JWT generation/verification
  - [ ] Add path security checks (prevent directory traversal)
  - [ ] Add integration tests for both RPC methods

- [ ] **Star Client (RemoteSystem)**
  - [ ] Add `replicate_file()` method to `RemoteSystem` struct
  - [ ] Implement connection to remote Star
  - [ ] Implement streaming write to local disk
  - [ ] Add checksum computation

- [ ] **Nucleus (magnetar)**
  - [ ] Update `Executor::execute_copy()` to detect Remote→Remote jobs
  - [ ] Add `AuthService::generate_transfer_token()` call
  - [ ] Add fallback to relay mode for incompatible Stars
  - [ ] Add metrics: `p2p_transfers_total`, `relay_transfers_total`

- [ ] **Security**
  - [ ] Implement JWT signing in Nucleus
  - [ ] Implement JWT verification in Stars
  - [ ] Add secret distribution mechanism (env var for alpha.4)
  - [ ] Document secret rotation procedure

- [ ] **Testing**
  - [ ] Triangle Test (Star→Star transfer)
  - [ ] Bandwidth verification test (Nucleus traffic near-zero)
  - [ ] Security tests (invalid/expired/wrong-file tokens)
  - [ ] Load test (1000 concurrent transfers)

- [ ] **Documentation**
  - [ ] Update `ORBIT_GRID_ARCHITECTURE.md` with Phase 4 section
  - [ ] Add "P2P Transfer" tutorial to docs
  - [ ] Update deployment guide with secret configuration
  - [ ] Create troubleshooting guide for transfer failures

- [ ] **Observability**
  - [ ] Add tracing spans for transfers
  - [ ] Add metrics: bytes transferred, transfer duration, error rate
  - [ ] Add logs for token verification failures
  - [ ] Create Grafana dashboard for transfer monitoring

---

## 12. Success Criteria

Phase 4 is **complete** when:

1. ✅ A 1GB file transfers from Star A to Star B in <10 seconds (on 10Gbps network)
2. ✅ Nucleus network traffic is <1MB during the transfer
3. ✅ Invalid tokens are rejected with `PermissionDenied` errors
4. ✅ All integration tests pass
5. ✅ No memory leaks during 1000-file stress test

---

## 13. Final Architecture Review

Congratulations. With **Phase 4** complete, you have evolved Orbit into a **Tier-1 Data Fabric**.

### The Journey

| Phase | Achievement | Impact |
|-------|-------------|--------|
| **Phase 1** | I/O Abstraction Layer (`OrbitSystem` trait) | Decoupled logic from disk |
| **Phase 2** | Star Protocol & Agent | Gave you a voice to speak to remote agents |
| **Phase 3** | Nucleus Client | Gave Nucleus the power to command agents |
| **Phase 4** | Data Plane (P2P Transfer) | Allowed agents to move mountains of data autonomously |

### What You Built

You are no longer building a **file copier**.

You are building a **distributed operating system for data**.

### What's Next

- **Phase 5:** Erasure Coding (data durability)
- **Phase 6:** Query Pushdown (compute where data lives)
- **Phase 7:** Global Namespace (unified view of distributed data)

---

**END OF SPECIFICATION**

*Document Version: 1.0*
*Last Updated: 2025-12-11*
*Author: Orbit Team*
