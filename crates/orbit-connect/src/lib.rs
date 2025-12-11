//! Orbit Connect: Client-side gRPC connectivity for Nucleus to control remote Stars
//!
//! This crate provides the client-side implementation for the Orbit Grid, allowing
//! the Nucleus (Hub) to orchestrate operations on remote Stars (Agents).
//!
//! # Architecture
//!
//! - **RemoteSystem**: Implements `OrbitSystem` trait by proxying to a remote Star via gRPC
//! - **StarManager**: Connection pool and session management for multiple Stars
//!
//! # Example
//!
//! ```rust,no_run
//! use orbit_connect::RemoteSystem;
//! use orbit_core_interface::OrbitSystem;
//! use tonic::transport::Channel;
//!
//! async fn example() -> anyhow::Result<()> {
//!     let channel = Channel::from_static("http://10.0.0.5:50051").connect().await?;
//!     let system = RemoteSystem::new(channel, "session-123".to_string());
//!
//!     // Now use it like any OrbitSystem
//!     let hash = system.calculate_hash(std::path::Path::new("/data/file.bin"), 0, 1024).await?;
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod manager;
pub mod system;

pub use error::ConnectError;
pub use manager::{StarManager, StarRecord, StarStatus};
pub use system::RemoteSystem;
