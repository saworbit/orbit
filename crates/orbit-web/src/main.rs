//! Orbit Web - Binary entry point

#![cfg(feature = "ssr")]

use std::net::SocketAddr;

#[cfg(not(feature = "ssr"))]
compile_error!("The orbit-web binary requires the `ssr` feature to run.");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Default bind address; can be overridden when embedded via the main CLI.
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    orbit_web::start_server(addr).await
}
