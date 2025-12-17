# 📝 Orbit Demo - Logging & Troubleshooting Guide

## Overview

The Orbit demo harness includes comprehensive logging to help diagnose issues when things don't work as expected. This guide explains the logging system and troubleshooting workflows.

## Log Files

### Automatic Logs (Created by Demo Scripts)

| Log File | Location | Purpose | When Created |
|----------|----------|---------|--------------|
| **Demo Run Log** | `demo-logs/demo-run-YYYYMMDD-HHMMSS.log` | Complete event timeline | Every demo run |
| **Error Log** | `demo-logs/demo-errors-YYYYMMDD-HHMMSS.log` | Errors only | When errors occur |
| **Server Log** | `orbit-server.log` | Rust backend output | When server starts |
| **Dashboard Log** | `orbit-dashboard.log` | React frontend output | When dashboard starts |
| **Metrics** | `e2e-metrics.json` | Performance metrics | CI mode only |

### Log Directory Structure

```
orbit/
├── demo-logs/                    # Demo orchestration logs
│   ├── demo-run-20251217-103045.log
│   ├── demo-errors-20251217-103045.log
│   └── ... (one per run)
├── orbit-server.log              # Backend logs
├── orbit-dashboard.log           # Frontend logs
└── e2e-metrics.json              # Metrics (CI mode)
```

## Log Levels

### Demo Run Log

```
[YYYY-MM-DD HH:MM:SS] [LEVEL] Message
```

**Levels:**
- `[INFO]` - Normal operational events
- `[WARN]` - Warnings that don't stop execution
- `[ERROR]` - Errors that cause failure
- `[DEBUG]` - Detailed diagnostic info (debug mode only)

**Example:**
```
[2025-12-17 10:30:45] [INFO] Demo orchestrator started
[2025-12-17 10:30:46] [DEBUG] Checking for command: cargo
[2025-12-17 10:30:46] [DEBUG] Found cargo at: /usr/bin/cargo
[2025-12-17 10:31:15] [WARN] Health check taking longer than expected
[2025-12-17 10:31:20] [INFO] Control Plane is online
[2025-12-17 10:32:45] [ERROR] Job creation failed: Connection refused
```

## Enabling Debug Mode

### Unix/Linux/macOS

```bash
# Enable verbose debug output
export ORBIT_DEMO_DEBUG=true
./demo-orbit.sh

# One-liner
ORBIT_DEMO_DEBUG=true ./demo-orbit.sh
```

### Windows

```batch
REM Set environment variable
set ORBIT_DEMO_DEBUG=true
demo-orbit.bat

REM Or inline (PowerShell)
$env:ORBIT_DEMO_DEBUG="true"; .\demo-orbit.bat
```

### What Debug Mode Does

- ✅ Prints all log messages to terminal (not just errors)
- ✅ Shows command paths and versions
- ✅ Displays process IDs
- ✅ Logs all API calls and responses
- ✅ Shows file operations (create, delete)

## Analyzing Logs

### Quick Log Analysis

```bash
# View latest demo log
cat demo-logs/demo-run-*.log | tail -1

# View all errors
cat demo-logs/demo-errors-*.log

# Search for specific term
grep "health check" demo-logs/demo-run-*.log

# Count errors
grep -c "\[ERROR\]" demo-logs/demo-run-*.log
```

### Automated Analysis Tool

Use the built-in log analyzer:

```bash
./scripts/analyze-logs.sh
```

**Output:**
```
╔════════════════════════════════════════════╗
║       📊 ORBIT LOG ANALYZER               ║
╚════════════════════════════════════════════╝

Analyzing logs...
  Demo log:      demo-logs/demo-run-20251217-103045.log
  Error log:     demo-logs/demo-errors-20251217-103045.log
  Server log:    orbit-server.log
  Dashboard log: orbit-dashboard.log

═══════════════════════════════════════════
LOG STATISTICS
═══════════════════════════════════════════
Total lines:    127
Errors:         2
Warnings:       1
Info messages:  45

═══════════════════════════════════════════
ERRORS FOUND
═══════════════════════════════════════════
[2025-12-17 10:32:45] [ERROR] API health check timeout after 60s
[2025-12-17 10:32:45] [ERROR] Server PID: 12345 (running: no)

═══════════════════════════════════════════
DIAGNOSTIC CHECKS
═══════════════════════════════════════════
✗ Port conflict detected
  Issue: API port 8080 is already in use
  Fix: Kill process using port: lsof -ti:8080 | xargs kill -9
✓ No compilation errors
✓ No NPM errors
✗ API health check timeout
  Likely causes:
    1. Server failed to start (check orbit-server.log)
    2. Port 8080 blocked by firewall
    3. Database initialization failed
```

## Common Issues & Diagnostics

### Issue 1: Port Already in Use

**Symptoms:**
```
[ERROR] Address already in use
[ERROR] API health check timeout
```

**Diagnosis:**
```bash
# Check what's using port 8080
lsof -ti:8080  # Unix/Linux/macOS
netstat -ano | findstr :8080  # Windows

# View error in logs
grep "Address already in use" orbit-server.log
```

**Solution:**
```bash
# Kill process on port 8080
lsof -ti:8080 | xargs kill -9  # Unix/Linux/macOS
taskkill /F /PID <PID>  # Windows (get PID from netstat)

# Or change port in demo script
export API_URL=http://localhost:9080
```

### Issue 2: Rust Compilation Failure

**Symptoms:**
```
[ERROR] Failed to compile orbit-server
error[E0425]: cannot find value...
```

**Diagnosis:**
```bash
# View compilation errors
cat orbit-server.log | grep "error\["

# Check Rust version
cargo --version
```

**Solution:**
```bash
# Update Rust toolchain
rustup update stable

# Clean and rebuild
cargo clean
cd crates/orbit-web && cargo build --bin orbit-server
```

### Issue 3: NPM Install Failure

**Symptoms:**
```
npm ERR! code ENOSPC
npm ERR! syscall write
```

**Diagnosis:**
```bash
# Check NPM errors
cat orbit-dashboard.log | grep "npm ERR!"

# Check disk space
df -h .  # Unix/Linux/macOS
Get-PSDrive C  # Windows (PowerShell)
```

**Solution:**
```bash
# Free up space
cargo clean  # Remove Rust build cache

# Clear npm cache
npm cache clean --force

# Retry
cd dashboard && npm ci
```

### Issue 4: Job Creation Fails

**Symptoms:**
```
[ERROR] Failed to create job. Response: {"error":"Unauthorized"}
```

**Diagnosis:**
```bash
# Check if JWT secret is set
echo $ORBIT_JWT_SECRET

# Test API manually
curl http://localhost:8080/api/health
```

**Solution:**
```bash
# Set JWT secret
export ORBIT_JWT_SECRET="demo-secret-key-must-be-32-chars-long"

# Restart demo
./demo-orbit.sh
```

### Issue 5: Server Crashes Mid-Demo

**Symptoms:**
```
[ERROR] Server PID: 12345 (running: no)
[ERROR] Job monitoring failed: Connection refused
```

**Diagnosis:**
```bash
# Check server log for panic
tail -50 orbit-server.log

# Look for specific errors
grep "panic\|thread.*panicked" orbit-server.log
```

**Solution:**
```bash
# Enable Rust backtrace
export RUST_BACKTRACE=1
./demo-orbit.sh

# Or full backtrace
export RUST_BACKTRACE=full
```

## Viewing Logs in Real-Time

### Follow Server Log

```bash
# Unix/Linux/macOS
tail -f orbit-server.log

# Windows (PowerShell)
Get-Content orbit-server.log -Wait
```

### Follow Dashboard Log

```bash
# Unix/Linux/macOS
tail -f orbit-dashboard.log

# Windows (PowerShell)
Get-Content orbit-dashboard.log -Wait
```

### Follow All Logs

```bash
# Unix/Linux/macOS (requires multitail or tmux)
tmux new-session \; \
  split-window -h \; \
  send-keys 'tail -f orbit-server.log' C-m \; \
  split-window -v \; \
  send-keys 'tail -f orbit-dashboard.log' C-m \; \
  select-pane -t 0 \; \
  send-keys 'tail -f demo-logs/demo-run-*.log' C-m
```

## Log Retention

### Automatic Cleanup

Demo logs are kept indefinitely by default. To clean up:

```bash
# Remove logs older than 7 days
find demo-logs/ -name "*.log" -mtime +7 -delete

# Remove all demo logs
rm -rf demo-logs/*.log

# Keep only last 10 runs
ls -t demo-logs/demo-run-*.log | tail -n +11 | xargs rm -f
```

### Archiving Logs

```bash
# Archive old logs
tar czf demo-logs-archive-$(date +%Y%m%d).tar.gz demo-logs/*.log
mv demo-logs-archive-*.tar.gz archives/
rm -f demo-logs/*.log
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
- name: Run Demo
  run: ./demo-orbit-ci.sh
  continue-on-error: true

- name: Analyze Logs on Failure
  if: failure()
  run: ./scripts/analyze-logs.sh

- name: Upload Logs
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: demo-logs
    path: |
      demo-logs/
      orbit-server.log
      orbit-dashboard.log
      e2e-metrics.json
```

### Parsing Logs in CI

```bash
# Extract error count
ERROR_COUNT=$(grep -c "\[ERROR\]" demo-logs/demo-run-*.log || echo "0")

# Fail if errors found
if [ "$ERROR_COUNT" -gt 0 ]; then
    echo "::error::Demo failed with $ERROR_COUNT errors"
    cat demo-logs/demo-errors-*.log
    exit 1
fi
```

## Advanced Diagnostics

### Enable Rust Tracing

```bash
# Detailed Rust logs
export RUST_LOG=debug
./demo-orbit.sh

# Specific module
export RUST_LOG=orbit_web=trace,orbit_core=debug
./demo-orbit.sh

# Filter noise
export RUST_LOG=orbit_web=debug,tower_http=warn
```

### Network Diagnostics

```bash
# Test API connectivity
curl -v http://localhost:8080/api/health

# Watch network traffic
tcpdump -i lo0 port 8080  # macOS
tcpdump -i lo port 8080   # Linux
```

### Process Monitoring

```bash
# Watch processes
watch -n 1 'ps aux | grep -E "orbit-server|npm run dev"'

# Monitor resource usage
top -p $(pgrep orbit-server)
```

## Getting Help

When reporting issues, include:

1. **Demo log** - `demo-logs/demo-run-*.log`
2. **Error log** - `demo-logs/demo-errors-*.log`
3. **Server log excerpt** - Last 50 lines of `orbit-server.log`
4. **System info** - OS, Rust version, Node version
5. **Command used** - Exact command that triggered the error

**Example bug report:**

```markdown
## Issue
Demo fails with "API health check timeout"

## Environment
- OS: Ubuntu 22.04
- Rust: 1.75.0
- Node: 20.10.0
- Command: `./demo-orbit.sh`

## Logs
<details>
<summary>Error Log</summary>

```
[2025-12-17 10:32:45] [ERROR] API health check timeout after 60s
[2025-12-17 10:32:45] [ERROR] Server PID: 12345 (running: no)
```
</details>

<details>
<summary>Server Log (last 20 lines)</summary>

```
[tail of orbit-server.log]
```
</details>
```

---

**Orbit v2.2.0-alpha** - The intelligent file transfer tool that never gives up 💪
