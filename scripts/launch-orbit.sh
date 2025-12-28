#!/bin/bash
set -e

# ==============================================================================
#  Orbit Launchpad - v2.2.0-alpha
#  "The best way to orchestrate your data."
# ==============================================================================

# --- Styling & Colors ---
BOLD="\033[1m"
RED="\033[1;31m"
GREEN="\033[1;32m"
BLUE="\033[1;34m"
CYAN="\033[1;36m"
YELLOW="\033[1;33m"
RESET="\033[0m"

# --- ASCII Art Header ---
clear
echo -e "${BLUE}"
cat << "EOF"
   ____      _     _ _
  / __ \    | |   (_) |
 | |  | |_ _| |__  _| |_
 | |  | | '__| '_ \| | __|
 | |__| | |  | |_) | | |_
  \____/|_|  |_.__/|_|\__|
      C O N T R O L   P L A N E
EOF
echo -e "${RESET}"

# --- Helper Functions ---

# Spinner: Runs a command in background and shows a spinner until it finishes
# Usage: spinner "Label text" command_to_run
spinner() {
    local pid=$!
    local delay=0.1
    local spinstr='|/-\'

    # Launch the command (passed as args) in background
    eval "$2" &
    local cmd_pid=$!

    # Hide cursor
    tput civis

    echo -ne "${CYAN}  [ .. ] ${RESET}$1"

    while kill -0 "$cmd_pid" 2> /dev/null; do
        local temp=${spinstr#?}
        printf "\r${CYAN}  [ %c ] ${RESET}" "$spinstr"
        local spinstr=$temp${spinstr%"$temp"}
        sleep $delay
    done

    # Check exit status
    wait "$cmd_pid"
    local exit_code=$?

    # Restore cursor
    tput cnorm

    if [ $exit_code -eq 0 ]; then
        echo -e "\r${GREEN}  [ OK ] ${RESET}$1"
    else
        echo -e "\r${RED}  [FAIL] ${RESET}$1"
        echo -e "${RED}Error details:${RESET}"
        cat /tmp/orbit_error.log
        exit 1
    fi
}

log_info() { echo -e "${BLUE}[INFO]${RESET} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${RESET} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${RESET} $1"; }

# --- 1. System Check ---
echo -e "\n${BOLD}1. System Diagnostic${RESET}"

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust is not installed.${RESET}"
    exit 1
fi
echo -e "${GREEN}  ✓ Rust detected${RESET}"

# Check Node
if ! command -v npm &> /dev/null; then
    echo -e "${RED}Error: Node.js/npm is not installed.${RESET}"
    exit 1
fi
echo -e "${GREEN}  ✓ Node.js detected${RESET}"

# --- 2. Build Services ---
echo -e "\n${BOLD}2. Preparing Engines${RESET}"

# Build Backend
spinner "Compiling Orbit Control Plane (Rust)..." \
    "cd crates/orbit-web && cargo build --quiet --bin orbit-server > /tmp/orbit_build.log 2>&1"

# Install/Build Frontend
if [ ! -d "dashboard/node_modules" ]; then
    spinner "Installing Dashboard Dependencies (npm)..." \
        "cd dashboard && npm ci --silent > /dev/null 2>&1"
fi

# --- 3. Launch Sequence ---
echo -e "\n${BOLD}3. Ignition${RESET}"

# Function to handle kill signal
cleanup() {
    echo -e "\n\n${RED}🛑 Shutting down Orbit...${RESET}"
    kill $SERVER_PID 2>/dev/null || true
    kill $UI_PID 2>/dev/null || true
    echo -e "${GREEN}✓ Systems offline. Have a nice day!${RESET}"
    exit 0
}

# Trap Ctrl+C
trap cleanup SIGINT

# Start Backend
log_info "Starting Control Plane on :8080..."
cd crates/orbit-web
RUST_LOG=info cargo run --quiet --bin orbit-server &
SERVER_PID=$!
cd ../..

# Start Frontend
log_info "Starting Dashboard on :5173..."
cd dashboard
npm run dev -- --clearScreen false > /dev/null 2>&1 &
UI_PID=$!
cd ..

# Wait for health check
echo -ne "${YELLOW}  Waiting for API connection...${RESET}"
MAX_RETRIES=30
COUNT=0
while ! curl -s http://localhost:8080/api/health > /dev/null; do
    sleep 0.5
    COUNT=$((COUNT+1))
    if [ $COUNT -ge $MAX_RETRIES ]; then
        echo -e "\n${RED}Timeout waiting for Server!${RESET}"
        cleanup
    fi
done
echo -e "\r${GREEN}  ✓ Connection established!     ${RESET}"

# --- 4. Liftoff ---
echo -e "\n${BOLD}========================================${RESET}"
echo -e "   ${GREEN}Orbit v2.2 is ACTIVE${RESET}"
echo -e "   ------------------------------------"
echo -e "   📊 Dashboard : ${CYAN}http://localhost:5173${RESET}"
echo -e "   🧠 API Docs  : ${CYAN}http://localhost:8080/swagger-ui${RESET}"
echo -e "${BOLD}========================================${RESET}"
echo -e "${YELLOW}Press Ctrl+C to stop all services.${RESET}\n"

# Open Browser (Cross-platform)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    xdg-open http://localhost:5173
elif [[ "$OSTYPE" == "darwin"* ]]; then
    open http://localhost:5173
elif [[ "$OSTYPE" == "msys" ]]; then
    start http://localhost:5173
fi

# Wait forever (keeps script running so trap works)
wait
