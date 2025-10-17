## OVERVIEW.md

### Purpose
Make manifests a first-class part of ORBIT. They provide a stable control plane for planning, verification, resume, audit, and policy across any transport module, including Wormhole.

### Concepts and names
- **Flight Plan** (job-level): Human readable plan and policy for a transfer job. Signed optionally.
- **Cargo Manifest** (per object): Human readable description of one file or logical object. Includes chunking, windows, and digests.
- **Star Map** (binary index): Compact, memory-mappable index for chunks, windows, blooms, and bitmaps. Optimised for the engine.
- **Telemetry Log** (audit): Append-only JSON Lines of events for compliance and forensics.
- **Beacon** (signature): Detached signature and minimal summary that binds digests and policy.
- **Ephemeris** (progress): Small rotating resume snapshot. Safe to delete. Rebuilt from Star Map if lost.

### Why split control and data planes
- **Human clarity:** Flight Plan and Cargo Manifest are easy to read, diff, and review.
- **Machine speed:** Star Map avoids parser overhead, supports zero-copy, and answers “what is missing” in constant time.
- **Transport agnostic:** Any sender or receiver module can implement the same contract.

### Core benefits
- **Verification:** End-to-end integrity without rereading all bytes. Window-level confirms via Merkle.
- **Resume and de-dup:** Content IDs and compressed bitmaps allow precise restarts and skip work.
- **Planning:** Dry-run capacity and time ranges before any data moves.
- **Audit:** Chain-of-custody ready. Hand to an auditor with a signature.
- **Policy:** Security and retention tags bind into crypto. Prevents silent policy drift.

### Default formats
- Flight Plan and Cargo Manifest: **JSON**, optionally compressed at rest with zstd.
- Star Map: **Cap’n Proto** or **FlatBuffers**. Cap’n Proto shown here.
- Telemetry Log: **JSON Lines**.
- Beacon: detached signature files (PGP or age).

### Operational flow
1. **Plan:** Build Cargo Manifests and Star Maps. Emit Flight Plan. Optionally sign.
2. **Preflight:** Send Flight Plan to the receiver. Perform capacity and policy checks. Return an acknowledgement.
3. **Transfer:** Wormhole streams bytes. Uses Star Map for order and fast missing-queries. Emits Telemetry.
4. **Commit:** Receiver validates Merkle roots per window. Records file digests. Emits Beacon with final job digest.
5. **Close-out:** Export analytics if enabled. Archive manifests and audit under `manifests/<job_id>/`.

### Wormhole integration highlights
- Uses Star Map to choose next chunk quickly and to avoid head-of-line stalls.
- Emits `window_ok` events that reference `window.id` and `merkle_root` from Cargo Manifest.
- Writes Ephemeris locally every few seconds for robust resume after crash or blackout.
- Uses policy in Flight Plan to select AEAD and to bind security tags as associated data.
- Optional parity tuning uses prior Telemetry to warm start K for similar paths.

### Directory layout on disk
```
/var/lib/orbit/manifests/
  job-2025-10-17T06_25Z/
    job.flightplan.json
    vm_big.vmdk.cargo.json
    vm_big.vmdk.starmap.bin
    audit.jsonl
    beacon.json            # optional summary
    job.flightplan.json.asc
    beacon.json.asc
```

### CLI user stories
```
# Plan and review without sending data
orbit manifest plan --source /data/in --target /data/out --out /tmp/job

# Verify a completed run quickly
orbit manifest verify --manifests /var/lib/orbit/manifests/job-.../

# Reconcile a target and send only deltas
orbit manifest diff --manifests /var/lib/orbit/manifests/job-.../ --target /data/out

# Run through Wormhole using existing manifests
orbit wormhole send --dest 10.0.0.2:8443 --flight /tmp/job/job.flightplan.json
```

---

## SCHEMA.md

This section contains pragmatic schemas and examples. Keep them stable. Bump `schema` on breaking change. Provide upgraders when needed.

### Flight Plan (JSON)
```json
{
  "schema": "orbit.flightplan.v1",
  "job_id": "job-2025-10-17T06:25:00Z",
  "created_utc": "2025-10-17T06:25:00Z",
  "source": {"type": "fs", "root": "/data/in", "fingerprint": "src-b7b2..."},
  "target": {"type": "fs", "root": "/data/out", "fingerprint": "dst-49a1..."},
  "policy": {
    "encryption": {"aead": "aes256-gcm", "key_ref": "env:ORBIT_KEY"},
    "retention_days": 180,
    "redaction_profile": "none",
    "verify_on_arrival": true,
    "classification": "OFFICIAL:Sensitive"
  },
  "capacity_vector": {
    "bytes_total": 1234567890,
    "bytes_unique": 1200000000,
    "est_overhead_pct": 7.5,
    "eta_minutes": {"clean": 18, "moderate": 24, "rough": 39}
  },
  "files": [
    {"path": "vm/big.vmdk", "cargo": "sha256:...-cargo.json", "starmap": "sha256:...-starmap.bin"},
    {"path": "logs/2025-10-17.tar.zst", "cargo": "sha256:...-cargo.json", "starmap": "sha256:...-starmap.bin"}
  ],
  "job_digest": null
}
```

#### JSON Schema (abbreviated)
```json
{
  "$id": "https://orbit.io/schemas/flightplan.v1.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["schema", "job_id", "created_utc", "source", "target", "policy", "files"],
  "properties": {
    "schema": {"const": "orbit.flightplan.v1"},
    "job_id": {"type": "string"},
    "created_utc": {"type": "string", "format": "date-time"},
    "source": {"$ref": "#/defs/endpoint"},
    "target": {"$ref": "#/defs/endpoint"},
    "policy": {"$ref": "#/defs/policy"},
    "capacity_vector": {"$ref": "#/defs/capacityVector"},
    "files": {
      "type": "array",
      "items": {"$ref": "#/defs/fileRef"},
      "minItems": 1
    },
    "job_digest": {"type": ["string", "null"]}
  },
  "defs": {
    "endpoint": {
      "type": "object",
      "required": ["type", "root"],
      "properties": {
        "type": {"enum": ["fs", "s3", "smb", "custom"]},
        "root": {"type": "string"},
        "fingerprint": {"type": "string"}
      }
    },
    "policy": {
      "type": "object",
      "properties": {
        "encryption": {"type": "object", "properties": {"aead": {"type": "string"}, "key_ref": {"type": "string"}}},
        "retention_days": {"type": "integer", "minimum": 0},
        "redaction_profile": {"type": "string"},
        "verify_on_arrival": {"type": "boolean"},
        "classification": {"type": "string"}
      },
      "required": ["encryption"]
    },
    "capacityVector": {
      "type": "object",
      "properties": {
        "bytes_total": {"type": "integer"},
        "bytes_unique": {"type": "integer"},
        "est_overhead_pct": {"type": "number"},
        "eta_minutes": {"type": "object", "properties": {"clean": {"type": "integer"}, "moderate": {"type": "integer"}, "rough": {"type": "integer"}}}
      }
    },
    "fileRef": {
      "type": "object",
      "required": ["path", "cargo"],
      "properties": {
        "path": {"type": "string"},
        "cargo": {"type": "string"},
        "starmap": {"type": "string"}
      }
    }
  }
}
```

### Cargo Manifest (JSON)
```json
{
  "schema": "orbit.cargo.v1",
  "path": "vm/big.vmdk",
  "size": 1099511627776,
  "chunking": {"type": "cdc", "avg_kib": 256, "algo": "gear"},
  "digests": {"blake3": "...", "sha256": "..."},
  "windows": [
    {"id": 0, "first_chunk": 0, "count": 64, "merkle_root": "...", "overlap": 4},
    {"id": 1, "first_chunk": 60, "count": 64, "merkle_root": "...", "overlap": 4}
  ],
  "xattrs": {"mode": "0644", "owner": "root", "mtime": "2025-10-17T06:00:00Z"},
  "file_digest": null
}
```

#### JSON Schema (abbreviated)
```json
{
  "$id": "https://orbit.io/schemas/cargo.v1.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["schema", "path", "size", "chunking", "windows"],
  "properties": {
    "schema": {"const": "orbit.cargo.v1"},
    "path": {"type": "string"},
    "size": {"type": "integer", "minimum": 0},
    "chunking": {"type": "object", "properties": {"type": {"enum": ["cdc", "fixed"]}, "avg_kib": {"type": "integer"}, "algo": {"type": "string"}, "fixed_kib": {"type": "integer"}}, "required": ["type"]},
    "digests": {"type": "object", "properties": {"blake3": {"type": "string"}, "sha256": {"type": "string"}}},
    "windows": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "first_chunk", "count", "merkle_root"],
        "properties": {
          "id": {"type": "integer", "minimum": 0},
          "first_chunk": {"type": "integer", "minimum": 0},
          "count": {"type": "integer", "minimum": 1},
          "merkle_root": {"type": "string"},
          "overlap": {"type": "integer", "minimum": 0}
        }
      },
      "minItems": 1
    },
    "xattrs": {"type": "object"},
    "file_digest": {"type": ["string", "null"]}
  }
}
```

### Star Map (Cap’n Proto)
```capnp
@0xbed1_0f11_5e7c_0c0a;

struct StarMap {
  version @0 :UInt16;
  fileSize @1 :UInt64;
  chunkCount @2 :UInt32;
  windowCount @3 :UInt32;
  chunks @4 :List(ChunkEntry);
  windows @5 :List(WindowEntry);
  bloom   @6 :Data;      # serialised bloom filter
  bitmaps @7 :List(Data); # rank-select bitmaps per window
}

struct ChunkEntry {
  offset @0 :UInt64;
  len    @1 :UInt32;
  cid    @2 :Data;   # 32-byte blake3 content id
}

struct WindowEntry {
  id      @0 :UInt32;
  first   @1 :UInt32;
  count   @2 :UInt16;
  merkle  @3 :Data;  # 32-byte root
  overlap @4 :UInt16; # preceding overlap
}
```

### Telemetry Log (JSON Lines examples)
```json
{"ts":"2025-10-17T06:25:00Z","job":"job-...","event":"plan","files":3,"bytes":123456}
{"ts":"2025-10-17T06:27:11Z","job":"job-...","event":"window_ok","path":"vm/big.vmdk","id":0,"bytes":16777216,"repair":2}
{"ts":"2025-10-17T06:30:00Z","job":"job-...","event":"job_digest","digest":"sha256:...","files":3,"bytes":...}
```

### Beacon (summary + signature)
```json
{
  "schema": "orbit.beacon.v1",
  "job_id": "job-2025-10-17T06:25:00Z",
  "job_digest": "sha256:...",
  "signer": "CN=ORBIT-BUILD-SIGNER",
  "ts": "2025-10-17T06:31:14Z",
  "policy": {"classification": "OFFICIAL:Sensitive"}
}
```

---

## Rust scaffolding (sketch)

Create crates or modules as you prefer. Workspace style shown.

```
/crates/core-manifest
  src/lib.rs
  src/flightplan.rs
  src/cargo.rs
  src/validate.rs
  src/sign.rs

/crates/core-starmap
  build.rs               # capnp compile
  src/lib.rs
  src/bitmap.rs          # rank-select helper
  src/bloom.rs

/crates/core-audit
  src/lib.rs

/crates/wormhole
  src/...
```

### `core-manifest` minimal types
```rust
// crates/core-manifest/src/lib.rs
pub mod flightplan; pub mod cargo; pub mod validate; pub mod sign;

pub type Digest = String; // later: newtype with algo prefix

#[derive(thiserror::Error, Debug)]
pub enum Error { #[error("validation: {0}")] Validate(String), #[error("io: {0}")] Io(#[from] std::io::Error) }

pub type Result<T> = std::result::Result<T, Error>;
```

```rust
// crates/core-manifest/src/flightplan.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FlightPlan { pub schema: String, pub job_id: String, pub created_utc: String, pub source: Endpoint, pub target: Endpoint, pub policy: Policy, pub capacity_vector: Option<CapacityVector>, pub files: Vec<FileRef>, pub job_digest: Option<String>, }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Endpoint { pub r#type: String, pub root: String, pub fingerprint: Option<String> }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Policy { pub encryption: Encryption, pub retention_days: Option<u32>, pub redaction_profile: Option<String>, pub verify_on_arrival: Option<bool>, pub classification: Option<String> }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Encryption { pub aead: String, pub key_ref: String }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CapacityVector { pub bytes_total: u64, pub bytes_unique: u64, pub est_overhead_pct: f32, pub eta_minutes: Option<Eta> }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Eta { pub clean: Option<u32>, pub moderate: Option<u32>, pub rough: Option<u32> }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileRef { pub path: String, pub cargo: String, pub starmap: Option<String> }
```

```rust
// crates/core-manifest/src/cargo.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CargoManifest { pub schema: String, pub path: String, pub size: u64, pub chunking: Chunking, pub digests: Option<Digests>, pub windows: Vec<WindowMeta>, pub xattrs: Option<serde_json::Value>, pub file_digest: Option<String>, }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chunking { pub r#type: String, pub avg_kib: Option<u32>, pub algo: Option<String>, pub fixed_kib: Option<u32> }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Digests { pub blake3: Option<String>, pub sha256: Option<String> }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowMeta { pub id: u32, pub first_chunk: u32, pub count: u16, pub merkle_root: String, pub overlap: Option<u16> }
```

### `core-starmap` usage notes
- Build Cap’n Proto schema at compile time with `capnp` crate and `build.rs`.
- Provide a reader that wraps an `mmap` and yields:
  - `has_chunk(cid)` using the bloom filter
  - `next_missing(window_id)` using rank-select
  - `window_meta(window_id)`

### Wormhole hooks
- Accept `FlightPlan` or generate it when absent.
- Use `CargoManifest.windows[*].merkle_root` for window verification.
- Use `StarMap` for chunk scheduling and resume.
- Bind `policy.classification` into AEAD associated data.
- Emit Telemetry Log. Optionally export a Parquet summary out of band.

---

## Validation and security
- Validate JSON against JSON Schema before use. Fail early.
- Star Map contains a magic and CRC to detect corruption fast.
- Detached signatures sign Flight Plan and final `job_digest`. Keys stay outside manifests.
- Time is UTC. Paths are NFC normalised. Handle sparse files and platform metadata as optional sidecars.

---

## Testing plan
- Golden vectors for Merkle, windowing, and AEAD.
- Fault injector for loss, jitter, reorder, MTU shift, and blackout.
- Deterministic seed to reproduce schedules and window sets.
- Benchmarks for `next_missing` latency and memory footprint.

---

## Defaults
- Control plane: JSON in repo, readable by humans.
- Data plane: Cap’n Proto for speed.
- Audit: JSONL, one event per line.
- Signatures: off by default. Single flag to enable.
- Compression: off for control docs, on for Star Map if it grows large.

---

## Summary
This design gives ORBIT a clear control plane and a fast data plane. Wormhole benefits immediately: faster resume, precise repair, and strong, auditable integrity without operational friction. Copy this file into `/docs/manifest/` and start scaffolding the Rust crates as shown.

