# VORAX ESP32-S3 Physical Firmware

Production-grade ESP-IDF v5 embedded firmware for repurposing commodity **ESP32-S3** microcontrollers ($5 each) as high-frequency Wi-Fi CSI spatial intelligence and biometric radar transceivers.

---

## 1. System Architecture

```
                  ┌───────────────────────────────────────────────┐
                  │              ESP32-S3 SoC (240 MHz)           │
                  │                                               │
                  │  [Core 0: Wi-Fi Promiscuous RX ISR]           │
                  │  wifi_csi_raw_cb()                            │
                  │  - 64 Subcarriers (HT20)                      │
                  │  - Hardware timer (esp_timer_get_time())      │
                  │  - Pack 20B Header (Magic 0x5658)             │
                  │              │                                │
                  │              ▼ (xQueueSendFromISR)            │
                  │  [FreeRTOS Bounded Queue (32 frames)]         │
                  │              │                                │
                  │              ▼ (xQueueReceive)                │
                  │  [Core 1: High-Speed UDP Task]                │
                  │  udp_tx_task() (Priority 5)                   │
                  │  - Serializes 148-byte wire packet            │
                  │  - sendto() UDP broadcast / unicast           │
                  └──────────────┬────────────────────────────────┘
                                 │
                                 ▼ (200 - 500 Hz UDP / Port 5555)
                    [ RUST VORAX INGEST ENGINE ]
```

---

## 2. Wire Protocol Format

Each datagram sent to UDP port `5555` matches `core/src/protocol.rs` in Rust bit-for-bit:

| Offset (Bytes) | Field Name | Data Type | Description |
| :--- | :--- | :--- | :--- |
| `0..2` | `magic` | `uint16_t` | `0x5658` (`"VX"`) |
| `2..4` | `node_id` | `uint16_t` | Sensor index (1–4) |
| `4..8` | `sequence_number` | `uint32_t` | Monotonic frame counter |
| `8..16` | `timestamp_us` | `uint64_t` | Microsecond clock from `esp_timer_get_time()` |
| `16` | `channel` | `uint8_t` | Wi-Fi Channel (e.g. 6) |
| `17` | `rssi` | `int8_t` | Received signal strength in dBm |
| `18` | `noise_floor` | `int8_t` | Background noise in dBm |
| `19` | `subcarrier_count`| `uint8_t` | `64` (HT20) |
| `20..148` | `subcarriers` | `int8_t[128]` | 64 pairs of interleaved $[Q, I]$ signed integers |

**Total Packet Size**: $20 + 128 = 148\text{ bytes}$.

---

## 3. Quickstart & Flashing Guide

### Prerequisites
1. [ESP-IDF v5.1+](https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/get-started/) installed.
2. ESP32-S3 development board connected via USB (UART/USB port).

### Building & Flashing

1. Set up target:
   ```bash
   idf.py set-target esp32s3
   ```
2. Build firmware:
   ```bash
   idf.py build
   ```
3. One-Click Flash (Windows):
   ```cmd
   flash.bat COM3 921600
   ```
4. One-Click Flash (Linux/macOS):
   ```bash
   chmod +x flash.sh
   ./flash.sh /dev/ttyUSB0 921600
   ```

---

## 4. Multi-Node Deployment (Mesh Configuration)

To deploy a 4-node spatial mesh:
1. In `main/main.c`, adjust `DEFAULT_NODE_ID` for each physical board:
   - Node 1: Front-Left `(0.0, 0.0)`
   - Node 2: Front-Right `(6.0, 0.0)`
   - Node 3: Rear-Right `(6.0, 6.0)`
   - Node 4: Rear-Left `(0.0, 6.0)`
2. Flash each board with its assigned ID.
3. Once powered, all boards stream CSI frames simultaneously to `255.255.255.255:5555`, automatically feeding into the Rust 2D-MUSIC multilateration and 3D visualizer.
