#!/bin/bash
set -e

# ==============================================================================
#  Orbit Docker Entrypoint
#  Handles different runtime modes for containerized demos
# ==============================================================================

MODE=${1:-server}

case "$MODE" in
  server)
    echo "🚀 Starting Orbit Server..."
    exec /app/orbit-server
    ;;

  demo)
    echo "🛰️ Starting E2E Demo..."
    # Wait for API to be ready
    until curl -s -f http://orbit-server:8080/api/health > /dev/null 2>&1; do
      echo "Waiting for API..."
      sleep 2
    done
    echo "API is ready. Starting demo..."
    exec /app/demo-orbit-ci.sh
    ;;

  shell)
    echo "🐚 Starting interactive shell..."
    exec /bin/bash
    ;;

  *)
    echo "Unknown mode: $MODE"
    echo "Valid modes: server, demo, shell"
    exit 1
    ;;
esac
