//! VORAX 3D Volumetric Occupancy Voxel Grid
//!
//! Maintains a probabilistic 3D voxel field representing RF reflection density across a room:
//! - Sub-meter voxel resolution (e.g. 0.25m per voxel)
//! - Temporal decay persistence for continuous tracking
//! - Gaussian spatial deposit kernel for localized human target point-clouds

use super::multilateration::Point3D;

pub const DEFAULT_GRID_X: usize = 24; // 24 * 0.25m = 6.0m
pub const DEFAULT_GRID_Y: usize = 24; // 24 * 0.25m = 6.0m
pub const DEFAULT_GRID_Z: usize = 12; // 12 * 0.25m = 3.0m
pub const VOXEL_RESOLUTION_METERS: f32 = 0.25;

#[derive(Debug, Clone)]
pub struct VoxelOccupancyGrid {
    dim_x: usize,
    dim_y: usize,
    dim_z: usize,
    resolution: f32,
    probabilities: Vec<f32>,
}

impl VoxelOccupancyGrid {
    pub fn new() -> Self {
        let total_voxels = DEFAULT_GRID_X * DEFAULT_GRID_Y * DEFAULT_GRID_Z;
        Self {
            dim_x: DEFAULT_GRID_X,
            dim_y: DEFAULT_GRID_Y,
            dim_z: DEFAULT_GRID_Z,
            resolution: VOXEL_RESOLUTION_METERS,
            probabilities: vec![0.0f32; total_voxels],
        }
    }

    #[inline(always)]
    fn index_of(&self, ix: usize, iy: usize, iz: usize) -> usize {
        (iz * self.dim_y + iy) * self.dim_x + ix
    }

    /// Converts physical coordinates (x, y, z) into voxel indices
    pub fn world_to_grid(&self, point: &Point3D) -> (usize, usize, usize) {
        let ix = ((point.x / self.resolution) as usize).clamp(0, self.dim_x - 1);
        let iy = ((point.y / self.resolution) as usize).clamp(0, self.dim_y - 1);
        let iz = ((point.z / self.resolution) as usize).clamp(0, self.dim_z - 1);
        (ix, iy, iz)
    }

    /// Converts voxel indices into physical center coordinates
    pub fn grid_to_world(&self, ix: usize, iy: usize, iz: usize) -> Point3D {
        Point3D::new(
            (ix as f32 + 0.5) * self.resolution,
            (iy as f32 + 0.5) * self.resolution,
            (iz as f32 + 0.5) * self.resolution,
        )
    }

    /// Updates the occupancy grid with a new localized human target point
    pub fn update_target(&mut self, target: &Point3D, confidence: f32) {
        // 1. Temporal decay on all existing voxels
        let decay_factor = 0.88f32;
        for val in &mut self.probabilities {
            *val *= decay_factor;
        }

        // 2. Deposit 3D Gaussian kernel around the target position
        let (cx, cy, cz) = self.world_to_grid(target);
        let kernel_radius: isize = 2;
        let sigma_sq: f32 = 0.5; // Gaussian spread in voxels

        for dz in -kernel_radius..=kernel_radius {
            let iz = cz as isize + dz;
            if iz < 0 || iz >= self.dim_z as isize {
                continue;
            }

            for dy in -kernel_radius..=kernel_radius {
                let iy = cy as isize + dy;
                if iy < 0 || iy >= self.dim_y as isize {
                    continue;
                }

                for dx in -kernel_radius..=kernel_radius {
                    let ix = cx as isize + dx;
                    if ix < 0 || ix >= self.dim_x as isize {
                        continue;
                    }

                    let dist_sq = (dx * dx + dy * dy + dz * dz) as f32;
                    let deposit = (-dist_sq / (2.0 * sigma_sq)).exp() * confidence;

                    let idx = self.index_of(ix as usize, iy as usize, iz as usize);
                    self.probabilities[idx] = (self.probabilities[idx] + deposit).min(1.0);
                }
            }
        }
    }

    /// Returns list of active 3D points and intensities exceeding an occupancy threshold
    pub fn get_active_points(&self, threshold: f32) -> Vec<(Point3D, f32)> {
        let mut points = Vec::new();

        for iz in 0..self.dim_z {
            for iy in 0..self.dim_y {
                for ix in 0..self.dim_x {
                    let p = self.probabilities[self.index_of(ix, iy, iz)];
                    if p >= threshold {
                        points.push((self.grid_to_world(ix, iy, iz), p));
                    }
                }
            }
        }

        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_occupancy_grid_updates_and_retrieves_point() {
        let mut grid = VoxelOccupancyGrid::new();
        let target = Point3D::new(2.5, 3.5, 1.0);

        grid.update_target(&target, 0.95);

        let active = grid.get_active_points(0.5);
        assert!(!active.is_empty(), "Should have active voxels near target");

        let (nearest, intensity) = active[0];
        assert!(intensity >= 0.5);
        assert!(nearest.distance_to(&target) < 0.6);
    }
}
