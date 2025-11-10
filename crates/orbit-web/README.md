# Orbit Web - Web GUI for Orbit File Transfer

A modern, reactive web interface for orchestrating Orbit file transfers with real-time progress tracking and job management.

## Features

- **Real-time Dashboard**: Live monitoring of active and completed jobs
- **Job Creation**: Intuitive form interface for creating new transfer jobs
- **Progress Tracking**: Real-time progress bars with detailed statistics
- **WebSocket Updates**: Low-latency progress updates via WebSocket connections
- **Persistent State**: All job state stored in Magnetar database for crash recovery
- **Responsive Design**: Works on desktop and mobile devices

## Quick Start

### Prerequisites

- Rust 1.70+ with `wasm32-unknown-unknown` target
- Cargo Leptos CLI tool

### Installation

Install Cargo Leptos if you haven't already:

```bash
cargo install cargo-leptos
```

Install the WASM target:

```bash
rustup target add wasm32-unknown-unknown
```

### Running the Server

From the `orbit-web` directory:

```bash
cargo leptos watch
```

This will start the development server with hot-reload at http://127.0.0.1:8080

For production:

```bash
cargo leptos build --release
cargo run --release
```

## Architecture

Orbit Web is built using:

- **Leptos**: Full-stack Rust framework for reactive UI
- **Axum**: Fast, ergonomic web framework for the backend
- **Magnetar**: Persistent job state management
- **WebSockets**: Real-time progress updates
- **Tailwind CSS**: Utility-first styling

### Project Structure

```
orbit-web/
├── src/
│   ├── main.rs           # Axum server + Leptos integration
│   ├── lib.rs            # Library exports
│   ├── app.rs            # Root Leptos component
│   ├── components/       # UI components
│   │   ├── dashboard.rs  # Main dashboard
│   │   ├── job_form.rs   # Job creation form
│   │   ├── job_list.rs   # Job list with auto-refresh
│   │   └── progress_bar.rs # Progress visualization
│   ├── server_fns.rs     # Server-side functions
│   ├── progress.rs       # Progress registry
│   ├── types.rs          # Shared types
│   └── error.rs          # Error handling
├── Cargo.toml
├── Leptos.toml           # Leptos configuration
└── README.md
```

## Configuration

Set environment variables:

```bash
# Database path for Magnetar
export ORBIT_WEB_DB=orbit-web.db

# Log level
export RUST_LOG=info,orbit_web=debug
```

## API Endpoints

### HTTP Endpoints

- `GET /` - Main dashboard
- `GET /api/health` - Health check
- `POST /api/list_jobs` - List all jobs (Leptos server function)
- `POST /api/create_job` - Create a new job (Leptos server function)
- `POST /api/get_job_stats` - Get job statistics (Leptos server function)

### WebSocket Endpoints

- `WS /ws/progress/:job_id` - Real-time progress updates for a specific job

## Development

### Watch Mode

Run with hot-reload during development:

```bash
cargo leptos watch
```

### Building for Production

```bash
cargo leptos build --release
```

The compiled binary will be in `target/release/orbit-web`

### Testing

```bash
cargo test
```

## Integration with Orbit CLI

Orbit Web can be run standalone or integrated as a subcommand in the main Orbit CLI.

To integrate, add to `orbit/src/main.rs`:

```rust
use orbit_web::WebConfig;

#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    Web {
        #[arg(long, default_value = "8080")]
        port: u16,
    },
}

// In the command handler:
Commands::Web { port } => {
    let config = WebConfig {
        port,
        ..Default::default()
    };
    orbit_web::start_server(config).await?;
}
```

## Roadmap

### MVP (v0.1.0) ✅
- [x] Job listing with status
- [x] Job creation form
- [x] Real-time progress bars
- [x] WebSocket support
- [x] Basic error handling

### Post-MVP
- [ ] Manifest drag-and-drop editor
- [ ] Log tail viewer with streaming
- [ ] Job pause/resume controls
- [ ] Analytics dashboard with Parquet export
- [ ] Authentication and authorization
- [ ] TLS support
- [ ] Dark mode theme
- [ ] Mobile PWA support

## License

Apache-2.0

## Contributing

Contributions welcome! Please see the main Orbit repository for contribution guidelines.
