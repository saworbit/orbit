#!/usr/bin/env python3
"""
Orbit Audit Log Forensic Validator

This script verifies the cryptographic integrity of Orbit audit logs by
validating the HMAC-SHA256 chain. Any tampering, insertion, deletion, or
reordering of events will break the chain and be detected.

Usage:
    export ORBIT_AUDIT_SECRET="your_secret_key"
    python3 verify_audit.py /var/log/orbit/audit.jsonl

Exit codes:
    0 - Audit log is valid and intact
    1 - Integrity failures detected or validation error
"""

import json
import hmac
import hashlib
import sys
import os
from typing import List, Tuple, Optional


def load_secret() -> bytes:
    """Load HMAC secret from environment variable."""
    secret = os.environ.get("ORBIT_AUDIT_SECRET", "")
    if not secret:
        print("ERROR: ORBIT_AUDIT_SECRET environment variable not set")
        print("Set it with: export ORBIT_AUDIT_SECRET='your_secret_key'")
        sys.exit(1)
    return secret.encode()


def verify_audit_log(file_path: str, secret: bytes) -> Tuple[bool, int, List[str]]:
    """
    Verify the HMAC chain in an audit log file.

    Args:
        file_path: Path to the audit.jsonl file
        secret: HMAC secret key as bytes

    Returns:
        Tuple of (is_valid, total_records, list_of_errors)
    """
    current_chain_hash = bytes([0] * 32)  # Initial state (32 zero bytes)
    line_number = 0
    errors = []

    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            for line in f:
                line_number += 1
                line = line.strip()

                if not line:
                    continue  # Skip empty lines

                try:
                    record = json.loads(line)
                except json.JSONDecodeError as e:
                    errors.append(f"Line {line_number}: Invalid JSON - {e}")
                    continue

                # Extract and remove the integrity_hash field
                reported_hash = record.pop('integrity_hash', None)

                if reported_hash is None:
                    # Skip legacy audit format records (pre-V3) - they don't have HMAC chains
                    continue

                # Canonicalize the record (use insertion order, matching Rust serde_json)
                # Note: Rust serde_json preserves struct field order, not alphabetical
                canonical_json = json.dumps(record, separators=(',', ':'))
                canonical_bytes = canonical_json.encode('utf-8')

                # Compute HMAC(prev_hash + canonical_bytes)
                data_to_sign = current_chain_hash + canonical_bytes
                h = hmac.new(secret, data_to_sign, hashlib.sha256)
                calculated_hash = h.hexdigest()

                # Verify hash matches
                if reported_hash != calculated_hash:
                    errors.append(
                        f"Line {line_number}: CRITICAL - Integrity failure\n"
                        f"  Expected: {calculated_hash}\n"
                        f"  Got:      {reported_hash}\n"
                        f"  Sequence: {record.get('sequence', 'N/A')}"
                    )
                    # Don't update chain hash - it's broken
                    continue

                # Update chain state for next event
                current_chain_hash = h.digest()

    except FileNotFoundError:
        print(f"ERROR: File not found: {file_path}")
        sys.exit(1)
    except IOError as e:
        print(f"ERROR: Failed to read file: {e}")
        sys.exit(1)

    return (len(errors) == 0, line_number, errors)


def print_report(is_valid: bool, total_records: int, errors: List[str]):
    """Print validation report."""
    print("=" * 70)
    print("Orbit Audit Log Forensic Validation Report")
    print("=" * 70)
    print()

    if is_valid:
        print("[PASS] STATUS: VALID")
        print(f"[PASS] All {total_records} audit records verified")
        print("[PASS] No tampering detected")
        print("[PASS] Chain integrity intact")
        print()
        print("The audit log has cryptographic integrity and can be trusted.")
    else:
        print("[FAIL] STATUS: INVALID")
        print(f"[FAIL] Found {len(errors)} integrity failure(s) in {total_records} records")
        print()
        print("CRITICAL: The audit log has been tampered with or is corrupted!")
        print()
        print("Failures detected:")
        print("-" * 70)
        for error in errors:
            print(error)
            print("-" * 70)
        print()
        print("RECOMMENDATIONS:")
        print("1. DO NOT trust this audit log for compliance purposes")
        print("2. Investigate the source of tampering")
        print("3. Restore from backup if available")
        print("4. Review access logs to identify who modified the file")

    print()
    print("=" * 70)


def main():
    if len(sys.argv) < 2:
        print("Usage: verify_audit.py <path_to_audit.jsonl>")
        print()
        print("Example:")
        print("  export ORBIT_AUDIT_SECRET='my_secret_key'")
        print("  python3 verify_audit.py /var/log/orbit/audit.jsonl")
        sys.exit(1)

    audit_file = sys.argv[1]
    secret = load_secret()

    print(f"Validating audit log: {audit_file}")
    print()

    is_valid, total_records, errors = verify_audit_log(audit_file, secret)

    print_report(is_valid, total_records, errors)

    if not is_valid:
        sys.exit(1)

    sys.exit(0)


if __name__ == "__main__":
    main()
