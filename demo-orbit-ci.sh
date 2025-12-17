#!/bin/bash
set -e

# ==============================================================================
#  🛰️  ORBIT E2E CI/CD HARNESS (Headless Mode)
#  Scenario: Deep Space Telemetry Ingestion
#  Version: 2.2.0-alpha
#  Purpose: Non-interactive automated testing for CI/CD pipelines
# ==============================================================================

# Configuration
ORBIT_ROOT="$(pwd)"
DEMO_SOURCE="/tmp/orbit_demo_source_$(date +%s)"
DEMO_DEST="/tmp/orbit_demo_dest_$(date +%s)"
API_URL="http://localhost:8080"
DASHBOARD_URL="http://localhost:5173"
METRICS_FILE="$ORBIT_ROOT/e2e-metrics.json"

# Start timestamps for metrics
START_TIME=$(date +%s)

# Colors (safe for CI logs)
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${CYAN}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

log_info "╔════════════════════════════════════════════╗"
log_info "║  ORBIT E2E CI/CD HARNESS (HEADLESS MODE)  ║"
log_info "║   Scenario: Deep Space Telemetry Sync      ║"
log_info "╚════════════════════════════════════════════╝"

# Initialize metrics
METRICS_DATA="{}"

add_metric() {
    local key=$1
    local value=$2
    METRICS_DATA=$(echo "$METRICS_DATA" | jq --arg k "$key" --arg v "$value" '.[$k] = $v')
}

# Cleanup Trap
cleanup() {
    log_warn "Cleanup initiated..."

    if [ ! -z "$SERVER_PID" ] && ps -p $SERVER_PID > /dev/null 2>&1; then
        log_info "Stopping server (PID: $SERVER_PID)"
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi

    if [ ! -z "$UI_PID" ] && ps -p $UI_PID > /dev/null 2>&1; then
        log_info "Stopping dashboard (PID: $UI_PID)"
        kill $UI_PID 2>/dev/null || true
        wait $UI_PID 2>/dev/null || true
    fi

    if [ -d "$DEMO_SOURCE" ]; then
        rm -rf "$DEMO_SOURCE"
    fi

    if [ -d "$DEMO_DEST" ]; then
        rm -rf "$DEMO_DEST"
    fi

    # Write metrics
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    add_metric "total_duration_seconds" "$DURATION"
    add_metric "timestamp" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"

    echo "$METRICS_DATA" | jq '.' > "$METRICS_FILE"
    log_success "Metrics written to $METRICS_FILE"

    log_success "Cleanup complete"
}
trap cleanup EXIT

# 1. Pre-flight Checks
log_info "[1/6] Pre-flight Systems Check..."
PREFLIGHT_START=$(date +%s)

check_cmd() {
    if ! command -v "$1" &> /dev/null; then
        log_error "'$1' is not installed"
        exit 1
    fi
    log_success "Found $1"
}

check_cmd "cargo"
check_cmd "npm"
check_cmd "curl"
check_cmd "jq"

PREFLIGHT_END=$(date +%s)
add_metric "preflight_duration_seconds" "$((PREFLIGHT_END - PREFLIGHT_START))"

# 2. Data Fabrication
log_info "[2/6] Fabricating Synthetic Telemetry Data..."
DATA_START=$(date +%s)

mkdir -p "$DEMO_SOURCE"
mkdir -p "$DEMO_DEST"

# Create test files (smaller for CI speed)
dd if=/dev/zero of="$DEMO_SOURCE/telemetry_alpha.bin" bs=1M count=10 2>/dev/null
dd if=/dev/zero of="$DEMO_SOURCE/telemetry_beta.bin" bs=1M count=5 2>/dev/null
dd if=/dev/zero of="$DEMO_SOURCE/telemetry_gamma.bin" bs=1M count=20 2>/dev/null

for i in {1..10}; do
    echo "$(date '+%Y-%m-%d %H:%M:%S') [TELEMETRY] Sensor $i: Temp=$(($RANDOM % 100))°C" > "$DEMO_SOURCE/flight_log_$i.log"
done

cat > "$DEMO_SOURCE/mission_manifest.json" << EOF
{
  "mission_id": "CI_TEST_$(date +%s)",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "ci_mode": true
}
EOF

TOTAL_FILES=$(ls -1 "$DEMO_SOURCE" | wc -l)
TOTAL_SIZE=$(du -sb "$DEMO_SOURCE" | cut -f1)

add_metric "test_files_count" "$TOTAL_FILES"
add_metric "test_data_bytes" "$TOTAL_SIZE"

DATA_END=$(date +%s)
add_metric "data_fabrication_duration_seconds" "$((DATA_END - DATA_START))"

log_success "Created $TOTAL_FILES files ($TOTAL_SIZE bytes)"

# 3. System Ignition
log_info "[3/6] Igniting Orbit Core Systems..."
IGNITION_START=$(date +%s)

export ORBIT_JWT_SECRET="${ORBIT_JWT_SECRET:-ci-test-secret-key-must-be-32-chars}"

# Start Backend (using pre-built binary if available)
if [ -f "target/release/orbit-server" ]; then
    log_info "Using pre-built binary"
    cd "$ORBIT_ROOT/crates/orbit-web"
    RUST_LOG=info ../../target/release/orbit-server > "$ORBIT_ROOT/orbit-server.log" 2>&1 &
    SERVER_PID=$!
    cd "$ORBIT_ROOT"
else
    log_info "Building and running from source"
    cd "$ORBIT_ROOT/crates/orbit-web"
    RUST_LOG=info cargo run --quiet --bin orbit-server > "$ORBIT_ROOT/orbit-server.log" 2>&1 &
    SERVER_PID=$!
    cd "$ORBIT_ROOT"
fi

# Start Frontend
cd "$ORBIT_ROOT/dashboard"
npm run dev -- --host 0.0.0.0 > "$ORBIT_ROOT/orbit-dashboard.log" 2>&1 &
UI_PID=$!
cd "$ORBIT_ROOT"

log_info "Server PID: $SERVER_PID, Dashboard PID: $UI_PID"

# Wait for Health Check
log_info "Waiting for API to become healthy..."
MAX_RETRIES=60
COUNT=0
HEALTH_CHECK_START=$(date +%s)

while ! curl -s -f "$API_URL/api/health" > /dev/null 2>&1; do
    sleep 1
    COUNT=$((COUNT+1))
    if [ $COUNT -ge $MAX_RETRIES ]; then
        log_error "Timeout waiting for API"
        log_error "Server log:"
        tail -50 "$ORBIT_ROOT/orbit-server.log" || echo "No server log available"
        exit 1
    fi
    if [ $((COUNT % 10)) -eq 0 ]; then
        log_info "Still waiting... ($COUNT/$MAX_RETRIES)"
    fi
done

HEALTH_CHECK_END=$(date +%s)
add_metric "health_check_duration_seconds" "$((HEALTH_CHECK_END - HEALTH_CHECK_START))"

log_success "Control Plane is online"

IGNITION_END=$(date +%s)
add_metric "ignition_duration_seconds" "$((IGNITION_END - IGNITION_START))"

# Wait for dashboard
sleep 3

# 4. Job Injection
log_info "[4/6] Injecting Job via Magnetar API..."
JOB_START=$(date +%s)

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

log_info "Creating job..."
RESPONSE=$(curl -s -X POST "$API_URL/api/create_job" \
  -H "Content-Type: application/json" \
  -d "$JOB_PAYLOAD")

if [[ "$RESPONSE" =~ ^[0-9]+$ ]]; then
    JOB_ID=$RESPONSE
    add_metric "job_id" "$JOB_ID"
    log_success "Job created: ID=$JOB_ID"

    # Start the job
    log_info "Starting job execution..."
    RUN_RESPONSE=$(curl -s -X POST "$API_URL/api/run_job" \
      -H "Content-Type: application/json" \
      -d "{\"job_id\": $JOB_ID}")

    log_success "Job started: $RUN_RESPONSE"
else
    log_error "Failed to create job: $RESPONSE"
    exit 1
fi

JOB_END=$(date +%s)
add_metric "job_creation_duration_seconds" "$((JOB_END - JOB_START))"

# 5. Observation Phase (Automated)
log_info "[5/6] Monitoring Job Progress..."
MONITOR_START=$(date +%s)

# Poll job status until completion
MAX_WAIT=120  # 2 minutes max
ELAPSED=0

while [ $ELAPSED -lt $MAX_WAIT ]; do
    sleep 2
    ELAPSED=$((ELAPSED + 2))

    # Get job status
    JOB_STATUS=$(curl -s "$API_URL/api/jobs/$JOB_ID" 2>/dev/null || echo "{}")
    STATUS=$(echo "$JOB_STATUS" | jq -r '.status' 2>/dev/null || echo "unknown")
    PROGRESS=$(echo "$JOB_STATUS" | jq -r '.progress' 2>/dev/null || echo "0")

    log_info "Status: $STATUS, Progress: $PROGRESS%"

    if [ "$STATUS" = "completed" ]; then
        log_success "Job completed successfully!"
        add_metric "job_status" "completed"
        add_metric "job_progress" "$PROGRESS"
        break
    elif [ "$STATUS" = "failed" ] || [ "$STATUS" = "cancelled" ]; then
        log_error "Job failed with status: $STATUS"
        add_metric "job_status" "$STATUS"
        exit 1
    fi
done

if [ $ELAPSED -ge $MAX_WAIT ]; then
    log_error "Job did not complete within timeout"
    add_metric "job_status" "timeout"
    exit 1
fi

MONITOR_END=$(date +%s)
add_metric "job_monitoring_duration_seconds" "$((MONITOR_END - MONITOR_START))"

# Verify destination
DEST_FILES=$(ls -1 "$DEMO_DEST" 2>/dev/null | wc -l)
add_metric "destination_files_count" "$DEST_FILES"

if [ "$DEST_FILES" -eq "$TOTAL_FILES" ]; then
    log_success "All files transferred successfully ($DEST_FILES/$TOTAL_FILES)"
    add_metric "transfer_success" "true"
else
    log_error "File count mismatch: Expected $TOTAL_FILES, got $DEST_FILES"
    add_metric "transfer_success" "false"
    exit 1
fi

# 6. Final validation
log_info "[6/6] Validation Complete"

log_success "════════════════════════════════════════"
log_success "  E2E Demo Test: PASSED"
log_success "  Job ID: $JOB_ID"
log_success "  Files Transferred: $DEST_FILES"
log_success "  Total Duration: $(($(date +%s) - START_TIME))s"
log_success "════════════════════════════════════════"

exit 0
