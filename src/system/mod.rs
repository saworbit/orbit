//! System implementations for OrbitSystem trait
//!
//! This module provides concrete implementations of the `OrbitSystem` trait:
//! - `LocalSystem`: Direct filesystem access for standalone mode
//! - `MockSystem`: In-memory implementation for testing (in tests module)

mod local;

pub use local::LocalSystem;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub use mock::MockSystem;
