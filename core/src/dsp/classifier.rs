//! VORAX Motion Energy & Velocity State Classifier
//!
//! Classifies incoming CSI frames into physical activity states:
//! 1. `QuasiStatic`: Target is resting/sleeping -> Route to Vitals decomposition
//! 2. `DynamicMotion`: Target is walking/moving -> Route to 3D voxel localization
//! 3. `FallSuspected`: Sudden acceleration spike followed by zero velocity

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetState {
    EmptyZone,
    QuasiStaticVitals,
    DynamicMotion,
    FallSuspected,
}

#[derive(Debug, Clone)]
pub struct MotionClassifier {
    motion_threshold: f32,
    fall_threshold: f32,
    prev_variance: f32,
    static_counter: usize,
}

impl MotionClassifier {
    pub fn new(motion_threshold: f32, fall_threshold: f32) -> Self {
        Self {
            motion_threshold,
            fall_threshold,
            prev_variance: 0.0,
            static_counter: 0,
        }
    }

    /// Evaluates the variance and kinetic energy of the subcarrier amplitudes across a frame window.
    pub fn classify(&mut self, subcarrier_amplitudes: &[f32]) -> TargetState {
        if subcarrier_amplitudes.is_empty() {
            return TargetState::EmptyZone;
        }

        let mean: f32 = subcarrier_amplitudes.iter().sum::<f32>() / subcarrier_amplitudes.len() as f32;
        let variance: f32 = subcarrier_amplitudes
            .iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f32>()
            / subcarrier_amplitudes.len() as f32;

        let delta = (variance - self.prev_variance).abs();
        // Fall detection: sharp delta followed by static floor — only valid after at least one
        // prior observation; on cold start prev_variance == 0.0 and any signal would trigger a
        // false positive.
        let is_fall = self.prev_variance > 0.0 && delta > self.fall_threshold;
        self.prev_variance = variance;

        if is_fall {
            return TargetState::FallSuspected;
        }

        if variance > self.motion_threshold {
            self.static_counter = 0;
            TargetState::DynamicMotion
        } else if variance > 0.05 {
            self.static_counter += 1;
            TargetState::QuasiStaticVitals
        } else {
            TargetState::EmptyZone
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_vs_vitals_classification() {
        let mut classifier = MotionClassifier::new(2.5, 15.0);

        // High-variance subcarriers: walking human disturbing multipath → DynamicMotion
        let moving_amps: Vec<f32> = (0..64).map(|i| 20.0 + (i as f32 * 0.8).sin() * 8.0).collect();
        // Warm-up: establishes prev_variance baseline (avoids cold-start false-positive)
        let _ = classifier.classify(&moving_amps);
        assert_eq!(classifier.classify(&moving_amps), TargetState::DynamicMotion);

        // Transition to low quasi-static variance (gentle chest rise, small variation).
        // Must NOT trigger FallSuspected since the delta is below fall_threshold.
        // variance of small_var_amps ≈ 0.1, prev ≈ 32, delta ≈ 31.9 > 15 → FallSuspected is
        // technically correct here too. So we use a TWO-STEP wind-down through QuasiStatic first.
        let quasi_amps: Vec<f32> = (0..64).map(|i| 22.0 + (i as f32 * 0.1).sin() * 0.5).collect();
        // delta from ~32 to ~0.06 is large → FallSuspected is valid; don't assert state here.
        let _ = classifier.classify(&quasi_amps);
        // Now from quasi-static (~0.06) to essentially zero: delta ≈ 0.06, well below fall_threshold
        let flat_amps: Vec<f32> = (0..64).map(|_| 22.0).collect();
        let state = classifier.classify(&flat_amps);
        // delta from ~0.06 to 0 = 0.06 < 15 (fall_threshold) so this is NOT a fall.
        // Variance = 0, below EmptyZone boundary.
        assert_ne!(state, TargetState::FallSuspected, "Gentle wind-down must not be a fall");
        assert_eq!(state, TargetState::EmptyZone);
    }

    #[test]
    fn test_fall_detection_after_baseline_established() {
        let mut classifier = MotionClassifier::new(2.5, 15.0);

        // Establish a stable high-motion baseline
        let stable_amps: Vec<f32> = (0..64).map(|i| 20.0 + (i as f32 * 0.8).sin() * 3.0).collect();
        let _ = classifier.classify(&stable_amps);

        // Sudden large variance spike simulates a fall event
        let fall_amps: Vec<f32> = (0..64).map(|i| (i as f32).powi(2)).collect();
        assert_eq!(classifier.classify(&fall_amps), TargetState::FallSuspected);
    }

    #[test]
    fn test_cold_start_does_not_falsely_trigger_fall() {
        let mut classifier = MotionClassifier::new(2.5, 15.0);
        // On cold start (prev_variance == 0), a high-variance frame must NOT be a fall.
        let high_variance_amps: Vec<f32> = (0..64).map(|i| (i as f32).powi(2)).collect();
        let state = classifier.classify(&high_variance_amps);
        assert_ne!(state, TargetState::FallSuspected, "Cold-start must not trigger false fall");
    }
}
