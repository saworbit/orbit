#!/bin/bash
# start-orbit-v2.sh - Orbit V2.2.0 Development Environment

set -e

# Color codes for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   🚀 Orbit V2.2.0 Development Launcher    ║${NC}"
echo -e "${BLUE}║   The Separation: Control Plane + Dashboard ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo ""

# Trap to handle Ctrl+C and cleanup
cleanup() {
    echo ""
    echo -e "${YELLOW}🛑 Shutting down Orbit V2.2.0...${NC}"
    kill $SERVER_PID 2>/dev/null || true
    kill $UI_PID 2>/dev/null || true
    echo -e "${GREEN}✓ Clean shutdown complete${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

# 1. Start the Control Plane (Backend)
echo -e "${GREEN}🧠 Starting Orbit Control Plane (Rust API)...${NC}"
echo -e "${BLUE}   → Directory: crates/orbit-web${NC}"
echo -e "${BLUE}   → Endpoint: http://localhost:8080${NC}"
echo -e "${BLUE}   → Swagger UI: http://localhost:8080/swagger-ui${NC}"
echo ""

cd crates/orbit-web
cargo run --bin orbit-server &
SERVER_PID=$!
cd ../..

# Wait a moment for the server to start
sleep 2

# 2. Start the Dashboard (Frontend)
echo ""
echo -e "${GREEN}🎨 Starting Orbit Dashboard (React SPA)...${NC}"
echo -e "${BLUE}   → Directory: dashboard${NC}"
echo -e "${BLUE}   → Dev Server: http://localhost:5173${NC}"
echo -e "${BLUE}   → HMR: Enabled (Vite)${NC}"
echo ""

cd dashboard
npm run dev &
UI_PID=$!
cd ..

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║        ✓ Orbit V2.2.0 is Running!         ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}📋 Access Points:${NC}"
echo -e "   Dashboard:    ${BLUE}http://localhost:5173${NC}"
echo -e "   API:          ${BLUE}http://localhost:8080/api${NC}"
echo -e "   API Docs:     ${BLUE}http://localhost:8080/swagger-ui${NC}"
echo ""
echo -e "${YELLOW}💡 Tips:${NC}"
echo -e "   • Dashboard has hot reload enabled"
echo -e "   • API changes require cargo rebuild"
echo -e "   • Press Ctrl+C to stop both services"
echo ""

# Wait for both processes
wait
