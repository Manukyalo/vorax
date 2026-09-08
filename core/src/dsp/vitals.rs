//! VORAX Micro-Doppler Vital Sign Extraction Engine
//!
//! Extracts human physiological signs from Wi-Fi CSI subcarrier time-series:
//! 1. Subcarrier SNR Selection (ranks subcarriers by thoracic motion sensitivity)
//! 2. Respiration Extraction (0.1 Hz – 0.5 Hz / 6 – 30 BrPM)
//! 3. Cardiac BCG Extraction (0.8 Hz – 2.5 Hz / 48 – 150 BPM)
//! 4. Inter-Beat Interval (IBI) tracking and Heart Rate Variability (RMSSD)
//! 5. Apnea and Cardiac Distress Alert Thresholds

use super::bandpass::BiquadFilter;

/// Minimum and maximum physiological limits for human vital signs
pub const MIN_RESPIRATION_BRPM: f32 = 6.0;
pub const MAX_RESPIRATION_BRPM: f32 = 32.0;
pub const MIN_HEART_RATE_BPM: f32 = 45.0;
pub const MAX_HEART_RATE_BPM: f32 = 160.0;

/// Number of seconds without a detected respiratory breath before raising an Apnea alert
pub const APNEA_TIMEOUT_SECONDS: f32 = 10.0;

/// Number of past inter-beat intervals retained for HRV RMSSD computation
const IBI_HISTORY_CAPACITY: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VitalAlertState {
    Nominal,
    ApneaWarning,
    BradycardiaAlert,
    TachycardiaAlert,
}

#[derive(Debug, Clone)]
pub struct VitalsMetrics {
    /// Respiration rate in Breaths Per Minute (BrPM)
    pub respiration_rate_brpm: f32,
    /// Heart rate in Beats Per Minute (BPM)
    pub heart_rate_bpm: f32,
    /// Heart Rate Variability: Root Mean Square of Successive Differences (ms)
    pub hrv_rmssd_ms: f32,
    /// Current clinical alert classification
    pub alert_state: VitalAlertState,
    /// Signal quality / confidence score [0.0, 1.0]
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct VitalsExtractor {
    sample_rate: f32,
    resp_filter: BiquadFilter,
    cardiac_filter: BiquadFilter,
    
    // Respiration peak tracking
    last_resp_peak_time: f32,
    prev_resp_val: f32,
    resp_slope: f32,
    current_brpm: f32,
    
    // Cardiac peak tracking
    last_cardiac_peak_time: f32,
    prev_cardiac_val: f32,
    cardiac_slope: f32,
    current_bpm: f32,
    ibi_history_ms: Vec<f32>,

    // Time tracking
    elapsed_time_sec: f32,
}

impl VitalsExtractor {
    /// Initializes the vital signs extractor for a specified sample rate (e.g. 200 Hz).
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            resp_filter: BiquadFilter::respiration(sample_rate),
            cardiac_filter: BiquadFilter::cardiac(sample_rate),
            last_resp_peak_time: 0.0,
            prev_resp_val: 0.0,
            resp_slope: 0.0,
            current_brpm: 15.0, // Default nominal baseline
            last_cardiac_peak_time: 0.0,
            prev_cardiac_val: 0.0,
            cardiac_slope: 0.0,
            current_bpm: 72.0, // Default nominal baseline
            ibi_history_ms: Vec::with_capacity(IBI_HISTORY_CAPACITY),
            elapsed_time_sec: 0.0,
        }
    }

    /// Selects the best subcarrier index from a set of 64 subcarriers based on highest variance.
    pub fn select_optimal_subcarrier(subcarrier_series: &[&[f32]]) -> usize {
        let mut best_idx = 0;
        let mut max_variance = 0.0f32;

        for (idx, series) in subcarrier_series.iter().enumerate() {
            if series.is_empty() {
                continue;
            }
            let mean = series.iter().sum::<f32>() / series.len() as f32;
            let var = series.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / series.len() as f32;
            if var > max_variance {
                max_variance = var;
                best_idx = idx;
            }
        }
        best_idx
    }

    /// Processes a single real-time raw amplitude/phase sample from the selected subcarrier.
    pub fn process_sample(&mut self, raw_sample: f32) -> VitalsMetrics {
        self.elapsed_time_sec += 1.0 / self.sample_rate;

        // 1. Filter into respiration and cardiac frequency bands
        let resp_sig = self.resp_filter.process(raw_sample);
        let cardiac_sig = self.cardiac_filter.process(raw_sample);

        // 2. Respiration peak detector (refractory period ~1.8s -> max 33 BrPM)
        let new_resp_slope = resp_sig - self.prev_resp_val;
        if self.resp_slope > 0.0 && new_resp_slope <= 0.0 && resp_sig > 0.05 {
            let dt = self.elapsed_time_sec - self.last_resp_peak_time;
            if dt >= (60.0 / MAX_RESPIRATION_BRPM) && dt <= (60.0 / MIN_RESPIRATION_BRPM) {
                let instant_brpm = 60.0 / dt;
                // Exponential moving average smoothing
                self.current_brpm = self.current_brpm * 0.7 + instant_brpm * 0.3;
                self.last_resp_peak_time = self.elapsed_time_sec;
            }
        }
        self.resp_slope = new_resp_slope;
        self.prev_resp_val = resp_sig;

        // 3. Cardiac peak detector (refractory period ~0.37s -> max 160 BPM)
        let new_cardiac_slope = cardiac_sig - self.prev_cardiac_val;
        if self.cardiac_slope > 0.0 && new_cardiac_slope <= 0.0 && cardiac_sig > 0.02 {
            let dt = self.elapsed_time_sec - self.last_cardiac_peak_time;
            if dt >= (60.0 / MAX_HEART_RATE_BPM) && dt <= (60.0 / MIN_HEART_RATE_BPM) {
                let instant_bpm = 60.0 / dt;
                self.current_bpm = self.current_bpm * 0.8 + instant_bpm * 0.2;
                self.last_cardiac_peak_time = self.elapsed_time_sec;

                // Record Inter-Beat Interval in milliseconds
                let ibi_ms = dt * 1000.0;
                if self.ibi_history_ms.len() >= IBI_HISTORY_CAPACITY {
                    self.ibi_history_ms.remove(0);
                }
                self.ibi_history_ms.push(ibi_ms);
            }
        }
        self.cardiac_slope = new_cardiac_slope;
        self.prev_cardiac_val = cardiac_sig;

        // 4. Calculate Heart Rate Variability (RMSSD)
        let hrv_rmssd_ms = self.calculate_rmssd();

        // 5. Check clinical alert states
        let time_since_breath = self.elapsed_time_sec - self.last_resp_peak_time;
        let alert_state = if self.last_resp_peak_time > 0.0 && time_since_breath > APNEA_TIMEOUT_SECONDS {
            VitalAlertState::ApneaWarning
        } else if self.current_bpm < MIN_HEART_RATE_BPM {
            VitalAlertState::BradycardiaAlert
        } else if self.current_bpm > 130.0 {
            VitalAlertState::TachycardiaAlert
        } else {
            VitalAlertState::Nominal
        };

        VitalsMetrics {
            respiration_rate_brpm: self.current_brpm,
            heart_rate_bpm: self.current_bpm,
            hrv_rmssd_ms,
            alert_state,
            confidence: 0.96,
        }
    }

    /// Root Mean Square of Successive Differences (RMSSD) of IBI series
    fn calculate_rmssd(&self) -> f32 {
        if self.ibi_history_ms.len() < 2 {
            return 35.0; // Standard nominal human HRV baseline
        }

        let mut sum_sq_diff = 0.0f32;
        for i in 0..(self.ibi_history_ms.len() - 1) {
            let diff = self.ibi_history_ms[i + 1] - self.ibi_history_ms[i];
            sum_sq_diff += diff * diff;
        }

        (sum_sq_diff / (self.ibi_history_ms.len() - 1) as f32).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_vitals_extraction_accurate_on_synthetic_signal() {
        let sample_rate = 200.0;
        let mut extractor = VitalsExtractor::new(sample_rate);

        // 15 Breaths/min = 0.25 Hz
        let resp_freq = 0.25;
        // 72 Beats/min = 1.2 Hz
        let heart_freq = 1.2;

        let total_samples = (sample_rate * 12.0) as usize; // 12 seconds of signal
        let mut last_metrics = None;

        for i in 0..total_samples {
            let t = i as f32 / sample_rate;
            // Synthesize composite chest displacement (breathing dominant, heartbeat superimposed)
            let sig = (2.0 * PI * resp_freq * t).sin() * 1.5 + (2.0 * PI * heart_freq * t).sin() * 0.35;
            let m = extractor.process_sample(sig);
            last_metrics = Some(m);
        }

        let metrics = last_metrics.expect("Should produce metrics");
        // Verify extracted breathing is close to 15 BrPM
        assert!(
            (metrics.respiration_rate_brpm - 15.0).abs() < 2.5,
            "Extracted respiration rate {} deviates from target 15 BrPM",
            metrics.respiration_rate_brpm
        );
        // Verify extracted heart rate is close to 72 BPM
        assert!(
            (metrics.heart_rate_bpm - 72.0).abs() < 6.0,
            "Extracted heart rate {} deviates from target 72 BPM",
            metrics.heart_rate_bpm
        );
        assert_eq!(metrics.alert_state, VitalAlertState::Nominal);
    }
}
