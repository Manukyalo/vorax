# 06 — Development Roadmap & Execution Plan

## Phase 1: High-Performance Ingestion & DSP Core (Weeks 1–4)
- [ ] **ESP32-S3 Firmware Refactor:**
  - Build optimized ESP-IDF firmware capturing 64-subcarrier CSI frames @ 200–500 Hz.
  - Implement dual-antenna differential capture with microsecond timestamp sync.
- [ ] **Rust Core Ingest Service:**
  - Build async UDP receiver with zero-copy packet parsing using `tokio` and `crossbeam`.
  - Implement SIMD-accelerated phase unwrapping and conjugate antenna multiplication.
  - Implement Hampel outlier and Fresnel bandpass filtering.

## Phase 2: Dual-Mode Motion & Biometric Pipeline (Weeks 5–8)
- [ ] **Motion Energy & Velocity Classifier:**
  - Implement real-time Short-Time Fourier Transform (STFT) for Doppler Velocity Spectrograms.
  - Build the dynamic motion vs. quasi-static vitals state dispatcher.
- [ ] **Micro-Doppler Biometrics Engine:**
  - Implement Variational Mode Decomposition (VMD) to isolate cardiac and respiratory harmonics.
  - Continuous Wavelet Transform (CWT) peak extractor for real-time BPM and HRV metrics.
  - Apnea and distress alert threshold logic.

## Phase 3: Spatial Localization & Cross-Modal ML (Weeks 9–12)
- [ ] **3D Voxel Grid Engine:**
  - Implement 2D-MUSIC (AoA / ToF) and spatial multilateration across 4+ nodes.
  - Generate 3D point-cloud occupancy vectors.
- [ ] **Cross-Modal Teacher-Student Model:**
  - Synchronize RF arrays with RGB Pose cameras in a training testbed.
  - Train Domain Adversarial (DANN) RF skeleton model to eliminate room-specific multipath overfitting.
  - Export quantized model to ONNX / TensorRT / OpenVINO.

## Phase 4: Enterprise Integrations & 3D UI (Weeks 13–16)
- [ ] **3D WebGL / WebGPU Visualizer:**
  - Real-time Three.js spatial canvas rendering live skeletal bodies, voxel heatmaps, and vital statistics.
  - WebSockets streaming over binary Protocol Buffers.
- [ ] **Security & Industrial Gateways:**
  - Implement SIA DC-09 / ONVIF Profile M integration for Video Management Systems.
  - Implement MQTT / ROS2 bridge for warehouse robot safety interlocks.

## Phase 5: Field Testing & Commercial Pilots (Weeks 17+)
- [ ] **Warehouse Pilot Deployment:** Test forklift blind-spot detection and container yard perimeter mesh in live logistics facilities.
- [ ] **Security & Healthcare Pilot:** Deploy non-contact through-wall sleep and vital monitoring in eldercare/assisted living environments.
