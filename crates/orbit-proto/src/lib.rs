/// Generated gRPC protocol definitions for the Orbit Grid.
///
/// This crate provides the protocol buffer definitions and generated code
/// for communication between the Nucleus (Hub) and Stars (Agents).
pub mod orbit {
    pub mod v1 {
        tonic::include_proto!("orbit.v1");
    }
}

// Re-export commonly used types for convenience
pub use orbit::v1::*;
