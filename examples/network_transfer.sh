#!/bin/bash
# Network Transfer Script using Orbit
# Purpose: Transfer large files to remote server with resume capability

# Configuration
SOURCE_FILE="$1"
DEST_SERVER="$2"
DEST_PATH="$3"

# Validate arguments
if [ -z "$SOURCE_FILE" ] || [ -z "$DEST_SERVER" ] || [ -z "$DEST_PATH" ]; then
    echo "Usage: $0 <source_file> <server> <dest_path>"
    echo ""
    echo "Example:"
    echo "  $0 large_database.sql backup-server /backups/db/"
    exit 1
fi

echo "🌐 Network Transfer"
echo "Source: $SOURCE_FILE"
echo "Destination: $DEST_SERVER:$DEST_PATH"
echo ""

# Transfer with compression, resume, and aggressive retry
orbit \
  -s "$SOURCE_FILE" \
  -d "/mnt/$DEST_SERVER$DEST_PATH/$(basename $SOURCE_FILE)" \
  --compress zstd:3 \
  --resume \
  --retry-attempts 10 \
  --exponential-backoff \
  --retry-delay 10 \
  --max-bandwidth 100 \
  --preserve-metadata \
  --audit-format json \
  --audit-log "./transfer_audit.log"

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Transfer completed!"
    echo "📊 Audit log: ./transfer_audit.log"
else
    echo ""
    echo "❌ Transfer failed!"
    echo "💡 You can resume by running the same command again"
    exit 1
fi