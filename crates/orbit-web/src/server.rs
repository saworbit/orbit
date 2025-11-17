//! Server bootstrap utilities for running Orbit Web from the main binary or library users.

use crate::{App, ProgressRegistry, WebConfig};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    leptos_options: LeptosOptions,
    progress_registry: ProgressRegistry,
}

impl axum::extract::FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos_options.clone()
    }
}

impl axum::extract::FromRef<AppState> for ProgressRegistry {
    fn from_ref(state: &AppState) -> Self {
        state.progress_registry.clone()
    }
}

/// Starts the Orbit Web server on the provided address.
///
/// This is shared by the standalone `orbit-web` binary and the main `orbit` CLI
/// when compiled with the `gui` feature.
pub async fn start_server(addr: SocketAddr) -> anyhow::Result<()> {
    init_tracing();

    let config = WebConfig::default();
    let db_path = config.magnetar_db_path.clone();

    // Ensure Leptos has a default output name when launched outside cargo-leptos.
    let leptos_output =
        std::env::var("LEPTOS_OUTPUT_NAME").unwrap_or_else(|_| "orbit-web".to_string());
    std::env::set_var("LEPTOS_OUTPUT_NAME", &leptos_output);

    // Load Leptos configuration and override the bind address
    let conf = get_configuration(Some("Cargo.toml")).await?;
    let mut leptos_options = conf.leptos_options;
    leptos_options.site_addr = addr;

    // Shared app state for routes
    let progress_registry = ProgressRegistry::new();
    let routes = generate_route_list(App);
    let app_state = AppState {
        leptos_options: leptos_options.clone(),
        progress_registry,
    };

    let app = build_router(app_state, routes);

    info!("Listening on http://{}", leptos_options.site_addr);
    info!("Magnetar DB: {}", db_path);

    let listener = TcpListener::bind(&leptos_options.site_addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

fn build_router(app_state: AppState, routes: Vec<leptos_router::RouteListing>) -> Router {
    Router::new()
        // WebSocket endpoint for progress updates
        .route("/ws/progress/:job_id", get(ws_progress_handler))
        // Health check endpoint
        .route("/api/health", get(health_check))
        // Serve static files (if any)
        .nest_service("/pkg", ServeDir::new("./pkg"))
        // Leptos routes
        .leptos_routes(&app_state, routes, App)
        .with_state(app_state)
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "orbit-web",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// WebSocket handler for progress updates
async fn ws_progress_handler(
    ws: WebSocketUpgrade,
    Path(job_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    info!("WebSocket connection requested for job: {}", job_id);
    ws.on_upgrade(move |socket| handle_progress_socket(socket, job_id, state))
}

/// Handle WebSocket connection for a specific job
async fn handle_progress_socket(mut socket: WebSocket, job_id: String, state: AppState) {
    info!("WebSocket connected for job: {}", job_id);

    // Subscribe to progress updates
    let mut rx = match state.progress_registry.subscribe(&job_id).await {
        Some(rx) => rx,
        None => {
            warn!("No progress channel found for job: {}", job_id);
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({
                        "error": "Job not found or not active"
                    })
                    .to_string(),
                ))
                .await;
            return;
        }
    };

    // Send progress updates to the client
    while let Ok(update) = rx.recv().await {
        let msg = match serde_json::to_string(&update) {
            Ok(json) => Message::Text(json),
            Err(e) => {
                warn!("Failed to serialize progress update: {}", e);
                continue;
            }
        };

        if socket.send(msg).await.is_err() {
            info!("WebSocket disconnected for job: {}", job_id);
            break;
        }
    }

    info!("WebSocket handler finished for job: {}", job_id);
}

fn init_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,orbit_web=debug".into()),
        )
        .finish();

    // Ignore errors if a global subscriber was already set by the caller.
    let _ = tracing::subscriber::set_global_default(subscriber);
}
