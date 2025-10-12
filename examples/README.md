# Orbit Examples

This directory contains practical example scripts demonstrating common Orbit use cases.

---

## 📁 Available Examples

### 1. `backup_script.sh` - Daily Backup
**Purpose:** Automated daily backups with compression and retention

**Features:**
- High compression (Zstd level 9)
- Incremental sync mode
- Bandwidth limiting
- Excludes temporary files
- Keeps last 7 days of backups

**Usage:**
```bash
# Edit the script to set your paths
nano backup_script.sh

# Run the backup
./backup_script.sh

Ideal for:

Daily automated backups
Scheduled cron jobs
Important document backups


2. sync_projects.sh - Development Project Sync
Purpose: Sync development projects between local and external drives
Features:

Fast sync mode (only copies changed files)
Excludes build artifacts
Parallel copying
Preserves all metadata

Usage:

# Edit paths in the script
nano sync_projects.sh

# Run sync
./sync_projects.sh

Ideal for:

Backing up active projects
Syncing between workstations
External drive backups

3. network_transfer.sh - Remote Server Transfer
Purpose: Transfer large files to remote servers with resume capability
Features:

Compression for faster transfer
Resume on interruption
Aggressive retry with backoff
Bandwidth limiting
Audit logging

Usage:

./network_transfer.sh <source_file> <server> <dest_path>

# Example:
./network_transfer.sh database.sql backup-server /backups/db/

Ideal for:

Large database transfers
Unreliable network connections
Remote backups

4. batch_compress.sh - Archive Compression
Purpose: Compress directories with maximum compression for archival
Features:

Maximum compression (Zstd level 19)
Dry run mode first
Space savings report
Batch processing

Usage:

# Dry run (default)
./batch_compress.sh /path/to/source /path/to/output

# After reviewing, edit script to remove --dry-run
nano batch_compress.sh
./batch_compress.sh /path/to/source /path/to/output

Ideal for:

Long-term archival
Reducing storage costs
Compressing old data

🎯 Common Patterns

Exclude Patterns
--exclude "*.tmp"           # Temporary files
--exclude "*.log"           # Log files
--exclude ".git/*"          # Git repository
--exclude "node_modules/*"  # Node dependencies
--exclude "target/*"        # Rust build artifacts
--exclude "__pycache__/*"   # Python cache

Compression Levels
--compress lz4        # Fast (good for local copies)
--compress zstd:3     # Balanced (default)
--compress zstd:9     # Good compression (backups)
--compress zstd:19    # Maximum (archival)

Copy Modes
--mode copy    # Always copy
--mode sync    # Only copy new/changed files
--mode update  # Only copy newer files
--mode mirror  # Sync and delete extras

🔧 Customization Tips

For Faster Transfers
--parallel 16           # More threads
--compress lz4         # Faster compression
--chunk-size 4096      # Larger chunks

For Better Compression
--compress zstd:19     # Maximum compression
--parallel 2           # Fewer threads, more CPU for compression

For Unreliable Networks
--resume                # Enable resume
--retry-attempts 20     # More retries
--exponential-backoff   # Smart retry delays
--max-bandwidth 10      # Limit to stable speed

📝 Setting Up Automated Backups

Linux/macOS (Cron)
Edit crontab:

crontab -e
Add daily backup at 2 AM:
0 2 * * * /path/to/examples/backup_script.sh >> /var/log/orbit-backup.log 2>&1

Windows (Task Scheduler)

Open Task Scheduler
Create Basic Task
Set trigger (e.g., Daily at 2:00 AM)
Action: Start a program
Program: bash
Arguments: /path/to/backup_script.sh

🐛 Troubleshooting
"Permission denied"
chmod +x examples/*.sh

"Command not found: orbit"
Ensure Orbit is installed:
cargo install --path .

Or use full path:
/path/to/orbit/target/release/orbit ...

Resume not working
Make sure you're using the exact same command. Resume files are stored next to the destination file.

💡 Need Help?

Main README
Quick Start Guide
GitHub Issues


🤝 Contributing Examples
Have a useful script? Please contribute!

Fork the repository
Add your script to examples/
Update this README
Submit a pull request


Happy transferring with Orbit! 🚀