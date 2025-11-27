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
