#!/bin/bash
# SMB/CIFS Examples for Orbit v0.4.0

echo "🌐 Orbit SMB Examples"
echo "===================="
echo ""

# =============================================================================
# Basic SMB Operations
# =============================================================================

echo "📁 1. Basic SMB Copy"
echo "-------------------"
echo "Copy local file to SMB share:"
echo ""
echo "  orbit -s /tmp/document.pdf -d smb://fileserver/documents/document.pdf"
echo ""

echo "📁 2. Copy from SMB to Local"
echo "----------------------------"
echo "Download file from network share:"
echo ""
echo "  orbit -s smb://fileserver/documents/report.xlsx -d /tmp/report.xlsx"
echo ""

echo "📁 3. Copy between SMB Shares"
echo "-----------------------------"
echo "Transfer between network locations:"
echo ""
echo "  orbit -s smb://server1/share1/data.zip \\"
echo "        -d smb://server2/share2/data.zip"
echo ""

# =============================================================================
# Authentication
# =============================================================================

echo "🔐 4. SMB with Authentication"
echo "-----------------------------"
echo "Provide credentials in URI:"
echo ""
echo "  orbit -s local.txt \\"
echo "        -d smb://username:password@fileserver/private/file.txt"
echo ""
echo "⚠️  WARNING: Passwords in URIs are visible in process lists!"
echo "   Better: Use environment variables (see example 5)"
echo ""

echo "🔐 5. Using Environment Variables"
echo "---------------------------------"
echo "Secure credential handling:"
echo ""
echo "  export SMB_USERNAME=myuser"
echo "  export SMB_PASSWORD=mypass"
echo "  export SMB_DOMAIN=CORPORATE"
echo ""
echo "  orbit -s local.txt -d smb://fileserver/share/file.txt"
echo ""
echo "  # Orbit will use env vars if credentials not in URI"
echo ""

echo "🔐 6. Username Only (Prompt for Password)"
echo "-----------------------------------------"
echo "Interactive password entry:"
echo ""
echo "  orbit -s local.txt -d smb://myuser@fileserver/share/file.txt"
echo "  # Will prompt: Password for myuser@fileserver:"
echo ""

# =============================================================================
# Directory Operations
# =============================================================================

echo "📦 7. Recursive SMB Copy"
echo "-----------------------"
echo "Backup entire directory to network share:"
echo ""
echo "  orbit -s ~/Documents -d smb://fileserver/backups/Documents \\"
echo "        -R \\"
echo "        --preserve-metadata \\"
echo "        --compress zstd:9"
echo ""

echo "📦 8. Sync Mode with SMB"
echo "------------------------"
echo "Incremental backup (only copy changes):"
echo ""
echo "  orbit -s ~/Projects -d smb://fileserver/projects \\"
echo "        -R \\"
echo "        --mode sync \\"
echo "        --parallel 8 \\"
echo "        --exclude '.git/*' \\"
echo "        --exclude 'target/*'"
echo ""

echo "📦 9. Mirror Directories"
echo "-----------------------"
echo "Keep SMB share synchronized (delete extras):"
echo ""
echo "  orbit -s ~/Photos -d smb://nas/photos \\"
echo "        -R \\"
echo "        --mode mirror \\"
echo "        --dry-run  # Preview changes first"
echo ""

# =============================================================================
# Advanced Features
# =============================================================================

echo "⚡ 10. Large File Transfer with Resume"
echo "--------------------------------------"
echo "Transfer database backup with retry:"
echo ""
echo "  orbit -s /var/backups/database.sql \\"
echo "        -d smb://backup-server/databases/db.sql \\"
echo "        --compress zstd:3 \\"
echo "        --resume \\"
echo "        --retry-attempts 10 \\"
echo "        --exponential-backoff \\"
echo "        --max-bandwidth 50"
echo ""

echo "⚡ 11. Parallel Multi-File Transfer"
echo "-----------------------------------"
echo "Fast transfer of many files:"
echo ""
echo "  orbit -s ./media -d smb://fileserver/media \\"
echo "        -R \\"
echo "        --parallel 16 \\"
echo "        --compress lz4"
echo ""

echo "⚡ 12. Bandwidth-Limited Transfer"
echo "--------------------------------"
echo "Don't saturate network:"
echo ""
echo "  orbit -s ./large_dataset -d smb://server/data \\"
echo "        -R \\"
echo "        --max-bandwidth 25  # 25 MB/s"
echo ""

# =============================================================================
# Windows-Specific
# =============================================================================

echo "🪟 13. Windows UNC Paths"
echo "------------------------"
echo "Windows-style network paths:"
echo ""
echo "  # Method 1: Use smb:// protocol"
echo "  orbit -s local.txt -d smb://SERVER/Share/file.txt"
echo ""
echo "  # Method 2: UNC path (Windows only)"
echo "  orbit -s local.txt -d '\\\\SERVER\\Share\\file.txt'"
echo ""

echo "🪟 14. Windows Domain Authentication"
echo "------------------------------------"
echo "Corporate network access:"
echo ""
echo "  export SMB_DOMAIN=CORPORATE"
echo "  export SMB_USERNAME=john.doe"
echo "  export SMB_PASSWORD=SecurePass123"
echo ""
echo "  orbit -s report.pdf -d smb://corpserver/reports/report.pdf"
echo ""

# =============================================================================
# Production Scenarios
# =============================================================================

echo "🏢 15. Automated Backup Script"
echo "------------------------------"
echo "Daily backup to network storage:"
echo ""
cat << 'EOF'
#!/bin/bash
# backup.sh - Run via cron

DATE=$(date +%Y-%m-%d)
SOURCE="/var/www/html"
DEST="smb://backup-server/web-backups/$DATE"

orbit -s "$SOURCE" -d "$DEST" \
  -R \
  --compress zstd:9 \
  --mode sync \
  --retry-attempts 5 \
  --audit-log "/var/log/orbit/backup-$DATE.log" \
  --exclude "*.tmp" \
  --exclude "cache/*"

if [ $? -eq 0 ]; then
    echo "✅ Backup completed: $DATE"
else
    echo "❌ Backup failed: $DATE"
    exit 1
fi
EOF
echo ""

echo "🏢 16. Log Aggregation"
echo "---------------------"
echo "Collect logs from servers to central share:"
echo ""
cat << 'EOF'
#!/bin/bash
# aggregate_logs.sh

SERVERS=("web1" "web2" "db1" "db2")
DEST_BASE="smb://log-server/logs/$(date +%Y-%m-%d)"

for server in "${SERVERS[@]}"; do
    echo "Collecting logs from $server..."
    
    orbit -s "smb://$server/logs" \
          -d "$DEST_BASE/$server" \
          -R \
          --compress zstd:9 \
          --parallel 4
done
EOF
echo ""

echo "🏢 17. Data Migration"
echo "--------------------"
echo "Migrate data between file servers:"
echo ""
echo "  orbit -s smb://old-server/data -d smb://new-server/data \\"
echo "        -R \\"
echo "        --mode mirror \\"
echo "        --parallel 8 \\"
echo "        --preserve-metadata \\"
echo "        --verify-checksum \\"
echo "        --audit-log migration.log"
echo ""

# =============================================================================
# Troubleshooting
# =============================================================================

echo "🔧 18. Debug Mode"
echo "----------------"
echo "Verbose output for troubleshooting:"
echo ""
echo "  RUST_LOG=debug orbit -s file.txt -d smb://server/share/file.txt"
echo ""

echo "🔧 19. Test Connection"
echo "---------------------"
echo "Verify SMB access:"
echo ""
echo "  # Dry run to test without copying"
echo "  orbit -s test.txt -d smb://fileserver/share/test.txt --dry-run"
echo ""

echo "🔧 20. Check SMB Statistics"
echo "--------------------------"
echo "View transfer statistics:"
echo ""
echo "  orbit stats --log orbit_audit.log"
echo ""
echo "  # Shows:"
echo "  # - Total transfers"
echo "  # - Success/failure rates"
echo "  # - Average transfer speeds"
echo "  # - Protocol breakdown (local vs SMB)"
echo ""

# =============================================================================
# Configuration File Example
# =============================================================================

echo "⚙️  21. Configuration File"
echo "-------------------------"
echo "Create ~/.orbit/orbit.toml:"
echo ""
cat << 'EOF'
[defaults]
compress = "zstd:3"
retry_attempts = 5
preserve_metadata = true
parallel = 4

[smb]
default_username = "myuser"
timeout_secs = 30
use_encryption = true
workgroup = "WORKGROUP"

[exclude]
patterns = [
    "*.tmp",
    "*.log",
    ".git/*",
    "node_modules/*",
    "__pycache__/*"
]

[audit]
format = "json"
path = "~/.orbit/audit.log"
EOF
echo ""

# =============================================================================
# Security Best Practices
# =============================================================================

echo "🔒 Security Best Practices"
echo "============================"
echo ""
echo "1. ❌ DON'T store passwords in scripts or config files"
echo "   ✅ DO use environment variables or credential managers"
echo ""
echo "2. ❌ DON'T use passwords in URIs (visible in process list)"
echo "   ✅ DO prompt for passwords interactively"
echo ""
echo "3. ❌ DON'T disable encryption"
echo "   ✅ DO use SMB3 encryption when available"
echo ""
echo "4. ❌ DON'T ignore certificate warnings"
echo "   ✅ DO validate server certificates"
echo ""
echo "5. ❌ DON'T use 'guest' or anonymous access for sensitive data"
echo "   ✅ DO use proper authentication with strong passwords"
echo ""

# =============================================================================
# Common Errors and Solutions
# =============================================================================

echo "🚨 Common Errors"
echo "================"
echo ""
echo "Error: 'SMB connection failed'"
echo "  → Check network connectivity: ping fileserver"
echo "  → Verify share name: smbclient -L //fileserver -N"
echo "  → Check firewall: ports 139, 445"
echo ""
echo "Error: 'SMB authentication failed'"
echo "  → Verify username and password"
echo "  → Check domain name"
echo "  → Try: smbclient //server/share -U username"
echo ""
echo "Error: 'Share not found'"
echo "  → List available shares: smbclient -L //server -N"
echo "  → Verify share permissions"
echo ""
echo "Error: 'Permission denied'"
echo "  → Check share ACLs"
echo "  → Verify user has write permission"
echo "  → Check filesystem permissions"
echo ""

echo ""
echo "📚 More Help"
echo "============"
echo "  orbit --help"
echo "  man orbit"
echo "  https://github.com/saworbit/orbit"
echo ""
echo "Happy transferring! 🚀"