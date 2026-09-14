//! VORAX Core Library — public API for integration tests and downstream consumers.
//!
//! All production DSP, spatial, simulator, and protocol modules are re-exported
//! from the library target so that core/tests/ integration tests can reference
//! them without duplicating module declarations.

pub mod protocol;
pub mod dsp;
pub mod spatial;
pub mod simulator;
pub mod audit;
// ingest is tokio-runtime dependent; excluded from lib to keep tests hermetic.
