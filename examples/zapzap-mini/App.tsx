// ZapZap Mini — 8x8 circuit puzzle showcasing dynamic lighting + normal maps.
// Tap tiles to rotate. Connect left pins to right pins to zap!

import { useState, useCallback } from 'react';
import { useZapEngine } from '@zap/web/react';
import type { GameEvent } from '@zap/web/react';

const WASM_URL = '/examples/zapzap-mini/pkg/zapzap_mini.js';
const ASSETS_URL = '/examples/zapzap-mini/public/assets/assets.json';

export function App() {
  const [score, setScore] = useState(0);
  const [lastZap, setLastZap] = useState(0);

  const onGameEvent = useCallback((events: GameEvent[]) => {
    for (const e of events) {
      if (e.kind === 1) {
        setScore(e.a);
        if (e.b > 0) {
          setLastZap(e.b);
        }
      }
    }
  }, []);

  const { canvasRef, sendEvent, fps, isReady, canvasKey } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 800,
    gameHeight: 600,
    onGameEvent,
  });

  const handleNewGame = () => {
    sendEvent({ type: 'custom', kind: 1 });
    setScore(0);
    setLastZap(0);
  };

  return (
    <div style={{ position: 'relative', width: '100vw', height: '100vh', background: '#0a0a1a' }}>
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
          color: '#7af',
          fontFamily: 'monospace',
          fontSize: 20,
          background: 'rgba(0,0,0,0.7)',
          padding: '6px 14px',
          borderRadius: 6,
          textShadow: '0 0 8px rgba(100,180,255,0.5)',
        }}>
          Score: {Math.round(score)}
        </div>
        {lastZap > 0 && (
          <div style={{
            color: '#ff6',
            fontFamily: 'monospace',
            fontSize: 14,
            background: 'rgba(0,0,0,0.5)',
            padding: '4px 10px',
            borderRadius: 4,
          }}>
            +{Math.round(lastZap)} tiles zapped!
          </div>
        )}
        <button
          onClick={handleNewGame}
          style={{
            fontFamily: 'monospace',
            fontSize: 14,
            padding: '6px 14px',
            borderRadius: 6,
            border: '1px solid rgba(100,180,255,0.3)',
            background: 'rgba(30,50,80,0.8)',
            color: '#7af',
            cursor: 'pointer',
          }}
        >
          New Game
        </button>
      </div>
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
      }}>
        {isReady ? `${fps} FPS` : 'Loading...'}
      </div>
      {isReady && (
        <div style={{
          position: 'absolute',
          bottom: 12,
          left: '50%',
          transform: 'translateX(-50%)',
          color: 'rgba(100,180,255,0.5)',
          fontFamily: 'monospace',
          fontSize: 12,
        }}>
          Tap tiles to rotate — connect left to right to zap!
        </div>
      )}
    </div>
  );
}
