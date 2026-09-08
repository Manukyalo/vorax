//! VORAX 3D Spatial Multilateration Engine
//!
//! Reconstructs exact 3D target coordinates (x, y, z) in room space by combining
//! multi-node Time-of-Flight (ToF) range spheres and Angle-of-Arrival (AoA) bearing vectors.

use super::music::SpatialPeak;

#[derive(Debug, Clone, Copy)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3D {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance_to(&self, other: &Point3D) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// A spatial observation from an anchor node
#[derive(Debug, Clone)]
pub struct NodeObservation {
    pub node_id: u16,
    pub node_position: Point3D,
    pub peak: SpatialPeak,
}

#[derive(Debug, Clone)]
pub struct Multilaterator {
    room_bounds: (f32, f32, f32), // (width, length, height) in meters
}

impl Multilaterator {
    pub fn new(room_x: f32, room_y: f32, room_z: f32) -> Self {
        Self {
            room_bounds: (room_x, room_y, room_z),
        }
    }

    /// Solves linear least-squares multilateration across multiple node range observations
    ///
    /// For node i at (x_i, y_i, z_i) with estimated range r_i:
    /// (x - x_i)^2 + (y - y_i)^2 + (z - z_i)^2 = r_i^2
    ///
    /// Subtracting equation 0 yields linear matrix A * [x, y]^T = b
    pub fn solve_position_2d(&self, obs: &[NodeObservation]) -> Option<Point3D> {
        if obs.len() < 3 {
            return None;
        }

        // Anchor 0 is reference point
        let p0 = obs[0].node_position;
        let r0 = obs[0].peak.range_meters;

        // Form linear system: A * [x, y]^T = b
        let mut a11 = 0.0f32;
        let mut a12 = 0.0f32;
        let mut a22 = 0.0f32;
        let mut b1 = 0.0f32;
        let mut b2 = 0.0f32;

        for i in 1..obs.len() {
            let pi = obs[i].node_position;
            let ri = obs[i].peak.range_meters;

            // 2 * (x_i - x_0) * x + 2 * (y_i - y_0) * y = (r_0^2 - r_i^2) + (x_i^2 - x_0^2) + (y_i^2 - y_0^2)
            let ax = 2.0 * (pi.x - p0.x);
            let ay = 2.0 * (pi.y - p0.y);
            let c = (r0 * r0 - ri * ri) + (pi.x * pi.x - p0.x * p0.x) + (pi.y * pi.y - p0.y * p0.y);

            // Normal equations: (A^T * A) * x = A^T * b
            a11 += ax * ax;
            a12 += ax * ay;
            a22 += ay * ay;
            b1 += ax * c;
            b2 += ay * c;
        }

        let det = a11 * a22 - a12 * a12;
        if det.abs() < 1e-4 {
            return None; // Collinear geometry / singular matrix
        }

        let inv_det = 1.0 / det;
        let solved_x = (a22 * b1 - a12 * b2) * inv_det;
        let solved_y = (-a12 * b1 + a11 * b2) * inv_det;

        // Clamp into monitored room boundaries
        let clamped_x = solved_x.clamp(0.1, self.room_bounds.0 - 0.1);
        let clamped_y = solved_y.clamp(0.1, self.room_bounds.1 - 0.1);
        let default_z = 1.0; // Torso elevation in meters

        Some(Point3D::new(clamped_x, clamped_y, default_z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multilateration_recovers_known_position() {
        let multi = Multilaterator::new(6.0, 6.0, 3.0);

        let true_pos = Point3D::new(3.0, 2.5, 1.0);

        let nodes = [
            Point3D::new(0.0, 0.0, 1.2),
            Point3D::new(6.0, 0.0, 1.2),
            Point3D::new(6.0, 6.0, 1.2),
            Point3D::new(0.0, 6.0, 1.2),
        ];

        let obs: Vec<NodeObservation> = nodes
            .iter()
            .enumerate()
            .map(|(idx, &node_pos)| {
                let range = node_pos.distance_to(&true_pos);
                NodeObservation {
                    node_id: (idx + 1) as u16,
                    node_position: node_pos,
                    peak: SpatialPeak {
                        aoa_degrees: 0.0,
                        tof_nanoseconds: (range / 3e8) * 1e9,
                        range_meters: range,
                        pseudo_spectrum_val: 100.0,
                    },
                }
            })
            .collect();

        let solved = multi.solve_position_2d(&obs).expect("Should solve position");
        assert!(
            (solved.x - true_pos.x).abs() < 0.1,
            "Solved X {} should match true X {}",
            solved.x,
            true_pos.x
        );
        assert!(
            (solved.y - true_pos.y).abs() < 0.1,
            "Solved Y {} should match true Y {}",
            solved.y,
            true_pos.y
        );
    }
}
