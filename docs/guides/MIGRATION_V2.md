# Migration Guide: v1.0 (Nebula) → v2.2.0 (Control Plane)

**Date:** December 3, 2025
**Status:** Alpha Release
**Breaking Changes:** Yes - Architecture Complete Rewrite

---

## Executive Summary

Orbit v2.2.0 introduces **"The Separation"** - a fundamental architectural shift from a monolithic Leptos SSR application to a decoupled **Control Plane (Rust API)** + **Dashboard (React SPA)** architecture.

**Why this change?**
- **Performance**: Removed WASM/SSR overhead, pure JSON APIs
- **Scalability**: Backend and frontend can scale independently
- **Developer Experience**: Modern React ecosystem with HMR
- **Deployment Flexibility**: API and UI can be versioned separately

---

## Breaking Changes

| Component | v1.0 (Nebula) | v2.2.0 (Control Plane) |
|-----------|---------------|------------------------|
| **Backend** | `orbit-web` (Leptos SSR) | `orbit-server` (Axum REST) |
| **Frontend** | Server-rendered Leptos | `orbit-dashboard` (React SPA) |
| **Build** | `cargo leptos watch` | `cargo run` + `npm run dev` |
| **Binary** | `orbit-web` | `orbit-server` |
| **Deployment** | Single binary with embedded UI | Two separate services |
| **API Docs** | None | OpenAPI/Swagger at `/swagger-ui` |
| **Features** | `gui` | `api` |

---

## Step-by-Step Migration

### 1. Update Your Codebase

```bash
# Pull latest changes
git pull origin main

# Install Node.js dependencies for dashboard
cd dashboard
npm install
cd ..
```

### 2. Update Build Commands

**Old (v1.0):**
```bash
# Development
cd crates/orbit-web
cargo leptos watch

# Production
cargo build --release --features gui
```

**New (v2.2.0):**
```bash
# Development (use helper scripts)
./launch-orbit.sh  # Unix/Linux/macOS
launch-orbit.bat   # Windows

# Or manually:
# Terminal 1: Backend
cd crates/orbit-web
cargo run --bin orbit-server

# Terminal 2: Frontend
cd dashboard
npm run dev

# Production
cargo build --release --features api
cd dashboard && npm run build
```

### 3. Update Feature Flags

**Old:**
```toml
# In your Cargo.toml dependencies
orbit-web = { path = "crates/orbit-web", features = ["ssr"] }
```

**New:**
```toml
# In your Cargo.toml dependencies
orbit-server = { path = "crates/orbit-web", features = [] }
```

**CLI Build:**
```bash
# Old
cargo build --features gui

# New
cargo build --features api
```

### 4. Update Environment Variables

**Old:**
```bash
export ORBIT_WEB_HOST=127.0.0.1
export ORBIT_WEB_PORT=8080
export ORBIT_USER_DB=orbit-web-users.db
```

**New:**
```bash
export ORBIT_SERVER_HOST=127.0.0.1
export ORBIT_SERVER_PORT=8080
export ORBIT_USER_DB=orbit-server-users.db
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)  # REQUIRED!
```

### 5. Update API Client Code

**Old (Leptos Server Functions):**
```rust
use orbit_web::api::*;

#[server(ListJobs, "/api")]
pub async fn list_jobs() -> Result<Vec<JobInfo>, ServerFnError> {
    // ...
}
```

**New (Direct HTTP Calls):**
```typescript
// From dashboard or external client
import { api } from './lib/api';

const response = await api.get('/jobs');
const jobs = response.data;
```

### 6. Update Deployment

#### Docker Deployment Example

**Old (v1.0) - Single Container:**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features gui

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/orbit-web /usr/local/bin/
EXPOSE 8080
CMD ["orbit-web"]
```

**New (v2.2.0) - Separate Containers:**

**Backend (Control Plane):**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin orbit-server

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/orbit-server /usr/local/bin/
EXPOSE 8080
CMD ["orbit-server"]
```

**Frontend (Dashboard):**
```dockerfile
FROM node:20 as builder
WORKDIR /app
COPY dashboard/package*.json ./
RUN npm install
COPY dashboard/ .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf
EXPOSE 80
```

**docker-compose.yml:**
```yaml
version: '3.8'
services:
  control-plane:
    build:
      context: .
      dockerfile: Dockerfile.backend
    ports:
      - "8080:8080"
    environment:
      - ORBIT_JWT_SECRET=${ORBIT_JWT_SECRET}
      - ORBIT_MAGNETAR_DB=/data/magnetar.db
    volumes:
      - ./data:/data

  dashboard:
    build:
      context: .
      dockerfile: Dockerfile.frontend
    ports:
      - "80:80"
    depends_on:
      - control-plane
    environment:
      - VITE_API_URL=http://control-plane:8080
```

### 7. Update Nginx Configuration (Production)

**New Configuration for Reverse Proxy:**
```nginx
# /etc/nginx/sites-available/orbit

upstream orbit_api {
    server 127.0.0.1:8080;
}

server {
    listen 80;
    server_name orbit.example.com;

    # Dashboard (Static Files)
    location / {
        root /var/www/orbit/dashboard/dist;
        try_files $uri $uri/ /index.html;
    }

    # API Proxy
    location /api {
        proxy_pass http://orbit_api;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }

    # WebSocket Proxy
    location /ws {
        proxy_pass http://orbit_api;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header Host $host;
    }

    # Swagger UI
    location /swagger-ui {
        proxy_pass http://orbit_api;
    }
}
```

---

## API Compatibility

### Endpoints (Mostly Compatible)

| Endpoint | v1.0 | v2.2.0 | Status |
|----------|------|--------|--------|
| `POST /api/auth/login` | ✅ | ✅ | ✅ Compatible |
| `GET /api/jobs` | ✅ | ✅ | ✅ Compatible |
| `POST /api/jobs` | ✅ | ✅ | ✅ Compatible |
| `GET /api/jobs/:id` | ✅ | ✅ | ✅ Compatible |
| `DELETE /api/jobs/:id` | ✅ | ✅ | ✅ Compatible |
| `POST /api/jobs/:id/run` | ✅ | ✅ | ✅ Compatible |
| `POST /api/jobs/:id/cancel` | ✅ | ✅ | ✅ Compatible |
| `GET /api/backends` | ✅ | ✅ | ✅ Compatible |
| `GET /swagger-ui` | ❌ | ✅ | ✨ New |
| `/` (HTML UI) | ✅ | ❌ | ⚠️ Removed |

### Response Format Changes

**Mostly identical**, but v2.2.0 uses stricter OpenAPI schema validation:

```json
// v1.0 - Some fields optional
{
  "id": 1,
  "status": "running"
}

// v2.2.0 - Strict schema
{
  "id": 1,
  "source": "/data/backup",
  "destination": "s3://bucket",
  "status": "running",
  "progress": 0.45,
  "total_chunks": 100,
  "completed_chunks": 45,
  "failed_chunks": 0,
  "created_at": 1701619200,
  "updated_at": 1701619300
}
```

---

## Feature Parity

| Feature | v1.0 (Nebula) | v2.2.0 (Control Plane) |
|---------|---------------|------------------------|
| Job Management | ✅ | ✅ |
| Authentication (JWT) | ✅ | ✅ |
| WebSocket Updates | ✅ | ✅ |
| Backend Configuration | ✅ | ✅ |
| Visual Pipeline Builder | ✅ | 🚧 Coming in alpha.2 |
| File Browser | ✅ | 🚧 Coming in alpha.2 |
| User Management UI | ✅ | 🚧 Coming in beta.1 |
| API Documentation | ❌ | ✅ OpenAPI/Swagger |
| Hot Module Replacement | ❌ | ✅ Vite HMR |

---

## Troubleshooting

### "orbit serve" command not found
**Solution:** v2.2.0 removes the `orbit serve` subcommand. Use the new launcher scripts or run manually.

### CORS errors in browser console
**Solution:** Ensure Control Plane is running with CORS enabled (default in dev mode):
```rust
// server.rs - CORS already configured
.layer(CorsLayer::permissive())  // Dev mode
```

### Dashboard shows "Network Error"
**Solution:** Verify Control Plane is running:
```bash
curl http://localhost:8080/api/health
# Should return: {"status":"ok"}
```

### JWT Authentication fails
**Solution:** Ensure `ORBIT_JWT_SECRET` is set:
```bash
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)
```

### Database migration errors
**Solution:** v2.2.0 uses the same Magnetar database format. No migration needed. If issues persist:
```bash
# Backup your data
cp magnetar.db magnetar.db.backup

# Restart fresh (WARNING: Loses job history)
rm magnetar.db
rm orbit-server-users.db
```

---

## Rollback Plan

If you need to rollback to v1.0:

```bash
# 1. Checkout previous version
git checkout v1.0.0-rc.1

# 2. Rebuild
cd crates/orbit-web
cargo build --release --features gui

# 3. Restore old startup
cargo leptos watch  # Dev
./target/release/orbit-web  # Production
```

---

## Support

- **Issues**: [GitHub Issues](https://github.com/saworbit/orbit/issues)
- **Discussions**: [GitHub Discussions](https://github.com/saworbit/orbit/discussions)
- **Documentation**: [README.md](README.md), [CHANGELOG.md](CHANGELOG.md)
- **API Docs**: http://localhost:8080/swagger-ui (when running)

---

## Timeline & Roadmap

- **v2.2.0-alpha.1** (Current) - Basic separation, API refactoring
- **v2.2.0-alpha.2** (Next 2-3 weeks) - Interactive dashboard features
- **v2.2.0-beta.1** (1-2 months) - Feature-complete dashboard
- **v2.2.0-rc.1** (2-3 months) - Production hardening
- **v2.2.0** (3-4 months) - Stable release

---

## FAQ

**Q: Can I still use the old Nebula UI?**
A: No, v2.2.0 completely removes the Leptos UI. Use the new React dashboard or build your own client using the OpenAPI spec.

**Q: Will my job history be preserved?**
A: Yes, Magnetar database format is unchanged. Your job history remains intact.

**Q: Do I need to learn React to contribute to the UI?**
A: Yes, for UI contributions. Backend remains pure Rust.

**Q: Can I deploy the API without the dashboard?**
A: Yes! The Control Plane is fully headless and can be used via direct API calls or custom clients.

**Q: Is the API versioned?**
A: Yes, all endpoints are `/api/v1/...` (implied in current `/api/...` routes). Future versions will use explicit versioning.

---

## Need Help?

The migration is a significant change. If you encounter issues:

1. Check the [CHANGELOG.md](CHANGELOG.md) for detailed changes
2. Review [README.md](README.md) for updated documentation
3. Open an issue on [GitHub](https://github.com/saworbit/orbit/issues)
4. Join the discussion in [Discussions](https://github.com/saworbit/orbit/discussions)

Happy migrating! 🚀
