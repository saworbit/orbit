# 🛰️ Orbit E2E Demonstration Harness

## 🛡️ Safety First - Recommended First Step

**Before running the demo for the first time**, use the safety validator to verify your system is ready **without making any changes**:

```bash
# Unix/Linux/macOS
./scripts/validate-demo-safety.sh

# Windows (Git Bash or WSL)
bash scripts/validate-demo-safety.sh
```

**What the validator checks:**
- ✓ System requirements (OS, architecture)
- ✓ Required commands (cargo, npm, curl)
- ✓ Port availability (8080, 5173)
- ✓ Disk space (minimum 4GB)
- ✓ Existing processes that might conflict
- ✓ Write permissions to /tmp

**Guaranteed safe:** The validator is read-only and makes **zero changes** to your system. It tells you exactly what the demo will do before you run it.

📖 **Complete Safety Documentation:** See [SAFETY_FIRST.md](SAFETY_FIRST.md) for comprehensive safety information, including what the demo will/won't do, step-by-step breakdown, cleanup mechanisms, and answers to common concerns.

---

## Overview

The Orbit E2E Demonstration Harness provides a sophisticated orchestration layer to automate the deployment, validation, and demonstration of the Orbit v2.2.0 stack. Rather than requiring manual setup and job creation, this harness implements "**The Deep Space Telemetry Scenario**" - a complete end-to-end workflow that showcases Orbit's capabilities.

## The Scenario: Deep Space Telemetry Ingestion

This demonstration simulates a critical data ingestion workflow with the following phases:

### 1. **Environment Validation**
- Verifies required tools (Rust/Cargo, Node.js/NPM, curl)
- Checks port availability (8080 for API, 5173 for Dashboard)
- Ensures the system is ready for demonstration

### 2. **Data Fabrication**
- Generates synthetic telescope telemetry dataset (~170MB)
- Creates three binary blob files simulating sensor data:
  - `telemetry_alpha.bin` (50MB)
  - `telemetry_beta.bin` (20MB)
  - `telemetry_gamma.bin` (100MB)
- Generates 20 simulated flight log files
- Creates a mission manifest JSON file

### 3. **System Ignition**
- Launches the Magnetar Control Plane (Rust backend on port 8080)
- Launches the React Dashboard (frontend on port 5173)
- Waits for health check confirmation
- Opens browser to dashboard automatically

### 4. **Job Injection**
- Programmatically creates a transfer job via REST API
- Configures job with:
  - Compression: Enabled
  - Verification: Enabled (checksum validation)
  - Parallel Workers: 4
- Starts job execution automatically

### 5. **Observation Phase**
- Pauses to allow inspection of the dashboard
- User can observe:
  - **Visual Chunk Map** - Real-time chunk transfer visualization
  - **Live Telemetry** - Transfer speed and progress graphs
  - **Job Status** - Current state and statistics

### 6. **Orbital Decay (Cleanup)**
- Gracefully terminates backend and frontend services
- Removes synthetic data directories
- Ensures clean system state

## Prerequisites

### Disk Space Requirements

| Scenario | Minimum | Recommended | Notes |
|----------|---------|-------------|-------|
| **Quick Demo** (pre-built binaries) | 400 MB | 1 GB | Source (170MB) + Dest (170MB) + overhead |
| **Full Build** (compile from source) | 4 GB | 6 GB | Includes Rust target/ (~3GB) + node_modules (~500MB) |
| **With Video Recording** | 5 GB | 8 GB | Add ~500MB per 5 minutes of recording (1080p) |
| **Docker Deployment** | 6 GB | 10 GB | Base images (~1GB) + build cache (~2GB) |

**Breakdown:**
- **Demo Data (Source)**: 170 MB (50MB + 20MB + 100MB binary files + logs)
- **Demo Data (Destination)**: 170 MB (copy of source)
- **Rust Build Artifacts** (`target/`): 2-5 GB (varies by features enabled)
- **Node Modules** (`dashboard/node_modules/`): 400-600 MB
- **Compiled Binary** (`orbit-server`): 15-30 MB
- **Database + Logs** (`magnetar.db`, `*.log`): 10-50 MB
- **Video Recording** (optional): ~100 MB/minute at 1080p30
- **Docker Images** (optional): 1-2 GB (base images + layers)

**💡 Space-Saving Tips:**
- Use pre-built binaries to avoid Rust compilation (saves ~3GB)
- Run `cargo clean` after building (removes temporary artifacts)
- Use Docker multi-stage builds (final image only ~200MB)
- Reduce test data size in CI mode (demo-orbit-ci.sh uses smaller files)

📖 **Detailed Guide:** See [`DISK_SPACE_GUIDE.md`](DISK_SPACE_GUIDE.md) for comprehensive information.

### All Platforms
- **Rust/Cargo** - For building and running the backend
- **Node.js/NPM** - For running the React dashboard
- **curl** - For API health checks and job creation

### Platform-Specific Tools

**Unix/Linux/macOS:**
- `bash` shell (standard on most systems)
- `lsof` or `netstat` (for port checking, optional)
- `dd` or `/dev/urandom` (for file generation)

**Windows:**
- Windows 10 or later (for ANSI color support)
- **PowerShell** 5.0+ (for `Read-Host` user input - required)
- `fsutil` or PowerShell (for file generation)
- `netstat` (standard Windows utility)
- **Note:** PowerShell is used for reliable user input when background processes are running

## Usage

### Unix/Linux/macOS

```bash
cd /path/to/orbit
./demo-orbit.sh
```

### Windows

```batch
cd C:\path\to\orbit
demo-orbit.bat
```

## Execution Flow

1. **Pre-flight Checks** (~5 seconds)
   - System validation and dependency checking

2. **Data Fabrication** (~10-30 seconds)
   - Synthetic dataset generation

3. **System Ignition** (~30-60 seconds)
   - Backend compilation and startup
   - Frontend development server startup
   - Health check polling

4. **Launch Sequence**
   - Browser opens automatically to dashboard
   - Press ENTER when ready to inject the job

5. **Job Execution**
   - Job created and started via API
   - Watch dashboard for real-time updates

6. **Observation**
   - Press ENTER when ready to cleanup

7. **Cleanup**
   - Automatic process termination
   - Data removal
   - System restored to original state

## Key Features Demonstrated

### 🎯 Magnetar State Machine
- Job lifecycle: `pending` → `running` → `completed`
- Persistent job storage in SQLite
- Reactor pattern for asynchronous job processing

### 📊 Real-Time Dashboard
- Live WebSocket updates
- Visual chunk map showing transfer progress
- Telemetry graphs (speed, throughput)
- Job management interface

### 🔐 API Integration
- RESTful job creation (`POST /api/create_job`)
- Job execution control (`POST /api/run_job`)
- Health check monitoring (`GET /api/health`)

### 🚀 Resilient Transfer
- Content-defined chunking
- Parallel worker execution
- Compression and verification
- Checksum validation

## Script Architecture

### demo-orbit.sh (Unix/Linux/macOS)
- **Language**: Bash with POSIX compliance
- **Error Handling**: `set -e` for immediate exit on error
- **Cleanup**: `trap` ensures cleanup on Ctrl+C or exit
- **Portability**: Fallbacks for systems without `dd` or `lsof`
- **Logging**: Outputs to `orbit-server.log` and `orbit-dashboard.log`

### demo-orbit.bat (Windows)
- **Language**: Windows Batch with PowerShell fallbacks
- **Color Support**: ANSI escape codes for Windows 10+
- **Process Management**: Background processes with window titles
- **Cleanup**: Graceful termination by window title or PID
- **Error Handling**: Error level checking with informative messages

## Troubleshooting

### Port Already in Use
If ports 8080 or 5173 are already occupied:
```bash
# Linux/macOS
lsof -ti:8080 | xargs kill -9
lsof -ti:5173 | xargs kill -9

# Windows
netstat -ano | findstr :8080
taskkill /F /PID <PID>
```

### Health Check Timeout
If the API doesn't become healthy within 60 seconds:
1. Check logs: `orbit-server.log` and `orbit-dashboard.log`
2. Verify Rust compilation succeeded
3. Check for port conflicts
4. Ensure SQLite database is accessible

### File Generation Fails
If synthetic data generation fails:
- **Unix**: Ensure `/dev/urandom` or `/dev/zero` is accessible
- **Windows**: Run as Administrator for `fsutil` or ensure PowerShell is available

### Script Hangs at User Input Prompts (Windows)

**⚠️ KNOWN ISSUE: Windows Batch File Input Handling**

If the script hangs at the pause prompt, shows key codes instead of continuing, or Ctrl+C doesn't work, this is a known limitation of Windows batch file input commands when background processes are running.

**Technical Background:**

Windows batch commands like `pause`, `timeout`, and `choice` have unreliable input handling when:
1. Background processes (`cargo.exe`, `node.exe`) are running via `start /B`
2. stdin is shared between parent script and background processes
3. Multiple processes compete for keyboard input

**Symptoms You Might See (Older Versions):**
- Pressing any key shows countdown numbers (e.g., `225`, `180`)
- "Press C to continue" does nothing when pressing C
- "Terminate batch job (Y/N)?" appears but Y/N responses do nothing
- Error: `'tinue' is not recognized` (corrupted command execution)
- Ctrl+C doesn't work or gets stuck

**Solution Implemented (Current Version):**

The demo now uses PowerShell `Read-Host` for all user input, which properly isolates stdin:

```batch
# OLD (unreliable with background processes):
pause
timeout /t 300
choice /c C /n /t 300 /d C

# NEW (reliable):
powershell -Command "$null = Read-Host 'Press ENTER to continue'"
```

**How the PowerShell Fix Works:**
- ✓ Properly handles stdin without conflicts
- ✓ Works reliably with background processes running via `< nul` redirection
- ✓ Supports Ctrl+C abort
- ✓ Provides clear user feedback
- ✓ No corruption of command execution

**Immediate Recovery (If Stuck):**
1. **Force close** the Command Prompt window, or
2. Open Task Manager (Ctrl+Shift+Esc) and end the `cmd.exe` process

**Check Logs After Killing:**
```batch
REM In a new Command Prompt:

REM Check if background processes are still running
tasklist | findstr "cargo node orbit"

REM Kill any orphaned processes
taskkill /F /IM cargo.exe
taskkill /F /IM node.exe

REM Check server log for errors
type orbit-server.log

REM Check demo orchestration log (if created)
dir demo-logs\
type demo-logs\demo-run-*.log
```

**What to Look For in Logs:**
- **orbit-server.log**: Check last 50 lines for panics, errors, or compilation failures
- **orbit-dashboard.log**: Check for NPM errors or build failures
- **demo-logs/demo-errors-*.log**: Any errors during orchestration

**Version Check:**

Ensure you're using the latest version of the demo scripts. Check if your `demo-orbit.bat` uses PowerShell:

```batch
REM Look for this line in demo-orbit.bat (indicates latest version):
powershell -Command "$null = Read-Host 'Press ENTER to continue'"

REM If you see these instead, update your scripts:
pause                                    REM OLD - unreliable
timeout /t 300                           REM OLD - unreliable
choice /c C /n /t 300 /d C              REM OLD - unreliable
```

**Update to Latest Version:**
```batch
git pull origin main
```

**Alternative Workaround (Older Versions):**

If you can't update, use Windows Terminal or PowerShell instead of Command Prompt for better input handling, or redirect stdin:
```batch
REM Redirect stdin to prevent hanging (auto-continues demo)
demo-orbit.bat < NUL
```

### Garbled Characters or Odd Symbols (Windows)
If you see strange characters instead of box-drawing and emojis (e.g., `ÔòöÔòÉ` instead of `╔═`):

**Cause:** Windows console not set to UTF-8 code page.

**Fix (Automatic):** The latest batch files now automatically set UTF-8 encoding with `chcp 65001`. If you're still seeing issues:

```batch
REM Run this before the demo
chcp 65001

REM Then run the demo
demo-orbit.bat
```

**Alternative:** Use Windows Terminal (recommended over Command Prompt) or PowerShell with a Unicode-compatible font like "Cascadia Code" or "Consolas".

**If using Git Bash on Windows:** The encoding should work correctly by default. If not, ensure your terminal is set to UTF-8 in settings.

### Job Creation Fails
If the API returns an error instead of job ID:
1. Check that `ORBIT_JWT_SECRET` environment variable is set
2. Verify API endpoint is correct (`/api/create_job`)
3. Check JSON payload formatting
4. Review server logs for specific error messages

## API Endpoints Used

| Endpoint | Method | Purpose | Payload |
|----------|--------|---------|---------|
| `/api/health` | GET | Health check | None |
| `/api/create_job` | POST | Create new job | `CreateJobRequest` |
| `/api/run_job` | POST | Start pending job | `{job_id: i64}` |

### CreateJobRequest Schema
```json
{
  "source": "/path/to/source",
  "destination": "/path/to/destination",
  "compress": true,
  "verify": true,
  "parallel_workers": 4
}
```

## Integration with Existing Scripts

This demonstration harness complements the existing launch scripts:

- **launch-orbit.sh / launch-orbit.bat**: Quick development server startup
- **demo-orbit.sh / demo-orbit.bat**: Full E2E demonstration with synthetic data

The demo scripts are designed for:
- **Sales demonstrations**: Show Orbit's capabilities to potential users
- **Development testing**: Quickly validate full stack functionality
- **CI/CD integration**: Automated E2E testing in pipelines
- **Training**: Onboard new developers to the architecture

## Customization

### Adjust Data Volume
Edit the `dd` commands (Unix) or `fsutil` commands (Windows) to change file sizes:

```bash
# Unix - Create 200MB file instead of 100MB
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_gamma.bin" bs=1M count=200 status=none
```

```batch
REM Windows - Create 200MB file
fsutil file createnew "%DEMO_SOURCE%\telemetry_gamma.bin" 209715200
```

### Modify Job Configuration
Edit the JSON payload in the script:

```bash
JOB_PAYLOAD=$(cat <<EOF
{
  "source": "$DEMO_SOURCE",
  "destination": "$DEMO_DEST",
  "compress": true,
  "verify": true,
  "parallel_workers": 8  # Increase workers
}
EOF
)
```

### Change Ports
Modify the configuration variables at the top of the script:

```bash
API_URL="http://localhost:9000"
DASHBOARD_URL="http://localhost:3000"
```

## Security Considerations

- **JWT Secret**: The demo uses a hardcoded secret (`demo-secret-key-must-be-32-chars-long`)
  - **WARNING**: Only suitable for local development
  - Never use in production
  - Replace with secure secret management

- **Temporary Directories**: Files are created in `/tmp` (Unix) or `%TEMP%` (Windows)
  - Automatically cleaned up on exit
  - Verify cleanup completed after demonstration

- **Process Permissions**:
  - Unix script requires executable permission (`chmod +x`)
  - Windows script may require Administrator for `fsutil`

## Best Practices

1. **Run from Repository Root**: Always execute from the Orbit repository root directory
2. **Clean State**: Ensure no other Orbit instances are running before demo
3. **Browser Ready**: Have browser open and ready before starting
4. **Log Review**: Check logs if issues occur during demonstration
5. **Cleanup Verification**: Verify temporary directories are removed after demo

## Future Enhancements

Potential improvements for future versions:

- [ ] Add Docker containerization support
- [ ] Implement custom scenario selection (small files, large files, mixed)
- [ ] Add performance benchmarking and metrics collection
- [ ] Support for remote backend deployment
- [ ] Integration with CI/CD pipelines (headless mode)
- [ ] Automated screenshot/video capture for documentation
- [ ] Multi-job concurrent demonstration
- [ ] Network simulation (latency, packet loss)

## Contributing

When modifying the demo scripts:

1. Test on all supported platforms (Linux, macOS, Windows)
2. Maintain backward compatibility with Orbit v2.2.0+
3. Update this documentation for any new features
4. Ensure cleanup is robust (no orphaned processes or files)
5. Follow shell scripting best practices (shellcheck for bash)

## License

These demonstration scripts are part of the Orbit project and licensed under Apache 2.0.

---

**Orbit v2.2.0-alpha** - The intelligent file transfer tool that never gives up 💪
