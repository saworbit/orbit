//! Orbit Control Plane Server
//!
//! Headless API and Orchestration Engine for Orbit V2.2.0

use orbit_server::{reactor::Reactor, start_server, ServerConfig};
use sqlx::SqlitePool;
use std::{env, sync::Arc};
use tokio::sync::Notify;

#[cfg(feature = "sentinel")]
use orbit_connect::StarManager;
#[cfg(feature = "sentinel")]
use orbit_core_starmap::universe_v3::Universe;
#[cfg(feature = "sentinel")]
use orbit_sentinel::{Sentinel, SentinelPolicy};
#[cfg(feature = "sentinel")]
use orbit_star::auth::AuthService;

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

    // Create shared notification channel for reactor
    let reactor_notify = Arc::new(Notify::new());

    // Initialize database pool for reactor
    let reactor_pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", config.magnetar_db))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

    println!("☢️  Starting Orbit Reactor (job execution engine)...");

    // Start reactor in background
    let reactor = Reactor::new(reactor_pool, reactor_notify.clone());
    tokio::spawn(async move {
        reactor.run().await;
    });

    // Optionally start Sentinel (resilience engine)
    #[cfg(feature = "sentinel")]
    if env::var("ORBIT_SENTINEL_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        == "true"
    {
        println!("🛡️  Starting Orbit Sentinel (autonomous resilience engine)...");

        // Load Sentinel configuration from environment
        let universe_db_path =
            env::var("ORBIT_UNIVERSE_DB").unwrap_or_else(|_| "universe_v3.db".to_string());

        let auth_secret = env::var("ORBIT_AUTH_SECRET").unwrap_or_else(|_| {
            eprintln!("⚠️  WARNING: ORBIT_AUTH_SECRET not set for Sentinel!");
            eprintln!("   Using insecure default. Set ORBIT_AUTH_SECRET in production!");
            eprintln!("   This secret is shared with Stars for P2P transfer authentication.");
            eprintln!();
            "insecure-default-secret".to_string()
        });

        let sentinel_policy = SentinelPolicy {
            min_redundancy: env::var("ORBIT_SENTINEL_MIN_REDUNDANCY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),
            max_parallel_heals: env::var("ORBIT_SENTINEL_MAX_PARALLEL_HEALS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            scan_interval_s: env::var("ORBIT_SENTINEL_SCAN_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            healing_bandwidth_limit: env::var("ORBIT_SENTINEL_BANDWIDTH_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .or(Some(50 * 1024 * 1024)), // Default: 50 MB/s
        };

        println!(
            "   Min Redundancy: {} copies",
            sentinel_policy.min_redundancy
        );
        println!(
            "   Scan Interval: {} seconds",
            sentinel_policy.scan_interval_s
        );
        println!(
            "   Max Parallel Heals: {}",
            sentinel_policy.max_parallel_heals
        );
        println!();

        // Initialize Universe V3
        let universe = match Universe::open(&universe_db_path) {
            Ok(db) => Arc::new(db),
            Err(e) => {
                eprintln!(
                    "❌ Failed to open Universe database at {}: {}",
                    universe_db_path, e
                );
                eprintln!("   Sentinel will not start. Check the ORBIT_UNIVERSE_DB path.");
                return Err(Box::new(e) as Box<dyn std::error::Error + Send>);
            }
        };

        // Initialize auth service for P2P transfers
        let auth_service = Arc::new(AuthService::new(&auth_secret));

        // Initialize Star manager
        let star_manager = Arc::new(StarManager::new());

        // Create and spawn Sentinel
        let sentinel = Sentinel::new(universe, auth_service, star_manager, sentinel_policy);

        tokio::spawn(async move {
            sentinel.run().await;
        });

        println!("✅ Sentinel active and monitoring grid health");
        println!();
    }

    // Start API server
    start_server(config, reactor_notify).await
}
