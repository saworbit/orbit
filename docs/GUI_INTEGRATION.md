# GUI Integration Guide

## Overview
- The Orbit web UI lives in the `orbit-web` crate and exposes `orbit_web::start_server` for reuse.
- The main `orbit` binary runs the GUI via the new `serve` subcommand when the `gui` Cargo feature is enabled (default).
- Feature gating keeps the CLI lightweight for users who prefer a minimal build.

## Running the GUI from the CLI
- Build with defaults: `cargo build --release` (includes the GUI).
- Start the server: `./target/release/orbit serve --addr 127.0.0.1:8080`.
- Open `http://127.0.0.1:8080` in your browser.
- Disable the GUI for slim builds: `cargo build --release --no-default-features --features zero-copy`.

## Development Workflow
- Hot reload: `cd crates/orbit-web && cargo leptos watch` (frontend + backend).
- Production assets: `cd crates/orbit-web && cargo leptos build --release` for optimized WASM output.
- The `orbit-web` crate remains runnable standalone; the binary delegates to the same `start_server` function.

## Feature Flags
- `gui` (enabled by default) pulls in `orbit-web` with the `ssr` feature and the Tokio runtime.
- Other features remain unaffected; you can combine `gui` with protocol features like `s3-native` as needed.

## Customization Points
- Embed the server in other binaries by calling `orbit_web::start_server(SocketAddr)` with a custom bind address.
- Extend routes inside `crates/orbit-web/src/server.rs` to add APIs or telemetry endpoints.
- Swap the bind address with `orbit serve --addr 0.0.0.0:3000` for LAN access.

## Common questions
- **Do I need cargo-leptos to run the GUI?** No. The main `orbit serve` path works without it; cargo-leptos is only needed for hot-reload/front-end development.
- **Where are the server assets built?** The default `cargo build` will place assets under `crates/orbit-web/target`; `cargo leptos build --release` creates optimized WASM for production.
- **How do I disable it?** Use `--no-default-features --features zero-copy` when building the `orbit` binary.

## Troubleshooting
- **Port already in use:** pass a different `--addr` value.
- **Assets missing:** rebuild the `orbit-web` assets with `cargo leptos build --release`.
- **GUI disabled:** ensure the build includes the `gui` feature (defaults to on).
