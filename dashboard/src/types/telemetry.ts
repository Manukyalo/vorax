export type CameraViewMode = '3d-orbit' | '2d-tactical' | 'target-lock';

export type AlertState = 'Nominal' | 'Warning' | 'Critical' | 'Breach';

export interface MeshNodeStatus {
  id: number;
  label: string;
  ip: string;
  position: [number, number, number]; // [X, Y, Z] in meters
  rssi_dbm: number;
  sync_offset_us: number; // microsecond sync offset
  packet_rate_hz: number;
  is_active: boolean;
}

export interface SecurityZone {
  id: 'zone-a' | 'zone-b';
  name: string;
  type: 'public' | 'restricted';
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
  color: string;
  description: string;
}

export interface SecurityEvent {
  id: string;
  timestamp: string;
  type: 'ZONE_TRANSITION' | 'MESH_SYNC' | 'ONVIF_DISPATCH' | 'SIA_ALARM' | 'VITALS_ANOMALY';
  level: 'info' | 'warning' | 'critical';
  title: string;
  detail: string;
}

export interface TelemetryData {
  node_id: number;
  seq: number;
  pps: number;
  mean_amp: number;
  center_amp: number;
  heart_rate_bpm: number;
  respiration_rate_brpm: number;
  hrv_rmssd_ms: number;
  velocity_m_s: number;
  target_x: number;
  target_y: number;
  target_z: number;
  alert_state: AlertState;
  presence_detected: boolean;
  presence_confidence: number;
  skeleton?: number[][]; // 17 COCO joints [x, y, z]
  amplitudes: number[]; // 64 subcarriers
  phases: number[];
  nodes: MeshNodeStatus[];
  current_zone?: SecurityZone;
}
