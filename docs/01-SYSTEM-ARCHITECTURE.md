# 01 — System Architecture & Data Pipeline

## 1. High-Level Multi-Tier Architecture

```
┌───────────────────────────────────────────────────────────────────────────┐
│                        TIER 1: SENSOR EDGE NODES                          │
│                                                                           │
│   [ESP32-S3 Node 1]      [ESP32-S3 Node 2]      [ESP32-S3 Node N]         │
│   • 802.11n HT20/HT40    • 802.11n HT20/HT40    • 802.11n HT20/HT40       │
│   • 64 Subcarriers CSI   • 64 Subcarriers CSI   • 64 Subcarriers CSI      │
│   • Packet Rate: 200Hz   • Packet Rate: 200Hz   • Packet Rate: 200Hz      │
└───────────────────────────────┬───────────────────────────────────────────┘
                                │ UDP Multicast / Raw Ethernet Frames
                                ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                  TIER 2: RUST HIGH-THROUGHPUT DSP CORE                    │
│                                                                           │
│   [Zero-Copy Ingest] ──▶ [Ring Buffers] ──▶ [SIMD Phase Sanitizer]        │
│          │                                         │                      │
│          │                                         ▼                      │
│          └───────────────────────────────▶ [Hampel / Fresnel Denoising]   │
│                                                    │                      │
│                                                    ▼                      │
│                                            [2D-MUSIC / AoA / ToF]         │
└───────────────────────────────┬───────────────────────────────────────────┘
                                │ Sanitized RF Tensor (Flatbuffers / Shared Mem)
                                ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                     TIER 3: SPATIAL & ML INFERENCE                        │
│                                                                           │
│     ┌────────────────────────────────┐ ┌────────────────────────────────┐ │
│     │      DYNAMIC MOTION ENGINE     │ │      QUASI-STATIC VITALS       │ │
│     │ • 3D Voxel Occupancy Grid      │ │ • Subcarrier Selection Engine  │ │
│     │ • 17-Point 3D Skeleton (TCN)   │ │ • Variational Mode Decomp(VMD) │ │
│     │ • Velocity & Fall State Vector │ │ • Continuous Wavelet Transform │ │
│     └───────────────┬────────────────┘ └────────────────┬───────────────┘ │
└─────────────────────┼───────────────────────────────────┼─────────────────┘
                      │                                   │
                      ▼                                   ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                   TIER 4: INTEGRATION & 3D VISUALIZATION                  │
│                                                                           │
│   • gRPC / WebSockets Streaming Server                                    │
│   • WebGPU / Three.js 3D Real-Time Spatial Canvas                         │
│   • Security / VMS Webhooks (Milestone, Genetec, SIA DC-09)               │
│   • Warehouse WMS / Industrial Safety Bus (MQTT / ROS2)                   │
└───────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Tier 1: Sensor Node Firmware & RF Capture

### Hardware Platform
- **Microcontroller:** Espressif ESP32-S3 (Dual-core Xtensa LX7 @ 240MHz, 8MB PSRAM, Wi-Fi 802.11 b/g/n).
- **Antenna Config:** Dual external dipole antennas (2.4 GHz) or custom ceramic patch antennas positioned at $\lambda/2$ spacing ($6.25\text{ cm}$) for physical phase-differential extraction.

### Capture Mechanism
- Operates in **Promiscuous CSI Capture Mode** using Espressif ESP-IDF API `esp_wifi_set_csi_rx_cb()`.
- Captures 64 OFDM subcarrier complex pairs $(I, Q)$ per packet.
- Header attaches:
  - 64-bit hardware nanosecond timestamp.
  - Node ID and Antenna ID.
  - RSSI, noise floor, channel index, and sequence counter.

### Raw CSI Packet Structure
```rust
#[repr(C, packed)]
pub struct CsiPacketHeader {
    pub magic: u16,            // 0x5256 ("RV")
    pub node_id: u16,          // Unique sensor identifier
    pub sequence_number: u32,  // Drop detection
    pub timestamp_us: u64,     // Monotonic microsecond clock
    pub channel: u8,           // Wi-Fi channel index
    pub rssi: i8,              // Received Signal Strength
    pub noise_floor: i8,       // Background noise level
    pub subcarrier_count: u8,  // 64 for 20MHz, 128 for 40MHz
}

#[repr(C, packed)]
pub struct SubcarrierComplex {
    pub imaginary: i8,
    pub real: i8,
}
```

---

## 3. Tier 2: Rust DSP Ingestion Engine

### Key Design Pillars:
1. **Lock-Free Concurrency:** Uses `crossbeam-channel` and bounded lock-free ring buffers to isolate network I/O from numerical crunching.
2. **SIMD Vectorization:** Leverages `std::simd` / AVX2 for parallel subcarrier phase unwrapping and matrix multiplication.
3. **Low Latency:** End-to-end processing latency from packet receipt to sanitized tensor is under **$1.2\text{ ms}$**.

---

## 4. Tier 3: ML & Spatial Pipeline

- **Inference Runtime:** ONNX Runtime with TensorRT execution provider (GPU) or OpenVINO (Intel CPU / NPU).
- **Dual Pipeline Dispatcher:** An adaptive motion energy classifier routes data:
  - **Energy $> \tau_{motion}$:** Dynamic 3D tracking & skeletal reconstruction.
  - **Energy $< \tau_{motion}$:** Micro-Doppler vital sign decomposition.

---

## 5. Tier 4: Output & Integration Contracts

- **Streaming Protocol:** Binary WebSockets via Protocol Buffers / Flatbuffers for $60\text{ FPS}$ UI rendering.
- **Enterprise Security Interfaces:**
  - **SIA DC-09 / Contact ID** over IP for central monitoring stations.
  - **ONVIF Profile M** metadata streamer for Video Management Systems (Milestone XProtect, Genetec Security Center).
- **Industrial Safety Interfaces:**
  - **MQTT** telemetry bus for SCADA integration.
  - **ROS2 Node** (`sensor_msgs/PointCloud2` and `nav_msgs/OccupancyGrid`) for autonomous mobile robots (AMRs) and automated guided vehicles (AGVs).
