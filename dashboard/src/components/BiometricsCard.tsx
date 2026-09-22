import React, { useMemo } from 'react';
import { Heart, Wind, Activity } from 'lucide-react';
import type { AlertState } from '../types/telemetry';

interface BiometricsCardProps {
  heartRate: number;
  respirationRate: number;
  hrv: number;
  confidence: number;
  alertState: AlertState;
}

export const BiometricsCard: React.FC<BiometricsCardProps> = ({
  heartRate,
  respirationRate,
  hrv,
  confidence,
  alertState,
}) => {
  // Generate dynamic micro-Doppler physiological waveform scaled by vitals
  const wavePoints = useMemo(() => {
    const points: string[] = [];
    const width = 300;
    const height = 45;
    const midY = height / 2;

    const cardiacPeriod = Math.max(8, 14 * (72 / Math.max(40, heartRate)));
    const respPeriod = Math.max(20, 40 * (15 / Math.max(8, respirationRate)));

    for (let x = 0; x <= width; x += 3) {
      const cardiac = Math.sin((x / cardiacPeriod) * Math.PI) > 0.85 ? Math.sin((x / cardiacPeriod) * Math.PI) * 16 : 0;
      const resp = Math.sin((x / respPeriod) * Math.PI) * 6;
      const y = midY - (cardiac + resp);
      points.push(`${x},${y.toFixed(1)}`);
    }
    return points.join(' ');
  }, [heartRate, respirationRate]);

  const isNominal = alertState === 'Nominal';

  return (
    <div className="tactical-card" style={{ padding: '16px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <Activity style={{ width: 16, height: 16, color: '#ff3366' }} />
          <span style={{ fontSize: '0.72rem', fontWeight: 700, letterSpacing: '0.08em', color: '#8e9bb3' }}>
            CONTACTLESS VITAL SIGNS (RF MICRO-DOPPLER)
          </span>
        </div>
        <span
          style={{
            fontSize: '0.62rem',
            padding: '2px 8px',
            borderRadius: 4,
            fontWeight: 700,
            background: isNominal ? 'rgba(0, 255, 157, 0.12)' : 'rgba(255, 51, 102, 0.15)',
            color: isNominal ? '#00ff9d' : '#ff3366',
            border: `1px solid ${isNominal ? 'rgba(0, 255, 157, 0.3)' : 'rgba(255, 51, 102, 0.35)'}`,
          }}
        >
          {alertState.toUpperCase()}
        </span>
      </div>

      {/* Metrics Row */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
        {/* Heart Rate */}
        <div style={{ background: 'rgba(255, 255, 255, 0.02)', borderRadius: 8, padding: '10px 12px', border: '1px solid rgba(255,255,255,0.05)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '5px', fontSize: '0.62rem', color: '#8e9bb3' }}>
            <Heart style={{ width: 12, height: 12, color: '#ff3366' }} />
            HEART RATE (MICRO-DOPPLER)
          </div>
          <div className="mono" style={{ fontSize: '1.45rem', fontWeight: 800, color: '#ff3366', marginTop: 2 }}>
            {heartRate.toFixed(1)} <span style={{ fontSize: '0.68rem', color: '#8e9bb3', fontWeight: 500 }}>BPM</span>
          </div>
        </div>

        {/* Respiration */}
        <div style={{ background: 'rgba(255, 255, 255, 0.02)', borderRadius: 8, padding: '10px 12px', border: '1px solid rgba(255,255,255,0.05)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '5px', fontSize: '0.62rem', color: '#8e9bb3' }}>
            <Wind style={{ width: 12, height: 12, color: '#00f0ff' }} />
            RESPIRATION RATE
          </div>
          <div className="mono" style={{ fontSize: '1.45rem', fontWeight: 800, color: '#00f0ff', marginTop: 2 }}>
            {respirationRate.toFixed(1)} <span style={{ fontSize: '0.68rem', color: '#8e9bb3', fontWeight: 500 }}>BrPM</span>
          </div>
        </div>
      </div>

      {/* Real-Time Thoracic Micro-Displacement Waveform */}
      <div
        style={{
          background: 'rgba(0, 0, 0, 0.35)',
          borderRadius: 6,
          padding: '6px 8px',
          border: '1px solid rgba(255, 255, 255, 0.05)',
          overflow: 'hidden',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.58rem', color: '#687791', marginBottom: 2 }}>
          <span>THORACIC PHASE DISPLACEMENT (±0.12mm)</span>
          <span style={{ color: '#00ff9d' }}>CSI SUB-CARRIER 28</span>
        </div>
        <svg width="100%" height="40" viewBox="0 0 300 45" preserveAspectRatio="none" style={{ display: 'block' }}>
          <polyline
            fill="none"
            stroke="#00ff9d"
            strokeWidth="1.6"
            strokeLinecap="round"
            strokeLinejoin="round"
            points={wavePoints}
          />
        </svg>
      </div>

      {/* Secondary Metrics */}
      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.68rem', color: '#8e9bb3', borderTop: '1px solid rgba(255,255,255,0.05)', paddingTop: 8 }}>
        <div>
          HRV (rMSSD): <strong className="mono" style={{ color: '#fff' }}>{hrv.toFixed(1)} ms</strong>
        </div>
        <div>
          CONFIDENCE: <strong className="mono" style={{ color: '#00ff9d' }}>{(confidence * 100).toFixed(1)}%</strong>
        </div>
      </div>
    </div>
  );
};
