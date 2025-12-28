#!/bin/bash
set -u

# ==============================================================================
# Orbit Validation Suite v2.0 - Linux/macOS
# ==============================================================================
# Architecture: End-to-end lifecycle validation with resource governance.
# Author: Orbit Architecture Team
#
# Documentation: docs/guides/TESTING_SCRIPTS_GUIDE.md
# ==============================================================================

# --- Configuration ---
TEST_DIR="./orbit_validation_workspace"
SRC_DIR="$TEST_DIR/source_data"
DST_DIR="$TEST_DIR/destination_data"
BINARY_PATH="./target/release/orbit"
LOG_FILE="$TEST_DIR/validation.log"
REQUIRED_SPACE_MB=500  # Safety threshold

# --- Styling ---
BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# --- Core Functions ---
log_info() { echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1" | tee -a "$LOG_FILE"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"; }

header() {
    echo -e "\n${BOLD}${CYAN}:: $1 ::${NC}"
    echo "------------------------------------------------------------"
}

check_disk_space() {
    log_info "Performing pre-flight storage allocation check..."

    # Get available space in KB for current directory
    local available_kb
    available_kb=$(df -k . | awk 'NR==2 {print $4}')
    local available_mb=$((available_kb / 1024))

    if [ "$available_mb" -lt "$REQUIRED_SPACE_MB" ]; then
        log_error "Insufficient disk space. Available: ${available_mb}MB. Required: ${REQUIRED_SPACE_MB}MB."
        log_error "Aborting validation to preserve system stability."
        exit 1
    fi
    log_success "Storage Check Passed: ${available_mb}MB available."
}

observe() {
    echo -e "\n${YELLOW}>>> OBSERVATION POINT: $1${NC}"
    echo -e "${YELLOW}>>> Action: $2${NC}"
    echo -e "${YELLOW}>>> Press [ENTER] when ready to proceed...${NC}"
    read -r
}

cleanup() {
    echo ""
    header "Phase 6: Infrastructure Teardown"
    log_info "Releasing allocated storage..."
    if [ -d "$TEST_DIR" ]; then
        rm -rf "$TEST_DIR"
        log_success "Workspace '$TEST_DIR' successfully decommissioned."
    else
        log_warn "Workspace not found; manual cleanup may be required."
    fi
    echo -e "${BOLD}Orbit Validation Sequence Complete.${NC}"
}
trap cleanup EXIT

# --- Workflow ---

# 1. Environment Analysis
header "Phase 1: Environment Analysis"
mkdir -p "$TEST_DIR"
touch "$LOG_FILE"

check_disk_space

if ! command -v cargo &> /dev/null; then
    log_error "Rust toolchain ('cargo') not detected in PATH."
    exit 1
fi
log_success "Rust toolchain detected."

# 2. Compilation
header "Phase 2: Binary Compilation"
log_info "Compiling Orbit (Release Mode) for optimal throughput..."
log_info "This may take time depending on CPU cores..."

if cargo build --release >> "$LOG_FILE" 2>&1; then
    log_success "Compilation complete."
else
    log_error "Compilation failed. Review '$LOG_FILE' for compiler stderr."
    exit 1
fi

if [ ! -f "$BINARY_PATH" ]; then
    log_error "Binary artifact missing at $BINARY_PATH"
    exit 1
fi

# 3. Data Generation
header "Phase 3: Synthetic Workload Generation"
mkdir -p "$SRC_DIR"

log_info "Allocating small configuration files (High IOPS simulation)..."
for i in {1..20}; do echo "cluster_config_shard_$i" > "$SRC_DIR/shard_$i.dat"; done

log_info "Allocating large binary blob (Throughput simulation)..."
dd if=/dev/urandom of="$SRC_DIR/payload.bin" bs=1M count=15 status=none

log_success "Dataset initialized."

observe "Source Data Created" \
    "Open a new terminal. List files in '$SRC_DIR'. Verify 'payload.bin' is ~15MB."

# 4. Functional Testing (Copy)
header "Phase 4: Replication Testing (Copy)"
log_info "Initiating transfer: Source -> Destination..."

START_TIME=$(date +%s%N)
if $BINARY_PATH -s "$SRC_DIR" -d "$DST_DIR" -R -m copy >> "$LOG_FILE" 2>&1; then
    END_TIME=$(date +%s%N)
    DURATION=$(( (END_TIME - START_TIME) / 1000000 ))
    log_success "Transfer complete in ${DURATION}ms."
else
    log_error "Copy operation failed."
    exit 1
fi

observe "Replication Integrity" \
    "Navigate to '$DST_DIR'. Ensure file counts match Source. Verify directory structure."

# 5. Functional Testing (Sync/Delta)
header "Phase 5: Differential Sync Verification"
log_info "Mutating source state (simulating drift)..."
rm "$SRC_DIR/shard_1.dat"
echo "drift_data" > "$SRC_DIR/shard_new.dat"

log_info "Executing Orbit Sync..."
if $BINARY_PATH -s "$SRC_DIR" -d "$DST_DIR" -R -m sync >> "$LOG_FILE" 2>&1; then
    log_success "Sync logic executed."
else
    log_error "Sync operation failed."
    exit 1
fi

# Automated Integrity Check
log_info "Performing automated checksum audit..."
DIFF_OUT=$(diff -r "$SRC_DIR" "$DST_DIR")
if [ -z "$DIFF_OUT" ]; then
    log_success "Audit Passed: Destination mirrors Source exactly."
else
    log_error "Audit Failed: Divergence detected."
    echo "$DIFF_OUT"
    exit 1
fi

observe "Synchronization State" \
    "Check '$DST_DIR'. Confirm 'shard_1.dat' is deleted and 'shard_new.dat' exists."

log_success "ALL SYSTEMS OPERATIONAL."
