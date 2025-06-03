# 🌌 Orbit

**Open Resilient Bulk Information Transfer**

Orbit is a modern, open-source file transfer engine built from the ground up in Rust. Inspired by tools like `rsync`, `rclone`, `xcopy`, and `robocopy`, Orbit aims to unify and modernise data movement with a cross-platform, protocol-agnostic engine for seamless, resilient, and intelligent file transfers. Our goal is simple: make moving data fast, reliable, and universal.

> Because data has gravity — and gravity shapes architecture.

---

## 🔹 Why Orbit?

After years in the field working with enterprise storage systems, we realised that moving data between platforms, vendors, and protocols still relies heavily on outdated tools and brittle scripts. Most modern environments are hybrid, decentralised, and unpredictable — and the tooling hasn’t kept up.

**Orbit** is an answer to that.

Designed to be open, extensible, and built for real-world challenges, Orbit is:
- A smarter, modular alternative to Robocopy, rsync, and xcopy
- Cross-platform from day one
- Resilient to failure, dropouts, interruptions, and delays
- Built in Rust for performance, portability, and safety

---

## 🌐 Architecture: Data Movement Inspired by Physics

Orbit mirrors the structure of real-world orbital systems:

- **🪙 Nexus**: The core engine. This is the central logic, responsible for discovery, transfer, integrity validation, logging, and orchestration.
- **🚁 Satellites** *(future)*: Optional lightweight agents deployed to endpoints or edge nodes for secure, encrypted, or accelerated data handling.
- **🔗 DOCKs (Data-Orbital Connection Kits)** *(planned)*: Plugin-like modules that extend Nexus. Each DOCK can support a new protocol, feature, or vendor-specific integration. Think of them as spacecraft modules docking into the central system.
- **🌍 Gravity**: The core philosophy. Data pulls systems toward it. Orbit is built to flow with that force, not fight it.

---

## ✅ What Orbit Can Do Today

### 🔄 Core File Operations
- Copy files locally or across network paths
- Byte-for-byte SHA-256 verification
- Cross-platform support (Windows, Linux, macOS)
- File size and timestamp validation

### 📦 Compression & Bandwidth Optimisation
- Fast LZ4 compression with decompression on receive
- Smart compress → transfer → decompress workflows
- Compression ratio and transfer stats
- Temporary file cleanup

### 🔄 Resiliency & Reliability
- Resume interrupted transfers from checkpoint
- Save progress every 5 seconds
- Configurable retry logic and backoff
- Robust handling for flaky networks, I/O faults, and power loss

### ⚙️ Performance & Configurability
- Chunk size tuning (64KB to 4MB+)
- Buffered I/O
- Disk space pre-checks
- Real-time progress output

### 📊 Monitoring & Audit
- Full audit log with timestamped operations
- Success/failure state tracking with reason codes
- Detailed error context
- Statistics: bytes transferred, duration, compression ratio

### 📲 CLI-First Experience
- Intuitive syntax:
  ```bash
  orbit ./source /dest/path --compress --retry 3 --verify
  ```
- Built-in help and version display

---

## 🔢 Roadmap

### 🚀 Phase 1: Core Foundation
- [x] Core engine (Nexus) with resumable transfer
- [x] Compression and checksum verification
- [x] CLI interface
- [ ] Directory walking, symbolic links
- [ ] Metadata preservation (ACLs, permissions)
- [ ] SMB and network share support

### ☁️ Phase 2: Protocol Expansion
- [ ] Multi-threaded parallelisation
- [ ] Cloud support (S3, Azure Blob)
- [ ] Sync modes (mirror, delta)
- [ ] File watchers for change detection
- [ ] DOCKs (plugin system)

### 🚁 Phase 3: Enterprise-Grade
- [ ] Satellites (optional endpoint agents)
- [ ] Secure key exchange and agent-managed encryption
- [ ] REST API and external integration
- [ ] GUI/Web dashboard

---

## 📱 How It Started

Orbit was born out of frustration and field experience. Moving large datasets between platforms, vendors, and storage systems often involved a mix of brittle scripts, Robocopy hacks, or ad-hoc rsync tunnels. We wanted something better:

- A single, modular tool
- Not bound to any OS or ecosystem
- Open-source and extensible

Orbit is the tool we wish we had. So we’re building it.

---

## 🚪 Why Open Source?

Because data belongs to users, and the tools to move it should too.

We want Orbit to be:
- Free to use and inspect
- Easy to extend for new use cases
- A foundation for community-driven innovation
- A stepping stone toward commercial enterprise features

---

## 🙌 Get Involved

We welcome contributions of all kinds:
- Build a DOCK (plugin) for a new protocol
- Help test Orbit in your environment
- Suggest features or raise issues
- Improve docs or CLI help

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) to get started.

---

## 📜 License

This project is licensed under the MIT License — see [`LICENSE`](./LICENSE).

Orbit is and will remain open-source. A separate commercial edition with advanced capabilities may be introduced in future.

---

**Move anything. Anywhere. Reliably.**

Orbit — because data has gravity.
