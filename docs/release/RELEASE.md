# Orbit Release Process

This document describes the release process for Orbit and its associated crates.

## Table of Contents

- [Overview](#overview)
- [Versioning Strategy](#versioning-strategy)
- [Pre-Release Checklist](#pre-release-checklist)
- [Release Steps](#release-steps)
- [Post-Release Tasks](#post-release-tasks)
- [Hotfix Process](#hotfix-process)
- [Automation](#automation)

---

## Overview

Orbit uses a workspace-based release strategy where:
- The main `orbit` binary and core crates share version numbers
- Independent crates (`magnetar`, `orbit-web`) may have their own versions
- Releases include both source code and pre-built binaries for major platforms

**Current Version:** `0.6.0`

---

## Versioning Strategy

We follow [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH

MAJOR - Incompatible API changes
MINOR - New functionality (backwards-compatible)
PATCH - Bug fixes (backwards-compatible)
```

### Version Bumping Guidelines

**MAJOR (x.0.0):**
- Breaking API changes
- Major architectural changes
- Incompatible CLI flag changes

**MINOR (0.x.0):**
- New features (Web GUI, new protocols, etc.)
- New CLI flags (backwards-compatible)
- Performance improvements
- New crates added to workspace

**PATCH (0.0.x):**
- Bug fixes
- Documentation updates
- Dependency updates (security)
- Performance optimizations (no API changes)

---

## Pre-Release Checklist

### 1. Code Quality

```bash
# Run all tests
cargo test --workspace --all-features

# Run ignored tests (timing-sensitive)
cargo test --workspace --all-features -- --ignored

# Check for compilation warnings
cargo clippy --workspace --all-features -- -D warnings

# Format code
cargo fmt --all -- --check

# Check documentation
cargo doc --workspace --all-features --no-deps
```

### 2. Version Updates

Update version numbers in:

```toml
# Root Cargo.toml
[package]
version = "0.5.0"  # Update this

# crates/magnetar/Cargo.toml
[package]
version = "0.2.0"  # Update if changed

# crates/orbit-web/Cargo.toml
[package]
version = "0.2.0"  # Update if changed

# Other workspace crates
# crates/core-manifest/Cargo.toml
# crates/core-starmap/Cargo.toml
# crates/core-audit/Cargo.toml
```

### 3. Update CHANGELOG

Create/update `CHANGELOG.md`:

```markdown
# Changelog

## [0.5.0] - 2025-11-15

### Added
- Web GUI for transfer orchestration (orbit-web crate)
- Real-time progress tracking via WebSocket
- Job management dashboard with auto-refresh
- Comprehensive Web GUI documentation

### Changed
- Improved error handling with better diagnostics
- Enhanced progress reporting system

### Fixed
- Fixed race condition in parallel copy operations
- Corrected metadata preservation on Windows

### Security
- Updated dependencies with security patches

## [0.4.1] - 2025-11-10
...
```

### 4. Update Documentation

- [ ] Update `README.md` with new features
- [ ] Update version references in docs
- [ ] Verify all code examples work
- [ ] Check all documentation links
- [ ] Update screenshots if UI changed

### 5. Verify Build Artifacts

```bash
# Build release binaries for local platform
cargo build --release --all-features

# Test the binary
./target/release/orbit --version

# Test orbit-web
cd crates/orbit-web
cargo leptos build --release
./target/release/orbit-web &
curl http://localhost:8080/api/health
pkill orbit-web
cd ../..
```

---

## Release Steps

### Step 1: Create Release Branch

```bash
# Create release branch from main
git checkout main
git pull origin main
git checkout -b release/v0.5.0
```

### Step 2: Update Version Numbers

```bash
# Update version in all Cargo.toml files
# Use sed or manually edit:

# Root package
sed -i 's/^version = "0.4.1"/version = "0.5.0"/' Cargo.toml

# Update workspace crates as needed
# ...

# Regenerate Cargo.lock
cargo update --workspace
```

### Step 3: Update CHANGELOG

```bash
# Edit CHANGELOG.md
nano CHANGELOG.md

# Add entry for new version at the top
# Include: Added, Changed, Fixed, Security sections
```

### Step 4: Commit Changes

```bash
# Add all version changes
git add Cargo.toml Cargo.lock CHANGELOG.md
git add crates/*/Cargo.toml

# Commit with conventional commit message
git commit -m "chore(release): prepare v0.5.0

- Updated version numbers across workspace
- Updated CHANGELOG.md
- Verified all tests pass
"
```

### Step 5: Run Final Tests

```bash
# Full test suite
cargo test --workspace --all-features

# Build release artifacts
cargo build --release --all-features
cargo build --release -p orbit-web

# Verify binaries work
./target/release/orbit --version
./target/release/orbit-web &
sleep 2
curl http://localhost:8080/api/health
pkill orbit-web
```

### Step 6: Create Pull Request

```bash
# Push release branch
git push origin release/v0.5.0

# Create PR on GitHub
gh pr create --title "Release v0.5.0" --body "Release v0.5.0

## Changes
See CHANGELOG.md for full details.

## Checklist
- [x] All tests pass
- [x] Version numbers updated
- [x] CHANGELOG updated
- [x] Documentation updated
- [x] Binaries build successfully
"
```

### Step 7: Merge and Tag

```bash
# After PR approval and merge to main
git checkout main
git pull origin main

# Create annotated tag
git tag -a v0.5.0 -m "Release v0.5.0

Major release adding Web GUI and enhanced features.

See CHANGELOG.md for full details.
"

# Push tag
git push origin v0.5.0
```

### Step 8: Build Release Binaries (automated)

Pushing a `v*` tag triggers [`.github/workflows/release.yml`](../../.github/workflows/release.yml), which builds and publishes:

| Asset | Built with | Notes |
|-------|------------|-------|
| `orbit-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` | `cargo-zigbuild` on `ubuntu-latest` | Static musl, no glibc dependency |
| `orbit-vX.Y.Z-aarch64-unknown-linux-musl.tar.gz` | `cargo-zigbuild` on `ubuntu-latest` | Static musl, ARM64 |
| `orbit-vX.Y.Z-universal2-apple-darwin.tar.gz` | `cargo-zigbuild` on `macos-latest` | Universal fat binary (Intel + Apple Silicon) |
| `orbit-vX.Y.Z-x86_64-pc-windows-msvc.zip` | `cargo build` on `windows-latest` | Native MSVC |
| `SHA256SUMS` | `sha256sum` over all of the above | Published alongside the archives |

All binaries are built with `--features s3-native,backend-abstraction`. If you need a manual local rebuild for any reason:

```bash
cargo build --release --target x86_64-unknown-linux-musl --features s3-native,backend-abstraction
# (etc. — see release.yml for the exact target list)
```

### Step 9: Create GitHub Release (automated)

The same workflow's `release` job creates the GitHub release with `softprops/action-gh-release@v2`, attaches all archives plus `SHA256SUMS`, and renders install instructions in the release body. The release is marked as a pre-release automatically if the tag name contains `alpha`, `beta`, or `rc`.

If the workflow fails or you need to publish manually, fall back to:

```bash
gh release create v0.5.0 \
  --title "Orbit v0.5.0" \
  --notes-file RELEASE_NOTES.md \
  releases/v0.5.0/orbit-v0.5.0-*.tar.gz \
  releases/v0.5.0/orbit-v0.5.0-*.zip
```

### Step 10: Publish Announcement

Create announcement post with:
- Link to GitHub release
- Highlights of major features
- Installation instructions
- Upgrade guide (if breaking changes)

---

## Post-Release Tasks

### 1. Verify Release

```bash
# Test installation from GitHub release (Linux x86_64 example)
curl -L https://github.com/saworbit/orbit/releases/download/v0.6.0/orbit-v0.6.0-x86_64-unknown-linux-musl.tar.gz | tar xz
./orbit --version

# Verify checksum
curl -L https://github.com/saworbit/orbit/releases/download/v0.6.0/SHA256SUMS \
  | grep orbit-v0.6.0-x86_64-unknown-linux-musl.tar.gz | sha256sum -c
```

### 2. Update Documentation

- [ ] Update installation instructions in README
- [ ] Update version references in documentation
- [ ] Update Docker images (if applicable)

### 3. Create Release Notes Blog Post

Draft blog post covering:
- Major features
- Breaking changes (if any)
- Upgrade instructions
- Deprecation notices
- Thank contributors

### 4. Update Project Board

- [ ] Close milestone for this release
- [ ] Create milestone for next release
- [ ] Move unreleased features to next milestone

### 5. Social Media Announcement

- Twitter/X announcement
- Reddit (r/rust)
- This Week in Rust submission
- Hacker News (for major releases)

---

## Hotfix Process

For critical bugs in released versions:

### 1. Create Hotfix Branch

```bash
# Branch from the release tag
git checkout -b hotfix/v0.5.1 v0.5.0
```

### 2. Apply Fix

```bash
# Make minimal changes to fix the issue
# Update version to 0.5.1
# Update CHANGELOG
git commit -m "fix: critical bug in transfer resume

Fixes #123"
```

### 3. Test Thoroughly

```bash
cargo test --workspace --all-features
cargo build --release
```

### 4. Merge to Main and Tag

```bash
# Create PR to main
git push origin hotfix/v0.5.1
gh pr create --title "Hotfix v0.5.1"

# After merge
git checkout main
git pull origin main
git tag -a v0.5.1 -m "Hotfix v0.5.1"
git push origin v0.5.1
```

### 5. Create Hotfix Release

```bash
# Build and publish same as regular release
# Mark as hotfix in release notes
```

---

## Automation

### GitHub Actions Release Workflow

The release pipeline lives at [`.github/workflows/release.yml`](../../.github/workflows/release.yml). It triggers on any tag matching `v*` and runs two jobs:

1. **`build`** — a matrix across Linux musl (x86_64, aarch64) via `cargo-zigbuild`, macOS universal2 via `cargo-zigbuild`, and Windows MSVC via plain `cargo build`. All builds enable `--features s3-native,backend-abstraction`. Each job uploads its archive as a workflow artifact.
2. **`release`** — downloads all artifacts, generates `SHA256SUMS`, and publishes a GitHub Release with platform-specific install instructions in the body. Tags containing `alpha`/`beta`/`rc` are marked as pre-releases automatically.

Why these target choices:

- **Static musl** for Linux means the same binary runs on Debian, RHEL, Alpine, and anything else with a modern kernel — no glibc version drama.
- **universal2** for macOS ships one archive that works on both Intel and Apple Silicon Macs.
- **`cargo-zigbuild`** lets us cross-compile from clean Linux/macOS runners without juggling per-target C toolchains; `zig cc` handles the `lz4-sys` / `zstd-sys` builds against musl out of the box.

To re-run a failed release without re-tagging, use the workflow's run page in the Actions tab and click **Re-run jobs**.

---

## Release Checklist Template

Copy this for each release:

```markdown
## Release vX.Y.Z Checklist

### Pre-Release
- [ ] All tests pass
- [ ] All features documented
- [ ] Version numbers updated in all Cargo.toml files
- [ ] CHANGELOG.md updated
- [ ] Breaking changes documented
- [ ] Migration guide created (if needed)
- [ ] Documentation reviewed and updated

### Release
- [ ] Release branch created
- [ ] Final tests passed
- [ ] PR created and reviewed
- [ ] PR merged to main
- [ ] Tag created and pushed
- [ ] Binaries built for all platforms
- [ ] GitHub release created with artifacts
- [ ] Release notes published

### Post-Release
- [ ] Release verified (binaries work)
- [ ] Documentation updated
- [ ] Next milestone created
- [ ] Announcement posted
- [ ] Docker images updated (if applicable)

### Communication
- [ ] GitHub release notes
- [ ] Twitter/X announcement
- [ ] Reddit r/rust post
- [ ] This Week in Rust submission (major releases)
```

---

## Versioning Quick Reference

| Change Type | Version Bump | Example |
|-------------|--------------|---------|
| Bug fix | PATCH (0.0.x) | 0.4.1 → 0.4.2 |
| New feature | MINOR (0.x.0) | 0.4.1 → 0.5.0 |
| Breaking change | MAJOR (x.0.0) | 0.5.0 → 1.0.0 |
| Hotfix | PATCH (0.0.x) | 0.5.0 → 0.5.1 |

---

## Contact

For questions about the release process:
- GitHub Issues: https://github.com/saworbit/orbit/issues
- Maintainer: Shane Wall (@saworbit)

---

**Last Updated:** 2026-05-17
**Current Version:** 0.6.0
**Release Workflow:** [`.github/workflows/release.yml`](../../.github/workflows/release.yml) (musl Linux + universal2 macOS + MSVC Windows via cargo-zigbuild)
