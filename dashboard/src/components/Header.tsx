import React from 'react';
import { Radio, Eye, Layers } from 'lucide-react';
import type { CameraViewMode } from '../types/telemetry';

interface HeaderProps {
  connected: boolean;
  seq: number;
  pps: number;
  cameraMode: CameraViewMode;
  onCameraModeChange: (mode: CameraViewMode) => void;
}

export const Header: React.FC<HeaderProps> = ({
  connected,
  seq,
  pps,
  cameraMode,
  onCameraModeChange,
}) => {
  return (
    <header
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '10px 20px',
        borderBottom: '1px solid rgba(255, 255, 255, 0.08)',
        background: 'rgba(6, 8, 13, 0.92)',
        backdropFilter: 'blur(16px)',
        zIndex: 20,
        height: '56px',
        flexShrink: 0,
      }}
    >
      {/* Brand & Subsystem Badge */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <div
            style={{
              width: 32,
              height: 32,
              borderRadius: 6,
              background: 'linear-gradient(135deg, rgba(0,240,255,0.2), rgba(0,255,157,0.1))',
              border: '1px solid rgba(0, 240, 255, 0.4)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <Radio style={{ width: 18, height: 18, color: '#00f0ff' }} />
          </div>
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <h1 style={{ fontSize: '1.05rem', fontWeight: 800, letterSpacing: '0.12em', color: '#fff', margin: 0 }}>
                VORAX<span style={{ color: '#00f0ff' }}>.AI</span>
              </h1>
              <span
                style={{
                  padding: '2px 6px',
                  borderRadius: 4,
                  fontSize: '0.62rem',
                  fontWeight: 700,
                  background: 'rgba(0, 240, 255, 0.1)',
                  color: '#00f0ff',
                  border: '1px solid rgba(0, 240, 255, 0.3)',
                  letterSpacing: '0.06em',
                }}
              >
                TACTICAL RADAR v2.0
              </span>
            </div>
            <div style={{ fontSize: '0.62rem', color: '#687791', letterSpacing: '0.04em' }}>
              4-NODE MULTI-STATIC CSI MESH & VITAL SENSING
            </div>
          </div>
        </div>
      </div>

      {/* Center Operational Controls: 3D Free-Orbit vs 2D Tactical View */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', background: 'rgba(13, 17, 26, 0.8)', padding: '3px', borderRadius: 6, border: '1px solid rgba(255,255,255,0.06)' }}>
        <button
          onClick={() => onCameraModeChange('3d-orbit')}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            padding: '5px 12px',
            borderRadius: 4,
            fontSize: '0.72rem',
            fontWeight: 600,
            cursor: 'pointer',
            border: 'none',
            background: cameraMode === '3d-orbit' ? 'rgba(0, 240, 255, 0.18)' : 'transparent',
            color: cameraMode === '3d-orbit' ? '#00f0ff' : '#8e9bb3',
            boxShadow: cameraMode === '3d-orbit' ? '0 0 10px rgba(0, 240, 255, 0.2)' : 'none',
            transition: 'all 0.15s ease',
          }}
        >
          <Eye style={{ width: 14, height: 14 }} />
          3D ORBIT VIEW
        </button>
        <button
          onClick={() => onCameraModeChange('2d-tactical')}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            padding: '5px 12px',
            borderRadius: 4,
            fontSize: '0.72rem',
            fontWeight: 600,
            cursor: 'pointer',
            border: 'none',
            background: cameraMode === '2d-tactical' ? 'rgba(0, 255, 157, 0.18)' : 'transparent',
            color: cameraMode === '2d-tactical' ? '#00ff9d' : '#8e9bb3',
            boxShadow: cameraMode === '2d-tactical' ? '0 0 10px rgba(0, 255, 157, 0.2)' : 'none',
            transition: 'all 0.15s ease',
          }}
        >
          <Layers style={{ width: 14, height: 14 }} />
          2D TACTICAL (TOP-DOWN)
        </button>
      </div>

      {/* Right Core Connection Telemetry */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              background: connected ? '#00ff9d' : '#ffb703',
              boxShadow: connected ? '0 0 8px #00ff9d' : '0 0 8px #ffb703',
            }}
          />
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            <span className="mono" style={{ fontSize: '0.75rem', fontWeight: 600, color: connected ? '#00ff9d' : '#ffb703' }}>
              {connected ? 'CORE WS:8765 LOCKED' : 'AUTONOMOUS RF SIM'}
            </span>
            <span style={{ fontSize: '0.6rem', color: '#687791' }}>
              {connected ? 'ZERO PACKET LOSS' : 'VERCEL EDGE RUNTIME'}
            </span>
          </div>
        </div>

        <div style={{ display: 'flex', gap: '16px', borderLeft: '1px solid rgba(255,255,255,0.08)', paddingLeft: '16px' }}>
          <div>
            <div style={{ fontSize: '0.6rem', color: '#687791' }}>INGEST</div>
            <div className="mono" style={{ fontSize: '0.78rem', fontWeight: 700, color: '#fff' }}>
              {pps} <span style={{ fontSize: '0.62rem', color: '#687791' }}>PKTS/S</span>
            </div>
          </div>
          <div>
            <div style={{ fontSize: '0.6rem', color: '#687791' }}>FRAME</div>
            <div className="mono" style={{ fontSize: '0.78rem', fontWeight: 700, color: '#00f0ff' }}>
              #{seq.toString().padStart(6, '0')}
            </div>
          </div>
        </div>
      </div>
    </header>
  );
};
