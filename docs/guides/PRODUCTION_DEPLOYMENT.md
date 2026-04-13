# Orbit Production Deployment Guide

**Status:** Recommended for Production
**Date:** December 2025

---

## 1. The Core Principle: The Single-Writer Mandate

The most critical rule for running Orbit with SQLite/redb backends is:

> **NEVER place `universe.db` on a shared network drive (SMB/NFS) if multiple processes will access it.**

**Why?**
Network file locking is insufficient for the microsecond-level coordination required by high-performance embedded databases. Doing so **guarantees database corruption**.

### The Correct Architecture

Keep databases on local fast storage:

- **The State:** The database (`universe.db`) resides on **local NVMe/SSD**
- **The Storage:** The data files (SMB shares, S3 buckets) are mounted/accessed locally

---

## 2. Deployment Step-by-Step

### Phase 1: Infrastructure Prep

#### Select the Deployment Node

**Operating System:**
- **Recommended:** Linux (Debian/Ubuntu) for best filesystem performance
- **Supported:** Windows Server (if properly configured)

**Storage:**
- **CRITICAL:** You must have a **fast local disk (SSD/NVMe)** for the database files
- **DO NOT** run the DB on a spinning rust boot drive
- Minimum: 50GB free space for database growth
- Recommended: Dedicated partition/volume for `/var/lib/orbit`

**Memory:**
- **Minimum:** 8GB RAM
- **Recommended:** 16GB+ RAM (for buffering and in-memory deduplication maps)

**Network:**
- 10GbE recommended for high-throughput workloads
- 1GbE minimum for typical workloads

#### Mount Remote Storage

Mount your target SMB shares to the OS level.

**Linux (CIFS):**
```bash
# Install CIFS utilities
sudo apt install cifs-utils

# Create mount point
sudo mkdir -p /mnt/smb_source

# Add to /etc/fstab for persistent mounting
echo "//server/share /mnt/smb_source cifs credentials=/root/.smbcredentials,uid=orbit,gid=orbit,file_mode=0755,dir_mode=0755 0 0" | sudo tee -a /etc/fstab

# Create credentials file
sudo cat > /root/.smbcredentials <<EOF
username=your_user
password=your_password
domain=YOUR_DOMAIN
EOF
sudo chmod 600 /root/.smbcredentials

# Mount
sudo mount -a
```

**Windows:**
```powershell
# Map network drive (persistent)
net use Z: \\server\share /user:DOMAIN\username password /persistent:yes

# Verify
dir Z:\
```

---

### Phase 2: CLI Configuration

Orbit operates via the CLI (`orbit sync`). Ensure it uses a local database path.

**CRITICAL:** Do NOT place the database on a network share.

```bash
# Use a local database
orbit sync \
  --source /local/path \
  --destination s3://bucket/path \
  --db-path /var/lib/orbit/universe.db
```

### Phase 3: Scheduled Jobs (Cron/Systemd Timers)

For nightly/weekly jobs, use systemd timers or cron.

**Example: Systemd Timer**

```bash
# Create timer unit
sudo cat > /etc/systemd/system/orbit-nightly-backup.timer <<'EOF'
[Unit]
Description=Nightly Orbit Backup Job

[Timer]
OnCalendar=daily
OnCalendar=02:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

# Create service unit
sudo cat > /etc/systemd/system/orbit-nightly-backup.service <<'EOF'
[Unit]
Description=Orbit Nightly Backup

[Service]
Type=oneshot
ExecStart=/usr/local/bin/orbit-nightly-backup.sh
User=orbit
EOF

# Enable timer
sudo systemctl enable orbit-nightly-backup.timer
sudo systemctl start orbit-nightly-backup.timer
```

---

### Phase 4: Performance Tuning for SMB

Since you are pulling data over SMB, you must tune the "ingest" parameters to avoid stalling the pipeline.

#### Key Configuration Parameters

**In the CLI or config file:**

1. **`concurrency`**: Set to **4-8**
   - **Why:** SMB latency is high. You need multiple parallel readers to saturate the link.
   - **Warning:** Too many (>16) will trash the disk seek time.

2. **`chunk_size`**: Set to **1MB - 4MB**
   - **Why:** Larger chunks reduce the number of SMB round-trips ("chattiness").
   - **Warning:** Defaulting to small 64KB chunks on SMB is a performance killer.

3. **Build with `smb-native` feature** (Windows):
   ```bash
   cargo build --release --features smb-native
   ```

**Example CLI usage:**

```bash
orbit sync \
  --source /mnt/smb_source/data \
  --destination s3://backup/data \
  --concurrency 8 \
  --retry-attempts 5 \
  --compress zstd:3
```

#### SMB Mount Optimization (Linux)

Add these options to your `/etc/fstab` for better performance:

```
//server/share /mnt/smb cifs credentials=/root/.smbcredentials,uid=orbit,gid=orbit,file_mode=0755,dir_mode=0755,cache=strict,actimeo=60,rsize=1048576,wsize=1048576 0 0
```

**Key options:**
- `cache=strict`: Enable aggressive client-side caching
- `actimeo=60`: Cache file attributes for 60 seconds
- `rsize=1048576`: 1MB read buffer
- `wsize=1048576`: 1MB write buffer

---

## 4. Disaster Recovery & Maintenance

### The "Crash-Proof" Guarantee

Because Orbit uses **redb** (ACID-compliant embedded database), power failures are non-fatal. The universe database recovers automatically on restart.

### Backing up the Database

You should periodically backup Orbit's state separately from the data.

**Daily Backup Script:**

```bash
#!/bin/bash
# /usr/local/bin/orbit-backup-state.sh

BACKUP_DIR="/backup/orbit-state"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup database
cp /var/lib/orbit/universe_v3.db "$BACKUP_DIR/universe_v3_$TIMESTAMP.db"

# Optional: Upload to S3
aws s3 sync "$BACKUP_DIR" s3://orbit-backups/state/

# Cleanup old backups (keep 30 days)
find "$BACKUP_DIR" -type f -mtime +30 -delete

echo "Backup completed: $TIMESTAMP"
```

**Add to cron:**
```bash
sudo crontab -e
# Add line:
0 3 * * * /usr/local/bin/orbit-backup-state.sh
```

**Note:** redb files can be safely copied even while open (hot backup), but it is cleaner to momentarily pause writes if perfect consistency is required.

### Monitoring and Alerting

**Monitor these metrics:**

1. **Database size growth:**
   ```bash
   du -sh /var/lib/orbit/*.db
   ```

2. **Disk space:**
   ```bash
   df -h /var/lib/orbit
   ```

**Set up alerts with your monitoring system (Prometheus, Grafana, etc.):**

- Disk usage > 80%
- Transfer failure rate > 10%

---

## 5. Common Pitfalls and Solutions

### ❌ Pitfall 1: Database on Network Share

**Symptoms:**
- "database is locked" errors
- Corrupt database files
- Random job failures

**Solution:**
- Move databases to local SSD
- Follow the Hub topology

### ❌ Pitfall 2: Insufficient Disk Space

**Symptoms:**
- Jobs fail mid-transfer
- Database growth errors

**Solution:**
- Monitor disk usage
- Set up alerts at 80% capacity
- Use dedicated partition with at least 100GB free

### ❌ Pitfall 4: Poor SMB Performance

**Symptoms:**
- Slow transfer rates (< 10 MB/s on 1GbE)
- High latency
- Timeouts

**Solution:**
- Increase `chunk_size_mb` to 2-4 MB
- Set `concurrency` to 4-8
- Optimize SMB mount options (see Phase 4)
- Consider 10GbE network upgrade

---

## Summary Checklist

Before going live, verify:

- ✅ **Local DB:** `universe.db` is on fast local storage (SSD/NVMe)
- ✅ **Remote Data:** SMB shares mounted at OS level
- ✅ **Single Writer:** No other process touches the DB files
- ✅ **Tuning:** Concurrency set to ~8, Chunk Size ~1MB+
- ✅ **Monitoring:** Disk space, transfer metrics
- ✅ **Backups:** Daily state backups to S3 or separate storage
- ✅ **Documentation:** Operations runbook created for your team

---

## Architecture Decision Record (ADR)

**Decision:** Keep databases on local fast storage

**Context:**
redb is an embedded database optimized for single-process access. Network file systems (SMB, NFS) do not provide the file locking semantics required for ACID guarantees.

**Consequences:**

**Positive:**
- Eliminates database corruption risks
- Simplified monitoring and backups
- Better performance (local I/O)

**Alternatives Considered:**
1. **Distributed Database (PostgreSQL):** Adds complexity, requires DBA expertise
2. **Shared File System:** Causes corruption (rejected)
3. **Multiple Hubs with Sharding:** Over-engineered for current scale

**Status:** Accepted (Dec 2025)

---

## Support and Resources

- **Performance Tuning:** [PERFORMANCE.md](PERFORMANCE.md)
- **GitHub Issues:** https://github.com/saworbit/orbit/issues

---

**Follow this topology, and Orbit will run indefinitely without corruption.**

**Status:** ✅ Production-Ready
**License:** Apache-2.0
**Maintainer:** Shane Wall <shaneawall@gmail.com>
