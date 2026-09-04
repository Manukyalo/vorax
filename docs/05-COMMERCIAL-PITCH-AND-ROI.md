# 05 — Commercial Pitch, Unit Economics & ROI

## 1. Enterprise Security & Industrial Pitch Deck Outline

### Pitch Hook
> *"What if your security system could see in total darkness, track intruders through interior walls, verify vital signs in smoke-filled warehouses, and never violate employee privacy or GDPR?"*

---

## 2. Technology Comparison Matrix

| Feature | RF Spatial Mesh (Our System) | Optical CCTV / AI Cameras | Passive Infrared (PIR) | 3D LiDAR Sensors |
| :--- | :--- | :--- | :--- | :--- |
| **Hardware Cost / Unit** | **$15 – $30** | $150 – $800 | $15 – $40 | $1,500 – $8,000 |
| **Through-Wall Sensing** | **Yes (Drywall, Wood, Glass)**| No | No | No |
| **Zero-Light / Darkness**| **100% Operational** | Requires IR Illumination | Yes (Low fidelity) | Yes |
| **Smoke / Fog / Dust**   | **Unaffected** | Completely blinded | Blinded by heat/smoke | Degraded by scattering |
| **Vital Sign Monitoring**| **Yes (Breathing & Heartbeat)**| No | No | No |
| **Privacy Compliance**   | **GDPR / HIPAA Native (No PII)**| High Liability / Audits | Native | Native |
| **Tamper Resistance**    | **Cannot be spray-painted/covered**| Easily blinded | Easily blocked | Easily blocked |

---

## 3. Hardware Bill of Materials (BOM) & Unit Economics

### Industrial Edge Sensor Node (BOM @ 1,000 Units)

| Component | Part Description | Unit Cost |
| :--- | :--- | :--- |
| **SoC / RF MCU** | Espressif ESP32-S3-WROOM-1U (8MB PSRAM, IPEX) | $3.80 |
| **Antennas** | Dual $2.4\text{ GHz} / 5\text{ GHz}$ High-Gain Dipole / Patch Antennas | $2.20 |
| **Power Supply** | PoE (Power over Ethernet) Module or USB-C 5V Regulator | $3.10 |
| **Enclosure** | IP67 Dust/Waterproof Polycarbonate Wall-Mount Case | $2.40 |
| **PCB & Passive SMT** | Custom carrier board + surge protection | $1.90 |
| **Total Node BOM** | | **~$13.40** |

### Complete 4-Node Room / Aisle Kit
- **4x Edge Sensor Nodes:** ~$53.60
- **1x Local Edge Processing Hub (Raspberry Pi 5 / RK3588 NPU):** ~$85.00
- **Total Kit Hardware Cost:** **~$138.60** (Retails for **$699 – $1,200** with software license).

---

## 4. Revenue Model

```
                    ┌───────────────────────────────────────┐
                    │            REVENUE STREAMS            │
                    └───────────────────┬───────────────────┘
                                        │
             ┌──────────────────────────┼──────────────────────────┐
             ▼                          ▼                          ▼
   [Hardware Gateway Kits]     [SaaS / Edge License]      [Custom Integrations]
   $699 – $1,499 per kit       $25 – $65 / zone / month   Enterprise VMS & WMS
   (65% gross hardware margin) (Recurring software ARR)   SCADA custom deployments
```

---

## 5. Regulatory & Compliance Advantages

1. **GDPR / California CCPA:** No facial images, biological identifiable imagery, or video streams stored on servers. Zero risk of video data breaches.
2. **OSHA & Industrial Safety:** Certified non-optical blind-spot detection for machinery/forklift zones.
3. **HIPAA Compliant Healthcare:** Continuous non-contact patient monitoring in hospitals and eldercare without intrusive cameras.
