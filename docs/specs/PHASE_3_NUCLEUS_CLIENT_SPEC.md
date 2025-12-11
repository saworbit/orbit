# Phase 3 Specification: The Nucleus Client & RemoteSystem

**Status:** DRAFT
**Target Version:** v0.6.0-alpha.3
**Prerequisites:** Phase 1 (OrbitSystem Trait), Phase 2 (Star Proto)

---

## 1. Executive Summary

Phase 3 implements the client-side logic required for the Nucleus to control Stars.

We will introduce:

- **StarClient**: A resilient wrapper around the gRPC connection.
- **RemoteSystem**: An implementation of `OrbitSystem` that translates local function calls into remote gRPC commands.
- **StarRegistry**: A state manager in the Nucleus that maintains active connections to registered Stars.

By the end of this phase, `magnetar` will be able to execute a job where the "filesystem" is actually a remote server, **without magnetar needing to know the difference**.

---

## 2. Architecture: The RemoteSystem

We must adhere to the **Liskov Substitution Principle**. `magnetar` expects an `OrbitSystem`. We provide `RemoteSystem`.

### 2.1 Crate Structure

We will add a new module (or crate) to encapsulate this logic, likely within `orbit-web` or a new **`orbit-connect`** crate to keep dependencies clean.

**Recommendation:** `crates/orbit-connect`

**Dependencies:** `orbit-core-interface`, `orbit-proto`, `tonic`, `tokio`.

### 2.2 The RemoteSystem Implementation

This struct acts as the proxy.

```rust
// crates/orbit-connect/src/system.rs

use async_trait::async_trait;
use orbit_core_interface::{OrbitSystem, FileMetadata};
use orbit_proto::star_service_client::StarServiceClient;
use orbit_proto::{ScanRequest, HashRequest, ReadHeaderRequest};
use tonic::transport::Channel;
use std::path::Path;

#[derive(Clone)]
pub struct RemoteSystem {
    // The raw gRPC client (cheap to clone)
    client: StarServiceClient<Channel>,
    // Security token for this session
    session_id: String,
}

impl RemoteSystem {
    pub fn new(channel: Channel, session_id: String) -> Self {
        let client = StarServiceClient::new(channel);
        Self { client, session_id }
    }
}

#[async_trait]
impl OrbitSystem for RemoteSystem {
    // --- 1. Discovery ---
    async fn scan_directory(&self, path: &Path) -> Result<Vec<FileMetadata>> {
        let req = ScanRequest {
            path: path.to_string_lossy().to_string(),
        };

        // Add Auth Headers
        let mut request = tonic::Request::new(req);
        request.metadata_mut().insert("x-orbit-session", self.session_id.parse()?);

        // Execute gRPC call
        let mut stream = self.client.clone().scan_directory(request).await?.into_inner();

        let mut results = Vec::new();
        while let Some(entry) = stream.message().await? {
            results.push(FileMetadata {
                len: entry.size,
                is_dir: entry.is_dir,
                modified: std::time::UNIX_EPOCH + std::time::Duration::from_secs(entry.modified_at_ts),
            });
        }
        Ok(results)
    }

    // --- 2. Compute (The Big Win) ---
    async fn calculate_hash(&self, path: &Path, offset: u64, len: u64) -> Result<[u8; 32]> {
        let req = HashRequest {
            path: path.to_string_lossy().to_string(),
            offset,
            length: len,
        };

        let mut request = tonic::Request::new(req);
        request.metadata_mut().insert("x-orbit-session", self.session_id.parse()?);

        let response = self.client.clone().calculate_hash(request).await?;
        let hash_bytes = response.into_inner().hash;

        // Convert Vec<u8> to [u8; 32]
        hash_bytes.try_into().map_err(|_| anyhow::anyhow!("Invalid hash length from Star"))
    }

    // --- 3. Intelligence ---
    async fn read_header(&self, path: &Path, len: usize) -> Result<Vec<u8>> {
        let req = ReadHeaderRequest {
            path: path.to_string_lossy().to_string(),
            length: len as u32,
        };

        let mut request = tonic::Request::new(req);
        request.metadata_mut().insert("x-orbit-session", self.session_id.parse()?);

        let response = self.client.clone().read_header(request).await?;
        Ok(response.into_inner().data)
    }

    // --- 4. Data Access (Fallback) ---
    async fn reader(&self, path: &Path) -> Result<Box<dyn AsyncRead + Unpin + Send>> {
        // NOTE: For Phase 3, we might NOT implement full streaming if we want to force
        // Star-to-Star transfer. However, for compatibility, we can implement
        // a "PullFile" gRPC method.
        // For now, return Error to signal "Not Implemented - Use Direct Transfer"
        Err(anyhow::anyhow!("Direct read from Nucleus not supported in Phase 3. Use Grid Transfer."))
    }
}
```

---

## 3. The StarManager (Connection Pooling)

The Nucleus needs a central place to manage these connections. It shouldn't create a new TCP connection for every file check.

**Location:** `crates/orbit-web/src/state.rs` (or `manager.rs`)

```rust
// crates/orbit-web/src/stars.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use orbit_connect::RemoteSystem;
use tonic::transport::Endpoint;

#[derive(Debug, Clone)]
pub struct StarRecord {
    pub id: String,
    pub address: String, // "http://10.0.0.5:50051"
    pub token: String,
    pub status: StarStatus,
}

pub struct StarManager {
    // Persistent records (SQLite)
    // ... logic to load from DB ...

    // Active connections (In-Memory)
    connections: RwLock<HashMap<String, RemoteSystem>>,
}

impl StarManager {
    /// Get a usable System for a specific Star ID
    pub async fn get_system(&self, star_id: &str) -> Result<Arc<dyn OrbitSystem>> {
        // 1. Check if connected
        let read_guard = self.connections.read().await;
        if let Some(system) = read_guard.get(star_id) {
            return Ok(Arc::new(system.clone()));
        }
        drop(read_guard);

        // 2. Connect if missing (Lazy Connection)
        self.connect(star_id).await
    }

    async fn connect(&self, star_id: &str) -> Result<Arc<dyn OrbitSystem>> {
        // Fetch config from DB (address, token)
        let config = self.fetch_config(star_id).await?;

        // Handshake
        let channel = Endpoint::from_shared(config.address)?.connect().await?;
        let mut client = StarServiceClient::new(channel.clone());

        let resp = client.handshake(HandshakeRequest {
            star_token: config.token,
            ..Default::default()
        }).await?;

        // Create System
        let system = RemoteSystem::new(channel, resp.into_inner().session_id);

        // Cache it
        let mut write_guard = self.connections.write().await;
        write_guard.insert(star_id.to_string(), system.clone());

        Ok(Arc::new(system))
    }
}
```

---

## 4. Integration with magnetar

The final piece is teaching the Job Executor to ask the `StarManager` for the right system.

**Changes in `magnetar/src/lib.rs`:**

1. Update Job struct (in DB migration) to include `source_star_id` and `dest_star_id` (Nullable).
   - `NULL` = Local execution (Nucleus).
   - `UUID` = Remote Star.

2. Update Executor instantiation:

```rust
// crates/orbit-web/src/executor_setup.rs

pub async fn prepare_job(job: Job, stars: &StarManager) -> Result<Executor> {
    // Resolve Source System
    let source_system: Arc<dyn OrbitSystem> = match job.source_star_id {
        Some(id) => stars.get_system(&id).await?,
        None => Arc::new(LocalSystem), // The Nucleus's own filesystem
    };

    // Resolve Dest System
    let dest_system: Arc<dyn OrbitSystem> = match job.dest_star_id {
        Some(id) => stars.get_system(&id).await?,
        None => Arc::new(LocalSystem),
    };

    // Create Executor with injected systems
    Ok(Executor::new(job, source_system, dest_system))
}
```

---

## 5. Security & Handshake Logic

For Phase 3, we implement the **Initiator side** of the handshake defined in Phase 2.

- **Trigger:** When `StarManager` attempts to connect.
- **Request:** Sends the `star_token` (stored in Nucleus DB).
- **Response:** Receives `session_id`.
- **Storage:** `RemoteSystem` stores `session_id` and attaches it to `x-orbit-session` header for every subsequent call.

---

## 6. Testing Strategy

We can mock the `StarServiceClient` using `tonic-mock` or simply spin up a real `orbit-star` process during integration tests.

**Test Scenario: "Remote Hash Check"**

1. Spawn `orbit-star` on `localhost:50051` (with a test file `data.bin`).
2. Configure `StarManager` in Nucleus with `id="test-star"`, `addr="http://localhost:50051"`.
3. Call `manager.get_system("test-star")`.
4. Call `system.calculate_hash("data.bin")`.
5. **Assert:** Result matches local BLAKE3 hash of `data.bin`.
6. **Verify:** Nucleus CPU usage was near zero (hashing happened in Star process).

---

## 7. Deliverables Checklist

- [ ] **Crate:** `crates/orbit-connect` created.
- [ ] **Struct:** `RemoteSystem` implemented with `scan`, `read_header`, `hash`.
- [ ] **Manager:** `StarManager` implemented with lazy connection pooling.
- [ ] **Integration:** `magnetar` Executor accepts dynamic `OrbitSystem`.
- [ ] **Database:** Schema migration to add `source_star_id` / `dest_star_id` to Jobs table.
- [ ] **Tests:** Integration test for remote hash calculation.

---

## 8. Next Steps (Lookahead to Phase 4)

Phase 3 allows the Nucleus to orchestrate. However, **data transfer (copying) is still stuck** because `RemoteSystem::reader` returns an error (or would route through Nucleus).

**Phase 4 (The Data Plane)** will implement the `ThirdPartyTransfer` logic:

1. Nucleus tells Star A: "Send file X to Star B."
2. Star A connects directly to Star B.

This requires a new `OrbitSystem` method: `transfer_to(&self, target: &str, file: &Path)`.

---

## 9. API Design Decisions

### 9.1 Error Handling

All gRPC errors will be wrapped in `anyhow::Error` to maintain consistency with the existing codebase. Connection failures will be automatically retried using exponential backoff (to be implemented in future resilience phase).

### 9.2 Session Management

Sessions are stateless from the Nucleus perspective. If a session expires, the `StarManager` will automatically re-handshake on the next request.

### 9.3 Concurrency Model

`StarManager` uses `RwLock` for connection pooling, allowing multiple concurrent readers (requests to different stars) while ensuring exclusive access during connection establishment.

---

## 10. Performance Considerations

- **Connection Reuse:** gRPC channels are multiplexed, so a single connection can handle multiple concurrent requests.
- **Lazy Connection:** Stars are only connected when first accessed, not at startup.
- **Clone Semantics:** `RemoteSystem` is cheaply cloneable (Arc internally in tonic Channel).

---

## 11. Security Model

- **Authentication:** Star tokens are stored securely in the Nucleus database.
- **Authorization:** Session IDs are short-lived and validated by the Star on each request.
- **Transport Security:** TLS support will be added in a future phase; for MVP, we assume trusted network.

---

## 12. Migration Path

Existing jobs (without `source_star_id`/`dest_star_id`) will continue to work using `LocalSystem`. This ensures backward compatibility during the rollout.
