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

---

## 🏗️ Architecture: OrbitSystem Pattern

### Phase 1: I/O Abstraction Layer

As of v0.6.0, Orbit uses the **OrbitSystem trait** to abstract filesystem and compute operations. This enables both local (standalone) and distributed (Grid/Star) topologies with the same codebase.

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
use orbit_core_interface::OrbitSystem;
use std::path::Path;

async fn process_file<S: OrbitSystem>(system: &S, path: &Path) -> anyhow::Result<()> {
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

For more details, see [`docs/specs/PHASE_1_ABSTRACTION_SPEC.md`](docs/specs/PHASE_1_ABSTRACTION_SPEC.md).

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

This configuration strips debug symbols from external dependencies (like AWS SDKs and Leptos) while keeping them for the Orbit codebase. This creates significantly smaller binaries, preventing "Bus Error" and "No space left on device" crashes on CI and saving disk space on your local machine.

### Running Full Suite
To run the full test suite exactly as the CI does (including S3, SMB, and GUI components):

```bash
cargo test --features full
```

---

## 🚦 Feature Ideas

We welcome contributions such as:

- New protocol handlers (e.g. SFTP, SMB)
- Optimisation improvements (e.g. multithreading, caching)
- UX enhancements (e.g. better CLI output)
- Reliability features (e.g. smarter retry/backoff)
- Docs and examples

---

## 💬 Communication

- Open an Issue for bugs, feature requests, or design discussions
- For now, we're using GitHub Issues + Discussions. A Discord or Matrix space may follow if interest grows.

---

## 📜 License and Agreement

By submitting a contribution, you agree that your code will be licensed under the Apache License 2.0 as specified in this repository. See [LICENSE](LICENSE) for details.

---

Let’s build something great. Welcome aboard.
