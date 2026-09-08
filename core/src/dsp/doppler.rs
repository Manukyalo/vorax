//! VORAX Short-Time Fourier Transform (STFT) Micro-Doppler Engine
//!
//! Extracts human body velocity profiles from OFDM subcarrier time series:
//! 1. Windowed sliding buffer (Hann window)
//! 2. Fast Fourier Transform into discrete frequency bins
//! 3. Conversion from Doppler frequency shift to physical target velocity:
//!    v = (c * f_d) / (2 * f_c)
//! 4. Total Doppler kinetic energy calculation for human movement classification

use std::f32::consts::PI;

/// Speed of light in meters per second
pub const SPEED_OF_LIGHT_M_S: f32 = 299_792_458.0;

/// Default Wi-Fi 2.4 GHz channel 1 center frequency in Hz
pub const DEFAULT_CARRIER_FREQ_HZ: f32 = 2_412_000_000.0;

/// Minimum Doppler energy threshold to consider physical human movement present
pub const MOTION_ENERGY_THRESHOLD: f32 = 0.85;

#[derive(Debug, Clone)]
pub struct DopplerProfile {
    /// Velocity corresponding to each spectrum bin in m/s
    pub velocities: Vec<f32>,
    /// Magnitude of each Doppler velocity bin
    pub magnitudes: Vec<f32>,
    /// Peak observed velocity in m/s
    pub peak_velocity_m_s: f32,
    /// Total integrated Doppler kinetic energy
    pub total_energy: f32,
    /// Dominant Doppler frequency shift in Hz
    pub dominant_freq_hz: f32,
}

#[derive(Debug, Clone)]
pub struct StftProcessor {
    window_size: usize,
    sample_rate: f32,
    carrier_freq: f32,
    buffer: Vec<f32>,
    hann_window: Vec<f32>,
}

impl StftProcessor {
    /// Initializes an STFT processor with power-of-two window size (e.g. 64 or 128)
    /// and sampling frequency (typically 200 Hz).
    pub fn new(window_size: usize, sample_rate: f32, carrier_freq: f32) -> Self {
        assert!(window_size.is_power_of_two(), "Window size must be a power of two");
        assert!(sample_rate > 0.0, "Sample rate must be positive");

        // Precompute Hann window weights: w[n] = 0.5 * (1 - cos(2*pi*n / (N - 1)))
        let hann_window: Vec<f32> = (0..window_size)
            .map(|n| 0.5 * (1.0 - (2.0 * PI * n as f32 / (window_size - 1) as f32).cos()))
            .collect();

        Self {
            window_size,
            sample_rate,
            carrier_freq,
            buffer: Vec::with_capacity(window_size),
            hann_window,
        }
    }

    /// Pushes a single time-series sample (e.g. selected subcarrier amplitude or phase delta)
    pub fn push_sample(&mut self, sample: f32) {
        if self.buffer.len() == self.window_size {
            self.buffer.remove(0);
        }
        self.buffer.push(sample);
    }

    /// Returns true when the sliding window has collected enough samples for STFT analysis
    pub fn is_ready(&self) -> bool {
        self.buffer.len() == self.window_size
    }

    /// Computes the Doppler velocity spectrum across the current window
    pub fn compute_spectrum(&self) -> Option<DopplerProfile> {
        if !self.is_ready() {
            return None;
        }

        // Apply Hann window and remove DC bias
        let mean: f32 = self.buffer.iter().sum::<f32>() / self.window_size as f32;
        let windowed: Vec<f32> = self
            .buffer
            .iter()
            .zip(self.hann_window.iter())
            .map(|(&x, &w)| (x - mean) * w)
            .collect();

        // Radix-2 Cooley-Tukey In-Place FFT
        let (re, im) = fft_radix2(&windowed);

        let n = self.window_size;
        let half = n / 2;
        let mut magnitudes = Vec::with_capacity(half);
        let mut velocities = Vec::with_capacity(half);

        let freq_resolution = self.sample_rate / n as f32;
        let mut total_energy = 0.0f32;
        let mut peak_mag = 0.0f32;
        let mut dominant_freq_hz = 0.0f32;

        // Positive Doppler velocities (approaching / moving targets)
        // Bin 0 is DC (0 Hz), bins 1..half are positive frequencies
        for k in 1..half {
            let mag = (re[k].powi(2) + im[k].powi(2)).sqrt();
            magnitudes.push(mag);

            let freq = k as f32 * freq_resolution;
            // v = (c * f_d) / (2 * f_c)
            let velocity = (SPEED_OF_LIGHT_M_S * freq) / (2.0 * self.carrier_freq);
            velocities.push(velocity);

            total_energy += mag * mag;
            if mag > peak_mag {
                peak_mag = mag;
                dominant_freq_hz = freq;
            }
        }

        let peak_velocity_m_s = (SPEED_OF_LIGHT_M_S * dominant_freq_hz) / (2.0 * self.carrier_freq);

        Some(DopplerProfile {
            velocities,
            magnitudes,
            peak_velocity_m_s,
            total_energy,
            dominant_freq_hz,
        })
    }
}

/// Radix-2 Cooley-Tukey in-place Fast Fourier Transform for real input
fn fft_radix2(input: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let n = input.len();
    assert!(n.is_power_of_two(), "FFT length must be power of two");

    let mut re = input.to_vec();
    let mut im = vec![0.0f32; n];

    // Bit-reversal permutation
    let mut j = 0;
    for i in 0..(n - 1) {
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
        let mut k = n / 2;
        while k <= j {
            j -= k;
            k /= 2;
        }
        j += k;
    }

    // Cooley-Tukey butterflies
    let mut len = 2;
    while len <= n {
        let half_len = len / 2;
        let angle_step = -2.0 * PI / len as f32;

        for i in (0..n).step_by(len) {
            for k in 0..half_len {
                let angle = k as f32 * angle_step;
                let (cos_a, sin_a) = (angle.cos(), angle.sin());

                let u_re = re[i + k];
                let u_im = im[i + k];

                let v_re = re[i + k + half_len] * cos_a - im[i + k + half_len] * sin_a;
                let v_im = re[i + k + half_len] * sin_a + im[i + k + half_len] * cos_a;

                re[i + k] = u_re + v_re;
                im[i + k] = u_im + v_im;

                re[i + k + half_len] = u_re - v_re;
                im[i + k + half_len] = u_im - v_im;
            }
        }
        len *= 2;
    }

    (re, im)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stft_detects_known_velocity() {
        let sample_rate = 200.0;
        let window_size = 64;
        let carrier_freq = DEFAULT_CARRIER_FREQ_HZ;
        let mut stft = StftProcessor::new(window_size, sample_rate, carrier_freq);

        // Inject 16 Hz Doppler shift (corresponding to walking target at ~1 m/s)
        // v = (3e8 * 16) / (2 * 2.412e9) = 4.8e9 / 4.824e9 = ~0.995 m/s
        let target_freq = 15.625; // Exactly bin 5 at 200 Hz / 64 bins (3.125 Hz per bin)
        for i in 0..window_size {
            let t = i as f32 / sample_rate;
            let sample = (2.0 * PI * target_freq * t).sin();
            stft.push_sample(sample);
        }

        let profile = stft.compute_spectrum().expect("STFT profile should be ready");
        assert!(profile.total_energy > MOTION_ENERGY_THRESHOLD, "Energy should exceed threshold");
        assert!(
            (profile.dominant_freq_hz - target_freq).abs() < 2.0,
            "Detected frequency {} should match target {}",
            profile.dominant_freq_hz,
            target_freq
        );
        assert!(
            (profile.peak_velocity_m_s - 0.97).abs() < 0.15,
            "Velocity {} should match ~0.97 m/s",
            profile.peak_velocity_m_s
        );
    }
}
