# 🗂 Orbit Manifest System

> **Define, plan, and audit** complex data transfers with precision.

---

## 💡 Overview

The **Manifest System** is the heart of Orbit’s automation engine.  
It transforms manual copy commands into reusable, declarative jobs defined in a single TOML file.  
A manifest not only describes *what* to move — but also *how* and *when* — while integrating with Orbit’s **Starmap** and **Audit** layers for planning and accountability.

---

## 🧠 Core Components

| Component | Role |
|------------|------|
| 🗂 **Manifest Engine** | Parses and executes declarative job definitions |
| 🌌 **Starmap** | The execution planner that orders, validates, and maps jobs to modules |
| 📊 **Audit System** | Collects and records structured telemetry for every operation |
| 🧩 **Core Modules** | Compression, checksum, and zero-copy subsystems called by Starmap |

Together, these form the **Manifest Pipeline**:
```
TOML manifest
   ↓
Parser & Validator
   ↓
Starmap (Execution Plan)
   ↓
Core Modules (Copy, Compress, Verify)
   ↓
Audit System (Structured Logs)
```

---

## 🧩 Design Philosophy

The Manifest System was built around three principles:

1. **Declarative over imperative** — users describe desired outcomes, not shell commands  
2. **Planned over ad-hoc** — every run passes through the Starmap planner for validation and ordering  
3. **Observable over opaque** — every action emits structured audit events for traceability  

---

## 📄 Current Workflow (v0.4.0)

The manifest system in v0.4.0 supports transfer planning, verification, and auditing:

### 1. Create a Flight Plan

Generate manifests without transferring data:

```bash
# Create flight plan for a transfer
orbit manifest plan --source /data/source --dest /mnt/backup --output ./manifests

# This creates:
#   ./manifests/job.flightplan.json     (transfer metadata)
#   ./manifests/*.cargo.json            (per-file manifests with chunks)
```

### 2. Execute Transfer with Manifest Generation

Perform the actual transfer while generating/updating manifests:

```bash
orbit --source /data/source --dest /mnt/backup \
  --recursive \
  --generate-manifest \
  --manifest-dir ./manifests
```

### 3. Verify Transfer

Verify the transfer was successful:

```bash
orbit manifest verify --manifest-dir ./manifests
```

### 4. Check Differences

Compare manifests with target directory:

```bash
orbit manifest diff --manifest-dir ./manifests --target /mnt/backup
```

---

## 🚧 Planned: Declarative Manifest Execution (v0.6.0+)

The full declarative manifest system with `[[job]]` entries and `orbit run` is planned for v0.6.0:

```toml
# orbit.manifest.toml (PLANNED - not yet implemented)

[defaults]
checksum = "sha256"
compression = "zstd:6"
resume = true
concurrency = 4
audit_log = "audit.log"
plan_visualisation = true

[[job]]
name = "source-sync"
source = "/data/source/"
destination = "/mnt/backup/source/"
include = ["**/*.rs", "**/*.toml"]
exclude = ["target/**", ".git/**"]

[[job]]
name = "media-archive"
source = "/media/camera/"
destination = "/tank/archive/"
compression = "zstd:1"
depends_on = ["source-sync"]  # Job ordering
```

**Planned execution:**
```bash
orbit run --manifest orbit.manifest.toml  # Coming in v0.6.0+
```

---

## 🌌 Starmap: The Planner

**Starmap** is Orbit’s internal execution planner.  
It reads the manifest, resolves dependencies, and produces an ordered, validated plan before any transfer begins.

### Responsibilities

- 🧭 Validates paths, options, and dependencies  
- 🪐 Builds a directed graph of jobs and execution order  
- 🧮 Checks free space and resource constraints (planned)  
- 🔁 Optimises concurrency by grouping non-overlapping jobs  
- 🧱 Provides a visual or logged plan (when enabled via `plan_visualisation = true`)

### Example Starmap Output

```
📡 Orbit Starmap Plan
──────────────────────────────────────────────
1. source-sync       (checksum=sha256, compression=zstd:6)
2. media-archive     → depends_on: source-sync
──────────────────────────────────────────────
Jobs ready: 2
Parallel groups: 1
Estimated runtime: ~4m12s
──────────────────────────────────────────────
```

Starmap ensures your jobs run in a **predictable and safe order**, avoiding conflicts and resource contention.

---

## 📊 Audit Integration

The Manifest System is tightly coupled with the **core-audit** crate.  
Every job execution emits structured audit events for analysis, troubleshooting, and compliance.

### Audit Flow

```
Manifest Job
   ↓
Starmap Execution
   ↓
Audit Event Stream
   ↓
audit.log (JSONL / CSV)
```

### Example Audit Event

```json
{
  "timestamp": "2025-10-18T16:42:19Z",
  "job": "media-archive",
  "source": "/media/camera/",
  "destination": "/tank/archive/",
  "bytes": 104857600,
  "duration_ms": 2341,
  "compression": "zstd:1",
  "checksum": "sha256",
  "checksum_match": true,
  "status": "ok",
  "retries": 0,
  "starmap_node": "orbit.node.media-archive"
}
```

Audit data is:
- Written as JSON Lines for easy ingestion  
- Timestamped with job and file context  
- Enriched with Starmap metadata (node, dependencies, result)  
- Emittable via stdout or telemetry channel for real-time dashboards  

---

## 🧰 Planned: Manifest Keys (v0.6.0+)

> **Note:** The following sections describe the planned declarative manifest system for v0.6.0+. The current v0.4.0 release uses flight plans generated via `orbit manifest plan` rather than declarative TOML manifests.

**Planned manifest structure:**

| Section | Key | Type | Description |
|----------|-----|------|-------------|
| `[defaults]` | `checksum` | string | Hash algorithm (`sha256`, `none`) |
|  | `compression` | string | `lz4`, `zstd`, or `zstd:LEVEL` |
|  | `resume` | bool | Resume interrupted transfers |
|  | `audit_log` | path | Path for audit output |
|  | `concurrency` | int | Parallel file workers |
|  | `plan_visualisation` | bool | Show Starmap before execution |
| `[[job]]` | `name` | string | Unique job name |
|  | `source` | path | Source path or URI |
|  | `destination` | path | Destination path or URI |
|  | `include` | array | Include globs |
|  | `exclude` | array | Exclude globs |
|  | `compression` | string | Overrides default |
|  | `checksum` | string | Overrides default |
|  | `depends_on` | array | Upstream job names |
|  | `resume` | bool | Enable resume per job |

---

## ⚙️ Planned: Execution Flow (v0.6.0+)

**When `orbit run --manifest` is implemented:**

1. **Parse** manifest file
2. **Validate** syntax, keys, and paths
3. **Build Starmap** job graph
4. **Preflight audit** record created
5. **Execute jobs** in dependency order
6. **Emit per-job audit logs**
7. **Generate completion summary**

---

## 🧩 Planned: Error Handling and Recovery (v0.6.0+)

**Planned features:**

- 🛑 If a job fails, dependent jobs are paused automatically
- 🧩 Partial progress is recorded in audit logs
- 🔁 Rerunning the same manifest resumes from failed jobs only
- ⚠️ Warnings (non-critical) are logged but do not halt Starmap execution
- 🧮 Checkpoint restoration directly from audit logs

---

## 🧮 Planned Example: Multi-Stage Backup with Dependencies

```toml
[defaults]
checksum = "sha256"
compression = "zstd:3"
resume = true
concurrency = 4
plan_visualisation = true

[[job]]
name = "sync-source"
source = "/data/source/"
destination = "/mnt/backup/source/"

[[job]]
name = "compress-archive"
source = "/mnt/backup/source/"
destination = "/mnt/archive/source.zst"
compression = "zstd:9"
depends_on = ["sync-source"]

[[job]]
name = "offload-to-nas"
source = "/mnt/archive/source.zst"
destination = "smb://nas/backup/source.zst"
depends_on = ["compress-archive"]
```

Execution order (Starmap will display):
```
1. sync-source
2. compress-archive (→ sync-source)
3. offload-to-nas (→ compress-archive)
```

---

## 🧭 Future Features

- 🕒 **Job scheduling** inside manifests  
- 🧠 **Conditional triggers** (run if newer, skip if checksum matches)  
- 📦 **Parameter injection** from environment variables  
- ⚙️ **Pre/post hooks** for integration with external scripts  
- 📡 **Remote execution** via Orbit agents  
- 🧩 **Starmap visualisation** export to GraphViz or JSON schema  

---

## 🧑‍💻 Developer Notes

- The **`core-manifest`** crate handles parsing and validation  
- The **`core-starmap`** module (inside it) builds DAGs of jobs  
- The **`core-audit`** crate writes telemetry events and logs  
- All manifest runs produce a single `audit.log` file by default  
- Manifest execution is thread-safe and composable  

---

## 🤝 Best Practices

- Keep manifests in version control
- Test transfers with `--dry-run` flag before production runs
- Use `orbit manifest plan` to preview transfer operations before executing
- Use `orbit manifest verify` after transfers to ensure integrity
- Periodically archive audit log outputs for compliance
- Use descriptive naming for manifest directories (e.g., `./manifests/backup-2025-01`)  

---

## 📜 Summary

The **Manifest System** + **Starmap** + **Audit** integration transforms Orbit into an orchestrated, observable data transfer platform.  
It provides:
- Declarative and auditable workflows  
- Intelligent execution planning  
- Safe recovery and dependency management  

✨ *Define your data movement as code — let Orbit handle the rest.*
