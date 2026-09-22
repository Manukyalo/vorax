import React, { useState } from 'react';
import { Bell, ShieldAlert, CheckCircle, Radio } from 'lucide-react';
import type { SecurityEvent } from '../types/telemetry';

interface EnterpriseEventLogProps {
  events: SecurityEvent[];
}

export const EnterpriseEventLog: React.FC<EnterpriseEventLogProps> = ({ events }) => {
  const [filter, setFilter] = useState<'all' | 'breach' | 'gateways'>('all');

  const filtered = events.filter((evt) => {
    if (filter === 'breach') return evt.level === 'warning' || evt.level === 'critical';
    if (filter === 'gateways') return evt.type === 'ONVIF_DISPATCH' || evt.type === 'SIA_ALARM';
    return true;
  });

  return (
    <div className="tactical-card" style={{ padding: '14px', display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }}>
      {/* Title & Filter Tabs */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 10 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <Bell style={{ width: 15, height: 15, color: '#00ff9d' }} />
          <span style={{ fontSize: '0.72rem', fontWeight: 700, letterSpacing: '0.08em', color: '#8e9bb3' }}>
            ENTERPRISE EVENT STREAM (ONVIF / SIA / VMS)
          </span>
        </div>

        <div style={{ display: 'flex', gap: '4px' }}>
          {(['all', 'breach', 'gateways'] as const).map((mode) => (
            <button
              key={mode}
              onClick={() => setFilter(mode)}
              style={{
                background: filter === mode ? 'rgba(0, 240, 255, 0.15)' : 'rgba(255,255,255,0.03)',
                color: filter === mode ? '#00f0ff' : '#687791',
                border: 'none',
                padding: '2px 7px',
                borderRadius: 3,
                fontSize: '0.58rem',
                fontWeight: 600,
                cursor: 'pointer',
                textTransform: 'uppercase',
              }}
            >
              {mode}
            </button>
          ))}
        </div>
      </div>

      {/* Scrolling List */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', overflowY: 'auto', flex: 1, paddingRight: '4px' }}>
        {filtered.map((evt) => {
          let borderColor = 'rgba(0, 255, 157, 0.4)';
          let bg = 'rgba(0, 255, 157, 0.04)';
          let textColor = '#00ff9d';
          let IconComponent = CheckCircle;

          if (evt.level === 'warning' || evt.level === 'critical') {
            borderColor = 'rgba(255, 51, 102, 0.5)';
            bg = 'rgba(255, 51, 102, 0.08)';
            textColor = '#ff3366';
            IconComponent = ShieldAlert;
          } else if (evt.type === 'ONVIF_DISPATCH' || evt.type === 'SIA_ALARM') {
            borderColor = 'rgba(0, 240, 255, 0.4)';
            bg = 'rgba(0, 240, 255, 0.05)';
            textColor = '#00f0ff';
            IconComponent = Radio;
          }

          return (
            <div
              key={evt.id}
              style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: '8px',
                padding: '6px 8px',
                borderRadius: 5,
                background: bg,
                borderLeft: `3px solid ${borderColor}`,
                borderTop: '1px solid rgba(255,255,255,0.03)',
                borderRight: '1px solid rgba(255,255,255,0.03)',
                borderBottom: '1px solid rgba(255,255,255,0.03)',
              }}
            >
              <IconComponent style={{ width: 13, height: 13, color: textColor, marginTop: 1, flexShrink: 0 }} />
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontSize: '0.66rem', fontWeight: 700, color: textColor }}>
                    {evt.title}
                  </span>
                  <span className="mono" style={{ fontSize: '0.58rem', color: '#687791' }}>
                    {evt.timestamp}
                  </span>
                </div>
                <div style={{ fontSize: '0.62rem', color: '#8e9bb3', marginTop: 1, lineHeight: 1.3 }}>
                  {evt.detail}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
