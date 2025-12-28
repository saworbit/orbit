# Orbit Testing & Validation Scripts Guide

## Overview

Orbit provides four comprehensive testing scripts designed to validate build integrity, data replication, and lifecycle management across all supported platforms. These scripts go beyond simple performance testing to prove **mathematical correctness** of data replication through cryptographic verification.

### Testing Philosophy

The Orbit testing suite follows a **"Data Lifecycle Verification"** methodology:

- **Not just throughput** - We test provenance and integrity across the lifespan of data
- **Human-in-the-loop** - Interactive observation points allow operators to verify side effects
- **Cryptographic guarantees** - SHA-256 hashing proves bit-level consistency
- **Automated cleanup** - No manual intervention required for teardown

## Available Scripts

| Script | Platform | Purpose | Duration |
|--------|----------|---------|----------|
| `validate_orbit.sh` | Linux/macOS | End-to-end validation with resource governance | ~2-5 min |
| `Validate-Orbit.ps1` | Windows | PowerShell-based validation suite | ~2-5 min |
| `orbit_lifecycle_demo.sh` | Linux/macOS | Guided data lifecycle demonstration | ~3-7 min |
| `Lifecycle-Demo.ps1` | Windows | Interactive lifecycle verification | ~3-7 min |

---

## Part 1: Validation Scripts

### Purpose

The validation scripts provide **automated end-to-end testing** of Orbit's core functionality:

- Binary compilation verification
- Storage safety checks (500MB minimum required)
- Copy operation testing with timing
- Sync/delta operation testing
- Automated integrity audits
- Workspace cleanup

### Architecture

```
Phase 1: Environment Analysis
  ├── Disk space verification (500MB minimum)
  ├── Rust toolchain detection
  └── Workspace initialization

Phase 2: Binary Compilation
  ├── Release mode build
  ├── Optimization for throughput
  └── Artifact verification

Phase 3: Synthetic Workload Generation
  ├── 20 small configuration files (High IOPS simulation)
  ├── 15MB binary blob (Throughput simulation)
  └── Dataset initialization

Phase 4: Replication Testing (Copy)
  ├── Source → Destination transfer
  ├── Performance timing
  └── Manual observation point

Phase 5: Differential Sync Verification
  ├── Simulate data drift (delete + add files)
  ├── Execute Orbit sync
  ├── Automated checksum audit
  └── Manual observation point

Phase 6: Infrastructure Teardown
  ├── Workspace cleanup
  └── Final status report
```

### Usage

#### Linux/macOS

```bash
cd /path/to/orbit
./validate_orbit.sh
```

**First-time setup:**
```bash
# Make script executable
chmod +x validate_orbit.sh
```

#### Windows (PowerShell)

```powershell
cd C:\path\to\orbit
.\Validate-Orbit.ps1
```

**Execution policy (if blocked):**
```powershell
# Check current policy
Get-ExecutionPolicy

# Enable script execution (one-time, run as Administrator)
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# Or bypass for single execution
powershell -ExecutionPolicy Bypass -File .\Validate-Orbit.ps1
```

### What Gets Validated

| Test | Validation Method | Success Criteria |
|------|-------------------|------------------|
| **Disk Space** | `df` (Unix) / PSDrive (Windows) | ≥500MB available |
| **Rust Toolchain** | `cargo --version` | Command exists in PATH |
| **Binary Build** | `cargo build --release` | Exit code 0, artifact exists |
| **Data Generation** | File creation + size check | 21 files, ~15MB total |
| **Copy Operation** | Orbit copy command | Exit code 0, timing recorded |
| **File Replication** | Directory comparison | File counts match |
| **Sync Operation** | Orbit sync command | Exit code 0, drift handled |
| **Data Integrity** | `diff -r` (Unix) / Compare-Object (Windows) | 100% consistency |

### Observation Points

The validation scripts include **two manual observation points**:

#### Observation 1: Source Data Verification
**When:** After data generation, before copy operation
**Action:** Open a new terminal and inspect the source directory
**What to verify:**
```bash
# Unix/macOS
ls -lh ./orbit_validation_workspace/source_data/
# Should see: 20 .dat files + 1 payload.bin (~15MB)

# Windows
dir .\orbit_validation_workspace\source_data\
```

#### Observation 2: Replication Integrity
**When:** After copy operation, before sync test
**Action:** Navigate to destination directory
**What to verify:**
```bash
# Ensure file counts match source
# Verify directory structure is preserved
# Check payload.bin exists and is ~15MB
```

#### Observation 3: Synchronization State
**When:** After sync operation, before cleanup
**Action:** Check destination for drift changes
**What to verify:**
- `shard_1.dat` should be **deleted** (was removed from source)
- `shard_new.dat` should **exist** (was added to source)

### Output Logs

All Orbit binary operations are logged to:
- **Unix/macOS:** `./orbit_validation_workspace/validation.log`
- **Windows:** `.\orbit_validation_workspace\validation.log`

**Log contents include:**
- Build stdout/stderr
- Copy operation output
- Sync operation output
- Error messages and stack traces

**Viewing logs:**
```bash
# Unix/macOS
tail -f ./orbit_validation_workspace/validation.log

# Windows
Get-Content .\orbit_validation_workspace\validation.log -Wait
```

### Configuration

Both scripts use the following constants (adjustable):

```bash
# Bash (validate_orbit.sh)
REQUIRED_SPACE_MB=500        # Minimum disk space
TEST_DIR="./orbit_validation_workspace"
BINARY_PATH="./target/release/orbit"
```

```powershell
# PowerShell (Validate-Orbit.ps1)
$RequiredSpaceMB = 500       # Minimum disk space
$WorkDir = "orbit_validation_workspace"
$BinaryPath = "target\release\orbit.exe"
```

**To adjust disk space requirement:**
```bash
# Edit the script and modify:
REQUIRED_SPACE_MB=1000  # Require 1GB instead of 500MB
```

---

## Part 2: Lifecycle Demonstration Scripts (v3.0)

### Purpose

The lifecycle scripts provide **guided, interactive demonstrations** that focus on:

- Complex directory topology preservation
- File type agnosticism (text and binary)
- **Data mutation and synchronization** (v3.0)
- Cryptographic integrity verification (SHA-256)
- Human-in-the-loop verification at each stage

### What Makes Lifecycle Scripts Different

| Aspect | Validation Scripts | Lifecycle Scripts v3.0 |
|--------|-------------------|-------------------|
| **Focus** | Throughput & basic operations | Provenance, integrity & sync |
| **Data Complexity** | Simple flat files | Nested directories with mixed types |
| **Operations Tested** | Copy + Sync | Copy + Hash Audit + Mutation + Sync |
| **Verification** | `diff` comparison | SHA-256 cryptographic audit |
| **Interaction** | Automated with observation points | Guided tour with explicit verification |
| **Observation Points** | 3 | 4 |
| **Logging** | `validation.log` | `mission.log` + hash manifests |
| **Audience** | Developers/CI | Operators/Stakeholders |

### Data Topology Generated (v3.0)

The lifecycle scripts create a **complex nested structure** to prove Orbit's topology preservation:

```
sector_alpha/
├── config.json                    (text file)
├── logs/
│   └── archive/
│       ├── log_1.txt              (5 log files)
│       ├── log_2.txt
│       ├── log_3.txt
│       ├── log_4.txt
│       └── log_5.txt
├── images/
│   └── raw/
│       └── texture_map.bin        (5MB binary blob)
└── db/
    └── shards/
        └── primary.db             (1MB binary database)
```

**Why this structure?**
- **Nested directories** - Proves Orbit doesn't flatten hierarchies
- **Multiple text files** - Verifies batch handling and encoding
- **Binary blobs** - Tests raw byte preservation
- **Mixed types** - Demonstrates type agnosticism
- **Database files** - Simulates real-world application data

### Usage

#### Linux/macOS

```bash
cd /path/to/orbit
./orbit_lifecycle_demo.sh
```

**First-time setup:**
```bash
chmod +x orbit_lifecycle_demo.sh
```

#### Windows (PowerShell)

```powershell
cd C:\path\to\orbit
.\Lifecycle-Demo.ps1
```

### Execution Flow (v3.0)

```
1. Initialization
   ├── Workspace creation
   ├── Disk space check (500MB minimum)
   └── Binary compilation (if needed)

2. Complex Data Generation
   ├── Create nested directory structure (logs/archive/, images/raw/, db/shards/)
   ├── Generate text files (config.json + 5 log files)
   ├── Generate binary files (5MB texture_map.bin, 1MB primary.db)
   └── >>> OBSERVATION POINT 1: Source Topology <<<

3. Replication (Copy Mode)
   ├── Execute Orbit copy operation
   ├── Record transfer duration
   ├── Verify exit code
   └── >>> OBSERVATION POINT 2: Replication Verification <<<

4. Cryptographic Integrity Audit
   ├── Generate SHA-256 hashes for source tree (src.sha)
   ├── Generate SHA-256 hashes for destination tree (dst.sha)
   ├── Compare hash manifests with diff
   └── Report integrity confirmation

5. Data Mutation Phase (NEW in v3.0)
   ├── Delete file (remove log_1.txt)
   ├── Add file (create new_file.dat)
   ├── Modify file (update config.json)
   └── >>> OBSERVATION POINT 3: Data Drift <<<

6. Synchronization (Sync Mode) (NEW in v3.0)
   ├── Execute Orbit sync operation
   ├── Verify exit code
   └── >>> OBSERVATION POINT 4: Convergence Verification <<<

7. Decommissioning Protocol
   ├── Remove workspace
   └── Final status report
```

**Key v3.0 Enhancement:** The lifecycle scripts now test the complete data lifecycle including mutation and synchronization, proving Orbit can handle real-world scenarios where source data changes after initial replication.

### Observation Points (Lifecycle v3.0)

#### Observation 1: Source Topology
**When:** After data generation, before replication
**Action:** Navigate to source directory and inspect structure
**What to verify:**

```bash
# Unix/macOS
tree orbit_lifecycle_lab/sector_alpha/
# or
find orbit_lifecycle_lab/sector_alpha/ -type f -ls

# Windows
tree /F orbit_lifecycle_lab\sector_alpha\
```

**Expected structure:**
- Root contains `config.json`
- `logs/archive/` contains 5 log files (`log_1.txt` through `log_5.txt`)
- `images/raw/` contains `texture_map.bin` (~5MB)
- `db/shards/` contains `primary.db` (~1MB)

#### Observation 2: Replication Verification
**When:** After copy operation, before integrity audit
**Action:** Navigate to destination and verify mirroring
**What to verify:**

```bash
# Compare directory trees
# Unix/macOS
diff -r orbit_lifecycle_lab/sector_alpha orbit_lifecycle_lab/sector_beta

# Windows
# Manually verify in File Explorer
```

**Expected result:**
- Directory structure is **identical** to source
- All 8 files exist in same relative paths
- File sizes match source files exactly

#### Observation 3: Data Drift (NEW in v3.0)
**When:** After mutation, before sync operation
**Action:** Compare source and destination to observe divergence
**What to verify:**

```bash
# Check source changes
ls -la orbit_lifecycle_lab/sector_alpha/logs/archive/
# Should show: log_1.txt is MISSING

ls -la orbit_lifecycle_lab/sector_alpha/
# Should show: new_file.dat is PRESENT

cat orbit_lifecycle_lab/sector_alpha/config.json
# Should show: "Modified Config"

# Check destination (unchanged)
ls -la orbit_lifecycle_lab/sector_beta/logs/archive/
# Should show: log_1.txt still EXISTS

ls -la orbit_lifecycle_lab/sector_beta/
# Should show: new_file.dat does NOT exist
```

**Expected state:**
- **Source:** `log_1.txt` deleted, `new_file.dat` added, `config.json` modified
- **Destination:** Still in original state (OUT OF SYNC)

#### Observation 4: Convergence Verification (NEW in v3.0)
**When:** After sync operation, before cleanup
**Action:** Verify destination now mirrors mutated source
**What to verify:**

```bash
# Unix/macOS
ls -la orbit_lifecycle_lab/sector_beta/logs/archive/
# Should show: log_1.txt is NOW DELETED

ls -la orbit_lifecycle_lab/sector_beta/
# Should show: new_file.dat is NOW PRESENT

cat orbit_lifecycle_lab/sector_beta/config.json
# Should show: "Modified Config"

# Verify full sync
diff -r orbit_lifecycle_lab/sector_alpha orbit_lifecycle_lab/sector_beta
# Should return: no differences
```

**Expected result:**
- Destination **mirrors** the mutated source exactly
- Deletions propagated: `log_1.txt` removed
- Additions propagated: `new_file.dat` present
- Modifications propagated: `config.json` updated
- **Full convergence achieved**

### Cryptographic Verification Details

#### Linux/macOS Implementation

```bash
# Generate hash manifests with relative paths
(cd "$SRC_DIR" && find . -type f -exec shasum -a 256 {} \;) | sort > src.sha
(cd "$DST_DIR" && find . -type f -exec shasum -a 256 {} \;) | sort > dst.sha

# Compare manifests
if diff src.sha dst.sha; then
    echo "INTEGRITY CONFIRMED: Bit-perfect replication"
else
    echo "INTEGRITY FAILURE: Hash mismatch detected"
fi
```

**Hash manifest files:**
- `orbit_lifecycle_lab/src.sha` - Source directory checksums
- `orbit_lifecycle_lab/dst.sha` - Destination directory checksums

#### Windows Implementation

```powershell
# Generate hash objects with relative paths
function Get-TreeHash($Root) {
    Get-ChildItem $Root -Recurse -File | ForEach-Object {
        @{
            Path = $_.FullName.Substring($Root.Length)
            Hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash
        }
    } | Sort-Object Path
}

$SrcHashes = Get-TreeHash $SrcDir
$DstHashes = Get-TreeHash $DstDir

# Compare hash collections
if (Compare-Object $SrcHashes $DstHashes -Property Path, Hash) {
    throw "Integrity Mismatch Detected!"
}
Write-Host "AUDIT PASSED: 100% Data Consistency"
```

**Why SHA-256?**
- Industry-standard cryptographic hash function
- Collision probability: ~2^-256 (effectively impossible)
- Proves **mathematical certainty** of data fidelity
- Detects single-bit corruption

**v3.0 Note:** The integrity audit occurs BEFORE mutation. After sync, the destination should match the mutated source (verified by final `diff` check at Observation 4).

---

## Prerequisites

### All Scripts

| Requirement | Validation Scripts | Lifecycle Scripts | Check Command |
|-------------|-------------------|-------------------|---------------|
| **Rust/Cargo** | ✓ | ✓ | `cargo --version` |
| **Disk Space** | 500MB minimum | 500MB minimum | `df -h .` / `Get-PSDrive` |
| **Write Permissions** | Current directory | Current directory | `touch test && rm test` |

### Platform-Specific

#### Linux/macOS

| Tool | Purpose | Check Command |
|------|---------|---------------|
| `bash` | Script execution | `bash --version` |
| `df` | Disk space check | `df --version` |
| `dd` | Binary file generation | `dd --version` |
| `shasum` | Hash calculation | `shasum --version` |
| `find` | File traversal | `find --version` |
| `diff` | Directory comparison | `diff --version` |

#### Windows

| Tool | Purpose | Check Command |
|------|---------|---------------|
| PowerShell 5.0+ | Script execution | `$PSVersionTable.PSVersion` |
| .NET Framework | Hash calculation | `[System.Security.Cryptography.SHA256]::Create()` |
| `Get-FileHash` | Integrity verification | `Get-Command Get-FileHash` |

---

## Disk Space Requirements

### Validation Scripts

| Component | Size | Notes |
|-----------|------|-------|
| Workspace directory | ~20MB | Temporary, auto-deleted |
| Source data | ~15MB | 20 files + 1 binary blob |
| Destination data | ~15MB | Copy of source |
| Rust build artifacts | 2-5GB | Only if not pre-built |
| Binary executable | ~15-30MB | `target/release/orbit` |
| **Total (pre-built)** | **~50MB** | Assumes binary exists |
| **Total (full build)** | **~5GB** | Includes compilation |

### Lifecycle Scripts (v3.0)

| Component | Size | Notes |
|-----------|------|-------|
| Workspace directory | ~6MB | Temporary, auto-deleted |
| Source data | ~6MB | 8 files: 5 logs + config + 2 binaries (5MB + 1MB) |
| Destination data (after copy) | ~6MB | Copy of source |
| Destination data (after mutation) | ~6MB | Synced with mutated source |
| Hash manifests | ~1KB | src.sha + dst.sha (SHA-256 checksums) |
| Mission log | ~10KB | mission.log (operation telemetry) |
| **Total (pre-built)** | **~12MB** | Assumes binary exists |
| **Total (full build)** | **~5GB** | Includes compilation |

### Space-Saving Tips

```bash
# Pre-build Orbit binary once
cargo build --release

# Run validation scripts (no rebuild needed)
./validate_orbit.sh

# Run lifecycle scripts (no rebuild needed)
./orbit_lifecycle_demo.sh

# Clean up Rust artifacts after testing
cargo clean  # Frees ~3-5GB
```

---

## Troubleshooting

### Insufficient Disk Space

**Symptom:**
```
[ERROR] Insufficient disk space. Available: 300MB. Required: 500MB.
[ERROR] Aborting validation to preserve system stability.
```

**Solutions:**

```bash
# 1. Check actual available space
df -h .                          # Unix/macOS
Get-PSDrive C | Format-Table     # Windows

# 2. Clean up Rust build cache
cargo clean                      # Frees 3-5GB

# 3. Remove old Orbit workspaces
rm -rf orbit_*_workspace         # Unix
Remove-Item orbit_*_workspace -Recurse  # Windows

# 4. Adjust required space threshold (temporary workaround)
# Edit script and change:
REQUIRED_SPACE_MB=250            # Reduce to 250MB
```

### Compilation Failures

**Symptom:**
```
[ERROR] Compilation failed. Review 'validation.log' for compiler stderr.
```

**Solutions:**

```bash
# 1. Check Rust version (minimum 1.70 required)
rustc --version
cargo --version

# 2. Update Rust toolchain
rustup update stable

# 3. Verify workspace is clean
cargo clean
cargo build --release

# 4. Check compilation log
tail -100 orbit_validation_workspace/validation.log  # Unix
Get-Content validation.log -Tail 100                 # Windows

# 5. Check for missing dependencies
cargo check
```

### Binary Artifact Missing

**Symptom:**
```
[ERROR] Binary artifact missing at ./target/release/orbit
```

**Solutions:**

```bash
# 1. Verify target directory exists
ls -la target/release/           # Unix/macOS
dir target\release\              # Windows

# 2. Check binary name (platform-specific)
# Unix/macOS: orbit
# Windows: orbit.exe

# 3. Manual build with verbose output
cargo build --release --verbose

# 4. Check if binary is in different location
find . -name orbit -type f       # Unix/macOS
Get-ChildItem -Recurse -Filter orbit.exe  # Windows
```

### File Generation Fails

**Symptom (Unix):**
```
dd: failed to open '/dev/urandom': Permission denied
```

**Solutions:**

```bash
# 1. Use /dev/zero as fallback
dd if=/dev/zero of=file.bin bs=1M count=5

# 2. Use head -c instead of dd
head -c 5M /dev/urandom > file.bin

# 3. Check permissions
ls -la /dev/urandom
# Should show: crw-rw-rw-
```

**Symptom (Windows):**
```
Access is denied. (fsutil requires Administrator privileges)
```

**Solutions:**

```powershell
# 1. Run PowerShell as Administrator, OR

# 2. Use .NET instead of fsutil (automatic fallback in latest scripts)
$buffer = New-Object byte[] (5MB)
[System.Random]::new().NextBytes($buffer)
[System.IO.File]::WriteAllBytes("file.bin", $buffer)
```

### Hash Calculation Fails

**Symptom (Unix):**
```
shasum: command not found
```

**Solutions:**

```bash
# 1. Install coreutils (contains shasum)
# Debian/Ubuntu
sudo apt install coreutils

# macOS (shasum is built-in, but if missing)
brew install coreutils

# 2. Use sha256sum as alternative
sha256sum file.txt

# 3. Use openssl
openssl sha256 file.txt
```

**Symptom (Windows):**
```
Get-FileHash : The term 'Get-FileHash' is not recognized
```

**Solutions:**

```powershell
# 1. Check PowerShell version (requires 4.0+)
$PSVersionTable.PSVersion

# 2. Update PowerShell
# Download from: https://docs.microsoft.com/en-us/powershell/

# 3. Use .NET directly
[BitConverter]::ToString(
    [System.Security.Cryptography.SHA256]::Create().ComputeHash(
        [System.IO.File]::ReadAllBytes("file.txt")
    )
) -replace '-', ''
```

### Integrity Audit Fails

**Symptom:**
```
[ERROR] AUDIT FAILED: Data corruption or mismatch detected.
```

**Investigation steps:**

```bash
# 1. Review hash manifests manually
diff orbit_lifecycle_lab/src_hashes.txt orbit_lifecycle_lab/dst_hashes.txt

# 2. Check for file count mismatch
find sector_alpha -type f | wc -l
find sector_beta -type f | wc -l

# 3. Identify specific differences
diff -r sector_alpha sector_beta

# 4. Check Orbit operation logs
tail -50 orbit_lifecycle_lab/mission_log.txt

# 5. Verify source data wasn't modified during transfer
ls -lt sector_alpha  # Check timestamps
```

### Cleanup Issues

**Symptom:**
```
[WARN] Workspace not found; manual cleanup may be required.
```

**Solutions:**

```bash
# 1. Verify workspace was created
ls -la orbit_*_workspace         # Unix
dir orbit_*_workspace            # Windows

# 2. Manual cleanup if needed
rm -rf orbit_validation_workspace orbit_lifecycle_lab  # Unix
Remove-Item orbit_*_workspace, orbit_lifecycle_lab -Recurse -Force  # Windows

# 3. Check for orphaned processes
ps aux | grep orbit              # Unix
Get-Process | Where-Object {$_.ProcessName -like "*orbit*"}  # Windows
```

### Script Hangs at Observation Point

**Symptom:**
Script pauses indefinitely, waiting for user input.

**Expected behavior:**
This is **intentional** - the scripts pause at observation points to allow manual verification.

**To proceed:**
Press **ENTER** (Unix/macOS) or **any key** (Windows) after verifying the requested information.

**To skip observation points (automated testing):**
```bash
# Unix/macOS - Pipe yes to auto-continue
yes "" | ./validate_orbit.sh

# Or comment out pause_for_observation / observe calls in script
```

---

## Best Practices

### 1. Pre-Build Orbit Binary

```bash
# Build once before running multiple test scripts
cargo build --release

# Verify binary exists
ls -lh target/release/orbit          # Unix/macOS
dir target\release\orbit.exe         # Windows

# Now run validation/lifecycle scripts (no rebuild)
```

**Why:** Saves 2-5 minutes per script run, avoids repeated compilation.

### 2. Don't Skip Observation Points

The observation points are **not optional fluff** - they verify:
- File creation side effects
- Directory structure preservation
- Synchronization logic (deletions, additions)

**How to observe:**
1. Open a **second terminal** (don't background the script)
2. Navigate to the workspace directory
3. Use `ls`, `tree`, or File Explorer to inspect
4. Verify the checklist provided by the script
5. Return to script terminal and press ENTER

### 3. Review Logs After Failures

```bash
# Validation scripts
tail -100 orbit_validation_workspace/validation.log

# Lifecycle scripts
tail -100 orbit_lifecycle_lab/mission_log.txt

# Look for:
# - Rust compilation errors
# - Orbit runtime panics
# - I/O errors (permissions, disk full)
# - Exit codes (non-zero = failure)
```

### 4. Clean Up After Testing

```bash
# Remove workspaces (should auto-cleanup, but verify)
rm -rf orbit_validation_workspace orbit_lifecycle_lab  # Unix
Remove-Item orbit_*_workspace, orbit_lifecycle_lab -Recurse  # Windows

# Optional: Clean Rust build cache
cargo clean  # Frees 3-5GB
```

### 5. Run Both Script Types

```bash
# 1. Validation scripts (automated testing)
./validate_orbit.sh
# or
.\Validate-Orbit.ps1

# 2. Lifecycle scripts (guided verification)
./orbit_lifecycle_demo.sh
# or
.\Lifecycle-Demo.ps1
```

**Why both?**
- **Validation scripts** - Fast automated regression testing
- **Lifecycle scripts** - Detailed cryptographic verification for stakeholder demos

### 6. Adjust for CI/CD

```bash
# Disable interactive pauses for automated pipelines
# Option 1: Pipe yes to auto-continue
yes "" | ./validate_orbit.sh

# Option 2: Comment out observation points
# Edit script and comment:
# observe "Source Data Created" "..."

# Option 3: Create CI-specific variant
cp validate_orbit.sh validate_orbit_ci.sh
# Remove all observe() calls
```

---

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Orbit Validation

on: [push, pull_request]

jobs:
  validate:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Run Validation Script (Unix)
        if: runner.os != 'Windows'
        run: |
          chmod +x validate_orbit.sh
          yes "" | ./validate_orbit.sh

      - name: Run Validation Script (Windows)
        if: runner.os == 'Windows'
        run: |
          # Auto-continue by piping input
          echo "" | .\Validate-Orbit.ps1

      - name: Upload Logs
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: validation-logs-${{ matrix.os }}
          path: |
            orbit_validation_workspace/validation.log
            orbit_lifecycle_lab/mission_log.txt
```

### GitLab CI Example

```yaml
stages:
  - validate

validate:unix:
  stage: validate
  image: rust:latest
  script:
    - chmod +x validate_orbit.sh
    - yes "" | ./validate_orbit.sh
  artifacts:
    when: on_failure
    paths:
      - orbit_validation_workspace/validation.log

validate:lifecycle:
  stage: validate
  image: rust:latest
  script:
    - chmod +x orbit_lifecycle_demo.sh
    - yes "" | ./orbit_lifecycle_demo.sh
  artifacts:
    when: on_failure
    paths:
      - orbit_lifecycle_lab/mission_log.txt
      - orbit_lifecycle_lab/src_hashes.txt
      - orbit_lifecycle_lab/dst_hashes.txt
```

---

## Security Considerations

### Temporary File Creation

Both script types create temporary workspaces in the current directory:

```
./orbit_validation_workspace/    # Validation scripts
./orbit_lifecycle_lab/            # Lifecycle scripts
```

**Security implications:**
- Files are created with **current user permissions**
- Workspaces are **automatically deleted** on exit (via `trap` or `finally`)
- Binary blobs contain **random data** (`/dev/urandom` or `Random.NextBytes`)

**To verify cleanup:**
```bash
# After script completion, check for orphaned workspaces
ls -la | grep orbit_

# Should return empty (both workspaces deleted)
```

### Log Files

Logs may contain:
- Absolute file paths
- System configuration details
- Rust compilation environment variables

**Recommendations:**
- Review logs before sharing: `cat validation.log`
- Redact sensitive paths if needed
- Delete logs after debugging: `rm validation.log mission_log.txt`

### Script Execution Permissions

**Unix/macOS:**
```bash
# Scripts require execute permission
chmod +x validate_orbit.sh
chmod +x orbit_lifecycle_demo.sh

# Verify permissions
ls -l *.sh
# Should show: -rwxr-xr-x (executable)
```

**Windows:**
```powershell
# PowerShell execution policy may block scripts
Get-ExecutionPolicy

# If Restricted, enable for current user
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# Or bypass for single execution
powershell -ExecutionPolicy Bypass -File .\Validate-Orbit.ps1
```

---

## Performance Benchmarks

### Typical Execution Times

| Script | Platform | Duration | Notes |
|--------|----------|----------|-------|
| `validate_orbit.sh` | Linux (Ubuntu 22.04) | 2m 15s | Intel i7, SSD |
| `validate_orbit.sh` | macOS (M1) | 1m 45s | Apple Silicon, fast compilation |
| `Validate-Orbit.ps1` | Windows 11 | 2m 30s | Intel i5, NVMe SSD |
| `orbit_lifecycle_demo.sh` | Linux (Ubuntu 22.04) | 3m 10s | Includes hash calculation |
| `Lifecycle-Demo.ps1` | Windows 11 | 3m 20s | .NET hash calculation |

**Breakdown (validation scripts):**
- Environment checks: ~5s
- Cargo build (release): ~60-120s (varies by CPU cores)
- Data generation: ~10s
- Copy operation: ~5s
- Sync operation: ~5s
- Integrity audit: ~2s
- Cleanup: ~1s

**Breakdown (lifecycle scripts):**
- Environment checks: ~5s
- Cargo build (if needed): ~60-120s
- Data generation: ~15s (larger binary blobs)
- Copy operation: ~3s
- Hash calculation: ~10s (SHA-256 for all files)
- Cleanup: ~1s

### Optimization Tips

```bash
# 1. Pre-build binary to skip compilation
cargo build --release
# Saves: 60-120s per script run

# 2. Use faster compression (validation only)
# Edit Orbit source to default to LZ4 instead of Zstd
# Saves: ~2-3s on copy operations

# 3. Reduce data volume (testing only, not recommended for validation)
# Edit scripts:
# dd bs=1M count=5  # Instead of count=15
# Saves: ~5-10s on file generation

# 4. Run on SSD instead of HDD
# Saves: ~20-40% total time

# 5. Use ramdisk for temporary workspace
mkdir -p /dev/shm/orbit_test
cd /dev/shm/orbit_test
# Saves: ~30-50% on I/O operations
```

---

## Script Output Reference

### Validation Script Output

```
============================================================
   Phase 1: Environment Analysis
============================================================
[INFO] Performing pre-flight storage allocation check...
[SUCCESS] Storage Check Passed: 15234MB available.
[SUCCESS] Rust toolchain detected.

============================================================
   Phase 2: Binary Compilation
============================================================
[INFO] Compiling Orbit (Release Mode) for optimal throughput...
[INFO] This may take time depending on CPU cores...
[SUCCESS] Compilation complete.

============================================================
   Phase 3: Synthetic Workload Generation
============================================================
[INFO] Allocating small configuration files (High IOPS simulation)...
[INFO] Allocating large binary blob (Throughput simulation)...
[SUCCESS] Dataset initialized.

>>> OBSERVATION POINT: Source Data Created
>>> Action: Open a new terminal. List files in 'orbit_validation_workspace/source_data'. Verify 'payload.bin' is ~15MB.
>>> Press [ENTER] when ready to proceed...

============================================================
   Phase 4: Replication Testing (Copy)
============================================================
[INFO] Initiating transfer: Source -> Destination...
[SUCCESS] Transfer complete in 4523ms.

>>> OBSERVATION POINT: Replication Integrity
>>> Action: Navigate to 'orbit_validation_workspace/destination_data'. Ensure file counts match Source. Verify directory structure.
>>> Press [ENTER] when ready to proceed...

============================================================
   Phase 5: Differential Sync Verification
============================================================
[INFO] Mutating source state (simulating drift)...
[INFO] Executing Orbit Sync...
[SUCCESS] Sync logic executed.
[INFO] Performing automated checksum audit...
[SUCCESS] Audit Passed: Destination mirrors Source exactly.

>>> OBSERVATION POINT: Synchronization State
>>> Action: Check 'orbit_validation_workspace/destination_data'. Confirm 'shard_1.dat' is deleted and 'shard_new.dat' exists.
>>> Press [ENTER] when ready to proceed...

[SUCCESS] ALL SYSTEMS OPERATIONAL.

============================================================
   Phase 6: Infrastructure Teardown
============================================================
[INFO] Releasing allocated storage...
[SUCCESS] Workspace 'orbit_validation_workspace' successfully decommissioned.

Orbit Validation Sequence Complete.
```

### Lifecycle Script Output (v3.0)

```
Initializing Orbit Lifecycle Protocol...
[ORBIT] Analyzing host resources...
[SUCCESS] Storage confirmed.
[ORBIT] Generating Source Data Topology...
[ORBIT] Synthesizing binary payloads (Entropy simulation)...
[SUCCESS] Data Generation Complete.

>>> OBSERVATION POINT: Source Topology
Action required: Open a new terminal. Navigate to 'orbit_lifecycle_lab/sector_alpha'.
   Observe the nested folder structure and file types.
--> Press [ENTER] when ready to proceed...

[ORBIT] Engaging Replication Engine (Mode: COPY)...
[SUCCESS] Replication concluded in 2834ms.

>>> OBSERVATION POINT: Replication Verification
Action required: Check 'orbit_lifecycle_lab/sector_beta'.
   Verify that the folder structure mirrors the Source exactly.
--> Press [ENTER] when ready to proceed...

[ORBIT] Calculating cryptographic signatures (SHA256)...
[SUCCESS] INTEGRITY CONFIRMED: Bit-perfect replication.

[ORBIT] Simulating Data Drift (Mutation Phase)...

>>> OBSERVATION POINT: Data Drift
Action required: Check 'orbit_lifecycle_lab/sector_alpha'.
   Notice 'log_1.txt' is gone, 'new_file.dat' exists, and 'config.json' changed.
   'orbit_lifecycle_lab/sector_beta' is now OUT OF SYNC.
--> Press [ENTER] when ready to proceed...

[ORBIT] Engaging Synchronization Engine (Mode: SYNC)...
[SUCCESS] Sync Operation Complete.

>>> OBSERVATION POINT: Convergence Verification
Action required: Check 'orbit_lifecycle_lab/sector_beta'.
   'log_1.txt' should be DELETED.
   'new_file.dat' should be PRESENT.
   The simulated drift should be resolved.
--> Press [ENTER] when ready to proceed...

[ORBIT] Demo Complete. Preparing for auto-decommissioning.
[ORBIT] Decommissioning Simulation Environment...
[SUCCESS] Workspace decommissioned.

Lifecycle Protocol Complete.
```

---

## FAQ

### Q: Can I run multiple scripts simultaneously?

**A:** Not recommended. Both script types:
- Compile Orbit binary (resource-intensive)
- Create workspaces in current directory
- May conflict if run in same directory

**Workaround:**
```bash
# Run in separate directories
cd /tmp/test1 && /path/to/orbit/validate_orbit.sh &
cd /tmp/test2 && /path/to/orbit/orbit_lifecycle_demo.sh &
```

### Q: Do I need to run all 4 scripts?

**A:** No. Recommendations:

| Scenario | Recommended Scripts |
|----------|---------------------|
| **Quick validation** | `validate_orbit.sh` OR `Validate-Orbit.ps1` |
| **Stakeholder demo** | `orbit_lifecycle_demo.sh` OR `Lifecycle-Demo.ps1` |
| **Full verification** | Run validation + lifecycle (both platforms if cross-platform) |
| **CI/CD** | Validation scripts only (faster, automated) |

### Q: Can I customize the test data?

**A:** Yes. Edit the scripts to adjust:

```bash
# Validation scripts - Change file sizes
dd if=/dev/urandom of="payload.bin" bs=1M count=50  # 50MB instead of 15MB

# Lifecycle scripts - Add more files
echo "New data" > "$SRC_DIR/additional_file.txt"
mkdir -p "$SRC_DIR/another_directory"
```

**Note:** Larger datasets require more disk space (adjust `REQUIRED_SPACE_MB`).

### Q: What if the binary is already built?

**A:** Scripts detect existing binaries and skip compilation:

```bash
# Both scripts check:
if [ ! -f "$BINARY_PATH" ]; then
    cargo build --release
fi

# If binary exists, compilation is skipped (saves 60-120s)
```

### Q: How do I verify data integrity manually?

**A:** Use the same methods as the scripts:

```bash
# Unix/macOS - Compare directories
diff -r source_dir dest_dir

# Unix/macOS - Compare hashes
(cd source && find . -type f -exec shasum -a 256 {} \;) | sort > src.txt
(cd dest && find . -type f -exec shasum -a 256 {} \;) | sort > dst.txt
diff src.txt dst.txt

# Windows - Compare hashes
Get-ChildItem source -Recurse | Get-FileHash | Sort-Object Path > src.txt
Get-ChildItem dest -Recurse | Get-FileHash | Sort-Object Path > dst.txt
Compare-Object (Get-Content src.txt) (Get-Content dst.txt)
```

### Q: Can I use these scripts for production validation?

**A:** **Not recommended** without modifications:

**Issues:**
- Hardcoded paths (workspace in current directory)
- Interactive pauses (not suitable for automation)
- No artifact retention (auto-cleanup)

**For production:**
1. Fork the scripts
2. Remove observation pauses
3. Add artifact retention (`--no-cleanup` flag)
4. Parameterize paths and data sizes
5. Add exit code checks for CI integration

Example:
```bash
#!/bin/bash
# Production validation script (non-interactive)
set -e
CLEANUP=true

# Parse flags
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --no-cleanup) CLEANUP=false ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
    shift
done

# ... (rest of script) ...

# Conditional cleanup
if [ "$CLEANUP" = true ]; then
    rm -rf "$WORKSPACE"
fi
```

---

## Related Documentation

- **Quick Start Guide** - [`quickstart_guide.md`](quickstart_guide.md)
- **Demo Orchestration** - [`DEMO_GUIDE.md`](DEMO_GUIDE.md)
- **Disk Space Management** - [`DISK_SPACE_GUIDE.md`](DISK_SPACE_GUIDE.md)
- **Performance Tuning** - [`PERFORMANCE.md`](PERFORMANCE.md)
- **Production Deployment** - [`PRODUCTION_DEPLOYMENT.md`](PRODUCTION_DEPLOYMENT.md)

---

## Contributing

When modifying or extending the testing scripts:

1. **Test on all platforms** (Linux, macOS, Windows)
2. **Maintain backward compatibility** (Orbit v2.0+)
3. **Update this guide** for any new features
4. **Follow scripting best practices**:
   - Use `set -e` (bash) or `$ErrorActionPreference = "Stop"` (PowerShell)
   - Implement cleanup traps/finally blocks
   - Validate prerequisites before execution
   - Provide clear error messages
5. **Run shellcheck** (bash scripts) or **PSScriptAnalyzer** (PowerShell)

```bash
# Validate bash scripts
shellcheck validate_orbit.sh orbit_lifecycle_demo.sh

# Validate PowerShell scripts
Invoke-ScriptAnalyzer -Path Validate-Orbit.ps1
Invoke-ScriptAnalyzer -Path Lifecycle-Demo.ps1
```

---

## License

These testing scripts are part of the Orbit project and licensed under Apache 2.0.

---

**Orbit v2.2.0** - Proving data integrity through cryptographic verification 🔐
