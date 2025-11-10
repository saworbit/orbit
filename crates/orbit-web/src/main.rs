//! Orbit Web - Main server entry point

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
use orbit_web::{App, ProgressRegistry, WebConfig};
use tower_http::services::ServeDir;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    leptos_options: LeptosOptions,
    progress_registry: ProgressRegistry,
    config: WebConfig,
}

// Implement FromRef to extract LeptosOptions from AppState
impl axum::extract::FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos_options.clone()
    }
}

// Implement FromRef to extract ProgressRegistry from AppState
impl axum::extract::FromRef<AppState> for ProgressRegistry {
    fn from_ref(state: &AppState) -> Self {
        state.progress_registry.clone()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,orbit_web=debug".into()),
        )
        .init();

    info!("Starting Orbit Web server...");

    // Load configuration
    let config = WebConfig::default();

    // Get Leptos configuration
    let conf = get_configuration(None).await?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    // Create progress registry
    let progress_registry = ProgressRegistry::new();

    // Generate route list from Leptos app
    let routes = generate_route_list(App);

    // Store config values before moving
    let db_path = config.magnetar_db_path.clone();

    // Create app state
    let app_state = AppState {
        leptos_options: leptos_options.clone(),
        progress_registry,
        config,
    };

    // Build the Axum router
    let app = Router::new()
        // WebSocket endpoint for progress updates
        .route("/ws/progress/:job_id", get(ws_progress_handler))
        // Health check endpoint
        .route("/api/health", get(health_check))
        // Serve static files (if any)
        .nest_service("/pkg", ServeDir::new("./pkg"))
        // Leptos routes
        .leptos_routes(&app_state, routes, App)
        .with_state(app_state);

    info!("Listening on http://{}", addr);
    info!("Magnetar DB: {}", db_path);

    // Start the server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
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
