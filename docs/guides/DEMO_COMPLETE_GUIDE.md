# 🛰️ Orbit E2E Demo - Complete Guide

> **Comprehensive documentation for the Orbit v2.2.0 E2E Demonstration Harness**

## Table of Contents

- [Overview](#overview)
- [Demo Variants](#demo-variants)
- [Quick Start Matrix](#quick-start-matrix)
- [Environment Setup](#environment-setup)
- [Usage Scenarios](#usage-scenarios)
- [Metrics & Analytics](#metrics--analytics)
- [Video Recording](#video-recording)
- [Docker Deployment](#docker-deployment)
- [CI/CD Integration](#cicd-integration)
- [Troubleshooting](#troubleshooting)
- [Advanced Topics](#advanced-topics)

---

## Overview

The Orbit E2E Demonstration Harness is a sophisticated orchestration system that automates the deployment, validation, and demonstration of the Orbit v2.2.0 stack. It implements "**The Deep Space Telemetry Scenario**" - a realistic workflow showcasing Orbit's file transfer capabilities through a simulated telescope data ingestion pipeline.

### What Gets Demonstrated

1. **Magnetar State Machine** - Persistent job lifecycle management
2. **Real-Time Dashboard** - Visual Chunk Map and live telemetry
3. **REST API** - Programmatic job control and monitoring
4. **Resilient Transfer** - Compression, verification, parallel processing
5. **Production Readiness** - Full-stack deployment and observability

### Architecture

```
┌──────────────────────────────────────────────────────────┐
│                 Demo Orchestration Layer                  │
├──────────────────────────────────────────────────────────┤
│                                                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │   Data      │  │   System    │  │    Job      │      │
│  │ Fabrication │→ │  Ignition   │→ │  Injection  │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
│         ↓                 ↓                 ↓             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │ Validation  │  │ Observation │  │  Cleanup    │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
│                                                            │
├──────────────────────────────────────────────────────────┤
│                     Orbit v2.2.0 Stack                    │
├──────────────────────────────────────────────────────────┤
│  ┌───────────────────┐         ┌────────────────────┐    │
│  │  React Dashboard  │◄──────► │  Control Plane API │    │
│  │  (Port 5173)      │  HTTP   │  (Port 8080)       │    │
│  └───────────────────┘         └────────┬───────────┘    │
│                                          │                │
│                                 ┌────────▼────────┐       │
│                                 │ Magnetar (SQLite)│       │
│                                 └─────────────────┘       │
└──────────────────────────────────────────────────────────┘
```

---

## Demo Variants

The harness provides multiple execution modes for different use cases:

| Script | Platform | Purpose | User Interaction | Output |
|--------|----------|---------|------------------|--------|
| `demo-orbit.sh` | Unix/Linux/macOS | Interactive demo | Required (2 pauses) | Terminal + Browser |
| `demo-orbit.bat` | Windows | Interactive demo | Required (2 pauses) | Terminal + Browser |
| `demo-orbit-ci.sh` | Unix/Linux/macOS | Automated testing | None (headless) | Terminal + Metrics JSON |
| `demo-orbit-ci.bat` | Windows | Automated testing | None (headless) | Terminal + Metrics JSON |
| `demo-orbit-record.sh` | Unix/Linux/macOS | Video capture | Required (2 pauses) | Terminal + Browser + Video |
| `demo-orbit-record.bat` | Windows | Video capture | Required (2 pauses) | Terminal + Browser + Video |
| Docker Compose | All (via Docker) | Containerized | Configurable | Logs + Metrics |

### Windows Batch Input Handling

**Important Technical Note:**

The Windows batch scripts (`*.bat`) use PowerShell `Read-Host` for user input instead of native batch commands (`pause`, `timeout`, `choice`). This is necessary because:

1. **stdin Conflicts**: Native batch input commands fail when background processes (`cargo.exe`, `node.exe`) are running via `start /B`
2. **Input Corruption**: Background processes intercept keyboard input, causing commands to break or hang
3. **Ctrl+C Issues**: Abort signals don't propagate properly through shared stdin

**Implementation:**

```batch
# User input (PowerShell):
powershell -Command "$null = Read-Host 'Press ENTER to continue'"

# Background process stdin isolation:
start /B "Orbit-Server" cmd /c "cargo run < nul > orbit-server.log 2>&1"
```

**Key Features:**
- ✓ Reliable input handling with background processes
- ✓ Proper Ctrl+C abort support
- ✓ UTF-8 encoding via `chcp 65001`
- ✓ Clear visual status indicators
- ✓ Graceful cleanup on exit or abort

See [Troubleshooting → Windows-Specific Issues](#windows-specific-issues) for more details.

---

## Quick Start Matrix

### 🛡️ Recommended First Step: Safety Validation

**Before running the demo**, validate your system is ready **without making any changes**:

```bash
# Unix/Linux/macOS or Git Bash (Windows)
./scripts/validate-demo-safety.sh

# What it checks (read-only, no changes made):
# ✓ System requirements (OS, architecture, disk space)
# ✓ Required commands (cargo, npm, curl)
# ✓ Port availability (8080, 5173)
# ✓ Existing processes
# ✓ Shows exactly what the demo will do

# Complete safety documentation:
# See SAFETY_FIRST.md for all safety assurances
```

### For First-Time Users

```bash
# Step 1: Validate (recommended)
./scripts/validate-demo-safety.sh

# Step 2: Run the demo
# Unix/Linux/macOS
./demo-orbit.sh

# Windows
demo-orbit.bat
```

### For Sales Demonstrations

```bash
# With video recording (requires ffmpeg)
./demo-orbit-record.sh  # Unix/Linux/macOS
demo-orbit-record.bat   # Windows
```

### For CI/CD Pipelines

```bash
# Headless automated testing
./demo-orbit-ci.sh      # Unix/Linux/macOS
demo-orbit-ci.bat       # Windows

# Analyze results
python scripts/analyze-metrics.py e2e-metrics.json
```

### For Docker Users

```bash
# Full stack with demo
docker-compose -f docker-compose.demo.yml --profile demo up

# Services only (no auto-demo)
docker-compose -f docker-compose.demo.yml up
```

---

## Environment Setup

### Prerequisites

#### Disk Space Requirements

**TL;DR:** Need at least **4GB free** for a full demo with build. Just **400MB** if using pre-built binaries.

| Scenario | Minimum Free Space | Recommended | What's Included |
|----------|-------------------|-------------|-----------------|
| 🚀 **Quick Demo** (binaries exist) | **400 MB** | 1 GB | Demo data only |
| 🔨 **First-Time Build** | **4 GB** | 6 GB | Rust + Node.js compilation |
| 🎬 **With Video Recording** | **5 GB** | 8 GB | + Video files (~500MB/5min) |
| 🐳 **Docker Deployment** | **6 GB** | 10 GB | + Container images & layers |
| 🧪 **CI/CD** (smaller test data) | **1 GB** | 2 GB | Reduced data volume for speed |

<details>
<summary><b>📊 Detailed Breakdown</b></summary>

**Demo Runtime Files:**
- Source data (synthetic telemetry): **170 MB**
  - `telemetry_alpha.bin`: 50 MB
  - `telemetry_beta.bin`: 20 MB
  - `telemetry_gamma.bin`: 100 MB
  - Flight logs (20 files): ~500 KB
  - Manifest JSON: ~1 KB
- Destination data (transferred files): **170 MB**
- Database (`magnetar.db`): **5-10 MB**
- Logs (`orbit-server.log`, `orbit-dashboard.log`): **10-50 MB**

**Build Artifacts** (one-time, reusable):
- Rust compilation cache (`target/`): **2-5 GB**
  - Debug build: ~3 GB
  - Release build: ~2 GB
  - Incremental compilation artifacts
- Node.js dependencies (`dashboard/node_modules/`): **400-600 MB**
  - React, Vite, TanStack Query, etc.
- Compiled binary (`orbit-server`): **15-30 MB**
  - Varies by features enabled
  - Release mode is smaller

**Optional Components:**
- **Video Recording** (if using `demo-orbit-record.sh/bat`):
  - 1080p30 H.264: ~100 MB per minute
  - 5-minute demo: ~500 MB
  - Thumbnail: ~100 KB
- **Docker Images** (if using Docker):
  - Base images (Rust, Node, Debian): **1-1.5 GB**
  - Built application image: **200-500 MB**
  - Volume data: Same as runtime files above

</details>

**💡 Space-Saving Strategies:**

```bash
# Strategy 1: Use pre-built binaries (saves ~3GB)
cd crates/orbit-web
cargo build --release --bin orbit-server  # Build once
# Then use: target/release/orbit-server directly

# Strategy 2: Clean build artifacts after compilation (saves ~3GB)
cargo clean  # Run after successful build

# Strategy 3: Use smaller test data in CI (saves ~140MB)
# Edit demo-orbit-ci.sh to use 10MB files instead of 170MB

# Strategy 4: Docker multi-stage builds (saves ~2GB)
# Final image is only ~200MB vs ~2GB+ full build image

# Strategy 5: Reduce video quality (saves ~300MB per recording)
# Edit ffmpeg command to use 720p instead of 1080p:
ffmpeg ... -s 1280x720 -crf 28 ...  # vs -s 1920x1080 -crf 18
```

**⚠️ Common Pitfalls:**

- **"No space left on device" during `cargo build`**: Rust compilation is disk-intensive. Free up at least 4GB before building.
- **"ENOSPC: no space left" during `npm install`**: Node modules can be large. Ensure 1GB free for dashboard dependencies.
- **Video recording fills disk**: Monitor free space during recording. Stop recording if <1GB free.

**✅ Quick Space Check:**

```bash
# Unix/Linux/macOS
df -h .  # Check free space in current directory

# Windows (PowerShell)
Get-PSDrive C | Select-Object Used,Free

# Recommended: At least 6GB free before starting
```

</details>

#### Core Requirements (All Variants)

- **Rust/Cargo** 1.75+
  ```bash
  # Ubuntu/Debian
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

  # macOS
  brew install rust

  # Windows
  # Download from https://rustup.rs/
  ```

- **Node.js/NPM** 20+
  ```bash
  # Ubuntu/Debian
  curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
  sudo apt-get install -y nodejs

  # macOS
  brew install node

  # Windows
  # Download from https://nodejs.org/
  ```

- **curl**
  ```bash
  # Ubuntu/Debian
  sudo apt-get install curl

  # macOS (pre-installed)

  # Windows
  # Available in Windows 10+ or install via choco install curl
  ```

#### Additional Tools (Optional)

##### For Metrics Analysis
- **Python** 3.8+
- **jq** (JSON processor)
  ```bash
  # Ubuntu/Debian
  sudo apt-get install jq

  # macOS
  brew install jq

  # Windows
  choco install jq
  ```

##### For Video Recording
- **ffmpeg**
  ```bash
  # Ubuntu/Debian
  sudo apt-get install ffmpeg

  # macOS
  brew install ffmpeg

  # Windows
  choco install ffmpeg
  ```

##### For Docker Deployment
- **Docker Engine** 20.10+
- **docker-compose** 2.0+

### Port Requirements

| Port | Service | Configurable? |
|------|---------|---------------|
| 8080 | Control Plane API | Yes (via env) |
| 5173 | React Dashboard | Yes (via env) |

Ensure these ports are not in use before running demos.

---

## Usage Scenarios

### Scenario 1: Sales Demo for Stakeholders

**Goal:** Showcase Orbit's capabilities to potential customers or investors.

**Recommended Variant:** `demo-orbit-record.sh/bat` (with video recording)

**Steps:**

1. **Preparation** (5 minutes before meeting)
   ```bash
   # Pre-build to save time
   cd crates/orbit-web && cargo build --release --bin orbit-server
   cd ../../dashboard && npm ci
   ```

2. **Recording** (during meeting or pre-recorded)
   ```bash
   ./demo-orbit-record.sh
   # Follow prompts, narrate features as demo runs
   ```

3. **Output:** Video file in `demo-recordings/` directory
   - Share with attendees post-meeting
   - Use in marketing materials
   - Include in documentation

**Pro Tips:**
- Rehearse the demo beforehand
- Prepare talking points for each phase
- Have the README open in another tab for reference
- Consider a second monitor for presenter notes

### Scenario 2: Developer Onboarding

**Goal:** Familiarize new team members with the architecture.

**Recommended Variant:** `demo-orbit.sh/bat` (interactive)

**Steps:**

1. **Walkthrough** (hands-on)
   ```bash
   ./demo-orbit.sh
   # Pause at each phase to explain components
   ```

2. **Exploration**
   - Have developer examine generated files in `/tmp/orbit_demo_source_*`
   - Review API calls in `orbit-server.log`
   - Inspect dashboard source code while demo runs
   - Monitor WebSocket traffic in browser DevTools

3. **Follow-up**
   ```bash
   # Show them how to modify demo
   vim demo-orbit.sh  # Change data size, job config, etc.
   ```

**Learning Objectives:**
- ✅ Understand Magnetar job lifecycle
- ✅ Learn REST API endpoints
- ✅ Explore React dashboard structure
- ✅ See real-time WebSocket updates

### Scenario 3: Automated Testing in GitHub Actions

**Goal:** Catch integration regressions in CI pipeline.

**Recommended Variant:** `demo-orbit-ci.sh` (headless)

**Implementation:** See [`.github/workflows/e2e-demo.yml`](.github/workflows/e2e-demo.yml)

**Workflow:**

```yaml
# Excerpt from e2e-demo.yml
- name: Run E2E Demo
  run: ./demo-orbit-ci.sh
  timeout-minutes: 10

- name: Analyze Metrics
  run: python scripts/analyze-metrics.py e2e-metrics.json

- name: Upload Results
  uses: actions/upload-artifact@v4
  with:
    name: e2e-metrics
    path: e2e-metrics.json
```

**Benefits:**
- Automated full-stack validation
- Performance regression detection
- Cross-platform testing (Ubuntu, Windows, macOS)
- Artifact retention for trend analysis

### Scenario 4: Docker-Based Development Environment

**Goal:** Consistent dev environment across team.

**Recommended Variant:** Docker Compose

**Steps:**

1. **Initial Setup**
   ```bash
   docker-compose -f docker-compose.demo.yml build
   ```

2. **Daily Development**
   ```bash
   # Start services
   docker-compose -f docker-compose.demo.yml up

   # Edit code (hot reload enabled)
   # Dashboard changes reflect immediately
   # Backend requires rebuild
   ```

3. **E2E Testing**
   ```bash
   # Run automated demo
   docker-compose -f docker-compose.demo.yml --profile demo up
   ```

**Advantages:**
- No local Rust/Node.js installation required
- Reproducible builds
- Isolated from host system
- Easy cleanup: `docker-compose down -v`

---

## Metrics & Analytics

### Metrics Collection

All demo variants (except interactive) automatically collect performance metrics:

**Collected Metrics:**

```json
{
  "timestamp": "2025-12-17T10:30:45Z",
  "job_id": 42,
  "total_duration_seconds": 87,
  "test_files_count": 23,
  "test_data_bytes": 178257920,
  "destination_files_count": 23,
  "transfer_success": true,
  "preflight_duration_seconds": 2,
  "data_fabrication_duration_seconds": 15,
  "ignition_duration_seconds": 35,
  "health_check_duration_seconds": 8,
  "job_creation_duration_seconds": 1,
  "job_monitoring_duration_seconds": 26,
  "job_status": "completed",
  "job_progress": 100.0
}
```

### Analyzing Metrics

#### Command-Line Analysis

```bash
# Generate report
python scripts/analyze-metrics.py e2e-metrics.json

# Export to JSON
python scripts/analyze-metrics.py e2e-metrics.json --export-json analysis.json

# Export to Markdown
python scripts/analyze-metrics.py e2e-metrics.json --export-md report.md
```

#### Sample Report

```
================================================================================
ORBIT E2E METRICS ANALYSIS REPORT
================================================================================

📊 SUMMARY
--------------------------------------------------------------------------------
  Timestamp:       2025-12-17T10:30:45Z
  Job ID:          42
  Total Duration:  87s
  Success:         ✓
  Files:           23
  Data Size:       170.0 MB

⚡ PERFORMANCE
--------------------------------------------------------------------------------
  Throughput:      6.54 MB/s
  Overhead:        61s (70.1%)
  Transfer Time:   26s

  Phase Breakdown:
    preflight               2.00s
    data_fabrication       15.00s
    ignition               35.00s
    job_creation            1.00s
    job_monitoring         26.00s
    health_check            8.00s

🏥 HEALTH
--------------------------------------------------------------------------------
  Status:          ✓ HEALTHY
  No issues or warnings detected.

💡 RECOMMENDATIONS
--------------------------------------------------------------------------------
  • High overhead. Optimize startup time or increase test data size.
  • Pre-build binaries to reduce startup time.
================================================================================
```

### Benchmarking

#### Comparing Runs

```bash
# Run demo multiple times
for i in {1..5}; do
  ./demo-orbit-ci.sh
  mv e2e-metrics.json metrics-run-$i.json
done

# Compare throughput
for f in metrics-run-*.json; do
  echo "$f: $(jq '.throughput_mbps' $f)"
done
```

#### Tracking Trends

Store metrics in a time-series database:

```python
# Example: Push to Prometheus
import json
from prometheus_client import CollectorRegistry, Gauge, push_to_gateway

with open('e2e-metrics.json') as f:
    metrics = json.load(f)

registry = CollectorRegistry()
g = Gauge('orbit_e2e_throughput_mbps', 'E2E Demo Throughput', registry=registry)
g.set(metrics['throughput_mbps'])

push_to_gateway('localhost:9091', job='orbit_e2e', registry=registry)
```

---

## Video Recording

### Prerequisites

- **ffmpeg** installed and in PATH
- Display server available (X11 on Linux, Quartz on macOS, GDI on Windows)
- Sufficient disk space (~100MB per minute at 1080p)

### Recording Process

#### Unix/Linux/macOS

```bash
./demo-orbit-record.sh
```

**What happens:**

1. **Pre-flight** - Checks for ffmpeg
2. **Setup** - Creates `demo-recordings/` directory
3. **User Prompt** - "Arrange windows, press ENTER to start recording"
4. **Recording Starts** - 3-second countdown
5. **Demo Runs** - Normal demo flow with visual cues
6. **Recording Stops** - On ENTER press
7. **Output** - MP4 file + thumbnail (JPG)

#### Windows

```bat
demo-orbit-record.bat
```

**Additional Fallback:**

If ffmpeg is not available, the script offers to continue without recording.

### Customizing Recording Settings

Edit the ffmpeg command in the script:

```bash
# High quality (larger file)
ffmpeg -f x11grab -r 60 -s 1920x1080 -i $DISPLAY \
    -vcodec libx264 -preset slow -crf 18 \
    -pix_fmt yuv420p "$VIDEO_FILE"

# Low quality (smaller file, faster)
ffmpeg -f x11grab -r 15 -s 1280x720 -i $DISPLAY \
    -vcodec libx264 -preset ultrafast -crf 28 \
    -pix_fmt yuv420p "$VIDEO_FILE"
```

### Post-Processing

```bash
# Trim video
ffmpeg -i orbit-demo.mp4 -ss 00:00:10 -to 00:05:30 -c copy trimmed.mp4

# Add watermark
ffmpeg -i orbit-demo.mp4 -i logo.png -filter_complex "overlay=10:10" watermarked.mp4

# Convert to GIF for sharing
ffmpeg -i orbit-demo.mp4 -vf "fps=10,scale=640:-1" orbit-demo.gif

# Extract frames for slides
ffmpeg -i orbit-demo.mp4 -vf "fps=1/5" frame-%03d.png
```

---

## Docker Deployment

See [DOCKER_DEMO_GUIDE.md](DOCKER_DEMO_GUIDE.md) for comprehensive Docker documentation.

### Quick Reference

#### Build and Run

```bash
# Build images
docker-compose -f docker-compose.demo.yml build

# Run services only
docker-compose -f docker-compose.demo.yml up

# Run with E2E demo
docker-compose -f docker-compose.demo.yml --profile demo up
```

#### Environment Variables

```bash
# Headless mode
export ORBIT_DEMO_HEADLESS=true
export ORBIT_DEMO_AUTO_CONFIRM=true
docker-compose -f docker-compose.demo.yml --profile demo up
```

#### Volume Management

```bash
# Backup database
docker run --rm -v orbit-demo-data:/data -v $(pwd):/backup \
  alpine tar czf /backup/backup.tar.gz -C /data .

# Clean up
docker-compose -f docker-compose.demo.yml down -v
```

---

## CI/CD Integration

### GitHub Actions

Full workflow: [`.github/workflows/e2e-demo.yml`](.github/workflows/e2e-demo.yml)

**Key Features:**
- ✅ Matrix testing (Ubuntu, Windows, macOS)
- ✅ Caching (Cargo, NPM)
- ✅ Metrics collection and analysis
- ✅ Artifact retention (logs, metrics)
- ✅ Scheduled runs (daily at 2 AM UTC)

**Manual Trigger:**

```bash
# Via GitHub CLI
gh workflow run e2e-demo.yml

# With inputs
gh workflow run e2e-demo.yml \
  -f record_video=true \
  -f collect_metrics=true
```

### GitLab CI

```yaml
# .gitlab-ci.yml
e2e-demo:
  stage: test
  script:
    - ./demo-orbit-ci.sh
    - python scripts/analyze-metrics.py e2e-metrics.json
  artifacts:
    when: always
    paths:
      - e2e-metrics.json
      - orbit-server.log
      - orbit-dashboard.log
    reports:
      junit: e2e-metrics.json  # If converted to JUnit format
```

### Jenkins

```groovy
pipeline {
    agent any
    stages {
        stage('E2E Demo') {
            steps {
                sh './demo-orbit-ci.sh'
                sh 'python scripts/analyze-metrics.py e2e-metrics.json'
            }
        }
    }
    post {
        always {
            archiveArtifacts artifacts: 'e2e-metrics.json, *.log', allowEmptyArchive: true
        }
    }
}
```

---

## Troubleshooting

### Common Issues

#### Issue: Port 8080 or 5173 already in use

**Solution:**

```bash
# Find and kill process
lsof -ti:8080 | xargs kill -9
lsof -ti:5173 | xargs kill -9

# Or change ports (edit demo script)
export API_URL=http://localhost:9080
export DASHBOARD_URL=http://localhost:6173
```

#### Issue: Health check timeout

**Symptoms:** "Timeout waiting for API to become healthy"

**Solutions:**

1. Check server logs:
   ```bash
   tail -f orbit-server.log
   ```

2. Verify Rust compilation succeeded:
   ```bash
   cd crates/orbit-web && cargo build --bin orbit-server
   ```

3. Check for port conflicts (see above)

4. Increase timeout in script:
   ```bash
   # Edit demo-orbit.sh
   MAX_RETRIES=120  # Increase from 60
   ```

#### Issue: Job creation fails

**Symptoms:** "Failed to create job. Response: <error>"

**Solutions:**

1. Verify JWT secret is set:
   ```bash
   echo $ORBIT_JWT_SECRET
   # Should output: demo-secret-key-must-be-32-chars-long
   ```

2. Check API is responsive:
   ```bash
   curl http://localhost:8080/api/health
   ```

3. Review API logs for specific error:
   ```bash
   grep ERROR orbit-server.log
   ```

#### Issue: No files transferred (count mismatch)

**Symptoms:** "File count mismatch: Expected 23, got 0"

**Solutions:**

1. Check job status:
   ```bash
   curl http://localhost:8080/api/jobs/$JOB_ID
   ```

2. Verify source files were created:
   ```bash
   ls -la /tmp/orbit_demo_source_*
   ```

3. Check permissions on destination:
   ```bash
   ls -ld /tmp/orbit_demo_dest_*
   ```

### Windows-Specific Issues

#### Script Hangs at User Input Prompts (Windows)

**⚠️ KNOWN ISSUE: Windows Batch File Input Handling**

If running `demo-orbit.bat` and the script hangs at pause prompts, shows key codes instead of continuing, or Ctrl+C doesn't work:

**Root Cause:**

Windows batch commands (`pause`, `timeout`, `choice`) have unreliable stdin handling when background processes are running:
1. Background processes (`cargo.exe`, `node.exe`) launched via `start /B` share stdin with parent script
2. Multiple processes compete for keyboard input
3. Input gets intercepted by wrong process or corrupted

**Symptoms:**
- Pressing keys shows numbers like `225`, `180` instead of continuing
- "Press C to continue" does nothing when pressing C
- "Terminate batch job (Y/N)?" appears but Y/N do nothing
- Error: `'tinue' is not recognized` (command corruption)
- Ctrl+C gets stuck or doesn't abort

**Solution Implemented (Latest Version):**

The demo now uses PowerShell `Read-Host` instead of native batch input commands:

```batch
# OLD (unreliable):
pause
timeout /t 300
choice /c C /t 300 /d C

# NEW (reliable):
powershell -Command "$null = Read-Host 'Press ENTER to continue'"

# Also isolates background process stdin:
start /B "Orbit-Server" cmd /c "cargo run < nul > log 2>&1"
```

**Why PowerShell Works:**
- ✓ Dedicated PowerShell process handles input (no stdin sharing)
- ✓ Background processes isolated with `< nul` redirection
- ✓ Ctrl+C properly propagates to PowerShell
- ✓ No command corruption
- ✓ Works in Command Prompt, PowerShell, Windows Terminal

**Version Check:**

Check if your scripts have the fix:

```batch
REM Look for PowerShell Read-Host in demo-orbit.bat:
findstr /C:"Read-Host" demo-orbit.bat

REM Should see:
REM powershell -Command "$null = Read-Host 'Press ENTER to continue'"

REM If you see these instead, update:
REM pause
REM timeout /t 300
REM choice /c C
```

**Update to Latest:**
```batch
git pull origin main
```

**Recovery If Stuck:**

1. Force close Command Prompt window, or use Task Manager (Ctrl+Shift+Esc) to end `cmd.exe`
2. Clean up orphaned processes:
   ```batch
   tasklist | findstr "cargo node orbit"
   taskkill /F /IM cargo.exe
   taskkill /F /IM node.exe
   ```
3. Check logs:
   ```batch
   type orbit-server.log
   type orbit-dashboard.log
   ```

**Workaround (Older Versions):**

If you can't update immediately:
- Use Windows Terminal instead of Command Prompt (better stdin handling)
- Run with stdin redirect (auto-continues demo):
  ```batch
  demo-orbit.bat < NUL
  ```

#### Character Encoding Issues (Windows)

If you see garbled characters like `ÔòöÔòÉ` instead of `╔═══╗`:

**Fix (Automatic):** Latest scripts set UTF-8 with `chcp 65001`

**Manual Fix:**
```batch
chcp 65001
demo-orbit.bat
```

**Best Practice:** Use Windows Terminal with a Unicode font (Cascadia Code, Consolas)

### Debug Mode

Enable verbose logging:

```bash
# Set environment variable
export RUST_LOG=debug

# Or edit demo script
RUST_LOG=debug cargo run --bin orbit-server
```

**Windows:**
```batch
set RUST_LOG=debug
demo-orbit.bat
```

### Clean Slate

If all else fails, reset everything:

**Unix/Linux/macOS:**
```bash
# Kill all processes
pkill -f orbit-server
pkill -f "npm run dev"

# Remove temp files
rm -rf /tmp/orbit_demo_*

# Remove database
rm -f crates/orbit-web/magnetar.db

# Re-run demo
./demo-orbit.sh
```

**Windows:**
```batch
REM Kill all processes
taskkill /F /IM cargo.exe
taskkill /F /IM node.exe
taskkill /F /IM orbit-server.exe

REM Remove temp files
rd /s /q %TEMP%\orbit_demo_source_*
rd /s /q %TEMP%\orbit_demo_dest_*

REM Remove database
del /f crates\orbit-web\magnetar.db

REM Re-run demo
demo-orbit.bat
```

---

## Advanced Topics

### Customizing Data Volume

**For larger transfers (performance testing):**

Edit `demo-orbit.sh`:

```bash
# Increase file sizes (in MB)
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_alpha.bin" bs=1M count=500   # 500MB
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_beta.bin" bs=1M count=200    # 200MB
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_gamma.bin" bs=1M count=1000  # 1GB
```

**For faster CI runs:**

```bash
# Smaller files
dd if=/dev/zero of="$DEMO_SOURCE/test.bin" bs=1M count=10  # 10MB
```

### Adding Custom Jobs

Inject multiple jobs for parallel testing:

```bash
# After first job, add second job
JOB_PAYLOAD_2=$(cat <<EOF
{
  "source": "$DEMO_SOURCE_2",
  "destination": "$DEMO_DEST_2",
  "compress": false,
  "verify": true,
  "parallel_workers": 8
}
EOF
)

JOB_ID_2=$(curl -s -X POST "$API_URL/api/create_job" \
  -H "Content-Type: application/json" \
  -d "$JOB_PAYLOAD_2")
```

### Integrating with Monitoring

**Prometheus Metrics:**

Modify `demo-orbit-ci.sh` to export metrics:

```bash
# At end of script
cat > metrics.prom <<EOF
# HELP orbit_demo_duration_seconds Total demo duration
# TYPE orbit_demo_duration_seconds gauge
orbit_demo_duration_seconds $DURATION

# HELP orbit_demo_throughput_mbps Transfer throughput
# TYPE orbit_demo_throughput_mbps gauge
orbit_demo_throughput_mbps $THROUGHPUT
EOF

# Push to Pushgateway
curl -X POST --data-binary @metrics.prom http://pushgateway:9091/metrics/job/orbit_demo
```

**Grafana Dashboard:**

Create dashboard with panels:
- Demo success rate (over time)
- Average throughput (by platform)
- Duration breakdown (stacked area chart)
- Health check time (histogram)

### Scripting Variations

**Headless browser testing (with Selenium):**

```python
from selenium import webdriver
from selenium.webdriver.common.by import By
import time

# Start demo in background
subprocess.Popen(['./demo-orbit-ci.sh'])

# Wait for dashboard
time.sleep(30)

# Launch browser
driver = webdriver.Chrome()
driver.get('http://localhost:5173')

# Verify elements
assert "Orbit Dashboard" in driver.title
jobs = driver.find_elements(By.CLASS_NAME, 'job-card')
assert len(jobs) > 0

driver.quit()
```

**Load testing:**

```bash
# Start services once
./launch-orbit.sh

# Inject 100 jobs
for i in {1..100}; do
  curl -X POST http://localhost:8080/api/create_job \
    -H "Content-Type: application/json" \
    -d '{"source":"/tmp/src","destination":"/tmp/dst","compress":true,"verify":true,"parallel_workers":4}'
done
```

---

## Additional Resources

- **Main Documentation**: [README.md](README.md)
- **Demo Guide**: [DEMO_GUIDE.md](DEMO_GUIDE.md)
- **Docker Guide**: [DOCKER_DEMO_GUIDE.md](DOCKER_DEMO_GUIDE.md)
- **GitHub Workflow**: [.github/workflows/e2e-demo.yml](.github/workflows/e2e-demo.yml)
- **API Documentation**: http://localhost:8080/swagger-ui (when running)

---

**Orbit v2.2.0-alpha** - The intelligent file transfer tool that never gives up 💪
