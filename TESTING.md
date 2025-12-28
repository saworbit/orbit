# Orbit Testing & Validation

Quick reference for Orbit's testing and validation scripts.

## 🆕 What's New in v3.0

**Lifecycle Scripts Enhanced:**
- ✨ **Data Mutation Testing** - Simulates real-world data drift (delete/add/modify operations)
- ✨ **Synchronization Verification** - Tests Orbit's sync capability to resolve divergence
- ✨ **4 Observation Points** - Extended verification workflow (was 3)
- ✨ **8 Test Files** - More comprehensive topology (was 4 files)
- ✨ **"Trust, but Verify"** - Complete lifecycle: Copy → Audit → Mutate → Sync

## Available Test Scripts

### Validation Scripts (Automated Testing)

**Purpose:** End-to-end validation with automated integrity checks

| Script | Platform | Duration | Use Case |
|--------|----------|----------|----------|
| [`validate_orbit.sh`](validate_orbit.sh) | Linux/macOS | ~2-5 min | CI/CD, automated regression testing |
| [`Validate-Orbit.ps1`](Validate-Orbit.ps1) | Windows | ~2-5 min | CI/CD, automated regression testing |

**What they test:**
- ✓ Binary compilation
- ✓ Disk space safety (500MB minimum)
- ✓ Copy operations with timing
- ✓ Sync/delta operations
- ✓ Automated integrity audits (`diff`)

**Usage:**
```bash
# Linux/macOS
./validate_orbit.sh

# Windows (PowerShell)
.\Validate-Orbit.ps1
```

### Lifecycle Demonstration Scripts (Interactive Verification) - v3.0

**Purpose:** Guided data lifecycle verification with cryptographic proofs and synchronization testing

| Script | Platform | Duration | Use Case |
|--------|----------|----------|----------|
| [`orbit_lifecycle_demo.sh`](orbit_lifecycle_demo.sh) | Linux/macOS | ~3-7 min | Stakeholder demos, detailed verification |
| [`Lifecycle-Demo.ps1`](Lifecycle-Demo.ps1) | Windows | ~3-7 min | Stakeholder demos, detailed verification |

**What they test (v3.0):**
- ✓ Complex nested directory preservation
- ✓ File type agnosticism (8 files: text + binary)
- ✓ Cryptographic integrity (SHA-256)
- ✓ **Data mutation (delete/add/modify)** - NEW
- ✓ **Synchronization with drift resolution** - NEW
- ✓ Human-in-the-loop verification (4 observation points)
- ✓ Complete data lifecycle (copy → audit → mutate → sync)

**Usage:**
```bash
# Linux/macOS
./orbit_lifecycle_demo.sh

# Windows (PowerShell)
.\Lifecycle-Demo.ps1
```

## Quick Start

### First Time Setup

**Linux/macOS:**
```bash
# Make scripts executable
chmod +x validate_orbit.sh orbit_lifecycle_demo.sh

# Run validation
./validate_orbit.sh

# Run lifecycle demo
./orbit_lifecycle_demo.sh
```

**Windows:**
```powershell
# May need to enable script execution
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# Run validation
.\Validate-Orbit.ps1

# Run lifecycle demo
.\Lifecycle-Demo.ps1
```

## Key Differences

| Aspect | Validation Scripts | Lifecycle Scripts v3.0 |
|--------|-------------------|-------------------|
| **Focus** | Throughput & operations | Provenance, integrity & sync |
| **Data** | Simple flat files | Nested directories (8 files) |
| **Operations** | Copy + Sync | Copy + Audit + Mutate + Sync |
| **Verification** | `diff` comparison | SHA-256 hashing + convergence check |
| **Observation Points** | 3 | 4 |
| **Interaction** | Automated | Guided tour |
| **Audience** | Developers/CI | Operators/Stakeholders |

## Prerequisites

- **Rust/Cargo** - For building Orbit binary
- **500MB disk space** minimum
- **Platform-specific tools:**
  - Linux/macOS: `bash`, `dd`, `shasum`, `diff`
  - Windows: PowerShell 5.0+, .NET Framework

## Documentation

For comprehensive information, see:

**[Complete Testing Scripts Guide →](docs/guides/TESTING_SCRIPTS_GUIDE.md)**

Includes:
- Detailed script architecture
- All observation points explained
- Troubleshooting guide
- CI/CD integration examples
- Performance benchmarks
- Security considerations

## Quick Troubleshooting

### "Insufficient disk space"
```bash
# Check available space
df -h .                    # Unix/macOS
Get-PSDrive C              # Windows

# Clean up to free space
cargo clean                # Frees 3-5GB
```

### "Compilation failed"
```bash
# Update Rust toolchain
rustup update stable

# Check logs
tail -100 orbit_validation_workspace/validation.log
```

### "Integrity audit failed"
```bash
# Review differences
diff -r source_dir dest_dir    # Unix/macOS

# Check Orbit logs
cat orbit_lifecycle_lab/mission_log.txt
```

## CI/CD Integration

```yaml
# GitHub Actions example
- name: Run Validation
  run: |
    chmod +x validate_orbit.sh
    yes "" | ./validate_orbit.sh  # Auto-continue
```

## Related Documentation

- [Quick Start Guide](docs/guides/quickstart_guide.md)
- [Demo Guide](docs/guides/DEMO_GUIDE.md)
- [Disk Space Guide](docs/guides/DISK_SPACE_GUIDE.md)
- [Performance Guide](docs/guides/PERFORMANCE.md)

---

**Orbit v2.2.0** - Proving data integrity through cryptographic verification 🔐
