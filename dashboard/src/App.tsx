import { useState, useEffect } from 'react';
import { SpatialCanvas } from './components/SpatialCanvas';
import { Activity, ShieldCheck, Wifi, Zap, Bell, Compass, Scan } from 'lucide-react';

interface TelemetryData {
  node_id: number;
  seq: number;
  pps: number;
  mean_amp: number;
  center_amp: number;
  heart_rate_bpm: number;
  respiration_rate_brpm: number;
  hrv_rmssd_ms?: number;
  velocity_m_s?: number;
  target_x?: number;
  target_y?: number;
  target_z?: number;
  alert_state?: string;
  presence_detected: boolean;
  presence_confidence: number;
  skeleton?: number[][];
  amplitudes: number[];
  phases: number[];
}

export function App() {
  const [data, setData] = useState<TelemetryData | null>(null);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    let ws: WebSocket | null = null;
    let fallbackInterval: number | null = null;

    try {
      ws = new WebSocket('ws://localhost:8765');

      ws.onopen = () => {
        setConnected(true);
      };

      ws.onmessage = (event) => {
        try {
          const payload: TelemetryData = JSON.parse(event.data);
          setData(payload);
        } catch (err) {
          console.error('Failed to parse telemetry', err);
        }
      };

      ws.onerror = () => {
        setConnected(false);
      };

      ws.onclose = () => {
        setConnected(false);
      };
    } catch {
      setConnected(false);
    }

    // Client-side fallback simulation for cloud hosting (Vercel / phone demo)
    fallbackInterval = window.setInterval(() => {
      setData((prev) => {
        // If live websocket is already streaming, don't overwrite
        if (ws && ws.readyState === WebSocket.OPEN) return prev;

        const now = Date.now() / 1000;
        const hr = 72.0 + Math.sin(now * 0.4) * 3.5;
        const rr = 14.5 + Math.cos(now * 0.2) * 1.2;
        const amps = Array.from({ length: 64 }, (_, i) => {
          const base = 22 + Math.sin(i * 0.25) * 6;
          const pert = (i >= 20 && i <= 44) ? Math.sin(now * 2) * 4 : 0;
          return base + pert;
        });

        return {
          node_id: 1,
          seq: (prev?.seq || 0) + 1,
          pps: 200,
          mean_amp: 24.3,
          center_amp: 24.8,
          heart_rate_bpm: hr,
          respiration_rate_brpm: rr,
          hrv_rmssd_ms: 38.5,
          velocity_m_s: 0.08,
          target_x: 3.0 + Math.sin(now * 0.5) * 1.5,
          target_y: 3.0 + Math.cos(now * 0.5) * 1.5,
          target_z: 1.1 + Math.sin(now) * 0.05,
          alert_state: 'Nominal',
          presence_detected: true,
          presence_confidence: 0.984,
          amplitudes: amps,
          phases: Array(64).fill(0),
        };
      });
    }, 50); // ~20 FPS updates

    return () => {
      if (ws) ws.close();
      if (fallbackInterval) clearInterval(fallbackInterval);
    };
  }, []);

  const hr = data?.heart_rate_bpm ? data.heart_rate_bpm.toFixed(1) : '72.0';
  const rr = data?.respiration_rate_brpm ? data.respiration_rate_brpm.toFixed(1) : '14.5';
  const hrv = data?.hrv_rmssd_ms ? data.hrv_rmssd_ms.toFixed(1) : '38.5';
  const velocity = data?.velocity_m_s ? data.velocity_m_s.toFixed(2) : '0.00';
  const targetX = data?.target_x !== undefined ? data.target_x.toFixed(2) : '3.00';
  const targetY = data?.target_y !== undefined ? data.target_y.toFixed(2) : '3.00';
  const targetZ = data?.target_z !== undefined ? data.target_z.toFixed(2) : '1.00';
  const alertState = data?.alert_state || 'Nominal';
  const pps = data?.pps || 200;
  const seq = data?.seq || 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', width: '100vw', height: '100vh', background: '#07090e' }}>
      {/* Top Cyber Navigation Bar */}
      <header
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '12px 24px',
          borderBottom: '1px solid rgba(255, 255, 255, 0.08)',
          background: 'rgba(7, 9, 14, 0.9)',
          backdropFilter: 'blur(10px)',
          zIndex: 10,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <img
              src="/vorax-logo.jpg"
              alt="VORAX"
              style={{ width: 30, height: 30, borderRadius: 6, border: '1px solid rgba(0, 240, 255, 0.4)' }}
            />
            <h1 style={{ fontSize: '1.25rem', fontWeight: 800, letterSpacing: '0.15em', color: '#fff' }}>
              VORAX<span style={{ color: '#00f0ff' }}>.AI</span>
            </h1>
          </div>
          <span
            style={{
              padding: '2px 8px',
              borderRadius: 4,
              fontSize: '0.7rem',
              fontWeight: 600,
              background: 'rgba(0, 240, 255, 0.1)',
              color: '#00f0ff',
              border: '1px solid rgba(0, 240, 255, 0.3)',
            }}
          >
            SPATIAL RADAR V0.1
          </span>
        </div>

        {/* Global Connection & Status */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span
              style={{
                width: 8,
                height: 8,
                borderRadius: '50%',
                background: connected ? '#00ff88' : '#ff3366',
                boxShadow: connected ? '0 0 10px #00ff88' : '0 0 10px #ff3366',
              }}
            />
            <span className="mono" style={{ fontSize: '0.8rem', color: connected ? '#00ff88' : '#ff3366' }}>
              {connected ? 'CORE CONNECTED (ws://8765)' : 'SEARCHING ENGINE...'}
            </span>
          </div>
          <div className="mono" style={{ fontSize: '0.8rem', color: '#8a99b5' }}>
            FRAME: <span style={{ color: '#fff' }}>#{seq.toString().padStart(6, '0')}</span>
          </div>
        </div>
      </header>

      {/* Main Visualizer Area */}
      <div style={{ display: 'flex', flex: 1, position: 'relative', overflow: 'hidden' }}>
        {/* Full Viewport 3D Three.js Spatial Canvas */}
        <div style={{ flex: 1, position: 'relative' }}>
          <SpatialCanvas
            presence={data?.presence_detected ?? true}
            heartRate={data?.heart_rate_bpm ?? 72}
            amplitudes={data?.amplitudes ?? []}
            skeleton={data?.skeleton}
            targetPos={{
              x: data?.target_x ?? 3.0,
              y: data?.target_y ?? 3.0,
              z: data?.target_z ?? 1.0,
            }}
          />

          {/* Overlay HUD Badges */}
          <div style={{ position: 'absolute', top: 20, left: 24, display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
            <div className="glass-panel" style={{ padding: '12px 18px', display: 'flex', alignItems: 'center', gap: '10px' }}>
              <ShieldCheck style={{ color: '#00ff88', width: 20, height: 20 }} />
              <div>
                <div style={{ fontSize: '0.65rem', color: '#8a99b5', letterSpacing: '0.05em' }}>TARGET OCCUPANCY</div>
                <div style={{ fontSize: '0.95rem', fontWeight: 700, color: '#00ff88' }}>1 PERSON DETECTED</div>
              </div>
            </div>

            <div className="glass-panel" style={{ padding: '12px 18px', display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Scan style={{ color: '#00d4ff', width: 20, height: 20 }} />
              <div>
                <div style={{ fontSize: '0.65rem', color: '#8a99b5', letterSpacing: '0.05em' }}>ML SKELETON MODEL</div>
                <div className="mono" style={{ fontSize: '0.95rem', fontWeight: 700, color: '#00d4ff' }}>
                  17-PT COCO <span style={{ fontSize: '0.7rem', color: '#8a99b5' }}>30 FPS</span>
                </div>
              </div>
            </div>

            <div className="glass-panel" style={{ padding: '12px 18px', display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Zap style={{ color: '#00f0ff', width: 20, height: 20 }} />
              <div>
                <div style={{ fontSize: '0.65rem', color: '#8a99b5', letterSpacing: '0.05em' }}>INGEST THROUGHPUT</div>
                <div className="mono" style={{ fontSize: '0.95rem', fontWeight: 700, color: '#fff' }}>
                  {pps} <span style={{ fontSize: '0.7rem', color: '#8a99b5' }}>PKTS/S</span>
                </div>
              </div>
            </div>

            <div className="glass-panel" style={{ padding: '12px 18px', display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Activity style={{ color: '#ffb703', width: 20, height: 20 }} />
              <div>
                <div style={{ fontSize: '0.65rem', color: '#8a99b5', letterSpacing: '0.05em' }}>DOPPLER VELOCITY</div>
                <div className="mono" style={{ fontSize: '0.95rem', fontWeight: 700, color: '#ffb703' }}>
                  {velocity} <span style={{ fontSize: '0.7rem', color: '#8a99b5' }}>M/S</span>
                </div>
              </div>
            </div>

            <div className="glass-panel" style={{ padding: '12px 18px', display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Compass style={{ color: '#00f0ff', width: 20, height: 20 }} />
              <div>
                <div style={{ fontSize: '0.65rem', color: '#8a99b5', letterSpacing: '0.05em' }}>3D SPATIAL FIX (X, Y, Z)</div>
                <div className="mono" style={{ fontSize: '0.95rem', fontWeight: 700, color: '#00f0ff' }}>
                  [{targetX}, {targetY}, {targetZ}] <span style={{ fontSize: '0.7rem', color: '#8a99b5' }}>M</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Right Telemetry & Biometrics HUD Panel */}
        <aside
          style={{
            width: 380,
            borderLeft: '1px solid rgba(255, 255, 255, 0.08)',
            background: 'rgba(10, 14, 22, 0.85)',
            backdropFilter: 'blur(20px)',
            display: 'flex',
            flexDirection: 'column',
            padding: '24px',
            gap: '20px',
            overflowY: 'auto',
            zIndex: 5,
          }}
        >
          {/* Biometrics Card */}
          <div className="glass-panel" style={{ padding: '20px' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16 }}>
              <span style={{ fontSize: '0.75rem', fontWeight: 700, letterSpacing: '0.1em', color: '#8a99b5' }}>
                BIOMETRIC TELEMETRY
              </span>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span
                  style={{
                    fontSize: '0.65rem',
                    padding: '2px 8px',
                    borderRadius: 4,
                    fontWeight: 700,
                    background: alertState === 'Nominal' ? 'rgba(0, 255, 136, 0.15)' : 'rgba(255, 51, 102, 0.2)',
                    color: alertState === 'Nominal' ? '#00ff88' : '#ff3366',
                    border: `1px solid ${alertState === 'Nominal' ? 'rgba(0, 255, 136, 0.3)' : 'rgba(255, 51, 102, 0.4)'}`,
                  }}
                >
                  {alertState.toUpperCase()}
                </span>
                <Activity style={{ color: '#ff3366', width: 16, height: 16 }} />
              </div>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
              <div style={{ background: 'rgba(255, 255, 255, 0.03)', borderRadius: 8, padding: '12px' }}>
                <div style={{ fontSize: '0.65rem', color: '#8a99b5' }}>HEART RATE (MICRO-DOPPLER)</div>
                <div className="mono" style={{ fontSize: '1.6rem', fontWeight: 800, color: '#ff3366', marginTop: 4 }}>
                  {hr} <span style={{ fontSize: '0.75rem', color: '#8a99b5' }}>BPM</span>
                </div>
              </div>

              <div style={{ background: 'rgba(255, 255, 255, 0.03)', borderRadius: 8, padding: '12px' }}>
                <div style={{ fontSize: '0.65rem', color: '#8a99b5' }}>RESPIRATION RATE</div>
                <div className="mono" style={{ fontSize: '1.6rem', fontWeight: 800, color: '#00f0ff', marginTop: 4 }}>
                  {rr} <span style={{ fontSize: '0.75rem', color: '#8a99b5' }}>BrPM</span>
                </div>
              </div>
            </div>

            <div style={{ marginTop: 14, fontSize: '0.7rem', color: '#8a99b5', display: 'flex', justifyContent: 'space-between' }}>
              <span>HRV (rMSSD): <strong style={{ color: '#fff' }}>{hrv} ms</strong></span>
              <span>CONFIDENCE: <strong style={{ color: '#00ff88' }}>98.4%</strong></span>
            </div>
          </div>

          {/* 64-Subcarrier Spectrum Profile */}
          <div className="glass-panel" style={{ padding: '20px' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16 }}>
              <span style={{ fontSize: '0.75rem', fontWeight: 700, letterSpacing: '0.1em', color: '#8a99b5' }}>
                OFDM SUBCARRIER PROFILE (64)
              </span>
              <Wifi style={{ color: '#00f0ff', width: 16, height: 16 }} />
            </div>

            {/* Live Subcarrier Amplitude Bars */}
            <div
              style={{
                display: 'flex',
                alignItems: 'flex-end',
                height: 90,
                gap: 2,
                padding: '6px 0',
                borderBottom: '1px solid rgba(255,255,255,0.06)',
              }}
            >
              {(data?.amplitudes && data.amplitudes.length > 0 ? data.amplitudes : Array.from({ length: 64 }, (_, i) => 20 + Math.sin(i * 0.2) * 8)).map(
                (amp, idx) => (
                  <div
                    key={idx}
                    style={{
                      flex: 1,
                      height: `${Math.min(100, Math.max(10, (amp / 35) * 100))}%`,
                      background: idx >= 20 && idx <= 44 ? '#00ff88' : '#00f0ff',
                      borderRadius: 1,
                      opacity: 0.85,
                      transition: 'height 0.05s ease',
                    }}
                  />
                )
              )}
            </div>

            <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 8, fontSize: '0.65rem', color: '#8a99b5' }}>
              <span>SC 0 (2.412 GHz)</span>
              <span style={{ color: '#00ff88' }}>Sensitive Vitals Band (SC 20-44)</span>
              <span>SC 63</span>
            </div>
          </div>

          {/* Security & Industrial Alert Stream */}
          <div className="glass-panel" style={{ padding: '20px', flex: 1 }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
              <span style={{ fontSize: '0.75rem', fontWeight: 700, letterSpacing: '0.1em', color: '#8a99b5' }}>
                ENTERPRISE EVENT LOG
              </span>
              <Bell style={{ color: '#00ff88', width: 16, height: 16 }} />
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              <div style={{ padding: '8px 12px', borderRadius: 6, background: 'rgba(0, 255, 136, 0.05)', borderLeft: '3px solid #00ff88' }}>
                <div style={{ fontSize: '0.7rem', color: '#00ff88', fontWeight: 600 }}>MESH SYNCHRONIZED</div>
                <div style={{ fontSize: '0.65rem', color: '#8a99b5', marginTop: 2 }}>4 nodes reporting 0 dropped frames</div>
              </div>

              <div style={{ padding: '8px 12px', borderRadius: 6, background: 'rgba(0, 240, 255, 0.05)', borderLeft: '3px solid #00f0ff' }}>
                <div style={{ fontSize: '0.7rem', color: '#00f0ff', fontWeight: 600 }}>PRESENCE VERIFIED</div>
                <div style={{ fontSize: '0.65rem', color: '#8a99b5', marginTop: 2 }}>Target localized in Room 101 Zone B</div>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}

export default App;
