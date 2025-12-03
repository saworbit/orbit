# Running Orbit v2.2 - Web Platform Guide

This guide covers running the Orbit v2.2 architecture, which includes:
- **Rust Control Plane** (API Server) on port 8080
- **React Dashboard** (Frontend UI) on port 5173

## Quick Start

### Automated Launch (Recommended)

The easiest way to run both services is using the launchpad scripts.

#### On Linux/macOS:
```bash
./launch-orbit.sh
```

#### On Windows:
```cmd
launch-orbit.bat
```

Or double-click the `.bat` file in Windows Explorer.

---

## What the Launch Scripts Do

Both scripts handle the complete startup sequence:

1. **System Diagnostic**
   - Checks for Rust/Cargo installation
   - Checks for Node.js/npm installation
   - Fails fast if dependencies are missing

2. **Build Phase**
   - Compiles the Rust backend (`orbit-server`)
   - Installs npm dependencies if not present
   - Shows progress with spinners

3. **Service Launch**
   - Starts Rust API on `http://localhost:8080`
   - Starts React UI on `http://localhost:5173`
   - Both run in parallel

4. **Health Check**
   - Waits for API to be ready
   - Checks `/api/health` endpoint
   - Fails after 30 retries (15 seconds)

5. **Auto-Open Browser**
   - Opens dashboard at `http://localhost:5173`
   - Works on Windows, macOS, and Linux

6. **Graceful Shutdown**
   - Press **Ctrl+C** (Bash) or **Any Key** (Batch)
   - Kills both processes cleanly
   - No orphaned processes

---

## Manual Launch (Advanced)

If you need more control, run the services separately.

### Terminal 1: Start the Rust API
```bash
cd crates/orbit-web
cargo run --bin orbit-server
```

The API will be available at:
- Health: `http://localhost:8080/api/health`
- Swagger: `http://localhost:8080/swagger-ui`

### Terminal 2: Start the React Dashboard
```bash
cd dashboard
npm install  # First time only
npm run dev
```

The dashboard will be available at:
- UI: `http://localhost:5173`

---

## Prerequisites

### Required Software

| Tool | Minimum Version | Check Command |
|------|----------------|---------------|
| **Rust** | 1.70+ | `cargo --version` |
| **Node.js** | 18+ | `node --version` |
| **npm** | 9+ | `npm --version` |

### Installing Prerequisites

#### Rust
```bash
# Linux/macOS/Windows
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Or visit: https://rustup.rs/

#### Node.js
- **Linux**: Use your package manager or [nvm](https://github.com/nvm-sh/nvm)
- **macOS**: `brew install node` or download from [nodejs.org](https://nodejs.org/)
- **Windows**: Download installer from [nodejs.org](https://nodejs.org/)

---

## Troubleshooting

### "Rust is not installed"
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart your terminal, then verify
cargo --version
```

### "Node.js is not installed"
Download and install from https://nodejs.org/

### Port Already in Use

If port 8080 or 5173 is already taken:

**Find the process:**
```bash
# Linux/macOS
lsof -i :8080
lsof -i :5173

# Windows
netstat -ano | findstr :8080
netstat -ano | findstr :5173
```

**Kill the process:**
```bash
# Linux/macOS
kill -9 <PID>

# Windows
taskkill /F /PID <PID>
```

### Build Fails

Check the log files:
- **Bash**: `/tmp/orbit_build.log`
- **Batch**: `orbit_build.log` (in project root)

Common fixes:
```bash
# Clean and rebuild
cargo clean
cargo build
```

### API Health Check Timeout

If the script times out waiting for the API:

1. Check if the server started:
   ```bash
   # Bash
   tail -f orbit_server.log

   # Or check manually
   curl http://localhost:8080/api/health
   ```

2. Look for errors in the server output

3. Try running manually to see detailed logs:
   ```bash
   cd crates/orbit-web
   RUST_LOG=debug cargo run --bin orbit-server
   ```

### "Permission Denied" (Bash Script)

The script needs execute permissions:
```bash
chmod +x launch-orbit.sh
```

### Windows Batch Script Shows Weird Characters

Your terminal doesn't support ANSI colors. This doesn't affect functionality, just visual output.

Update to Windows 10/11 or use Windows Terminal: https://aka.ms/terminal

---

## Environment Variables

You can customize behavior with environment variables:

### Rust API
```bash
# Set log level
export RUST_LOG=debug

# Change port (requires code modification)
export ORBIT_PORT=8080
```

### React Dashboard
```bash
# Change Vite port
export VITE_PORT=5173

# Point to different API (if needed)
export VITE_API_URL=http://localhost:8080
```

---

## Development Workflow

### Typical Development Session

1. **Start services:**
   ```bash
   ./launch-orbit.sh
   ```

2. **Make changes** to either:
   - Rust code in `crates/orbit-web/`
   - React code in `dashboard/src/`

3. **Hot reload:**
   - **React**: Changes auto-reload (Vite HMR)
   - **Rust**: Requires manual restart (Ctrl+C, re-run script)

4. **Test API endpoints:**
   ```bash
   # Check health
   curl http://localhost:8080/api/health

   # Explore Swagger docs
   open http://localhost:8080/swagger-ui
   ```

5. **Stop services:**
   - Press **Ctrl+C** (or any key on Windows)

### Running Tests

```bash
# Test Rust backend
cargo test

# Test React frontend
cd dashboard
npm test
```

---

## Production Deployment

The launch scripts are for **development only**. For production:

### Build for Production

```bash
# Build optimized Rust binary
cargo build --release --bin orbit-server

# Build optimized React bundle
cd dashboard
npm run build
```

### Serve in Production

1. **Backend**: Run the release binary
   ```bash
   ./target/release/orbit-server
   ```

2. **Frontend**: Serve the `dashboard/dist` folder with nginx, Apache, or any static file server

3. **Reverse Proxy**: Use nginx to route:
   - `/api/*` → Rust backend (8080)
   - `/*` → React static files

---

## Architecture Overview

```
┌─────────────────────────────────────────┐
│  React Dashboard (Port 5173)            │
│  - Vite Dev Server                      │
│  - Hot Module Replacement               │
│  - Proxies API calls to :8080           │
└─────────────────┬───────────────────────┘
                  │
                  │ HTTP Requests
                  ▼
┌─────────────────────────────────────────┐
│  Rust API Server (Port 8080)            │
│  - Actix-web / Axum                     │
│  - REST API + WebSocket support         │
│  - Swagger/OpenAPI docs                 │
└─────────────────────────────────────────┘
```

---

## Available Endpoints

Once running, access:

| Service | URL | Description |
|---------|-----|-------------|
| **Dashboard** | http://localhost:5173 | React UI |
| **API Health** | http://localhost:8080/api/health | Health check endpoint |
| **Swagger UI** | http://localhost:8080/swagger-ui | Interactive API docs |
| **OpenAPI Spec** | http://localhost:8080/api-docs/openapi.json | API specification |

---

## Next Steps

- **Explore the API**: Visit http://localhost:8080/swagger-ui
- **Build a feature**: See [Development Guide](../architecture/ORBIT_WEB_IMPLEMENTATION_SUMMARY.md)
- **Configure backends**: See [Backend Guide](BACKEND_GUIDE.md)
- **Deploy to production**: See deployment section above

---

## Getting Help

- **Architecture docs**: [ORBIT_WEB_IMPLEMENTATION_SUMMARY.md](../architecture/ORBIT_WEB_IMPLEMENTATION_SUMMARY.md)
- **API implementation**: [WEB_GUI.md](../architecture/WEB_GUI.md)
- **Report issues**: https://github.com/saworbit/orbit/issues

---

**Happy coding!** 🚀
