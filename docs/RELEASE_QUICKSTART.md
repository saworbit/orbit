# Quick Release Guide

**TL;DR** for releasing a new version of Orbit.

## Prerequisites

```bash
# Ensure you have:
- GitHub CLI installed: gh --version
- All tests passing: cargo test --workspace --all-features
- Clean working directory: git status
```

## Release in 5 Steps

### 1. Prepare Release

```bash
# Create release branch
git checkout main && git pull
git checkout -b release/v0.5.0

# Update version numbers
sed -i 's/version = "0.4.1"/version = "0.5.0"/' Cargo.toml
# Manually update crates/*/Cargo.toml if needed

# Update CHANGELOG.md
# Add new [0.5.0] section with changes

# Commit changes
git add .
git commit -m "chore(release): prepare v0.5.0"
```

### 2. Final Checks

```bash
# Run full test suite
cargo test --workspace --all-features

# Build release binaries
cargo build --release --all-features

# Verify binaries work
./target/release/orbit --version
```

### 3. Create PR and Merge

```bash
# Push branch
git push origin release/v0.5.0

# Create PR
gh pr create --title "Release v0.5.0" --body "Release v0.5.0 - See CHANGELOG.md"

# After approval, merge
gh pr merge --squash
```

### 4. Tag Release

```bash
# Pull merged changes
git checkout main && git pull

# Create and push tag
git tag -a v0.5.0 -m "Release v0.5.0"
git push origin v0.5.0
```

### 5. GitHub Actions Does the Rest

The release workflow automatically:
- Builds binaries for all platforms
- Creates GitHub release
- Uploads release assets
- Generates release notes

## That's It! 🎉

The release is live at: `https://github.com/saworbit/orbit/releases/tag/v0.5.0`

---

## Hotfix Release (Emergency)

```bash
# Branch from tag
git checkout -b hotfix/v0.5.1 v0.5.0

# Fix the bug
# Update version to 0.5.1
# Update CHANGELOG

git commit -m "fix: critical bug"
git push origin hotfix/v0.5.1

# PR, merge, tag
git checkout main && git pull
git tag -a v0.5.1 -m "Hotfix v0.5.1"
git push origin v0.5.1
```

---

## Manual Release (Without GitHub Actions)

If you need to create a release manually:

```bash
# Build for all platforms
./scripts/build-release.sh  # Create this script

# Create release
gh release create v0.5.0 \
  --title "Orbit v0.5.0" \
  --notes-file RELEASE_NOTES.md \
  releases/v0.5.0/*.tar.gz \
  releases/v0.5.0/*.zip
```

---

## Version Bump Cheat Sheet

| Change | Version | Example |
|--------|---------|---------|
| Bug fix | `0.x.y → 0.x.(y+1)` | `0.4.1 → 0.4.2` |
| New feature | `0.x.y → 0.(x+1).0` | `0.4.1 → 0.5.0` |
| Breaking | `0.x.y → (x+1).0.0` | `0.5.0 → 1.0.0` |
| Hotfix | `0.x.y → 0.x.(y+1)` | `0.5.0 → 0.5.1` |

---

**Full Documentation:** See [RELEASE.md](../RELEASE.md)
