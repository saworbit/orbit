# 🪐 Orbit Production Deployment Guide (V2.2+)

**Architectural Standard:** Hub-and-Spoke (Single Writer)
**Status:** Recommended for Production
**Date:** December 2025

---

## ⚠️ Critical Warning: The "It Works on My Machine" Trap

In systems architecture, "it works on my machine" is the precursor to catastrophe. To run Orbit in a production environment today—specifically avoiding the **"SQLite-over-SMB" corruption trap**—you must adhere to the **Hub Topology**.

This is the authoritative guide on deploying Orbit V2.2+ for maximum stability and performance.

---

## 1. The Core Principle: The Single-Writer Mandate

The most critical rule for running Orbit with SQLite/redb backends is:

> 🛑 **NEVER place `jobs.db` or `universe.db` on a shared network drive (SMB/NFS) if multiple processes will access it.**

**Why?**
Network file locking is insufficient for the microsecond-level coordination required by high-performance embedded databases. Doing so **guarantees database corruption**.

### The Correct Architecture: "The Hub"

Instead of sharing the database file, you **share the process**.

- **The Hub:** A single machine (server, VM, or container) running `orbit-web`
- **The State:** The databases (`jobs.db`, `universe.db`) reside on the **local NVMe/SSD** of the Hub
- **The Storage:** The massive data files (SMB shares, S3 buckets) are mounted/accessed **by the Hub**
- **The Clients:** Users interacting via the **Web UI or API**

```
[ SMB / NAS ] <====(10GbE)====> [ ORBIT HUB ] <----(HTTP)----> [ Users / Dashboard ]
                                     |
                                [ Local SSD ]
                                (jobs.db, universe.db)
```

---

## 2. Deployment Step-by-Step

### Phase 1: Infrastructure Prep

#### Select the Hub Node

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

### Phase 2: Server Configuration

We will deploy the `orbit-server` (Orbit Web) binary. This binary acts as the traffic controller, ensuring all database writes are serialized.

#### Install Orbit Server

Assuming you have built the release binary:

```bash
# Build the release binary
cargo build --release -p orbit-server

# Install to system path
sudo cp target/release/orbit-server /usr/local/bin/
sudo chmod +x /usr/local/bin/orbit-server

# Create orbit user (security best practice)
sudo useradd -r -s /bin/false -d /var/lib/orbit orbit

# Create data directory
sudo mkdir -p /var/lib/orbit
sudo chown orbit:orbit /var/lib/orbit
```

#### Environment Configuration

Create a `.env` file or systemd environment overrides.

**Option 1: Environment File (`/etc/orbit/orbit.env`)**

```bash
# Create config directory
sudo mkdir -p /etc/orbit

# Create environment file
sudo cat > /etc/orbit/orbit.env <<'EOF'
# Network Binding
ORBIT_HOST=0.0.0.0
ORBIT_PORT=3000

# Database Locations (CRITICAL: KEEP THESE LOCAL)
# Do NOT point these to /mnt/smb_share/
DATABASE_URL=sqlite:///var/lib/orbit/jobs.db
UNIVERSE_PATH=/var/lib/orbit/universe_v3.db

# Security (REQUIRED - generate your own!)
ORBIT_AUTH_SECRET=CHANGE_THIS_TO_A_SECURE_RANDOM_STRING

# Optional: Logging
RUST_LOG=info,orbit_web=debug,magnetar=debug
EOF

# Secure the file
sudo chmod 600 /etc/orbit/orbit.env
sudo chown orbit:orbit /etc/orbit/orbit.env
```

**Generate a secure auth secret:**
```bash
openssl rand -base64 32
```

#### Run as a Service (systemd)

**DO NOT** run this in a screen session. Use systemd to ensure it restarts on failure and starts on boot.

```bash
# Create systemd service file
sudo cat > /etc/systemd/system/orbit.service <<'EOF'
[Unit]
Description=Orbit Data Fabric Hub
After=network.target mnt-data.mount
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/orbit-server
WorkingDirectory=/var/lib/orbit
User=orbit
Group=orbit
Restart=always
RestartSec=10

# Environment
EnvironmentFile=/etc/orbit/orbit.env

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/orbit

# Resource limits
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd
sudo systemctl daemon-reload

# Enable and start service
sudo systemctl enable orbit.service
sudo systemctl start orbit.service

# Check status
sudo systemctl status orbit.service
```

**Verify the service is running:**
```bash
# Check logs
sudo journalctl -u orbit.service -f

# Check HTTP endpoint
curl http://localhost:3000/api/health
```

Expected output:
```json
{"status":"healthy","version":"2.2.0"}
```

---

### Phase 3: Operational Workflow

Now that the Hub is running, how do you actually move data?

#### 1. The "Smart Sync" (Ad-Hoc via CLI)

**Use case:** Running a quick sync from your laptop.

**Method:** You can still run the CLI locally (`orbit sync`), but ensure it uses its own local ephemeral DB or a separate local db file.

**CRITICAL:** Do NOT point your local CLI to the server's production DB file.

```bash
# Use a local database
orbit sync \
  --source /local/path \
  --destination s3://bucket/path \
  --db-path /tmp/orbit-local.db
```

#### 2. The Production Job (Via UI/API)

**Use case:** Nightly replication of 50TB.

**Method:** Open the Web Dashboard at `http://hub-ip:3000`

**Steps:**
1. Navigate to **Create Job** tab
2. Configure job:
   - **Source:** `/mnt/smb_source/projects` (path as seen by the Hub)
   - **Destination:** `s3://backup-bucket/projects`
3. Click **Launch Orbit Job**
4. Navigate to **Jobs** tab
5. Click **Run** to start the job

**Benefit:** The Hub handles the locking, the retry logic, and the resilience. If the network blips, the Hub's magnetar engine pauses and resumes automatically.

#### 3. Scheduled Jobs (Cron/Systemd Timers)

For nightly/weekly jobs, use the API with systemd timers or cron.

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
ExecStart=/usr/local/bin/orbit-job-trigger.sh
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

**In the Web UI or API job configuration:**

1. **`pipeline.concurrency`**: Set to **4-8**
   - **Why:** SMB latency is high. You need multiple parallel readers to saturate the link.
   - **Warning:** Too many (>16) will trash the disk seek time.

2. **`pipeline.chunk_size`**: Set to **1MB - 4MB**
   - **Why:** Larger chunks reduce the number of SMB round-trips ("chattiness").
   - **Warning:** Defaulting to small 64KB chunks on SMB is a performance killer.

3. **Build with `smb-native` feature** (Windows):
   ```bash
   cargo build --release --features smb-native
   ```

**Example Job Configuration (API):**

```json
{
  "name": "Nightly SMB to S3 Backup",
  "source": "/mnt/smb_source/data",
  "destination": "s3://backup/data",
  "config": {
    "concurrency": 8,
    "chunk_size_mb": 2,
    "retry_attempts": 5,
    "compression": "zstd:3"
  }
}
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

Because you are using the Hub model with **magnetar** (SQLite WAL) and **universe_v3** (redb ACID), power failures are non-fatal.

**If the Hub loses power:**

1. Restart the machine
2. `systemd` starts `orbit-server` automatically
3. **Magnetar** detects "Processing" jobs that are not running
4. It automatically rewinds them to "Pending" and resumes
5. **No manual intervention required**

### Backing up the Brain

You should periodically backup the Hub's state (the "Brain"), separately from the data.

**Daily Backup Script:**

```bash
#!/bin/bash
# /usr/local/bin/orbit-backup-state.sh

BACKUP_DIR="/backup/orbit-state"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup databases
sqlite3 /var/lib/orbit/jobs.db ".backup '$BACKUP_DIR/jobs_$TIMESTAMP.db'"
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

**Note:** SQLite/redb files can be safely copied even while open (hot backup), but it is cleaner to use the `.backup` command or momentarily pause writes if perfect consistency is required.

### Monitoring and Alerting

**Monitor these metrics:**

1. **Database size growth:**
   ```bash
   du -sh /var/lib/orbit/*.db
   ```

2. **Service health:**
   ```bash
   systemctl status orbit.service
   ```

3. **Disk space:**
   ```bash
   df -h /var/lib/orbit
   ```

4. **Job queue depth:**
   ```bash
   curl http://localhost:3000/api/list_jobs | jq '.jobs | length'
   ```

**Set up alerts with your monitoring system (Prometheus, Grafana, etc.):**

- Disk usage > 80%
- Service restart count > 3 in 1 hour
- Job failure rate > 10%
- API response time > 5s

---

## 5. Security Hardening

### Firewall Configuration

**Open only necessary ports:**

```bash
# UFW (Ubuntu)
sudo ufw allow 3000/tcp comment 'Orbit Web UI'
sudo ufw enable

# Firewalld (RHEL/CentOS)
sudo firewall-cmd --permanent --add-port=3000/tcp
sudo firewall-cmd --reload
```

### TLS/SSL Termination

**DO NOT** expose the Orbit server directly to the internet. Use a reverse proxy.

**Nginx Configuration:**

```nginx
server {
    listen 443 ssl http2;
    server_name orbit.example.com;

    ssl_certificate /etc/letsencrypt/live/orbit.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/orbit.example.com/privkey.pem;

    # Serve Dashboard (if using static build)
    location / {
        root /var/www/orbit-dashboard/dist;
        try_files $uri /index.html;
    }

    # Proxy API requests to backend
    location /api/ {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts for long-running operations
        proxy_connect_timeout 600s;
        proxy_send_timeout 600s;
        proxy_read_timeout 600s;
    }

    # WebSocket support (future)
    location /ws/ {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header Host $host;
    }
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name orbit.example.com;
    return 301 https://$server_name$request_uri;
}
```

### Authentication

**Enable authentication in production:**

```bash
# Add to /etc/orbit/orbit.env
ORBIT_AUTH_ENABLED=true
ORBIT_AUTH_SECRET=$(openssl rand -base64 32)
```

**Create admin user:**

```bash
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "SecurePassword123!",
    "role": "admin"
  }'
```

---

## 6. Common Pitfalls and Solutions

### ❌ Pitfall 1: Database on Network Share

**Symptoms:**
- "database is locked" errors
- Corrupt database files
- Random job failures

**Solution:**
- Move databases to local SSD
- Follow the Hub topology

### ❌ Pitfall 2: Multiple orbit-server Instances

**Symptoms:**
- Duplicate job execution
- Race conditions
- Database corruption

**Solution:**
- Run ONLY ONE `orbit-server` instance
- Use the Web UI/API for all operations
- Disable direct CLI access to production databases

### ❌ Pitfall 3: Insufficient Disk Space

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

- ✅ **Hub Installed:** `orbit-server` running on a dedicated node
- ✅ **Local DBs:** `jobs.db` and `universe.db` are on fast local storage (SSD/NVMe)
- ✅ **Remote Data:** SMB shares mounted at OS level
- ✅ **Single Writer:** No other process touches the DB files
- ✅ **Systemd Service:** Configured for auto-restart and boot-time start
- ✅ **Tuning:** Concurrency set to ~8, Chunk Size ~1MB+
- ✅ **Monitoring:** Disk space, service health, job metrics
- ✅ **Backups:** Daily state backups to S3 or separate storage
- ✅ **Security:** Firewall configured, TLS enabled, authentication enforced
- ✅ **Documentation:** Operations runbook created for your team

---

## Architecture Decision Record (ADR)

**Decision:** Use Hub-and-Spoke topology for production deployments

**Context:**
SQLite and redb are embedded databases optimized for single-process access. Network file systems (SMB, NFS) do not provide the file locking semantics required for ACID guarantees.

**Consequences:**

**Positive:**
- Eliminates database corruption risks
- Centralized state management
- Simplified monitoring and backups
- Better performance (local I/O)

**Negative:**
- Single point of failure (mitigated by systemd auto-restart)
- Requires dedicated server/VM
- All traffic routed through Hub (mitigated by 10GbE)

**Alternatives Considered:**
1. **Distributed Database (PostgreSQL):** Adds complexity, requires DBA expertise
2. **Shared File System:** Causes corruption (rejected)
3. **Multiple Hubs with Sharding:** Over-engineered for current scale

**Status:** Accepted (Dec 2025)

---

## Support and Resources

- **Documentation:** [ORBIT_V2_ARCHITECTURE.md](../architecture/ORBIT_V2_ARCHITECTURE.md)
- **Troubleshooting:** [RUNNING_V2.md](RUNNING_V2.md)
- **Performance Tuning:** [PERFORMANCE.md](PERFORMANCE.md)
- **GitHub Issues:** https://github.com/saworbit/orbit/issues

---

**Follow this topology, and Orbit will run indefinitely without corruption.**

**Status:** ✅ Production-Ready
**License:** Apache-2.0
**Maintainer:** Shane Wall <shaneawall@gmail.com>
