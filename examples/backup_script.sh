#!/bin/bash
# Daily Backup Script using Orbit
# Purpose: Backup important directories with compression and verification

# Configuration
SOURCE_DIR="$HOME/Documents"
BACKUP_DIR="/mnt/backup/daily"
DATE=$(date +%Y-%m-%d)
BACKUP_PATH="$BACKUP_DIR/$DATE"

# Orbit settings
COMPRESS_LEVEL=9  # High compression for backups
PARALLEL_THREADS=4
MAX_BANDWIDTH=50  # MB/s - limit to not saturate network

# Create backup directory
mkdir -p "$BACKUP_PATH"

echo "🚀 Starting daily backup: $DATE"
echo "Source: $SOURCE_DIR"
echo "Destination: $BACKUP_PATH"
echo ""

# Run Orbit with optimal backup settings
orbit \
  -s "$SOURCE_DIR" \
  -d "$BACKUP_PATH" \
  -R \
  --compress zstd:$COMPRESS_LEVEL \
  --mode sync \
  --parallel $PARALLEL_THREADS \
  --max-bandwidth $MAX_BANDWIDTH \
  --preserve-metadata \
  --retry-attempts 5 \
  --exponential-backoff \
  --exclude "*.tmp" \
  --exclude "*.log" \
  --exclude ".git/*" \
  --exclude "node_modules/*" \
  --audit-format json \
  --audit-log "$BACKUP_DIR/audit.log"

# Check exit status
if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Backup completed successfully!"
    echo "📊 Check audit log: $BACKUP_DIR/audit.log"
else
    echo ""
    echo "❌ Backup failed! Check logs for details."
    exit 1
fi

# Optional: Clean up old backups (keep last 7 days)
echo ""
echo "🧹 Cleaning up old backups..."
find "$BACKUP_DIR" -type d -mtime +7 -exec rm -rf {} + 2>/dev/null || true

echo "✨ Done!"