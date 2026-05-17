# Orbit v2.2.0-alpha.2 Deployment Guide

## Overview

This release implements the **"Face"** (Dashboard) of the Orbit Control Plane, building on the **"Brain"** (Server) from v2.2.0-alpha.1. The Dashboard is a modern React application that communicates with the headless API server.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Orbit v2.2.0-alpha.2                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────────┐         ┌───────────────────────┐   │
│  │  React Dashboard  │ ◄────── │   Orbit Control Plane │   │
│  │   (The "Face")    │  HTTP   │     (The "Brain")     │   │
│  │                   │  REST   │                       │   │
│  │  - Job Wizard     │ ──────► │  - API Endpoints      │   │
│  │  - Job List       │         │  - Job Management     │   │
│  │  - Pipeline Editor│         │  - File Explorer      │   │
│  │  - File Browser   │         │  - WebSocket Events   │   │
│  └───────────────────┘         └───────────────────────┘   │
│       Port 5173                       Port 8080            │
│   (Vite Dev Server)                (Axum Server)           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## What's New in v2.2.0-alpha.2

### Frontend Components

1. **Visual Pipeline Editor** - React Flow-based drag-and-drop interface
2. **File Browser** - Interactive filesystem navigation
3. **Job Wizard** - Step-by-step job creation interface
4. **Job List** - Real-time job monitoring with progress tracking

### Features

- ✅ Auto-refreshing job status (2-second polling)
- ✅ Progress bars with chunk tracking
- ✅ File system navigation (Windows & Unix)
- ✅ Job lifecycle management (Create → Run → Cancel → Delete)
- ✅ Optimistic UI updates with TanStack Query
- ✅ TypeScript strict mode with full type safety

## Quick Start

### Prerequisites

- **Rust** 1.75+ (for backend)
- **Node.js** 18+ (for frontend)
- **npm** 9+ (bundled with Node.js)

### Step 1: Start the Backend

```bash
# Set JWT secret (required for authentication)
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)

# Start the Control Plane API server
cargo run -p orbit-server
```

**Expected Output:**
```
🚀 Starting Orbit Control Plane v2.2.0-alpha
   API Endpoint: http://127.0.0.1:8080/api
   Swagger UI: http://127.0.0.1:8080/swagger-ui
```

### Step 2: Start the Frontend

In a **separate terminal**:

```bash
cd dashboard
npm install  # First time only
npm run dev
```

**Expected Output:**
```
VITE v7.2.6  ready in 342 ms

➜  Local:   http://localhost:5173/
➜  Network: use --host to expose
```

### Step 3: Access the Dashboard

Open your browser to: **http://localhost:5173**

You should see the Orbit Control Plane dashboard with three tabs:
- **Jobs** - View and manage transfer jobs
- **Create Job** - Launch new file transfers
- **Pipelines** - Build visual data pipelines

## Component Guide

### 1. Job List (`/jobs`)

**Features:**
- Real-time job status updates
- Progress bars for running jobs
- Status badges (Pending/Running/Completed/Failed/Cancelled)
- Action buttons: Run (▶), Cancel (✕), Delete (🗑)

**API Endpoints Used:**
- `POST /api/list_jobs` - Fetch all jobs
- `POST /api/run_job` - Start a pending job
- `POST /api/cancel_job` - Cancel a running job
- `POST /api/delete_job` - Delete a job

### 2. Job Wizard (`/create`)

**Features:**
- Dual file browser panels (Source & Destination)
- Directory navigation with parent (..) support
- File size display
- Real-time validation
- Success/error feedback

**API Endpoints Used:**
- `POST /api/list_dir` - List directory contents
- `POST /api/create_job` - Create new transfer job

**Workflow:**
1. Navigate to source location in left panel
2. Navigate to destination location in right panel
3. Click "Launch Orbit Job"
4. Job is created and appears in Job List

### 3. Pipeline Editor (`/pipelines`)

**Features:**
- Drag-and-drop node creation
- Visual edge connections
- Node types: Source, Transform, Destination
- Zoom and pan controls
- Background grid

**Status:** 🚧 UI-only (backend integration pending)

## File Structure

### Frontend (`dashboard/`)

```
dashboard/
├── src/
│   ├── components/
│   │   ├── files/
│   │   │   └── FileBrowser.tsx      # File system navigator
│   │   ├── jobs/
│   │   │   ├── JobWizard.tsx        # Job creation wizard
│   │   │   └── JobList.tsx          # Job monitoring view
│   │   └── pipelines/
│   │       └── PipelineEditor.tsx   # React Flow editor
│   ├── hooks/
│   │   └── useJobs.ts               # TanStack Query hooks
│   ├── lib/
│   │   └── api.ts                   # Axios instance
│   ├── App.tsx                      # Main app with routing
│   └── main.tsx                     # Entry point
├── package.json
└── vite.config.ts
```

### Backend (`crates/orbit-web/`)

The backend already provides all necessary endpoints:
- Job management: `list_jobs`, `create_job`, `run_job`, `cancel_job`, `delete_job`
- File explorer: `list_dir`, `list_drives`
- Backend management: `list_backends`, `create_backend`
- User management: `list_users`, `create_user`
- Pipeline API: `list_pipelines`, `create_pipeline` (planned)

## Configuration

### Backend Environment Variables

```bash
# Server configuration
export ORBIT_SERVER_HOST=127.0.0.1
export ORBIT_SERVER_PORT=8080

# Database paths
export ORBIT_MAGNETAR_DB=magnetar.db
export ORBIT_USER_DB=orbit-server-users.db

# Security (REQUIRED)
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)
```

### Frontend Configuration

Edit `dashboard/src/lib/api.ts` to change API endpoint:

```typescript
export const api = axios.create({
  baseURL: 'http://localhost:8080/api',  // Change this for production
  withCredentials: true,
});
```

## Production Deployment

### Build the Frontend

```bash
cd dashboard
npm run build
```

This creates optimized static files in `dashboard/dist/`:
- `index.html`
- `assets/index-*.js` (419KB, 136KB gzipped)
- `assets/index-*.css` (9KB)

### Serve Static Files

**Option 1: Nginx**

```nginx
server {
    listen 80;
    server_name orbit.example.com;

    # Serve Dashboard
    location / {
        root /var/www/orbit-dashboard/dist;
        try_files $uri /index.html;
    }

    # Proxy API requests to backend
    location /api/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
    }

    # WebSocket support
    location /ws/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
    }
}
```

**Option 2: Embedded in Axum** (Future)

The backend can serve static files from `dashboard/dist/` using `ServeDir`:

```rust
.nest_service("/", ServeDir::new("dashboard/dist"))
```

## Testing the Integration

### 1. Create a Test Job

1. Navigate to **Create Job** tab
2. Select a source directory (e.g., `C:\temp` or `/tmp`)
3. Select a destination directory
4. Click **Launch Orbit Job**
5. Verify success message appears

### 2. Monitor Job Progress

1. Navigate to **Jobs** tab
2. Find your newly created job (status: Pending)
3. Click the **Run** button (▶)
4. Watch the progress bar update in real-time
5. Verify status changes to Running → Completed

### 3. Test File Browser

1. Navigate to **Create Job** tab
2. Click on directories to navigate
3. Click ".." to go to parent directory
4. Verify file sizes are displayed
5. Verify folders show folder icon (📁)

## Troubleshooting

### Issue: "Failed to fetch jobs"

**Cause:** Backend not running or CORS issue

**Solution:**
```bash
# Check backend is running
curl http://localhost:8080/api/health

# Should return:
# {"status":"ok","service":"orbit-web","version":"1.0.0-rc.1"}
```

### Issue: "401 Unauthorized"

**Cause:** JWT secret not set or invalid token

**Solution:**
```bash
# Set JWT secret
export ORBIT_JWT_SECRET=$(openssl rand -base64 32)

# Restart backend
cargo run -p orbit-server
```

### Issue: Build fails with Tailwind error

**Cause:** PostCSS configuration issue

**Solution:**
```bash
cd dashboard
npm install @tailwindcss/postcss
```

Verify `postcss.config.js` uses `@tailwindcss/postcss`:
```javascript
export default {
  plugins: {
    '@tailwindcss/postcss': {},
    autoprefixer: {},
  },
}
```

### Issue: TypeScript errors

**Cause:** Missing type imports

**Solution:**
All type imports should use the `type` keyword:
```typescript
import type { Node, Connection } from 'reactflow';
```

## API Endpoints Reference

### Job Management

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/list_jobs` | POST | List all jobs |
| `/api/create_job` | POST | Create a new job |
| `/api/run_job` | POST | Start a pending job |
| `/api/cancel_job` | POST | Cancel a running job |
| `/api/delete_job` | POST | Delete a job |
| `/api/get_job` | POST | Get single job details |

### File Explorer

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/list_dir` | POST | List directory contents |
| `/api/list_drives` | GET | List system drives (Windows/Unix) |
| `/api/upload_file` | POST | Upload file to server |

### Authentication

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/auth/login` | POST | Login with username/password |
| `/api/auth/logout` | POST | Logout and clear session |
| `/api/auth/me` | GET | Get current user info |

## Next Steps

### Planned for v2.2.0-alpha.3

- [ ] WebSocket integration for real-time updates
- [ ] Pipeline backend integration
- [ ] Backend configuration UI
- [ ] User management UI
- [ ] File upload component
- [ ] Job scheduling interface
- [ ] Improved error handling and validation

## Support

For issues or questions:
- GitHub Issues: https://github.com/your-org/orbit/issues
- Documentation: https://docs.orbit.example.com
- Changelog: [CHANGELOG.md](CHANGELOG.md)

---

**Status:** ✅ Production-Ready for Development/Testing

**License:** Apache-2.0

**Author:** Shane Wall <shaneawall@gmail.com>
