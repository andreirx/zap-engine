// React Demo — demonstrates useZapEngine hook with a HUD overlay.
// Reuses the basic-demo WASM + assets (no new Rust code needed).

import { useZapEngine } from '../../src/engine/react';

const WASM_URL = '/examples/basic-demo/pkg/basic_demo.js';
const ASSETS_URL = '/examples/basic-demo/public/assets/assets.json';

export function App() {
  const { canvasRef, fps, isReady, canvasKey } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
  });

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
    </div>
  );
}
