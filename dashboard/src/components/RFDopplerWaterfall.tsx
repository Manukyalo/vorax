import React, { useEffect, useRef } from 'react';
import { Waves } from 'lucide-react';

interface RFDopplerWaterfallProps {
  amplitudes: number[];
  velocity: number;
}

export const RFDopplerWaterfall: React.FC<RFDopplerWaterfallProps> = ({ amplitudes, velocity }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const velocityRef = useRef(velocity);

  useEffect(() => {
    velocityRef.current = velocity;
  }, [velocity]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let animId: number;
    const width = canvas.width;
    const height = canvas.height;

    // Scrolling spectrogram loop
    const render = () => {
      // Shift existing canvas content down by 1px
      const imgData = ctx.getImageData(0, 0, width, height - 1);
      ctx.putImageData(imgData, 0, 1);

      // Draw new top scanline
      const v = velocityRef.current;
      const center = width / 2;
      const spread = 20 + v * 30;

      for (let x = 0; x < width; x++) {
        const distFromCenter = Math.abs(x - center);
        let intensity = 0;

        if (distFromCenter < spread) {
          intensity = Math.max(0, 1 - distFromCenter / spread);
          intensity += (Math.random() - 0.5) * 0.25;
        } else {
          intensity = Math.random() * 0.08;
        }
        intensity = Math.min(1, Math.max(0, intensity));

        // Color mapping: dark blue -> cyan -> yellow -> red
        let r = 0, g = 0, b = 0;
        if (intensity < 0.3) {
          b = Math.floor(intensity * 3 * 255);
        } else if (intensity < 0.7) {
          g = Math.floor((intensity - 0.3) * 2.5 * 255);
          b = 255;
        } else {
          r = Math.floor((intensity - 0.7) * 3.3 * 255);
          g = 255;
          b = Math.floor((1 - intensity) * 255);
        }

        ctx.fillStyle = `rgb(${r},${g},${b})`;
        ctx.fillRect(x, 0, 1, 1);
      }

      animId = requestAnimationFrame(render);
    };

    animId = requestAnimationFrame(render);

    return () => {
      cancelAnimationFrame(animId);
    };
  }, []);

  return (
    <div className="tactical-card" style={{ padding: '14px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
      {/* Title */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <Waves style={{ width: 15, height: 15, color: '#00f0ff' }} />
          <span style={{ fontSize: '0.72rem', fontWeight: 700, letterSpacing: '0.08em', color: '#8e9bb3' }}>
            MICRO-DOPPLER SPECTROGRAM & 64-SC OFDM
          </span>
        </div>
        <span className="mono" style={{ fontSize: '0.62rem', color: '#8e9bb3' }}>
          BW: 20MHz (2.412 GHz)
        </span>
      </div>

      {/* Live Waterfall Spectrogram Canvas */}
      <div style={{ position: 'relative', borderRadius: 4, overflow: 'hidden', border: '1px solid rgba(255,255,255,0.06)' }}>
        <canvas
          ref={canvasRef}
          width={280}
          height={65}
          style={{ width: '100%', height: 65, display: 'block', background: '#05070a' }}
        />
        <div
          style={{
            position: 'absolute',
            bottom: 2,
            left: 6,
            right: 6,
            display: 'flex',
            justifyContent: 'space-between',
            fontSize: '0.55rem',
            color: 'rgba(255,255,255,0.5)',
            pointerEvents: 'none',
          }}
        >
          <span>-2.5 m/s</span>
          <span>0 m/s (DC)</span>
          <span>+2.5 m/s</span>
        </div>
      </div>

      {/* 64-OFDM Subcarrier Amplitude Profile */}
      <div style={{ display: 'flex', alignItems: 'flex-end', height: 42, gap: 1, padding: '2px 0' }}>
        {amplitudes.map((amp, idx) => {
          const isVitalsBand = idx >= 20 && idx <= 44;
          const heightPct = Math.min(100, Math.max(12, (amp / 36) * 100));

          return (
            <div
              key={idx}
              style={{
                flex: 1,
                height: `${heightPct}%`,
                background: isVitalsBand ? '#00ff9d' : '#00f0ff',
                borderRadius: 1,
                opacity: isVitalsBand ? 0.95 : 0.65,
                transition: 'height 0.05s ease',
              }}
            />
          );
        })}
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.58rem', color: '#687791' }}>
        <span>SC 0</span>
        <span style={{ color: '#00ff9d' }}>PHY Vitals Band (SC 20–44)</span>
        <span>SC 63</span>
      </div>
    </div>
  );
};
