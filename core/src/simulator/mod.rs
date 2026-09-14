//! VORAX 4-Node Spatial RF Mesh Simulator
//!
//! Exported as a library module for integration tests.
//! The simulator binary is at src/simulator/main.rs (compiled as orax-sim).
pub mod mesh;
pub mod protocol {
    pub use crate::protocol::*;
}
