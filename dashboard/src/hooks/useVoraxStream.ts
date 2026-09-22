import { useState, useEffect, useRef } from 'react';
import type { TelemetryData, MeshNodeStatus, SecurityEvent, SecurityZone, AlertState } from '../types/telemetry';

export const SECURITY_ZONES: SecurityZone[] = [
  {
    id: 'zone-a',
    name: 'ZONE A — RECEPTION / TRANSIT',
    type: 'public',
    minX: 0,
    maxX: 3.0,
    minY: 0,
    maxY: 6.0,
    color: '#00ff9d',
    description: 'General personnel access permitted',
  },
  {
    id: 'zone-b',
    name: 'ZONE B — RESTRICTED VAULT / SERVER',
    type: 'restricted',
    minX: 3.0,
    maxX: 6.0,
    minY: 0,
    maxY: 6.0,
    color: '#ff3366',
    description: 'Requires biometrics & authorized credential',
  },
];

const INITIAL_NODES: MeshNodeStatus[] = [
  { id: 1, label: 'ANCHOR-01 (NW)', ip: '192.168.4.101', position: [0, 0, 1.2], rssi_dbm: -42, sync_offset_us: 1.4, packet_rate_hz: 200, is_active: true },
  { id: 2, label: 'ANCHOR-02 (NE)', ip: '192.168.4.102', position: [6, 0, 1.2], rssi_dbm: -46, sync_offset_us: -2.1, packet_rate_hz: 200, is_active: true },
  { id: 3, label: 'ANCHOR-03 (SE)', ip: '192.168.4.103', position: [6, 6, 1.2], rssi_dbm: -49, sync_offset_us: 0.8, packet_rate_hz: 200, is_active: true },
  { id: 4, label: 'ANCHOR-04 (SW)', ip: '192.168.4.104', position: [0, 6, 1.2], rssi_dbm: -44, sync_offset_us: -1.2, packet_rate_hz: 200, is_active: true },
];

export function useVoraxStream() {
  const [data, setData] = useState<TelemetryData | null>(null);
  const [connected, setConnected] = useState(false);
  const [events, setEvents] = useState<SecurityEvent[]>([
    {
      id: 'evt-init-0',
      timestamp: new Date().toLocaleTimeString(),
      type: 'MESH_SYNC',
      level: 'info',
      title: '4-NODE MESH SYNCHRONIZED',
      detail: 'ESP32-S3 anchors locked via hardware timer (jitter < 3µs)',
    },
    {
      id: 'evt-init-1',
      timestamp: new Date().toLocaleTimeString(),
      type: 'ONVIF_DISPATCH',
      level: 'info',
      title: 'ONVIF PROFILE M ACTIVE',
      detail: 'WS-BaseNotification bridge linked to local VMS broker',
    },
  ]);

  const lastZoneRef = useRef<'zone-a' | 'zone-b'>('zone-a');

  useEffect(() => {
    let ws: WebSocket | null = null;
    let fallbackInterval: number | null = null;
    let reconnectTimeout: number | null = null;

    const connectWs = () => {
      try {
        ws = new WebSocket('ws://localhost:8765');

        ws.onopen = () => {
          setConnected(true);
        };

        ws.onmessage = (event) => {
          try {
            const raw = JSON.parse(event.data);
            const targetX = raw.target_x ?? 3.0;
            const currentZone = targetX > 3.0 ? SECURITY_ZONES[1] : SECURITY_ZONES[0];

            setData({
              node_id: raw.node_id ?? 1,
              seq: raw.seq ?? 0,
              pps: raw.pps ?? 800,
              mean_amp: raw.mean_amp ?? 24.3,
              center_amp: raw.center_amp ?? 24.8,
              heart_rate_bpm: raw.heart_rate_bpm ?? 72.0,
              respiration_rate_brpm: raw.respiration_rate_brpm ?? 14.5,
              hrv_rmssd_ms: raw.hrv_rmssd_ms ?? 38.5,
              velocity_m_s: raw.velocity_m_s ?? 0.08,
              target_x: targetX,
              target_y: raw.target_y ?? 3.0,
              target_z: raw.target_z ?? 1.1,
              alert_state: (raw.alert_state as AlertState) || (targetX > 3.0 ? 'Warning' : 'Nominal'),
              presence_detected: raw.presence_detected ?? true,
              presence_confidence: raw.presence_confidence ?? 0.984,
              skeleton: raw.skeleton,
              amplitudes: raw.amplitudes ?? Array(64).fill(22),
              phases: raw.phases ?? Array(64).fill(0),
              nodes: INITIAL_NODES,
              current_zone: currentZone,
            });
          } catch (err) {
            console.error('Failed to parse telemetry', err);
          }
        };

        ws.onerror = () => {
          setConnected(false);
        };

        ws.onclose = () => {
          setConnected(false);
          // Try reconnecting in 5s
          reconnectTimeout = window.setTimeout(connectWs, 5000);
        };
      } catch {
        setConnected(false);
      }
    };

    connectWs();

    // Client-side simulation fallback when standalone / on Vercel preview
    fallbackInterval = window.setInterval(() => {
      setData((prev) => {
        if (ws && ws.readyState === WebSocket.OPEN) return prev;

        const now = Date.now() / 1000;
        // Smooth figure-8 walking trajectory through both zones [0.8..5.2] x [1.0..5.0]
        const tx = 3.0 + Math.sin(now * 0.45) * 1.9;
        const ty = 3.0 + Math.sin(now * 0.9) * 1.6;
        const tz = 1.05 + Math.sin(now * 1.2) * 0.04;

        // Velocity vector magnitude
        const vx = 0.45 * 1.9 * Math.cos(now * 0.45);
        const vy = 0.9 * 1.6 * Math.cos(now * 0.9);
        const velocity = Math.sqrt(vx * vx + vy * vy);

        // Biometric vital signs with physiological modulation
        const hr = 71.5 + Math.sin(now * 0.35) * 4.2;
        const rr = 14.2 + Math.cos(now * 0.18) * 1.4;
        const hrv = 39.2 + Math.sin(now * 0.22) * 3.1;

        // Active Zone Evaluation
        const currentZone = tx > 3.0 ? SECURITY_ZONES[1] : SECURITY_ZONES[0];
        const alertState: AlertState = tx > 3.0 ? 'Warning' : 'Nominal';

        // Zone transition event trigger
        if (currentZone.id !== lastZoneRef.current) {
          const isEnteringRestricted = currentZone.id === 'zone-b';
          lastZoneRef.current = currentZone.id;

          const newEvt: SecurityEvent = {
            id: `evt-${Date.now()}`,
            timestamp: new Date().toLocaleTimeString(),
            type: isEnteringRestricted ? 'ZONE_TRANSITION' : 'ZONE_TRANSITION',
            level: isEnteringRestricted ? 'warning' : 'info',
            title: isEnteringRestricted ? 'ZONE B RESTRICTED ENTRY' : 'ZONE A TRANSIT RETURN',
            detail: isEnteringRestricted
              ? `Target #01 entered ${currentZone.name} [Coords: ${tx.toFixed(2)}, ${ty.toFixed(2)}]`
              : `Target #01 returned to ${currentZone.name}`,
          };

          setEvents((eList) => [newEvt, ...eList.slice(0, 19)]);
        }

        // 64-Subcarrier OFDM channel state information
        const amps = Array.from({ length: 64 }, (_, i) => {
          const base = 21 + Math.sin(i * 0.22) * 7;
          // Subcarriers 20-44 sensitive to thoracic Doppler
          const pert = i >= 20 && i <= 44 ? Math.sin(now * (rr / 60) * Math.PI * 2) * 5.5 : 0;
          const noise = (Math.random() - 0.5) * 1.2;
          return Math.max(8, base + pert + noise);
        });

        // 4 Dynamic Node telemetry metrics
        const updatedNodes: MeshNodeStatus[] = INITIAL_NODES.map((node) => {
          const dx = tx - node.position[0];
          const dy = ty - node.position[1];
          const dist = Math.sqrt(dx * dx + dy * dy);
          // Free space path loss approximation
          const rssi = Math.round(-38 - 18 * Math.log10(Math.max(1, dist)));
          const syncJitter = Number(((Math.random() - 0.5) * 1.6).toFixed(1));

          return {
            ...node,
            rssi_dbm: rssi,
            sync_offset_us: syncJitter,
            packet_rate_hz: 200,
            is_active: true,
          };
        });

        return {
          node_id: 1,
          seq: (prev?.seq || 0) + 1,
          pps: 800,
          mean_amp: 24.8,
          center_amp: 25.1,
          heart_rate_bpm: hr,
          respiration_rate_brpm: rr,
          hrv_rmssd_ms: hrv,
          velocity_m_s: velocity,
          target_x: tx,
          target_y: ty,
          target_z: tz,
          alert_state: alertState,
          presence_detected: true,
          presence_confidence: 0.988,
          amplitudes: amps,
          phases: Array(64).fill(0),
          nodes: updatedNodes,
          current_zone: currentZone,
        };
      });
    }, 50); // 20 FPS stream

    return () => {
      if (ws) ws.close();
      if (fallbackInterval) clearInterval(fallbackInterval);
      if (reconnectTimeout) clearTimeout(reconnectTimeout);
    };
  }, []);

  return {
    data,
    connected,
    events,
    zones: SECURITY_ZONES,
  };
}
