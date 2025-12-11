//! Orbit Star Agent - Remote execution server for the Orbit Grid.
//!
//! The Star agent exposes local filesystem and CPU resources to the Nucleus (Hub)
//! over gRPC. It provides secure, sandboxed access for distributed data operations.

mod security;
mod server;

use anyhow::{Context, Result};
use clap::Parser;
use orbit_proto::star_service_server::StarServiceServer;
use server::StarImpl;
use std::path::PathBuf;
use tonic::transport::Server;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Orbit Star Agent - Remote execution server for distributed data operations.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value = "50051")]
    port: u16,

    /// Authentication token (shared secret with Nucleus)
    #[arg(short, long, env = "ORBIT_STAR_TOKEN")]
    token: String,

    /// Allowed root directories (can be specified multiple times)
    #[arg(short, long = "allow", required = true)]
    allow_paths: Vec<PathBuf>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Bind address (default: all interfaces)
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // Initialize logging
    let filter = if args.debug {
        "debug,orbit_star=trace"
    } else {
        "info"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Validate configuration
    if args.token.is_empty() {
        anyhow::bail!("Authentication token cannot be empty");
    }

    if args.allow_paths.is_empty() {
        anyhow::bail!("At least one allowed path must be specified");
    }

    // Log configuration
    info!("🚀 Orbit Star Agent v{}", env!("CARGO_PKG_VERSION"));
    info!("📁 Allowed directories:");
    for path in &args.allow_paths {
        if !path.exists() {
            warn!("  ⚠️  {} (does not exist)", path.display());
        } else {
            info!("  ✓ {}", path.display());
        }
    }

    // Create the server address
    let addr = format!("{}:{}", args.bind, args.port)
        .parse()
        .context("Failed to parse bind address")?;

    // Create the Star service implementation
    let star = StarImpl::new(args.allow_paths, args.token);

    info!("✨ Starting gRPC server on {}", addr);

    // Build and start the gRPC server
    Server::builder()
        .add_service(StarServiceServer::new(star))
        .serve(addr)
        .await
        .context("gRPC server failed")?;

    Ok(())
}
