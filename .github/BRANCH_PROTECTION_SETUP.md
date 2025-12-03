# Branch Protection Setup Guide

This guide walks you through setting up branch protection rules for the `main` branch to enforce CI/CD quality checks.

## Prerequisites

- Repository admin access
- CI/CD pipeline configured (`.github/workflows/ci.yml`)
- At least one successful CI run to populate status checks

## Step-by-Step Instructions

### 1. Navigate to Branch Protection Settings

1. Go to your repository on GitHub
2. Click **Settings** (top menu)
3. Click **Branches** (left sidebar)
4. Find **Branch protection rules** section
5. Click **Add rule** (or edit existing rule for `main`)

### 2. Configure Protection Rule

#### Basic Settings
- **Branch name pattern**: `main`
- Check ✅ **Require a pull request before merging**
  - Check ✅ **Require approvals**: `1` (or as desired)
  - Check ✅ **Dismiss stale pull request approvals when new commits are pushed**

#### Status Checks (CRITICAL)
Check ✅ **Require status checks to pass before merging**

Then check ✅ **Require branches to be up to date before merging**

In the search box, add these required status checks:
1. `build-and-test` (Rust backend checks)
2. `dashboard-quality` (React dashboard checks)
3. `test-s3-enabled` (S3 feature validation)
4. `test-minimal` (Minimal build)
5. `test-full-features` (Full feature set)
6. `cross-platform` (Cross-platform matrix)

**Note**: These checks will only appear after they've run at least once. Push a commit to trigger the CI pipeline, then come back to add them.

#### Additional Protections (Recommended)
- Check ✅ **Require conversation resolution before merging**
- Check ✅ **Require linear history** (keeps git history clean)
- Check ✅ **Include administrators** (enforce rules for admins too)
- Check ✅ **Restrict who can push to matching branches** (optional)

### 3. Save Changes

Click **Create** (or **Save changes** if editing)

## Verification

### Test the Protection

1. Create a new branch:
   ```bash
   git checkout -b test/branch-protection
   ```

2. Make a change that breaks CI (e.g., remove a semicolon):
   ```bash
   echo "const x = 1" >> dashboard/src/test-fail.ts
   git add .
   git commit -m "test: intentional failure"
   git push origin test/branch-protection
   ```

3. Create a pull request

4. Verify:
   - ❌ Dashboard quality check should fail (TypeScript error)
   - ❌ Merge button should be blocked
   - ✅ Error message shows which checks failed

5. Fix the issue:
   ```bash
   git rm dashboard/src/test-fail.ts
   git commit -m "test: fix intentional failure"
   git push
   ```

6. Verify:
   - ✅ All checks should pass
   - ✅ Merge button should be enabled

## Status Check Matrix

| Check Name | Description | Failure Scenario | Fix |
|------------|-------------|------------------|-----|
| `build-and-test` | Rust compilation, tests, clippy, fmt, audit | Build error, test failure, linting issue, vulnerability | Fix Rust code, update dependencies |
| `dashboard-quality` | TypeScript, ESLint, Prettier, npm audit, tests | Type error, linting issue, format error, vulnerability | Run `npm run ci:check`, fix issues |
| `test-s3-enabled` | S3 feature compilation | S3 feature broken | Fix S3-related code |
| `test-minimal` | Minimal build (no default features) | Core functionality broken | Fix core code |
| `test-full-features` | Full feature set | Feature integration issue | Fix feature code |
| `cross-platform` | Ubuntu, macOS, Windows | Platform-specific bug | Fix platform-specific code |

## Troubleshooting

### "Status check not found"

**Problem**: Can't find `dashboard-quality` in the status check list.

**Solution**:
1. Verify CI workflow has run at least once
2. Check Actions tab for recent runs
3. Look at a recent commit's status checks
4. Refresh the branch protection page

### CI Always Failing

**Problem**: CI fails on every commit.

**Solution**:
1. Run checks locally first:
   ```bash
   # Backend
   cargo fmt --all --check
   cargo clippy --all
   cargo test
   cargo audit

   # Frontend
   cd dashboard
   npm run ci:check
   ```

2. Fix issues before pushing
3. Consider adding pre-commit hooks

### "Required reviews not met"

**Problem**: Can't merge even though CI passes.

**Solution**:
1. Get the required number of approvals
2. Or temporarily disable review requirement (not recommended)

## Dependabot Configuration

Dependabot is configured in `.github/dependabot.yml` to automatically:
- Update Rust dependencies weekly
- Update npm dependencies weekly
- Update GitHub Actions weekly

Dependabot PRs will automatically trigger CI checks. Review and merge them regularly to stay up-to-date with security patches.

## Best Practices

1. **Never skip CI checks** - They exist for a reason
2. **Run `npm run ci:check` before pushing** - Catch issues early
3. **Fix issues immediately** - Don't let broken code accumulate
4. **Keep dependencies updated** - Review Dependabot PRs weekly
5. **Monitor CI performance** - Slow CI means slow development

## Support

If you encounter issues:
1. Check CI logs in Actions tab
2. Run checks locally to reproduce
3. Review this guide
4. Open an issue if the problem persists

---

Last updated: 2025-12-03
Version: v2.2.0-rc.1
