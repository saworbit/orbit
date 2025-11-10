# Orbit Web GUI - Complete Guide

**Modern web interface for orchestrating Orbit file transfers with real-time monitoring**

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Installation & Setup](#installation--setup)
- [Quick Start](#quick-start)
- [Features](#features)
- [User Interface](#user-interface)
- [API Reference](#api-reference)
- [Configuration](#configuration)
- [Development](#development)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)
- [Examples](#examples)
- [Roadmap](#roadmap)

---

## Overview

**Orbit Web** is a full-stack Rust web application that provides a modern, reactive interface for managing Orbit file transfers. Built with Leptos and Axum, it combines server-side rendering with client-side reactivity for optimal performance and user experience.

### Key Benefits

- **Real-time Monitoring** - WebSocket-based live updates for job progress
- **Zero Configuration** - Works out-of-the-box with sensible defaults
- **Crash Recovery** - Persistent state via Magnetar ensures no data loss
- **Production Ready** - Built with enterprise-grade Rust technologies
- **Modular Design** - Clean separation from Orbit core engine
- **Lightweight** - Minimal resource footprint, runs efficiently

### Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Frontend Framework | Leptos 0.6 | Reactive UI with server-side rendering |
| Backend Server | Axum 0.7 | High-performance async HTTP server |
| Real-time Communication | WebSockets | Live progress updates |
| State Management | Magnetar (SQLite) | Persistent job state and recovery |
| Styling | Tailwind CSS | Modern, responsive design |
| Async Runtime | Tokio | Efficient async task execution |

---

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      Browser (Client)                        │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Leptos Components (WASM + SSR Hydration)          │    │
│  │  - Dashboard, JobList, JobForm, ProgressBar        │    │
│  └────────────────────────────────────────────────────┘    │
└────────────────┬────────────────────────────────────────────┘
                 │
                 │ HTTP/WebSocket
                 ↓
┌─────────────────────────────────────────────────────────────┐
│                    Axum Web Server                           │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Server Functions (Leptos)                         │    │
│  │  - list_jobs, create_job, get_job_stats           │    │
│  └────────────────────────────────────────────────────┘    │
│  ┌────────────────────────────────────────────────────┐    │
│  │  WebSocket Handler                                 │    │
│  │  - Progress broadcasting per job                   │    │
│  └────────────────────────────────────────────────────┘    │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Progress Registry                                 │    │
│  │  - Broadcast channels (tokio)                      │    │
│  │  - Per-job subscriber management                   │    │
│  └────────────────────────────────────────────────────┘    │
└────────────────┬────────────────────────────────────────────┘
                 │
                 │ SQL Queries
                 ↓
┌─────────────────────────────────────────────────────────────┐
│              Magnetar (Job State Machine)                    │
│  - SQLite database (WAL mode)                               │
│  - Atomic job state transitions                             │
│  - Crash recovery support                                   │
│  - Job statistics and metrics                               │
└─────────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### Frontend (Leptos Components)

- **`App`** - Root component with routing and navigation
- **`Dashboard`** - Main view combining job list and creation form
- **`JobList`** - Auto-refreshing table of active/completed jobs
- **`JobForm`** - Form for creating new transfer jobs
- **`ProgressBar`** - Visual progress indicator with percentage
- **`About`** - Information and documentation page

#### Backend (Axum + Leptos Server)

- **Server Functions** - RPC-style functions callable from client
- **WebSocket Handler** - Real-time progress streaming
- **Progress Registry** - Manages broadcast channels per job
- **Health Check** - Service health monitoring endpoint

#### Data Layer (Magnetar)

- **Job Storage** - Persistent SQLite database
- **State Machine** - Atomic job state transitions
- **Statistics** - Job metrics and completion tracking

---

## Installation & Setup

### Prerequisites

1. **Rust Toolchain** (1.70+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **WASM Target**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **Cargo Leptos** (development tool)
   ```bash
   cargo install cargo-leptos
   ```

### Installation

```bash
# Clone the Orbit repository
git clone https://github.com/saworbit/orbit.git
cd orbit

# Navigate to orbit-web
cd crates/orbit-web

# Build the project
cargo leptos build --release
```

---

## Quick Start

### Development Mode

Start the development server with hot-reload:

```bash
cd crates/orbit-web
cargo leptos watch
```

The server will start at **http://127.0.0.1:8080** with automatic browser refresh on code changes.

### Production Mode

Build and run for production:

```bash
# Build optimized release
cargo leptos build --release

# Run the server
cargo run --release
```

### Standalone Binary

Run the compiled binary directly:

```bash
./target/release/orbit-web
```

---

## Features

### 1. Job Management

**Create Jobs**
- Configure source and destination paths
- Enable/disable compression
- Toggle checksum verification
- Set parallel worker count (1-16)
- Visual slider for worker configuration

**View Jobs**
- List all active and completed jobs
- Real-time status updates (pending, processing, completed, failed)
- Completion percentage display
- Chunk-level progress (done/total/failed)

**Job Actions**
- View detailed job statistics
- Delete completed/failed jobs
- Monitor real-time progress

### 2. Real-Time Progress Tracking

**Progress Updates**
- Auto-refreshing job list (2-second interval)
- WebSocket-based live updates (future enhancement)
- Visual progress bars with color coding:
  - **Blue** - In progress (0-99%)
  - **Green** - Completed (100%)
  - **Gray** - Pending (0%)

**Statistics Display**
- Total chunks processed
- Pending/processing/done/failed counts
- Completion percentage
- Job status indicators

### 3. User Interface

**Responsive Design**
- Desktop-optimized layout
- Mobile-friendly (responsive grid)
- Tailwind CSS styling
- Clean, modern aesthetic

**Navigation**
- Dashboard (main view)
- About page (documentation)
- 404 error handling

**User Experience**
- Loading states with suspense boundaries
- Error display with user-friendly messages
- Form validation and feedback
- Visual feedback on actions

---

## User Interface

### Dashboard

The main dashboard provides a comprehensive view of all transfer operations:

```
┌─────────────────────────────────────────────────────────────┐
│  Orbit Web                              Dashboard | About    │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────┐  ┌──────────────────────────────────┐ │
│  │  Create New Job  │  │  Active Jobs                     │ │
│  │                  │  │                                  │ │
│  │  Source Path:    │  │  ┌────┬─────────┬────────┬─────┐ │ │
│  │  [___________]   │  │  │ ID │ Source  │ Status │ ... │ │ │
│  │                  │  │  ├────┼─────────┼────────┼─────┤ │ │
│  │  Dest Path:      │  │  │ 1  │ Job 1   │ Done   │ ███ │ │ │
│  │  [___________]   │  │  │ 2  │ Job 2   │ Active │ ▓▓░ │ │ │
│  │                  │  │  └────┴─────────┴────────┴─────┘ │ │
│  │  [✓] Compress    │  │                                  │ │
│  │  [✓] Verify      │  │  Refresh                         │ │
│  │                  │  │                                  │ │
│  │  Workers: 4      │  └──────────────────────────────────┘ │
│  │  [====|====]     │                                        │
│  │                  │                                        │
│  │  [Create Job]    │                                        │
│  └──────────────────┘                                        │
└─────────────────────────────────────────────────────────────┘
```

### Job Creation Form

Intuitive form with real-time validation:

- **Source Path** - Input field for source location
- **Destination Path** - Input field for destination location
- **Compression Toggle** - Enable/disable compression
- **Verification Toggle** - Enable/disable checksum verification
- **Parallel Workers** - Slider (1-16 workers) with live value display
- **Create Button** - Submits job with loading state
- **Status Display** - Success/error messages below form

### Job List Table

Auto-refreshing table with comprehensive job information:

| Column | Description |
|--------|-------------|
| ID | Unique job identifier |
| Source | Source path or description |
| Status | Current state (completed, failed, processing, pending) |
| Progress | Visual progress bar with percentage |
| Chunks | Breakdown (done/total, failed count) |

---

## API Reference

### HTTP Endpoints

#### Health Check
```
GET /api/health
```

**Response:**
```json
{
  "status": "ok",
  "service": "orbit-web",
  "version": "0.1.0"
}
```

### Leptos Server Functions

All server functions use POST requests to `/api/<function_name>`:

#### List Jobs
```
POST /api/list_jobs
```

**Response:**
```json
[
  {
    "id": "1",
    "source": "Job 1",
    "destination": "Dest 1",
    "status": "completed",
    "total_chunks": 100,
    "pending": 0,
    "processing": 0,
    "done": 100,
    "failed": 0,
    "completion_percent": 100.0
  }
]
```

#### Create Job
```
POST /api/create_job
```

**Request Body:**
```json
{
  "source": "/path/to/source",
  "destination": "/path/to/dest",
  "compress": true,
  "verify": true,
  "parallel": 4
}
```

**Response:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### Get Job Statistics
```
POST /api/get_job_stats
```

**Request Body:**
```json
{
  "job_id": "1"
}
```

**Response:**
```json
{
  "id": "1",
  "status": "processing",
  "total_chunks": 100,
  "done": 65,
  "pending": 20,
  "processing": 10,
  "failed": 5,
  "completion_percent": 65.0
}
```

#### Delete Job
```
POST /api/delete_job
```

**Request Body:**
```json
{
  "job_id": "1"
}
```

### WebSocket Endpoints

#### Progress Updates
```
WS /ws/progress/:job_id
```

Connect to receive real-time progress updates for a specific job.

**Message Format:**
```json
{
  "job_id": "1",
  "bytes_transferred": 1048576,
  "total_bytes": 10485760,
  "speed_mbps": 125.5,
  "eta_seconds": 72,
  "current_file": "/path/to/file.txt"
}
```

---

## Configuration

### Environment Variables

```bash
# Database path (default: orbit-web.db)
export ORBIT_WEB_DB=/var/lib/orbit/web.db

# Log level (default: info)
export RUST_LOG=info,orbit_web=debug

# Server address (default: 127.0.0.1:8080)
export LEPTOS_SITE_ADDR=0.0.0.0:8080

# Reload port for development (default: 3001)
export LEPTOS_RELOAD_PORT=3001
```

### Leptos.toml Configuration

The `Leptos.toml` file configures build and runtime settings:

```toml
[package]
name = "orbit-web"
output-name = "orbit-web"

# Server configuration
site-addr = "127.0.0.1:8080"
reload-port = 3001

# Build configuration
site-root = "target/site"
site-pkg-dir = "pkg"

# Features
ssr-features = ["ssr"]
csr-features = ["hydrate"]

# Tailwind
tailwind-input-file = "style/input.css"
tailwind-config-file = "tailwind.config.js"
```

---

## Development

### Project Structure

```
crates/orbit-web/
├── Cargo.toml              # Dependencies and metadata
├── Leptos.toml             # Leptos build configuration
├── README.md               # Quick reference guide
├── .gitignore              # Build artifacts exclusions
└── src/
    ├── main.rs             # Axum server entry point
    ├── lib.rs              # Library exports
    ├── app.rs              # Root Leptos component
    ├── error.rs            # Error types and handling
    ├── types.rs            # Shared data structures
    ├── progress.rs         # Progress registry
    ├── server_fns.rs       # Server function definitions
    └── components/
        ├── mod.rs          # Component exports
        ├── dashboard.rs    # Main dashboard layout
        ├── job_form.rs     # Job creation form
        ├── job_list.rs     # Job list table
        └── progress_bar.rs # Progress visualization
```

### Development Workflow

1. **Start Development Server**
   ```bash
   cargo leptos watch
   ```

2. **Make Changes**
   - Edit source files in `src/`
   - Browser auto-reloads on save
   - Compilation errors shown in terminal

3. **Test Changes**
   - Create test jobs via the form
   - Verify progress updates
   - Check error handling

4. **Build for Production**
   ```bash
   cargo leptos build --release
   ```

### Adding New Features

#### Add a New Component

1. Create component file: `src/components/my_component.rs`
2. Implement the component:
   ```rust
   use leptos::*;

   #[component]
   pub fn MyComponent() -> impl IntoView {
       view! {
           <div class="my-component">
               "Hello, World!"
           </div>
       }
   }
   ```
3. Export in `src/components/mod.rs`:
   ```rust
   pub mod my_component;
   pub use my_component::MyComponent;
   ```
4. Use in parent component:
   ```rust
   use crate::components::MyComponent;

   view! {
       <MyComponent />
   }
   ```

#### Add a Server Function

1. Define in `src/server_fns.rs`:
   ```rust
   #[server(MyFunction, "/api")]
   pub async fn my_function(param: String) -> Result<String, ServerFnError> {
       // Implementation
       Ok("result".to_string())
   }
   ```

2. Call from component:
   ```rust
   let my_action = create_action(|param: &String| {
       let param = param.clone();
       async move { my_function(param).await }
   });
   ```

---

## Deployment

### Production Build

```bash
# Build optimized release
cargo leptos build --release

# Binary location
./target/release/orbit-web
```

### Systemd Service

Create `/etc/systemd/system/orbit-web.service`:

```ini
[Unit]
Description=Orbit Web GUI
After=network.target

[Service]
Type=simple
User=orbit
WorkingDirectory=/opt/orbit-web
Environment="ORBIT_WEB_DB=/var/lib/orbit/web.db"
Environment="RUST_LOG=info,orbit_web=info"
ExecStart=/opt/orbit-web/orbit-web
Restart=always

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable orbit-web
sudo systemctl start orbit-web
sudo systemctl status orbit-web
```

### Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name orbit.example.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # WebSocket support
    location /ws/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### Docker Deployment

Create `Dockerfile`:

```dockerfile
FROM rust:1.75 AS builder

# Install cargo-leptos
RUN cargo install cargo-leptos

# Add WASM target
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
COPY . .

# Build
RUN cargo leptos build --release

FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /app/target/release/orbit-web .
COPY --from=builder /app/target/site ./site

ENV ORBIT_WEB_DB=/data/orbit-web.db
ENV RUST_LOG=info

EXPOSE 8080
CMD ["./orbit-web"]
```

Build and run:
```bash
docker build -t orbit-web .
docker run -p 8080:8080 -v orbit-data:/data orbit-web
```

---

## Troubleshooting

### Common Issues

#### Port Already in Use

**Error:** `Address already in use (os error 98)`

**Solution:**
```bash
# Find process using port 8080
lsof -i :8080

# Kill the process
kill -9 <PID>

# Or change port
export LEPTOS_SITE_ADDR=127.0.0.1:3000
```

#### Database Locked

**Error:** `database is locked`

**Solution:**
- Ensure no other Orbit processes are accessing the database
- Check for stale lock files
- Use WAL mode (enabled by default in Magnetar)

#### WASM Compilation Fails

**Error:** `wasm32-unknown-unknown not found`

**Solution:**
```bash
rustup target add wasm32-unknown-unknown
```

#### WebSocket Connection Failed

**Error:** Connection refused on `/ws/progress/:id`

**Solution:**
- Verify job ID exists in database
- Check progress registry has channel for job
- Ensure WebSocket route is properly configured
- Check firewall/proxy WebSocket support

---

## Examples

### Creating a Job Programmatically

```rust
use orbit_web::types::CreateJobRequest;

let request = CreateJobRequest {
    source: "/data/source".to_string(),
    destination: "/backup/dest".to_string(),
    compress: true,
    verify: true,
    parallel: Some(8),
};

// Via server function
let job_id = create_job(request).await?;
println!("Created job: {}", job_id);
```

### Monitoring Progress via WebSocket

```javascript
const jobId = "550e8400-e29b-41d4-a716-446655440000";
const ws = new WebSocket(`ws://localhost:8080/ws/progress/${jobId}`);

ws.onmessage = (event) => {
    const progress = JSON.parse(event.data);
    console.log(`Progress: ${progress.bytes_transferred}/${progress.total_bytes}`);
    console.log(`Speed: ${progress.speed_mbps} MB/s`);
    console.log(`ETA: ${progress.eta_seconds} seconds`);
};
```

---

## Roadmap

### v0.2.0 - Enhanced Monitoring
- [ ] Log tail viewer with live streaming
- [ ] Job pause/resume controls
- [ ] Advanced filtering and search
- [ ] Export job history to CSV/JSON

### v0.3.0 - Manifest Integration
- [ ] Drag-and-drop manifest upload
- [ ] Visual manifest editor
- [ ] Manifest validation and preview
- [ ] Template management

### v0.4.0 - Analytics & Reporting
- [ ] Parquet export integration
- [ ] Interactive charts (Chart.js/D3)
- [ ] Transfer statistics dashboard
- [ ] Historical trend analysis

### v0.5.0 - Security & Auth
- [ ] TLS/HTTPS support
- [ ] Basic authentication
- [ ] Role-based access control (RBAC)
- [ ] API key management

### v0.6.0 - Advanced Features
- [ ] Dark mode theme
- [ ] Mobile PWA support
- [ ] Multi-language support (i18n)
- [ ] Notification system

---

## Contributing

Contributions welcome! See the main [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

### Areas for Contribution

- UI/UX improvements
- Additional chart visualizations
- Mobile responsive enhancements
- Documentation and examples
- Testing and bug fixes

---

## License

Apache License 2.0 - See [LICENSE](../LICENSE) for details.

---

## Support

- **GitHub Issues:** [saworbit/orbit/issues](https://github.com/saworbit/orbit/issues)
- **Documentation:** [README.md](../README.md)
- **Web GUI README:** [crates/orbit-web/README.md](../crates/orbit-web/README.md)

---

**Built with ❤️ and 🦀 for the Orbit project**
