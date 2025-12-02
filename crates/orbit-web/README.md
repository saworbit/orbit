# Orbit Control Plane v2.2.0-alpha

**Headless API and Orchestration Engine**

This directory contains the Orbit Control Plane - a RESTful API server built with Axum that provides job orchestration, backend management, and real-time WebSocket updates.

## Quick Start

```bash
# Run the Control Plane
cargo run --bin orbit-server

# With custom configuration
export ORBIT_SERVER_PORT=9000
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)
cargo run --bin orbit-server
```

**Access Points:**
- API: http://localhost:8080/api
- Swagger UI: http://localhost:8080/swagger-ui
- WebSocket: ws://localhost:8080/ws/:job_id

## Architecture

The Control Plane follows the Hexagonal (Ports & Adapters) pattern:

```
┌─────────────────────────────────────┐
│       Control Plane (Axum)          │
│                                     │
│  ┌──────────┐      ┌────────────┐  │
│  │   API    │◄────►│ State Mgmt │  │
│  │ Handlers │      │  (AppState)│  │
│  └──────────┘      └────────────┘  │
│       ▲                   ▲         │
│       │                   │         │
│  ┌────┴────┐         ┌────┴─────┐  │
│  │ OpenAPI │         │WebSockets│  │
│  │(Swagger)│         │  Events  │  │
│  └─────────┘         └──────────┘  │
└─────────────────────────────────────┘
           │
           ▼
    ┌──────────────┐
    │   Magnetar   │  (Job State)
    │   Database   │
    └──────────────┘
```

## API Endpoints

### Authentication
- `POST /api/auth/login` - Authenticate and receive JWT
- `POST /api/auth/logout` - Invalidate session
- `GET /api/auth/me` - Get current user info

### Jobs
- `GET /api/jobs` - List all jobs
- `POST /api/jobs` - Create new job
- `GET /api/jobs/:id` - Get job details
- `DELETE /api/jobs/:id` - Delete job
- `POST /api/jobs/:id/run` - Start job execution
- `POST /api/jobs/:id/cancel` - Cancel running job

### Backends
- `GET /api/backends` - List configured backends
- `GET /api/backends/:id` - Get backend details

### Health
- `GET /api/health` - Health check endpoint

### Documentation
- `GET /swagger-ui` - Interactive API documentation

## Configuration

**Environment Variables:**

| Variable | Default | Description |
|----------|---------|-------------|
| `ORBIT_SERVER_HOST` | 127.0.0.1 | Bind address |
| `ORBIT_SERVER_PORT` | 8080 | API server port |
| `ORBIT_JWT_SECRET` | ⚠️ Required | JWT signing secret (min 32 chars) |
| `ORBIT_MAGNETAR_DB` | magnetar.db | Job database path |
| `ORBIT_USER_DB` | orbit-server-users.db | Auth database path |

**Security:**
```bash
# Generate secure JWT secret
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)

# For production
export ORBIT_SERVER_HOST=0.0.0.0
export RUST_LOG=info
```

## Development

```bash
# Run with auto-reload
cargo watch -x 'run --bin orbit-server'

# Run tests
cargo test

# Generate API documentation
cargo doc --open -p orbit-server

# Check code
cargo clippy --all-targets --all-features
cargo fmt --all -- --check
```

## Project Structure

```
src/
├── main.rs          # Entry point
├── lib.rs           # Public library API
├── server.rs        # Axum server setup
├── api/             # API handlers
│   ├── auth.rs      # Authentication endpoints
│   ├── backends.rs  # Backend management
│   └── jobs.rs      # Job CRUD operations
├── auth/            # Authentication logic
│   ├── middleware.rs
│   └── models.rs
├── state.rs         # Application state
├── error.rs         # Error types
├── utils/           # Utilities
└── ws.rs            # WebSocket handlers
```

## Migration from v1.0 (Nebula)

**Key Changes:**
- Package renamed: `orbit-web` → `orbit-server`
- Removed Leptos SSR framework
- Removed WASM client-side code
- Added OpenAPI/Swagger documentation
- Separated UI into standalone React dashboard

**Breaking Changes:**
- No longer serves HTML/UI
- API-only server (headless)
- Dashboard must be hosted separately

See [CHANGELOG.md](../../CHANGELOG.md) for full migration details.

## Dashboard

The Control Plane is designed to work with the separate **Orbit Dashboard** (located in `/dashboard`):

```bash
# Run both services concurrently
../../start-orbit-v2.sh  # Unix/Linux/macOS
../../start-orbit-v2.bat # Windows
```

The dashboard is a React SPA that consumes this API.

## Security Considerations

- **Always set `ORBIT_JWT_SECRET`** in production
- **Change default admin password** immediately
- **Use HTTPS** via reverse proxy (nginx, Caddy)
- **Configure CORS** for your dashboard domain
- **Enable request logging** with `RUST_LOG=info`
- **Restrict network access** with firewall rules

## API Documentation

Full API documentation is available via Swagger UI when the server is running:

```
http://localhost:8080/swagger-ui
```

Alternatively, generate Rust documentation:

```bash
cargo doc --open -p orbit-server
```

## Support

- Main README: [../../README.md](../../README.md)
- Changelog: [../../CHANGELOG.md](../../CHANGELOG.md)
- Issues: https://github.com/saworbit/orbit/issues

## License

Apache 2.0 - See [LICENSE](../../LICENSE) for details.
