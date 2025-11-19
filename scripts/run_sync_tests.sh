#!/bin/bash
# ORBIT Sync/Mirror Test Script for Unix
# Run comprehensive tests for sync and mirror features

set -e

VERBOSE=""
RELEASE=""
FILTER=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE="-- --nocapture"
            shift
            ;;
        -r|--release)
            RELEASE="--release"
            shift
            ;;
        -f|--filter)
            FILTER="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "ORBIT Sync/Mirror Test Suite"
echo "============================="
echo ""

if [ -n "$RELEASE" ]; then
    echo "Building in RELEASE mode"
else
    echo "Building in DEBUG mode"
fi

# Run unit tests for resilient_sync module
echo ""
echo "Running resilient_sync unit tests..."
cargo test resilient_sync --lib $VERBOSE

# Run filter tests
echo ""
echo "Running filter tests..."
cargo test filter --lib $VERBOSE

# Run delta tests
echo ""
echo "Running delta detection tests..."
cargo test delta --lib $VERBOSE

# Run integration tests
echo ""
echo "Running sync/mirror integration tests..."
if [ -n "$FILTER" ]; then
    cargo test --test sync_mirror_tests $VERBOSE -- $FILTER
else
    cargo test --test sync_mirror_tests $VERBOSE
fi

echo ""
echo "All sync/mirror tests completed!"
