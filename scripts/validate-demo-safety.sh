#!/bin/bash

# ==============================================================================
#  Orbit Demo Safety Validator
#  Runs comprehensive pre-flight checks WITHOUT making any changes
#  Version: 2.2.0-alpha
# ==============================================================================

set -e

# Colors
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

ORBIT_ROOT="$(pwd)"
CHECKS_PASSED=0
CHECKS_FAILED=0
WARNINGS=0

echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║    🛡️  ORBIT DEMO SAFETY VALIDATOR       ║${NC}"
echo -e "${BLUE}║    NO CHANGES WILL BE MADE TO YOUR SYSTEM ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo ""

check_pass() {
    echo -e "${GREEN}✓${NC} $1"
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    CHECKS_FAILED=$((CHECKS_FAILED + 1))
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

# 1. System Requirements
echo -e "${BOLD}[1/8] Checking System Requirements${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check OS
OS_TYPE=$(uname -s)
case "$OS_TYPE" in
    Linux*) check_pass "Operating System: Linux" ;;
    Darwin*) check_pass "Operating System: macOS" ;;
    CYGWIN*|MINGW*|MSYS*) check_pass "Operating System: Windows (Git Bash)" ;;
    *) check_warn "Operating System: $OS_TYPE (untested)" ;;
esac

# Check architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64) check_pass "Architecture: $ARCH (supported)" ;;
    arm64|aarch64) check_pass "Architecture: $ARCH (supported)" ;;
    *) check_warn "Architecture: $ARCH (may have issues)" ;;
esac
echo ""

# 2. Required Commands
echo -e "${BOLD}[2/8] Checking Required Commands${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

for cmd in cargo npm curl; do
    if command -v "$cmd" &> /dev/null; then
        VERSION=$($cmd --version 2>&1 | head -1)
        check_pass "$cmd found: $VERSION"
    else
        check_fail "$cmd NOT FOUND - required for demo"
        echo "       Install: "
        case "$cmd" in
            cargo) echo "       curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" ;;
            npm) echo "       https://nodejs.org/" ;;
            curl) echo "       apt-get install curl (Linux) or brew install curl (macOS)" ;;
        esac
    fi
done
echo ""

# 3. Port Availability
echo -e "${BOLD}[3/8] Checking Port Availability${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

for port in 8080 5173; do
    if command -v lsof &> /dev/null; then
        if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
            PID=$(lsof -ti:$port)
            PROCESS=$(ps -p $PID -o comm= 2>/dev/null || echo "unknown")
            check_fail "Port $port IN USE by PID $PID ($PROCESS)"
            echo "       Fix: lsof -ti:$port | xargs kill -9"
        else
            check_pass "Port $port available"
        fi
    elif command -v netstat &> /dev/null; then
        if netstat -tuln 2>/dev/null | grep -q ":$port "; then
            check_warn "Port $port appears to be in use (netstat check)"
        else
            check_pass "Port $port available"
        fi
    else
        check_warn "Port $port (cannot verify - lsof/netstat not available)"
    fi
done
echo ""

# 4. Disk Space
echo -e "${BOLD}[4/8] Checking Disk Space${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if command -v df &> /dev/null; then
    FREE_SPACE=$(df -k . | awk 'NR==2 {print $4}')
    FREE_GB=$((FREE_SPACE / 1024 / 1024))

    echo "   Current free space: ${CYAN}${FREE_GB} GB${NC}"

    if [ $FREE_GB -ge 6 ]; then
        check_pass "Plenty of space (${FREE_GB}GB >= 6GB recommended)"
    elif [ $FREE_GB -ge 4 ]; then
        check_warn "Adequate space (${FREE_GB}GB >= 4GB minimum)"
        echo "       Recommend: 6GB+ for comfortable demo"
    elif [ $FREE_GB -ge 1 ]; then
        check_warn "Tight space (${FREE_GB}GB) - only for pre-built binaries"
        echo "       Recommend: Run 'cargo clean' to free space"
    else
        check_fail "Insufficient space (${FREE_GB}GB < 1GB)"
        echo "       Required: At least 4GB for full demo"
        echo "       Free space: cargo clean (saves ~3GB)"
    fi
else
    check_warn "Cannot check disk space (df not available)"
fi
echo ""

# 5. Existing Orbit Installation
echo -e "${BOLD}[5/8] Checking Existing Orbit Files${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -d "crates/orbit-web" ]; then
    check_pass "Orbit source code found"
else
    check_fail "Not in Orbit repository root"
    echo "       Run from: git repository root directory"
fi

if [ -f "target/release/orbit-server" ]; then
    SIZE=$(du -h target/release/orbit-server | cut -f1)
    check_pass "Pre-built binary found ($SIZE) - demo will be faster!"
else
    check_warn "No pre-built binary - demo will compile from source (slower)"
    echo "       Speed up: cd crates/orbit-web && cargo build --release --bin orbit-server"
fi

if [ -d "dashboard/node_modules" ]; then
    check_pass "Node modules already installed - demo will be faster!"
else
    check_warn "Node modules not installed - demo will run npm install (slower)"
    echo "       Speed up: cd dashboard && npm ci"
fi
echo ""

# 6. Running Processes
echo -e "${BOLD}[6/8] Checking for Running Orbit Processes${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if pgrep -f "orbit-server" > /dev/null; then
    check_warn "orbit-server process already running"
    echo "       Demo will attempt to start on same port (may conflict)"
    echo "       Fix: pkill -f orbit-server"
else
    check_pass "No orbit-server processes running"
fi

if pgrep -f "npm run dev" > /dev/null; then
    check_warn "npm dev server already running"
    echo "       May conflict with demo dashboard"
    echo "       Fix: pkill -f 'npm run dev'"
else
    check_pass "No npm dev server running"
fi
echo ""

# 7. Temporary Directory
echo -e "${BOLD}[7/8] Checking Temporary Directory${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -w "/tmp" ]; then
    check_pass "Temporary directory (/tmp) writable"

    # Check for old demo files
    OLD_FILES=$(find /tmp -maxdepth 1 -name "orbit_demo_*" 2>/dev/null | wc -l)
    if [ $OLD_FILES -gt 0 ]; then
        check_warn "Found $OLD_FILES old demo directory/directories in /tmp"
        echo "       Cleanup: rm -rf /tmp/orbit_demo_*"
    else
        check_pass "No leftover demo files in /tmp"
    fi
else
    check_fail "Temporary directory (/tmp) not writable"
fi
echo ""

# 8. Network Connectivity
echo -e "${BOLD}[8/8] Checking Network Connectivity${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if curl -s --max-time 5 https://www.google.com > /dev/null 2>&1; then
    check_pass "Internet connectivity available"
else
    check_warn "Internet connectivity issue (not required for demo)"
    echo "       Demo will work offline if dependencies are cached"
fi

if curl -s --max-time 2 http://localhost:8080/api/health > /dev/null 2>&1; then
    check_warn "Something already responding on http://localhost:8080"
    echo "       Demo may conflict with existing service"
else
    check_pass "localhost:8080 not responding (good)"
fi
echo ""

# Summary
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${BOLD}VALIDATION SUMMARY${NC}"
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "  ${GREEN}Checks Passed:${NC}  $CHECKS_PASSED"
echo -e "  ${YELLOW}Warnings:${NC}       $WARNINGS"
echo -e "  ${RED}Checks Failed:${NC}  $CHECKS_FAILED"
echo ""

# What will happen
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${BOLD}WHAT THE DEMO WILL DO${NC}"
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${CYAN}The demo will:${NC}"
echo "  1. Create temp files in /tmp/orbit_demo_source_* (~170MB)"
echo "  2. Create temp files in /tmp/orbit_demo_dest_* (~170MB)"
echo "  3. Start orbit-server on port 8080 (background process)"
echo "  4. Start npm dev server on port 5173 (background process)"
echo "  5. Create database at crates/orbit-web/magnetar.db (~5MB)"
echo "  6. Create logs: orbit-server.log, orbit-dashboard.log"
echo "  7. Open browser to http://localhost:5173"
echo ""
echo -e "${GREEN}The demo will NOT:${NC}"
echo "  ✗ Modify any of your code"
echo "  ✗ Delete any of your files (except temp files it creates)"
echo "  ✗ Change system settings or configuration"
echo "  ✗ Install any software"
echo "  ✗ Require sudo/admin privileges"
echo "  ✗ Make any permanent changes to your system"
echo ""

# Cleanup explanation
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${BOLD}CLEANUP & SAFETY${NC}"
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo "When you press Ctrl+C or the demo completes:"
echo "  ✓ All temp files are automatically deleted"
echo "  ✓ All processes are terminated"
echo "  ✓ Your system is restored to original state"
echo ""
echo "What remains after demo (safe to keep or delete):"
echo "  • orbit-server.log (application logs)"
echo "  • orbit-dashboard.log (application logs)"
echo "  • magnetar.db (demo job database, ~5MB)"
echo "  • demo-logs/ directory (orchestration logs)"
echo ""
echo "To clean up manually:"
echo "  rm -f orbit-*.log magnetar.db"
echo "  rm -rf demo-logs/"
echo ""

# Recommendation
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${BOLD}RECOMMENDATION${NC}"
echo -e "${BOLD}═══════════════════════════════════════════${NC}"

if [ $CHECKS_FAILED -eq 0 ]; then
    if [ $WARNINGS -eq 0 ]; then
        echo -e "${GREEN}✅ ALL CHECKS PASSED!${NC}"
        echo -e "${GREEN}Your system is ready for the demo.${NC}"
        echo ""
        echo "Next steps:"
        echo "  1. Save any unsaved work (demo is safe, but good practice)"
        echo "  2. Run: ./demo-orbit.sh"
        echo "  3. Press Ctrl+C anytime to stop and cleanup"
    else
        echo -e "${YELLOW}⚠️  READY WITH WARNINGS${NC}"
        echo "Your system can run the demo, but consider addressing warnings above."
        echo ""
        echo "Run demo now:"
        echo "  ./demo-orbit.sh"
        echo ""
        echo "Or fix warnings first (recommended):"
        for warn in $(seq 1 $WARNINGS); do
            echo "  • Review warnings above"
        done
    fi
else
    echo -e "${RED}❌ ISSUES FOUND${NC}"
    echo "Please address the failed checks above before running the demo."
    echo ""
    echo "Common fixes:"
    echo "  • Install missing commands (cargo, npm, curl)"
    echo "  • Free up disk space (cargo clean)"
    echo "  • Kill processes using required ports"
    echo ""
    echo "Re-run this validator after fixes:"
    echo "  ./scripts/validate-demo-safety.sh"
fi
echo ""

# Dry-run option
echo -e "${CYAN}💡 TIP: Want to see what the demo does step-by-step?${NC}"
echo -e "   Run in dry-run mode: ${YELLOW}ORBIT_DEMO_DRY_RUN=true ./demo-orbit.sh${NC}"
echo ""

# Exit code
if [ $CHECKS_FAILED -gt 0 ]; then
    exit 1
else
    exit 0
fi
