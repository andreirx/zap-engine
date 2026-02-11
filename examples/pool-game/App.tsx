// Pool Game - 2D billiards with Rapier2D physics and SDF ball rendering.

import { useState, useCallback, useEffect } from 'react';
import { useZapEngine, TimingBars } from '@zap/web/react';
import type { GameEvent } from '@zap/web/react';

const WASM_URL = '/examples/pool-game/pkg/pool_game.js';
const ASSETS_URL = '/examples/pool-game/public/assets/assets.json';

export function App() {
  const [ballsRemaining, setBallsRemaining] = useState(15);
  const [timingCollapsed, setTimingCollapsed] = useState(false);
  const [isPortrait, setIsPortrait] = useState(false);
  const [containerSize, setContainerSize] = useState({ width: 0, height: 0 });

  // Detect portrait mode and compute container size
  useEffect(() => {
    function updateLayout() {
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      const portrait = vh > vw;
      setIsPortrait(portrait);

      if (portrait) {
        // Portrait: container is 1:2 (tall)
        // Fit within 80% of viewport width and 90% of viewport height
        const maxW = vw * 0.8;
        const maxH = vh * 0.9;
        // For 1:2 aspect: height = 2 * width
        const widthFromHeight = maxH / 2;
        const width = Math.min(maxW, widthFromHeight);
        const height = width * 2;
        setContainerSize({ width, height });
      } else {
        // Landscape: container is 2:1 (wide)
        // Fit within 95% of viewport width and 80% of viewport height
        const maxW = vw * 0.95;
        const maxH = vh * 0.8;
        // For 2:1 aspect: width = 2 * height
        const widthFromHeight = maxH * 2;
        const width = Math.min(maxW, widthFromHeight);
        const height = width / 2;
        setContainerSize({ width, height });
      }
    }
    updateLayout();
    window.addEventListener('resize', updateLayout);
    return () => window.removeEventListener('resize', updateLayout);
  }, []);

  const onGameEvent = useCallback((events: GameEvent[]) => {
    for (const e of events) {
      if (e.kind === 1) {  // BALLS_REMAINING
        setBallsRemaining(e.a);
      }
    }
  }, []);

  const { canvasRef, sendEvent, fps, isReady, canvasKey, timing } = useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 1000,
    gameHeight: 500,
    onGameEvent,
  });

  const handleReset = () => {
    sendEvent({ type: 'custom', kind: 1 });  // RESET event
  };

  return (
    <div style={{
      position: 'relative',
      width: '100vw',
      height: '100vh',
      background: '#1a1a2e',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
    }}>
      {/* Canvas container */}
      <div style={{
        position: 'relative',
        width: containerSize.width,
        height: containerSize.height,
      }}>
        {/* Canvas wrapper - rotates only the canvas in portrait mode */}
        <div style={{
          position: 'absolute',
          ...(isPortrait ? {
            // Portrait: rotate canvas 90° CW
            // Container is W × 2W. Canvas wrapper is 2W × W (will appear as W × 2W after rotation).
            width: containerSize.height,   // 2W
            height: containerSize.width,   // W
            left: (containerSize.width - containerSize.height) / 2,  // Center horizontally
            top: (containerSize.height - containerSize.width) / 2,   // Center vertically
            transform: 'rotate(90deg)',
            transformOrigin: 'center center',
          } : {
            // Landscape: fill container normally
            inset: 0,
          }),
        }}>
          <canvas
            key={canvasKey}
            ref={canvasRef}
            style={{
              width: '100%',
              height: '100%',
              display: 'block',
              borderRadius: 8,
              boxShadow: '0 4px 20px rgba(0,0,0,0.5)',
            }}
          />
        </div>

        {/* HUD overlay */}
        <div style={{
          position: 'absolute',
          top: 12,
          left: 16,
          display: 'flex',
          gap: 12,
          alignItems: 'center',
          zIndex: 10,
        }}>
          <div style={{
            color: '#fff',
            fontFamily: 'monospace',
            fontSize: 16,
            background: 'rgba(0,0,0,0.7)',
            padding: '6px 14px',
            borderRadius: 6,
          }}>
            Balls: {ballsRemaining}
          </div>
          <button
            onClick={handleReset}
            style={{
              fontFamily: 'monospace',
              fontSize: 14,
              padding: '6px 14px',
              borderRadius: 6,
              border: 'none',
              background: '#2d5a27',
              color: '#fff',
              cursor: 'pointer',
            }}
          >
            Reset
          </button>
        </div>

        {/* FPS and timing */}
        <div style={{
          position: 'absolute',
          top: 12,
          right: 16,
          zIndex: 10,
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

      {/* Instructions */}
      {isReady && (
        <div style={{
          marginTop: 16,
          color: 'rgba(255,255,255,0.6)',
          fontFamily: 'monospace',
          fontSize: 13,
          textAlign: 'center',
        }}>
          Click near the cue ball and drag to aim, release to shoot
        </div>
      )}

      {/* Attribution */}
      <div style={{
        position: 'absolute',
        bottom: 8,
        right: 12,
        fontSize: 10,
        color: 'rgba(255,255,255,0.3)',
        fontFamily: 'sans-serif',
      }}>
        Pool table diagram from Wikimedia Commons (Public Domain)
      </div>
    </div>
  );
}
