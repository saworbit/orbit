#!/bin/bash

# ==============================================================================
#  Orbit Demo Log Analyzer
#  Analyzes demo logs for errors, warnings, and diagnostic information
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

ORBIT_ROOT="${ORBIT_ROOT:-$(pwd)}"
LOG_DIR="$ORBIT_ROOT/demo-logs"

echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       📊 ORBIT LOG ANALYZER               ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo ""

# Find latest logs
LATEST_DEMO_LOG=$(ls -t $LOG_DIR/demo-run-*.log 2>/dev/null | head -1)
LATEST_ERROR_LOG=$(ls -t $LOG_DIR/demo-errors-*.log 2>/dev/null | head -1)
SERVER_LOG="$ORBIT_ROOT/orbit-server.log"
DASHBOARD_LOG="$ORBIT_ROOT/orbit-dashboard.log"

if [ -z "$LATEST_DEMO_LOG" ]; then
    echo -e "${YELLOW}No demo logs found in $LOG_DIR${NC}"
    exit 1
fi

echo -e "${CYAN}Analyzing logs...${NC}"
echo -e "  Demo log:      $LATEST_DEMO_LOG"
echo -e "  Error log:     ${LATEST_ERROR_LOG:-N/A}"
echo -e "  Server log:    ${SERVER_LOG:-N/A}"
echo -e "  Dashboard log: ${DASHBOARD_LOG:-N/A}"
echo ""

# Statistics
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${BOLD}LOG STATISTICS${NC}"
echo -e "${BOLD}═══════════════════════════════════════════${NC}"

if [ -f "$LATEST_DEMO_LOG" ]; then
    TOTAL_LINES=$(wc -l < "$LATEST_DEMO_LOG")
    ERROR_COUNT=$(grep -c "\[ERROR\]" "$LATEST_DEMO_LOG" || echo "0")
    WARN_COUNT=$(grep -c "\[WARN\]" "$LATEST_DEMO_LOG" || echo "0")
    INFO_COUNT=$(grep -c "\[INFO\]" "$LATEST_DEMO_LOG" || echo "0")

    echo -e "Total lines:    $TOTAL_LINES"
    echo -e "Errors:         ${RED}$ERROR_COUNT${NC}"
    echo -e "Warnings:       ${YELLOW}$WARN_COUNT${NC}"
    echo -e "Info messages:  ${GREEN}$INFO_COUNT${NC}"
fi
echo ""

# Show errors
if [ -f "$LATEST_ERROR_LOG" ] && [ -s "$LATEST_ERROR_LOG" ]; then
    echo -e "${BOLD}═══════════════════════════════════════════${NC}"
    echo -e "${BOLD}ERRORS FOUND${NC}"
    echo -e "${BOLD}═══════════════════════════════════════════${NC}"
    cat "$LATEST_ERROR_LOG"
    echo ""
fi

# Show warnings
if [ -f "$LATEST_DEMO_LOG" ]; then
    WARNINGS=$(grep "\[WARN\]" "$LATEST_DEMO_LOG" || true)
    if [ ! -z "$WARNINGS" ]; then
        echo -e "${BOLD}═══════════════════════════════════════════${NC}"
        echo -e "${BOLD}WARNINGS${NC}"
        echo -e "${BOLD}═══════════════════════════════════════════${NC}"
        echo "$WARNINGS"
        echo ""
    fi
fi

# Check for common issues
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${BOLD}DIAGNOSTIC CHECKS${NC}"
echo -e "${BOLD}═══════════════════════════════════════════${NC}"

# Port conflicts
if grep -q "Address already in use" "$SERVER_LOG" 2>/dev/null; then
    echo -e "${RED}✗ Port conflict detected${NC}"
    echo -e "  Issue: API port 8080 is already in use"
    echo -e "  Fix: Kill process using port: lsof -ti:8080 | xargs kill -9"
else
    echo -e "${GREEN}✓ No port conflicts${NC}"
fi

# Build failures
if grep -qi "error.*Compiling\|failed to compile" "$SERVER_LOG" 2>/dev/null; then
    echo -e "${RED}✗ Rust compilation errors${NC}"
    echo -e "  Check: $SERVER_LOG"
    grep -i "error.*Compiling\|failed to compile" "$SERVER_LOG" | head -5
else
    echo -e "${GREEN}✓ No compilation errors${NC}"
fi

# NPM issues
if grep -qi "npm ERR!" "$DASHBOARD_LOG" 2>/dev/null; then
    echo -e "${RED}✗ NPM errors detected${NC}"
    echo -e "  Check: $DASHBOARD_LOG"
    grep "npm ERR!" "$DASHBOARD_LOG" | head -5
else
    echo -e "${GREEN}✓ No NPM errors${NC}"
fi

# Health check issues
if grep -q "Timeout waiting for API" "$LATEST_DEMO_LOG" 2>/dev/null; then
    echo -e "${RED}✗ API health check timeout${NC}"
    echo -e "  Likely causes:"
    echo -e "    1. Server failed to start (check orbit-server.log)"
    echo -e "    2. Port 8080 blocked by firewall"
    echo -e "    3. Database initialization failed"
else
    echo -e "${GREEN}✓ API health check passed${NC}"
fi

# Job creation issues
if grep -q "Failed to create job" "$LATEST_DEMO_LOG" 2>/dev/null; then
    echo -e "${RED}✗ Job creation failed${NC}"
    echo -e "  Check JWT secret and API connectivity"
else
    echo -e "${GREEN}✓ Job creation successful${NC}"
fi

echo ""

# Timeline
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${BOLD}EVENT TIMELINE${NC}"
echo -e "${BOLD}═══════════════════════════════════════════${NC}"

if [ -f "$LATEST_DEMO_LOG" ]; then
    grep "\[INFO\]" "$LATEST_DEMO_LOG" | tail -20
fi
echo ""

# Recommendations
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${BOLD}RECOMMENDATIONS${NC}"
echo -e "${BOLD}═══════════════════════════════════════════${NC}"

if [ "$ERROR_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}Errors detected. Recommended actions:${NC}"
    echo -e "  1. Review error log: cat $LATEST_ERROR_LOG"
    echo -e "  2. Check server log: tail -50 $SERVER_LOG"
    echo -e "  3. Enable debug mode: ORBIT_DEMO_DEBUG=true ./demo-orbit.sh"
    echo -e "  4. Verify prerequisites: cargo --version && npm --version"
elif [ "$WARN_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}Warnings detected. Consider reviewing:${NC}"
    echo -e "  - Full log: cat $LATEST_DEMO_LOG"
else
    echo -e "${GREEN}No issues detected! Demo ran successfully.${NC}"
fi

echo ""
echo -e "${CYAN}For detailed analysis:${NC}"
echo -e "  View full demo log:      cat $LATEST_DEMO_LOG"
echo -e "  Search for term:         grep 'term' $LATEST_DEMO_LOG"
echo -e "  View last 50 lines:      tail -50 $LATEST_DEMO_LOG"
echo -e "  Enable debug mode:       ORBIT_DEMO_DEBUG=true ./demo-orbit.sh"
echo ""
