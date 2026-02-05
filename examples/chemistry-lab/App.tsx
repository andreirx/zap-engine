// Chemistry Lab — interactive molecule builder.
// Click to place atoms, drag between atoms to create bonds.

import { useState, useCallback } from 'react';
import { useZapEngine } from '../../src/engine/react';
import type { GameEvent } from '../../src/engine/react/useZapEngine';

const WASM_URL = '/examples/chemistry-lab/pkg/chemistry_lab.js';
const ASSETS_URL = '/examples/chemistry-lab/public/assets/assets.json';

const ELEMENTS = [
  { kind: 1, symbol: 'H', name: 'Hydrogen', color: '#e0e0e0' },
  { kind: 2, symbol: 'O', name: 'Oxygen', color: '#e63946' },
  { kind: 3, symbol: 'C', name: 'Carbon', color: '#555' },
  { kind: 4, symbol: 'N', name: 'Nitrogen', color: '#4a7cf7' },
];

export function App() {
  const [selected, setSelected] = useState(3); // Carbon default
  const [atomCount, setAtomCount] = useState(0);
  const [bondCount, setBondCount] = useState(0);

  const onGameEvent = useCallback((events: GameEvent[]) => {
    for (const e of events) {
      if (e.kind === 1) setAtomCount(e.a);
      if (e.kind === 2) setBondCount(e.a);
    }
  }, []);

  const { canvasRef, sendEvent, fps, isReady, canvasKey } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 800,
    gameHeight: 600,
    onGameEvent,
  });

  const selectElement = (kind: number) => {
    setSelected(kind);
    sendEvent({ type: 'custom', kind: 1, a: kind });
  };

  const handleClear = () => {
    sendEvent({ type: 'custom', kind: 2 });
  };

  return (
    <div style={{ position: 'relative', width: '100vw', height: '100vh', background: '#000' }}>
      <canvas
        key={canvasKey}
        ref={canvasRef}
        style={{ width: '100%', height: '100%', display: 'block' }}
      />
      {/* Element selector */}
      <div style={{
        position: 'absolute',
        top: 12,
        left: 16,
        display: 'flex',
        gap: 8,
        alignItems: 'center',
      }}>
        {ELEMENTS.map(el => (
          <button
            key={el.kind}
            onClick={() => selectElement(el.kind)}
            title={el.name}
            style={{
              width: 40,
              height: 40,
              borderRadius: '50%',
              border: selected === el.kind ? '3px solid #fff' : '2px solid rgba(255,255,255,0.3)',
              background: el.color,
              color: el.kind === 3 ? '#fff' : '#000',
              fontFamily: 'monospace',
              fontWeight: 'bold',
              fontSize: 16,
              cursor: 'pointer',
              boxShadow: selected === el.kind ? '0 0 12px rgba(255,255,255,0.4)' : 'none',
            }}
          >
            {el.symbol}
          </button>
        ))}
        <button
          onClick={handleClear}
          style={{
            fontFamily: 'monospace',
            fontSize: 12,
            padding: '6px 12px',
            borderRadius: 6,
            border: 'none',
            background: '#e74c3c',
            color: '#fff',
            cursor: 'pointer',
            marginLeft: 8,
          }}
        >
          Clear
        </button>
      </div>
      {/* Stats */}
      <div style={{
        position: 'absolute',
        top: 12,
        right: 16,
        color: '#fff',
        fontFamily: 'monospace',
        fontSize: 14,
        background: 'rgba(0,0,0,0.5)',
        padding: '4px 10px',
        borderRadius: 4,
        display: 'flex',
        gap: 12,
      }}>
        <span>Atoms: {atomCount}</span>
        <span>Bonds: {bondCount}</span>
        <span>{isReady ? `${fps} FPS` : 'Loading...'}</span>
      </div>
      {isReady && (
        <div style={{
          position: 'absolute',
          bottom: 12,
          left: '50%',
          transform: 'translateX(-50%)',
          color: 'rgba(255,255,255,0.5)',
          fontFamily: 'monospace',
          fontSize: 12,
        }}>
          Click to place atoms &middot; Drag between atoms to bond
        </div>
      )}
    </div>
  );
}
