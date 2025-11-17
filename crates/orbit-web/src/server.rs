//! Axum server setup with Leptos integration

use crate::{api, state::AppState, ws, WebConfig};
use axum::{
    routing::{get, post},
    Router,
};
use leptos::*;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};

/// Run the Axum + Leptos server
pub async fn run_server(config: WebConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,orbit_web=debug".into()),
        )
        .json()
        .init();

    tracing::info!("Starting Orbit Nebula web server v1.0.0-alpha.2");
    tracing::info!("Magnetar DB: {}", config.magnetar_db);
    tracing::info!("User DB: {}", config.user_db);

    // Initialize application state
    let state = AppState::new(&config.magnetar_db, &config.user_db).await?;

    tracing::info!("Application state initialized");

    // Build Axum router (simplified for MVP - Leptos integration will be added in next iteration)
    let app = Router::new()
        // WebSocket endpoint
        .route("/ws/*path", get(ws::ws_handler))
        // Auth endpoints
        .route("/api/auth/login", post(api::login_handler))
        .route("/api/auth/logout", post(api::logout_handler))
        .route("/api/auth/me", get(api::me_handler))
        // Health check
        .route(
            "/api/health",
            get(|| async {
                axum::Json(serde_json::json!({
                    "status": "ok",
                    "service": "orbit-web",
                    "version": "1.0.0-alpha.2"
                }))
            }),
        )
        // Serve static files
        .nest_service("/pkg", ServeDir::new("target/site/pkg"))
        .nest_service("/public", ServeDir::new("crates/orbit-web/public"))
        // Fallback for SPA
        .fallback(get(|| async {
            axum::response::Html(include_str!("../public/index.html"))
        }))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("🚀 Nebula listening on http://{}", addr);
    tracing::info!("   Dashboard: http://{}/", addr);
    tracing::info!("   Login: http://{}/login", addr);
    tracing::info!("   Health: http://{}/api/health", addr);

    // Run server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
