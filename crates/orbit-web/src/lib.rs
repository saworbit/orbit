//! Orbit Web - Web GUI for Orbit file transfer orchestration
//!
//! This module provides a full-stack Rust web interface for managing
//! Orbit file transfers with real-time progress tracking and job management.

pub mod app;
pub mod components;
pub mod error;
pub mod progress;
pub mod server_fns;
pub mod types;

pub use app::App;
pub use error::{WebError, WebResult};
pub use progress::ProgressRegistry;

use leptos::*;

/// Configuration for the web server
#[derive(Debug, Clone)]
pub struct WebConfig {
    pub port: u16,
    pub host: String,
    pub magnetar_db_path: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            magnetar_db_path: "orbit-web.db".to_string(),
        }
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
