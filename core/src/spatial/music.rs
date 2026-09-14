//! VORAX 2D-MUSIC (Multiple Signal Classification) Spatial Spectrum Engine
//!
//! Evaluates incoming RF multipath signals along two physical dimensions:
//! 1. Angle of Arrival (AoA, θ): Incident angle of reflection relative to the antenna array
//! 2. Time of Flight (ToF, τ): Propagation delay in nanoseconds
//!
//! Uses projection onto the orthogonal noise subspace:
//! P_MUSIC(θ, τ) = 1 / (||a(θ, τ)||^2 - sum_{d=1}^D |v_d^H a(θ, τ)|^2)
//! Where v_d are the dominant signal subspace eigenvectors.

use std::f32::consts::PI;
use super::smoothing::{Complex32, ComplexMatrix};

pub const SPEED_OF_LIGHT_M_S: f32 = 299_792_458.0;
pub const CARRIER_FREQ_HZ: f32 = 2_412_000_000.0;
pub const SUBCARRIER_SPACING_HZ: f32 = 312_500.0;
pub const ANTENNA_SPACING_METERS: f32 = 0.062; // ~λ/2 for 2.4 GHz

#[derive(Debug, Clone)]
pub struct SpatialPeak {
    /// Estimated Angle of Arrival in degrees [-90.0, 90.0]
    pub aoa_degrees: f32,
    /// Estimated Time of Flight in nanoseconds
    pub tof_nanoseconds: f32,
    /// Estimated physical propagation distance in meters: c * tof
    pub range_meters: f32,
    /// Pseudo-spectrum peak power (confidence metric)
    pub pseudo_spectrum_val: f32,
}

#[derive(Debug, Clone)]
pub struct MusicEngine2D {
    subcarrier_window: usize,
    num_signals: usize, // Number of dominant paths to separate (e.g. 2: LoS + 1 Target)
    aoa_grid_deg: Vec<f32>,
    tof_grid_ns: Vec<f32>,
}

impl MusicEngine2D {
    /// Initializes the 2D-MUSIC search grid
    /// `subcarrier_window`: Dimension of smoothed covariance matrix (e.g. 16)
    /// `num_signals`: Rank of signal subspace (typically 2 for direct + human path)
    pub fn new(subcarrier_window: usize, num_signals: usize) -> Self {
        // AoA grid from -80 to +80 degrees in 5-degree increments
        let mut aoa_grid_deg = Vec::new();
        let mut angle = -80.0f32;
        while angle <= 80.0 {
            aoa_grid_deg.push(angle);
            angle += 5.0;
        }

        // ToF grid from 0 to 40 nanoseconds in 2 ns increments
        let mut tof_grid_ns = Vec::new();
        let mut delay = 0.0f32;
        while delay <= 40.0 {
            tof_grid_ns.push(delay);
            delay += 2.0;
        }

        Self {
            subcarrier_window,
            num_signals,
            aoa_grid_deg,
            tof_grid_ns,
        }
    }

    /// Evaluates the 2D-MUSIC pseudo-spectrum across (AoA, ToF) space and extracts the human reflection peak
    pub fn estimate_peak(&self, cov: &ComplexMatrix) -> SpatialPeak {
        assert_eq!(cov.dim, self.subcarrier_window);

        // 1. Extract dominant signal subspace eigenvectors via Power Iteration with Hotelling deflation
        let signal_eigenvectors = self.extract_signal_eigenvectors(cov, self.num_signals);

        // Track the MINIMUM noise energy (= MUSIC pseudo-spectrum peak).
        // We search for minimum noise_energy directly rather than maximum 1/noise_energy —
        // the inversion of near-zero floats causes all "good" grid points to tie at the clamp
        // floor (1e-4), making the search degenerate. Direct minimization is numerically robust.
        let mut min_noise_energy = f32::MAX;
        let mut best_aoa = 0.0f32;
        let mut best_tof = 0.0f32;

        // 2. Grid search across AoA and ToF
        for &aoa_deg in &self.aoa_grid_deg {
            let aoa_rad = aoa_deg * PI / 180.0;

            for &tof_ns in &self.tof_grid_ns {
                let tof_sec = tof_ns * 1e-9;

                // Construct steering vector a(θ, τ)
                let steering = self.compute_steering_vector(aoa_rad, tof_sec);

                // Compute signal subspace projection energy: sum |v_d^H a|^2
                let norm_a_sq: f32 = steering.len() as f32; // Unit-magnitude elements

                let mut signal_energy = 0.0f32;
                for v_d in &signal_eigenvectors {
                    let mut dot = Complex32::ZERO;
                    for i in 0..self.subcarrier_window {
                        // v_d^H * a = sum conj(v_d[i]) * a[i]
                        dot = dot.add(v_d[i].conj().mul(steering[i]));
                    }
                    signal_energy += dot.norm_sq();
                }

                // Clamp to [0, norm_a_sq] to prevent negative noise energy from float32 error
                let signal_energy_clamped = signal_energy.clamp(0.0, norm_a_sq);
                // Noise energy = residual orthogonal to signal subspace (MUSIC denominator)
                let noise_energy = norm_a_sq - signal_energy_clamped;

                if noise_energy < min_noise_energy {
                    min_noise_energy = noise_energy;
                    best_aoa = aoa_deg;
                    best_tof = tof_ns;
                }
            }
        }
        // Pseudo-spectrum peak value (for callers that want the confidence metric)
        let max_spectrum = if min_noise_energy > 1e-6 { 1.0 / min_noise_energy } else { f32::MAX };

        let range_meters = SPEED_OF_LIGHT_M_S * (best_tof * 1e-9);

        SpatialPeak {
            aoa_degrees: best_aoa,
            tof_nanoseconds: best_tof,
            range_meters,
            pseudo_spectrum_val: max_spectrum,
        }
    }

    /// Computes candidate steering vector a(θ, τ) across subcarrier indices
    fn compute_steering_vector(&self, aoa_rad: f32, tof_sec: f32) -> Vec<Complex32> {
        let mut steering = Vec::with_capacity(self.subcarrier_window);

        for k in 0..self.subcarrier_window {
            let f_k = CARRIER_FREQ_HZ + (k as f32) * SUBCARRIER_SPACING_HZ;
            let lambda_k = SPEED_OF_LIGHT_M_S / f_k;

            // Phase accumulation due to ToF delay + AoA spatial phase offset
            let phase_delay = -2.0 * PI * f_k * tof_sec;
            let phase_aoa = -2.0 * PI * (ANTENNA_SPACING_METERS * aoa_rad.sin()) / lambda_k;

            let total_phase = phase_delay + phase_aoa;
            steering.push(Complex32::from_polar(1.0, total_phase));
        }

        steering
    }

    /// Extracts the D dominant eigenvectors using Power Iteration with Hotelling deflation
    fn extract_signal_eigenvectors(
        &self,
        matrix: &ComplexMatrix,
        d: usize,
    ) -> Vec<Vec<Complex32>> {
        let mut deflated = matrix.clone();
        let mut eigenvectors = Vec::with_capacity(d);

        for _ in 0..d {
            let mut v = vec![Complex32::new(1.0, 0.0); matrix.dim];
            // Normalize initial vector
            let inv_norm = 1.0 / (v.len() as f32).sqrt();
            for val in &mut v {
                *val = val.scale(inv_norm);
            }

            // Power iteration loop (15 iterations is typically sufficient for convergence)
            for _ in 0..15 {
                let mut v_next = vec![Complex32::ZERO; matrix.dim];
                for r in 0..matrix.dim {
                    let mut sum = Complex32::ZERO;
                    for c in 0..matrix.dim {
                        sum = sum.add(deflated.get(r, c).mul(v[c]));
                    }
                    v_next[r] = sum;
                }

                // Compute norm of v_next
                let mut norm_sq = 0.0f32;
                for val in &v_next {
                    norm_sq += val.norm_sq();
                }
                let norm = norm_sq.sqrt().max(1e-6);

                for i in 0..matrix.dim {
                    v[i] = v_next[i].scale(1.0 / norm);
                }
            }

            // Compute corresponding eigenvalue: λ = v^H * A * v
            let mut lambda = Complex32::ZERO;
            for r in 0..matrix.dim {
                let mut av_r = Complex32::ZERO;
                for c in 0..matrix.dim {
                    av_r = av_r.add(deflated.get(r, c).mul(v[c]));
                }
                lambda = lambda.add(v[r].conj().mul(av_r));
            }

            // Deflate matrix: A_{k+1} = A_k - λ * v * v^H
            let lambda_re = lambda.re.max(0.0);
            for r in 0..matrix.dim {
                for c in 0..matrix.dim {
                    let sub = v[r].mul(v[c].conj()).scale(lambda_re);
                    let current = deflated.get(r, c);
                    deflated.set(r, c, current.sub(sub));
                }
            }

            eigenvectors.push(v);
        }

        eigenvectors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::smoothing::compute_smoothed_covariance;

    #[test]
    fn test_music_finds_injected_delay_and_angle() {
        let window_size = 16;
        // D=1: we inject one known target. The algorithm subtracts signal subspace and
        // minimises noise energy — minimum at the true (AoA, ToF).
        let music = MusicEngine2D::new(window_size, 1);

        // Target: AoA = 20 degrees, ToF = 14 ns (range ~ 4.2 m)
        let target_aoa_deg = 20.0f32;
        let target_aoa_rad = target_aoa_deg * PI / 180.0;
        let target_tof_sec = 14.0e-9f32;

        // Build the target steering vector a(θ, τ) for the first `window_size` subcarriers.
        // This matches exactly what compute_steering_vector produces.
        let target_steering: Vec<Complex32> = (0..window_size)
            .map(|k| {
                let f_k = CARRIER_FREQ_HZ + (k as f32) * SUBCARRIER_SPACING_HZ;
                let lambda_k = SPEED_OF_LIGHT_M_S / f_k;
                let phase = -2.0 * PI * f_k * target_tof_sec
                    - 2.0 * PI * (ANTENNA_SPACING_METERS * target_aoa_rad.sin()) / lambda_k;
                Complex32::from_polar(1.0, phase)
            })
            .collect();

        // Construct covariance directly: R = SNR * a * a^H + I
        // This gives a perfectly conditioned rank-1 signal in a white noise floor.
        // Power iteration will extract `target_steering` (up to phase) as dominant eigenvector.
        let snr = 100.0f32; // 20 dB
        let mut cov = ComplexMatrix::zeros(window_size);
        for i in 0..window_size {
            for j in 0..window_size {
                let signal_term = target_steering[i].mul(target_steering[j].conj()).scale(snr);
                // Identity noise floor: adds 1.0 on diagonal only
                let noise_term = if i == j { Complex32::new(1.0, 0.0) } else { Complex32::ZERO };
                cov.set(i, j, signal_term.add(noise_term));
            }
        }

        let peak = music.estimate_peak(&cov);

        assert!(
            (peak.aoa_degrees - target_aoa_deg).abs() <= 10.0,
            "AoA {} deg should be within 10 deg of target {} deg (grid resolution: 5 deg)",
            peak.aoa_degrees, target_aoa_deg
        );
        assert!(
            (peak.tof_nanoseconds - 14.0).abs() <= 4.0,
            "ToF {} ns should be within 4 ns of target 14 ns (grid resolution: 2 ns)",
            peak.tof_nanoseconds
        );
        assert!(
            (peak.range_meters - 4.2).abs() <= 1.5,
            "Range {} m should be within 1.5 m of target ~4.2 m",
            peak.range_meters
        );
    }
}
