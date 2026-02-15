// React Demo — demonstrates useZapEngine hook with a HUD overlay.
// Reuses the basic-demo WASM + assets (no new Rust code needed).

import { useState } from 'react';
import { useZapEngine, TimingBars } from '@zap/web/react';

const WASM_URL = '/examples/basic-demo/pkg/basic_demo.js';
const ASSETS_URL = '/examples/basic-demo/public/assets/assets.json';

export function App() {
  const [timingCollapsed, setTimingCollapsed] = useState(true);

  const { canvasRef, fps, isReady, canvasKey, timing } = useZapEngine({
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
        {isReady && (
          <TimingBars
            timing={timing}
            usPerPixel={50}
            maxWidth={150}
            barHeight={6}
            collapsed={timingCollapsed}
            onToggle={() => setTimingCollapsed(!timingCollapsed)}
          />
        )}
      </div>
    </div>
  );
}
