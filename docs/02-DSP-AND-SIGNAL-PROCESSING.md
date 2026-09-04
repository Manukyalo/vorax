# 02 — DSP & Signal Processing Deep Dive

## 1. Physics of Wi-Fi Channel State Information (CSI)

In orthogonal frequency-division multiplexing (OFDM), Wi-Fi divides the radio channel into multiple subcarriers (e.g., 64 subcarriers across a $20\text{ MHz}$ channel). 

For subcarrier $k$, the transmitted signal $X(f_k)$ and received signal $Y(f_k)$ are related by the channel frequency response $H(f_k)$:

$$Y(f_k) = H(f_k) \cdot X(f_k) + N(f_k)$$

Where the CSI value for subcarrier $k$ is represented as a complex number:

$$H(f_k) = |H(f_k)| e^{j \angle H(f_k)} = I_k + j Q_k$$

- **Amplitude $|H(f_k)| = \sqrt{I_k^2 + Q_k^2}$**: Measures signal attenuation caused by obstacles and absorption.
- **Phase $\angle H(f_k) = \arctan2(Q_k, I_k)$**: Measures wave cycle delay caused by distance traveled and multipath reflection.

---

## 2. Noise Sources in Commodity Wi-Fi Chips

Raw phase measurements $\hat{\phi}_k$ from hardware like the ESP32 contain severe non-target distortions:

$$\hat{\phi}_k = \phi_k - 2\pi \frac{k}{N} \cdot \delta_{PDD} - 2\pi \cdot \Delta f_{CFO} \cdot t + \beta_{SFO} + Z_{noise}$$

Where:
1. **$\delta_{PDD}$ (Packet Detection Delay):** The hardware ADC triggers at slightly different microsecond offsets for each packet, creating a random linear phase slope across subcarriers.
2. **$\Delta f_{CFO}$ (Carrier Frequency Offset):** Small frequency mismatches between the transmitter and receiver local oscillators cause rapid, unpredictable phase rotations over time.
3. **$\beta_{SFO}$ (Sampling Frequency Offset):** Clock drift between DAC and ADC sampling rates.

---

## 3. Production Phase Sanitization Pipeline

```
 [Raw CSI (I, Q)]
        │
        ▼
 [Amplitude & Phase Calculation] ──▶ [Phase Unwrapping (Modulo 2π)]
                                             │
                                             ▼
 [Conjugate Antenna Multiplication] ◀────────┘
        │ (Cancels CFO and PDD shared across hardware channels)
        ▼
 [Linear Fitting & Slope Removal]
        │
        ▼
 [Hampel Outlier Filtering (Sliding Window σ)]
        │
        ▼
 [Sanitized Phase Tensor]
```

### 3.1. Conjugate Antenna Multiplication
When two antennas are connected to the same receiver RF frontend, they share the identical $\delta_{PDD}$ and $\Delta f_{CFO}$. By multiplying the complex CSI of Antenna 1 with the complex conjugate of Antenna 2:

$$H_{sanitized}(f_k) = H_1(f_k) \cdot H_2^*(f_k)$$

The common phase noise terms cancel out, leaving the pure **relative phase difference** ($\Delta \phi_k$), which directly encodes the physical Angle of Arrival (AoA) of the incoming waves.

### 3.2. Hampel Outlier Filter
To remove non-human burst noise (Bluetooth coexistence spikes, microwave radiation):
```rust
pub fn hampel_filter(data: &mut [f32], window_size: usize, n_sigmas: f32) {
    let half_w = window_size / 2;
    let len = data.len();
    for i in half_w..(len - half_w) {
        let window = &data[(i - half_w)..=(i + half_w)];
        let mut sorted = window.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let median = sorted[half_w];
        let mut diffs: Vec<f32> = window.iter().map(|&x| (x - median).abs()).collect();
        diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mad = 1.4826 * diffs[half_w]; // Median Absolute Deviation

        if (data[i] - median).abs() > n_sigmas * mad {
            data[i] = median; // Replace outlier with local median
        }
    }
}
```

---

## 4. Angle of Arrival (AoA) & Time of Flight (ToF): 2D-MUSIC

To localize objects in 3D space, we compute the multi-path parameters using the 2D-MUSIC (Multiple Signal Classification) algorithm:

1. **Construct Spatial-Frequency Smoothing Matrix:** Group smoothed subcarrier covariance matrices across antennas.
2. **Eigenvalue Decomposition:** Separate the signal subspace from the noise subspace $E_n$.
3. **Pseudo-Spectrum Calculation:**

$$P_{MUSIC}(\theta, \tau) = \frac{1}{a(\theta, \tau)^H E_n E_n^H a(\theta, \tau)}$$

Where $a(\theta, \tau)$ is the steering vector representing an RF path arriving at angle $\theta$ with time-delay $\tau$. Peaks in $P_{MUSIC}$ represent exact physical reflection points (i.e., human bodies).

---

## 5. Micro-Doppler Vital Sign Extraction

```
 [Sanitized Phase Time Series (100 Hz)]
                   │
                   ▼
 [Optimal Subcarrier Selection (Max SNR / Phase Variance)]
                   │
                   ▼
 [Variational Mode Decomposition (VMD / 3 Modes)]
                   │
         ┌─────────┴─────────┐
         ▼                   ▼
 [Mode 1: Breathing]  [Mode 2: Heartbeat (BCG)]
 (0.1 Hz – 0.5 Hz)    (0.8 Hz – 2.5 Hz)
         │                   │
         ▼                   ▼
 [Zero-Crossing Peak] [Continuous Wavelet Transform]
         │                   │
         ▼                   ▼
 [Respiration Rate]   [Instantaneous BPM & HRV]
```

### Vital Sign DSP Specifications:
- **Respiration Range:** $6\text{ to }30\text{ breaths/min}$ ($0.1\text{–}0.5\text{ Hz}$). Extracted via Butterworth 4th-order zero-phase bandpass filter.
- **Heart Rate Range:** $48\text{ to }150\text{ BPM}$ ($0.8\text{–}2.5\text{ Hz}$). Extracted via Morlet Wavelet Transform after respiratory fundamental and 2nd harmonics are notch-filtered.
- **Accuracy Target:** Respiration $\pm 1\text{ bpm}$, Heart Rate $\pm 2.5\text{ bpm}$ in stationary posture within 4 meters.
