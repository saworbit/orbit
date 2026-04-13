# 🐳 Orbit Docker Demo Guide

## Overview

Run the Orbit E2E demonstration in an isolated, reproducible Docker environment. Perfect for CI/CD pipelines, development, and demonstrations across different platforms.

## 🛡️ Safety Considerations for Docker

**Docker provides additional isolation**, but you should still verify your system is ready:

### Native System Validation (Recommended)

Even when using Docker, you can run the safety validator to check host system requirements:

```bash
# Check host system (disk space, Docker availability, ports)
./scripts/validate-demo-safety.sh

# Docker-specific checks
docker system df  # Check Docker disk usage
docker info       # Verify Docker is running
```

### Docker-Specific Safety Features

Docker deployment is inherently safer than native execution because:

- ✅ **Isolated File System** - Demo runs in container, not directly on your host
- ✅ **Named Volumes** - Data stored in managed Docker volumes
- ✅ **Port Mapping** - Only specified ports exposed to host
- ✅ **Resource Limits** - Can set memory and CPU limits
- ✅ **Easy Cleanup** - `docker-compose down -v` removes everything

**What the Docker demo WILL do:**
- Create Docker images (~4-6GB on first build)
- Create named volumes for data persistence
- Expose ports 8080 and 5173 on localhost
- Create containers that run the demo

**What it WON'T do:**
- Modify files outside the container
- Require sudo/admin (if Docker is configured for your user)
- Leave processes running after `docker-compose down`
- Access your network beyond localhost

📖 **For complete safety documentation**, see [SAFETY_FIRST.md](SAFETY_FIRST.md) - most safety assurances apply to Docker as well, with additional container isolation benefits.

## Quick Start

### Prerequisites

- Docker Engine 20.10+ or Docker Desktop
- docker-compose 2.0+
- At least 4GB of available RAM
- **Minimum 6GB of free disk space** (10GB recommended)

#### Disk Space Requirements

| Component | Space Required | Notes |
|-----------|----------------|-------|
| **Base Images** | 1-1.5 GB | Rust (700MB), Node (200MB), Debian (100MB) |
| **Build Cache** | 2-3 GB | Intermediate compilation artifacts |
| **Final Images** | 200-500 MB | Multi-stage build output |
| **Volumes** | 500 MB | Demo data, database, logs |
| **Total First Build** | **4-6 GB** | One-time cost |
| **Subsequent Runs** | **500 MB** | Only volume data changes |

**Space-Saving Tips:**
```bash
# Remove build cache after successful build
docker builder prune -a

# Remove stopped containers
docker container prune

# Remove unused volumes
docker volume prune

# Check Docker disk usage
docker system df

# Full cleanup (⚠️ removes everything)
docker system prune -a --volumes
```

### Option 1: Full Stack with Demo (Recommended)

Run the complete stack (backend + frontend) and execute the demo scenario:

```bash
# Build images
docker-compose -f docker-compose.demo.yml build

# Start services and run demo
docker-compose -f docker-compose.demo.yml --profile demo up

# Or run services in background and demo interactively
docker-compose -f docker-compose.demo.yml up -d
docker-compose -f docker-compose.demo.yml run --rm orbit-demo
```

### Option 2: Services Only (No Auto-Demo)

Run just the backend and frontend without the automated demo:

```bash
# Start services
docker-compose -f docker-compose.demo.yml up

# Access dashboard at http://localhost:5173
# Access API at http://localhost:8080
```

### Option 3: Headless CI/CD Mode

For automated testing in pipelines:

```bash
# Set environment for headless operation
export ORBIT_DEMO_HEADLESS=true
export ORBIT_DEMO_AUTO_CONFIRM=true

# Run complete E2E test
docker-compose -f docker-compose.demo.yml --profile demo up --abort-on-container-exit

# Check exit code
echo $?
```

## Architecture

```
┌─────────────────────────────────┐
│   orbit-dashboard (Node.js)     │  Port 5173
│   React 19 + Vite + TypeScript  │
└────────────┬────────────────────┘
             │ HTTP/WebSocket
             ↓
┌─────────────────────────────────┐
│   orbit-server (Rust)           │  Port 8080
│   Axum + SQLite (Magnetar)      │
└─────────────────────────────────┘
             │
             ↓
┌─────────────────────────────────┐
│   orbit-demo (Optional)         │
│   E2E Test Orchestrator         │
└─────────────────────────────────┘
```

## Services

### orbit-server

**Image:** `orbit-demo-server:latest`
**Port:** 8080
**Purpose:** Rust-based Control Plane API

**Health Check:**
```bash
curl http://localhost:8080/api/health
```

**Logs:**
```bash
docker-compose -f docker-compose.demo.yml logs -f orbit-server
```

### orbit-dashboard

**Image:** `node:20-alpine`
**Port:** 5173
**Purpose:** React development server with HMR

**Access:**
- Dashboard: http://localhost:5173
- Vite HMR: WebSocket on 5173

**Logs:**
```bash
docker-compose -f docker-compose.demo.yml logs -f orbit-dashboard
```

### orbit-demo (profile: demo)

**Image:** `orbit-demo-server:latest`
**Purpose:** Automated E2E test orchestrator

**Run manually:**
```bash
docker-compose -f docker-compose.demo.yml run --rm orbit-demo
```

## Environment Variables

### Server Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Logging level (trace, debug, info, warn, error) |
| `ORBIT_JWT_SECRET` | `demo-secret-...` | JWT signing secret (⚠️ change for production) |
| `DATABASE_URL` | `sqlite:///app/data/magnetar.db` | SQLite database path |
| `API_PORT` | `8080` | API server port |

### Dashboard Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_API_URL` | `http://localhost:8080` | Backend API URL |
| `NODE_ENV` | `development` | Node environment |

### Demo Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `ORBIT_DEMO_HEADLESS` | `false` | Run without user interaction |
| `ORBIT_DEMO_AUTO_CONFIRM` | `false` | Auto-confirm all prompts |
| `API_URL` | `http://orbit-server:8080` | Internal API URL |

## Volumes

### Persistent Data

| Volume | Purpose | Location |
|--------|---------|----------|
| `orbit-demo-data` | Database and job state | `/app/data` |
| `orbit-demo-logs` | Application logs | `/app/logs` |
| `orbit-demo-source` | Demo source files | `/tmp/orbit_demo_source` |
| `orbit-demo-dest` | Demo destination files | `/tmp/orbit_demo_dest` |
| `orbit-dashboard-node-modules` | Cached NPM dependencies | `/app/node_modules` |

### Managing Volumes

```bash
# List volumes
docker volume ls | grep orbit

# Inspect volume
docker volume inspect orbit-demo-data

# Remove all demo volumes (⚠️ deletes all data)
docker-compose -f docker-compose.demo.yml down -v

# Backup database
docker run --rm -v orbit-demo-data:/data -v $(pwd):/backup \
  alpine tar czf /backup/orbit-demo-backup.tar.gz -C /data .

# Restore database
docker run --rm -v orbit-demo-data:/data -v $(pwd):/backup \
  alpine tar xzf /backup/orbit-demo-backup.tar.gz -C /data
```

## Development Workflow

### Hot Reload Development

The dashboard supports hot module replacement (HMR):

```bash
# Start services
docker-compose -f docker-compose.demo.yml up

# Edit files in ./dashboard/src/
# Changes are automatically reflected in the browser
```

### Rebuilding After Code Changes

```bash
# Rebuild specific service
docker-compose -f docker-compose.demo.yml build orbit-server

# Rebuild and restart
docker-compose -f docker-compose.demo.yml up --build -d orbit-server
```

### Accessing Container Shell

```bash
# Server container
docker-compose -f docker-compose.demo.yml exec orbit-server bash

# Dashboard container
docker-compose -f docker-compose.demo.yml exec orbit-dashboard sh

# Demo container (one-off)
docker-compose -f docker-compose.demo.yml run --rm --entrypoint bash orbit-demo
```

## Troubleshooting

### Port Conflicts

If ports 8080 or 5173 are already in use:

```bash
# Check what's using the port
lsof -i :8080
lsof -i :5173

# Or modify docker-compose.demo.yml to use different ports:
ports:
  - "9080:8080"  # API on host port 9080
  - "6173:5173"  # Dashboard on host port 6173
```

### Container Won't Start

```bash
# Check logs
docker-compose -f docker-compose.demo.yml logs orbit-server

# Check container status
docker-compose -f docker-compose.demo.yml ps

# Restart services
docker-compose -f docker-compose.demo.yml restart

# Full reset (⚠️ removes volumes)
docker-compose -f docker-compose.demo.yml down -v
docker-compose -f docker-compose.demo.yml up
```

### Build Failures

```bash
# Clean build (no cache)
docker-compose -f docker-compose.demo.yml build --no-cache

# Prune Docker build cache
docker builder prune -a

# Check disk space
docker system df
```

### Performance Issues

```bash
# Allocate more resources in Docker Desktop:
# Settings > Resources > Advanced
# - CPUs: 4+
# - Memory: 4GB+
# - Swap: 2GB+

# Check resource usage
docker stats

# Optimize node_modules volume (use named volume instead of bind mount)
# Already configured in docker-compose.demo.yml
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Docker E2E Test

on: [push, pull_request]

jobs:
  e2e-docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build images
        run: docker-compose -f docker-compose.demo.yml build

      - name: Run E2E demo
        run: |
          export ORBIT_DEMO_HEADLESS=true
          export ORBIT_DEMO_AUTO_CONFIRM=true
          docker-compose -f docker-compose.demo.yml --profile demo up --abort-on-container-exit

      - name: Upload logs
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: demo-logs
          path: |
            orbit-server.log
            orbit-dashboard.log
```

### GitLab CI Example

```yaml
docker-e2e:
  stage: test
  image: docker:latest
  services:
    - docker:dind
  variables:
    ORBIT_DEMO_HEADLESS: "true"
    ORBIT_DEMO_AUTO_CONFIRM: "true"
  script:
    - docker-compose -f docker-compose.demo.yml build
    - docker-compose -f docker-compose.demo.yml --profile demo up --abort-on-container-exit
  artifacts:
    when: always
    paths:
      - orbit-server.log
      - orbit-dashboard.log
```

## Production Considerations

### Security

```bash
# Generate secure JWT secret
openssl rand -base64 32

# Use with Docker
docker-compose -f docker-compose.demo.yml up \
  -e ORBIT_JWT_SECRET="your-secure-random-secret-here"
```

### Multi-Stage Production Build

The `Dockerfile.demo` uses multi-stage builds for:
- ✅ Smaller final image (~200MB vs 2GB+)
- ✅ No build tools in production image
- ✅ Only runtime dependencies included
- ✅ Security: minimal attack surface

### Resource Limits

Add to `docker-compose.demo.yml`:

```yaml
services:
  orbit-server:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
```

## Advanced Usage

### Custom Network Configuration

```bash
# Use custom network
docker network create orbit-custom
docker-compose -f docker-compose.demo.yml --network orbit-custom up
```

### Monitoring

```bash
# Real-time stats
docker stats orbit-demo-server orbit-demo-dashboard

# Export metrics to Prometheus
# Add prometheus exporter sidecar in docker-compose.demo.yml
```

### Scaling (Future)

```bash
# Scale dashboard instances (requires load balancer)
docker-compose -f docker-compose.demo.yml up --scale orbit-dashboard=3
```

## Cleanup

### Remove Everything

```bash
# Stop and remove containers
docker-compose -f docker-compose.demo.yml down

# Remove volumes too (⚠️ deletes all data)
docker-compose -f docker-compose.demo.yml down -v

# Remove images
docker rmi orbit-demo-server:latest

# Full cleanup (all unused Docker resources)
docker system prune -a --volumes
```

## FAQ

### Q: Can I use this in production?

**A:** The demo setup is optimized for development and testing. For production:
- Change `ORBIT_JWT_SECRET` to a secure random value
- Use production-grade database (PostgreSQL instead of SQLite)
- Enable HTTPS/TLS with reverse proxy (nginx, Traefik, Caddy)
- Set `NODE_ENV=production` and build dashboard statically
- Configure proper backup strategy for volumes

### Q: How do I persist data between restarts?

**A:** Data in named volumes persists automatically. Use `docker-compose down` (without `-v`) to preserve data.

### Q: Can I run this on ARM (Apple Silicon, Raspberry Pi)?

**A:** Yes! Docker will automatically pull ARM-compatible base images. Build might be slower on first run.

### Q: How do I update to the latest version?

```bash
git pull origin main
docker-compose -f docker-compose.demo.yml build --pull
docker-compose -f docker-compose.demo.yml up -d
```

## Support

- 📖 Main Documentation: [README.md](README.md)
- 🛰️ Demo Guide: [DEMO_GUIDE.md](DEMO_GUIDE.md)
- 🐛 Issues: [GitHub Issues](https://github.com/saworbit/orbit/issues)

---

**Orbit v2.2.0-alpha** - The intelligent file transfer tool that never gives up 💪
