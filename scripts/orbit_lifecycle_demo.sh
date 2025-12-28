#!/bin/bash
set -u

# ==============================================================================
# Orbit Data Lifecycle Demonstration v3.0
# ==============================================================================
# "Trust, but Verify."
# This protocol generates a complex data topology, replicates it, mutates it,
# synchronizes it, and cryptographically audits the results.
#
# Documentation: docs/guides/TESTING_SCRIPTS_GUIDE.md
# ==============================================================================

# --- Configuration ---
WORKSPACE="./orbit_lifecycle_lab"
SRC_DIR="$WORKSPACE/sector_alpha"
DST_DIR="$WORKSPACE/sector_beta"
BINARY_PATH="./target/release/orbit"
LOG_FILE="$WORKSPACE/mission.log"
REQUIRED_SPACE_MB=500

# --- Styling & Telemetry ---
BOLD='\033[1m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

orbit_log() { echo -e "${CYAN}[ORBIT]${NC} $1" | tee -a "$LOG_FILE"; }
orbit_success() { echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"; }
orbit_warn() { echo -e "${YELLOW}[ATTENTION]${NC} $1" | tee -a "$LOG_FILE"; }
orbit_error() { echo -e "${RED}[CRITICAL]${NC} $1" | tee -a "$LOG_FILE"; }

# --- Interactive Guidance ---
pause_for_observation() {
    echo -e "\n${YELLOW}>>> OBSERVATION POINT: $1${NC}"
    echo -e "${BOLD}Action required:${NC} $2"
    echo -e "${CYAN}--> Press [ENTER] when ready to proceed...${NC}"
    read -r
}

# --- Safety & Cleanup ---
cleanup() {
    echo ""
    orbit_log "Decommissioning Simulation Environment..."
    if [ -d "$WORKSPACE" ]; then
        rm -rf "$WORKSPACE"
        orbit_success "Workspace decommissioned."
    fi
    echo -e "${BOLD}Lifecycle Protocol Complete.${NC}"
}
trap cleanup EXIT

check_resources() {
    orbit_log "Analyzing host resources..."
    local avail_mb=$(df -k . | awk 'NR==2 {print $4}')
    avail_mb=$((avail_mb / 1024))
    if [ "$avail_mb" -lt "$REQUIRED_SPACE_MB" ]; then
        orbit_error "Insufficient storage. Available: ${avail_mb}MB. Required: ${REQUIRED_SPACE_MB}MB."
        exit 1
    fi
    orbit_success "Storage confirmed."
}

# ==============================================================================
# EXECUTION FLOW
# ==============================================================================

# 1. Initialization
mkdir -p "$WORKSPACE"
touch "$LOG_FILE"
echo -e "${BOLD}Initializing Orbit Lifecycle Protocol...${NC}"
check_resources

if [ ! -f "$BINARY_PATH" ]; then
    orbit_log "Compiling Orbit Engine (Release Mode)..."
    cargo build --release >> "$LOG_FILE" 2>&1
    if [ $? -ne 0 ]; then orbit_error "Build failed."; exit 1; fi
fi

# 2. Topology Generation
orbit_log "Generating Source Data Topology..."
mkdir -p "$SRC_DIR/logs/archive"
mkdir -p "$SRC_DIR/images/raw"
mkdir -p "$SRC_DIR/db/shards"

# Text Data
echo "Orbit Configuration v1.0" > "$SRC_DIR/config.json"
for i in {1..5}; do echo "Log entry $i" > "$SRC_DIR/logs/archive/log_$i.txt"; done

# Binary Data (Entropy Simulation)
orbit_log "Synthesizing binary payloads (Entropy simulation)..."
dd if=/dev/urandom of="$SRC_DIR/images/raw/texture_map.bin" bs=1M count=5 status=none
dd if=/dev/urandom of="$SRC_DIR/db/shards/primary.db" bs=1024 count=1024 status=none

orbit_success "Data Generation Complete."

# 3. Observation: The "Before" State
pause_for_observation "Source Topology" \
    "Open a new terminal. Navigate to '$SRC_DIR'.
   Observe the nested folder structure and file types."

# 4. Replication (Copy)
orbit_log "Engaging Replication Engine (Mode: COPY)..."
START=$(date +%s%N)
$BINARY_PATH -s "$SRC_DIR" -d "$DST_DIR" -R -m copy >> "$LOG_FILE" 2>&1
EXIT_CODE=$?
END=$(date +%s%N)

if [ $EXIT_CODE -ne 0 ]; then orbit_error "Replication failed."; exit 1; fi
DURATION=$(( (END - START) / 1000000 ))
orbit_success "Replication concluded in ${DURATION}ms."

# 5. Observation: The "Replicated" State
pause_for_observation "Replication Verification" \
    "Check '$DST_DIR'.
   Verify that the folder structure mirrors the Source exactly."

# 6. Integrity Audit (Hash)
orbit_log "Calculating cryptographic signatures (SHA256)..."
(cd "$SRC_DIR" && find . -type f -exec shasum -a 256 {} \;) | sort > "$WORKSPACE/src.sha"
(cd "$DST_DIR" && find . -type f -exec shasum -a 256 {} \;) | sort > "$WORKSPACE/dst.sha"

if diff "$WORKSPACE/src.sha" "$WORKSPACE/dst.sha"; then
    orbit_success "INTEGRITY CONFIRMED: Bit-perfect replication."
else
    orbit_error "INTEGRITY FAILURE: Hash mismatch detected."
    exit 1
fi

# 7. Mutation & Sync
orbit_log "Simulating Data Drift (Mutation Phase)..."
rm "$SRC_DIR/logs/archive/log_1.txt"       # Delete a file
echo "Drift Data" > "$SRC_DIR/new_file.dat" # Add a file
echo "Modified Config" > "$SRC_DIR/config.json" # Modify a file

pause_for_observation "Data Drift" \
    "Check '$SRC_DIR'.
   Notice 'log_1.txt' is gone, 'new_file.dat' exists, and 'config.json' changed.
   '$DST_DIR' is now OUT OF SYNC."

orbit_log "Engaging Synchronization Engine (Mode: SYNC)..."
$BINARY_PATH -s "$SRC_DIR" -d "$DST_DIR" -R -m sync >> "$LOG_FILE" 2>&1
orbit_success "Sync Operation Complete."

# 8. Final Observation
pause_for_observation "Convergence Verification" \
    "Check '$DST_DIR'.
   'log_1.txt' should be DELETED.
   'new_file.dat' should be PRESENT.
   The simulated drift should be resolved."

orbit_log "Demo Complete. Preparing for auto-decommissioning."
