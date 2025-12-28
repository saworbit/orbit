#!/bin/bash
set -e

# ==============================================================================
#  🛰️  ORBIT E2E DEMONSTRATION HARNESS
#  Scenario: Deep Space Telemetry Ingestion
#  Version: 2.2.0-alpha
# ==============================================================================

# Configuration
ORBIT_ROOT="$(pwd)"
DEMO_SOURCE="/tmp/orbit_demo_source_$(date +%s)"
DEMO_DEST="/tmp/orbit_demo_dest_$(date +%s)"
API_URL="http://localhost:8080"
DASHBOARD_URL="http://localhost:5173"

# Logging configuration
DEBUG_MODE="${ORBIT_DEMO_DEBUG:-false}"
LOG_DIR="$ORBIT_ROOT/demo-logs"
DEMO_LOG="$LOG_DIR/demo-run-$(date +%Y%m%d-%H%M%S).log"
ERROR_LOG="$LOG_DIR/demo-errors-$(date +%Y%m%d-%H%M%S).log"

# Create log directory
mkdir -p "$LOG_DIR"

# Logging functions
log_event() {
    local level=$1
    shift
    local message="$@"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$DEMO_LOG"
    if [ "$DEBUG_MODE" = "true" ]; then
        echo "[$timestamp] [$level] $message" >&2
    fi
}

log_error() {
    local message="$@"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [ERROR] $message" | tee -a "$DEMO_LOG" "$ERROR_LOG" >&2
}

log_debug() {
    if [ "$DEBUG_MODE" = "true" ]; then
        local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        echo "[$timestamp] [DEBUG] $@" | tee -a "$DEMO_LOG" >&2
    fi
}

# Branding
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       🛰️  ORBIT DEMO ORCHESTRATOR         ║${NC}"
echo -e "${BLUE}║     Scenario: Deep Space Telemetry Sync    ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"

log_event "INFO" "Demo orchestrator started"
log_event "INFO" "Log file: $DEMO_LOG"
log_event "INFO" "Error log: $ERROR_LOG"
log_event "INFO" "Debug mode: $DEBUG_MODE"

if [ "$DEBUG_MODE" = "true" ]; then
    echo -e "${YELLOW}🐛 DEBUG MODE ENABLED${NC}"
    echo -e "   Logs: $DEMO_LOG"
fi

# Cleanup Trap (defined early so it's always available)
cleanup() {
    local exit_code=$?
    echo -e "\n${YELLOW}[6/6] Initiating Orbital Decay (Cleanup)...${NC}"
    log_event "INFO" "Cleanup initiated (exit code: $exit_code)"

    if [ ! -z "$SERVER_PID" ]; then
        log_debug "Stopping server (PID: $SERVER_PID)"
        kill $SERVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$UI_PID" ]; then
        log_debug "Stopping dashboard (PID: $UI_PID)"
        kill $UI_PID 2>/dev/null || true
    fi
    if [ -d "$DEMO_SOURCE" ]; then
        log_debug "Removing demo source: $DEMO_SOURCE"
        rm -rf "$DEMO_SOURCE"
    fi
    if [ -d "$DEMO_DEST" ]; then
        log_debug "Removing demo destination: $DEMO_DEST"
        rm -rf "$DEMO_DEST"
    fi

    echo -e "${GREEN}✓ Systems Offline. Data purged.${NC}"
    log_event "INFO" "Cleanup complete"

    if [ $exit_code -ne 0 ]; then
        echo -e "\n${RED}╔════════════════════════════════════════════╗${NC}"
        echo -e "${RED}║          DEMO FAILED - ERROR LOGS          ║${NC}"
        echo -e "${RED}╚════════════════════════════════════════════╝${NC}"
        echo -e "${YELLOW}Check the following logs for details:${NC}"
        echo -e "  📄 Demo log:       ${CYAN}$DEMO_LOG${NC}"
        echo -e "  ❌ Error log:      ${CYAN}$ERROR_LOG${NC}"
        echo -e "  🔧 Server log:     ${CYAN}$ORBIT_ROOT/orbit-server.log${NC}"
        echo -e "  🎨 Dashboard log:  ${CYAN}$ORBIT_ROOT/orbit-dashboard.log${NC}"
        echo -e ""
        echo -e "${YELLOW}Quick diagnostics:${NC}"
        echo -e "  View errors:       ${CYAN}cat $ERROR_LOG${NC}"
        echo -e "  View server:       ${CYAN}tail -50 orbit-server.log${NC}"
        echo -e "  View dashboard:    ${CYAN}tail -50 orbit-dashboard.log${NC}"
        echo -e "  Analyze logs:      ${CYAN}./scripts/analyze-logs.sh${NC}"
        echo -e ""
    else
        echo -e "\n${GREEN}✅ Demo completed successfully!${NC}"
        echo -e "📄 Logs available at: ${CYAN}$DEMO_LOG${NC}"
    fi
}
trap cleanup EXIT

# 1. Pre-flight Checks
echo -e "\n${YELLOW}[1/6] Initiating Pre-flight Systems Check...${NC}"

check_cmd() {
    log_debug "Checking for command: $1"
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}❌ Critical Error: '$1' is not installed.${NC}"
        log_error "Required command not found: $1"
        log_error "Installation instructions:"
        case "$1" in
            cargo)
                log_error "  Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
                ;;
            npm)
                log_error "  Install Node.js: https://nodejs.org/"
                ;;
            curl)
                log_error "  Install curl: apt-get install curl (Ubuntu) or brew install curl (macOS)"
                ;;
        esac
        exit 1
    fi
    echo -e "${GREEN}✓ Found $1${NC}"
    log_debug "Found $1 at: $(which $1)"
}

check_port() {
    if command -v lsof &> /dev/null; then
        if lsof -Pi :$1 -sTCP:LISTEN -t >/dev/null 2>&1; then
            echo -e "${RED}❌ Critical Error: Port $1 is already in use.${NC}"
            exit 1
        fi
    elif command -v netstat &> /dev/null; then
        if netstat -tuln | grep -q ":$1 "; then
            echo -e "${RED}❌ Critical Error: Port $1 is already in use.${NC}"
            exit 1
        fi
    else
        echo -e "${YELLOW}⚠ Warning: Cannot check port $1 (lsof/netstat not available)${NC}"
    fi
}

check_cmd "cargo"
check_cmd "npm"
check_cmd "curl"
check_port 8080
check_port 5173

# 2. Data Fabrication
echo -e "\n${YELLOW}[2/6] Fabricating Synthetic Telemetry Data...${NC}"
mkdir -p "$DEMO_SOURCE"
mkdir -p "$DEMO_DEST"

echo "   Generating binary blobs..."
# Create files of varying sizes to demonstrate progress bars
if command -v dd &> /dev/null; then
    dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_alpha.bin" bs=1M count=50 status=none 2>/dev/null || \
        dd if=/dev/zero of="$DEMO_SOURCE/telemetry_alpha.bin" bs=1M count=50 2>/dev/null
    dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_beta.bin" bs=1M count=20 status=none 2>/dev/null || \
        dd if=/dev/zero of="$DEMO_SOURCE/telemetry_beta.bin" bs=1M count=20 2>/dev/null
    dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_gamma.bin" bs=1M count=100 status=none 2>/dev/null || \
        dd if=/dev/zero of="$DEMO_SOURCE/telemetry_gamma.bin" bs=1M count=100 2>/dev/null
else
    # Fallback for systems without dd
    head -c 52428800 /dev/urandom > "$DEMO_SOURCE/telemetry_alpha.bin" 2>/dev/null || \
        head -c 52428800 /dev/zero > "$DEMO_SOURCE/telemetry_alpha.bin"
    head -c 20971520 /dev/urandom > "$DEMO_SOURCE/telemetry_beta.bin" 2>/dev/null || \
        head -c 20971520 /dev/zero > "$DEMO_SOURCE/telemetry_beta.bin"
    head -c 104857600 /dev/urandom > "$DEMO_SOURCE/telemetry_gamma.bin" 2>/dev/null || \
        head -c 104857600 /dev/zero > "$DEMO_SOURCE/telemetry_gamma.bin"
fi

# Create simulated logs
echo "   Generating flight logs..."
for i in {1..20}; do
    echo "$(date '+%Y-%m-%d %H:%M:%S') [TELEMETRY] Sensor reading #$i: Temperature $(($RANDOM % 100))°C, Radiation $(($RANDOM % 1000)) mSv" > "$DEMO_SOURCE/flight_log_$i.log"
done

# Create a manifest file
cat > "$DEMO_SOURCE/mission_manifest.json" << EOF
{
  "mission_id": "DEEP_SPACE_001",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "telescope": "Hubble-Successor",
  "data_type": "telemetry",
  "total_files": 23,
  "estimated_size_mb": 170
}
EOF

echo -e "${GREEN}✓ Created synthetic dataset at $DEMO_SOURCE${NC}"
echo -e "   Total files: $(ls -1 "$DEMO_SOURCE" | wc -l)"
echo -e "   Total size: $(du -sh "$DEMO_SOURCE" | cut -f1)"

# 3. System Ignition
echo -e "\n${YELLOW}[3/6] Igniting Orbit Core Systems...${NC}"

# Set Dev Secret (required for JWT authentication)
export ORBIT_JWT_SECRET="demo-secret-key-must-be-32-chars-long"

# Start Backend
echo -e "   → Launching Control Plane (Magnetar)..."
cd "$ORBIT_ROOT/crates/orbit-web"
RUST_LOG=info cargo run --quiet --bin orbit-server > "$ORBIT_ROOT/orbit-server.log" 2>&1 &
SERVER_PID=$!
cd "$ORBIT_ROOT"

# Start Frontend
echo -e "   → Launching Dashboard..."
cd "$ORBIT_ROOT/dashboard"
npm run dev -- --host 0.0.0.0 > "$ORBIT_ROOT/orbit-dashboard.log" 2>&1 &
UI_PID=$!
cd "$ORBIT_ROOT"

# Wait for Health Check
echo -e "   → Waiting for API stability..."
MAX_RETRIES=60
COUNT=0
while ! curl -s -f "$API_URL/api/health" > /dev/null 2>&1; do
    sleep 1
    echo -n "."
    COUNT=$((COUNT+1))
    if [ $COUNT -ge $MAX_RETRIES ]; then
        echo -e "\n${RED}❌ Timeout waiting for API to become healthy!${NC}"
        log_error "API health check timeout after ${MAX_RETRIES}s"
        log_error "Server PID: $SERVER_PID (running: $(ps -p $SERVER_PID > /dev/null && echo 'yes' || echo 'no'))"
        log_error "Dashboard PID: $UI_PID (running: $(ps -p $UI_PID > /dev/null && echo 'yes' || echo 'no'))"

        echo -e "${RED}Diagnostic information:${NC}"
        echo -e "  Server process: $(ps -p $SERVER_PID > /dev/null && echo '✓ Running' || echo '✗ Not running')"
        echo -e "  Dashboard process: $(ps -p $UI_PID > /dev/null && echo '✓ Running' || echo '✗ Not running')"

        if [ -f "$ORBIT_ROOT/orbit-server.log" ]; then
            echo -e "\n${YELLOW}Last 20 lines of server log:${NC}"
            tail -20 "$ORBIT_ROOT/orbit-server.log" | tee -a "$ERROR_LOG"
        fi

        if [ -f "$ORBIT_ROOT/orbit-dashboard.log" ]; then
            echo -e "\n${YELLOW}Last 20 lines of dashboard log:${NC}"
            tail -20 "$ORBIT_ROOT/orbit-dashboard.log" | tee -a "$ERROR_LOG"
        fi

        exit 1
    fi
done
echo -e "\n${GREEN}✓ Control Plane is Online.${NC}"

# Brief pause to ensure dashboard is ready
sleep 2

# 4. User Interaction & Scenario Execution
echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                  READY FOR LAUNCH                          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo -e ""
echo -e "   ${BOLD}Dashboard:${NC} ${CYAN}$DASHBOARD_URL${NC}"
echo -e "   ${BOLD}API Docs:${NC}  ${CYAN}$API_URL/swagger-ui${NC}"
echo -e ""
echo -e "${CYAN}Please open your browser to the Dashboard URL above.${NC}"
echo -e ""
read -p "Press [ENTER] to execute the Telemetry Ingestion Job..."

echo -e "\n${YELLOW}[4/6] Injecting Job via Magnetar API...${NC}"

# Construct JSON payload
JOB_PAYLOAD=$(cat <<EOF
{
  "source": "$DEMO_SOURCE",
  "destination": "$DEMO_DEST",
  "compress": true,
  "verify": true,
  "parallel_workers": 4
}
EOF
)

echo -e "   → Creating job with payload:"
echo -e "${CYAN}$JOB_PAYLOAD${NC}"

# Create the job
RESPONSE=$(curl -s -X POST "$API_URL/api/create_job" \
  -H "Content-Type: application/json" \
  -d "$JOB_PAYLOAD")

# Check if response is a valid job ID (should be a number)
if [[ "$RESPONSE" =~ ^[0-9]+$ ]]; then
    JOB_ID=$RESPONSE
    echo -e "${GREEN}✓ Job Created! Job ID: $JOB_ID${NC}"

    # Start the job (jobs are created in 'pending' state)
    echo -e "   → Starting job execution..."
    RUN_RESPONSE=$(curl -s -X POST "$API_URL/api/run_job" \
      -H "Content-Type: application/json" \
      -d "{\"job_id\": $JOB_ID}")

    echo -e "${GREEN}✓ Job Started! Response: $RUN_RESPONSE${NC}"
else
    echo -e "${RED}❌ Failed to create job. Response: $RESPONSE${NC}"
    exit 1
fi

# 5. Observation Phase
echo -e "\n${YELLOW}[5/6] Observation Phase...${NC}"
echo -e ""
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo -e "  ${CYAN}Watch the dashboard for live updates:${NC}"
echo -e "  • ${GREEN}Visual Chunk Map${NC} - Real-time transfer progress"
echo -e "  • ${GREEN}Live Telemetry${NC} - Transfer speed and statistics"
echo -e "  • ${GREEN}Job Status${NC} - Current state of the transfer"
echo -e ""
echo -e "  ${BOLD}Processing Details:${NC}"
echo -e "  • Source: $DEMO_SOURCE"
echo -e "  • Destination: $DEMO_DEST"
echo -e "  • Data Volume: ~170 MB"
echo -e "  • Verification: Enabled (checksum validation)"
echo -e "  • Compression: Enabled"
echo -e "  • Parallel Workers: 4"
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo -e ""
read -p "Press [ENTER] to terminate the demo and cleanup..."

# Cleanup happens automatically via trap
echo -e "\n${GREEN}Demo complete! Thank you for experiencing Orbit.${NC}"
