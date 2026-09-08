//! VORAX Hampel Identifier Filter
//!
//! Rejects sudden burst outlier spikes in WiFi CSI subcarrier time-series
//! caused by RF collisions, packet retransmissions, or multipath flashes.
//!
//! Uses rolling Median Absolute Deviation (MAD):
//! If |x_i - median| > threshold * 1.4826 * MAD, replace x_i with median.

#[derive(Debug, Clone)]
pub struct HampelFilter {
    window_size: usize,
    threshold: f32,
    buffer: Vec<f32>,
}

impl HampelFilter {
    /// Creates a Hampel filter with rolling window size (e.g. 7 or 11 samples)
    /// and MAD threshold multiplier (standard: 3.0).
    pub fn new(window_size: usize, threshold: f32) -> Self {
        assert!(window_size % 2 == 1, "Window size must be an odd integer");
        Self {
            window_size,
            threshold,
            buffer: Vec::with_capacity(window_size),
        }
    }

    /// Feeds a new sample and returns the sanitized (outlier-filtered) sample.
    pub fn filter(&mut self, sample: f32) -> f32 {
        self.buffer.push(sample);
        if self.buffer.len() > self.window_size {
            self.buffer.remove(0);
        }

        if self.buffer.len() < self.window_size {
            return sample;
        }

        let median = calculate_median(&self.buffer);
        let mad = calculate_mad(&self.buffer, median);

        let diff = (sample - median).abs();
        let limit = self.threshold * 1.4826 * mad;

        if diff > limit && limit > 1e-4 {
            median // Replace outlier spike with rolling median
        } else {
            sample
        }
    }

    /// Filters an entire contiguous time series in-place.
    pub fn filter_series(data: &mut [f32], half_window: usize, threshold: f32) {
        let n = data.len();
        if n <= 2 * half_window {
            return;
        }

        let original = data.to_vec();
        for i in half_window..(n - half_window) {
            let window = &original[(i - half_window)..=(i + half_window)];
            let median = calculate_median(window);
            let mad = calculate_mad(window, median);

            let diff = (original[i] - median).abs();
            let limit = threshold * 1.4826 * mad;

            if diff > limit.max(1e-4) {
                data[i] = median;
            }
        }
    }
}

fn calculate_median(slice: &[f32]) -> f32 {
    let mut sorted = slice.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) * 0.5
    }
}

fn calculate_mad(slice: &[f32], median: f32) -> f32 {
    let devs: Vec<f32> = slice.iter().map(|&x| (x - median).abs()).collect();
    calculate_median(&devs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hampel_eliminates_spikes() {
        let mut data = vec![10.0; 25];
        // Inject extreme outlier spikes
        data[8] = 95.0;
        data[16] = -80.0;

        HampelFilter::filter_series(&mut data, 3, 3.0);

        assert!((data[8] - 10.0).abs() < 1e-3, "Spike at 8 was not removed, got {}", data[8]);
        assert!((data[16] - 10.0).abs() < 1e-3, "Spike at 16 was not removed, got {}", data[16]);
    }
}
