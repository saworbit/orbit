#!/bin/bash
set -e

# ==============================================================================
#  🎬 ORBIT E2E DEMO WITH VIDEO RECORDING
#  Scenario: Deep Space Telemetry Ingestion + Screen Capture
#  Version: 2.2.0-alpha
#  Purpose: Record demonstration for marketing, training, documentation
# ==============================================================================

# Configuration
ORBIT_ROOT="$(pwd)"
DEMO_SOURCE="/tmp/orbit_demo_source_$(date +%s)"
DEMO_DEST="/tmp/orbit_demo_dest_$(date +%s)"
API_URL="http://localhost:8080"
DASHBOARD_URL="http://localhost:5173"
VIDEO_DIR="$ORBIT_ROOT/demo-recordings"
VIDEO_FILE="$VIDEO_DIR/orbit-demo-$(date +%Y%m%d-%H%M%S).mp4"

# Colors
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║    🎬 ORBIT DEMO ORCHESTRATOR + RECORDER  ║${NC}"
echo -e "${BLUE}║     Scenario: Deep Space Telemetry Sync    ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"

# Check for recording tools
RECORDER=""
if command -v ffmpeg &> /dev/null; then
    RECORDER="ffmpeg"
    echo -e "${GREEN}✓ Found ffmpeg for screen recording${NC}"
elif command -v recordmydesktop &> /dev/null; then
    RECORDER="recordmydesktop"
    echo -e "${GREEN}✓ Found recordmydesktop${NC}"
elif command -v kazam &> /dev/null; then
    RECORDER="kazam"
    echo -e "${GREEN}✓ Found kazam${NC}"
else
    echo -e "${YELLOW}⚠ No screen recorder found. Install ffmpeg for best results:${NC}"
    echo -e "  - Ubuntu/Debian: sudo apt-get install ffmpeg"
    echo -e "  - macOS: brew install ffmpeg"
    echo -e "  - Fedora: sudo dnf install ffmpeg"
    echo ""
    read -p "Continue without recording? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Create video directory
mkdir -p "$VIDEO_DIR"

# Recording state
RECORDING_PID=""

start_recording() {
    if [ -z "$RECORDER" ]; then
        return
    fi

    echo -e "${CYAN}🎥 Starting screen recording...${NC}"

    case "$RECORDER" in
        ffmpeg)
            # Detect display and resolution
            if [[ "$OSTYPE" == "darwin"* ]]; then
                # macOS
                ffmpeg -f avfoundation -i "1:0" \
                    -r 30 -s 1920x1080 \
                    -vcodec libx264 -preset ultrafast \
                    -pix_fmt yuv420p \
                    "$VIDEO_FILE" > /dev/null 2>&1 &
            elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
                # Linux with X11
                if [ -n "$DISPLAY" ]; then
                    ffmpeg -f x11grab -r 30 -s $(xdpyinfo | grep dimensions | awk '{print $2}') \
                        -i $DISPLAY \
                        -vcodec libx264 -preset ultrafast \
                        -pix_fmt yuv420p \
                        "$VIDEO_FILE" > /dev/null 2>&1 &
                else
                    echo -e "${YELLOW}⚠ No DISPLAY variable set, cannot record${NC}"
                    return
                fi
            fi
            RECORDING_PID=$!
            ;;
        recordmydesktop)
            recordmydesktop -o "$VIDEO_FILE" --no-sound > /dev/null 2>&1 &
            RECORDING_PID=$!
            ;;
        kazam)
            kazam --autosave "$VIDEO_DIR" > /dev/null 2>&1 &
            RECORDING_PID=$!
            ;;
    esac

    if [ -n "$RECORDING_PID" ]; then
        echo -e "${GREEN}✓ Recording started (PID: $RECORDING_PID)${NC}"
        echo -e "${GREEN}  Output: $VIDEO_FILE${NC}"
    fi
}

stop_recording() {
    if [ -n "$RECORDING_PID" ] && ps -p $RECORDING_PID > /dev/null 2>&1; then
        echo -e "${CYAN}🎬 Stopping recording...${NC}"
        kill -INT $RECORDING_PID 2>/dev/null || true
        wait $RECORDING_PID 2>/dev/null || true

        # Wait for file to be written
        sleep 2

        if [ -f "$VIDEO_FILE" ]; then
            FILE_SIZE=$(du -h "$VIDEO_FILE" | cut -f1)
            echo -e "${GREEN}✓ Recording saved: $VIDEO_FILE ($FILE_SIZE)${NC}"

            # Generate thumbnail if ffmpeg available
            if command -v ffmpeg &> /dev/null; then
                THUMB_FILE="${VIDEO_FILE%.mp4}.jpg"
                ffmpeg -i "$VIDEO_FILE" -ss 00:00:05 -vframes 1 "$THUMB_FILE" > /dev/null 2>&1
                if [ -f "$THUMB_FILE" ]; then
                    echo -e "${GREEN}  Thumbnail: $THUMB_FILE${NC}"
                fi
            fi
        else
            echo -e "${YELLOW}⚠ Recording file not found${NC}"
        fi
    fi
}

# Enhanced cleanup with recording stop
cleanup() {
    echo -e "\n${YELLOW}[6/6] Initiating Orbital Decay (Cleanup)...${NC}"

    # Stop recording first
    stop_recording

    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$UI_PID" ]; then
        kill $UI_PID 2>/dev/null || true
    fi
    if [ -d "$DEMO_SOURCE" ]; then
        rm -rf "$DEMO_SOURCE"
    fi
    if [ -d "$DEMO_DEST" ]; then
        rm -rf "$DEMO_DEST"
    fi

    echo -e "${GREEN}✓ Systems Offline. Data purged.${NC}"

    if [ -n "$RECORDER" ] && [ -f "$VIDEO_FILE" ]; then
        echo -e "\n${BOLD}════════════════════════════════════════${NC}"
        echo -e "${GREEN}📹 Demo recording available at:${NC}"
        echo -e "${CYAN}   $VIDEO_FILE${NC}"
        echo -e "${BOLD}════════════════════════════════════════${NC}"
    fi
}
trap cleanup EXIT

# Source the main demo script (without running it)
# We'll execute steps manually with recording control

# 1. Pre-flight Checks
echo -e "\n${YELLOW}[1/6] Initiating Pre-flight Systems Check...${NC}"

check_cmd() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}❌ Critical Error: '$1' is not installed.${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ Found $1${NC}"
}

check_cmd "cargo"
check_cmd "npm"
check_cmd "curl"

# 2. Data Fabrication
echo -e "\n${YELLOW}[2/6] Fabricating Synthetic Telemetry Data...${NC}"
mkdir -p "$DEMO_SOURCE"
mkdir -p "$DEMO_DEST"

echo "   Generating binary blobs..."
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_alpha.bin" bs=1M count=50 status=none 2>/dev/null || \
    dd if=/dev/zero of="$DEMO_SOURCE/telemetry_alpha.bin" bs=1M count=50 2>/dev/null
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_beta.bin" bs=1M count=20 status=none 2>/dev/null || \
    dd if=/dev/zero of="$DEMO_SOURCE/telemetry_beta.bin" bs=1M count=20 2>/dev/null
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_gamma.bin" bs=1M count=100 status=none 2>/dev/null || \
    dd if=/dev/zero of="$DEMO_SOURCE/telemetry_gamma.bin" bs=1M count=100 2>/dev/null

for i in {1..20}; do
    echo "$(date '+%Y-%m-%d %H:%M:%S') [TELEMETRY] Sensor $i: Temp=$(($RANDOM % 100))°C" > "$DEMO_SOURCE/flight_log_$i.log"
done

cat > "$DEMO_SOURCE/mission_manifest.json" << EOF
{
  "mission_id": "DEMO_RECORDING_$(date +%s)",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "recording": true
}
EOF

echo -e "${GREEN}✓ Created synthetic dataset at $DEMO_SOURCE${NC}"

# 3. System Ignition
echo -e "\n${YELLOW}[3/6] Igniting Orbit Core Systems...${NC}"
export ORBIT_JWT_SECRET="demo-secret-key-must-be-32-chars-long"

cd "$ORBIT_ROOT/crates/orbit-web"
RUST_LOG=info cargo run --quiet --bin orbit-server > "$ORBIT_ROOT/orbit-server.log" 2>&1 &
SERVER_PID=$!
cd "$ORBIT_ROOT"

cd "$ORBIT_ROOT/dashboard"
npm run dev -- --host 0.0.0.0 > "$ORBIT_ROOT/orbit-dashboard.log" 2>&1 &
UI_PID=$!
cd "$ORBIT_ROOT"

echo -e "   → Waiting for API stability..."
MAX_RETRIES=60
COUNT=0
while ! curl -s -f "$API_URL/api/health" > /dev/null 2>&1; do
    sleep 1
    COUNT=$((COUNT+1))
    if [ $COUNT -ge $MAX_RETRIES ]; then
        echo -e "\n${RED}❌ Timeout waiting for API${NC}"
        exit 1
    fi
    if [ $((COUNT % 10)) -eq 0 ]; then
        echo -n "."
    fi
done
echo ""
echo -e "${GREEN}✓ Control Plane is Online.${NC}"

sleep 2

# Open browser
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    xdg-open "$DASHBOARD_URL" 2>/dev/null &
elif [[ "$OSTYPE" == "darwin"* ]]; then
    open "$DASHBOARD_URL" 2>/dev/null &
fi

# 4. Start Recording
echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                  READY FOR LAUNCH                          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo -e ""
echo -e "   ${BOLD}Dashboard:${NC} ${CYAN}$DASHBOARD_URL${NC}"
echo -e ""
echo -e "${YELLOW}Arrange your windows so the dashboard is visible.${NC}"
echo ""
read -p "Press [ENTER] to START RECORDING and execute the demo..."

start_recording

# Give user time to focus the window
sleep 3

# 5. Job Injection
echo -e "\n${YELLOW}[4/6] Injecting Job via Magnetar API...${NC}"

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

RESPONSE=$(curl -s -X POST "$API_URL/api/create_job" \
  -H "Content-Type: application/json" \
  -d "$JOB_PAYLOAD")

if [[ "$RESPONSE" =~ ^[0-9]+$ ]]; then
    JOB_ID=$RESPONSE
    echo -e "${GREEN}✓ Job Created! Job ID: $JOB_ID${NC}"

    RUN_RESPONSE=$(curl -s -X POST "$API_URL/api/run_job" \
      -H "Content-Type: application/json" \
      -d "{\"job_id\": $JOB_ID}")

    echo -e "${GREEN}✓ Job Started!${NC}"
else
    echo -e "${RED}❌ Failed to create job${NC}"
    exit 1
fi

# 6. Observation Phase
echo -e "\n${YELLOW}[5/6] Observation Phase (Recording)...${NC}"
echo -e ""
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo -e "  ${CYAN}🎥 RECORDING IN PROGRESS${NC}"
echo -e "  ${CYAN}Watch the dashboard and narrate as needed.${NC}"
echo -e ""
echo -e "  • Demonstrate the Visual Chunk Map"
echo -e "  • Show the Live Telemetry graphs"
echo -e "  • Highlight real-time progress tracking"
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo -e ""
read -p "Press [ENTER] to STOP RECORDING and cleanup..."

# Cleanup (trap) will stop recording and clean up
echo -e "\n${GREEN}Demo complete! Video processing...${NC}"
