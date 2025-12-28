# 🛡️ Orbit Demo - Safety First Guide

## TL;DR - Is it Safe?

**YES!** The demo is designed to be 100% safe with automatic cleanup. Here's what you need to know:

### ✅ What the Demo WILL Do (Safe)

- Create temporary test files in `/tmp` (auto-deleted when done)
- Start two processes in the background (auto-killed when done)
- Create small log files (~50MB max)
- Open your browser to localhost

### ❌ What the Demo WON'T Do (Guaranteed)

- ✗ Modify your code or git repository
- ✗ Delete any of your existing files
- ✗ Change system settings
- ✗ Install software globally
- ✗ Require sudo/admin rights
- ✗ Make network connections (except localhost)
- ✗ Leave processes running after cleanup

## Pre-Flight Safety Validator

**Run this FIRST before the demo** - it checks everything without making changes:

```bash
./scripts/validate-demo-safety.sh
```

**What it checks:**
- ✓ System requirements (OS, architecture)
- ✓ Required commands (cargo, npm, curl)
- ✓ Port availability (8080, 5173)
- ✓ Disk space (need 4GB minimum)
- ✓ Existing processes
- ✓ Write permissions

**Example output:**
```
╔════════════════════════════════════════════╗
║    🛡️  ORBIT DEMO SAFETY VALIDATOR       ║
║    NO CHANGES WILL BE MADE TO YOUR SYSTEM ║
╚════════════════════════════════════════════╝

[1/8] Checking System Requirements
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ Operating System: Linux
✓ Architecture: x86_64 (supported)

[2/8] Checking Required Commands
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ cargo found: cargo 1.75.0
✓ npm found: npm 10.2.0
✓ curl found: curl 7.88.1

[3/8] Checking Port Availability
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ Port 8080 available
✓ Port 5173 available

═══════════════════════════════════════════
VALIDATION SUMMARY
═══════════════════════════════════════════
  Checks Passed:  15
  Warnings:       0
  Checks Failed:  0

✅ ALL CHECKS PASSED!
Your system is ready for the demo.
```

## What Happens Step-by-Step

### Phase 1: Pre-flight (30 seconds)
```
What happens:  Checks for cargo, npm, curl
Files touched: NONE
Processes:     NONE
Reversible:    N/A (read-only checks)
```

### Phase 2: Data Fabrication (10-30 seconds)
```
What happens:  Creates 170MB of random test data
Location:      /tmp/orbit_demo_source_TIMESTAMP/
Files created: 23 files (binary blobs + logs)
Reversible:    ✓ Auto-deleted on cleanup
```

### Phase 3: System Ignition (30-60 seconds)
```
What happens:  Compiles and starts backend + frontend
Processes:     orbit-server (port 8080), npm dev (port 5173)
Files created: orbit-server.log, orbit-dashboard.log, magnetar.db
Reversible:    ✓ Processes killed on cleanup, logs can be deleted
```

### Phase 4: Job Injection (5 seconds)
```
What happens:  Creates job via API, starts file transfer
Files touched: Creates files in /tmp/orbit_demo_dest_TIMESTAMP/
Database:      Adds job record to magnetar.db
Reversible:    ✓ Destination files auto-deleted
```

### Phase 5: Observation (User controlled)
```
What happens:  Waits for user to press ENTER
Files touched: NONE (just displays dashboard)
Reversible:    N/A (passive observation)
```

### Phase 6: Cleanup (5 seconds)
```
What happens:  Removes temp files, kills processes
Files deleted: /tmp/orbit_demo_source_*, /tmp/orbit_demo_dest_*
Processes:     All stopped
Remains:       Only logs (orbit-*.log, demo-logs/)
```

## Safety Mechanisms

### 1. Automatic Cleanup (Trap Handler)

**Ctrl+C anytime** - cleanup runs automatically:

```bash
trap cleanup EXIT  # Runs on ANY exit (success, failure, Ctrl+C)
```

**What gets cleaned:**
- Temporary source files
- Temporary destination files
- Background processes (server, dashboard)

**What remains:**
- Logs (for troubleshooting, safe to delete)
- Database (demo jobs only, safe to delete)
- Your code (untouched)

### 2. No Sudo Required

The demo **never** asks for sudo/admin because:
- Writes only to `/tmp` (user-writable)
- Uses user ports (8080, 5173)
- Compiles in user space (`target/`)
- No system modifications

### 3. Isolated Temporary Files

All test data goes to timestamped directories:
```
/tmp/orbit_demo_source_1702819845/  # Unique per run
/tmp/orbit_demo_dest_1702819845/
```

**Benefits:**
- Won't conflict with other files
- Easy to identify and delete
- Auto-cleanup knows exact paths

### 4. Process Isolation

Processes run as your user with minimal permissions:
```
USER    PID   COMMAND
you     12345 cargo run --bin orbit-server  # YOUR user, not root
you     12346 npm run dev                   # YOUR user, not root
```

### 5. Read-Only Code Access

Demo **reads** your code but never modifies:
- `crates/orbit-web/src/` → Read only
- `dashboard/src/` → Read only
- Git repository → Never touched

**Only writes:**
- `target/` (compilation artifacts)
- `demo-logs/` (orchestration logs)
- `/tmp/orbit_demo_*` (test data)

## Common Concerns Addressed

### Q: "Will it delete my files?"

**A:** No. The demo:
- ✓ Only creates NEW files in `/tmp`
- ✓ Never touches your existing files
- ✓ Never runs `rm` on your directories
- ✓ Never modifies git repository

### Q: "Will processes keep running?"

**A:** No. Cleanup runs automatically:
- ✓ On normal exit
- ✓ On error
- ✓ On Ctrl+C
- ✓ On terminal close (trap handler)

Worst case: `pkill -f orbit-server` kills any orphans.

### Q: "Will it mess up my ports?"

**A:** No. The demo:
- ✓ Checks ports before starting
- ✓ Fails fast if ports in use
- ✓ Frees ports on cleanup
- ✓ Only uses localhost (not exposed externally)

### Q: "What if it crashes mid-demo?"

**A:** Cleanup still runs:
- `trap cleanup EXIT` ensures cleanup on ANY exit
- Even if demo crashes, temp files are removed
- Processes are killed

Manual cleanup if needed:
```bash
pkill -f orbit-server
pkill -f "npm run dev"
rm -rf /tmp/orbit_demo_*
```

### Q: "Will it fill my disk?"

**A:** No. The demo:
- ✓ Checks free space first (needs 4GB)
- ✓ Creates only 340MB temp data
- ✓ Auto-deletes temp data on exit
- ✓ Logs are small (~50MB max)

You control build artifacts:
```bash
cargo clean  # Remove 3GB of Rust artifacts
```

### Q: "Can I stop it anytime?"

**A:** YES!
- Press **Ctrl+C** anytime
- Cleanup runs automatically
- Safe to interrupt at any phase

### Q: "What about my existing database?"

**A:** Demo uses its own database:
- Creates: `crates/orbit-web/magnetar.db`
- If exists: Adds demo jobs (doesn't delete yours)
- Safe to delete after demo

## Dry-Run Mode (See Without Doing)

**Want to see what would happen first?**

```bash
ORBIT_DEMO_DRY_RUN=true ./demo-orbit.sh
```

**Dry-run mode:**
- ✓ Shows each command before running
- ✓ Prints what files would be created
- ✓ Displays what processes would start
- ✓ NO ACTUAL CHANGES MADE
- ✓ Exit anytime safely

**Example dry-run output:**
```
[DRY-RUN] Would create: /tmp/orbit_demo_source_1702819845/
[DRY-RUN] Would execute: dd if=/dev/urandom of=telemetry_alpha.bin bs=1M count=50
[DRY-RUN] Would start: cargo run --bin orbit-server (PID: TBD)
[DRY-RUN] Would open: http://localhost:5173 in browser
```

## Step-by-Step Mode (Confirm Each Action)

**Want to approve each step?**

```bash
ORBIT_DEMO_INTERACTIVE=true ./demo-orbit.sh
```

**Interactive mode:**
- Pauses before EACH phase
- Shows what will happen next
- Waits for your approval (y/n)
- Can abort anytime

**Example:**
```
[2/6] Data Fabrication
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
This will:
  • Create /tmp/orbit_demo_source_1702819845/
  • Generate 170MB of test files
  • No existing files modified

Continue? [y/N] _
```

## Rollback & Cleanup

### Automatic Cleanup (On Exit)

Already explained above - runs on ANY exit.

### Manual Cleanup (If Needed)

```bash
# Stop all Orbit processes
pkill -f orbit-server
pkill -f "npm run dev"

# Remove temp files
rm -rf /tmp/orbit_demo_*

# Remove logs (optional)
rm -f orbit-server.log orbit-dashboard.log
rm -rf demo-logs/

# Remove database (optional, only demo data)
rm -f crates/orbit-web/magnetar.db

# Remove build artifacts (optional, frees ~3GB)
cargo clean
```

### Nuclear Option (Full Reset)

```bash
# Stop everything
pkill -f orbit

# Clean all demo artifacts
rm -rf /tmp/orbit_demo_* orbit-*.log demo-logs/ demo-recordings/
rm -f crates/orbit-web/magnetar.db

# Reset build
cargo clean
rm -rf dashboard/node_modules

# Your code is still untouched!
```

## Testing the Safety

### Test 1: Pre-Flight Validator

```bash
./scripts/validate-demo-safety.sh
# Should pass without making ANY changes
# Run twice - results should be identical
```

### Test 2: Dry-Run

```bash
ORBIT_DEMO_DRY_RUN=true ./demo-orbit.sh
# Watch what WOULD happen
# NO changes made
```

### Test 3: Quick Abort

```bash
./demo-orbit.sh
# Wait for data fabrication to complete
# Press Ctrl+C during "System Ignition"
# Check: /tmp should have no orbit_demo_* (cleaned up)
```

### Test 4: Full Demo + Cleanup

```bash
./demo-orbit.sh
# Let it complete fully
# Press ENTER at end
# Check: /tmp should have no orbit_demo_*
# Check: No orbit processes running (ps aux | grep orbit)
```

## Security Audit Checklist

✅ **No network access** (only localhost)
✅ **No sudo required** (user-space only)
✅ **No code modification** (read-only access)
✅ **No system changes** (temporary files only)
✅ **No data exfiltration** (everything local)
✅ **No persistence** (auto-cleanup)
✅ **No privilege escalation** (runs as your user)
✅ **No registry changes** (Unix/Windows)
✅ **No firewall rules** (uses existing permissions)
✅ **No background tasks** (processes killed on exit)

## Recommended First Run

**For maximum safety on first run:**

```bash
# 1. Validate first (no changes)
./scripts/validate-demo-safety.sh

# 2. Review what will happen
cat SAFETY_FIRST.md  # This file!

# 3. Dry-run mode (see without doing)
ORBIT_DEMO_DRY_RUN=true ./demo-orbit.sh

# 4. Actual run with debug enabled
ORBIT_DEMO_DEBUG=true ./demo-orbit.sh

# 5. Review logs after
cat demo-logs/demo-run-*.log
./scripts/analyze-logs.sh
```

## Support & Issues

If something goes wrong:

1. **Press Ctrl+C** - cleanup runs automatically
2. **Check logs** - `cat demo-logs/demo-errors-*.log`
3. **Manual cleanup** - See "Manual Cleanup" section above
4. **Report issue** - Include logs (they contain NO sensitive data)

## The Bottom Line

**The Orbit demo is designed to be:**
- ✅ Safe by default
- ✅ Reversible (everything cleaned up)
- ✅ Transparent (logs show everything)
- ✅ Abortable (Ctrl+C anytime)
- ✅ Isolated (temp files, user processes)
- ✅ Auditable (open source, readable scripts)

**You can trust it because:**
- Open source (you can read every line)
- No sudo required (can't make system changes)
- Automatic cleanup (guaranteed via trap handler)
- Validated by safety checker
- Tested on multiple platforms
- Used in CI/CD (automated testing)

---

**Still worried? Run the validator first:**

```bash
./scripts/validate-demo-safety.sh
```

**It will tell you EXACTLY what the demo will do, without doing it.** 🛡️

---

**Orbit v2.2.0-alpha** - The intelligent file transfer tool that never gives up 💪
