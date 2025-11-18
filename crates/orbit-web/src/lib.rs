//! Orbit Web - Nebula v1.0.0
//!
//! Next-generation real-time web control center for Orbit file transfers.
//! Built with Axum, Leptos, JWT authentication, and WebSocket real-time updates.

pub mod api;
pub mod auth;
pub mod components;
pub mod error;
pub mod pages;
pub mod state;
pub mod utils;
pub mod ws;

#[cfg(feature = "ssr")]
pub mod server;

pub use error::{WebError, WebResult};
pub use state::AppState;

// Export app root component
pub use components::App;

/// Re-export for Leptos hydration
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}

/// Configuration for web server
#[derive(Debug, Clone)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
    pub magnetar_db: String,
    pub user_db: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            magnetar_db: "magnetar.db".to_string(),
            user_db: "orbit-web-users.db".to_string(),
        }
    }
}

/// Start the web server
#[cfg(feature = "ssr")]
pub async fn start_server(config: WebConfig) -> Result<(), Box<dyn std::error::Error + Send>> {
    server::run_server(config).await
}
