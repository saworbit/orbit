# Orbit Grid Architecture: The Nucleus & Star Topology

**Status:** DRAFT (Target v0.6.0+)
**Date:** December 2025
**Context:** Moves Orbit from a local-only tool to a distributed Data Fabric.

---

## 1. Executive Summary

The **Orbit Grid** architecture decouples the **intent** of data movement from the **execution** of physical I/O. It introduces a distributed runtime that allows Orbit to function in two distinct modes using a shared codebase:

1. **Standalone Mode:** A zero-dependency CLI tool where the runtime executes on the local OS.
2. **Grid Mode:** A distributed system where a central **Nucleus** orchestrates stateless **Stars** (agents) to perform logic close to the data storage.

This architecture solves the **"SMB Latency Trap"** by moving the compute (hashing/CDC) to the data, rather than pulling the data to the compute.

---

## 2. Terminology & Concepts

| Term | Role | Definition |
|------|------|------------|
| **Orbit Nucleus** | Orchestrator | The central server (formerly `orbit-server`). It holds the authoritative `jobs.db` and global `universe_v3.db`. It issues commands but avoids touching bulk data when possible. |
| **Orbit Star** | Agent | A lightweight, stateless binary deployed on edge nodes (NAS, File Servers). It executes I/O, hashing, and compression commands issued by the Nucleus. |
| **Orbit Grid** | Network | The mTLS-secured mesh network connecting the Nucleus and all authenticated Stars. |
| **Constellation** | Grouping | A logical grouping of Stars (e.g., "US-East-Cluster") for policy and permission management. |
| **Direct Link** | Transport | A temporary, high-speed data pipe established directly between two Stars for peer-to-peer data transfer (bypassing the Nucleus). |

---

## 3. The Unified Runtime Architecture

To support both **Standalone** and **Grid** modes without code duplication, `magnetar` and `core-semantic` must abstract all I/O operations behind a trait.

### 3.1 The `OrbitSystem` Trait

This is the contract that abstracts the physical world.

```rust
// crate: orbit-core-interface

#[async_trait]
pub trait OrbitSystem: Send + Sync {
    /// 1. Discovery: List files in a directory (fast readdir)
    async fn scan_directory(&self, path: &Path) -> Result<Vec<FileMetadata>>;

    /// 2. Intelligence: Read header bytes to determine Semantic Priority
    async fn read_header(&self, path: &Path, len: usize) -> Result<Vec<u8>>;

    /// 3. Compute: Calculate hash of a specific byte range (CDC)
    ///    - Standalone: Runs on local CPU
    ///    - Grid: Runs on Remote Star CPU (Zero network IO for data)
    async fn calculate_hash(&self, path: &Path, offset: u64, len: u64) -> Result<[u8; 32]>;

    /// 4. Transport: Open a stream to read/write bulk data
    async fn open_stream(&self, path: &Path) -> Result<Box<dyn AsyncRead + Unpin>>;
}
```

### 3.2 Dual Implementations

#### `LocalSystem` (Standalone)
- Wraps `std::fs` and `tokio::fs`.
- Used when running `orbit sync ...`.
- Zero overhead (direct syscalls).

#### `RemoteSystem` (Grid)
- Wraps a **gRPC Client**.
- Used when `orbit-server` (Nucleus) talks to a registered Star.
- Serializes the request, sends to Star, Star executes `LocalSystem` logic, returns result.

---

## 4. Component Specification

### 4.1 Orbit Nucleus (The Central Server)

**Base:** Built upon the existing `crates/orbit-web`.

**State:** Exclusive owner of `jobs.db` (SQLite) and `universe_v3.db` (redb).

**Responsibilities:**
- **Registry:** Maintains the table of active Stars (ID, Status, Capabilities).
- **Planner:** Calculates the DAG (Dependency Graph) for jobs.
- **Locking:** Ensures ACID compliance for all metadata operations.
- **API:** Exposes the Web UI and REST/gRPC endpoints.

### 4.2 Orbit Star (The Remote Agent)

**Base:** A new crate `crates/orbit-star`.

**State:** Stateless. It has no database. It only has configuration (`star.toml`).

**Capabilities:**
- **Scanner:** Uses `jwalk` or platform-native APIs for fast directory listing.
- **CDC Engine:** Embeds `core-cdc` to perform hashing locally.
- **Jail:** Configured with `allowed_paths` to prevent access to the host OS outside specific directories.
- **Deployment:** Single binary (<10MB), static linking.

### 4.3 The Protocol (The Grid)

We will use **gRPC** (via `tonic`) for **Control** and **HTTP/2** (via `axum`/`hyper`) for **Data**.

#### A. Control Plane (Nucleus ↔ Star)

**Proto Definition:** `orbit.proto`

**Handshake:**
- mTLS required. Star presents a signed token to join the Grid.

**Heartbeat:**
- Star reports load (CPU/RAM) and status every 5s.

#### B. Data Plane (Star ↔ Star)

**Scenario:** Transferring a 100GB file from Star A (Source) to Star B (Dest).

**Flow:**
1. Nucleus tells Star B: *"Expect stream `job_123_chunk_456` from Star A."*
2. Nucleus tells Star A: *"Push `local_path/file.dat` (range X-Y) to `StarB_IP:Port`."*
3. Star A pushes bytes directly to Star B. **Data does not pass through the Nucleus.**

---

## 5. Operational Workflows

### Scenario 1: The "Smart Scan" (Discovery)

**Goal:** Detect changes in a 10TB remote directory.

**Legacy Way:**
- Nucleus mounts SMB share.
- Reads 1 million headers over network.
- Takes **4 hours**.

**Grid Way:**
1. Nucleus sends `ScanCommand(path="/data")` to Star.
2. Star executes local NVMe scan (takes **2 minutes**).
3. Star streams back a compressed list of `(Path, Size, ModTime)`.
4. Nucleus updates `magnetar`.

**Result:** 120x faster.

---

### Scenario 2: Distributed Hashing (CDC)

**Goal:** Deduplicate a 50GB VM image.

**Legacy Way:**
- Nucleus reads 50GB over SMB.
- Hashes locally.
- Network: 50GB transferred.

**Grid Way:**
1. Nucleus determines the file needs checking.
2. Nucleus sends `HashCommand(path="vm.vdi", alg="blake3")` to Star.
3. Star reads file locally, chunks it, hashes it.
4. Star sends back only the **hashes** (few KB).
5. Nucleus compares with `universe_v3.db`.

**Result:** 50GB of network traffic saved.

---

## 6. Security Model

The Grid assumes a **Zero-Trust** environment.

### Identity
- Every Star and Nucleus has a unique generated **UUID** and an **x509 certificate**.

### Encryption
- All traffic is **TLS 1.3**.

### Adoption Flow

1. Admin generates a **"Join Token"** on Nucleus UI (valid for 1 hour).
2. Admin installs Star on remote server:
   ```bash
   ./orbit-star join --token <TOKEN> --hub <HUB_IP>
   ```
3. Star generates keys, exchanges with Nucleus, and is **"adopted."**

### Path Restriction

Stars enforce a **chroot-like Allow List**.

**Config:**
```toml
# star.toml
[security]
allowed_paths = ["/mnt/data", "D:\\Projects"]
```

Requests for `/etc/shadow` or `C:\Windows` are **rejected** at the Star level.

---

## 7. Implementation Roadmap

### Phase 1: The Abstraction (Refactor)

**Action:**
- Modify `magnetar` and `core-semantic`.
- Replace `std::fs` calls with `OrbitSystem` trait.

**Deliverable:**
- Standalone Orbit still works, but internal plumbing is ready for injection.

---

### Phase 2: The Star Proto (Definition)

**Action:**
- Create `crates/orbit-proto`.
- Define service `StarService`.
- Implement `tonic` server in a new `orbit-star` binary.

**Deliverable:**
- `orbit-star` binary that can:
  - Accept gRPC commands
  - Execute local I/O operations
  - Report results back to Nucleus

---

### Phase 3: The Nucleus Client (Integration)

**Action:**
- Update `orbit-server` to manage a `HashMap<StarId, GrpcClient>`.
- Implement `RemoteSystem` struct that calls the gRPC client.

**Deliverable:**
- Nucleus can dispatch operations to remote Stars.
- Jobs can execute across the Grid.

---

### Phase 4: The Interface (UI)

**Action:**
- Add **"Agents"** tab to Dashboard.
- Allow selecting a Star as a Source/Destination in the Job Wizard.

**Deliverable:**
- Users can visually manage the Grid topology.
- Job creation supports remote Stars as endpoints.

---

## 8. Migration Strategy

### Existing Users
- Continue using `orbit sync` (Standalone Mode).
- **No breaking changes.**

### Power Users
- Install `orbit-server` (Nucleus).
- Access enhanced features (Web UI, job scheduling).

### Enterprise
- Deploy `orbit-star` agents to NAS/Servers.
- Connect to Nucleus.
- Unlock distributed Grid capabilities.

---

## 9. Benefits Summary

| Capability | Standalone | Grid |
|------------|-----------|------|
| **Zero Install** | ✅ Single binary | ❌ Requires deployment |
| **SMB Performance** | ❌ Slow (network latency) | ✅ Fast (local compute) |
| **Deduplication** | ✅ Local only | ✅ Global across Grid |
| **Scalability** | ❌ Single machine | ✅ Unlimited Stars |
| **Central Management** | ❌ CLI only | ✅ Web UI + API |
| **Direct Star-to-Star Transfer** | ❌ Not applicable | ✅ Peer-to-peer |

---

## 10. Architecture Diagrams

### Standalone Mode (v0.1 - v0.5)

```
┌──────────────────────┐
│   orbit CLI          │
│                      │
│  ┌────────────────┐  │
│  │  LocalSystem   │  │
│  │  (std::fs)     │  │
│  └────────────────┘  │
│         ▼            │
│   Local Disk / SMB   │
└──────────────────────┘
```

### Grid Mode (v0.6+)

```
                    ┌─────────────────────────────────┐
                    │      Orbit Nucleus              │
                    │   (Central Orchestrator)        │
                    │                                 │
                    │  ┌──────────────────────────┐   │
                    │  │  jobs.db (SQLite)        │   │
                    │  │  universe_v3.db (redb)   │   │
                    │  └──────────────────────────┘   │
                    │                                 │
                    │  ┌──────────────────────────┐   │
                    │  │  Star Registry           │   │
                    │  │  - Star A (NAS-1)        │   │
                    │  │  - Star B (NAS-2)        │   │
                    │  │  - Star C (FileServer)   │   │
                    │  └──────────────────────────┘   │
                    └─────────────────────────────────┘
                              │   mTLS/gRPC
                    ┌─────────┴─────────┬─────────────┐
                    ▼                   ▼             ▼
            ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
            │  Orbit Star A │   │  Orbit Star B │   │  Orbit Star C │
            │   (NAS-1)     │   │   (NAS-2)     │   │ (FileServer)  │
            │               │   │               │   │               │
            │ LocalSystem   │   │ LocalSystem   │   │ LocalSystem   │
            │ (Fast I/O)    │   │ (Fast I/O)    │   │ (Fast I/O)    │
            └───────────────┘   └───────────────┘   └───────────────┘
                    │                   │                     │
                    ▼                   ▼                     ▼
            Local NVMe/SSD      Local NVMe/SSD      Local NVMe/SSD
                /mnt/data           /mnt/backup         D:\Projects

            ◄────────────── Direct Link (Peer-to-Peer) ────────────►
```

---

## 11. FAQ

### Q: Does Grid Mode replace Standalone Mode?
**A:** No. Standalone Mode remains the default for single-machine use cases. Grid Mode is opt-in for distributed environments.

### Q: Can I mix Standalone and Grid operations?
**A:** Yes. You can run `orbit sync` locally while also having jobs running on the Grid.

### Q: What happens if the Nucleus goes down?
**A:**
- Running jobs on Stars continue executing (Stars are stateful during job execution).
- New jobs cannot be scheduled until Nucleus recovers.
- Auto-recovery: Systemd restarts Nucleus, resumes jobs.

### Q: How do Stars handle network partitions?
**A:**
- Stars buffer progress locally.
- Upon reconnection, Stars synchronize state with Nucleus.
- Jobs resume from last checkpoint (magnetar's resume system).

### Q: Can Stars talk directly to each other?
**A:** Yes. For large transfers, Stars establish **Direct Links** (peer-to-peer) to avoid routing through the Nucleus.

### Q: What about authentication and authorization?
**A:**
- **Authentication:** mTLS certificates (mutual authentication).
- **Authorization:** Role-Based Access Control (RBAC) enforced by Nucleus.
- Stars only accept commands from authenticated Nucleus instances.

---

## 12. Technical Constraints

### Performance Requirements
- **Star Heartbeat Latency:** < 10ms
- **Command Dispatch Latency:** < 50ms
- **Direct Link Throughput:** > 1GB/s (on 10GbE)

### Resource Limits
- **Nucleus:** Handle up to 1000 concurrent Stars
- **Star:** < 50MB RAM footprint when idle
- **Protocol:** gRPC message size limit: 4MB (streaming for larger payloads)

### Compatibility
- **Minimum Rust Version:** 1.75+
- **Supported Platforms:** Linux (x64/ARM64), Windows (x64), macOS (Intel/Apple Silicon)
- **Network:** IPv4/IPv6, mTLS 1.3

---

## 13. Open Questions

1. **Dynamic Star Discovery:** Should Stars support mDNS/Bonjour for local network auto-discovery?
2. **Star-to-Star Encryption:** Should Direct Links use TLS or a lighter protocol (e.g., Noise Protocol)?
3. **Billing/Metering:** Should the Grid track per-Star resource usage for multi-tenant environments?
4. **Observability:** Should we embed OpenTelemetry for distributed tracing?

---

## Conclusion

This specification ensures Orbit remains a **single, cohesive project** while gaining the capabilities of an **enterprise-grade distributed system**.

By abstracting I/O operations behind the `OrbitSystem` trait, we achieve:
- **Zero code duplication** between Standalone and Grid modes
- **Zero performance overhead** for Standalone users
- **Maximum flexibility** for Enterprise deployments

The Grid architecture positions Orbit as a modern **Data Fabric** capable of operating at planetary scale while remaining simple enough to deploy on a Raspberry Pi.

---

**Status:** 🚧 DRAFT (Pending community feedback)
**Target Release:** v0.6.0
**Maintainer:** Shane Wall <shaneawall@gmail.com>
**License:** Apache-2.0
