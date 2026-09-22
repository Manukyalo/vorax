import React from 'react';
import { Cpu, Wifi } from 'lucide-react';
import type { MeshNodeStatus } from '../types/telemetry';

interface NodeMeshMatrixProps {
  nodes: MeshNodeStatus[];
}

export const NodeMeshMatrix: React.FC<NodeMeshMatrixProps> = ({ nodes }) => {
  return (
    <div className="tactical-card" style={{ padding: '14px', display: 'flex', flexDirection: 'column', gap: '10px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <Cpu style={{ width: 15, height: 15, color: '#00f0ff' }} />
          <span style={{ fontSize: '0.72rem', fontWeight: 700, letterSpacing: '0.08em', color: '#8e9bb3' }}>
            ESP32-S3 MESH TOPOLOGY (4 ANCHORS)
          </span>
        </div>
        <span
          className="mono"
          style={{
            fontSize: '0.62rem',
            padding: '2px 6px',
            borderRadius: 3,
            background: 'rgba(0, 255, 157, 0.12)',
            color: '#00ff9d',
            border: '1px solid rgba(0, 255, 157, 0.25)',
          }}
        >
          SYNC: ±2.4µs
        </span>
      </div>

      {/* 2x2 Grid of Anchor Nodes */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px' }}>
        {nodes.map((node) => {
          const isGoodSignal = node.rssi_dbm > -55;
          const signalColor = isGoodSignal ? '#00ff9d' : '#ffb703';

          return (
            <div
              key={node.id}
              style={{
                background: 'rgba(255, 255, 255, 0.02)',
                border: '1px solid rgba(255, 255, 255, 0.06)',
                borderRadius: 6,
                padding: '8px 10px',
                display: 'flex',
                flexDirection: 'column',
                gap: '4px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <span className="mono" style={{ fontSize: '0.68rem', fontWeight: 700, color: '#f0f4fc' }}>
                  {node.label}
                </span>
                <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                  <span
                    style={{
                      width: 6,
                      height: 6,
                      borderRadius: '50%',
                      background: signalColor,
                      boxShadow: `0 0 6px ${signalColor}`,
                    }}
                  />
                  <Wifi style={{ width: 12, height: 12, color: signalColor }} />
                </div>
              </div>

              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginTop: 2 }}>
                <span style={{ fontSize: '0.6rem', color: '#687791' }}>RSSI / POWER</span>
                <span className="mono" style={{ fontSize: '0.74rem', fontWeight: 600, color: signalColor }}>
                  {node.rssi_dbm} <span style={{ fontSize: '0.58rem', color: '#687791' }}>dBm</span>
                </span>
              </div>

              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
                <span style={{ fontSize: '0.6rem', color: '#687791' }}>SYNC JITTER</span>
                <span className="mono" style={{ fontSize: '0.72rem', color: '#8e9bb3' }}>
                  {node.sync_offset_us > 0 ? `+${node.sync_offset_us}` : node.sync_offset_us}
                  <span style={{ fontSize: '0.58rem', color: '#687791' }}> µs</span>
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
