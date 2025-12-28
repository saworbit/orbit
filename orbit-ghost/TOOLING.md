# Orbit GhostFS Tooling & Quality Assurance

Comprehensive guide to code quality tools, linters, formatters, and CI/CD configuration.

## Table of Contents

- [Overview](#overview)
- [Code Formatting](#code-formatting)
- [Linting](#linting)
- [Security Auditing](#security-auditing)
- [Testing](#testing)
- [CI/CD Pipeline](#cicd-pipeline)
- [Development Workflow](#development-workflow)
- [Pre-commit Checks](#pre-commit-checks)

## Overview

Orbit GhostFS uses industry-standard Rust tooling to maintain code quality, security, and consistency.

### Quality Tools Stack

| Tool | Purpose | Configuration |
|------|---------|---------------|
| **rustfmt** | Code formatting | [rustfmt.toml](rustfmt.toml) |
| **clippy** | Linting & best practices | [clippy.toml](clippy.toml) |
| **cargo-audit** | Security vulnerability scanning | Built-in |
| **cargo-deny** | License & dependency management | [deny.toml](deny.toml) |
| **cargo-tarpaulin** | Code coverage | CI only |
| **criterion** | Benchmarking | [Cargo.toml](Cargo.toml) |

## Code Formatting

### rustfmt

**Configuration:** [rustfmt.toml](rustfmt.toml)

**Key Settings:**
- Max line width: 100 characters
- Edition: 2021
- Unix line endings
- 4-space indentation

**Usage:**

```bash
# Format all code
cargo fmt

# Check formatting without modifying
cargo fmt --check

# Via Makefile
make fmt
make fmt-check
```

**Editor Integration:**

**VS Code** (`.vscode/settings.json`):
```json
{
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

**IntelliJ/CLion:**
- **Settings** → **Languages & Frameworks** → **Rust** → **Rustfmt**
- Enable "Run rustfmt on Save"

**Pre-commit Hook:**

Create `.git/hooks/pre-commit`:
```bash
#!/bin/bash
cargo fmt --check
if [ $? -ne 0 ]; then
  echo "❌ Code formatting check failed. Run 'cargo fmt' to fix."
  exit 1
fi
```

## Linting

### Clippy

**Configuration:** [clippy.toml](clippy.toml)

**Enabled Lint Groups:**
- All default lints
- Pedantic lints (warnings for subtle issues)
- Cargo lints (manifest best practices)

**Custom Thresholds:**
- Max function arguments: 8
- Max function lines: 150
- Cognitive complexity: 30

**Usage:**

```bash
# Run clippy
cargo clippy --all-targets --all-features

# Deny all warnings (CI mode)
cargo clippy -- -D warnings

# Via Makefile
make lint
```

**Suppressing Lints:**

When necessary (rare), use attributes:

```rust
// Suppress specific lint for one item
#[allow(clippy::too_many_arguments)]
fn complex_function(a: i32, b: i32, c: i32, /* ... */) {}

// Suppress for entire module
#![allow(clippy::module_name_repetitions)]

// Better: Fix the issue instead of suppressing
```

**Common Lints:**

| Lint | What it catches | Fix |
|------|-----------------|-----|
| `needless_return` | Unnecessary `return` keyword | Remove explicit return |
| `unused_variable` | Unused variables | Remove or prefix with `_` |
| `single_match` | `match` that could be `if let` | Use `if let` |
| `unwrap_used` | `.unwrap()` in code | Use `?` or `expect()` |

## Security Auditing

### cargo-audit

Checks dependencies against the RustSec Advisory Database.

**Installation:**
```bash
cargo install cargo-audit
```

**Usage:**
```bash
# Check for vulnerabilities
cargo audit

# Update advisory database
cargo audit --update

# Via Makefile
make audit
```

**Output Example:**
```
Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
    Scanning Cargo.lock for vulnerabilities (105 crate dependencies)
✅ Success: No vulnerabilities found!
```

**If vulnerabilities found:**
1. Update the affected dependency: `cargo update -p <crate-name>`
2. Check if a patched version exists
3. If not, consider alternative crates
4. Add to `deny.toml` ignore list (with justification) only if no fix available

### cargo-deny

Comprehensive dependency management: licenses, security, bans, sources.

**Configuration:** [deny.toml](deny.toml)

**Installation:**
```bash
cargo install cargo-deny
```

**Usage:**
```bash
# Run all checks
cargo deny check

# Check specific category
cargo deny check licenses
cargo deny check advisories
cargo deny check bans
cargo deny check sources

# Via Makefile
make deny
```

**What it checks:**

1. **Licenses:**
   - Allowed: MIT, Apache-2.0, BSD-*, ISC, Unlicense
   - Denied: GPL-3.0, AGPL-3.0

2. **Advisories:**
   - Security vulnerabilities
   - Unmaintained crates
   - Yanked versions

3. **Bans:**
   - Duplicate dependency versions
   - Explicitly banned crates

4. **Sources:**
   - Unknown git repositories
   - Unknown registries

## Testing

### Unit Tests

**Location:** Inline with code (`#[cfg(test)]` modules)

**Running:**
```bash
# All tests
cargo test

# Specific test
cargo test test_block_calculation

# With output
cargo test -- --nocapture

# Via Makefile
make test
```

### Integration Tests

**Location:** `tests/` directory (planned)

**Running:**
```bash
cargo test --test integration_test -- --ignored
```

### Benchmarks

**Location:** `benches/` directory (planned)

**Running:**
```bash
cargo bench
make bench
```

**Framework:** Criterion

### Code Coverage

**Tool:** cargo-tarpaulin (Linux only)

**Installation:**
```bash
cargo install cargo-tarpaulin
```

**Running:**
```bash
# Generate HTML report
cargo tarpaulin --out html

# Generate XML (for CI)
cargo tarpaulin --out xml

# Via Makefile
make coverage
```

**CI Integration:** Automatically uploads to Codecov on main branch.

## CI/CD Pipeline

### GitHub Actions

**Configuration:** [.github/workflows/ci.yml](.github/workflows/ci.yml)

**Workflow Jobs:**

1. **Format Check** (`format`)
   - Runs: `cargo fmt --check`
   - Fails if code not formatted

2. **Clippy Lint** (`clippy`)
   - Runs: `cargo clippy -- -D warnings`
   - Fails on any warnings

3. **Build & Test** (`build-test`)
   - Platforms: Ubuntu, macOS
   - Rust versions: stable, beta
   - Runs full build and test suite

4. **Security Audit** (`security-audit`)
   - Runs: `cargo audit`
   - Checks for vulnerabilities

5. **Dependency Review** (`dependency-review`)
   - Runs on pull requests
   - Reviews new dependencies

6. **Outdated Check** (`outdated`)
   - Checks for outdated dependencies
   - Informational (doesn't fail)

7. **Cargo Deny** (`cargo-deny`)
   - Comprehensive dependency checks
   - License validation

8. **Code Coverage** (`coverage`)
   - Runs on main branch only
   - Uploads to Codecov

9. **MSRV Check** (`msrv`)
   - Verifies Rust 1.70 compatibility

**Triggering:**
- Push to `main` or `develop`
- Pull requests to `main` or `develop`
- Weekly cron (security audit)

**Status Badges:**

Add to README.md:
```markdown
[![CI](https://github.com/saworbit/orbit/workflows/CI/badge.svg)](https://github.com/saworbit/orbit/actions)
[![Security Audit](https://github.com/saworbit/orbit/workflows/Security%20Audit/badge.svg)](https://github.com/saworbit/orbit/actions)
```

## Development Workflow

### Makefile

**Configuration:** [Makefile](Makefile)

**Common Commands:**

```bash
# Development
make build          # Debug build
make release        # Optimized build
make run            # Run the application
make demo           # Run demo script

# Code quality
make fmt            # Format code
make lint           # Run clippy
make test           # Run tests
make check-all      # Run all checks

# Pre-commit
make pre-commit     # Format + lint + test

# CI simulation
make ci             # Replicate CI checks locally

# Documentation
make docs           # Build and open docs

# Cleaning
make clean          # Remove build artifacts
make clean-cache    # Remove runtime cache
```

### Recommended Workflow

**Before committing:**

```bash
# 1. Format code
make fmt

# 2. Run linter
make lint

# 3. Run tests
make test

# 4. Optional: Run all checks
make check-all
```

**Before pushing:**

```bash
# Simulate CI locally
make ci
```

**Before creating PR:**

```bash
# Full pre-release check
make pre-release
```

## Pre-commit Checks

### Manual Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
set -e

echo "🔍 Running pre-commit checks..."

# Format check
echo "  → Checking formatting..."
cargo fmt --check
if [ $? -ne 0 ]; then
  echo "❌ Formatting failed. Run 'cargo fmt' to fix."
  exit 1
fi

# Clippy
echo "  → Running clippy..."
cargo clippy --all-targets -- -D warnings
if [ $? -ne 0 ]; then
  echo "❌ Clippy failed. Fix warnings before committing."
  exit 1
fi

# Tests
echo "  → Running tests..."
cargo test --quiet
if [ $? -ne 0 ]; then
  echo "❌ Tests failed. Fix tests before committing."
  exit 1
fi

echo "✅ All pre-commit checks passed!"
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

### Automated with pre-commit Framework

**Installation:**
```bash
pip install pre-commit
```

**Configuration** (`.pre-commit-config.yaml`):

```yaml
repos:
  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt
        language: system
        types: [rust]
        pass_filenames: false

      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy -- -D warnings
        language: system
        types: [rust]
        pass_filenames: false

      - id: cargo-test
        name: cargo test
        entry: cargo test
        language: system
        types: [rust]
        pass_filenames: false
```

**Setup:**
```bash
pre-commit install
```

Now hooks run automatically on `git commit`.

## Configuration Files

### Summary

| File | Purpose |
|------|---------|
| [rustfmt.toml](rustfmt.toml) | Code formatting rules |
| [clippy.toml](clippy.toml) | Linter configuration |
| [deny.toml](deny.toml) | Dependency management |
| [.editorconfig](.editorconfig) | Editor settings |
| [.gitignore](.gitignore) | Git ignore rules |
| [Cargo.toml](Cargo.toml) | Project metadata & dependencies |
| [Makefile](Makefile) | Development commands |
| [.github/workflows/ci.yml](.github/workflows/ci.yml) | CI/CD pipeline |

## Troubleshooting

### "cargo fmt not found"

```bash
rustup component add rustfmt
```

### "cargo clippy not found"

```bash
rustup component add clippy
```

### "cargo audit not found"

```bash
cargo install cargo-audit
```

### CI fails but local passes

Ensure you're using the same Rust version:
```bash
rustc --version
# Should match CI (stable)

rustup update stable
```

### "Too many clippy warnings"

Temporarily allow while fixing:
```rust
#![allow(clippy::all)]  // Top of main.rs
```

Then remove attribute and fix one lint at a time.

## Best Practices

### ✅ Do

- Run `make fmt` before every commit
- Run `make lint` regularly during development
- Run `make check-all` before creating pull requests
- Update dependencies monthly: `cargo update`
- Check security advisories weekly: `cargo audit`
- Write tests for new features
- Document public APIs

### ❌ Don't

- Commit without formatting
- Ignore clippy warnings
- Use `#[allow]` without good reason
- Skip tests in CI
- Merge PRs with failing checks
- Use deprecated dependencies

## Resources

- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
- [rustfmt Config](https://rust-lang.github.io/rustfmt/)
- [cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit)
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/)
- [GitHub Actions](https://docs.github.com/en/actions)

---

**Questions?** Open an issue or discussion on GitHub.
