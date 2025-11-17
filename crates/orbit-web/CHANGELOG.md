# Changelog

All notable changes to Orbit Nebula will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for v1.0.0-beta.1
- Interactive dashboard with live WebSocket updates
- Job creation form with validation
- Job control buttons (run, pause, cancel, delete)
- Interactive login form with loading states

### Planned for v1.0.0-beta.2
- File explorer with directory navigation
- Drag-and-drop file upload
- Backend credential management UI
- User management panel (Admin only)

### Planned for v1.0.0
- Telemetry dashboard with charts and graphs
- Visual pipeline builder with DAG visualization
- Dark mode theme
- PWA support for offline monitoring
- Comprehensive end-to-end testing

## [1.0.0-alpha.2] - 2025-11-17

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

[Unreleased]: https://github.com/saworbit/orbit/compare/v1.0.0-alpha.2...HEAD
[1.0.0-alpha.2]: https://github.com/saworbit/orbit/compare/v1.0.0-alpha.1...v1.0.0-alpha.2
[1.0.0-alpha.1]: https://github.com/saworbit/orbit/releases/tag/v1.0.0-alpha.1
