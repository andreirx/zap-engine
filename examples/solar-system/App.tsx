// Solar System — Interactive orrery with time controls, pan & zoom.

import { useState, useCallback, useRef, useEffect } from 'react';
import { useZapEngine, TimingBars, GameEvent } from '@zap/web/react';

const WASM_URL = '/examples/solar-system/pkg/solar_system.js';
const ASSETS_URL = '/examples/solar-system/public/assets/assets.json';

// Custom event kinds → Rust
const CUSTOM_SET_DAYS = 1;
const CUSTOM_SET_SPEED = 2;
const CUSTOM_TOGGLE_PAUSE = 3;
const CUSTOM_SELECT = 4;
const CUSTOM_ZOOM = 5;
const CUSTOM_RESET_VIEW = 6;

// Game event kinds from Rust
const EVENT_TIME_INFO = 1;
const EVENT_DATE_INFO = 2;
const EVENT_SELECTION = 3;

// Speed presets (days per second)
const SPEEDS = [
  { label: '1 h/s',  value: 1 / 24 },
  { label: '1 d/s',  value: 1 },
  { label: '1 w/s',  value: 7 },
  { label: '1 m/s',  value: 30 },
  { label: '1 y/s',  value: 365 },
  { label: '10 y/s', value: 3650 },
];

// Planet names (must match Rust order)
const PLANET_NAMES = [
  'Mercury', 'Venus', 'Earth', 'Mars', 'Jupiter', 'Saturn', 'Uranus', 'Neptune', 'Pluto',
];
const PLANET_COLORS = [
  '#998877', '#ddbb66', '#3366cc', '#cc4422', '#ccaa77', '#ddcc88', '#88bbcc', '#4466cc', '#aa9977',
];

const MONTH_NAMES = [
  '', 'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
];

// J2000 day range: ±100 years
const DAY_MIN = -36525;
const DAY_MAX = 36525;

export function App() {
  const [days, setDays] = useState(8766); // ~Jan 2024
  const [speed, setSpeed] = useState(10);
  const [paused, setPaused] = useState(false);
  const [dateStr, setDateStr] = useState('');
  const [selectedPlanet, setSelectedPlanet] = useState<number | null>(null);
  const [selectedDist, setSelectedDist] = useState(0);
  const [timingCollapsed, setTimingCollapsed] = useState(true);

  const containerRef = useRef<HTMLDivElement>(null);

  const onGameEvent = useCallback((events: GameEvent[]) => {
    for (const ev of events) {
      switch (ev.kind) {
        case EVENT_TIME_INFO:
          setDays(ev.a);
          setSpeed(ev.b);
          setPaused(ev.c > 0.5);
          break;
        case EVENT_DATE_INFO: {
          const year = Math.round(ev.a);
          const month = Math.round(ev.b);
          const day = Math.round(ev.c);
          const mName = MONTH_NAMES[month] || '???';
          setDateStr(`${mName} ${day}, ${year}`);
          break;
        }
        case EVENT_SELECTION: {
          const idx = Math.round(ev.a);
          setSelectedPlanet(idx >= 0 && idx < PLANET_NAMES.length ? idx : null);
          setSelectedDist(ev.b);
          break;
        }
      }
    }
  }, []);

  const { canvasRef, sendEvent, isReady, canvasKey, timing, fps } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    onGameEvent,
  });

  // ── Wheel-to-zoom (non-passive to allow preventDefault) ──────────
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !sendEvent) return;
    const handler = (e: WheelEvent) => {
      e.preventDefault();
      const rect = canvas.getBoundingClientRect();
      const fx = (e.clientX - rect.left) / rect.width;
      const fy = (e.clientY - rect.top) / rect.height;
      const direction = e.deltaY < 0 ? 1 : -1; // scroll up = zoom in
      sendEvent({ type: 'custom', kind: CUSTOM_ZOOM, a: direction, b: fx, c: fy });
    };
    canvas.addEventListener('wheel', handler, { passive: false });
    return () => canvas.removeEventListener('wheel', handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sendEvent, canvasKey]);

  const handleSpeedClick = (value: number) => {
    sendEvent({ type: 'custom', kind: CUSTOM_SET_SPEED, a: value, b: 0, c: 0 });
  };

  const handleTogglePause = () => {
    sendEvent({ type: 'custom', kind: CUSTOM_TOGGLE_PAUSE, a: 0, b: 0, c: 0 });
  };

  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newDays = parseFloat(e.target.value);
    sendEvent({ type: 'custom', kind: CUSTOM_SET_DAYS, a: newDays, b: 0, c: 0 });
  };

  const handlePlanetClick = (idx: number) => {
    const newIdx = selectedPlanet === idx ? -1 : idx;
    sendEvent({ type: 'custom', kind: CUSTOM_SELECT, a: newIdx, b: 0, c: 0 });
  };

  const handleResetView = () => {
    sendEvent({ type: 'custom', kind: CUSTOM_RESET_VIEW, a: 0, b: 0, c: 0 });
  };

  return (
    <div ref={containerRef} style={{ width: '100%', height: '100%', position: 'relative', background: '#0a0a1a' }}>
      {/* Game canvas */}
      <canvas
        key={canvasKey}
        ref={canvasRef}
        style={{ width: '100%', height: '100%', display: 'block', cursor: 'grab' }}
      />

      {/* Loading overlay */}
      {!isReady && (
        <div style={{
          position: 'absolute', inset: 0,
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          background: '#0a0a1a', color: '#7af', fontSize: '1.2rem',
          fontFamily: 'monospace',
        }}>
          Loading solar system...
        </div>
      )}

      {/* Bottom control bar */}
      <div style={{
        position: 'absolute', bottom: 0, left: 0, right: 0,
        background: 'rgba(10, 10, 26, 0.85)',
        borderTop: '1px solid rgba(255,255,255,0.08)',
        padding: '10px 20px',
        display: 'flex', alignItems: 'center', gap: 16,
        fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
        fontSize: '0.85rem', color: '#e0e0e8',
      }}>
        {/* Date display */}
        <div style={{
          minWidth: 120, fontFamily: 'monospace', fontSize: '0.95rem',
          color: '#7af', fontWeight: 600,
        }}>
          {dateStr || '---'}
        </div>

        {/* Play/Pause */}
        <button onClick={handleTogglePause} style={btnStyle}>
          {paused ? '\u25B6' : '\u275A\u275A'}
        </button>

        {/* Speed buttons */}
        {SPEEDS.map(s => (
          <button
            key={s.value}
            onClick={() => handleSpeedClick(s.value)}
            style={{
              ...btnStyle,
              background: Math.abs(speed - s.value) < Math.max(0.02, s.value * 0.1)
                ? 'rgba(100,180,255,0.25)' : 'rgba(255,255,255,0.06)',
              borderColor: Math.abs(speed - s.value) < Math.max(0.02, s.value * 0.1)
                ? 'rgba(100,180,255,0.5)' : 'rgba(255,255,255,0.12)',
            }}
          >
            {s.label}
          </button>
        ))}

        {/* Time slider */}
        <input
          type="range"
          min={DAY_MIN}
          max={DAY_MAX}
          step={1}
          value={Math.round(days)}
          onChange={handleSliderChange}
          style={{ flex: 1, cursor: 'pointer', accentColor: '#7af' }}
        />

        {/* Reset view */}
        <button onClick={handleResetView} style={btnStyle} title="Reset pan & zoom">
          Reset
        </button>
      </div>

      {/* Planet legend (top-left) */}
      <div style={{
        position: 'absolute', top: 10, left: 10,
        display: 'flex', flexDirection: 'column', gap: 3,
      }}>
        {PLANET_NAMES.map((name, i) => (
          <button
            key={name}
            onClick={() => handlePlanetClick(i)}
            style={{
              background: selectedPlanet === i ? 'rgba(100,180,255,0.15)' : 'transparent',
              border: 'none', color: '#ccc', fontSize: '0.75rem',
              fontFamily: 'monospace', cursor: 'pointer',
              padding: '2px 8px', borderRadius: 4, textAlign: 'left',
              display: 'flex', alignItems: 'center', gap: 6,
            }}
          >
            <span style={{
              width: 8, height: 8, borderRadius: '50%',
              background: PLANET_COLORS[i], display: 'inline-block', flexShrink: 0,
            }} />
            {name}
          </button>
        ))}
      </div>

      {/* Selected planet info (top-right) */}
      {selectedPlanet !== null && (
        <div style={{
          position: 'absolute', top: 10, right: 10,
          background: 'rgba(10, 10, 26, 0.85)',
          border: '1px solid rgba(255,255,255,0.1)',
          borderRadius: 8, padding: '12px 16px',
          fontFamily: 'monospace', fontSize: '0.85rem', color: '#e0e0e8',
          minWidth: 180,
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
            <span style={{
              width: 12, height: 12, borderRadius: '50%',
              background: PLANET_COLORS[selectedPlanet], display: 'inline-block',
            }} />
            <span style={{ color: '#7af', fontWeight: 700, fontSize: '1rem' }}>
              {PLANET_NAMES[selectedPlanet]}
            </span>
          </div>
          <div style={{ color: 'rgba(255,255,255,0.5)', lineHeight: 1.8 }}>
            Distance: <span style={{ color: '#e0e0e8' }}>{selectedDist.toFixed(3)} AU</span>
          </div>
        </div>
      )}

      {/* Zoom hint (bottom-right, above control bar) */}
      <div style={{
        position: 'absolute', bottom: 52, right: 12,
        color: 'rgba(255,255,255,0.25)', fontSize: '0.7rem',
        fontFamily: 'monospace', pointerEvents: 'none',
      }}>
        scroll to zoom &middot; drag to pan
      </div>

      {/* Performance timing */}
      {isReady && (
        <div style={{ position: 'absolute', bottom: 52, left: 12 }}>
          <div style={{
            color: 'rgba(255,255,255,0.3)',
            fontFamily: 'monospace',
            fontSize: 10,
            marginBottom: 4,
          }}>
            {fps} FPS
          </div>
          <TimingBars
            timing={timing}
            usPerPixel={50}
            maxWidth={150}
            barHeight={6}
            collapsed={timingCollapsed}
            onToggle={() => setTimingCollapsed(!timingCollapsed)}
          />
        </div>
      )}
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  background: 'rgba(255,255,255,0.06)',
  border: '1px solid rgba(255,255,255,0.12)',
  borderRadius: 6, padding: '4px 12px',
  color: '#e0e0e8', cursor: 'pointer', fontSize: '0.8rem',
  fontFamily: 'monospace', whiteSpace: 'nowrap',
};
