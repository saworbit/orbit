//! Orbit Nebula Web Server
//!
//! Next-generation real-time web control center for Orbit

use orbit_web::{start_server, WebConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let config = WebConfig {
        host: env::var("ORBIT_WEB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        port: env::var("ORBIT_WEB_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080),
        magnetar_db: env::var("ORBIT_MAGNETAR_DB").unwrap_or_else(|_| "magnetar.db".to_string()),
        user_db: env::var("ORBIT_USER_DB").unwrap_or_else(|_| "orbit-web-users.db".to_string()),
    };

    // Check for JWT secret
    if env::var("ORBIT_JWT_SECRET").is_err() {
        eprintln!("⚠️  WARNING: ORBIT_JWT_SECRET not set!");
        eprintln!("   Using insecure default. Set ORBIT_JWT_SECRET in production!");
        eprintln!("   Example: export ORBIT_JWT_SECRET=$(openssl rand -base64 32)");
        eprintln!();
    }

    // Start server
    start_server(config).await
}
