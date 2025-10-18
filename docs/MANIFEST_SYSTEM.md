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

## 📄 Manifest Structure

A manifest defines:
1. Optional `[defaults]` section  
2. One or more `[[job]]` entries  

### Example

```toml
# orbit.manifest.toml

[defaults]
checksum = "sha256"
compression = "zstd:6"
resume = true
concurrency = 4
audit_log = "audit.log"
plan_visualisation = true  # renders the Starmap for inspection

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
checksum = "sha256"
resume = true
depends_on = ["source-sync"]  # Job ordering handled by Starmap
```

Run it:
```bash
orbit run --manifest orbit.manifest.toml
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

## 🧰 Supported Keys

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

## ⚙️ Execution Flow

1. **Parse** manifest file  
2. **Validate** syntax, keys, and paths  
3. **Build Starmap** job graph  
4. **Preflight audit** record created  
5. **Execute jobs** in dependency order  
6. **Emit per-job audit logs**  
7. **Generate completion summary**

---

## 🧩 Error Handling and Recovery

- 🛑 If a job fails, dependent jobs are paused automatically  
- 🧩 Partial progress is recorded in audit logs  
- 🔁 Rerunning the same manifest resumes from failed jobs only  
- ⚠️ Warnings (non-critical) are logged but do not halt Starmap execution  
- 🧮 Future versions will support *checkpoint restoration* directly from audit logs  

---

## 🧮 Example: Multi-Stage Backup with Dependencies

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
- Validate with `orbit run --manifest file.toml --dry-run`  
- Use clear `name` fields for jobs to simplify audit tracking  
- Enable `plan_visualisation = true` before first run  
- Periodically archive `audit.log` outputs for compliance  

---

## 📜 Summary

The **Manifest System** + **Starmap** + **Audit** integration transforms Orbit into an orchestrated, observable data transfer platform.  
It provides:
- Declarative and auditable workflows  
- Intelligent execution planning  
- Safe recovery and dependency management  

✨ *Define your data movement as code — let Orbit handle the rest.*
