//! VORAX Core — Phase Unwrapping & Amplitude Extraction
//!
//! Basic (scalar) implementation.
//! SIMD/AVX2 optimisation is Phase 1 scope — correctness first.

/// Unwrap a sequence of phase values (radians) in-place.
///
/// Standard phase unwrapping: if consecutive difference exceeds π,
/// shift by ±2π to minimise discontinuities.
pub fn unwrap_phase(phases: &mut [f32]) {
    if phases.len() < 2 {
        return;
    }
    use std::f32::consts::PI;
    let two_pi = 2.0 * PI;

    for i in 1..phases.len() {
        let mut delta = phases[i] - phases[i - 1];
        // Wrap delta into (-π, π]
        while delta > PI  { delta -= two_pi; }
        while delta < -PI { delta += two_pi; }
        phases[i] = phases[i - 1] + delta;
    }
}

/// Remove linear phase trend (caused by sampling time offset & carrier frequency offset).
///
/// Fits a least-squares line through the unwrapped phase series and subtracts it,
/// leaving only the environmental multipath modulation we care about.
pub fn remove_linear_trend(phases: &mut [f32]) {
    let n = phases.len();
    if n < 2 {
        return;
    }

    let n_f = n as f32;
    let x_mean = (n_f - 1.0) / 2.0;
    let y_mean: f32 = phases.iter().sum::<f32>() / n_f;

    let mut num = 0.0f32;
    let mut den = 0.0f32;
    for (i, &y) in phases.iter().enumerate() {
        let x = i as f32 - x_mean;
        num += x * (y - y_mean);
        den += x * x;
    }

    if den.abs() < f32::EPSILON {
        return;
    }

    let slope = num / den;
    let intercept = y_mean - slope * x_mean;

    for (i, p) in phases.iter_mut().enumerate() {
        *p -= slope * i as f32 + intercept;
    }
}

/// Compute amplitude for a CSI frame's subcarrier sequence.
/// Input: interleaved [I, Q, I, Q, ...] as i8 slices.
/// Output: amplitude per subcarrier as f32 vec.
pub fn compute_amplitudes(iq_bytes: &[i8]) -> Vec<f32> {
    iq_bytes
        .chunks_exact(2)
        .map(|chunk| {
            let q = chunk[0] as f32; // imaginary first (firmware convention)
            let i = chunk[1] as f32;
            (i * i + q * q).sqrt()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn unwrap_handles_pi_jump() {
        // A phase sequence that jumps past π should be unwrapped
        let mut phases = vec![0.0f32, 0.5 * PI, PI - 0.1, -PI + 0.1];
        unwrap_phase(&mut phases);
        // After unwrapping, sequence should be monotonically increasing
        for w in phases.windows(2) {
            assert!(
                (w[1] - w[0]).abs() < PI,
                "unwrapped jump too large: {} -> {}",
                w[0], w[1]
            );
        }
    }

    #[test]
    fn amplitude_3_4_5_triangle() {
        // I=3, Q=4 → amplitude should be 5.0
        let iq: &[i8] = &[4, 3]; // imaginary=4, real=3
        let amps = compute_amplitudes(iq);
        assert!((amps[0] - 5.0).abs() < 0.001, "expected 5.0, got {}", amps[0]);
    }

    #[test]
    fn linear_trend_removal_flat_residual() {
        // A perfectly linear phase series should flatten to near-zero after detrending
        let mut phases: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
        remove_linear_trend(&mut phases);
        let max_residual = phases.iter().map(|p| p.abs()).fold(0.0f32, f32::max);
        assert!(max_residual < 0.001, "residual too large: {}", max_residual);
    }
}
