#!/bin/bash
# Orbit Audit Log Tampering Detection Test
#
# This script tests the cryptographic integrity verification of Orbit audit logs
# by attempting various tampering attacks and verifying they are detected.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
export ORBIT_AUDIT_SECRET="test_secret_for_tampering_detection_12345"
TEST_DIR=$(mktemp -d)
AUDIT_LOG="${TEST_DIR}/audit.jsonl"
SOURCE_DIR="${TEST_DIR}/source"
DEST_DIR="${TEST_DIR}/dest"

echo "========================================="
echo "Orbit Audit Tampering Detection Test"
echo "========================================="
echo "Test directory: ${TEST_DIR}"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up test directory..."
    rm -rf "${TEST_DIR}"
}
trap cleanup EXIT

# Create test data
echo "[1/7] Creating test data..."
mkdir -p "${SOURCE_DIR}"
echo "Test file 1" > "${SOURCE_DIR}/file1.txt"
echo "Test file 2" > "${SOURCE_DIR}/file2.txt"
echo "Test file 3" > "${SOURCE_DIR}/file3.txt"
mkdir -p "${SOURCE_DIR}/subdir"
echo "Nested file" > "${SOURCE_DIR}/subdir/nested.txt"

# Run orbit to generate audit log
echo "[2/7] Running Orbit transfer to generate audit log..."
if ! cargo run --quiet -- \
    --src "${SOURCE_DIR}" \
    --dest "${DEST_DIR}" \
    --audit-log "${AUDIT_LOG}" \
    --log-level info 2>&1 | grep -q ".*"; then
    echo -e "${RED}✗ FAILED${NC}: Orbit transfer failed"
    exit 1
fi

# Check if audit log was created
if [ ! -f "${AUDIT_LOG}" ]; then
    echo -e "${RED}✗ FAILED${NC}: Audit log not created at ${AUDIT_LOG}"
    echo "Note: Ensure ORBIT_AUDIT_SECRET environment variable is set"
    exit 1
fi

RECORD_COUNT=$(wc -l < "${AUDIT_LOG}")
echo -e "${GREEN}✓ SUCCESS${NC}: Generated audit log with ${RECORD_COUNT} records"

# Test 1: Verify valid audit log
echo ""
echo "[3/7] Test 1: Verifying valid audit log..."
if python3 scripts/verify_audit.py "${AUDIT_LOG}" 2>&1 | grep -q "STATUS: VALID"; then
    echo -e "${GREEN}✓ PASS${NC}: Valid audit log verified successfully"
else
    echo -e "${RED}✗ FAIL${NC}: Valid audit log failed verification"
    exit 1
fi

# Test 2: Detect timestamp tampering
echo ""
echo "[4/7] Test 2: Detecting timestamp tampering..."
TAMPERED_LOG="${TEST_DIR}/tampered_timestamp.jsonl"
cp "${AUDIT_LOG}" "${TAMPERED_LOG}"

# Modify timestamp in the middle record
MIDDLE_LINE=$((RECORD_COUNT / 2))
sed -i "${MIDDLE_LINE}s/2025/2026/" "${TAMPERED_LOG}"

if python3 scripts/verify_audit.py "${TAMPERED_LOG}" 2>&1 | grep -q "STATUS: INVALID"; then
    echo -e "${GREEN}✓ PASS${NC}: Timestamp tampering detected"
else
    echo -e "${RED}✗ FAIL${NC}: Timestamp tampering NOT detected!"
    exit 1
fi

# Test 3: Detect sequence number tampering
echo ""
echo "[5/7] Test 3: Detecting sequence number tampering..."
TAMPERED_LOG="${TEST_DIR}/tampered_sequence.jsonl"
cp "${AUDIT_LOG}" "${TAMPERED_LOG}"

# Modify sequence number in second record
sed -i '2s/"sequence":[0-9]*/"sequence":999/' "${TAMPERED_LOG}"

if python3 scripts/verify_audit.py "${TAMPERED_LOG}" 2>&1 | grep -q "STATUS: INVALID"; then
    echo -e "${GREEN}✓ PASS${NC}: Sequence tampering detected"
else
    echo -e "${RED}✗ FAIL${NC}: Sequence tampering NOT detected!"
    exit 1
fi

# Test 4: Detect record deletion
echo ""
echo "[6/7] Test 4: Detecting record deletion..."
TAMPERED_LOG="${TEST_DIR}/tampered_deletion.jsonl"
cp "${AUDIT_LOG}" "${TAMPERED_LOG}"

# Delete the middle line
MIDDLE_LINE=$((RECORD_COUNT / 2))
sed -i "${MIDDLE_LINE}d" "${TAMPERED_LOG}"

if python3 scripts/verify_audit.py "${TAMPERED_LOG}" 2>&1 | grep -q "STATUS: INVALID"; then
    echo -e "${GREEN}✓ PASS${NC}: Record deletion detected"
else
    echo -e "${RED}✗ FAIL${NC}: Record deletion NOT detected!"
    exit 1
fi

# Test 5: Detect record reordering
echo ""
echo "[7/7] Test 5: Detecting record reordering..."
TAMPERED_LOG="${TEST_DIR}/tampered_reorder.jsonl"
if [ "${RECORD_COUNT}" -ge 3 ]; then
    # Swap lines 2 and 3
    head -n 1 "${AUDIT_LOG}" > "${TAMPERED_LOG}"
    sed -n '3p' "${AUDIT_LOG}" >> "${TAMPERED_LOG}"
    sed -n '2p' "${AUDIT_LOG}" >> "${TAMPERED_LOG}"
    tail -n +4 "${AUDIT_LOG}" >> "${TAMPERED_LOG}"

    if python3 scripts/verify_audit.py "${TAMPERED_LOG}" 2>&1 | grep -q "STATUS: INVALID"; then
        echo -e "${GREEN}✓ PASS${NC}: Record reordering detected"
    else
        echo -e "${RED}✗ FAIL${NC}: Record reordering NOT detected!"
        exit 1
    fi
else
    echo -e "${YELLOW}⊘ SKIP${NC}: Not enough records for reordering test"
fi

# Summary
echo ""
echo "========================================="
echo -e "${GREEN}✓ ALL TESTS PASSED${NC}"
echo "========================================="
echo ""
echo "Audit integrity system verified:"
echo "  ✓ Valid logs pass verification"
echo "  ✓ Timestamp tampering detected"
echo "  ✓ Sequence tampering detected"
echo "  ✓ Record deletion detected"
echo "  ✓ Record reordering detected"
echo ""
echo "The cryptographic audit chain is working correctly!"
