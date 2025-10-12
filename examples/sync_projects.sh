#!/bin/bash
# Project Sync Script using Orbit
# Purpose: Sync development projects between local and remote/external drive

# Configuration
LOCAL_PROJECT="$HOME/projects/myapp"
REMOTE_PROJECT="/mnt/external/projects/myapp"

echo "📦 Syncing project: myapp"
echo "Local:  $LOCAL_PROJECT"
echo "Remote: $REMOTE_PROJECT"
echo ""

# Sync with smart exclusions for development
orbit \
  -s "$LOCAL_PROJECT" \
  -d "$REMOTE_PROJECT" \
  -R \
  --mode sync \
  --parallel 8 \
  --preserve-metadata \
  --exclude "target/*" \
  --exclude "node_modules/*" \
  --exclude ".git/*" \
  --exclude "*.log" \
  --exclude ".DS_Store" \
  --exclude "__pycache__/*" \
  --exclude "*.pyc" \
  --exclude ".venv/*" \
  --exclude "dist/*" \
  --exclude "build/*"

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Project synced successfully!"
else
    echo ""
    echo "❌ Sync failed!"
    exit 1
fi