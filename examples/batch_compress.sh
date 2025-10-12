#!/bin/bash
# Batch Compression Script using Orbit
# Purpose: Compress multiple files/directories for archival

# Configuration
SOURCE_DIR="${1:-.}"  # Default to current directory
OUTPUT_DIR="${2:-./compressed}"
COMPRESS_LEVEL=19  # Maximum compression for archival

# Validate source
if [ ! -d "$SOURCE_DIR" ]; then
    echo "❌ Error: Source directory does not exist: $SOURCE_DIR"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo "🗜️  Batch Compression"
echo "Source: $SOURCE_DIR"
echo "Output: $OUTPUT_DIR"
echo "Compression: Zstd level $COMPRESS_LEVEL (maximum)"
echo ""

# Compress entire directory
orbit \
  -s "$SOURCE_DIR" \
  -d "$OUTPUT_DIR" \
  -R \
  --compress zstd:$COMPRESS_LEVEL \
  --parallel 4 \
  --preserve-metadata \
  --exclude "*.tmp" \
  --exclude "*.bak" \
  --dry-run  # Remove this line to actually compress

echo ""
echo "💡 This was a DRY RUN. Remove --dry-run to execute."
echo ""
echo "To actually compress, edit this script and remove the --dry-run flag"

# Uncomment these lines to show space savings
# echo ""
# echo "📊 Space Analysis:"
# echo "Original: $(du -sh $SOURCE_DIR | cut -f1)"
# echo "Compressed: $(du -sh $OUTPUT_DIR | cut -f1)"