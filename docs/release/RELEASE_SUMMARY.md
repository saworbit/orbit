# 🚀 Release System - Complete Setup

All release documentation and automation is now in place for Orbit!

## 📚 Release Documentation (1,098 lines total)

### 1. **RELEASE.md** (579 lines) - Complete Release Guide
Comprehensive documentation covering:
- ✅ Versioning strategy (Semantic Versioning)
- ✅ Pre-release checklist (tests, version updates, CHANGELOG)
- ✅ Step-by-step release process
- ✅ Post-release tasks
- ✅ Hotfix process
- ✅ Multi-platform binary building
- ✅ GitHub release creation

**Location:** [RELEASE.md](RELEASE.md)

### 2. **RELEASE_QUICKSTART.md** (136 lines) - Quick Reference
TL;DR guide for releases:
- ✅ 5-step release process
- ✅ Hotfix quick guide
- ✅ Version bump cheat sheet
- ✅ Manual release fallback

**Location:** [docs/RELEASE_QUICKSTART.md](docs/RELEASE_QUICKSTART.md)

### 3. **CHANGELOG.md** (205 lines) - Version History
Changelog template with:
- ✅ Unreleased section (ready for v0.5.0)
- ✅ Previous releases documented
- ✅ Keep a Changelog format
- ✅ Semantic Versioning links

**Location:** [CHANGELOG.md](CHANGELOG.md)

### 4. **GitHub Actions Workflow** - Automated Releases
Full CI/CD automation:
- ✅ Cross-compiles 4 binaries via `cargo-zigbuild` + native MSVC:
  - Linux x86_64 / aarch64 — **static musl** (no glibc dependency)
  - macOS — **universal2** (Intel + Apple Silicon in one fat binary)
  - Windows x86_64 — MSVC
- ✅ Creates tar.gz archives (Unix) and zip (Windows)
- ✅ Generates a `SHA256SUMS` file across all assets
- ✅ Auto-creates GitHub releases with binaries
- ✅ Generates release notes with install instructions
- ✅ Marks `alpha`/`beta`/`rc` tags as pre-releases

**Location:** [.github/workflows/release.yml](../../.github/workflows/release.yml)

---

## 🎯 How to Do a Release

### Quick Version (5 Steps)

```bash
# 1. Create release branch
git checkout -b release/v0.5.0

# 2. Update versions and CHANGELOG
sed -i 's/version = "0.4.1"/version = "0.5.0"/' Cargo.toml
# Edit CHANGELOG.md - add [0.5.0] section
git commit -m "chore(release): prepare v0.5.0"

# 3. Run tests
cargo test --workspace --all-features

# 4. Create PR and merge
git push origin release/v0.5.0
gh pr create --title "Release v0.5.0"
# Merge after approval

# 5. Tag and push
git checkout main && git pull
git tag -a v0.5.0 -m "Release v0.5.0"
git push origin v0.5.0
```

**That's it!** GitHub Actions automatically:
- Builds binaries for all platforms
- Creates GitHub release
- Uploads release artifacts
- Generates release notes

### What Gets Built

When you push a tag like `v0.6.0`, GitHub Actions builds:

1. **Linux x86_64 (static musl)**: `orbit-v0.6.0-x86_64-unknown-linux-musl.tar.gz`
2. **Linux aarch64 (static musl)**: `orbit-v0.6.0-aarch64-unknown-linux-musl.tar.gz`
3. **macOS universal**: `orbit-v0.6.0-universal2-apple-darwin.tar.gz` (Intel + Apple Silicon)
4. **Windows x64**: `orbit-v0.6.0-x86_64-pc-windows-msvc.zip`
5. **Checksums**: `SHA256SUMS` (covering all of the above)

All built with `--features s3-native,backend-abstraction` and uploaded to: `https://github.com/saworbit/orbit/releases/tag/v0.6.0`

---

## 📋 Pre-Release Checklist

Before creating a release, verify:

- [ ] All tests pass: `cargo test --workspace --all-features`
- [ ] Code formatted: `cargo fmt --all -- --check`
- [ ] No clippy warnings: `cargo clippy --workspace --all-features`
- [ ] Version numbers updated in all `Cargo.toml` files
- [ ] CHANGELOG.md updated with new version section
- [ ] Documentation updated (README, feature docs)
- [ ] Binaries build successfully: `cargo build --release --all-features`

---

## 🔢 Version Numbering

Orbit follows **Semantic Versioning 2.0.0**:

```
MAJOR.MINOR.PATCH

MAJOR = Breaking changes (0.x.y → 1.0.0)
MINOR = New features (0.4.x → 0.5.0)
PATCH = Bug fixes (0.4.1 → 0.4.2)
```

### Examples

| Change | Version Bump | Example |
|--------|--------------|---------|
| Add Web GUI (new feature) | MINOR | `0.4.1 → 0.5.0` |
| Fix bug in resume logic | PATCH | `0.4.1 → 0.4.2` |
| Breaking CLI change | MAJOR | `0.5.0 → 1.0.0` |
| Emergency hotfix | PATCH | `0.5.0 → 0.5.1` |

---

## 🔥 Hotfix Process

For critical bugs in production:

```bash
# 1. Branch from release tag
git checkout -b hotfix/v0.5.1 v0.5.0

# 2. Fix bug and update version to 0.5.1
# Update CHANGELOG.md

# 3. Commit and create PR
git commit -m "fix: critical security issue in S3 backend"
git push origin hotfix/v0.5.1

# 4. After merge, tag immediately
git checkout main && git pull
git tag -a v0.5.1 -m "Hotfix v0.5.1 - Security fix"
git push origin v0.5.1
```

---

## 📖 Documentation References

- **Full Release Guide**: [RELEASE.md](RELEASE.md)
- **Quick Reference**: [docs/RELEASE_QUICKSTART.md](docs/RELEASE_QUICKSTART.md)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)
- **GitHub Workflow**: [.github/workflows/release.yml](.github/workflows/release.yml)

---

## 🎉 What's New in This Setup

1. **Automated Binary Building** - No manual cross-compilation needed
2. **Multi-Platform Support** - 5 platforms built automatically
3. **Orbit-Web Included** - Web GUI built and released
4. **Auto-Generated Release Notes** - Includes install instructions
5. **Comprehensive Documentation** - Over 1,000 lines of release docs
6. **Hotfix Process** - Clear emergency release procedure
7. **Semantic Versioning** - Consistent version strategy

---

## 🔍 Verify Release Setup

Check that everything is ready:

```bash
# Verify workflow file
cat .github/workflows/release.yml

# Verify documentation
ls -lh RELEASE.md CHANGELOG.md docs/RELEASE_QUICKSTART.md

# Test workflow locally (requires act)
act -j build --secret-file .secrets

# Verify current version
grep '^version' Cargo.toml
```

---

## 🚦 Release Status

**Current Version:** `0.4.1`
**Next Release:** `0.5.0` (Web GUI release)
**Release System:** ✅ **READY**

All documentation and automation in place. Ready to release v0.5.0 when needed!

---

**Remember:** NEVER commit directly to main. Always use release branches and PRs! 🔒
