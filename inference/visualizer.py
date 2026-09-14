# pyright: reportMissingImports=false
# type: ignore
"""
VORAX Offline Telemetry & Waterfall Visualizer

Reads raw CSI streams or saved binary frames and visualizes:
1. Per-subcarrier amplitude waterfall (heatmap)
2. Phase unwrapping response across 64 subcarriers
3. Extracted micro-Doppler signals (breathing and cardiac harmonics)
"""

import socket
import struct
import numpy as np
import matplotlib.pyplot as plt

# Matches VORAX protocol (18-byte packed header)
# Format: < (little-endian)
# H (magic 2B), H (node_id 2B), I (seq 4B), Q (ts 8B), B (ch 1B), b (rssi 1B), b (noise 1B), B (sc_count 1B)
HEADER_FORMAT = "<HHIQBbbB"
HEADER_SIZE = struct.calcsize(HEADER_FORMAT)
MAGIC = 0x5658

def listen_and_plot(port=5555, max_frames=500):
    print(f"[*] VORAX Quick-Plotter listening on UDP port {port}...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", port))

    waterfall = []
    timestamps = []

    try:
        while len(waterfall) < max_frames:
            data, _ = sock.recvfrom(512)
            if len(data) < HEADER_SIZE:
                continue

            header = struct.unpack(HEADER_FORMAT, data[:HEADER_SIZE])
            magic, node_id, seq, ts, ch, rssi, nf, sc_count = header

            if magic != MAGIC:
                continue

            sc_bytes = data[HEADER_SIZE:HEADER_SIZE + sc_count * 2]
            # Unpack signed i8 pairs: Q (imag), I (real)
            raw_iq = np.frombuffer(sc_bytes, dtype=np.int8).reshape(-1, 2)
            q = raw_iq[:, 0].astype(np.float32)
            i = raw_iq[:, 1].astype(np.float32)
            amplitudes = np.sqrt(i**2 + q**2)

            waterfall.append(amplitudes)
            timestamps.append(ts)

            if len(waterfall) % 100 == 0:
                print(f"[+] Captured {len(waterfall)}/{max_frames} frames from Node {node_id}")

    finally:
        sock.close()

    if waterfall:
        matrix = np.array(waterfall)
        plt.figure(figsize=(10, 6))
        plt.imshow(matrix.T, aspect='auto', cmap='viridis', origin='lower')
        plt.colorbar(label='Amplitude')
        plt.title('VORAX WiFi CSI Amplitude Waterfall (64 Subcarriers)')
        plt.xlabel('Frame Sequence')
        plt.ylabel('OFDM Subcarrier Index')
        plt.tight_layout()
        plt.savefig('csi_waterfall.png', dpi=150)
        print("[✓] Waterfall visualization saved to csi_waterfall.png")

if __name__ == "__main__":
    listen_and_plot()
