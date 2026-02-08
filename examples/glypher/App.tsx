// Glypher — a ZapEngine handwriting game.

import { useZapEngine } from '@zap/web/react';

const WASM_URL = '/examples/glypher/pkg/glypher.js';
const ASSETS_URL = '/examples/glypher/public/assets/assets.json';

export function App() {
  const { canvasRef, sendEvent, fps, isReady, canvasKey } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 800,
    gameHeight: 600,
  });

  return (
    <div style={{ position: 'relative', width: '100vw', height: '100vh', background: '#000' }}>
      <canvas
        key={canvasKey}
        ref={canvasRef}
        style={{ width: '100%', height: '100%', display: 'block', touchAction: 'none' }}
      />
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
