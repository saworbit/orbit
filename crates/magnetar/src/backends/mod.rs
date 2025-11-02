//! Backend implementations for JobStore
//!
//! This module provides different persistent storage backends:
//! - `sqlite`: SQLite-based backend (default, requires `sqlite` feature)
//! - `redb`: Pure Rust embedded database (requires `redb` feature)

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "redb")]
pub mod redb;
