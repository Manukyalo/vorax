import { useState } from 'react';
import { useVoraxStream } from './hooks/useVoraxStream';
import { Header } from './components/Header';
import { SpatialCanvas } from './components/SpatialCanvas';
import { HUDOverlays } from './components/HUDOverlays';
import { NodeMeshMatrix } from './components/NodeMeshMatrix';
import { BiometricsCard } from './components/BiometricsCard';
import { RFDopplerWaterfall } from './components/RFDopplerWaterfall';
import { EnterpriseEventLog } from './components/EnterpriseEventLog';
import type { CameraViewMode } from './types/telemetry';

export function App() {
  const { data, connected, events } = useVoraxStream();
  const [cameraMode, setCameraMode] = useState<CameraViewMode>('3d-orbit');

  const pps = data?.pps ?? 800;
  const seq = data?.seq ?? 0;
  const targetX = data?.target_x ?? 3.0;
  const targetY = data?.target_y ?? 3.0;
  const targetZ = data?.target_z ?? 1.1;
  const velocity = data?.velocity_m_s ?? 0.08;
  const hr = data?.heart_rate_bpm ?? 72.0;
  const rr = data?.respiration_rate_brpm ?? 14.5;
  const hrv = data?.hrv_rmssd_ms ?? 38.5;
  const confidence = data?.presence_confidence ?? 0.988;
  const alertState = data?.alert_state ?? 'Nominal';
  const isBreach = alertState === 'Warning' || alertState === 'Critical' || alertState === 'Breach';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', width: '100vw', height: '100vh', background: 'var(--bg-deep)' }}>
      {/* 1. Tactical Command Header */}
      <Header
        connected={connected}
        seq={seq}
        pps={pps}
        cameraMode={cameraMode}
        onCameraModeChange={setCameraMode}
      />

      {/* 2. Main Operational Split Screen */}
      <div style={{ display: 'flex', flex: 1, position: 'relative', overflow: 'hidden' }}>
        {/* Left Viewport: Interactive 3D Spatial Radar */}
        <div style={{ flex: 1, position: 'relative', background: '#06080d' }}>
          <SpatialCanvas
            presence={data?.presence_detected ?? true}
            heartRate={hr}
            amplitudes={data?.amplitudes ?? []}
            targetPos={{ x: targetX, y: targetY, z: targetZ }}
            skeleton={data?.skeleton}
            cameraMode={cameraMode}
            nodes={data?.nodes ?? []}
            isBreach={isBreach}
          />

          {/* Floating HUD Indicators */}
          <HUDOverlays
            presence={data?.presence_detected ?? true}
            confidence={confidence}
            targetX={targetX}
            targetY={targetY}
            targetZ={targetZ}
            velocity={velocity}
            currentZone={data?.current_zone}
            alertState={alertState}
          />

          {/* Bottom Viewport Interactive Controls Hint */}
          <div
            style={{
              position: 'absolute',
              bottom: 12,
              left: 16,
              fontSize: '0.62rem',
              color: 'rgba(255, 255, 255, 0.4)',
              background: 'rgba(6, 8, 13, 0.75)',
              padding: '4px 10px',
              borderRadius: 4,
              border: '1px solid rgba(255, 255, 255, 0.06)',
              pointerEvents: 'none',
            }}
          >
            <span style={{ color: '#00f0ff' }}>MOUSE:</span> Left Click: Rotate • Right Click: Pan • Scroll: Zoom • Toggle: 2D Tactical
          </div>
        </div>

        {/* Right Telemetry & Subsystems Sidebar */}
        <aside
          style={{
            width: 390,
            borderLeft: '1px solid rgba(255, 255, 255, 0.08)',
            background: 'rgba(9, 12, 19, 0.94)',
            backdropFilter: 'blur(20px)',
            display: 'flex',
            flexDirection: 'column',
            padding: '14px',
            gap: '12px',
            overflowY: 'auto',
            zIndex: 10,
          }}
        >
          {/* A. 4-Anchor Hardware Mesh Topology */}
          <NodeMeshMatrix nodes={data?.nodes ?? []} />

          {/* B. Contactless Micro-Doppler Vitals */}
          <BiometricsCard
            heartRate={hr}
            respirationRate={rr}
            hrv={hrv}
            confidence={confidence}
            alertState={alertState}
          />

          {/* C. Live Micro-Doppler Spectrogram & 64-SC Profile */}
          <RFDopplerWaterfall
            amplitudes={data?.amplitudes ?? []}
            velocity={velocity}
          />

          {/* D. Enterprise Event Stream (ONVIF / SIA / VMS) */}
          <EnterpriseEventLog events={events} />
        </aside>
      </div>
    </div>
  );
}

export default App;
