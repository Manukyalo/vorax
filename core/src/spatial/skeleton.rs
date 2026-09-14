//! VORAX 17-Keypoint 3D Skeletal Kinematics & Forward Pose Engine
//!
//! Reconstructs anatomical human joint coordinates (COCO-17 topology) from
//! 2D-MUSIC target coordinates, Doppler velocity, and kinetic state classifications.

use super::Point3D;
use std::f32::consts::PI;

/// Total number of tracked anatomical keypoints in the COCO standard
pub const COCO_KEYPOINT_COUNT: usize = 17;

/// Canonical COCO Keypoint Index enumeration
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeypointId {
    Nose = 0,
    LeftEye = 1,
    RightEye = 2,
    LeftEar = 3,
    RightEar = 4,
    LeftShoulder = 5,
    RightShoulder = 6,
    LeftElbow = 7,
    RightElbow = 8,
    LeftWrist = 9,
    RightWrist = 10,
    LeftHip = 11,
    RightHip = 12,
    LeftKnee = 13,
    RightKnee = 14,
    LeftAnkle = 15,
    RightAnkle = 16,
}

/// 16 Anatomical Limb Connections (Bone segments) for 3D rendering
pub const COCO_LIMBS: [(usize, usize); 16] = [
    (0, 1), (0, 2),        // Nose to Eyes
    (1, 3), (2, 4),        // Eyes to Ears
    (5, 6),                // Clavicle (Shoulder to Shoulder)
    (5, 7), (7, 9),        // Left Arm (Shoulder -> Elbow -> Wrist)
    (6, 8), (8, 10),       // Right Arm (Shoulder -> Elbow -> Wrist)
    (5, 11), (6, 12),      // Left/Right Torso
    (11, 12),              // Pelvis (Hip to Hip)
    (11, 13), (13, 15),    // Left Leg (Hip -> Knee -> Ankle)
    (12, 14), (14, 16),    // Right Leg (Hip -> Knee -> Ankle)
];

/// Nominal rest offsets in meters from torso center (0, 0, 0)
const NOMINAL_OFFSETS: [[f32; 3]; 17] = [
    [0.0,   0.08,  0.42],  // 0: Nose (x, y_depth, z_height)
    [-0.03, 0.07,  0.45],  // 1: Left Eye
    [0.03,  0.07,  0.45],  // 2: Right Eye
    [-0.07, 0.02,  0.43],  // 3: Left Ear
    [0.07,  0.02,  0.43],  // 4: Right Ear
    [-0.20, 0.00,  0.32],  // 5: Left Shoulder
    [0.20,  0.00,  0.32],  // 6: Right Shoulder
    [-0.28, 0.02,  0.08],  // 7: Left Elbow
    [0.28,  0.02,  0.08],  // 8: Right Elbow
    [-0.32, 0.05, -0.15],  // 9: Left Wrist
    [0.32,  0.05, -0.15],  // 10: Right Wrist
    [-0.12, 0.00, -0.10],  // 11: Left Hip
    [0.12,  0.00, -0.10],  // 12: Right Hip
    [-0.14, 0.02, -0.50],  // 13: Left Knee
    [0.14,  0.02, -0.50],  // 14: Right Knee
    [-0.14, 0.00, -0.88],  // 15: Left Ankle
    [0.14,  0.00, -0.88],  // 16: Right Ankle
];

/// Dynamic forward kinematics solver for human skeleton pose tracking
#[derive(Debug, Clone)]
pub struct SkeletonSolver {
    gait_phase: f32,
    prev_x: f32,
    prev_y: f32,
}

impl SkeletonSolver {
    pub fn new() -> Self {
        Self {
            gait_phase: 0.0,
            prev_x: 3.0,
            prev_y: 3.0,
        }
    }

    /// Solves 17 3D world coordinates for the subject
    pub fn solve_skeleton(
        &mut self,
        target: &Point3D,
        velocity_m_s: f32,
        is_fall: bool,
        dt_secs: f32,
    ) -> Vec<[f32; 3]> {
        // Measure real displacement delta
        let dx_disp = target.x - self.prev_x;
        let dy_disp = target.y - self.prev_y;
        let disp = (dx_disp * dx_disp + dy_disp * dy_disp).sqrt();
        self.prev_x = target.x;
        self.prev_y = target.y;

        // Advance gait stride cycle proportional to physical motion
        let speed = velocity_m_s.abs().max(disp / dt_secs.max(0.001));
        if speed > 0.12 {
            self.gait_phase = (self.gait_phase + speed * dt_secs * 6.28).rem_euclid(2.0 * PI);
        }

        let phi = self.gait_phase;
        let mut keypoints = Vec::with_capacity(17);

        for (idx, offset) in NOMINAL_OFFSETS.iter().enumerate() {
            let (base_x, base_y, base_z) = (offset[0], offset[1], offset[2]);

            if is_fall || target.z < 0.5 {
                // Prone / Fallen pose on floor
                let jx = target.x + base_x * 1.6;
                let jy = target.y + base_z * 0.8;
                let jz = 0.12 + (base_y.abs() * 0.15);
                keypoints.push([round_3(jx), round_3(jy), round_3(jz)]);
            } else if speed > 0.12 {
                // Walking kinematics
                let (dx, dy, dz) = match idx {
                    15 | 13 => {
                        // Left leg swing
                        let stride = phi.sin();
                        (0.0, stride * 0.22 * (speed / 1.2).min(1.0), -phi.cos().abs() * 0.08)
                    }
                    16 | 14 => {
                        // Right leg swing (anti-phase)
                        let stride = (phi + PI).sin();
                        (0.0, stride * 0.22 * (speed / 1.2).min(1.0), -(phi + PI).cos().abs() * 0.08)
                    }
                    9 | 7 => {
                        // Left arm swing
                        let swing = (phi + PI).sin();
                        (0.0, swing * 0.18 * (speed / 1.2).min(1.0), 0.0)
                    }
                    10 | 8 => {
                        // Right arm swing
                        let swing = phi.sin();
                        (0.0, swing * 0.18 * (speed / 1.2).min(1.0), 0.0)
                    }
                    _ => {
                        // Torso / Head vertical bounce and slight lateral sway
                        let bounce = (phi * 2.0).sin() * 0.025;
                        let sway = (phi).sin() * 0.015;
                        (sway, 0.0, bounce)
                    }
                };

                let jx = target.x + base_x + dx;
                let jy = target.y + base_y + dy;
                let jz = target.z + base_z + dz;
                keypoints.push([round_3(jx), round_3(jy), round_3(jz)]);
            } else {
                // Stationary upright stance with subtle micro-motion
                let sway = (phi * 0.5).sin() * 0.01;
                let breath = (phi * 0.3).cos() * 0.008;
                let jx = target.x + base_x + sway;
                let jy = target.y + base_y;
                let jz = target.z + base_z + breath;
                keypoints.push([round_3(jx), round_3(jy), round_3(jz)]);
            }
        }

        keypoints
    }
}

#[inline(always)]
fn round_3(val: f32) -> f32 {
    (val * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_solver_17_joints() {
        let mut solver = SkeletonSolver::new();
        let target = Point3D::new(3.0, 3.0, 1.2);
        let joints = solver.solve_skeleton(&target, 0.8, false, 0.05);

        assert_eq!(joints.len(), 17);
        // Head / Nose should be above feet
        assert!(joints[0][2] > joints[15][2], "Nose z should be higher than ankle z");
        assert!(joints[0][2] > 1.2, "Head should be above torso origin");
    }

    #[test]
    fn test_skeleton_solver_fall_state() {
        let mut solver = SkeletonSolver::new();
        let target = Point3D::new(3.0, 3.0, 0.3);
        let joints = solver.solve_skeleton(&target, 0.0, true, 0.05);

        assert_eq!(joints.len(), 17);
        for j in &joints {
            assert!(j[2] < 0.6, "In fallen pose, all joints should be near floor plane");
        }
    }
}
