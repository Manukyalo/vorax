//! VORAX Spatial Localization & 3D Voxel Engine
//!
//! Sub-modules:
//! - `smoothing`: Spatial-frequency forward-backward smoothing covariance matrix
//! - `music`: 2D-MUSIC AoA & ToF pseudo-spectrum estimation
//! - `multilateration`: Multi-node range intersection into 3D coordinates (x, y, z)
//! - `occupancy`: Volumetric 3D voxel occupancy field and point cloud extraction

pub mod smoothing;
pub mod music;
pub mod multilateration;
pub mod occupancy;

pub use smoothing::{Complex32, ComplexMatrix, compute_smoothed_covariance};
pub use music::{MusicEngine2D, SpatialPeak};
pub use multilateration::{Multilaterator, NodeObservation, Point3D};
pub use occupancy::VoxelOccupancyGrid;
