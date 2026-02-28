# Orbit Scripts

This directory contains utility scripts for development, testing, and demonstrations.

## Demo Scripts

### Interactive Demos
- **`demo-orbit.sh`** / **`demo-orbit.bat`** - Interactive demonstration of Orbit features
- **`demo-orbit-record.sh`** / **`demo-orbit-record.bat`** - Record demonstration sessions
- **`orbit_lifecycle_demo.sh`** / **`Lifecycle-Demo.ps1`** - Complete lifecycle demonstration

### CI/Automated Demos
- **`demo-orbit-ci.sh`** / **`demo-orbit-ci.bat`** - Non-interactive demo for CI/CD pipelines

**Windows CI note:** `demo-orbit-ci.bat` is tuned for Windows runners. It uses `http://127.0.0.1:8080`, `curl.exe` with timeouts, skips the health probe, and retries login to avoid CI hangs. If a Windows job stalls, pull latest `main` and check `orbit-server.log` and `orbit-dashboard.log`.

## Launch Scripts

- **`launch-orbit.sh`** / **`launch-orbit.bat`** - Quick launch scripts for Orbit server and UI

## Validation Scripts

- **`validate_orbit.sh`** / **`Validate-Orbit.ps1`** - Validation and testing utilities

## Usage

### Unix/Linux/macOS
```bash
# Make scripts executable
chmod +x scripts/*.sh

# Run a demo
./scripts/demo-orbit.sh

# Launch Orbit
./scripts/launch-orbit.sh
```

### Windows
```powershell
# Run a demo
.\scripts\demo-orbit.bat

# Launch Orbit
.\scripts\launch-orbit.bat

# PowerShell scripts
.\scripts\Lifecycle-Demo.ps1
.\scripts\Validate-Orbit.ps1
```

## Script Descriptions

### Demo Scripts Details

**demo-orbit.sh/bat**: Interactive demonstration showcasing:
- File transfer capabilities
- Compression options
- Resume functionality
- Progress tracking

**demo-orbit-ci.sh/bat**: Automated demo suitable for:
- CI/CD pipelines
- Automated testing
- Non-interactive environments

**demo-orbit-record.sh/bat**: Recording-enabled demo for:
- Creating tutorials
- Documentation screenshots
- Video demonstrations

**orbit_lifecycle_demo.sh / Lifecycle-Demo.ps1**: Complete lifecycle showing:
- Installation
- Configuration via `orbit init`
- Various transfer scenarios
- Cleanup

### Launch Scripts Details

**launch-orbit.sh/bat**: Quick launcher that:
- Starts Orbit server (port 3000)
- Starts Orbit web UI
- Provides status monitoring

### Validation Scripts Details

**validate_orbit.sh / Validate-Orbit.ps1**: Validation suite that:
- Runs cargo tests
- Performs clippy checks
- Validates configuration
- Checks dependencies
