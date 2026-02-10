// Glypher — a ZapEngine handwriting game.

import { useZapEngine, TimingBars } from '@zap/web/react';

const WASM_URL = '/examples/glypher/pkg/glypher.js';
const ASSETS_URL = '/examples/glypher/public/assets/assets.json';

export function App() {
  const { canvasRef, sendEvent, fps, isReady, canvasKey, timing } = useZapEngine({
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
        pointerEvents: 'none',
      }}>
        <div style={{
          color: 'rgba(255,255,255,0.3)',
          fontFamily: 'monospace',
          fontSize: 12,
          textAlign: 'right',
          marginBottom: 4,
        }}>
          {isReady ? `${fps} fps` : 'loading...'}
        </div>
        {isReady && <TimingBars timing={timing} usPerPixel={50} maxWidth={150} barHeight={6} />}
      </div>
    </div>
  );
}
