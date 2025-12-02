//! Orbit Control Plane Server
//!
//! Headless API and Orchestration Engine for Orbit V2.2.0

use orbit_server::{start_server, ServerConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send>> {
    // Load configuration from environment variables
    let config = ServerConfig {
        host: env::var("ORBIT_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        port: env::var("ORBIT_SERVER_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080),
        magnetar_db: env::var("ORBIT_MAGNETAR_DB").unwrap_or_else(|_| "magnetar.db".to_string()),
        user_db: env::var("ORBIT_USER_DB").unwrap_or_else(|_| "orbit-server-users.db".to_string()),
    };

    // Check for JWT secret
    if env::var("ORBIT_JWT_SECRET").is_err() {
        eprintln!("⚠️  WARNING: ORBIT_JWT_SECRET not set!");
        eprintln!("   Using insecure default. Set ORBIT_JWT_SECRET in production!");
        eprintln!("   Example: export ORBIT_JWT_SECRET=$(openssl rand -base64 32)");
        eprintln!();
    }

    println!("🚀 Starting Orbit Control Plane v2.2.0-alpha");
    println!(
        "   API Endpoint: http://{}:{}/api",
        config.host, config.port
    );
    println!(
        "   Swagger UI: http://{}:{}/swagger-ui",
        config.host, config.port
    );
    println!();

    // Start server
    start_server(config).await
}
