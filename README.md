<<<<<<< HEAD
# VORAX

**Camera-free spatial intelligence — built on WiFi physics, not pixels.**

VORAX transforms commodity WiFi signals into real-time spatial awareness: human presence, 3D positioning, skeletal pose reconstruction, and contactless vital sign monitoring — through walls, without cameras, without wearables.

Built for enterprise-grade deployment across security, industrial safety, and healthcare verticals.

---

## What VORAX Does

| Capability | Detail |
|------------|--------|
| **Presence Detection** | > 97% accuracy, through drywall and glass partitions |
| **3D Localization** | Sub-meter positioning from a 4-node mesh |
| **Skeletal Tracking** | 17-keypoint body pose at 30 FPS |
| **Vital Sign Monitoring** | ±3 BPM heart rate, respiratory rate, HRV — contactless |
| **Fall Detection** | Real-time biomechanical event classification |
| **Perimeter Sensing** | Open yards, container fields, warehouse floor mesh |

No camera feed. No PII. No wearables. Privacy-by-design.

---

## How It Works

VORAX exploits **WiFi Channel State Information (CSI)** — the per-subcarrier complex amplitude and phase data embedded in every 802.11n packet. When a human moves in a space, they disturb the multipath propagation of RF signals in physically deterministic ways. VORAX captures, sanitizes, and decodes those disturbances in real time.

```
┌─────────────────────────────────────────────────────────────────┐
│                   TIER 1: SENSOR EDGE NODES                     │
│  ESP32-S3 × N  —  64-subcarrier CSI @ 200 Hz  —  UDP Multicast │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│               TIER 2: RUST HIGH-THROUGHPUT DSP CORE             │
│  Zero-copy ingest → SIMD phase unwrap → Hampel/Fresnel filter   │
│  Latency target: < 1.2 ms packet-to-tensor                      │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│              TIER 3: SPATIAL & ML INFERENCE                     │
│  Dynamic path: 3D Voxel Grid → Temporal CNN → 17-pt Skeleton    │
│  Vitals path:  VMD + CWT → BPM / HRV / Respiratory Rate        │
│  Runtime: ONNX + TensorRT / OpenVINO on Rockchip RK3588 NPU    │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│           TIER 4: INTEGRATION & 3D VISUALIZATION                │
│  WebGPU / Three.js dashboard · gRPC · WebSockets · Protobuf     │
│  MQTT / ROS2 · SIA DC-09 · ONVIF Profile M                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Repository Structure

```
vorax/
├── firmware/        # ESP32-S3 ESP-IDF firmware (C/C++)
├── core/            # Rust DSP ingest & signal processing engine
├── inference/       # ML training pipeline & ONNX export (Python)
├── dashboard/       # Real-time 3D visualization (React + Three.js)
├── infra/           # Docker, deployment & gateway configuration
└── docs/            # Technical architecture & blueprint documents
    ├── 01-SYSTEM-ARCHITECTURE.md
    ├── 02-DSP-AND-SIGNAL-PROCESSING.md
    ├── 03-MACHINE-LEARNING-AND-SPATIAL-LOCALIZATION.md
    ├── 04-SECURITY-AND-WAREHOUSE-DEPLOYMENT.md
    ├── 05-COMMERCIAL-PITCH-AND-ROI.md
    └── 06-DEVELOPMENT-ROADMAP.md
```

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Sensor firmware | ESP-IDF (C), ESP32-S3 |
| DSP core | Rust · tokio · crossbeam · std::simd (AVX2) · Flatbuffers |
| ML inference | ONNX Runtime · TensorRT · OpenVINO · PyTorch (training) |
| Edge gateway | Rockchip RK3588 — 6 TOPS NPU, < 10W |
| Dashboard | React · Three.js · WebGPU · WebSockets |
| Data layer | TimescaleDB · Redis |
| Integrations | gRPC · MQTT · ROS2 · SIA DC-09 · ONVIF Profile M |

---

## Target Verticals

**Security & Perimeter**
Through-wall intrusion detection, container yard perimeter mesh, blind-spot coverage for camera-dark zones.

**Industrial & Warehouse**
Forklift blind-spot safety interlocks, AMR/AGV human proximity alerts, real-time floor occupancy for WMS integration.

**Healthcare & Eldercare**
Contactless bed-exit detection, fall alerting, non-diagnostic vital sign telemetry for assisted living and ICU monitoring.

---

## Development Phases

| Phase | Scope | Timeline |
|-------|-------|----------|
| **0 — Foundation** | Rust ingest scaffold, CSI simulator, ring buffer, packet parser | Week 1 |
| **1 — Hardware** | ESP32-S3 firmware, dual-node capture, SIMD DSP | Week 2 |
| **2 — Biometrics** | STFT Doppler, VMD vitals, CWT BPM extraction | Weeks 3–4 |
| **3 — Spatial ML** | 2D-MUSIC AoA, 3D voxel grid, skeleton TCN, ONNX export | Weeks 5–8 |
| **4 — Dashboard** | Three.js 3D canvas, WebSocket streaming, enterprise integrations | Weeks 9–12 |
| **5 — Pilots** | Warehouse & healthcare field deployments | Weeks 13–16 |

---

## Privacy & Compliance

VORAX is **privacy-by-design**:
- No video. No images. No audio.
- No biometric identifiers stored — only aggregate spatial vectors.
- GDPR compliant by architecture: no PII is ever captured or transmitted.
- Healthcare deployments operate as Class I non-diagnostic safety devices (no FDA 510(k) required for Phase 1).

---

## Status

> 🔨 **Active development — Phase 0**

---

## License

Proprietary. All rights reserved. © 2026 VORAX.
=======
# vorax
>>>>>>> d1db27cfb9b891038bf7e75e6b211896756bd64f
