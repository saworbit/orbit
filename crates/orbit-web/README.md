# Orbit Nebula v1.0.0-rc.1

**Next-Generation Real-Time Web Control Center for Orbit**

## Status: Release Candidate 1 - Visual Pipeline Builder

This release adds the **Visual Pipeline Builder**, a DAG-based workflow editor for creating complex transfer pipelines with drag-and-drop nodes.

### What's New in RC.1

- **Visual Pipeline Builder** - Create and manage DAG-based transfer workflows
  - Canvas-based visual editor with drag-and-drop nodes
  - 7 node types: Source, Destination, Transform, Filter, Merge, Split, Conditional
  - Node palette with tooltips explaining each node type
  - Quick help guide showing how to build pipelines
  - Click and drag nodes to reposition them
  - Double-click nodes to configure properties
  - Connect nodes by dragging from output port to input port
  - Click edges to delete connections
  - Pipeline status tracking (draft, ready, running, completed, failed, paused)
- **File Browser Integration** - Source/Destination nodes have Browse button to select paths
- **Pipeline Validation** - Validate button checks rules (must have source/destination, paths configured, etc.)
- **Enhanced Node Config** - Each node type has descriptive config with hints and examples
- **10 New API Endpoints** - Full pipeline CRUD with node/edge management

### Previous: beta.2

- File Explorer with directory navigation
- Drag-and-Drop file upload
- User Management panel (Admin only)

### Previous: beta.1

- Job creation form with validation
- Job control buttons (run, cancel, delete)
- Job detail view modal
- Backend management UI (S3, SMB, Local)

## Quick Start

### Option 1: Using Startup Scripts (Recommended)

**Windows:**
```batch
cd crates\orbit-web
start-nebula.bat
```

**Unix/Linux/macOS:**
```bash
cd crates/orbit-web
chmod +x start-nebula.sh
./start-nebula.sh
```

### Option 2: Manual Start

```bash
# Set required environment variable
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)

# Build and run
cargo leptos build --release
cargo leptos serve --release
```

### Access the Dashboard

Open **http://localhost:8080** and login:
- **Username:** `admin`
- **Password:** `orbit2025`

## Demo Jobs

To see the dashboard with sample data, run the demo script:

**With sqlite3 installed:**
```batch
create-demo-jobs.bat
```

**Using the API (server must be running):**
```batch
create-demo-jobs-api.bat
```

This creates 6 sample jobs with various statuses (completed, running, pending, failed).

## Dashboard Features

### Overview Section
- Server status indicator
- Active transfer count
- Completed jobs today
- WebSocket connection status
- Recent activity table

### Jobs Section
- Full job listing with status badges and progress bars
- **"+ New Job"** button to create jobs via modal form
- **Action buttons** per job: Start, Cancel, Details, Delete
- Click job ID to open detail view modal
- Context-sensitive actions based on job status

### Backends Section
- List configured storage backends (S3, SMB, Local)
- Type badges with connection details
- **"+ Add Backend"** button with dynamic form
- Delete backends with confirmation

### API Explorer
- HTTP method selector (GET, POST, DELETE)
- Endpoint input field
- Request body editor for POST requests
- Syntax-highlighted JSON responses
- Quick action buttons for common endpoints

### Pipelines Section (RC.1)
The Visual Pipeline Builder lets you design transfer workflows as a DAG (Directed Acyclic Graph).

**Important:** Pipeline "nodes" are visual building blocks representing transfer operations - they are NOT separate Orbit server instances. You design the workflow in this GUI, then execute it.

- **Canvas Editor** - Visual workspace for building pipelines
- **Quick Help** - Built-in guide showing how to build pipelines
- **Node Palette** - Drag nodes from the palette onto the canvas (hover for tooltips):
  - **Source** - Where files come from. Has Browse button to select folder + optional file pattern filter
  - **Destination** - Where files go to. Has Browse button to select target folder
  - **Filter** - Only pass files matching a pattern (e.g., `*.csv`, `*.txt`)
  - **Transform** - Compress/encrypt data in transit
  - **Merge** - Combine multiple sources into one stream
  - **Split** - Send to multiple destinations (duplicate or round-robin)
  - **Conditional** - Branch based on file extension, size, or name
- **Connections** - Click output port (right side of node) and drag to input port (left side) to connect
- **Configuration** - Double-click any node to configure. Source/Destination have Browse button to use File Explorer
- **Validation** - Click "Validate" to check pipeline rules:
  - Must have at least one Source and one Destination
  - All Source/Destination nodes must have paths configured
  - Source nodes cannot have incoming connections
  - Destination nodes cannot have outgoing connections
  - Warns about unconnected nodes
- **Persistence** - Pipelines and node positions are saved to the database

### WebSocket Monitor
- Connect/disconnect controls
- Live event stream
- Timestamped events
- Clear log functionality

## Architecture

### Tech Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Frontend** | Vanilla JS + CSS | Lightweight interactive UI |
| **Backend** | Axum 0.7 | High-performance async HTTP |
| **Real-time** | WebSockets | Sub-500ms event delivery |
| **Auth** | JWT + Argon2 | Secure token-based auth |
| **Database** | SQLite (sqlx 0.8) | User accounts + jobs |
| **State** | Magnetar + Broadcast | Job persistence + events |
| **Runtime** | Tokio | Async task execution |

### File Structure

```
crates/orbit-web/
├── public/
│   └── index.html          # Interactive dashboard UI
├── src/
│   ├── api/                # REST API endpoints
│   │   ├── auth.rs         # Login, logout, me
│   │   ├── jobs.rs         # Job CRUD operations
│   │   └── backends.rs     # Backend management
│   ├── auth/               # Authentication system
│   │   ├── mod.rs          # JWT, password hashing
│   │   └── models.rs       # User, Role, Claims
│   ├── state.rs            # AppState, OrbitEvent
│   ├── ws.rs               # WebSocket handler
│   └── server.rs           # Axum server setup
├── start-nebula.bat        # Windows startup script
├── start-nebula.sh         # Unix startup script
├── create-demo-jobs.bat    # SQLite demo data script
└── create-demo-jobs-api.bat # API demo data script
```

## API Reference

### Authentication

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/auth/login` | POST | Login with username/password |
| `/api/auth/logout` | POST | Clear authentication cookie |
| `/api/auth/me` | GET | Get current user info |

### Jobs

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/list_jobs` | POST | List all jobs |
| `/api/get_job` | POST | Get single job details |
| `/api/create_job` | POST | Create new job |
| `/api/run_job` | POST | Start a pending job |
| `/api/cancel_job` | POST | Cancel running/pending job |
| `/api/delete_job` | POST | Delete a job |

### Backends

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/list_backends` | POST | List configured backends |
| `/api/create_backend` | POST | Add new backend |
| `/api/delete_backend` | POST | Remove backend |

### Files (beta.2)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/list_dir` | POST | List directory contents |
| `/api/list_drives` | GET | Get available drives/roots |
| `/api/upload_file` | POST | Upload file (multipart) |

### Users (Admin only, beta.2)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/list_users` | POST | List all users |
| `/api/create_user` | POST | Create new user |
| `/api/update_user` | POST | Update user password/role |
| `/api/delete_user` | POST | Delete user |

### Pipelines (RC.1)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/list_pipelines` | POST | List all pipelines |
| `/api/get_pipeline` | POST | Get pipeline with nodes/edges |
| `/api/create_pipeline` | POST | Create new pipeline |
| `/api/update_pipeline` | POST | Update pipeline metadata/status |
| `/api/delete_pipeline` | POST | Delete pipeline |
| `/api/add_node` | POST | Add node to pipeline |
| `/api/update_node` | POST | Update node position/config |
| `/api/remove_node` | POST | Remove node and connected edges |
| `/api/add_edge` | POST | Add edge between nodes |
| `/api/remove_edge` | POST | Remove edge |

### System

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Health check |
| `/ws/events` | WS | Real-time event stream |

## Security

### Current Implementation

- **JWT Tokens**: 24-hour expiration, httpOnly cookies
- **Password Hashing**: Argon2 with salt (OWASP recommended)
- **RBAC**: Role-based permission checks
- **Middleware**: Axum layers for authentication enforcement

### Production Checklist

```bash
# 1. Generate strong JWT secret
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)

# 2. Change default admin password after first login

# 3. Enable HTTPS (required for secure cookies)

# 4. Configure CORS for your domain
```

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ORBIT_JWT_SECRET` | Yes | - | Secret for JWT signing |
| `ORBIT_MAGNETAR_DB` | No | `magnetar.db` | Job database path |
| `ORBIT_USER_DB` | No | `orbit-web-users.db` | User database path |
| `ORBIT_WEB_HOST` | No | `127.0.0.1` | Server bind address |
| `ORBIT_WEB_PORT` | No | `8080` | Server port |
| `RUST_LOG` | No | `info` | Log level |

## Roadmap

### v1.0.0-rc.1 (Current)
- Visual Pipeline Builder with DAG editor
- File browser integration for node paths
- Pipeline validation with error messages
- Enhanced node configuration

### v1.0.0 (Next - Stable)
- Telemetry dashboard with charts and graphs
- PWA support for offline monitoring
- Comprehensive end-to-end testing
- Pipeline execution engine integration

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for detailed version history.

## License

Apache-2.0 - See [LICENSE](../../LICENSE)

---

**Built with Rust by the Orbit team**

*Nebula: Because your data orchestration deserves a control center worthy of the stars.*
