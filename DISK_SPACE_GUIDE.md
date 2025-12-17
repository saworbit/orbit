# 💾 Orbit Demo - Disk Space Guide

## Quick Reference

| Your Situation | Space Needed | Duration | Notes |
|----------------|--------------|----------|-------|
| 🆕 First-time user (never built) | **6 GB** | 10-15 min | Includes full compilation |
| 🔄 Have built before | **1 GB** | 2-5 min | Reuses cached artifacts |
| ⚡ Using pre-built CI binaries | **400 MB** | 1-2 min | Fastest option |
| 🎬 Want to record video | **+500 MB/5min** | Varies | Add to above |
| 🐳 Using Docker | **10 GB** | 15-20 min | First build only |
| 🧪 Running in CI/CD | **2 GB** | 5-10 min | Optimized for speed |

## Detailed Breakdown

### 1. Demo Data Files

**Created at runtime in `/tmp` (Linux/macOS) or `%TEMP%` (Windows):**

| File | Size | Purpose |
|------|------|---------|
| `telemetry_alpha.bin` | 50 MB | Large binary blob (simulates sensor data) |
| `telemetry_beta.bin` | 20 MB | Medium binary blob |
| `telemetry_gamma.bin` | 100 MB | Large binary blob |
| `flight_log_*.log` (20 files) | ~500 KB total | Text logs |
| `mission_manifest.json` | ~1 KB | Metadata |
| **Source Total** | **~170 MB** | |
| **Destination (copy)** | **~170 MB** | Transferred files |
| **Demo Data Total** | **~340 MB** | Automatically cleaned up |

### 2. Build Artifacts (Persistent)

**Located in project directory (reusable across runs):**

| Component | Location | Size | When Created |
|-----------|----------|------|--------------|
| Rust compilation cache | `target/` | 2-5 GB | First `cargo build` |
| Node.js dependencies | `dashboard/node_modules/` | 400-600 MB | First `npm install` |
| Compiled backend binary | `target/release/orbit-server` | 15-30 MB | `cargo build --release` |
| Dashboard build output | `dashboard/dist/` | 10-20 MB | `npm run build` (optional) |

**Total Build Artifacts:** **3-6 GB** (one-time, reusable)

### 3. Runtime Files (Persistent)

| File | Location | Size | Notes |
|------|----------|------|-------|
| SQLite database | `crates/orbit-web/magnetar.db` | 5-10 MB | Grows with job history |
| Server logs | `orbit-server.log` | 1-50 MB | Varies by log level |
| Dashboard logs | `orbit-dashboard.log` | 1-10 MB | Development mode only |

**Total Runtime:** **10-70 MB**

### 4. Optional Components

#### Video Recording

| Quality | Resolution | Bitrate | Size per Minute | 5-Min Demo |
|---------|------------|---------|-----------------|------------|
| High | 1920x1080 | 8 Mbps | ~60 MB | ~300 MB |
| Medium | 1920x1080 | 5 Mbps | ~37 MB | ~185 MB |
| Low | 1280x720 | 2 Mbps | ~15 MB | ~75 MB |
| Ultra (preset slow) | 1920x1080 | 12 Mbps | ~90 MB | ~450 MB |

**Default (ultrafast preset):** ~100 MB/minute = **~500 MB for 5-minute demo**

**Additional:** Thumbnail ~100 KB

#### Docker Images

| Image | Size | Compressed | Notes |
|-------|------|------------|-------|
| `rust:1.75-slim` | ~700 MB | ~250 MB | Base builder image |
| `node:20-alpine` | ~200 MB | ~70 MB | Frontend builder |
| `debian:bookworm-slim` | ~100 MB | ~50 MB | Runtime base |
| `orbit-demo-server:latest` | ~200 MB | ~80 MB | Final application image |
| **Build cache** | 2-3 GB | N/A | Intermediate layers |

**Total Docker (first build):** **4-6 GB**
**Total Docker (subsequent):** **~500 MB** (volume data only)

## Space Requirements by Scenario

### Scenario A: Interactive Demo (First Time)

```
✅ Prerequisites exist (Rust, Node.js installed)
🎯 Goal: Run ./demo-orbit.sh

Required:
├─ Demo data (source + dest)        340 MB
├─ Rust compilation (target/)     3,000 MB
├─ Node modules                     500 MB
├─ Database + logs                   20 MB
└─ Buffer (safety margin)           140 MB
────────────────────────────────────────────
   TOTAL REQUIRED:                 4,000 MB (4 GB)
   RECOMMENDED:                    6,000 MB (6 GB)
```

### Scenario B: Interactive Demo (Subsequent Runs)

```
✅ Already built once before
🎯 Goal: Run ./demo-orbit.sh again

Required:
├─ Demo data (source + dest)        340 MB
├─ Database + logs                   20 MB
└─ Buffer                           140 MB
────────────────────────────────────────────
   TOTAL REQUIRED:                   500 MB
   RECOMMENDED:                    1,000 MB (1 GB)
```

### Scenario C: CI/CD Headless Mode

```
✅ Optimized for speed with smaller data
🎯 Goal: Run ./demo-orbit-ci.sh in GitHub Actions

Required:
├─ Demo data (35 MB reduced size)    70 MB
├─ Rust compilation (cached)      3,000 MB
├─ Node modules (cached)            500 MB
├─ Metrics JSON                       1 MB
└─ Buffer                           429 MB
────────────────────────────────────────────
   TOTAL REQUIRED:                 4,000 MB (4 GB)
   WITH CACHING:                   1,000 MB (1 GB)
```

### Scenario D: Video Recording

```
✅ Interactive demo + screen recording
🎯 Goal: Run ./demo-orbit-record.sh

Required:
├─ Interactive demo                4,000 MB
├─ Video file (5 min @ 1080p)       500 MB
├─ Thumbnail                          1 MB
└─ Buffer                           499 MB
────────────────────────────────────────────
   TOTAL REQUIRED:                 5,000 MB (5 GB)
   RECOMMENDED:                    8,000 MB (8 GB)
```

### Scenario E: Docker Deployment

```
✅ Full containerized environment
🎯 Goal: docker-compose up

Required:
├─ Base images                     1,000 MB
├─ Build cache                     2,500 MB
├─ Final images                      500 MB
├─ Volumes (data)                    500 MB
└─ Buffer                          1,500 MB
────────────────────────────────────────────
   TOTAL REQUIRED:                 6,000 MB (6 GB)
   RECOMMENDED:                   10,000 MB (10 GB)
```

## Disk Space Management

### Before Running Demo

#### Check Available Space

```bash
# Unix/Linux/macOS
df -h . | awk 'NR==2 {print "Available: " $4}'

# Windows (PowerShell)
Get-PSDrive C | Select-Object @{Name="Free GB";Expression={[math]::Round($_.Free/1GB,2)}}

# Docker
docker system df
```

#### Free Up Space (If Needed)

```bash
# Clean Rust artifacts (saves ~3GB)
cargo clean

# Clean Node modules (saves ~500MB, requires reinstall)
rm -rf dashboard/node_modules

# Clean previous demo data (if cleanup failed)
rm -rf /tmp/orbit_demo_*

# Docker cleanup
docker system prune -a --volumes  # ⚠️ Removes ALL unused Docker data

# Remove old logs
rm -f orbit-*.log
```

### After Running Demo

#### Automatic Cleanup

The demo scripts automatically clean up:
- ✅ Demo source data (`/tmp/orbit_demo_source_*`)
- ✅ Demo destination data (`/tmp/orbit_demo_dest_*`)
- ✅ Background processes (server, dashboard)

#### Persistent Files Remaining

These stay on disk for future runs:
- `target/` - Rust build cache (reuse for faster builds)
- `dashboard/node_modules/` - NPM dependencies
- `crates/orbit-web/magnetar.db` - Job database
- `*.log` - Application logs
- `demo-recordings/*.mp4` - Video files (if recorded)

#### Manual Cleanup

```bash
# Keep build cache, remove demo artifacts only
rm -f orbit-*.log
rm -f e2e-metrics.json

# Full cleanup (next run will be slower)
cargo clean
rm -rf dashboard/node_modules
rm -f crates/orbit-web/magnetar.db
rm -rf demo-recordings/

# Docker full cleanup
docker-compose -f docker-compose.demo.yml down -v
docker system prune -a
```

## Optimization Strategies

### Strategy 1: Pre-Build Binaries

**Saves:** 2-3 GB (avoid repeated compilation)

```bash
# One-time build
cd crates/orbit-web
cargo build --release --bin orbit-server

# Demo will detect existing binary and skip rebuild
./demo-orbit.sh  # Uses cached binary
```

### Strategy 2: Reduce Test Data Size

**Saves:** 140 MB (smaller demo files)

Edit `demo-orbit.sh` or `demo-orbit-ci.sh`:

```bash
# Original (170MB total)
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_alpha.bin" bs=1M count=50
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_beta.bin" bs=1M count=20
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_gamma.bin" bs=1M count=100

# Optimized (30MB total) - saves 140MB
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_alpha.bin" bs=1M count=10
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_beta.bin" bs=1M count=5
dd if=/dev/urandom of="$DEMO_SOURCE/telemetry_gamma.bin" bs=1M count=15
```

### Strategy 3: Lower Video Quality

**Saves:** 300-400 MB per recording

Edit `demo-orbit-record.sh`:

```bash
# High quality (default) - ~500MB for 5 min
ffmpeg ... -s 1920x1080 -preset ultrafast -crf 18 ...

# Medium quality - ~185MB for 5 min (saves ~315MB)
ffmpeg ... -s 1920x1080 -preset fast -crf 23 ...

# Low quality - ~75MB for 5 min (saves ~425MB)
ffmpeg ... -s 1280x720 -preset veryfast -crf 28 ...
```

### Strategy 4: Use Docker Multi-Stage Builds

**Saves:** 2 GB (smaller final image)

Already implemented in `Dockerfile.demo`:
- Builder stages: ~3 GB
- Final image: ~200 MB
- Intermediate layers pruned automatically

### Strategy 5: CI/CD Optimized Mode

**Saves:** 140 MB (smaller test data)

CI scripts (`demo-orbit-ci.sh/bat`) use reduced data:
- Source: 35 MB (vs 170 MB)
- Destination: 35 MB (vs 170 MB)
- Total savings: 270 MB

## Monitoring Disk Usage

### Real-Time Monitoring

```bash
# Watch disk usage during demo (Unix/Linux/macOS)
watch -n 5 'df -h . | tail -1'

# Windows (PowerShell) - run in separate window
while ($true) { Get-PSDrive C | Select-Object Used,Free; Start-Sleep 5 }

# Docker disk usage
docker system df --verbose
```

### Set Alerts

```bash
# Bash script to warn if space < 2GB
check_space() {
  FREE=$(df -k . | awk 'NR==2 {print $4}')
  if [ $FREE -lt 2097152 ]; then  # 2GB in KB
    echo "⚠️  WARNING: Less than 2GB free!"
    return 1
  fi
}

# Run before demo
check_space || exit 1
./demo-orbit.sh
```

## FAQs

### Q: Why does the first build need 4GB but subsequent runs only 400MB?

**A:** The first build compiles Rust (~3GB in `target/`) and downloads Node.js dependencies (~500MB in `node_modules/`). These are cached and reused for future runs. Only the demo data (340MB) needs to be regenerated each time.

### Q: Can I run the demo on a system with only 2GB free?

**A:** Only if you use pre-built binaries:
```bash
# Build on a machine with more space
cargo build --release --bin orbit-server

# Copy binary to target machine
scp target/release/orbit-server user@target:/path/to/orbit/

# Run demo (will skip build, only need 400MB)
./demo-orbit.sh
```

### Q: What if I run out of space during the demo?

**A:** The demo will fail with "No space left on device". To recover:
1. Kill demo processes: `pkill -f orbit`
2. Clean up manually: `rm -rf /tmp/orbit_demo_*`
3. Free up space using strategies above
4. Re-run demo

### Q: How do I track space usage over time?

**A:** Use the metrics analyzer:
```bash
# After each demo run
du -sh target/ dashboard/node_modules/ /tmp/orbit_demo_* >> space-usage.log
```

### Q: Can I use a different temp directory with more space?

**A:** Yes! Edit the demo script:
```bash
# Change this line in demo-orbit.sh
DEMO_SOURCE="/mnt/large-drive/orbit_demo_source_$(date +%s)"
DEMO_DEST="/mnt/large-drive/orbit_demo_dest_$(date +%s)"
```

---

**Orbit v2.2.0-alpha** - The intelligent file transfer tool that never gives up 💪
