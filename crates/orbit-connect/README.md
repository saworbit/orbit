# orbit-connect

**Phase 3: The Nucleus Client & RemoteSystem**

Client-side gRPC connectivity for the Orbit Nucleus (Hub) to orchestrate operations on remote Stars (Agents).

## Overview

`orbit-connect` implements the client-side logic required for the Nucleus to control Stars in the Orbit Grid architecture. It provides a transparent proxy that implements the `OrbitSystem` trait, allowing the same code to work with both local and remote filesystems.

## Features

- **RemoteSystem**: Implements `OrbitSystem` by proxying to a remote Star via gRPC
- **StarManager**: Connection pool and session management for multiple Stars
- **Lazy Connections**: Stars are connected only when first accessed
- **Session Management**: Automatic handshake and session ID management
- **Liskov Substitution**: Drop-in replacement for `LocalSystem` in magnetar

## Architecture

```
┌─────────────────┐
│     Nucleus     │
│   (orbit-web)   │
└────────┬────────┘
         │
         │ orbit-connect
         │
    ┌────▼────┐
    │  Star   │  StarManager pools connections
    │ Manager │  and provides OrbitSystem instances
    └────┬────┘
         │
    ┌────▼────────────┐
    │  RemoteSystem   │  Implements OrbitSystem
    │                 │  Translates method calls to gRPC
    └────┬────────────┘
         │
         │ gRPC (orbit-proto)
         │
    ┌────▼────┐
    │  Star   │  Executes operations locally
    │ (Agent) │  (orbit-star)
    └─────────┘
```

## Usage

### Basic Setup

```rust
use orbit_connect::{StarManager, StarRecord};
use orbit_core_interface::OrbitSystem;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut manager = StarManager::new();

    // Register a Star
    let star = StarRecord::new(
        "star-1".to_string(),
        "http://10.0.0.5:50051".to_string(),
        "secret-token-123".to_string(),
    );
    manager.register(star).await;

    // Get a system for this Star (automatically connects)
    let system = manager.get_system("star-1").await?;

    // Use it like any OrbitSystem
    if system.exists(std::path::Path::new("/data/file.bin")).await {
        let hash = system.calculate_hash(
            std::path::Path::new("/data/file.bin"),
            0,
            1024
        ).await?;
        println!("Hash: {}", hex::encode(hash));
    }

    Ok(())
}
```

### Integration with magnetar

The `StarManager` allows magnetar to execute jobs with source and destination on different Stars:

```rust
use orbit_connect::StarManager;
use magnetar::JobStore;

async fn prepare_job(
    job: Job,
    stars: &StarManager
) -> anyhow::Result<Executor> {
    // Resolve Source System
    let source_system = match job.source_star_id {
        Some(id) => stars.get_system(&id).await?,
        None => Arc::new(LocalSystem), // Nucleus's own filesystem
    };

    // Resolve Dest System
    let dest_system = match job.dest_star_id {
        Some(id) => stars.get_system(&id).await?,
        None => Arc::new(LocalSystem),
    };

    // Create Executor with injected systems
    Ok(Executor::new(job, source_system, dest_system))
}
```

## Key Components

### RemoteSystem

Implements `OrbitSystem` by delegating to a remote Star via gRPC:

- **Discovery**: `exists()`, `metadata()`, `read_dir()` → `ScanDirectory` RPC
- **Compute**: `calculate_hash()` → `CalculateHash` RPC (offloads hashing to Star)
- **Intelligence**: `read_header()` → `ReadHeader` RPC (only transfers needed bytes)
- **Data Access**: `reader()`, `writer()` → Not implemented (use Phase 4 Grid Transfer)

### StarManager

Manages connections to multiple Stars:

- **Registration**: Add Stars with ID, address, and token
- **Lazy Connection**: Connect on first use, cache for subsequent calls
- **Handshake**: Automatic authentication and session establishment
- **Connection Pool**: Reuse gRPC channels across requests

## Performance Benefits

### 1. Compute Offloading

Instead of transferring data to compute hashes:

```
❌ Old: Star → Nucleus (network) → BLAKE3 (Nucleus CPU)
✅ New: BLAKE3 (Star CPU) → 32 bytes (network) → Nucleus
```

For a 1GB file with 1MB chunks (1000 chunks):
- Old: 1GB transferred
- New: 32KB transferred (1000 × 32 bytes)

**Savings: 99.997% reduction in network traffic**

### 2. Header Intelligence

Magic number detection only needs first 512 bytes:

```
❌ Old: Transfer entire file to detect type
✅ New: ReadHeader(512) → detect type → skip if unwanted
```

### 3. Connection Reuse

gRPC channels are multiplexed:
- Single TCP connection handles all operations
- No handshake overhead for subsequent requests

## Security

### Phase 3 Security Model

- **Authentication**: Star tokens stored in Nucleus database
- **Session IDs**: Short-lived, validated on each request
- **Metadata Headers**: `x-orbit-session` attached to all gRPC calls

### Future Enhancements (Phase 4+)

- TLS for encrypted transport
- Token rotation
- Rate limiting
- Audit logging

## Testing

### Unit Tests

```bash
cargo test -p orbit-connect
```

### Integration Tests (requires running Star)

```bash
# Start a test Star
cargo run -p orbit-star -- --port 50051 --token test-token

# Run integration tests
cargo test -p orbit-connect -- --ignored
```

## Dependencies

- `orbit-core-interface`: OrbitSystem trait definition
- `orbit-proto`: gRPC protocol definitions
- `tonic`: gRPC client implementation
- `tokio`: Async runtime

## Roadmap

### Phase 3 (Current)

- [x] RemoteSystem implementation
- [x] StarManager connection pooling
- [x] Handshake and session management
- [x] Integration with magnetar

### Phase 4 (Next)

- [ ] Star-to-Star direct transfer (bypass Nucleus)
- [ ] `PullFile` and `PushFile` RPCs
- [ ] Progress streaming for large transfers

### Future

- [ ] TLS support
- [ ] Connection health monitoring
- [ ] Automatic reconnection on failure
- [ ] Load balancing across multiple Stars

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
