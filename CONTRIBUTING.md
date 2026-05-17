# Contributing to Orbit

Thanks for your interest in contributing to Orbit! We welcome community contributions to help build a better, smarter, and more resilient data mover.

---

## 🛠 How to Contribute

1. **Fork the repository** and create a new branch:
   ```
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** with clear, readable code and comments where appropriate.

3. **Test your changes** locally. If possible, write or update unit tests.

4. **Commit** using clear commit messages:
   ```
   git commit -m "Add support for XYZ protocol"
   ```

5. **Push your branch** and open a **Pull Request**. In your PR description, explain the what, why, and how of your change.

---

## ✍ Contribution Guidelines

- Keep code clean, modular, and idiomatic Rust
- Try to match the existing formatting and structure
- Follow naming conventions and doc-comment (`///`) where applicable
- One logical change per PR
- Use `tracing` macros (`info!`, `warn!`, `error!`, `debug!`) instead of `println!`/`eprintln!` for all output
- Run `cargo fmt --all && cargo clippy --workspace` before submitting
- **CLI args that have config-file counterparts must use `Option<T>`** (not hardcoded defaults) to avoid clobbering profile/config presets
- **All human-visible output must be gated on `json_output` and `quiet`** — never write to stdout unconditionally in the transfer path
- **New CLI flags should be `global = true`** and tagged with an appropriate `help_heading` for grouped help output
- **Config resolution follows the 4-layer priority**: config file → auto-network overlay → active tuning → CLI flags. Extract testable helpers rather than adding inline logic to `run()`

---

## 🏗️ Architecture: OrbitSystem Pattern

Orbit uses the **OrbitSystem trait** to abstract filesystem and compute operations.

#### Key Components

1. **`orbit-core-interface`**: Defines the `OrbitSystem` trait
   - Discovery operations (`exists`, `metadata`, `read_dir`)
   - Data access (`reader`, `writer`)
   - Compute offloading (`read_header`, `calculate_hash`)

2. **`LocalSystem`**: Default implementation for standalone mode
   - Located in `src/system/local.rs`
   - Wraps `tokio::fs` operations
   - Zero-overhead abstraction via monomorphization

3. **`MockSystem`**: In-memory implementation for testing
   - Located in `src/system/mock.rs`
   - No filesystem I/O required
   - Deterministic test results

#### Using OrbitSystem in Your Code

When adding new features that need filesystem access, use the `OrbitSystem` trait:

```rust
use orbit_core_interface::{OrbitSystem, Result};
use std::path::Path;

async fn process_file<S: OrbitSystem>(system: &S, path: &Path) -> Result<()> {
    // Check if file exists
    if !system.exists(path).await {
        return Ok(());
    }

    // Read file header for analysis
    let header = system.read_header(path, 512).await?;

    // Process...
    Ok(())
}
```

> The `Result` type alias is `std::result::Result<T, OrbitSystemError>`. Orbit's first-party crates use `thiserror`-based error types throughout — avoid pulling `anyhow` into new code.

#### Testing with MockSystem

Write tests without touching the filesystem:

```rust
#[tokio::test]
async fn test_process_file() {
    use orbit::system::MockSystem;

    let system = MockSystem::new();
    system.add_file("/test.txt", b"Hello, World!");

    let result = process_file(&system, Path::new("/test.txt")).await;
    assert!(result.is_ok());
}
```

For more details, see the `orbit-core-interface` crate.

---

## 🏗️ Building and Testing

### Resource Optimization
You might notice that compiling dependencies takes a bit longer the first time, but running tests is significantly faster.

This is because `Cargo.toml` includes a specific profile override:
```toml
[profile.dev.package."*"]
opt-level = 3
debug = 0
```

This configuration strips debug symbols from external dependencies while keeping them for the Orbit codebase. This creates significantly smaller binaries, preventing "Bus Error" and "No space left on device" crashes on CI and saving disk space on your local machine.

### Running Full Suite
To run the full test suite:

```bash
cargo test --workspace
```

To include optional network backends (S3, SMB, SSH, etc.):

```bash
cargo test --features network
```

---

## 🏷️ Issue & PR Maturity Labels

We use maturity labels to track the stability of features and focus effort where it matters most:

| Label | Meaning | Contribution Focus |
|-------|---------|-------------------|
| `maturity:stable` | Production-ready, well-tested | Bug fixes, performance improvements |
| `maturity:beta` | Functional, needs validation | Integration tests, edge-case testing, real-world reports |
| `maturity:alpha` | Experimental, expect changes | Design feedback, API review (avoid large builds on top) |
| `good-first-issue` | Approachable for new contributors | Great starting point |
| `stabilize` | Promote a beta feature toward stable | Tests, docs, edge-case fixes needed |

When opening a PR, tag it with the appropriate maturity label if it touches a specific feature area.

---

## 🎯 Stabilization Contributions

The highest-leverage contributions right now are helping **promote beta features to stable**. To stabilize a feature:

1. **Add integration tests** covering the happy path and key edge cases
2. **Test on your platform** (Linux, macOS, Windows) and report results
3. **Document gaps** you find -- missing error handling, unclear behavior, platform-specific issues
4. **Write or improve guides** in `docs/guides/` for the feature

Priority stabilization targets:
- Resume/checkpoint edge cases (partial failures, concurrent access)
- S3 backend (multipart uploads, large file handling, credential chains)
- SSH/SFTP backend (connection handling, key formats, resume)
- Disk Guardian (edge cases: symlinks, mount points, quotas)
- Filter system (complex glob/regex interactions)

---

## 🔬 Testing Guidelines

### Running Tests

```bash
# Full workspace test suite
cargo test --workspace

# With network backends
cargo test --features network

# Specific crate
cargo test -p orbit-core-cdc
```

### Writing Tests

- **Integration tests** are more valuable than unit tests for transfer logic
- Use `MockSystem` (from `orbit-core-interface`) for tests that don’t need real I/O
- For S3 tests, use MinIO or LocalStack containers when possible
- Property-based tests (via `proptest`) are welcome for CDC and checksum logic
- Always test resume scenarios: interrupt mid-transfer, verify recovery

---

## 🚦 Feature Ideas

We welcome contributions such as:

- Stabilization of beta features (see above)
- Integration tests for real backends (MinIO, SSH containers)
- Performance benchmarks against rsync/rclone
- UX enhancements (better CLI output, error messages)
- Platform-specific testing and fixes
- Documentation improvements

---

## 📋 Scope Review

Before adding new features, consider:

- **Does this stabilize existing functionality?** Preferred over new features.
- **Can this be feature-gated?** New experimental features should use Cargo feature flags.
- **Does this increase the default binary size?** Keep the minimal build small.
- **Is there a simpler approach?** Three clear lines beat a clever abstraction.

---

## 💬 Communication

- Open an Issue for bugs, feature requests, or design discussions
- Tag issues with maturity labels and platform labels (`os:linux`, `os:macos`, `os:windows`)
- For now, we’re using GitHub Issues + Discussions. A Discord or Matrix space may follow if interest grows.

---

## 📜 License and Agreement

By submitting a contribution, you agree that your code will be licensed under the Apache License 2.0 as specified in this repository. See [LICENSE](LICENSE) for details.

---

Let’s build something great. Welcome aboard.
