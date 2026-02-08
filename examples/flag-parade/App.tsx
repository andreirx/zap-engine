// Flag Parade — SDF waving flag simulation.
// Pick a flag from the side panel and watch it wave.

import { useState, useEffect, useRef } from 'react';
import { useZapEngine } from '@zap/web/react';

const WASM_URL = '/examples/flag-parade/pkg/flag_parade.js';
const ASSETS_URL = '/examples/flag-parade/public/assets/assets.json';

const COLS = 24;
const ROWS = 16;

// ── Flag color logic (mirrors flags.rs) ──────────────────────────────

function flagColor(flag: number, col: number, row: number): [number, number, number] {
  switch (flag) {
    case 0: return france(col);
    case 1: return germany(row);
    case 2: return italy(col);
    case 3: return romania(col);
    case 4: return ukraine(row);
    case 5: return japan(col, row);
    case 6: return sweden(col, row);
    case 7: return switzerland(col, row);
    case 8: return usa(col, row);
    case 9: return uk(col, row);
    default: return [128, 128, 128];
  }
}

function france(col: number): [number, number, number] {
  const third = COLS / 3 | 0;
  if (col < third) return [0, 38, 153];
  if (col < third * 2) return [255, 255, 255];
  return [230, 26, 38];
}

function germany(row: number): [number, number, number] {
  const third = ROWS / 3 | 0;
  if (row < third) return [13, 13, 13];
  if (row < third * 2) return [217, 26, 26];
  return [255, 204, 0];
}

function italy(col: number): [number, number, number] {
  const third = COLS / 3 | 0;
  if (col < third) return [0, 140, 69];
  if (col < third * 2) return [255, 255, 255];
  return [204, 38, 38];
}

function romania(col: number): [number, number, number] {
  const third = COLS / 3 | 0;
  if (col < third) return [0, 41, 148];
  if (col < third * 2) return [242, 204, 0];
  return [204, 31, 38];
}

function ukraine(row: number): [number, number, number] {
  if (row < ROWS / 2) return [0, 89, 191];
  return [255, 217, 0];
}

function japan(col: number, row: number): [number, number, number] {
  const cx = (COLS - 1) / 2;
  const cy = (ROWS - 1) / 2;
  const dx = col - cx;
  const dy = row - cy;
  const dist = Math.sqrt(dx * dx + dy * dy);
  const radius = (ROWS - 1) * 0.30;
  if (dist < radius) return [191, 0, 26];
  return [255, 255, 255];
}

function sweden(col: number, row: number): [number, number, number] {
  const crossCol = (COLS * 0.36) | 0;
  const crossRowCenter = (ROWS - 1) / 2;
  const onV = Math.abs(col - crossCol) <= 1;
  const onH = Math.abs(row - crossRowCenter) <= 1.1;
  if (onH || onV) return [255, 204, 0];
  return [0, 77, 153];
}

function switzerland(col: number, row: number): [number, number, number] {
  const cx = (COLS - 1) / 2;
  const cy = (ROWS - 1) / 2;
  const dx = Math.abs(col - cx);
  const dy = Math.abs(row - cy);
  const onH = dy <= 1.1 && dx <= 4.0;
  const onV = dx <= 1.1 && dy <= 4.0;
  if (onH || onV) return [255, 255, 255];
  return [204, 13, 26];
}

function usa(col: number, row: number): [number, number, number] {
  const cantonCols = ((COLS * 2 + 4) / 5) | 0;
  const cantonRows = (ROWS / 2) | 0;
  const inCanton = col < cantonCols && row < cantonRows;
  if (inCanton) {
    if (col % 2 === 0 && row % 2 === 0) return [255, 255, 255];
    return [13, 26, 102];
  }
  if (row % 2 === 0) return [191, 26, 38];
  return [255, 255, 255];
}

function uk(col: number, row: number): [number, number, number] {
  const cx = (COLS - 1) / 2;
  const cy = (ROWS - 1) / 2;
  const dx = col - cx;
  const dy = row - cy;
  const nx = dx / cx;
  const ny = dy / cy;

  const redCrossV = Math.abs(dx) <= 1.0;
  const redCrossH = Math.abs(dy) <= 0.8;
  const whiteCrossV = Math.abs(dx) <= 1.8;
  const whiteCrossH = Math.abs(dy) <= 1.5;
  const d1 = Math.abs(nx - ny);
  const d2 = Math.abs(nx + ny);
  const redDiag = d1 < 0.14 || d2 < 0.14;
  const whiteDiag = d1 < 0.28 || d2 < 0.28;

  if (redCrossV || redCrossH) return [204, 26, 38];
  if (whiteCrossV || whiteCrossH) return [255, 255, 255];
  if (redDiag) return [204, 26, 38];
  if (whiteDiag) return [255, 255, 255];
  return [0, 38, 115];
}

// ── Flag icon component ──────────────────────────────────────────────

const FLAG_NAMES = [
  'France', 'Germany', 'Italy', 'Romania', 'Ukraine',
  'Japan', 'Sweden', 'Switzerland', 'USA', 'UK',
];

function FlagIcon({ flagIndex }: { flagIndex: number }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = COLS;
    const h = ROWS;
    const imageData = ctx.createImageData(w, h);
    const data = imageData.data;

    for (let row = 0; row < h; row++) {
      for (let col = 0; col < w; col++) {
        const [r, g, b] = flagColor(flagIndex, col, row);
        const idx = (row * w + col) * 4;
        data[idx] = r;
        data[idx + 1] = g;
        data[idx + 2] = b;
        data[idx + 3] = 255;
      }
    }

    ctx.putImageData(imageData, 0, 0);
  }, [flagIndex]);

  return (
    <canvas
      ref={canvasRef}
      width={COLS}
      height={ROWS}
      style={{
        width: 36,
        height: 24,
        borderRadius: 2,
        imageRendering: 'pixelated',
        flexShrink: 0,
      }}
    />
  );
}

// ── App ──────────────────────────────────────────────────────────────

export function App() {
  const [selected, setSelected] = useState(0);

  const { canvasRef, sendEvent, fps, isReady, canvasKey } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 800,
    gameHeight: 600,
  });

  const selectFlag = (idx: number) => {
    setSelected(idx);
    sendEvent({ type: 'custom', kind: 1, a: idx });
  };

  return (
    <div style={{ position: 'relative', width: '100vw', height: '100vh', background: '#0a0a1a' }}>
      <canvas
        key={canvasKey}
        ref={canvasRef}
        style={{ width: '100%', height: '100%', display: 'block' }}
      />

      {/* Flag selector panel */}
      <div style={{
        position: 'absolute',
        top: 0,
        left: 0,
        bottom: 0,
        width: 160,
        background: 'rgba(10,10,26,0.85)',
        borderRight: '1px solid rgba(255,255,255,0.08)',
        overflowY: 'auto',
        padding: '12px 0',
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
      }}>
        {FLAG_NAMES.map((name, idx) => (
          <button
            key={name}
            onClick={() => selectFlag(idx)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '8px 12px',
              background: selected === idx ? 'rgba(100,180,255,0.15)' : 'transparent',
              border: 'none',
              borderLeft: selected === idx ? '3px solid #7af' : '3px solid transparent',
              cursor: 'pointer',
              color: selected === idx ? '#7af' : 'rgba(255,255,255,0.6)',
              fontFamily: 'system-ui, sans-serif',
              fontSize: 13,
              textAlign: 'left',
              width: '100%',
            }}
          >
            <FlagIcon flagIndex={idx} />
            {name}
          </button>
        ))}
      </div>

      {/* FPS */}
      <div style={{
        position: 'absolute',
        top: 8,
        right: 12,
        color: 'rgba(255,255,255,0.3)',
        fontFamily: 'monospace',
        fontSize: 12,
        pointerEvents: 'none',
      }}>
        {isReady ? `${fps} fps` : 'loading...'}
      </div>
    </div>
  );
}
