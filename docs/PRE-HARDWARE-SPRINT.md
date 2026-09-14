# VORAX — Pre-Hardware Sprint Tasks (Sprint 1)

> **Objective:** Build, test, and verify all mathematical DSP engines, multi-node mesh simulations, and ESP32-S3 firmware files before physical boards arrive.
> **Scope:** Cleanly separated from the master roadmap. 100% executable without physical hardware.

---

## 🔬 MODULE 1: Advanced DSP & Filtering Pipeline (Rust)
Target: `core/src/dsp/`

- [x] **Hampel Outlier Filter (`filter.rs`):**
  - Implement rolling Median Absolute Deviation (MAD) window across subcarrier time-series.
  - Detect and eliminate high-frequency burst spikes caused by RF packet collision.
- [x] **Fresnel Bandpass IIR/FIR Filter (`bandpass.rs`):**
  - Design bandpass filter isolating $0.1\text{ Hz} - 4.0\text{ Hz}$ (physiological chest displacement).
  - Split into:
    - Respiration band: $0.1\text{ Hz} - 0.5\text{ Hz}$ ($6 - 30\text{ breaths/min}$)
    - Cardiac band: $0.8\text{ Hz} - 2.5\text{ Hz}$ ($48 - 150\text{ BPM}$)
- [x] **STFT Doppler Spectrogram (`doppler.rs`):**
  - Short-Time Fourier Transform on sanitized subcarrier amplitudes.
  - Generate micro-Doppler velocity profiles to distinguish walking, sitting, and falling.
- [x] **Motion Energy State Classifier (`classifier.rs`):**
  - Dynamic kinetic energy threshold $\tau_{motion}$.
  - Route state: `DynamicMotion` (Tracking path) vs. `QuasiStatic` (Vitals decomposition path).

---

## 📡 MODULE 2: Multi-Node Spatial Mesh Simulation
Target: `core/src/simulator/`

- [x] **4-Node Triangular Mesh Streamer (`simulator/mesh.rs`):**
  - Upgrade simulator to stream 4 distinct virtual nodes simultaneously:
    - Node 1: Corner $(0, 0)$
    - Node 2: Corner $(6, 0)$
    - Node 3: Corner $(6, 6)$
    - Node 4: Corner $(0, 6)$
  - Synchronize hardware timestamps across simulated nodes.
- [x] **Spatial Moving Target Perturbation:**
  - Simulate a walking human trajectory $(x(t), y(t))$ across the 4-node Fresnel zones.
  - Generate path-length phase differentials reflecting Doppler shift relative to each node.

---

## ⚡ MODULE 3: Ready-to-Flash ESP32-S3 Firmware
Target: `firmware/`

- [x] **ESP-IDF v5 Project Scaffolding:**
  - Create standard ESP-IDF structure: `CMakeLists.txt`, `sdkconfig.defaults`, `main/`
- [x] **Wi-Fi Promiscuous CSI Capture (`main/csi_rx.c`):**
  - Configure `wifi_csi_config_t` with 64-subcarrier raw capture callback.
  - Implement zero-copy buffer handoff into a FreeRTOS ring buffer.
- [x] **Hardware Timestamp & Header Serialization (`main/packet_builder.c`):**
  - Read nanosecond clock from `esp_timer_get_time()`.
  - Pack exact 20-byte `vorax_csi_header_t` struct with sequence counter.
- [x] **High-Speed UDP Multicast Task (`main/udp_sender.c`):**
  - Dedicated FreeRTOS task pinned to Core 1 transmitting 200 Hz UDP datagrams.
- [x] **Deployment Guide & Flashing Script:**
  - Single-command build/flash script: `flash.bat` / `flash.sh` (`idf.py -p COMx flash monitor`).

---

## 🧪 MODULE 4: Mathematical Verification & Tests
Target: `core/tests/`

- [x] **Hampel Filter Golden-Vector Test:**
  - Inject synthetic high-magnitude outlier spikes and verify 100% rejection.
- [x] **Bandpass Verification Test:**
  - Feed dual-sine wave ($0.25\text{ Hz}$ breath + $1.2\text{ Hz}$ heartbeat + $60\text{ Hz}$ AC line noise) and verify noise elimination.
- [x] **Multi-Node Ingest Benchmark:**
  - Verify sustained 800 pkts/sec throughput across 4 nodes with zero dropped frames.
