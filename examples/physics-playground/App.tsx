// Physics Playground — Angry Birds-style physics sandbox.
// Drag near the sling to aim, release to launch. Reset button to try again.

import { useState, useCallback } from 'react';
import { useZapEngine, TimingBars } from '@zap/web/react';
import type { GameEvent } from '@zap/web/react';

const WASM_URL = '/examples/physics-playground/pkg/physics_playground.js';
const ASSETS_URL = '/examples/physics-playground/public/assets/assets.json';

export function App() {
  const [score, setScore] = useState(0);

  const onGameEvent = useCallback((events: GameEvent[]) => {
    for (const e of events) {
      if (e.kind === 1) {
        setScore(e.a);
      }
    }
  }, []);

  const { canvasRef, sendEvent, fps, isReady, canvasKey, timing } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 1200,
    gameHeight: 600,
    onGameEvent,
  });

  const handleReset = () => {
    sendEvent({ type: 'custom', kind: 1 });
  };

  return (
    <div style={{ position: 'relative', width: '100vw', height: '100vh', background: '#000' }}>
      <canvas
        key={canvasKey}
        ref={canvasRef}
        style={{ width: '100%', height: '100%', display: 'block' }}
      />
      <div style={{
        position: 'absolute',
        top: 12,
        left: 16,
        display: 'flex',
        gap: 12,
        alignItems: 'center',
      }}>
        <div style={{
          color: '#fff',
          fontFamily: 'monospace',
          fontSize: 18,
          background: 'rgba(0,0,0,0.6)',
          padding: '6px 14px',
          borderRadius: 6,
        }}>
          Score: {score} / 15
        </div>
        <button
          onClick={handleReset}
          style={{
            fontFamily: 'monospace',
            fontSize: 14,
            padding: '6px 14px',
            borderRadius: 6,
            border: 'none',
            background: '#e74c3c',
            color: '#fff',
            cursor: 'pointer',
          }}
        >
          Reset
        </button>
      </div>
      <div style={{
        position: 'absolute',
        top: 12,
        right: 16,
      }}>
        <div style={{
          color: '#fff',
          fontFamily: 'monospace',
          fontSize: 14,
          textAlign: 'right',
          marginBottom: 4,
        }}>
          {isReady ? `${fps} FPS` : 'Loading...'}
        </div>
        {isReady && <TimingBars timing={timing} usPerPixel={50} maxWidth={150} barHeight={6} />}
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
          Drag near the left circle to aim and launch
        </div>
      )}
    </div>
  );
}
