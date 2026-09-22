import React from 'react';
import { ShieldCheck, Compass, Activity, Brain, AlertTriangle } from 'lucide-react';
import type { SecurityZone, AlertState } from '../types/telemetry';

interface HUDOverlaysProps {
  presence: boolean;
  confidence: number;
  targetX: number;
  targetY: number;
  targetZ: number;
  velocity: number;
  currentZone?: SecurityZone;
  alertState: AlertState;
}

export const HUDOverlays: React.FC<HUDOverlaysProps> = ({
  presence,
  confidence,
  targetX,
  targetY,
  targetZ,
  velocity,
  currentZone,
  alertState,
}) => {
  const isBreach = alertState === 'Warning' || alertState === 'Critical' || alertState === 'Breach';

  return (
    <div
      style={{
        position: 'absolute',
        top: 16,
        left: 16,
        display: 'flex',
        flexWrap: 'wrap',
        gap: '10px',
        zIndex: 10,
        pointerEvents: 'none',
      }}
    >
      {/* Target Occupancy Badge */}
      <div
        className="tactical-card"
        style={{
          padding: '8px 14px',
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
          pointerEvents: 'auto',
          borderLeft: `3px solid ${presence ? '#00ff9d' : '#8e9bb3'}`,
        }}
      >
        <ShieldCheck style={{ color: presence ? '#00ff9d' : '#8e9bb3', width: 18, height: 18 }} />
        <div>
          <div style={{ fontSize: '0.6rem', color: '#8e9bb3', letterSpacing: '0.05em' }}>TARGET OCCUPANCY</div>
          <div style={{ fontSize: '0.82rem', fontWeight: 700, color: presence ? '#00ff9d' : '#8e9bb3' }}>
            {presence ? '1 PERSON (LOCALIZED)' : 'SCANNING AREA...'}
          </div>
        </div>
      </div>

      {/* Security Zone Geofence Status */}
      <div
        className="tactical-card"
        style={{
          padding: '8px 14px',
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
          pointerEvents: 'auto',
          borderLeft: `3px solid ${isBreach ? '#ff3366' : '#00ff9d'}`,
          background: isBreach ? 'rgba(255, 51, 102, 0.12)' : 'rgba(13, 17, 26, 0.78)',
        }}
      >
        {isBreach ? (
          <AlertTriangle style={{ color: '#ff3366', width: 18, height: 18 }} />
        ) : (
          <div style={{ width: 12, height: 12, borderRadius: '50%', background: '#00ff9d', boxShadow: '0 0 8px #00ff9d' }} />
        )}
        <div>
          <div style={{ fontSize: '0.6rem', color: '#8e9bb3', letterSpacing: '0.05em' }}>ACTIVE GEOFENCE ZONE</div>
          <div className="mono" style={{ fontSize: '0.82rem', fontWeight: 700, color: isBreach ? '#ff3366' : '#00ff9d' }}>
            {currentZone ? currentZone.name : 'ZONE EVALUATION'}
          </div>
        </div>
      </div>

      {/* 3D Spatial Fix Coordinates */}
      <div
        className="tactical-card"
        style={{
          padding: '8px 14px',
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
          pointerEvents: 'auto',
          borderLeft: '3px solid #00f0ff',
        }}
      >
        <Compass style={{ color: '#00f0ff', width: 18, height: 18 }} />
        <div>
          <div style={{ fontSize: '0.6rem', color: '#8e9bb3', letterSpacing: '0.05em' }}>3D SPATIAL FIX [X, Y, Z]</div>
          <div className="mono" style={{ fontSize: '0.82rem', fontWeight: 700, color: '#00f0ff' }}>
            [{targetX.toFixed(2)}, {targetY.toFixed(2)}, {targetZ.toFixed(2)}]
            <span style={{ fontSize: '0.65rem', color: '#8e9bb3', marginLeft: 4 }}>M</span>
          </div>
        </div>
      </div>

      {/* Doppler Velocity Vector */}
      <div
        className="tactical-card"
        style={{
          padding: '8px 14px',
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
          pointerEvents: 'auto',
          borderLeft: '3px solid #ffb703',
        }}
      >
        <Activity style={{ color: '#ffb703', width: 18, height: 18 }} />
        <div>
          <div style={{ fontSize: '0.6rem', color: '#8e9bb3', letterSpacing: '0.05em' }}>DOPPLER VELOCITY</div>
          <div className="mono" style={{ fontSize: '0.82rem', fontWeight: 700, color: '#ffb703' }}>
            {velocity.toFixed(2)}
            <span style={{ fontSize: '0.65rem', color: '#8e9bb3', marginLeft: 4 }}>M/S (WALKING)</span>
          </div>
        </div>
      </div>

      {/* ML Kinematics & Confidence */}
      <div
        className="tactical-card"
        style={{
          padding: '8px 14px',
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
          pointerEvents: 'auto',
          borderLeft: '3px solid #b5179e',
        }}
      >
        <Brain style={{ color: '#b5179e', width: 18, height: 18 }} />
        <div>
          <div style={{ fontSize: '0.6rem', color: '#8e9bb3', letterSpacing: '0.05em' }}>ML POSE ENGINE</div>
          <div className="mono" style={{ fontSize: '0.82rem', fontWeight: 700, color: '#f0f4fc' }}>
            17-PT COCO <span style={{ fontSize: '0.65rem', color: '#00ff9d' }}>{(confidence * 100).toFixed(1)}% CONF</span>
          </div>
        </div>
      </div>
    </div>
  );
};
