//! VORAX Spatial-Frequency Smoothing Engine
//!
//! Solves the coherent multipath problem in indoor RF environments.
//! In closed spaces, multipath reflections are phase-coherent with the direct path,
//! causing the rank of the raw sample covariance matrix to collapse to 1.
//!
//! Forward-Backward Spatial-Frequency Smoothing restores the rank of the covariance
//! matrix, enabling 2D-MUSIC to separate multiple coherent RF arrival paths.

use std::f32::consts::PI;

/// A single complex number with standard algebraic operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex32 {
    pub re: f32,
    pub im: f32,
}

impl Complex32 {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };

    #[inline(always)]
    pub fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    #[inline(always)]
    pub fn from_polar(r: f32, theta: f32) -> Self {
        Self {
            re: r * theta.cos(),
            im: r * theta.sin(),
        }
    }

    #[inline(always)]
    pub fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    #[inline(always)]
    pub fn norm_sq(self) -> f32 {
        self.re * self.re + self.im * self.im
    }

    #[inline(always)]
    pub fn norm(self) -> f32 {
        self.norm_sq().sqrt()
    }

    #[inline(always)]
    pub fn add(self, rhs: Self) -> Self {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }

    #[inline(always)]
    pub fn sub(self, rhs: Self) -> Self {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }

    #[inline(always)]
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }

    #[inline(always)]
    pub fn scale(self, scalar: f32) -> Self {
        Self {
            re: self.re * scalar,
            im: self.im * scalar,
        }
    }
}

/// Square Complex Matrix for Covariance Operations
#[derive(Debug, Clone)]
pub struct ComplexMatrix {
    pub dim: usize,
    pub data: Vec<Complex32>,
}

impl ComplexMatrix {
    pub fn zeros(dim: usize) -> Self {
        Self {
            dim,
            data: vec![Complex32::ZERO; dim * dim],
        }
    }

    #[inline(always)]
    pub fn get(&self, row: usize, col: usize) -> Complex32 {
        self.data[row * self.dim + col]
    }

    #[inline(always)]
    pub fn set(&mut self, row: usize, col: usize, val: Complex32) {
        self.data[row * self.dim + col] = val;
    }

    #[inline(always)]
    pub fn add_assign_at(&mut self, row: usize, col: usize, val: Complex32) {
        let idx = row * self.dim + col;
        self.data[idx] = self.data[idx].add(val);
    }

    /// Computes the outer product x * x^H and accumulates into self
    pub fn accumulate_outer_product(&mut self, vec: &[Complex32]) {
        assert_eq!(vec.len(), self.dim);
        for i in 0..self.dim {
            let vi = vec[i];
            for j in 0..self.dim {
                let vj_conj = vec[j].conj();
                self.add_assign_at(i, j, vi.mul(vj_conj));
            }
        }
    }

    /// Multiplies all matrix elements by a real scalar
    pub fn scale(&mut self, scalar: f32) {
        for val in &mut self.data {
            *val = val.scale(scalar);
        }
    }
}

/// Computes Forward-Backward Frequency Smoothing Covariance Matrix
///
/// `csi_subcarriers`: Array of 64 subcarrier complex channel measurements.
/// `sub_window_size`: Size of smoothed sub-array window L (e.g. 16 or 32).
pub fn compute_smoothed_covariance(
    csi_subcarriers: &[Complex32],
    sub_window_size: usize,
) -> ComplexMatrix {
    let n = csi_subcarriers.len();
    assert!(n >= sub_window_size, "Total subcarriers must be >= sub-window size");

    let num_subarrays = n - sub_window_size + 1;
    let mut r_smooth = ComplexMatrix::zeros(sub_window_size);

    // 1. Forward Smoothing
    for m in 0..num_subarrays {
        let sub_vec = &csi_subcarriers[m..(m + sub_window_size)];
        r_smooth.accumulate_outer_product(sub_vec);
    }

    // 2. Backward Smoothing (conjugate reversal)
    for m in 0..num_subarrays {
        let mut backward_vec = Vec::with_capacity(sub_window_size);
        for i in (0..sub_window_size).rev() {
            backward_vec.push(csi_subcarriers[m + i].conj());
        }
        r_smooth.accumulate_outer_product(&backward_vec);
    }

    // Normalize by total forward + backward partitions: 2 * num_subarrays
    let normalizer = 1.0 / (2.0 * num_subarrays as f32);
    r_smooth.scale(normalizer);

    r_smooth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_algebra() {
        let a = Complex32::new(3.0, 4.0);
        assert_eq!(a.norm_sq(), 25.0);
        assert_eq!(a.norm(), 5.0);

        let b = Complex32::new(1.0, -2.0);
        let prod = a.mul(b);
        assert_eq!(prod, Complex32::new(11.0, -2.0));
    }

    #[test]
    fn test_smoothed_covariance_hermitian() {
        // Construct a synthetic 64-subcarrier series
        let csi: Vec<Complex32> = (0..64)
            .map(|k| Complex32::from_polar(20.0, k as f32 * 0.15))
            .collect();

        let l = 16;
        let r_smooth = compute_smoothed_covariance(&csi, l);

        // Covariance matrices must be Hermitian: R[i, j] == conj(R[j, i])
        for i in 0..l {
            for j in 0..l {
                let val_ij = r_smooth.get(i, j);
                let val_ji = r_smooth.get(j, i);
                let diff = (val_ij.re - val_ji.re).abs() + (val_ij.im + val_ji.im).abs();
                assert!(diff < 1e-4, "Matrix not Hermitian at ({}, {})", i, j);
            }
        }
    }
}
