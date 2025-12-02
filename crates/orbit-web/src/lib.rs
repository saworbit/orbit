//! Orbit Server - Control Plane v2.2.0-alpha
//!
//! Headless API and Orchestration Engine for Orbit file transfers.
//! Built with Axum, OpenAPI/Swagger, JWT authentication, and WebSocket real-time updates.

pub mod api;
pub mod auth;
pub mod error;
pub mod state;
pub mod utils;
pub mod ws;

pub mod server;

pub use error::{WebError, WebResult};
pub use state::AppState;

/// Configuration for Control Plane server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub magnetar_db: String,
    pub user_db: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            magnetar_db: "magnetar.db".to_string(),
            user_db: "orbit-server-users.db".to_string(),
        }
    }
}

/// Start the Control Plane API server
pub async fn start_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error + Send>> {
    server::run_server(config).await
}
