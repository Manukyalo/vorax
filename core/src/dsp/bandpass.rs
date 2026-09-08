//! VORAX Bandpass Biquad Filter Engine
//!
//! Isolates human physiological micro-Doppler signals from subcarrier amplitude time-series.
//!
//! Implements a 2nd-order IIR Biquad Filter (Direct Form II Transposed) for:
//! 1. Respiration Band: 0.1 Hz – 0.5 Hz (6 – 30 breaths/min)
//! 2. Cardiac Band: 0.8 Hz – 2.5 Hz (48 – 150 BPM)

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct BiquadCoeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

#[derive(Debug, Clone)]
pub struct BiquadFilter {
    coeffs: BiquadCoeffs,
    s1: f32,
    s2: f32,
}

impl BiquadFilter {
    pub fn new(coeffs: BiquadCoeffs) -> Self {
        Self { coeffs, s1: 0.0, s2: 0.0 }
    }

    /// Generates digital bandpass coefficients for a specified sample rate (e.g. 200 Hz).
    /// `center_freq` is (f_low + f_high) / 2
    /// `q` is center_freq / bandwidth
    pub fn bandpass(sample_rate: f32, center_freq: f32, bandwidth: f32) -> Self {
        let w0 = 2.0 * PI * center_freq / sample_rate;
        let q = center_freq / bandwidth.max(0.01);
        let alpha = w0.sin() / (2.0 * q);

        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * w0.cos();
        let a2 = 1.0 - alpha;

        let coeffs = BiquadCoeffs {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        };

        Self::new(coeffs)
    }

    /// Convenience constructor for Respiration Band (0.1 Hz – 0.5 Hz @ sample_rate)
    pub fn respiration(sample_rate: f32) -> Self {
        Self::bandpass(sample_rate, 0.3, 0.4)
    }

    /// Convenience constructor for Cardiac Heartbeat Band (0.8 Hz – 2.5 Hz @ sample_rate)
    pub fn cardiac(sample_rate: f32) -> Self {
        Self::bandpass(sample_rate, 1.65, 1.7)
    }

    /// Processes a single real-time sample through the Direct Form II Transposed topology.
    #[inline(always)]
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.coeffs.b0 * input + self.s1;
        self.s1 = self.coeffs.b1 * input - self.coeffs.a1 * output + self.s2;
        self.s2 = self.coeffs.b2 * input - self.coeffs.a2 * output;
        output
    }

    /// Resets filter state history
    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardiac_filter_passes_1_2hz_and_attenuates_high_freq() {
        let sample_rate = 200.0;
        let mut filter = BiquadFilter::cardiac(sample_rate);

        // Warm up filter
        for _ in 0..100 {
            filter.process(0.0);
        }

        // Test passing 1.2 Hz (72 BPM cardiac signal)
        let mut max_in_band = 0.0f32;
        for i in 0..200 {
            let t = i as f32 / sample_rate;
            let sig = (2.0 * PI * 1.2 * t).sin();
            let out = filter.process(sig);
            if i > 100 {
                max_in_band = max_in_band.max(out.abs());
            }
        }

        // Test blocking 30.0 Hz (high-frequency motor / interference noise)
        let mut filter_hf = BiquadFilter::cardiac(sample_rate);
        let mut max_out_of_band = 0.0f32;
        for i in 0..200 {
            let t = i as f32 / sample_rate;
            let sig = (2.0 * PI * 30.0 * t).sin();
            let out = filter_hf.process(sig);
            if i > 100 {
                max_out_of_band = max_out_of_band.max(out.abs());
            }
        }

        assert!(max_in_band > 0.05, "In-band signal was attenuated too strongly: {}", max_in_band);
        assert!(max_out_of_band < max_in_band * 0.1, "High frequency was not rejected: {}", max_out_of_band);
    }
}
