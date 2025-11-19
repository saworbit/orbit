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

**Current Version:** `0.4.1`

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

### Step 8: Build Release Binaries

Build binaries for all platforms:

```bash
# Linux x86_64
cargo build --release --target x86_64-unknown-linux-gnu

# Linux ARM64
cargo build --release --target aarch64-unknown-linux-gnu

# macOS x86_64
cargo build --release --target x86_64-apple-darwin

# macOS ARM64 (Apple Silicon)
cargo build --release --target aarch64-apple-darwin

# Windows x86_64
cargo build --release --target x86_64-pc-windows-msvc

# Create archives
mkdir -p releases/v0.5.0

# Linux x86_64
tar czf releases/v0.5.0/orbit-v0.5.0-x86_64-unknown-linux-gnu.tar.gz \
  -C target/x86_64-unknown-linux-gnu/release orbit

# macOS ARM64
tar czf releases/v0.5.0/orbit-v0.5.0-aarch64-apple-darwin.tar.gz \
  -C target/aarch64-apple-darwin/release orbit

# Windows (zip)
cd target/x86_64-pc-windows-msvc/release
zip ../../../releases/v0.5.0/orbit-v0.5.0-x86_64-pc-windows-msvc.zip orbit.exe
cd ../../..
```

### Step 9: Create GitHub Release

```bash
# Using GitHub CLI
gh release create v0.5.0 \
  --title "Orbit v0.5.0 - Web GUI Release" \
  --notes-file RELEASE_NOTES.md \
  releases/v0.5.0/orbit-v0.5.0-*.tar.gz \
  releases/v0.5.0/orbit-v0.5.0-*.zip
```

Or manually on GitHub:
1. Go to https://github.com/saworbit/orbit/releases/new
2. Choose tag `v0.5.0`
3. Set title: `Orbit v0.5.0 - Web GUI Release`
4. Add release notes from CHANGELOG
5. Upload binary archives
6. Check "Set as the latest release"
7. Publish release

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
# Test installation from GitHub release
wget https://github.com/saworbit/orbit/releases/download/v0.5.0/orbit-v0.5.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf orbit-v0.5.0-x86_64-unknown-linux-gnu.tar.gz
./orbit --version
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

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    name: Build release binaries
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }} --all-features

      - name: Create archive (Unix)
        if: matrix.os != 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../orbit-${{ github.ref_name }}-${{ matrix.target }}.tar.gz orbit

      - name: Create archive (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          7z a ../../../orbit-${{ github.ref_name }}-${{ matrix.target }}.zip orbit.exe

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: orbit-${{ matrix.target }}
          path: orbit-*

  release:
    name: Create GitHub Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download artifacts
        uses: actions/download-artifact@v4

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            orbit-*/orbit-*
          draft: false
          prerelease: false
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

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

**Last Updated:** 2025-11-10
**Current Version:** 0.4.1
**Next Planned Release:** 0.5.0 (Web GUI)
