#!/bin/bash
# Orbit Nebula Startup Script
# Version: 1.0.0-alpha.2
# Description: Complete setup and launch script for Nebula web interface

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Banner
echo -e "${BLUE}"
cat << "EOF"
   ____       __    _ __     _   __     __          __
  / __ \_____/ /_  (_) /_   / | / /__  / /_  __  __/ /___ _
 / / / / ___/ __ \/ / __/  /  |/ / _ \/ __ \/ / / / / __ `/
/ /_/ / /  / /_/ / / /_   / /|  /  __/ /_/ / /_/ / / /_/ /
\____/_/  /_.___/_/\__/  /_/ |_/\___/_.___/\__,_/_/\__,_/

           Real-Time Data Orchestration Control Center
                      v1.0.0-alpha.2
EOF
echo -e "${NC}"

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MAGNETAR_DB="${ORBIT_MAGNETAR_DB:-$SCRIPT_DIR/data/magnetar.db}"
USER_DB="${ORBIT_USER_DB:-$SCRIPT_DIR/data/users.db}"
JWT_SECRET="${ORBIT_JWT_SECRET:-}"
HOST="${ORBIT_HOST:-127.0.0.1}"
PORT="${ORBIT_PORT:-8080}"
RUST_LOG="${RUST_LOG:-info,orbit_web=debug}"

# Function to print status messages
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
info "Checking prerequisites..."

if ! command -v cargo &> /dev/null; then
    error "Rust/Cargo not found. Please install from https://rustup.rs"
    exit 1
fi
success "Rust/Cargo found"

if ! command -v rustc &> /dev/null; then
    error "Rust compiler not found"
    exit 1
fi
success "Rust compiler found"

# Check for wasm target (needed for Leptos)
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    warning "wasm32-unknown-unknown target not installed"
    info "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
    success "wasm32-unknown-unknown target installed"
else
    success "wasm32-unknown-unknown target found"
fi

# Generate JWT secret if not provided
if [ -z "$JWT_SECRET" ]; then
    warning "ORBIT_JWT_SECRET not set, generating random secret..."
    JWT_SECRET=$(openssl rand -base64 32 2>/dev/null || head -c 32 /dev/urandom | base64)
    export ORBIT_JWT_SECRET="$JWT_SECRET"
    success "Generated JWT secret"
    warning "⚠️  Production deployments should set a permanent ORBIT_JWT_SECRET"
else
    success "Using provided JWT secret"
fi

# Create data directory
info "Setting up data directory..."
mkdir -p "$SCRIPT_DIR/data"
success "Data directory ready at $SCRIPT_DIR/data"

# Export environment variables
export ORBIT_MAGNETAR_DB="$MAGNETAR_DB"
export ORBIT_USER_DB="$USER_DB"
export ORBIT_HOST="$HOST"
export ORBIT_PORT="$PORT"
export RUST_LOG="$RUST_LOG"

# Display configuration
echo ""
info "Configuration:"
echo "  Magnetar DB:  $MAGNETAR_DB"
echo "  User DB:      $USER_DB"
echo "  Host:         $HOST"
echo "  Port:         $PORT"
echo "  Log Level:    $RUST_LOG"
echo ""

# Check if we need to build
BUILD_NEEDED=false
if [ ! -f "$SCRIPT_DIR/../../target/release/orbit-web" ]; then
    BUILD_NEEDED=true
    info "First-time setup detected, build required"
else
    # Check if source files are newer than binary
    if [ "$SCRIPT_DIR/src" -nt "$SCRIPT_DIR/../../target/release/orbit-web" ]; then
        BUILD_NEEDED=true
        info "Source files updated, rebuild required"
    fi
fi

# Build if needed
if [ "$BUILD_NEEDED" = true ]; then
    info "Building Orbit Nebula (this may take a few minutes on first run)..."
    cd "$SCRIPT_DIR"
    cargo build --release
    success "Build completed successfully"
else
    success "Using existing build"
fi

# Display startup information
echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Orbit Nebula is starting...${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${BLUE}📡 Server Information:${NC}"
echo "   URL:              http://$HOST:$PORT"
echo "   Health Check:     http://$HOST:$PORT/api/health"
echo "   API Docs:         http://$HOST:$PORT/api/auth/login"
echo ""
echo -e "${BLUE}🔐 Default Credentials:${NC}"
echo "   Username:         admin"
echo "   Password:         orbit2025"
echo "   ${RED}⚠️  Change password after first login!${NC}"
echo ""
echo -e "${BLUE}🔧 API Endpoints:${NC}"
echo "   POST /api/auth/login     - Authenticate"
echo "   GET  /api/auth/me        - Get current user"
echo "   GET  /api/health         - Health check"
echo "   WS   /ws/:job_id         - WebSocket events"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop the server${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Run the server
cd "$SCRIPT_DIR"
info "Starting Orbit Nebula web server..."
cargo run --release

# Cleanup message (only shown on graceful shutdown)
echo ""
success "Orbit Nebula stopped gracefully"
