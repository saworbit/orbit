# Orbit - Workspace Structure

This document describes the workspace layout for the Orbit project.

---

## Workspace Members

The Orbit workspace (`Cargo.toml`) includes the following crates:

| Crate | Path | Purpose |
|-------|------|---------|
| `orbit` | `.` (root) | Main binary and CLI |
| `core-manifest` | `crates/core-manifest` | Manifest generation and parsing |
| `core-starmap` | `crates/core-starmap` | Star Map and Universe index |
| `core-audit` | `crates/core-audit` | Audit logging and provenance events |
| `core-cdc` | `crates/core-cdc` | Content-Defined Chunking (Gear Hash) |
| `core-semantic` | `crates/core-semantic` | Semantic file classification and prioritizers |
| `orbit-core-interface` | `crates/orbit-core-interface` | Core trait abstractions |
| `orbit-observability` | `crates/orbit-observability` | Telemetry and observability |

---

## Directory Tree

```
orbit/
├── .github/
│   └── workflows/
│       └── ci.yml
├── crates/
│   ├── core-audit/
│   ├── core-cdc/
│   ├── core-manifest/
│   ├── core-semantic/
│   ├── core-starmap/
│   ├── orbit-core-interface/
│   └── orbit-observability/
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── error.rs
│   ├── config.rs
│   ├── audit.rs
│   ├── core/
│   │   ├── mod.rs
│   │   ├── checksum.rs
│   │   ├── guidance.rs
│   │   ├── probe.rs
│   │   ├── resume.rs
│   │   ├── metadata.rs
│   │   └── ...
│   └── compression/
│       └── mod.rs
├── tests/
│   └── ...
├── docs/
│   ├── README.md
│   ├── architecture/
│   ├── guides/
│   ├── specs/
│   └── release/
├── Cargo.toml
├── Cargo.lock
├── LICENSE
└── README.md
```

---

## Verification

```bash
# Verify workspace compiles
cargo check

# Run all tests
cargo test

# Check formatting
cargo fmt --check

# Lint
cargo clippy
```
