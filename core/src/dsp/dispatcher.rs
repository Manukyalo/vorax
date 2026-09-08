//! VORAX Dual-Mode Pipeline State Dispatcher
//!
//! Intelligently routes incoming CSI frames:
//! - When kinetic motion energy is high -> Routes to STFT Doppler velocity tracking
//! - When subject is resting / quasi-static -> Routes to Micro-Doppler vitals decomposition
//! - When sudden acceleration spike followed by zero velocity occurs -> Triggers fall verification

use super::classifier::{MotionClassifier, TargetState};
use super::doppler::{DopplerProfile, StftProcessor};
use super::vitals::{VitalsExtractor, VitalsMetrics};

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub state: TargetState,
    pub vitals: Option<VitalsMetrics>,
    pub doppler: Option<DopplerProfile>,
    pub presence_detected: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct StateDispatcher {
    classifier: MotionClassifier,
    stft: StftProcessor,
    vitals: VitalsExtractor,
    sample_rate: f32,
    presence_confidence: f32,
}

impl StateDispatcher {
    pub fn new(sample_rate: f32, carrier_freq: f32) -> Self {
        Self {
            classifier: MotionClassifier::new(2.5, 14.0),
            stft: StftProcessor::new(64, sample_rate, carrier_freq),
            vitals: VitalsExtractor::new(sample_rate),
            sample_rate,
            presence_confidence: 0.0,
        }
    }

    /// Dispatches a single frame's subcarrier amplitude vector through the appropriate DSP pipeline
    pub fn dispatch(&mut self, amplitudes: &[f32]) -> PipelineOutput {
        if amplitudes.is_empty() {
            return PipelineOutput {
                state: TargetState::EmptyZone,
                vitals: None,
                doppler: None,
                presence_detected: false,
                confidence: 0.0,
            };
        }

        // 1. Classify state from subcarrier variance
        let state = self.classifier.classify(amplitudes);

        // Center subcarrier (subcarrier 32 in standard 64-subcarrier OFDM)
        let center_amp = amplitudes.get(amplitudes.len() / 2).copied().unwrap_or(0.0);

        match state {
            TargetState::DynamicMotion | TargetState::FallSuspected => {
                // High motion: update STFT Doppler engine
                self.stft.push_sample(center_amp);
                let doppler = self.stft.compute_spectrum();
                self.presence_confidence = (self.presence_confidence * 0.8 + 0.99 * 0.2).min(1.0);

                PipelineOutput {
                    state,
                    vitals: None,
                    doppler,
                    presence_detected: true,
                    confidence: self.presence_confidence,
                }
            }
            TargetState::QuasiStaticVitals => {
                // Quasi-static: target is resting, run physiological decomposition
                let vitals = self.vitals.process_sample(center_amp);
                self.presence_confidence = (self.presence_confidence * 0.85 + 0.98 * 0.15).min(1.0);

                PipelineOutput {
                    state,
                    vitals: Some(vitals),
                    doppler: None,
                    presence_detected: true,
                    confidence: self.presence_confidence,
                }
            }
            TargetState::EmptyZone => {
                self.presence_confidence = (self.presence_confidence * 0.95).max(0.0);
                let is_present = self.presence_confidence > 0.4;

                PipelineOutput {
                    state,
                    vitals: None,
                    doppler: None,
                    presence_detected: is_present,
                    confidence: self.presence_confidence,
                }
            }
        }
    }
}
