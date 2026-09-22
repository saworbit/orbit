# Orbit Reboot Foundation and Read-Only Planner Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Archive the experimental Orbit codebase, replace it with a minimal single-crate Rust foundation, and deliver a deterministic `orbit plan SOURCE DEST` command that performs no writes.

**Architecture:** The first milestone stops at the read-only boundary: CLI arguments become a typed request, paths are resolved and validated, the source is scanned without following links, destination entries are inspected, and a deterministic plan is rendered for humans or as versioned JSON. The crate has concrete local-filesystem modules and no backend trait, executor, journal, service, or plugin surface.

**Tech Stack:** Rust 2024 edition with MSRV 1.85.0; `clap`, `serde`, `serde_json`, `thiserror`, and `blake3`; `assert_cmd`, `predicates`, and `tempfile` for tests; GitHub Actions on Windows, Linux, and macOS.

## Global Constraints

- Work on branch `codex/orbit-reboot`; never rewrite or force-push `main`.
- Preserve the pre-reboot repository at annotated tag `archive/pre-reboot-2026-09-22` before deleting tracked legacy files.
- v0.1 accepts only local filesystem paths, including OS-mounted shares.
- Windows on NTFS is the release-gating platform; Linux and macOS run the portable subset.
- Keep one Rust crate, one binary, one toolchain, and one deployment unit.
- Keep CLI formatting at the boundary; internal modules exchange typed requests, plans, and errors.
- `DEST` is always the exact target path and is never reinterpreted as “copy beneath this directory.”
- Planning never changes the source, destination, or Orbit state.
- Equal file size never proves equality; BLAKE3 content hashes are required before `skip_identical`.
- Directory plans never delete unrelated destination entries.
- Do not add native network protocols, sync, mirror, move, deletion, compression, encryption, delta transfer, deduplication, configuration files, plugins, FFI, IPC, or services.
- Do not introduce a generic storage backend or filesystem abstraction.
- Use test-driven development: observe each focused test fail before adding its implementation.
- Use `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets` as the local quality gate.

## Scope Decomposition

This plan implements delivery slices 1–3 from the approved design: archive/reset, typed CLI/domain foundation, and read-only scan/plan. Later plans will separately cover:

1. staged copy, read-back verification, and safe replacement;
2. durable journal, cancellation, and resume;
3. Windows metadata fidelity, fault injection, packaging, and the complete `0.1.0-alpha` release gates.

The `copy` subcommand is intentionally not exposed by this milestone. It is introduced only when the safe executor exists, so no released command can imply that it copies while containing a stub.

## Target File Map

```text
Cargo.toml                         package metadata and minimal dependencies
Cargo.lock                         reproducible dependency resolution
rust-toolchain.toml                Rust 1.85.0 toolchain and components
src/lib.rs                         module exports
src/main.rs                        process boundary only
src/cli.rs                         clap types and conversion to PlanRequest
src/request.rs                     typed request and verification/output choices
src/error.rs                       domain errors and exit-code mapping
src/paths.rs                       exact-target resolution and safety validation
src/scan.rs                        source-tree traversal without link following
src/hash.rs                        streaming BLAKE3 hashing
src/plan.rs                        destination comparison and deterministic actions
src/report.rs                      human and NDJSON plan/error rendering
src/app.rs                         orchestration of resolve → scan → plan → report
tests/cli_contract.rs              public CLI and exit-code contract
tests/plan_integration.rs          real-filesystem planning behavior
.github/workflows/ci.yml           formatting, lint, test, and MSRV checks
.gitignore                         minimal Rust and Orbit transient exclusions
README.md                          honest reboot-stage user documentation
LEGACY.md                          immutable archive pointer
CONTRIBUTING.md                    current development commands and scope
SECURITY.md                        current reporting policy and alpha warning
```

---

### Task 1: Archive the Experiment and Create the Clean Reboot Branch

**Files:**
- Remove: `.cargo/`, `.github/`, `benches/`, `crates/`, `examples/`, `scripts/`, `src/`, `tests/`
- Remove: `ARCHITECTURE.md`, `CHANGELOG.md`, `Cargo.lock`, `Cargo.toml`, `Dockerfile`, `Dockerfile.demo`, `README.md`, `codecov.yml`, `deny.toml`, `docker-compose.demo.yml`
- Remove: legacy subdirectories beneath `docs/`, retaining only `docs/superpowers/`
- Modify: `.gitignore`
- Modify: `CONTRIBUTING.md`
- Modify: `SECURITY.md`
- Create: `README.md`
- Create: `LEGACY.md`
- Preserve unchanged: `LICENSE`, `CODE_OF_CONDUCT.md`, `docs/superpowers/specs/2026-09-22-orbit-reboot-design.md`, this plan

**Interfaces:**
- Consumes: current `main` commit containing the approved design and this implementation plan.
- Produces: immutable pre-reboot tag and a documentation-only `codex/orbit-reboot` branch with no legacy product implementation.

- [ ] **Step 1: Verify the archive target and create the annotated tag**

Run:

```powershell
git status --short
git branch --show-current
git rev-parse HEAD
git tag --list archive/pre-reboot-2026-09-22
git tag -a archive/pre-reboot-2026-09-22 -m "Archive Orbit before the 2026 reboot"
git rev-parse archive/pre-reboot-2026-09-22^{commit}
git push origin refs/tags/archive/pre-reboot-2026-09-22
git ls-remote --tags origin archive/pre-reboot-2026-09-22
```

Expected: the working tree is clean, the branch is `main`, the tag did not previously exist, the tag commit equals the recorded pre-reboot `HEAD`, and the same tag exists on `origin`. If the tag already exists at another commit or the remote push cannot be verified, stop before deleting files instead of moving the tag or relying on a local-only archive.

If `gh api user` succeeds, also publish the archive marker as a GitHub release:

```powershell
gh release create archive/pre-reboot-2026-09-22 --verify-tag --title "Orbit before the 2026 reboot" --notes "Immutable archive of the experimental Orbit implementation. The reboot makes no compatibility promise with this code."
```

If `gh api user` still returns `401`, record that the immutable remote tag exists and defer only the release-page metadata; do not repeat the device flow during implementation.

- [ ] **Step 2: Create the reboot branch**

Run:

```powershell
git switch -c codex/orbit-reboot
git branch --show-current
```

Expected: `codex/orbit-reboot`.

- [ ] **Step 3: Remove the tracked experimental implementation and product documentation**

Run:

```powershell
git rm -r .cargo .github benches crates examples scripts src tests
git rm ARCHITECTURE.md CHANGELOG.md Cargo.lock Cargo.toml Dockerfile Dockerfile.demo README.md codecov.yml deny.toml docker-compose.demo.yml
git rm -r docs/architecture docs/archive docs/guides docs/manifest docs/project-status docs/release docs/specs docs/wormhole
git rm docs/DEPENDABOT_ISSUES.md docs/GETTING_STARTED.md docs/README.md docs/file_structure_checklist.md docs/observability-v3.md
```

Expected: `docs/superpowers/specs/` and `docs/superpowers/plans/` remain tracked.

- [ ] **Step 4: Replace the root documentation with the reboot truth**

Write `README.md`:

```markdown
# Orbit

Orbit is being rebuilt as a small, trustworthy local data copier.

The first product contract is: provide an exact source and destination, preview a deterministic plan, copy through safe staging, resume interruption, and verify the result. The reboot currently has no supported release.

See [the reboot design](docs/superpowers/specs/2026-09-22-orbit-reboot-design.md) for the approved scope. Historical code and documentation are preserved at `archive/pre-reboot-2026-09-22`; see [LEGACY.md](LEGACY.md).
```

Write `LEGACY.md`:

```markdown
# Legacy Orbit

The pre-reboot experimental implementation is preserved by the annotated Git tag `archive/pre-reboot-2026-09-22`.

It is retained for historical reference only. Its CLI, manifests, modules, and advertised capabilities are not compatibility commitments for the rebooted product.
```

Replace `.gitignore` with:

```gitignore
/target/
*.rs.bk
*.log
*.tmp
.DS_Store
Thumbs.db
.idea/
.vscode/
.claude/
.orbit-state/
*.orbit-partial-*
```

Replace `CONTRIBUTING.md` with:

````markdown
# Contributing

Orbit is in a ground-up reboot. Keep changes within the approved design and the active implementation plan.

Before submitting a change, run:

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Do not add a protocol, backend abstraction, service, plugin system, or compatibility layer without an approved design.
````

Replace `SECURITY.md` with:

```markdown
# Security Policy

Orbit has no supported release during the reboot and must not yet be trusted as the only copy of important data.

Report vulnerabilities privately through GitHub's security advisory feature. Do not include sensitive data in a public issue.
```

- [ ] **Step 5: Verify the reset contains no orphaned legacy product files**

Run:

```powershell
git status --short
git ls-files
git diff --check
```

Expected: only the preserved community files, reboot documentation, and newly rewritten root documentation remain; no `src/`, `crates/`, old workflow, Docker, benchmark, example, or legacy product-documentation file remains.

- [ ] **Step 6: Commit the archival reset**

Run:

```powershell
git add .gitignore README.md LEGACY.md CONTRIBUTING.md SECURITY.md
git commit -m "chore: archive the experimental Orbit codebase"
```

Expected: one documentation-only reset commit on `codex/orbit-reboot`.

---

### Task 2: Bootstrap the Single Crate and Typed Plan CLI

**Files:**
- Create: `Cargo.toml`
- Create: `Cargo.lock` using Cargo
- Create: `rust-toolchain.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `src/cli.rs`
- Create: `src/request.rs`
- Create: `tests/cli_contract.rs`

**Interfaces:**
- Consumes: no product code; starts from the documentation-only reset.
- Produces: `request::PlanRequest`, `request::VerifyMode`, `request::OutputMode`, and `cli::Cli::into_request() -> PlanRequest` for later tasks.

- [ ] **Step 1: Add the package manifest and pinned toolchain**

Create `Cargo.toml`:

```toml
[package]
name = "orbit"
version = "0.1.0-alpha.0"
edition = "2024"
rust-version = "1.85"
authors = ["Shane Wall <shaneawall@gmail.com>"]
description = "A trustworthy local data copier"
license = "Apache-2.0"
readme = "README.md"
repository = "https://github.com/saworbit/orbit"

[dependencies]
blake3 = "1.8"
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3.23"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.85.0"
components = ["clippy", "rustfmt"]
profile = "minimal"
```

- [ ] **Step 2: Write failing parser tests for the complete milestone CLI**

Create `src/cli.rs` with only the type name so the tests compile far enough to fail:

```rust
pub struct Cli;
```

Create `src/request.rs` with the required domain contract:

```rust
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifyMode {
    Hash,
    Size,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputMode {
    Human,
    Json,
    Quiet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanRequest {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub replace: bool,
    pub verify: VerifyMode,
    pub ignore_unsupported: bool,
    pub output: OutputMode,
}
```

Append the following unit tests to `src/cli.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::Cli;
    use crate::request::{OutputMode, VerifyMode};

    #[test]
    fn parses_plan_with_safe_defaults() {
        let request = Cli::parse_request_from(["orbit", "plan", "from", "to"]).unwrap();
        assert_eq!(request.verify, VerifyMode::Hash);
        assert_eq!(request.output, OutputMode::Human);
        assert!(!request.replace);
        assert!(!request.ignore_unsupported);
    }

    #[test]
    fn rejects_json_and_quiet_together() {
        let error = Cli::parse_request_from([
            "orbit", "plan", "from", "to", "--json", "--quiet",
        ])
        .unwrap_err();
        assert_eq!(error.exit_code(), 2);
    }
}
```

Create `src/lib.rs`:

```rust
pub mod cli;
pub mod request;
```

- [ ] **Step 3: Run the parser tests and observe the missing method failure**

Run:

```powershell
cargo test cli::tests --lib
```

Expected: compilation fails because `Cli::parse_request_from` does not exist.

- [ ] **Step 4: Implement the clap boundary and typed conversion**

Replace `src/cli.rs` with:

```rust
use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::request::{OutputMode, PlanRequest, VerifyMode};

#[derive(Debug, Parser)]
#[command(name = "orbit", version, about = "A trustworthy local data copier")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect exactly what Orbit would do without writing anything.
    Plan(PlanArgs),
}

#[derive(Debug, Args)]
struct PlanArgs {
    source: PathBuf,
    destination: PathBuf,
    #[arg(long)]
    replace: bool,
    #[arg(long, value_enum, default_value_t = VerifyArg::Hash)]
    verify: VerifyArg,
    #[arg(long)]
    ignore_unsupported: bool,
    #[arg(long, conflicts_with = "quiet")]
    json: bool,
    #[arg(long, conflicts_with = "json")]
    quiet: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum VerifyArg {
    Hash,
    Size,
}

impl Cli {
    pub fn parse_request_from<I, T>(args: I) -> Result<PlanRequest, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let cli = Self::try_parse_from(args)?;
        Ok(cli.into_request())
    }

    pub fn into_request(self) -> PlanRequest {
        let Command::Plan(args) = self.command;
        PlanRequest {
            source: args.source,
            destination: args.destination,
            replace: args.replace,
            verify: match args.verify {
                VerifyArg::Hash => VerifyMode::Hash,
                VerifyArg::Size => VerifyMode::Size,
            },
            ignore_unsupported: args.ignore_unsupported,
            output: if args.json {
                OutputMode::Json
            } else if args.quiet {
                OutputMode::Quiet
            } else {
                OutputMode::Human
            },
        }
    }
}
```

- [ ] **Step 5: Run the parser tests and generate the lockfile**

Run:

```powershell
cargo test cli::tests --lib
cargo generate-lockfile
```

Expected: both parser tests pass and `Cargo.lock` is created.

- [ ] **Step 6: Add a minimal process boundary and public help test**

Create `src/main.rs`:

```rust
use clap::Parser;
use orbit::cli::Cli;

fn main() {
    let _request = Cli::parse().into_request();
}
```

Create `tests/cli_contract.rs`:

```rust
use predicates::prelude::*;

#[test]
fn help_exposes_only_plan() {
    assert_cmd::cargo::cargo_bin_cmd!("orbit")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("copy").not());
}

#[test]
fn invalid_arguments_exit_two() {
    assert_cmd::cargo::cargo_bin_cmd!("orbit")
        .args(["plan", "source-only"])
        .assert()
        .code(2);
}
```

Run:

```powershell
cargo test --test cli_contract
cargo fmt --all -- --check
```

Expected: both CLI tests pass and formatting is clean.

- [ ] **Step 7: Commit the crate bootstrap**

Run:

```powershell
git add Cargo.toml Cargo.lock rust-toolchain.toml src tests/cli_contract.rs
git commit -m "feat: bootstrap the Orbit planner CLI"
```

---

### Task 3: Resolve Exact Endpoints and Reject Unsafe Relationships

**Files:**
- Create: `src/error.rs`
- Create: `src/paths.rs`
- Modify: `src/lib.rs`
- Test: unit tests in `src/paths.rs`

**Interfaces:**
- Consumes: `request::PlanRequest` paths.
- Produces: `paths::ResolvedEndpoints { source: PathBuf, destination: PathBuf }`, `paths::resolve_endpoints(&Path, &Path, &Path) -> Result<ResolvedEndpoints>`, `error::OrbitError`, and `error::Result<T>`.

- [ ] **Step 1: Define errors and write failing endpoint tests**

Create `src/error.rs`:

```rust
use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum OrbitError {
    #[error("source does not exist: {0}")]
    SourceMissing(PathBuf),
    #[error("source and destination resolve to the same path: {0}")]
    SameEndpoint(PathBuf),
    #[error("destination is inside the source tree: {destination}")]
    DestinationInsideSource { destination: PathBuf },
    #[error("cannot {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl OrbitError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::SourceMissing(_)
            | Self::SameEndpoint(_)
            | Self::DestinationInsideSource { .. } => 3,
            Self::Io { .. } => 3,
        }
    }
}

pub type Result<T> = std::result::Result<T, OrbitError>;
```

Create `src/paths.rs` with the public type and three tests:

```rust
use std::path::{Path, PathBuf};

use crate::error::Result;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedEndpoints {
    pub source: PathBuf,
    pub destination: PathBuf,
}

pub fn resolve_endpoints(_source: &Path, _destination: &Path, _cwd: &Path) -> Result<ResolvedEndpoints> {
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::resolve_endpoints;
    use crate::error::OrbitError;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolves_a_nonexistent_exact_destination() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, b"source").unwrap();
        let result = resolve_endpoints(&source, &temp.path().join("new/target.txt"), temp.path()).unwrap();
        assert!(result.source.is_absolute());
        assert_eq!(
            result.destination,
            fs::canonicalize(temp.path()).unwrap().join("new/target.txt")
        );
    }

    #[test]
    fn rejects_the_same_endpoint() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("same");
        fs::write(&source, b"source").unwrap();
        assert!(matches!(
            resolve_endpoints(&source, &source, temp.path()),
            Err(OrbitError::SameEndpoint(_))
        ));
    }

    #[test]
    fn rejects_destination_inside_source_directory() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        assert!(matches!(
            resolve_endpoints(&source, &source.join("nested/copy"), temp.path()),
            Err(OrbitError::DestinationInsideSource { .. })
        ));
    }
}
```

Export both modules from `src/lib.rs`.

- [ ] **Step 2: Run endpoint tests and observe failure**

Run:

```powershell
cargo test paths::tests --lib
```

Expected: the tests fail at `unreachable!()`.

- [ ] **Step 3: Implement symlink-aware existing-source and nearest-ancestor resolution**

Add this narrowly target-scoped dependency to `Cargo.toml`; `CompareStringOrdinal` compares Windows path components as native UTF-16 ordinal text without lossy UTF-8 conversion or Unicode lowercasing:

```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61", features = ["Win32_Globalization"] }
```

Replace the deliberately failing function in `src/paths.rs` with helpers that preserve extended canonical paths, never pre-collapse `.` or `..`, resolve an intermediate symbolic link before a following `..`, and compare non-Windows components exactly:

```rust
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::{ffi::OsStr, os::windows::ffi::OsStrExt};

#[cfg(windows)]
use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};

use crate::error::{OrbitError, Result};

pub fn resolve_endpoints(source: &Path, destination: &Path, cwd: &Path) -> Result<ResolvedEndpoints> {
    let source_input = absolutize(source, cwd);
    let source_metadata = match fs::symlink_metadata(&source_input) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(OrbitError::SourceMissing(source_input));
        }
        Err(source_error) => {
            return Err(OrbitError::Io {
                operation: "inspect source endpoint",
                path: source_input,
                source: source_error,
            });
        }
    };
    let source = resolve_leaf(&source_input)?;
    let destination = resolve_leaf(&absolutize(destination, cwd))?;

    if paths_equal(&source, &destination) {
        return Err(OrbitError::SameEndpoint(source));
    }
    if source_metadata.file_type().is_dir() && is_descendant(&destination, &source) {
        return Err(OrbitError::DestinationInsideSource { destination });
    }
    Ok(ResolvedEndpoints { source, destination })
}

fn absolutize(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn resolve_leaf(path: &Path) -> Result<PathBuf> {
    let Some(name) = path.file_name() else {
        return resolve_ancestor(path);
    };
    let parent = path.parent().expect("a path with a file name has a parent");
    Ok(resolve_ancestor(parent)?.join(name))
}

fn resolve_ancestor(path: &Path) -> Result<PathBuf> {
    let mut resolved = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => resolved.push(prefix.as_os_str()),
            std::path::Component::RootDir => resolved.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            std::path::Component::Normal(name) => {
                let candidate = resolved.join(name);
                match fs::symlink_metadata(&candidate) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        resolved = canonicalize(&candidate, "resolve endpoint symlink")?;
                    }
                    Ok(_) => resolved = candidate,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        resolved.push(name);
                    }
                    Err(source) => {
                        return Err(OrbitError::Io {
                            operation: "inspect endpoint ancestor",
                            path: candidate,
                            source,
                        });
                    }
                }
            }
        }
    }
    resolve_nearest_existing_ancestor(&resolved)
}

fn resolve_nearest_existing_ancestor(path: &Path) -> Result<PathBuf> {
    let mut ancestor = path;
    let mut suffix = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = ancestor.file_name().ok_or_else(|| OrbitError::SourceMissing(path.to_path_buf()))?;
                suffix.push(name.to_os_string());
                ancestor = ancestor.parent().ok_or_else(|| OrbitError::SourceMissing(path.to_path_buf()))?;
            }
            Err(source) => {
                return Err(OrbitError::Io {
                    operation: "inspect endpoint ancestor",
                    path: ancestor.to_path_buf(),
                    source,
                });
            }
        }
    }
    let mut resolved = canonicalize(ancestor, "resolve endpoint ancestor")?;
    for name in suffix.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

fn canonicalize(path: &Path, operation: &'static str) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|source| OrbitError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let mut left_components = left.components();
    let mut right_components = right.components();
    loop {
        match (left_components.next(), right_components.next()) {
            (Some(left), Some(right)) if components_equal(left.as_os_str(), right.as_os_str()) => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

fn is_descendant(candidate: &Path, parent: &Path) -> bool {
    let mut candidate_components = candidate.components();
    for parent_component in parent.components() {
        let Some(candidate_component) = candidate_components.next() else {
            return false;
        };
        if !components_equal(candidate_component.as_os_str(), parent_component.as_os_str()) {
            return false;
        }
    }
    candidate_components.next().is_some()
}

#[cfg(not(windows))]
fn components_equal(left: &std::ffi::OsStr, right: &std::ffi::OsStr) -> bool {
    left == right
}

#[cfg(windows)]
fn components_equal(left: &OsStr, right: &OsStr) -> bool {
    let left: Vec<u16> = left.encode_wide().collect();
    let right: Vec<u16> = right.encode_wide().collect();
    let (Ok(left_len), Ok(right_len)) = (i32::try_from(left.len()), i32::try_from(right.len())) else {
        return false;
    };

    // The UTF-16 slices remain valid for the call, and the API receives their exact lengths.
    unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left_len,
            right.as_ptr(),
            right_len,
            1,
        ) == CSTR_EQUAL
    }
}
```

- [ ] **Step 4: Verify endpoint behavior and lint the module**

Run:

```powershell
cargo test paths::tests --lib
cargo clippy --lib -- -D warnings
```

Expected: all endpoint tests pass and Clippy reports no warnings. Before committing, append and run these final-component and Windows case-sensitivity regressions:

```rust
#[test]
fn resolution_preserves_a_final_symlink_instead_of_following_it() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("target.txt");
    let link = temp.path().join("link.txt");
    fs::write(&target, b"target").unwrap();
    if !create_file_link(&target, &link) {
        return;
    }
    let resolved = resolve_endpoints(&link, &temp.path().join("copy.txt"), temp.path()).unwrap();
    let resolved_parent = fs::canonicalize(temp.path()).unwrap();
    assert_eq!(resolved.source, resolved_parent.join("link.txt"));
    assert_ne!(resolved.source, resolved_parent.join("target.txt"));
}

#[test]
fn resolves_parent_components_after_symlinks_before_checking_containment() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("b/source");
    let link = temp.path().join("a/link");
    fs::create_dir_all(source.join("deep")).unwrap();
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    if !create_directory_link(&source.join("deep"), &link) {
        return;
    }
    assert!(matches!(
        resolve_endpoints(&link.join(".."), &source.join("nested/copy"), temp.path()),
        Err(OrbitError::DestinationInsideSource { .. })
    ));
}

#[cfg(unix)]
fn create_file_link(target: &std::path::Path, link: &std::path::Path) -> bool {
    std::os::unix::fs::symlink(target, link).unwrap();
    true
}

#[cfg(unix)]
fn create_directory_link(target: &std::path::Path, link: &std::path::Path) -> bool {
    std::os::unix::fs::symlink(target, link).unwrap();
    true
}

#[cfg(windows)]
fn create_directory_link(target: &std::path::Path, link: &std::path::Path) -> bool {
    match std::os::windows::fs::symlink_dir(target, link) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(error) => panic!("cannot create test symlink: {error}"),
    }
}

#[cfg(windows)]
fn create_file_link(target: &std::path::Path, link: &std::path::Path) -> bool {
    match std::os::windows::fs::symlink_file(target, link) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(error) => panic!("cannot create test symlink: {error}"),
    }
}

#[cfg(windows)]
#[test]
fn rejects_the_same_windows_endpoint_with_different_case() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("MixedCase.txt");
    fs::write(&source, b"source").unwrap();
    let differently_cased = temp.path().join("MIXEDCASE.TXT");
    assert!(matches!(
        resolve_endpoints(&source, &differently_cased, temp.path()),
        Err(OrbitError::SameEndpoint(_))
    ));
}

#[cfg(windows)]
#[test]
fn preserves_windows_extended_canonical_prefixes() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.txt");
    fs::write(&source, b"source").unwrap();
    let canonical_parent = fs::canonicalize(temp.path()).unwrap();
    assert!(canonical_parent.to_string_lossy().starts_with(r"\\?\"));

    let resolved = resolve_endpoints(&source, &temp.path().join("new/target.txt"), temp.path()).unwrap();
    assert_eq!(resolved.source, canonical_parent.join("source.txt"));
    assert_eq!(resolved.destination, canonical_parent.join("new/target.txt"));
}

#[cfg(windows)]
#[test]
fn rejects_the_same_windows_endpoint_with_non_ascii_case() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("MÜNCHEN.TXT");
    fs::write(&source, b"source").unwrap();
    let differently_cased = temp.path().join("münchen.txt");
    assert!(matches!(
        resolve_endpoints(&source, &differently_cased, temp.path()),
        Err(OrbitError::SameEndpoint(_))
    ));
}
```

Run `cargo test resolution_preserves_a_final_symlink_instead_of_following_it --lib` and `cargo test resolves_parent_components_after_symlinks_before_checking_containment --lib` on every platform. On Windows also run `cargo test preserves_windows_extended_canonical_prefixes --lib`, `cargo test rejects_the_same_windows_endpoint_with_different_case --lib`, and `cargo test rejects_the_same_windows_endpoint_with_non_ascii_case --lib`; expect PASS.

- [ ] **Step 5: Commit endpoint validation**

Run:

```powershell
git add src/error.rs src/paths.rs src/lib.rs
git commit -m "feat: validate exact copy endpoints"
```

---

### Task 4: Scan Source Trees Without Following Links

**Files:**
- Create: `src/scan.rs`
- Modify: `src/error.rs`
- Modify: `src/lib.rs`
- Test: unit tests in `src/scan.rs`

**Interfaces:**
- Consumes: `paths::ResolvedEndpoints.source`.
- Produces: `scan::scan_source(&Path) -> Result<SourceSnapshot>`, `SourceSnapshot { root, entries }`, `SourceEntry`, `EntryKind`, and `UnsupportedFeature`.

- [ ] **Step 1: Write failing tests for deterministic traversal and link handling**

Create `src/scan.rs` with these domain types:

```rust
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind { File, Directory, Symlink, Unsupported }

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedFeature { SpecialFile }

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceEntry {
    pub relative_path: PathBuf,
    pub kind: EntryKind,
    pub length: u64,
    pub modified_ns: Option<i128>,
    pub read_only: bool,
    pub link_target: Option<PathBuf>,
    pub unsupported: Vec<UnsupportedFeature>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSnapshot {
    pub root: PathBuf,
    pub entries: Vec<SourceEntry>,
}

pub fn scan_source(_root: &Path) -> Result<SourceSnapshot> { unreachable!() }
```

Append these tests to `src/scan.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{scan_source, EntryKind};
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[test]
    fn returns_entries_in_relative_path_order() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested/b.txt"), b"b").unwrap();
        fs::write(temp.path().join("a.txt"), b"a").unwrap();

        let snapshot = scan_source(temp.path()).unwrap();
        let paths: Vec<_> = snapshot.entries.into_iter().map(|entry| entry.relative_path).collect();
        assert_eq!(
            paths,
            vec![
                PathBuf::from(""),
                PathBuf::from("a.txt"),
                PathBuf::from("nested"),
                PathBuf::from("nested/b.txt"),
            ]
        );
    }

    #[test]
    fn records_a_symlink_without_following_its_target() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("inside.txt"), b"inside").unwrap();
        if !create_directory_link(&target, &link) {
            return;
        }

        let snapshot = scan_source(&link).unwrap();
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].kind, EntryKind::Symlink);
        assert_eq!(snapshot.entries[0].link_target.as_deref(), Some(target.as_path()));
    }

    #[cfg(unix)]
    fn create_directory_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn create_directory_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("cannot create test symlink: {error}"),
        }
    }
}
```

- [ ] **Step 2: Run scanner tests and observe the deliberate failure**

Run:

```powershell
cargo test scan::tests --lib
```

Expected: tests fail at `unreachable!()`.

- [ ] **Step 3: Implement recursive `read_dir` traversal with `symlink_metadata`**

Implement `scan_source` using this structure:

```rust
pub fn scan_source(root: &Path) -> Result<SourceSnapshot> {
    let mut entries = Vec::new();
    visit(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(SourceSnapshot { root: root.to_path_buf(), entries })
}

fn visit(root: &Path, path: &Path, entries: &mut Vec<SourceEntry>) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| OrbitError::Io {
        operation: "inspect source",
        path: path.to_path_buf(),
        source,
    })?;
    let file_type = metadata.file_type();
    let relative_path = path.strip_prefix(root).expect("entry is beneath scan root").to_path_buf();
    let kind = if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_file() {
        EntryKind::File
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::Unsupported
    };
    let unsupported = if kind == EntryKind::Unsupported {
        vec![UnsupportedFeature::SpecialFile]
    } else {
        Vec::new()
    };
    let link_target = if kind == EntryKind::Symlink {
        Some(std::fs::read_link(path).map_err(|source| OrbitError::Io {
            operation: "read symbolic link",
            path: path.to_path_buf(),
            source,
        })?)
    } else {
        None
    };
    entries.push(SourceEntry {
        relative_path,
        kind: kind.clone(),
        length: if kind == EntryKind::File { metadata.len() } else { 0 },
        modified_ns: metadata.modified().ok().and_then(system_time_ns),
        read_only: metadata.permissions().readonly(),
        link_target,
        unsupported,
    });
    if kind == EntryKind::Directory {
        let mut children = std::fs::read_dir(path)
            .map_err(|source| OrbitError::Io { operation: "read source directory", path: path.to_path_buf(), source })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|source| OrbitError::Io { operation: "read source entry", path: path.to_path_buf(), source })?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            visit(root, &child.path(), entries)?;
        }
    }
    Ok(())
}

fn system_time_ns(value: std::time::SystemTime) -> Option<i128> {
    value.duration_since(std::time::UNIX_EPOCH).ok()
        .map(|duration| i128::from(duration.as_secs()) * 1_000_000_000 + i128::from(duration.subsec_nanos()))
}
```

Import `OrbitError` and derive `Clone` for `EntryKind` as shown. Never call `Path::metadata` or `fs::canonicalize` inside traversal because both can follow links.

- [ ] **Step 4: Verify traversal, link behavior, and formatting**

Run:

```powershell
cargo test scan::tests --lib
cargo fmt --all -- --check
cargo clippy --lib -- -D warnings
```

Expected: deterministic traversal and the supported platform's symlink test pass with no warnings.

- [ ] **Step 5: Commit source scanning**

Run:

```powershell
git add src/scan.rs src/error.rs src/lib.rs
git commit -m "feat: scan local source trees deterministically"
```

---

### Task 5: Hash Files and Inspect Exact Destination Entries

**Files:**
- Create: `src/hash.rs`
- Modify: `src/plan.rs` (initial destination types only)
- Modify: `src/lib.rs`
- Test: unit tests in `src/hash.rs` and `src/plan.rs`

**Interfaces:**
- Consumes: source and destination file paths.
- Produces: `hash::hash_file(&Path) -> Result<blake3::Hash>` and private `plan::inspect_destination(&Path) -> Result<Option<DestinationEntry>>`.

- [ ] **Step 1: Write a failing streaming-hash test**

Create `src/hash.rs`:

```rust
use std::path::Path;

use crate::error::Result;

pub fn hash_file(_path: &Path) -> Result<blake3::Hash> { unreachable!() }

#[cfg(test)]
mod tests {
    use super::hash_file;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn hashes_the_complete_file() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("data");
        let bytes = vec![0x5a; 128 * 1024 + 17];
        fs::write(&path, &bytes).unwrap();
        assert_eq!(hash_file(&path).unwrap(), blake3::hash(&bytes));
    }
}
```

- [ ] **Step 2: Observe failure, then implement bounded-memory hashing**

Run `cargo test hash::tests --lib`; expect `unreachable!()`.

Replace `hash_file` with:

```rust
pub fn hash_file(path: &Path) -> Result<blake3::Hash> {
    use std::io::Read;

    let mut file = std::fs::File::open(path).map_err(|source| OrbitError::Io {
        operation: "open file for hashing",
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|source| OrbitError::Io {
            operation: "hash file",
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize())
}
```

Import `OrbitError`, then rerun `cargo test hash::tests --lib` and expect PASS.

- [ ] **Step 3: Write destination-inspection tests before implementation**

Create `src/plan.rs`:

```rust
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::scan::EntryKind;

#[derive(Clone, Debug, Eq, PartialEq)]
struct DestinationEntry {
    kind: EntryKind,
    length: u64,
    link_target: Option<PathBuf>,
}

fn inspect_destination(_path: &Path) -> Result<Option<DestinationEntry>> {
    unreachable!()
}

#[cfg(test)]
mod destination_tests {
    use super::inspect_destination;
    use crate::scan::EntryKind;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_destination_returns_none() {
        let temp = tempdir().unwrap();
        assert_eq!(inspect_destination(&temp.path().join("missing")).unwrap(), None);
    }

    #[test]
    fn regular_file_reports_kind_and_length() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("file");
        fs::write(&path, b"four").unwrap();
        let entry = inspect_destination(&path).unwrap().unwrap();
        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.length, 4);
        assert_eq!(entry.link_target, None);
    }

    #[test]
    fn directory_is_not_classified_as_file() {
        let temp = tempdir().unwrap();
        let entry = inspect_destination(temp.path()).unwrap().unwrap();
        assert_eq!(entry.kind, EntryKind::Directory);
    }
}
```

Run `cargo test plan::destination_tests --lib` and expect all three tests to fail at `unreachable!()`.

- [ ] **Step 4: Implement destination inspection using `symlink_metadata`**

Use the same `EntryKind` classification as source scanning:

```rust
fn inspect_destination(path: &Path) -> Result<Option<DestinationEntry>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(OrbitError::Io {
            operation: "inspect destination",
            path: path.to_path_buf(),
            source,
        }),
    };
    let file_type = metadata.file_type();
    let kind = if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_file() {
        EntryKind::File
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::Unsupported
    };
    let link_target = if kind == EntryKind::Symlink {
        Some(std::fs::read_link(path).map_err(|source| OrbitError::Io {
            operation: "read destination symbolic link",
            path: path.to_path_buf(),
            source,
        })?)
    } else {
        None
    };
    Ok(Some(DestinationEntry {
        kind,
        length: metadata.len(),
        link_target,
    }))
}
```

- [ ] **Step 5: Verify and commit hashing plus inspection**

Run:

```powershell
cargo test hash::tests plan::destination_tests --lib
cargo clippy --lib -- -D warnings
git add src/hash.rs src/plan.rs src/lib.rs
git commit -m "feat: inspect and hash local files"
```

Expected: all focused tests pass before the commit.

---

### Task 6: Build Deterministic Copy Plans

**Files:**
- Modify: `src/plan.rs`
- Modify: `src/error.rs`
- Test: unit tests in `src/plan.rs`

**Interfaces:**
- Consumes: `PlanRequest`, `ResolvedEndpoints`, `SourceSnapshot`, and `hash_file`.
- Produces: `plan::build_plan(&PlanRequest, &ResolvedEndpoints, &SourceSnapshot) -> Result<CopyPlan>`, `CopyPlan`, `PlanEntry`, and `Disposition`.

- [ ] **Step 1: Define the serialized plan contract**

Add to `src/plan.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Copy,
    SkipIdentical,
    Replace,
    Conflict,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct PlanEntry {
    pub relative_path: PathBuf,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub kind: EntryKind,
    pub disposition: Disposition,
    pub length: u64,
    pub source_digest: Option<String>,
    pub reason: String,
    pub blocking: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct CopyPlan {
    pub operation_id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub entries: Vec<PlanEntry>,
}

impl CopyPlan {
    pub fn is_executable(&self) -> bool {
        self.entries.iter().all(|entry| !entry.blocking)
    }
}
```

- [ ] **Step 2: Write the decision-table tests before `build_plan`**

Append this test module to `src/plan.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{build_plan, Disposition};
    use crate::paths::ResolvedEndpoints;
    use crate::request::{OutputMode, PlanRequest, VerifyMode};
    use crate::scan::{scan_source, EntryKind, SourceEntry, SourceSnapshot, UnsupportedFeature};
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn plan_for(source: &Path, destination: &Path, replace: bool, ignore: bool) -> super::CopyPlan {
        let source = source.to_path_buf();
        let destination = destination.to_path_buf();
        let request = PlanRequest {
            source: source.clone(),
            destination: destination.clone(),
            replace,
            verify: VerifyMode::Hash,
            ignore_unsupported: ignore,
            output: OutputMode::Human,
        };
        let endpoints = ResolvedEndpoints { source: source.clone(), destination };
        let snapshot = scan_source(&source).unwrap();
        build_plan(&request, &endpoints, &snapshot).unwrap()
    }

    #[test]
    fn missing_exact_destination_is_copy() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        fs::write(&source, b"same").unwrap();
        let plan = plan_for(&source, &temp.path().join("missing"), false, false);
        assert_eq!(plan.entries[0].disposition, Disposition::Copy);
        assert_eq!(plan.entries[0].reason, "destination does not exist");
        assert!(plan.entries[0].source_digest.is_some());
    }

    #[test]
    fn equal_size_is_hashed_before_identical_is_selected() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"same").unwrap();
        fs::write(&destination, b"same").unwrap();
        let plan = plan_for(&source, &destination, false, false);
        assert_eq!(plan.entries[0].disposition, Disposition::SkipIdentical);
        let expected_digest = blake3::hash(b"same").to_hex().to_string();
        assert_eq!(plan.entries[0].source_digest.as_deref(), Some(expected_digest.as_str()));
        assert_eq!(plan.entries[0].reason, "contents are identical");
    }

    #[test]
    fn equal_size_different_content_is_conflict_without_replace() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"left").unwrap();
        fs::write(&destination, b"rite").unwrap();
        let plan = plan_for(&source, &destination, false, false);
        assert_eq!(plan.entries[0].disposition, Disposition::Conflict);
        assert_eq!(plan.entries[0].reason, "destination file has different contents");
        assert!(!plan.is_executable());
    }

    #[test]
    fn replace_changes_regular_file_conflict_to_replace() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"new").unwrap();
        fs::write(&destination, b"old-value").unwrap();
        let plan = plan_for(&source, &destination, true, false);
        assert_eq!(plan.entries[0].disposition, Disposition::Replace);
        assert_eq!(plan.entries[0].reason, "destination file will be replaced");
        assert!(plan.is_executable());
    }

    #[test]
    fn type_conflict_blocks_even_with_replace() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"file").unwrap();
        fs::create_dir(&destination).unwrap();
        let plan = plan_for(&source, &destination, true, false);
        assert_eq!(plan.entries[0].disposition, Disposition::Conflict);
        assert_eq!(plan.entries[0].reason, "source and destination object types differ");
        assert!(!plan.is_executable());
    }

    #[test]
    fn ignored_unsupported_entry_remains_visible_and_non_blocking() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        fs::write(&source, b"source").unwrap();
        let destination = temp.path().join("destination");
        let request = PlanRequest {
            source: source.clone(),
            destination: destination.clone(),
            replace: false,
            verify: VerifyMode::Hash,
            ignore_unsupported: true,
            output: OutputMode::Human,
        };
        let endpoints = ResolvedEndpoints { source: source.clone(), destination };
        let snapshot = SourceSnapshot {
            root: source.clone(),
            entries: vec![SourceEntry {
                relative_path: PathBuf::new(),
                kind: EntryKind::Unsupported,
                length: 0,
                modified_ns: None,
                read_only: false,
                link_target: None,
                unsupported: vec![UnsupportedFeature::SpecialFile],
            }],
        };
        let plan = build_plan(&request, &endpoints, &snapshot).unwrap();
        assert_eq!(plan.entries[0].disposition, Disposition::Unsupported);
        assert_eq!(plan.entries[0].reason, "unsupported source entry will be omitted");
        assert!(!plan.entries[0].blocking);
        assert!(plan.is_executable());
    }

    #[test]
    fn plan_entries_remain_sorted_by_relative_path() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("z.txt"), b"z").unwrap();
        fs::write(source.join("a.txt"), b"a").unwrap();
        let plan = plan_for(&source, &temp.path().join("destination"), false, false);
        let paths: Vec<_> = plan.entries.iter().map(|entry| entry.relative_path.clone()).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
    }

    #[test]
    fn matching_symbolic_link_targets_are_identical_without_traversal() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("target");
        let source = temp.path().join("source-link");
        let destination = temp.path().join("destination-link");
        fs::write(&target, b"target contents must not be planned").unwrap();
        if !create_file_link(&target, &source) || !create_file_link(&target, &destination) {
            return;
        }
        let plan = plan_for(&source, &destination, false, false);
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].disposition, Disposition::SkipIdentical);
        assert_eq!(plan.entries[0].reason, "symbolic link targets are identical");
    }

    #[cfg(unix)]
    fn create_file_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn create_file_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("cannot create test symlink: {error}"),
        }
    }
}
```

Each test must assert `reason` as well as `disposition`, so explanations are part of the contract. Run `cargo test plan::tests --lib` and expect compilation failure because `build_plan` is absent.

- [ ] **Step 3: Implement the complete disposition table**

Implement `build_plan` as a loop over the already sorted `snapshot.entries`. For each entry, calculate `source_path` (`snapshot.root` for the empty relative root, otherwise `snapshot.root.join(relative)`) and `destination_path` using the same rule. Apply this table exactly:

```text
source unsupported + ignore=false       => Unsupported, blocking=true
source unsupported + ignore=true        => Unsupported, blocking=false
destination missing                     => Copy, blocking=false
same directory kind                     => SkipIdentical, blocking=false
same symlink kind and equal target       => SkipIdentical, blocking=false
different symlink target                 => Conflict, blocking=true
different object kind                    => Conflict, blocking=true
regular files, different length, replace=false => Conflict, blocking=true
regular files, different length, replace=true  => Replace, blocking=false
regular files, equal length and equal hash       => SkipIdentical, blocking=false
regular files, equal length and different hash,
  replace=false                          => Conflict, blocking=true
regular files, equal length and different hash,
  replace=true                           => Replace, blocking=false
```

When verification mode is `Hash`, hash every source regular file and store the lowercase BLAKE3 hex digest in `source_digest`. In either verification mode, same-size source and destination files must both be hashed before selecting `SkipIdentical`; store the source digest. Compute `operation_id` by hashing, in order, the UTF-8-lossy normalized source, a zero byte, destination, a zero byte, `replace`, `verify`, and `ignore_unsupported`; take the first 24 lowercase hex characters. This identifier locates later journal state but is not a security boundary.

Use this implementation shape, with imports for `Path`, `PathBuf`, `hash_file`, `ResolvedEndpoints`, `PlanRequest`, `VerifyMode`, `EntryKind`, `SourceSnapshot`, and `Result`:

```rust
pub fn build_plan(
    request: &PlanRequest,
    endpoints: &ResolvedEndpoints,
    snapshot: &SourceSnapshot,
) -> Result<CopyPlan> {
    let mut entries = Vec::with_capacity(snapshot.entries.len());
    for source_entry in &snapshot.entries {
        let source_path = at_root(&snapshot.root, &source_entry.relative_path);
        let destination_path = at_root(&endpoints.destination, &source_entry.relative_path);

        let mut source_hash = if source_entry.kind == EntryKind::File
            && request.verify == VerifyMode::Hash
        {
            Some(hash_file(&source_path)?)
        } else {
            None
        };

        let (disposition, reason, blocking) = if !source_entry.unsupported.is_empty() {
            if request.ignore_unsupported {
                (Disposition::Unsupported, "unsupported source entry will be omitted", false)
            } else {
                (Disposition::Unsupported, "source entry has unsupported fidelity", true)
            }
        } else {
            match inspect_destination(&destination_path)? {
                None => (Disposition::Copy, "destination does not exist", false),
                Some(destination) if destination.kind != source_entry.kind => (
                    Disposition::Conflict,
                    "source and destination object types differ",
                    true,
                ),
                Some(destination) => match source_entry.kind {
                    EntryKind::Directory => (
                        Disposition::SkipIdentical,
                        "destination directory already exists",
                        false,
                    ),
                    EntryKind::Symlink if destination.link_target == source_entry.link_target => (
                        Disposition::SkipIdentical,
                        "symbolic link targets are identical",
                        false,
                    ),
                    EntryKind::Symlink => (
                        Disposition::Conflict,
                        "destination symbolic link has a different target",
                        true,
                    ),
                    EntryKind::File if destination.length == source_entry.length => {
                        let source_value = match source_hash {
                            Some(value) => value,
                            None => {
                                let value = hash_file(&source_path)?;
                                source_hash = Some(value);
                                value
                            }
                        };
                        let destination_value = hash_file(&destination_path)?;
                        if source_value == destination_value {
                            (Disposition::SkipIdentical, "contents are identical", false)
                        } else if request.replace {
                            (Disposition::Replace, "destination file will be replaced", false)
                        } else {
                            (Disposition::Conflict, "destination file has different contents", true)
                        }
                    }
                    EntryKind::File if request.replace => (
                        Disposition::Replace,
                        "destination file will be replaced",
                        false,
                    ),
                    EntryKind::File => (
                        Disposition::Conflict,
                        "destination file has different contents",
                        true,
                    ),
                    EntryKind::Unsupported => (
                        Disposition::Unsupported,
                        "source entry has unsupported fidelity",
                        !request.ignore_unsupported,
                    ),
                },
            }
        };

        entries.push(PlanEntry {
            relative_path: source_entry.relative_path.clone(),
            source: source_path,
            destination: destination_path,
            kind: source_entry.kind.clone(),
            disposition,
            length: source_entry.length,
            source_digest: source_hash.map(|hash| hash.to_hex().to_string()),
            reason: reason.to_owned(),
            blocking,
        });
    }

    Ok(CopyPlan {
        operation_id: operation_id(request, endpoints),
        source: endpoints.source.clone(),
        destination: endpoints.destination.clone(),
        entries,
    })
}

fn at_root(root: &Path, relative: &Path) -> PathBuf {
    if relative.as_os_str().is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    }
}

fn operation_id(request: &PlanRequest, endpoints: &ResolvedEndpoints) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(endpoints.source.to_string_lossy().as_bytes());
    hasher.update(&[0]);
    hasher.update(endpoints.destination.to_string_lossy().as_bytes());
    hasher.update(&[0]);
    hasher.update(&[
        u8::from(request.replace),
        match request.verify { VerifyMode::Hash => 1, VerifyMode::Size => 2 },
        u8::from(request.ignore_unsupported),
    ]);
    hasher.finalize().to_hex()[..24].to_owned()
}
```

- [ ] **Step 4: Run focused tests, then the whole library suite**

Run:

```powershell
cargo test plan::tests --lib
cargo test --lib
cargo clippy --lib -- -D warnings
```

Expected: all decision-table cases and all earlier tests pass.

- [ ] **Step 5: Commit deterministic planning**

Run:

```powershell
git add src/plan.rs src/error.rs
git commit -m "feat: build deterministic local copy plans"
```

---

### Task 7: Render Human and Versioned JSON Results

**Files:**
- Create: `src/report.rs`
- Modify: `src/lib.rs`
- Test: unit tests in `src/report.rs`

**Interfaces:**
- Consumes: `CopyPlan`, `PlanEntry`, and `OrbitError`.
- Produces: `report::write_plan(&mut impl Write, OutputMode, &CopyPlan) -> io::Result<()>` and `report::write_error(&mut impl Write, OutputMode, &OrbitError) -> io::Result<()>`.

- [ ] **Step 1: Write exact-output tests for all three output modes**

Create `src/report.rs` with this test module below imports for `std::io::{self, Write}`, `OrbitError`, `CopyPlan`, `Disposition`, `PlanEntry`, and `OutputMode`:

```rust
#[cfg(test)]
mod tests {
    use super::{write_error, write_plan};
    use crate::error::OrbitError;
    use crate::plan::{CopyPlan, Disposition, PlanEntry};
    use crate::request::OutputMode;
    use crate::scan::EntryKind;
    use std::path::PathBuf;

    fn fixture() -> CopyPlan {
        CopyPlan {
            operation_id: "0123456789abcdef01234567".into(),
            source: PathBuf::from("C:/source"),
            destination: PathBuf::from("D:/target"),
            entries: vec![
                PlanEntry {
                    relative_path: PathBuf::from("new.txt"),
                    source: PathBuf::from("C:/source/new.txt"),
                    destination: PathBuf::from("D:/target/new.txt"),
                    kind: EntryKind::File,
                    disposition: Disposition::Copy,
                    length: 3,
                    source_digest: Some("abc".into()),
                    reason: "destination does not exist".into(),
                    blocking: false,
                },
                PlanEntry {
                    relative_path: PathBuf::from("blocked.txt"),
                    source: PathBuf::from("C:/source/blocked.txt"),
                    destination: PathBuf::from("D:/target/blocked.txt"),
                    kind: EntryKind::File,
                    disposition: Disposition::Conflict,
                    length: 7,
                    source_digest: Some("def".into()),
                    reason: "destination file has different contents".into(),
                    blocking: true,
                },
            ],
        }
    }

    #[test]
    fn human_report_explains_actions_and_blocking_summary() {
        let mut bytes = Vec::new();
        write_plan(&mut bytes, OutputMode::Human, &fixture()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("C:/source"));
        assert!(text.contains("D:/target"));
        assert!(text.contains("COPY"));
        assert!(text.contains("CONFLICT"));
        assert!(text.contains("Plan blocked: 1 conflict"));
    }

    #[test]
    fn quiet_suppresses_plan_but_not_error() {
        let mut plan_bytes = Vec::new();
        write_plan(&mut plan_bytes, OutputMode::Quiet, &fixture()).unwrap();
        assert!(plan_bytes.is_empty());

        let mut error_bytes = Vec::new();
        write_error(
            &mut error_bytes,
            OutputMode::Quiet,
            &OrbitError::SourceMissing(PathBuf::from("missing")),
        )
        .unwrap();
        assert_eq!(String::from_utf8(error_bytes).unwrap(), "error: source does not exist: missing\n");
    }

    #[test]
    fn json_is_one_versioned_object_per_line_with_final_result() {
        let mut bytes = Vec::new();
        write_plan(&mut bytes, OutputMode::Json, &fixture()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let values: Vec<serde_json::Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(values.len(), 3);
        assert!(values.iter().all(|value| value["schema_version"] == 1));
        assert_eq!(values[0]["event"], "plan_entry");
        assert_eq!(values[1]["event"], "plan_entry");
        assert_eq!(values[2]["event"], "plan_result");
        assert_eq!(values[2]["data"]["blocking"], 1);
    }
}
```

Declare `write_plan` and `write_error` with the signatures in this task's Interfaces block and bodies containing `unreachable!()`. Run `cargo test report::tests --lib`; expect failure at those bodies.

- [ ] **Step 2: Implement the JSON envelope and stable human summary**

Use this envelope rather than serializing ad hoc maps:

```rust
#[derive(serde::Serialize)]
struct JsonEvent<'a, T: serde::Serialize> {
    schema_version: u32,
    event: &'a str,
    data: T,
}

#[derive(serde::Serialize)]
struct PlanResult<'a> {
    operation_id: &'a str,
    executable: bool,
    entries: usize,
    blocking: usize,
}
```

`write_plan` iterates entries in plan order. JSON event names are `plan_entry` and `plan_result`. Human disposition labels are `COPY`, `SKIP`, `REPLACE`, `CONFLICT`, and `UNSUPPORTED`. `write_error` emits either `error: <message>` or one `error` JSON event with `code`, `exit_code`, and `message`; define a stable `OrbitError::code() -> &'static str` beside `exit_code()`.

Add this method to `OrbitError`:

```rust
pub fn code(&self) -> &'static str {
    match self {
        Self::SourceMissing(_) => "source_missing",
        Self::SameEndpoint(_) => "same_endpoint",
        Self::DestinationInsideSource { .. } => "destination_inside_source",
        Self::Io { .. } => "io_error",
    }
}
```

Implement the rendering functions as follows:

```rust
pub fn write_plan(
    writer: &mut impl Write,
    mode: OutputMode,
    plan: &CopyPlan,
) -> io::Result<()> {
    match mode {
        OutputMode::Quiet => Ok(()),
        OutputMode::Human => {
            writeln!(writer, "Source: {}", plan.source.display())?;
            writeln!(writer, "Destination: {}", plan.destination.display())?;
            for entry in &plan.entries {
                let label = match entry.disposition {
                    Disposition::Copy => "COPY",
                    Disposition::SkipIdentical => "SKIP",
                    Disposition::Replace => "REPLACE",
                    Disposition::Conflict => "CONFLICT",
                    Disposition::Unsupported => "UNSUPPORTED",
                };
                writeln!(writer, "{label} {} — {}", entry.relative_path.display(), entry.reason)?;
            }
            let blocking = plan.entries.iter().filter(|entry| entry.blocking).count();
            if blocking == 0 {
                writeln!(writer, "Plan ready: {} entries", plan.entries.len())
            } else {
                let noun = if blocking == 1 { "conflict" } else { "conflicts" };
                writeln!(writer, "Plan blocked: {blocking} {noun}")
            }
        }
        OutputMode::Json => {
            for entry in &plan.entries {
                write_json_line(writer, &JsonEvent {
                    schema_version: 1,
                    event: "plan_entry",
                    data: entry,
                })?;
            }
            let blocking = plan.entries.iter().filter(|entry| entry.blocking).count();
            write_json_line(writer, &JsonEvent {
                schema_version: 1,
                event: "plan_result",
                data: PlanResult {
                    operation_id: &plan.operation_id,
                    executable: plan.is_executable(),
                    entries: plan.entries.len(),
                    blocking,
                },
            })
        }
    }
}

#[derive(serde::Serialize)]
struct ErrorData<'a> {
    code: &'a str,
    exit_code: u8,
    message: String,
}

pub fn write_error(
    writer: &mut impl Write,
    mode: OutputMode,
    error: &OrbitError,
) -> io::Result<()> {
    match mode {
        OutputMode::Json => write_json_line(writer, &JsonEvent {
            schema_version: 1,
            event: "error",
            data: ErrorData {
                code: error.code(),
                exit_code: error.exit_code(),
                message: error.to_string(),
            },
        }),
        OutputMode::Human | OutputMode::Quiet => writeln!(writer, "error: {error}"),
    }
}

fn write_json_line(writer: &mut impl Write, value: &impl serde::Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writeln!(writer)
}
```

- [ ] **Step 3: Verify rendering and commit**

Run:

```powershell
cargo test report::tests --lib
cargo fmt --all -- --check
cargo clippy --lib -- -D warnings
git add src/report.rs src/error.rs src/lib.rs
git commit -m "feat: report plans for humans and automation"
```

Expected: exact-output tests pass and there are no warnings.

---

### Task 8: Wire the Planner Application and Prove It Never Writes

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Modify: `tests/cli_contract.rs`
- Create: `tests/plan_integration.rs`
- Modify: `README.md`
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: all preceding module interfaces.
- Produces: `app::run_plan(request, cwd, stdout, stderr) -> u8` and the public `orbit plan` behavior.

- [ ] **Step 1: Write failing end-to-end planner tests**

Create `tests/plan_integration.rs` with real temporary trees and these assertions:

```rust
use predicates::prelude::*;

fn orbit() -> assert_cmd::Command {
    assert_cmd::cargo::cargo_bin_cmd!("orbit")
}

#[test]
fn plan_missing_destination_reports_copy_and_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    let destination = temp.path().join("destination.txt");
    std::fs::write(&source, b"payload").unwrap();

    orbit()
        .args(["plan", source.to_str().unwrap(), destination.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("COPY"));

    assert!(!destination.exists());
    assert!(!temp.path().join(".orbit-state").exists());
}

#[test]
fn identical_destination_reports_skip() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    std::fs::write(&source, b"identical").unwrap();
    std::fs::write(&destination, b"identical").unwrap();
    orbit()
        .args(["plan", source.to_str().unwrap(), destination.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("SKIP"));
    assert_eq!(std::fs::read(&destination).unwrap(), b"identical");
}

#[test]
fn conflicting_destination_exits_three_and_is_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    std::fs::write(&source, b"source").unwrap();
    std::fs::write(&destination, b"before").unwrap();
    orbit()
        .args(["plan", source.to_str().unwrap(), destination.to_str().unwrap()])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("CONFLICT"));
    assert_eq!(std::fs::read(&destination).unwrap(), b"before");
}

#[test]
fn replace_reports_replace_but_still_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    std::fs::write(&source, b"new").unwrap();
    std::fs::write(&destination, b"old").unwrap();
    orbit()
        .args(["plan", source.to_str().unwrap(), destination.to_str().unwrap(), "--replace"])
        .assert()
        .success()
        .stdout(predicate::str::contains("REPLACE"));
    assert_eq!(std::fs::read(&destination).unwrap(), b"old");
}

#[test]
fn json_is_ndjson_and_ends_with_plan_result() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    std::fs::write(&source, b"payload").unwrap();
    let output = orbit()
        .args(["plan", source.to_str().unwrap(), destination.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let values: Vec<serde_json::Value> = text.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(values.last().unwrap()["event"], "plan_result");
    assert!(values.iter().all(|value| value["schema_version"] == 1));
}

#[test]
fn directory_destination_is_the_exact_root_not_a_basename_container() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("photos");
    let destination = temp.path().join("archive");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("one.jpg"), b"one").unwrap();
    let output = orbit()
        .args(["plan", source.to_str().unwrap(), destination.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains(&destination.join("one.jpg").display().to_string()));
    assert!(!text.contains(&destination.join("photos/one.jpg").display().to_string()));
    assert!(!destination.exists());
}

#[test]
fn unrelated_destination_entries_are_not_planned_or_removed() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(source.join("new.txt"), b"new").unwrap();
    std::fs::write(destination.join("unrelated.txt"), b"keep").unwrap();
    orbit()
        .args(["plan", source.to_str().unwrap(), destination.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("unrelated.txt").not());
    assert_eq!(std::fs::read(destination.join("unrelated.txt")).unwrap(), b"keep");
}

#[test]
fn missing_source_exits_three() {
    let temp = tempfile::tempdir().unwrap();
    orbit()
        .args([
            "plan",
            temp.path().join("missing").to_str().unwrap(),
            temp.path().join("target").to_str().unwrap(),
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("source does not exist"));
}

#[test]
fn destination_inside_source_exits_three() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    std::fs::create_dir(&source).unwrap();
    orbit()
        .args([
            "plan",
            source.to_str().unwrap(),
            source.join("nested/target").to_str().unwrap(),
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("inside the source tree"));
}
```

Run `cargo test --test plan_integration`; expect failures because `main` does not orchestrate planning.

- [ ] **Step 2: Implement orchestration with injectable streams and working directory**

Create `src/app.rs`:

```rust
use std::io::Write;
use std::path::Path;

use crate::paths::resolve_endpoints;
use crate::plan::build_plan;
use crate::report::{write_error, write_plan};
use crate::request::PlanRequest;
use crate::scan::scan_source;

pub fn run_plan(
    request: PlanRequest,
    cwd: &Path,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let output = request.output;
    let result = (|| {
        let endpoints = resolve_endpoints(&request.source, &request.destination, cwd)?;
        let snapshot = scan_source(&endpoints.source)?;
        build_plan(&request, &endpoints, &snapshot)
    })();

    match result {
        Ok(plan) => {
            if let Err(error) = write_plan(stdout, output, &plan) {
                let _ = writeln!(stderr, "error: cannot write report: {error}");
                return 1;
            }
            if plan.is_executable() { 0 } else { 3 }
        }
        Err(error) => {
            let code = error.exit_code();
            if output == crate::request::OutputMode::Json {
                let _ = write_error(stdout, output, &error);
            } else {
                let _ = write_error(stderr, output, &error);
            }
            code
        }
    }
}
```

Export `app` from `src/lib.rs`. Replace `src/main.rs` with:

```rust
use std::process::ExitCode;

use clap::Parser;
use orbit::app::run_plan;
use orbit::cli::Cli;

fn main() -> ExitCode {
    let request = Cli::parse().into_request();
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            eprintln!("error: cannot determine current directory: {error}");
            return ExitCode::from(1);
        }
    };
    ExitCode::from(run_plan(
        request,
        &cwd,
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    ))
}
```

- [ ] **Step 3: Run the full integration suite and verify zero writes**

Run:

```powershell
cargo test --test plan_integration
cargo test --test cli_contract
```

Expected: all process-level tests pass, every destination fingerprint captured before planning equals the fingerprint afterward, and no `.orbit-state` or partial file exists.

- [ ] **Step 4: Document only the working milestone**

Append to `README.md`:

````markdown
## Current milestone

The reboot currently provides read-only local planning:

```text
orbit plan SOURCE DEST [--replace] [--verify hash|size] [--ignore-unsupported] [--json|--quiet]
```

`DEST` is the exact target path. Planning scans and hashes as required but never writes source, destination, journal, or partial data. The `copy` command will appear only when safe staging and verification are implemented.
````

- [ ] **Step 5: Add the minimal cross-platform CI gate**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all-targets

  msrv:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: 1.85.0
      - run: cargo test --all-targets
```

- [ ] **Step 6: Run the complete local quality gate**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
git diff --check
git status --short
```

Expected: every command exits `0`; all tests pass; the only uncommitted changes are the Task 8 files.

- [ ] **Step 7: Manually exercise the exact-target contract**

Run in a temporary directory outside the repository:

```powershell
$orbitPlanCheck = Join-Path ([System.IO.Path]::GetTempPath()) ("orbit-plan-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $orbitPlanCheck | Out-Null
Set-Content -LiteralPath (Join-Path $orbitPlanCheck 'source.txt') -Value 'orbit'
cargo run -- plan (Join-Path $orbitPlanCheck 'source.txt') (Join-Path $orbitPlanCheck 'target.txt') --json
Test-Path -LiteralPath (Join-Path $orbitPlanCheck 'target.txt')
```

Expected: NDJSON reports one `copy`, the final result is executable, and `Test-Path` prints `False`. Remove only the explicitly printed `$orbitPlanCheck` directory after resolving and verifying that it is beneath `[System.IO.Path]::GetTempPath()`.

- [ ] **Step 8: Commit the working read-only planner**

Run:

```powershell
git add src/app.rs src/main.rs src/lib.rs tests README.md .github/workflows/ci.yml
git commit -m "feat: deliver the read-only Orbit planner"
```

---

## Milestone Acceptance

Before opening a review, verify from a clean checkout of `codex/orbit-reboot`:

```powershell
git status --short
git tag --list archive/pre-reboot-2026-09-22
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo run -- --help
```

Acceptance requires:

- the archive tag resolves to the commit immediately before the reset;
- the active tree contains no legacy implementation or misleading capability claims;
- `orbit --help` exposes `plan` but not `copy`;
- all unit, integration, CLI, and portable platform tests pass;
- planning missing, identical, conflicting, replacement, directory, and symlink cases is deterministic;
- planning never creates a destination, partial file, or Orbit state directory;
- blocked plans and invalid endpoints exit `3`; invalid CLI syntax exits `2`; successful plans exit `0`;
- each NDJSON line parses independently and uses schema version `1`;
- the working tree is clean.
