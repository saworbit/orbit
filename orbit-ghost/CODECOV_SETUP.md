# Codecov Setup Guide for Orbit GhostFS

Step-by-step guide to enable code coverage tracking with Codecov.

## What is Codecov?

Codecov is a code coverage reporting service that:
- Tracks test coverage over time
- Shows coverage on pull requests
- Identifies untested code
- Provides visual coverage reports
- Integrates with GitHub

## Prerequisites

- ✅ Codecov CLI installed locally (you've done this)
- GitHub repository with push access
- CI/CD enabled (GitHub Actions)

## Setup Steps

### 1. Sign Up for Codecov

**Option A: Using GitHub Account (Recommended)**

1. Go to [https://about.codecov.io/](https://about.codecov.io/)
2. Click **"Sign up with GitHub"**
3. Authorize Codecov to access your repositories
4. Select the `saworbit/orbit` repository

**Option B: Using Codecov CLI**

Already logged in via CLI? Skip to step 2.

### 2. Get Your Codecov Token

**Via Web Dashboard:**

1. Log in to [https://app.codecov.io/](https://app.codecov.io/)
2. Navigate to your repository: `saworbit/orbit`
3. Go to **Settings** → **General**
4. Copy the **Repository Upload Token**

**Via CLI:**

```bash
# In the orbit-ghost directory
codecov --help

# This will show your token in the output
```

### 3. Add Token to GitHub Secrets

**Required for private repositories** (optional for public):

1. Go to your GitHub repository: `https://github.com/saworbit/orbit`
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Name: `CODECOV_TOKEN`
5. Value: Paste the token from step 2
6. Click **Add secret**

**Note:** For public repositories, the token is optional but recommended.

### 4. Verify CI Configuration

The CI workflow [.github/workflows/ci.yml](.github/workflows/ci.yml) already includes Codecov:

```yaml
- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v4
  with:
    files: ./cobertura.xml
    fail_ci_if_error: false
    token: ${{ secrets.CODECOV_TOKEN }}  # Optional for public repos
```

**If using a public repository:**
- The `token` line is optional
- Codecov will work without it

**If using a private repository:**
- The `CODECOV_TOKEN` secret is required
- Add it as described in step 3

### 5. Trigger a CI Run

**Option A: Push a commit**

```bash
cd orbit-ghost
git add .
git commit -m "chore: enable codecov integration"
git push
```

**Option B: Re-run existing workflow**

1. Go to **Actions** tab in GitHub
2. Select the latest workflow run
3. Click **Re-run all jobs**

### 6. View Coverage Report

**After CI completes:**

1. Go to [https://app.codecov.io/](https://app.codecov.io/)
2. Navigate to `saworbit/orbit`
3. Browse the coverage report

**Coverage will show:**
- Overall coverage percentage
- File-by-file breakdown
- Line-by-line coverage visualization
- Coverage trends over time

### 7. Add Badges to README

**Update [README.md](README.md):**

```markdown
# Orbit GhostFS

[![CI](https://github.com/saworbit/orbit/actions/workflows/ci.yml/badge.svg)](https://github.com/saworbit/orbit/actions)
[![codecov](https://codecov.io/gh/saworbit/orbit/branch/main/graph/badge.svg)](https://codecov.io/gh/saworbit/orbit)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

Quantum Entanglement Filesystem...
```

**Get your specific badge:**
1. Go to Codecov dashboard → Settings → Badge
2. Copy the Markdown code
3. Paste into README.md

## Configuration

### codecov.yml

The configuration file [codecov.yml](codecov.yml) controls:

**Coverage Targets:**
```yaml
coverage:
  status:
    project:
      default:
        target: 70%      # Fail if below 70%
        threshold: 2%    # Allow 2% drop

    patch:
      default:
        target: 80%      # New code should be 80%+ covered
```

**Adjust targets** if needed:
- `target`: Minimum acceptable coverage
- `threshold`: How much coverage can drop before failing

**Common targets:**
- 50-60%: Minimal (not recommended)
- 70-80%: Good (current setting)
- 80-90%: Very good
- 90%+: Excellent (aspirational)

### Ignore Paths

**Already configured to ignore:**
```yaml
ignore:
  - "tests/"
  - "benches/"
  - "examples/"
  - "**/*.md"
```

**To ignore additional paths**, edit [codecov.yml](codecov.yml):
```yaml
ignore:
  - "src/legacy/"
  - "src/experimental/"
```

## Local Coverage Generation

**Generate coverage locally** (Linux only):

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate HTML report
cargo tarpaulin --out html

# Open in browser
firefox tarpaulin-report.html

# Or use Makefile
make coverage
```

**Upload to Codecov manually:**

```bash
# Generate XML report
cargo tarpaulin --out xml

# Upload (requires CODECOV_TOKEN environment variable)
codecov upload-process --file cobertura.xml
```

## Understanding Coverage Reports

### Dashboard Metrics

**Overall Coverage:**
- Percentage of code executed by tests
- Higher is better (aim for 70%+)

**Files:**
- Red: Low coverage (< 60%)
- Yellow: Medium coverage (60-80%)
- Green: Good coverage (> 80%)

**Diff Coverage:**
- Coverage of new code in PRs
- Should be higher than overall (80%+)

### Coverage Types

**Line Coverage:**
- Which lines were executed
- Most common metric

**Branch Coverage:**
- Which conditional branches taken
- More comprehensive than line coverage

**Function Coverage:**
- Which functions called
- Helps identify dead code

## Pull Request Integration

**On every PR, Codecov will:**

1. **Post a comment** with:
   - Coverage change (+ or -)
   - Diff coverage (coverage of new code)
   - File-by-file breakdown

2. **Status check**:
   - ✅ Pass: Coverage meets targets
   - ❌ Fail: Coverage below threshold

**Example PR comment:**
```
Coverage: 72.5% (+1.2%)
Diff Coverage: 85.7%

Files changed:
  src/entangler.rs: 75% → 80% (+5%)
  src/fs.rs: 70% → 68% (-2%)
```

## Troubleshooting

### "No coverage report found"

**Cause:** Coverage file not generated or uploaded

**Fix:**
```bash
# Ensure cargo-tarpaulin is installed
cargo install cargo-tarpaulin

# Check if cobertura.xml is generated
ls -l cobertura.xml

# Verify CI step is running
# Check GitHub Actions logs
```

### "Coverage upload failed"

**Cause:** Missing or invalid token (private repo)

**Fix:**
1. Verify `CODECOV_TOKEN` secret exists in GitHub
2. Regenerate token from Codecov dashboard
3. Update GitHub secret

### "Coverage decreased" on PR

**Cause:** New code not tested, or tests removed

**Fix:**
```bash
# Check which files need tests
cargo tarpaulin --out html
firefox tarpaulin-report.html

# Add tests for uncovered code
# See red lines in HTML report
```

### "Platform not supported" (Windows)

**Limitation:** `cargo-tarpaulin` only works on Linux

**Workarounds:**
1. **CI only:** Let GitHub Actions (Ubuntu) generate coverage
2. **WSL2:** Run in Windows Subsystem for Linux
3. **Docker:** Use Linux container

```bash
# WSL2
wsl
cd /mnt/c/orbit/orbit-ghost
cargo tarpaulin --out html
```

## Advanced Configuration

### Coverage Flags

**Track different test types separately:**

```yaml
# codecov.yml
flags:
  unit:
    paths:
      - src/
  integration:
    paths:
      - tests/
```

**Upload with flags:**
```bash
# Unit tests
cargo tarpaulin --lib --out xml
codecov upload-process --file cobertura.xml --flag unit

# Integration tests
cargo tarpaulin --test '*' --out xml
codecov upload-process --file cobertura.xml --flag integration
```

### Multiple Projects

**If orbit-ghost becomes a workspace member:**

```yaml
# codecov.yml
coverage:
  status:
    project:
      orbit-ghost:
        target: 70%
        paths:
          - orbit-ghost/
```

### Notification Settings

**Configure in Codecov web UI:**
- **Settings** → **Notifications**
- Email alerts for coverage drops
- Slack/Discord webhooks

## Best Practices

### ✅ Do

- Write tests for new features
- Aim for 80%+ coverage on new code
- Review coverage reports on PRs
- Fix failing coverage checks
- Update tests when refactoring

### ❌ Don't

- Write tests just to increase coverage
- Test trivial code (getters/setters)
- Ignore coverage drops without investigation
- Lower coverage targets to pass CI
- Disable coverage checks

## Integration with Development Workflow

### Before Committing

```bash
# Run tests
cargo test

# Check coverage locally (Linux)
cargo tarpaulin --out html

# Review untested code
firefox tarpaulin-report.html
```

### Before Creating PR

```bash
# Ensure coverage is good
make coverage

# Check if new code is tested
cargo tarpaulin --out html
```

### Reviewing PRs

**Check Codecov comment:**
1. Is diff coverage > 80%?
2. Did overall coverage increase?
3. Are critical paths tested?

**If coverage dropped:**
- Ask for tests to be added
- Or justify why tests aren't needed

## Resources

- **Codecov Docs:** [https://docs.codecov.com/](https://docs.codecov.com/)
- **cargo-tarpaulin:** [https://github.com/xd009642/tarpaulin](https://github.com/xd009642/tarpaulin)
- **GitHub Actions:** [https://docs.github.com/en/actions](https://docs.github.com/en/actions)
- **Rust Testing:** [https://doc.rust-lang.org/book/ch11-00-testing.html](https://doc.rust-lang.org/book/ch11-00-testing.html)

## Status Checklist

After completing setup, verify:

- [ ] Codecov account created and repository added
- [ ] `CODECOV_TOKEN` secret added (if private repo)
- [ ] CI workflow includes coverage upload
- [ ] First coverage report generated
- [ ] Badge added to README.md
- [ ] `codecov.yml` configured with appropriate targets
- [ ] Team understands coverage requirements

## Next Steps

1. **Monitor coverage trends** on dashboard
2. **Set coverage goals** for each milestone
3. **Add tests** for low-coverage files
4. **Review coverage** on every PR
5. **Celebrate** when coverage improves! 🎉

---

**Questions?** Check [Codecov Support](https://about.codecov.io/support/) or open a GitHub discussion.
