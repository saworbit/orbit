# Contributing to Orbit GhostFS

Thank you for your interest in contributing to Orbit GhostFS! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Submitting Changes](#submitting-changes)
- [Review Process](#review-process)
- [Community](#community)

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inspiring community for all.

### Our Standards

**Positive behavior:**
- Using welcoming and inclusive language
- Being respectful of differing viewpoints
- Accepting constructive criticism gracefully
- Focusing on what is best for the community
- Showing empathy towards others

**Unacceptable behavior:**
- Harassment, trolling, or derogatory comments
- Publishing others' private information
- Spam or excessive self-promotion
- Any conduct inappropriate in a professional setting

### Enforcement

Violations can be reported to shaneawall@gmail.com. All complaints will be reviewed and investigated promptly and fairly.

## How Can I Contribute?

### Reporting Bugs

Before creating a bug report:
1. Check the [FAQ](FAQ.md) for common issues
2. Search [existing issues](https://github.com/saworbit/orbit/issues) to avoid duplicates
3. Try the latest version from `main` branch

**Submit a bug report:**
1. Go to [Issues](https://github.com/saworbit/orbit/issues/new)
2. Use the "Bug Report" template
3. Provide detailed information:
   - Operating system and version
   - Rust version (`rustc --version`)
   - FUSE version (`fusermount --version`)
   - Steps to reproduce
   - Expected behavior
   - Actual behavior
   - Relevant logs (`RUST_LOG=debug`)

**Example:**
```markdown
### Bug Description
Application hangs when reading files over slow network.

### Environment
- OS: Ubuntu 22.04 LTS
- Rust: 1.75.0
- FUSE: 3.10.5

### Steps to Reproduce
1. Mount GhostFS: `./orbit-ghost`
2. Throttle network: `tc qdisc add dev eth0 root netem delay 2000ms`
3. Read file: `cat /tmp/orbit_ghost_mount/file.dat`
4. Observe: Application hangs indefinitely

### Expected Behavior
Should timeout after 30 seconds with error.

### Actual Behavior
Hangs forever, must be killed.

### Logs
[Attach logs with RUST_LOG=debug]
```

### Suggesting Enhancements

**Before submitting:**
1. Check the [Roadmap](ROADMAP.md) - feature may be planned
2. Search [existing feature requests](https://github.com/saworbit/orbit/issues?q=is%3Aissue+label%3Aenhancement)

**Submit an enhancement:**
1. Go to [Issues](https://github.com/saworbit/orbit/issues/new)
2. Use the "Feature Request" template
3. Provide:
   - Use case (what problem does it solve?)
   - Proposed solution (how should it work?)
   - Alternatives considered
   - Willingness to implement

**Example:**
```markdown
### Feature Request
Add automatic cache eviction based on LRU policy.

### Use Case
Currently cache grows indefinitely, filling disk. Users need manual cleanup.

### Proposed Solution
- Add `cache_limit` config option (e.g., 50 GB)
- When limit exceeded, delete least recently used blocks
- Track access timestamps in metadata file

### Alternatives
1. Manual cleanup script (current workaround)
2. TTL-based eviction (less optimal than LRU)

### Implementation
I can implement this if design is approved.
```

### Contributing Code

#### Good First Issues

Look for issues labeled [`good first issue`](https://github.com/saworbit/orbit/labels/good%20first%20issue):
- Small, well-defined scope
- Detailed implementation guidance
- Good learning opportunity

**Current good first issues:**
- Add `--version` flag
- Improve error messages
- Add unit tests for block calculation
- Documentation typos/improvements

#### Areas Needing Help

**High priority:**
- [ ] Windows support (WinFSP implementation)
- [ ] Timeout handling
- [ ] Thread pool for downloads
- [ ] LRU cache eviction

**Medium priority:**
- [ ] Prefetching algorithms
- [ ] Configuration file parsing
- [ ] CLI argument handling
- [ ] Integration tests

**Low priority:**
- [ ] ML-based access prediction
- [ ] Performance benchmarks
- [ ] Example applications
- [ ] Video tutorials

### Contributing Documentation

Documentation is as important as code!

**Ways to help:**
- Fix typos or grammar
- Improve clarity of explanations
- Add missing examples
- Translate to other languages (future)
- Create video tutorials
- Write blog posts about usage

**Process:**
1. Edit Markdown files directly
2. Submit pull request
3. Preview changes locally: `cargo doc --open`

## Development Setup

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install FUSE (Ubuntu)
sudo apt-get install fuse3 libfuse3-dev pkg-config

# Install FUSE (macOS)
brew install --cask macfuse
```

### Fork and Clone

```bash
# Fork repository on GitHub (click "Fork" button)

# Clone your fork
git clone https://github.com/YOUR_USERNAME/orbit.git
cd orbit/orbit-ghost

# Add upstream remote
git remote add upstream https://github.com/saworbit/orbit.git

# Verify
git remote -v
```

### Create a Branch

```bash
# Update main
git checkout main
git pull upstream main

# Create feature branch
git checkout -b feature/your-feature-name

# Or for bug fixes
git checkout -b fix/bug-description
```

**Branch naming conventions:**
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `refactor/` - Code refactoring
- `test/` - Adding tests
- `perf/` - Performance improvements

### Make Changes

```bash
# Build and test
cargo build
cargo test

# Run clippy (linter)
cargo clippy -- -D warnings

# Format code
cargo fmt

# Run demo
./demo_quantum.sh
```

### Commit Your Changes

**Commit message format:**
```
type(scope): short description

Longer explanation if needed.

Fixes #123
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting (no code change)
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance tasks

**Examples:**
```bash
# Good commits
git commit -m "feat(entangler): add timeout to ensure_block_available"
git commit -m "fix(fs): correct block boundary calculation"
git commit -m "docs(readme): add installation instructions for Fedora"

# Bad commits (avoid)
git commit -m "fixed stuff"
git commit -m "WIP"
git commit -m "updated code"
```

### Push Changes

```bash
# Push to your fork
git push origin feature/your-feature-name
```

## Coding Standards

### Rust Style

**Follow the [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/):**

```rust
// Good
pub fn ensure_block_available(&self, file_id: &str, block_index: u64) {
    let req = BlockRequest {
        file_id: file_id.to_string(),
        block_index,
    };
    // ...
}

// Bad
pub fn EnsureBlockAvailable(&self, FileID: &str, BlockIndex: u64) {
    let Req = BlockRequest{file_id: FileID.to_string(), block_index: BlockIndex};
    // ...
}
```

### Code Quality

**Required before submission:**

```bash
# No compiler warnings
cargo build 2>&1 | grep warning
# Should be empty

# Clippy passes
cargo clippy -- -D warnings

# Formatted
cargo fmt --check

# Tests pass
cargo test
```

### Error Handling

**Use `Result` for recoverable errors:**

```rust
// Good
fn fetch_block(&self, id: &str) -> Result<Vec<u8>, Error> {
    let data = download(id)?;
    Ok(data)
}

// Bad
fn fetch_block(&self, id: &str) -> Vec<u8> {
    let data = download(id).unwrap();  // Can panic!
    data
}
```

### Documentation

**Document public APIs:**

```rust
/// Ensures a block is available in the local cache.
///
/// This method blocks until the requested block has been downloaded.
///
/// # Arguments
///
/// * `file_id` - Unique file identifier
/// * `block_index` - Zero-indexed block number
///
/// # Returns
///
/// Returns `Ok(())` if block is available, or `Err` on timeout.
///
/// # Examples
///
/// ```
/// entangler.ensure_block_available("file_123", 42)?;
/// ```
pub fn ensure_block_available(&self, file_id: &str, block_index: u64) -> Result<(), Error> {
    // Implementation
}
```

### Testing

**Add tests for new features:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_range_calculation() {
        let offset = 1024;
        let size = 512;
        let block_size = 1024;

        let start = offset / block_size;
        let end = (offset + size) / block_size;

        assert_eq!(start, 1);
        assert_eq!(end, 1);
    }

    #[test]
    fn test_block_spanning_boundary() {
        let offset = 1024;
        let size = 1536;
        let block_size = 1024;

        let start = offset / block_size;
        let end = (offset + size) / block_size;

        assert_eq!(start, 1);
        assert_eq!(end, 2);
    }
}
```

## Submitting Changes

### Pull Request Process

1. **Update documentation** if needed
2. **Add tests** for new functionality
3. **Update CHANGELOG.md** under "Unreleased"
4. **Push to your fork**
5. **Create pull request** on GitHub

### Pull Request Template

```markdown
### Description
Brief description of changes.

### Motivation
Why is this change needed? What problem does it solve?

### Changes
- Added X
- Modified Y
- Removed Z

### Testing
How was this tested?
- [ ] Unit tests added
- [ ] Integration tests pass
- [ ] Manual testing performed

### Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] Tests added/updated
- [ ] CHANGELOG.md updated
- [ ] No new warnings

### Related Issues
Fixes #123
Closes #456
```

### Review Process

**What happens next:**

1. **Automated checks** run (CI):
   - Build verification
   - Test execution
   - Clippy lints
   - Format check

2. **Maintainer review**:
   - Code quality
   - Design approach
   - Test coverage
   - Documentation

3. **Feedback**:
   - Requested changes
   - Suggestions
   - Approval

4. **Merge**:
   - Squash commits (usually)
   - Update CHANGELOG
   - Close related issues

**Review timeline:**
- **Simple PRs** (docs, typos): 1-3 days
- **Small features**: 3-7 days
- **Large features**: 1-2 weeks

**Tips for faster review:**
- Keep PRs focused (one feature/fix per PR)
- Add clear description and examples
- Respond promptly to feedback
- Rebase on latest `main` if needed

## Community

### Communication Channels

- **GitHub Issues:** Bug reports, feature requests
- **GitHub Discussions:** General questions, ideas
- **Email:** shaneawall@gmail.com (maintainer)

### Getting Help

**Stuck on implementation?**
1. Comment on the issue you're working on
2. Ask in [Discussions](https://github.com/saworbit/orbit/discussions)
3. Email the maintainer

**Don't be shy!** We're here to help.

### Recognition

**Contributors are recognized:**
- In CHANGELOG.md
- In GitHub contributors list
- In release notes

**Top contributors may be invited to:**
- Maintainer team
- Architecture discussions
- Early feature access

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (Apache 2.0).

---

## Quick Start Checklist

Ready to contribute? Follow this checklist:

- [ ] Read Code of Conduct
- [ ] Fork repository
- [ ] Clone and set up development environment
- [ ] Find an issue to work on (or create one)
- [ ] Create feature branch
- [ ] Make changes
- [ ] Add tests
- [ ] Run `cargo clippy` and `cargo fmt`
- [ ] Update documentation
- [ ] Update CHANGELOG.md
- [ ] Commit with clear message
- [ ] Push to your fork
- [ ] Create pull request
- [ ] Respond to review feedback

---

**Thank you for contributing to Orbit GhostFS! 🚀**

Together, we're building the future of on-demand remote data access.
