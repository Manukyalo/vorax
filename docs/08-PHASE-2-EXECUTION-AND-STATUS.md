# VORAX — System Status Overview & Phase 2 Task List

**Generated:** September 2026  
**Status:** Active Implementation  
**Target:** Monorepo (`core/`, `dashboard/`, `firmware/`, `inference/`)

---

## 1. Master System Status: What's Done vs. What's Not Done

### 🟢 Completed & Tested (`Production Ready`)
1. **High-Throughput UDP Ingest Core (`core/src/ingest.rs`)**:
   - Async non-blocking socket receiver bound to `0.0.0.0:5555`.
   - Lock-free bounded crossbeam ring buffer (1024 capacity) preventing backpressure memory leaks.
   - Sustained throughput validated at 200 Hz with zero dropped frames.
2. **Wire Protocol Serialization (`core/src/protocol.rs`)**:
   - Packed 20-byte binary header parser (`0x5658` magic, node ID, sequence, timestamp, channel, RSSI, noise floor, subcarrier count).
   - Real ($I$) and Imaginary ($Q$) signed int8 subcarrier amplitude and phase unwrapping.
3. **Phase Sanitization (`core/src/dsp/phase.rs`)**:
   - $2\pi$ branch-cut phase unwrapping algorithm.
   - Linear regression trend and slope removal eliminating Packet Detection Delay (PDD).
4. **Hampel Outlier Filter (`core/src/dsp/filter.rs`)**:
   - Rolling Median Absolute Deviation (MAD) window algorithm.
   - Rejection of sudden burst spikes from RF collisions and multipath flash.
5. **Fresnel Biquad Bandpass Filters (`core/src/dsp/bandpass.rs`)**:
   - Direct Form II Transposed 2nd-order IIR Biquad filter engine.
   - Preset configurations for Respiration band ($0.1 - 0.5\text{ Hz}$) and Cardiac band ($0.8 - 2.5\text{ Hz}$).
6. **Kinetic Variance Classifier (`core/src/dsp/classifier.rs`)**:
   - Variance and energy evaluator distinguishing `DynamicMotion`, `QuasiStaticVitals`, and `FallSuspected`.
7. **3D Real-Time Spatial Dashboard (`dashboard/`)**:
   - Three.js 3D spatial voxel grid, human target marker, real-time 64-subcarrier waterfall spectrogram.
   - Dual-mode operation: Live WebSocket (`ws://localhost:8765`) + standalone client-side synthetic fallback.
8. **200 Hz CSI Single-Node Simulator (`core/src/simulator/main.rs`)**:
   - Generates realistic synthetic 64-subcarrier CSI packets with sinusoidal physiological perturbations.

---

### 🟡 In Progress / Next Up (`Phase 2 Execution`)
1. **STFT Micro-Doppler Spectrogram (`core/src/dsp/doppler.rs`)**:
   - Sliding window discrete Fourier transform producing velocity spectrograms ($v = \frac{c \cdot f_D}{2 f_c}$).
2. **Micro-Doppler Biometrics Engine (`core/src/dsp/vitals.rs`)**:
   - Subcarrier selection (selecting subcarriers with maximum breathing/cardiac SNR).
   - Peak detection and inter-beat interval (IBI) extraction for instantaneous Heart Rate (BPM) and Respiration Rate (BrPM).
   - Heart Rate Variability (HRV - RMSSD) computation.
   - Apnea (cessation of breath $> 10\text{ s}$) and distress alert threshold logic.
3. **Pipeline Dispatcher (`core/src/dsp/dispatcher.rs`)**:
   - Clean state routing between tracking trajectory and vitals extraction.

---

### 🔴 Not Started / Future Sprints (Phases 3 – 5)
1. **Multi-Node 4-Anchor Spatial Mesh Sim (`core/src/simulator/mesh.rs`)**:
   - 4-node synchronized packet generation with spatial moving target Fresnel perturbations.
2. **ESP32-S3 Physical Firmware (`firmware/`)**:
   - ESP-IDF v5 zero-copy Wi-Fi promiscuous CSI capture callback, 20-byte packet builder, UDP multicast task.
3. **Phase 3: 2D-MUSIC Spatial AoA / ToF Engine**:
   - Spatial smoothing covariance matrices, eigenvector decomposition, and 3D voxel point-cloud generation.
4. **Phase 3: Cross-Modal ML Skeleton Model**:
   - Domain Adversarial Neural Network (DANN) trained against camera ground-truth.
5. **Phase 4: Enterprise Protocols**:
   - ONVIF Profile M, SIA DC-09 VMS alarms, ROS2 / MQTT safety interlock bridges.

---

## 2. Phase 2 Detailed Task List & Execution Plan

| Task ID | Component | File Target | Scope Description | Status |
| :--- | :--- | :--- | :--- | :--- |
| **P2-01** | STFT Engine | `core/src/dsp/doppler.rs` | Windowed STFT on subcarrier time series; micro-Doppler velocity spectrogram | 🟢 Complete |
| **P2-02** | State Dispatcher | `core/src/dsp/dispatcher.rs` | Routes sanitized CSI frames to either Doppler Motion Tracker or Vitals Engine | 🟢 Complete |
| **P2-03** | Subcarrier SNR Selector | `core/src/dsp/vitals.rs` | Selects top-$K$ sensitive OFDM subcarriers by phase variance and SNR | 🟢 Complete |
| **P2-04** | Vitals Extractor | `core/src/dsp/vitals.rs` | Bandpass filtering + peak detection for respiration ($6-30\text{ BrPM}$) & cardiac ($48-150\text{ BPM}$) | 🟢 Complete |
| **P2-05** | Biometric Metrics & Alerts | `core/src/dsp/vitals.rs` | HRV (RMSSD), Apnea detection ($>10\text{s}$ no breath), Tachy/Bradycardia alerts | 🟢 Complete |
| **P2-06** | Engine Integration | `core/src/main.rs` & `dashboard/` | Wire STFT + Vitals + Dispatcher into live DSP worker thread & WebSocket stream | 🟢 Complete |
| **P2-07** | Verification Tests | `core/src/dsp/` | Comprehensive unit tests across STFT, vitals, bandpass, and filters | 🟡 In Verification |
