# Changelog

All notable changes to Orbit Nebula will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Pipeline Editor** - Complete visual workflow designer with React Flow
  - Interactive drag-and-drop canvas for building transfer workflows
  - 7 node types: Source, Destination, Transform, Filter, Merge, Split, Conditional
  - Color-coded node palette with drag-to-canvas support
  - Visual connection builder with animated edges
  - Real-time position persistence and auto-save
  - Validation logic (requires Source + Destination nodes)
  - MiniMap and grid background for navigation
  - Live status panel showing node/edge counts
  - Bulk save via `POST /api/sync_pipeline` endpoint

- **Pipeline Management UI**
  - Pipeline list view with card grid layout
  - Create pipeline form with name and description
  - Delete pipeline with confirmation dialog
  - Status badges (draft, ready, running, completed, failed)
  - Click-to-edit navigation with back button
  - Empty state onboarding message
  - Information panel explaining pipeline workflows

- **Backend API - Pipeline Bulk Sync**
  - New endpoint: `POST /api/sync_pipeline`
  - Accepts: `{ pipeline_id, nodes_json, edges_json }`
  - Validates JSON structure before database update
  - Replaces chatty add_node/remove_edge pattern with single snapshot
  - Updates both in-memory cache and SQLite database
  - Logs sync statistics (node count, edge count)

- **Frontend Hooks - usePipelines.ts**
  - `usePipelines()` - List all pipelines (5s polling)
  - `usePipeline(id)` - Get single pipeline with details (2s polling)
  - `useCreatePipeline()` - Create new empty pipeline
  - `useSavePipeline()` - Bulk save using sync_pipeline
  - `useDeletePipeline()` - Delete pipeline by ID
  - Full TypeScript support with proper types

### Fixed

- **CLI Integration** - Fixed synchronization with orbit v0.6.0 main CLI
  - Main CLI now correctly imports as `orbit_server` (matching package name)
  - Main CLI now uses `ServerConfig` (renamed from `WebConfig`)
  - Added `gui` feature alias in root `Cargo.toml` for backward compatibility
  - `start_server()` now properly receives `reactor_notify` parameter from CLI

- **User Management** - Fixed delete endpoint mismatch
  - Frontend now uses `POST /api/delete_user` with `{ user_id: number }`
  - Previously used `DELETE /api/admin/users/:id` (unsupported)
  - Aligns with backend RPC-style pattern

### Technical Details

- **Integration Status**:
  - ✅ JobDetail - Already connected to live API
  - ✅ UserList - Delete endpoint fixed
  - ✅ PipelineEditor - Fully implemented with React Flow v12.10.0
  - ✅ FileBrowser - Already working

- **Data Mapping**: Backend nodes/edges seamlessly convert to React Flow format
- **Verification**: TypeScript checks pass, Prettier formatting applied
- **Dependencies**: React Flow v12.10.0, all existing packages

### Planned for v1.0.0
- Telemetry dashboard with charts and graphs
- PWA support for offline monitoring
- Comprehensive end-to-end testing

## [1.0.0-rc.1] - 2025-11-23

### Added
- **Visual Pipeline Builder** - DAG-based workflow editor
  - Create, edit, and delete transfer pipelines
  - Canvas-based visual editor with drag-and-drop nodes
  - 7 node types: Source, Destination, Transform, Filter, Merge, Split, Conditional
  - Node palette with tooltips explaining each node type
  - Quick help guide showing how to build a pipeline
  - Click and drag nodes to reposition
  - Double-click nodes to configure properties
  - Visual edge connections between nodes (click output port, drag to input port)
  - Click edges to delete connections
  - Curved bezier path edges with arrow markers
  - Node type-specific configuration modals with descriptions
  - Real-time node position persistence
  - Pipeline status tracking (draft, ready, running, completed, failed, paused)

- **File Browser Integration for Nodes**
  - Source/Destination nodes have "Browse..." button
  - Opens File Explorer to select folder
  - "Use This Folder" banner for path selection
  - Optional file pattern filter for Source nodes

- **Pipeline Validation**
  - "Validate" button checks pipeline configuration
  - Rules: must have Source node, must have Destination node
  - Source nodes must have path configured
  - Destination nodes must have path configured
  - Source nodes cannot have incoming connections
  - Destination nodes cannot have outgoing connections
  - Warns about unconnected (orphan) nodes
  - Clear error/warning messages with fixes

- **Enhanced Node Configuration**
  - Source: path + optional file pattern
  - Destination: path with browse
  - Filter: glob pattern with examples
  - Transform: compression + encryption toggles
  - Merge: strategy (all, newest, largest)
  - Split: mode (duplicate to all, round robin)
  - Conditional: type (extension, size, name) + value

- **Pipeline Data Model**
  - Pipeline struct with nodes, edges, and metadata
  - PipelineNode with type, position, and configuration
  - PipelineEdge for DAG connections between nodes
  - NodeConfig for type-specific settings (path, pattern, compression, etc.)
  - Database persistence with JSON serialization for nodes/edges

- **New REST API Endpoints**
  - `POST /api/list_pipelines` - List all pipelines
  - `POST /api/get_pipeline` - Get pipeline with full node/edge details
  - `POST /api/create_pipeline` - Create new pipeline
  - `POST /api/update_pipeline` - Update pipeline metadata/status
  - `POST /api/delete_pipeline` - Delete pipeline
  - `POST /api/add_node` - Add node to pipeline
  - `POST /api/update_node` - Update node position/config
  - `POST /api/remove_node` - Remove node (and connected edges)
  - `POST /api/add_edge` - Add edge between nodes
  - `POST /api/remove_edge` - Remove edge

### Fixed
- **Pipeline drag-and-drop fully working**:
  - Canvas SVG and nodes-layer now use `pointer-events: none` for proper drop handling
  - Switched to `addEventListener` for more reliable event binding across browsers
  - Changed data transfer type to `text/plain` for better compatibility
  - Added `effectAllowed`/`dropEffect` for proper drag operation
  - Added visual feedback with dashed border when dragging over canvas
- WebSocket routes: changed from `/ws/*path` to explicit `/ws/events` and `/ws/events/:job_id` routes
- WebSocket handler: fixed optional path parameter extraction

### Technical Details
- SQLite persistence for pipeline DAG structures
- In-memory pipeline cache with database sync
- SVG-based edge rendering with bezier curves
- Drag-and-drop from palette to canvas
- Mouse event handling for node dragging and edge creation
- Cycle detection (prevents self-loops)
- Automatic edge cleanup when nodes are deleted

### Clarification
Pipeline "nodes" (Source, Destination, Transfer, etc.) are visual building blocks in the workflow editor - they are NOT separate Orbit server instances. The orbit-web crate is the dashboard/control panel for managing Orbit transfers.

## [1.0.0-beta.2] - 2025-11-22

### Added
- **File Explorer** - Full directory navigation and browsing
  - Drive/root selector for Windows and Unix systems
  - Directory listing with file icons, sizes, and dates
  - Navigate up button and direct path input
  - Click directories to navigate, path breadcrumbs
  - Copy path to clipboard functionality

- **Drag-and-Drop File Upload** - Upload files to any directory
  - Visual drop zone with drag-over effects
  - Multi-file upload support
  - Upload progress indicators per file
  - Success/failure status for each upload
  - Automatic directory refresh after upload

- **User Management Panel** (Admin only)
  - User list with username, role, and creation date
  - Role badges (Admin=red, Operator=yellow, Viewer=blue)
  - **Create User** - Add new users with role selection
  - **Edit User** - Change password and role
  - **Delete User** - With confirmation dialog
  - Protection against deleting the last admin user
  - Users nav only visible to admin users

- **New REST API Endpoints**
  - `POST /api/list_dir` - List directory contents
  - `GET /api/list_drives` - Get available drives/roots
  - `POST /api/upload_file` - Upload file (multipart)
  - `POST /api/list_users` - List all users (Admin)
  - `POST /api/create_user` - Create new user (Admin)
  - `POST /api/update_user` - Update user password/role (Admin)
  - `POST /api/delete_user` - Delete user (Admin)

### Technical Details
- Secure file system access with proper error handling
- Argon2 password hashing for new users
- RBAC enforcement for user management
- Multipart form handling for file uploads
- Cross-platform drive detection (Windows/Unix)

## [1.0.0-beta.1] - 2025-11-22

### Added
- **Job Creation Form** - Create transfer jobs directly from the UI
  - Modal dialog with source/destination path inputs
  - Parallel workers configuration (1-32)
  - Toggle switches for compression and verification options
  - Form validation with error feedback
  - Immediate job list refresh after creation

- **Job Control Buttons** - Full job lifecycle management
  - **Start** button for pending jobs (transitions to running)
  - **Cancel** button for running/pending jobs
  - **Delete** button with confirmation dialog
  - **Details** button to view job information
  - Context-sensitive button display based on job status

- **Job Detail View** - Comprehensive job information modal
  - Job ID, status badge, and progress visualization
  - Full source and destination paths
  - Chunk statistics (completed/total/failed)
  - Created and updated timestamps
  - Action buttons based on current status

- **Backend Management UI** - Configure S3, SMB, and Local storage
  - New "Backends" section in sidebar navigation
  - Backend list with type badges and connection details
  - "Add Backend" modal with dynamic form fields:
    - **S3**: Bucket, region, access key, secret key
    - **SMB**: Host, share, username, password (optional)
    - **Local**: Root path
  - Delete backend with confirmation

- **New REST API Endpoints**
  - `POST /api/get_job` - Get single job details
  - `POST /api/run_job` - Start a pending job
  - `POST /api/cancel_job` - Cancel running/pending job
  - `POST /api/delete_job` - Delete a job
  - `POST /api/list_backends` - List configured backends
  - `POST /api/create_backend` - Add new backend
  - `POST /api/delete_backend` - Remove backend

### Improved
- **Jobs Table** - Added Actions column with control buttons
- **Progress Display** - Now shows percentage alongside progress bar
- **Path Display** - Truncated long paths with full path in tooltip
- **Status Badges** - Added "cancelled" status styling

### Technical Details
- Modal system with overlay and close button
- Toggle switch components for boolean options
- Dynamic form visibility based on backend type selection
- Proper error handling with user-friendly messages
- Confirmation dialogs for destructive actions

## [1.0.0-alpha.3] - 2025-11-22

### Added
- **Modern Interactive Dashboard** - Complete UI redesign with professional dark theme
  - Sidebar navigation with Overview, Jobs, API Explorer, and WebSocket sections
  - Stats grid with colored indicator cards (Server Status, Active Transfers, Completed Today, WebSocket Status)
  - Responsive design with mobile support
  - Gradient accents and modern styling

- **Interactive Login Page** - Professional authentication UI
  - Gradient styling with branded logo
  - Loading spinner during authentication
  - Error message display with proper styling
  - Credential hints for demo access

- **Jobs Dashboard** - Real-time job monitoring
  - Job table with ID, source, destination, status badges, and progress bars
  - Status badges: completed (green), running (blue), pending (yellow), failed (red)
  - Auto-refresh capability with manual refresh button
  - Empty state handling with helpful messaging

- **API Explorer** - Built-in API testing interface
  - HTTP method selector (GET, POST, DELETE)
  - Endpoint input with request body textarea
  - Quick action buttons for common endpoints (Health, Me, Jobs, Backends)
  - Syntax-highlighted JSON response display
  - Error/success status indicators

- **WebSocket Monitor** - Real-time event visualization
  - Connect/disconnect controls
  - Live event stream with timestamps
  - Event log with auto-scroll
  - Clear log functionality

- **Demo Job Scripts** - Tools for populating test data
  - `create-demo-jobs.bat` - Direct SQLite insertion (6 demo jobs with various statuses)
  - `create-demo-jobs-api.bat` - API-based job creation via curl
  - Sample jobs: completed backups, running transfers, pending syncs, failed jobs

### Fixed
- **Login Screen Visibility** - Proper page hiding with CSS `.hidden` class
  - Login page now completely hides after successful authentication
  - Dashboard page properly shows after login
  - No more scrolling required to see dashboard

- **Login Response Handling** - Fixed nested JSON response parsing
  - API returns `{user: {...}, message: "..."}`
  - JavaScript now correctly extracts user from `data.user || data`

- **Database Schema** - Added missing columns to jobs table migration
  - Added `progress` (REAL) - job completion percentage
  - Added `total_chunks` (INTEGER) - total chunks in transfer
  - Added `completed_chunks` (INTEGER) - successfully transferred chunks
  - Added `failed_chunks` (INTEGER) - chunks that failed to transfer
  - Added `cancelled` status to CHECK constraint

### Improved
- **UI/UX** - Professional appearance matching modern dashboard standards
  - Dark theme with `#0a0e17` background and `#111827` cards
  - Consistent color palette with CSS variables
  - Smooth transitions and hover effects
  - Proper typography hierarchy

- **Code Quality** - Improved maintainability
  - Proper separation of concerns in JavaScript
  - Consistent state management for user session
  - Clear error handling patterns

### Technical Details
- Single-page application with vanilla JavaScript
- CSS Grid and Flexbox layouts
- CSS custom properties for theming
- Responsive breakpoints at 1024px and 768px

## [1.0.0-alpha.2] - 2025-11-17

### Added
- **Automated startup scripts** - Cross-platform scripts for easy server launch
  - `start-nebula.sh` for Unix/Linux/macOS with prerequisite checking, auto-installation, JWT generation, and smart building
  - `start-nebula.bat` for Windows with equivalent functionality
  - Automatic wasm32-unknown-unknown target installation if missing
  - Secure JWT secret generation if not provided
  - Data directory creation and environment variable setup
  - Comprehensive startup information display with API endpoints, credentials, and security warnings

### Fixed
- **Leptos ServerFnError type annotations** - Replaced `.ok_or_else()` and `.map_err()` with explicit `match` statements to resolve type inference issues in server functions
  - Fixed 6 functions in `src/api/jobs.rs` (list_jobs, get_job_stats, create_job, delete_job, run_job, cancel_job)
  - Fixed 2 functions in `src/api/backends.rs` (list_backends, get_backend)
- **Unused imports** - Cleaned up all unused imports across 6 files:
  - `src/api/auth.rs` - Removed `WebResult`, `IntoResponse`, `Deserialize`
  - `src/api/backends.rs` - Removed `WebError`, `WebResult`
  - `src/api/jobs.rs` - Removed `Claims`, `WebError`, `WebResult`
  - `src/components/dashboard.rs` - Removed `leptos_router::*`
  - `src/ws.rs` - Removed `Role`
  - `src/auth/middleware.rs` - Removed unnecessary `mut` keyword

### Improved
- **Compilation** - Achieved 0 errors and 0 warnings
- **Build time** - Successfully builds in ~37 seconds
- **Documentation** - Updated NEBULA_MVP_SUMMARY.md with alpha.2 status

### Technical Details
- All API endpoints now compile cleanly with proper error handling
- Match statements provide clearer type information for Rust compiler
- Cleaner codebase with no unused dependencies

## [1.0.0-alpha.1] - 2025-11-17

### Added
- **Complete ground-up rewrite** of orbit-web from basic polling dashboard to enterprise-grade control center
- **Authentication & Security (100%)**
  - JWT token-based authentication with 24-hour expiration
  - Argon2 password hashing (OWASP recommended)
  - RBAC with 3 roles: Admin, Operator, Viewer
  - httpOnly secure cookies for token storage
  - Default admin account (`admin` / `orbit2025`)
  - SQLite user database with automatic schema migration
  - Axum middleware for route protection

- **Real-Time Event System (100%)**
  - WebSocket handler with JWT validation
  - Broadcast channels for event distribution
  - 6 event types: JobUpdated, TransferSpeed, JobCompleted, JobFailed, AnomalyDetected, ChunkCompleted
  - Event filtering by role and job ID
  - Sub-500ms latency

- **RESTful API Layer (100%)**
  - Authentication endpoints: login, logout, me
  - Job CRUD operations (Leptos server functions)
  - Backend configuration endpoints
  - Health check endpoint

- **Database Layer (100%)**
  - Separate SQLite user database (isolated from Magnetar)
  - Runtime SQL queries for flexibility
  - Automatic schema initialization
  - Default admin user creation

- **State Management (100%)**
  - AppState with Magnetar pool
  - User database pool (SQLx 0.8)
  - Broadcast channels for events
  - Backend configuration storage (S3, SMB)
  - Thread-safe sharing via Arc/RwLock

- **Server Infrastructure (100%)**
  - Axum 0.7 HTTP server
  - Static file serving
  - CORS support
  - Request tracing and structured logging
  - Environment variable configuration

- **UI Components (60%)**
  - Leptos Router setup
  - Simplified login page (API documentation focus)
  - Dashboard structure (interactive features pending)
  - Landing page with API reference

- **Documentation (100%)**
  - Comprehensive README with quick start, API reference, security guide
  - Detailed MVP summary with implementation details
  - Migration guide from v0.5.0

### Technical Stack
- Leptos 0.6 - Full-stack Rust framework
- Axum 0.7 - High-performance async HTTP server
- SQLx 0.8 - Async SQL toolkit
- Argon2 - Password hashing
- jsonwebtoken 9.0 - JWT implementation
- Tokio - Async runtime

### Code Statistics
- ~2,000 lines of production Rust code
- 20+ files created
- 100% backend implementation
- 60% frontend implementation

[Unreleased]: https://github.com/saworbit/orbit/compare/v1.0.0-rc.1...HEAD
[1.0.0-rc.1]: https://github.com/saworbit/orbit/compare/v1.0.0-beta.2...v1.0.0-rc.1
[1.0.0-beta.2]: https://github.com/saworbit/orbit/compare/v1.0.0-beta.1...v1.0.0-beta.2
[1.0.0-beta.1]: https://github.com/saworbit/orbit/compare/v1.0.0-alpha.3...v1.0.0-beta.1
[1.0.0-alpha.3]: https://github.com/saworbit/orbit/compare/v1.0.0-alpha.2...v1.0.0-alpha.3
[1.0.0-alpha.2]: https://github.com/saworbit/orbit/compare/v1.0.0-alpha.1...v1.0.0-alpha.2
[1.0.0-alpha.1]: https://github.com/saworbit/orbit/releases/tag/v1.0.0-alpha.1
