# Phase 1 Specification: The I/O Abstraction Layer

**Status:** APPROVED
**Target Version:** v0.6.0-alpha.1
**Owner:** Orbit Architecture Team
**Created:** 2025-12-11

## 1. Executive Summary

Currently, Orbit's core crates (`magnetar`, `core-semantic`) directly invoke `std::fs` or `tokio::fs`. This couples the logic to the local machine and prevents the future "Grid" (Hub/Star) topology from being implemented.

Phase 1 introduces the **`OrbitSystem` trait**: a unified interface for all filesystem and compute-heavy operations. We will refactor the application to use **Dependency Injection**, passing an implementation of `OrbitSystem` into the core logic.

**Before:** Logic calls `File::open()`.
**After:** Logic calls `system.open_stream()`.
**Result:** The same code drives `LocalSystem` (Standalone) and `RemoteSystem` (Grid/Star).

## 2. Architecture: The OrbitSystem Trait

We will introduce a new crate `orbit-core-interface` to define the contract. This prevents circular dependencies between the "Consumer" (Magnetar) and the "Provider" (Local/Remote Implementations).

### 2.1 New Crate Definition

**Crate:** `crates/orbit-core-interface`

```rust
// crates/orbit-core-interface/src/lib.rs

use async_trait::async_trait;
use std::path::Path;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite};
use anyhow::Result;
use std::time::SystemTime;

/// Metadata for a file in the Orbit System
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub len: u64,
    pub is_dir: bool,
    pub modified: SystemTime,
    // capability for future expansion (permissions, owner)
}

/// The Universal Interface for Orbit I/O and Compute
#[async_trait]
pub trait OrbitSystem: Send + Sync + 'static {
    // --- 1. Discovery ---

    /// Check if a path exists
    async fn exists(&self, path: &Path) -> bool;

    /// Get metadata (stat)
    async fn metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// List directory contents (non-recursive)
    /// Returns stream or vec of paths relative to the root
    async fn read_dir(&self, path: &Path) -> Result<Vec<FileMetadata>>;

    // --- 2. Data Access ---

    /// Open a file for reading (streaming)
    async fn reader(&self, path: &Path) -> Result<Box<dyn AsyncRead + Unpin + Send>>;

    /// Open a file for writing
    async fn writer(&self, path: &Path) -> Result<Box<dyn AsyncWrite + Unpin + Send>>;

    // --- 3. Compute Offloading (The "Star" Power) ---

    /// Read the first N bytes (for Semantic Analysis)
    /// Optimized to avoid opening a full stream if possible
    async fn read_header(&self, path: &Path, len: usize) -> Result<Vec<u8>>;

    /// Calculate hash of a specific range (CDC)
    /// This is the Critical Path for distributed performance.
    /// - LocalSystem: Runs on local CPU.
    /// - RemoteSystem: Sends command to Star, Star hashes locally.
    async fn calculate_hash(&self, path: &Path, offset: u64, len: u64) -> Result<[u8; 32]>;
}
```

## 3. Implementation: LocalSystem (Standalone Mode)

We must implement the "default" provider that wraps the local OS. This ensures existing functionality (standalone `orbit sync`) continues to work.

**Location:** `src/backend/local_system.rs` (Moved from ad-hoc helpers)

```rust
// src/backend/local_system.rs

use orbit_core_interface::{OrbitSystem, FileMetadata};
use orbit_core_cdc::compute_hash_locally; // Re-export from core-cdc
use tokio::fs;

pub struct LocalSystem;

#[async_trait]
impl OrbitSystem for LocalSystem {
    async fn exists(&self, path: &Path) -> bool {
        path.exists() // Note: Use tokio::fs::try_exists for async correctness if needed
    }

    async fn metadata(&self, path: &Path) -> Result<FileMetadata> {
        let meta = fs::metadata(path).await?;
        Ok(FileMetadata {
            len: meta.len(),
            is_dir: meta.is_dir(),
            modified: meta.modified()?,
        })
    }

    async fn read_header(&self, path: &Path, len: usize) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;
        let mut file = fs::File::open(path).await?;
        let mut buffer = vec![0u8; len];
        let n = file.read(&mut buffer).await?;
        buffer.truncate(n);
        Ok(buffer)
    }

    async fn calculate_hash(&self, path: &Path, offset: u64, len: u64) -> Result<[u8; 32]> {
        // Leverages the existing core-cdc logic, running on THIS CPU
        // We might need to expose a helper in core-cdc that takes path+offset+len
        orbit_core_cdc::hash_file_range(path, offset, len).await
    }

    // ... implement reader/writer wrappers ...
}
```

## 4. Refactoring core-semantic

**Goal:** `SemanticRegistry` must no longer use `std::fs`. It must accept an `OrbitSystem`.

**Changes in `crates/core-semantic/src/lib.rs`:**

```rust
pub struct SemanticRegistry {
    // ... adapters ...
}

impl SemanticRegistry {
    /// Old: determine_intent(path: &Path) -> Intent
    /// New: Accepts the system trait
    pub async fn determine_intent<S: OrbitSystem>(
        &self,
        system: &S,
        path: &Path
    ) -> Result<ReplicationIntent> {

        // 1. Get header via Trait (Not std::fs)
        // We usually need the first ~512 bytes for magic numbers/text detection
        let header = system.read_header(path, 512).await?;

        // 2. Pass to adapters (adapters operate on bytes/filenames, they are pure)
        // ... existing logic ...
    }
}
```

**Note:** This changes `determine_intent` from synchronous (potentially) to `async`. This ripples up to the caller, which is correct (network IO is async).

## 5. Refactoring magnetar (The Executor)

**Goal:** The job executor currently likely uses `tokio::fs` or `std::fs` to move files. It needs to use `OrbitSystem`.

**Changes in `crates/magnetar/src/executor.rs`:**

```rust
pub struct Executor<S: OrbitSystem> {
    store: JobStore,
    system: S, // <--- Injected Dependency
}

impl<S: OrbitSystem> Executor<S> {
    pub fn new(store: JobStore, system: S) -> Self {
        Self { store, system }
    }

    pub async fn execute_job(&self, job_id: i64) -> Result<()> {
        // ... fetch job ...

        // Example: Verification Step
        if self.system.exists(&job.source_path).await {
            // ...
        }

        // Example: Hashing Step
        let hash = self.system.calculate_hash(&job.source_path, 0, job.size).await?;
    }
}
```

## 6. Wiring it Up (src/main.rs)

We need to compose the dependencies at the application entry point.

```rust
// src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // 1. Initialize the System Implementation
    // Future Phase 2: if args.remote { RemoteSystem::new(...) }
    let system = LocalSystem;

    // 2. Initialize Core Services
    let mut registry = SemanticRegistry::default();

    // 3. Run Logic (Dependency Injection)
    match args.command {
        Commands::Sync { source, dest } => {
            // High-level controller
            perform_sync(&system, &registry, source, dest).await?;
        }
    }

    Ok(())
}
```

## 7. Testing Strategy (The Hidden Benefit)

This refactor allows us to **Mock the Filesystem**. We no longer need to create temp files on disk to test the logic.

```rust
// tests/mock_system.rs

struct MockSystem {
    files: HashMap<PathBuf, Vec<u8>>,
}

#[async_trait]
impl OrbitSystem for MockSystem {
    async fn calculate_hash(&self, path: &Path, ...) -> Result<[u8; 32]> {
        // Return a deterministic hash for tests without IO
        Ok([0xAA; 32])
    }
    // ...
}

#[tokio::test]
async fn test_semantic_logic_on_virtual_files() {
    let sys = MockSystem::new();
    sys.add_file("config.toml", b"[config]");

    let registry = SemanticRegistry::default();
    let intent = registry.determine_intent(&sys, Path::new("config.toml")).await.unwrap();

    assert_eq!(intent.priority, Priority::Critical);
}
```

## 8. Migration Plan

1. **Create Crate:** `cargo new crates/orbit-core-interface --lib`.

2. **Define Trait:** Paste the `OrbitSystem` code.

3. **Update core-semantic:**
   - Add dependency `orbit-core-interface`.
   - Change `determine_intent` signature.
   - Fix compiler errors (replace `fs::` calls).

4. **Update magnetar:**
   - Inject `S: OrbitSystem` into the `Executor` struct.

5. **Create LocalSystem:** Implement the trait in the main binary crate (or a backend crate).

6. **Fix main.rs:** Instantiate `LocalSystem` and pass it down.

7. **Run Tests:** Ensure `cargo test` passes. Existing integration tests (`tests/v2_integration_test.rs`) will need to be updated to instantiate a `LocalSystem`.

## 9. Deliverables

- **Code:** `crates/orbit-core-interface`
- **Refactor:** `core-semantic` and `magnetar` clean of `std::fs`.
- **Proof:** `cargo test` passes in Standalone mode.
- **Docs:** Update `CONTRIBUTING.md` to explain the `OrbitSystem` pattern.

This foundation makes the distinction between "Local Disk" and "Remote Star" purely a configuration detail.

## 10. Benefits

### 10.1 Testability
- Mock filesystem operations without creating temp files
- Deterministic testing of complex workflows
- Faster test execution

### 10.2 Flexibility
- Swap between local and remote implementations at runtime
- Enable future distributed topologies (Hub/Star, Grid)
- Abstract away protocol details (local, SMB, S3, SSH)

### 10.3 Performance
- Offload compute-heavy operations (hashing, compression) to remote nodes
- Reduce network traffic by computing on the data side
- Enable parallel processing across multiple nodes

### 10.4 Maintainability
- Clear separation of concerns
- Single responsibility principle
- Easier to reason about code paths

## 11. Future Phases

### Phase 2: RemoteSystem Implementation
- gRPC or HTTP-based protocol for remote operations
- Star node implementation
- Network error handling and retries

### Phase 3: Grid Topology
- Multi-star orchestration
- Load balancing across stars
- Distributed consensus for metadata

## 12. Risk Assessment

### Low Risk
- LocalSystem is a thin wrapper around existing code
- No breaking changes to external APIs
- Incremental refactoring path

### Medium Risk
- Async refactoring may surface hidden bugs
- Performance overhead of trait dispatch (mitigated by monomorphization)

### High Risk
- None identified

## 13. Success Criteria

1. ✅ `cargo test` passes
2. ✅ Existing functionality preserved
3. ✅ No performance regression in standalone mode
4. ✅ MockSystem enables unit tests without filesystem
5. ✅ Clear path to RemoteSystem implementation

## 14. References

- [ORBIT_GRID_SPEC.md](./ORBIT_GRID_SPEC.md) - Overall Grid architecture
- [BACKEND_REFACTORING_PLAN.md](./BACKEND_REFACTORING_PLAN.md) - Backend abstraction context
