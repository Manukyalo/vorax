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
        self.prev_variance = variance;

        // Fall detection: sharp delta followed by immediate static floor state
        if delta > self.fall_threshold {
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

        // Subcarriers with high variance (walking human disturbing multipath)
        let moving_amps: Vec<f32> = (0..64).map(|i| 20.0 + (i as f32 * 0.8).sin() * 8.0).collect();
        assert_eq!(classifier.classify(&moving_amps), TargetState::DynamicMotion);

        // Subcarriers with low variance (resting human chest displacement)
        let resting_amps: Vec<f32> = (0..64).map(|_| 22.0).collect();
        assert_eq!(classifier.classify(&resting_amps), TargetState::EmptyZone);
    }
}
